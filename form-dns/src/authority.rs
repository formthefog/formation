use std::net::IpAddr;
use trust_dns_client::client::AsyncClient;
use trust_dns_proto::rr::rdata::CNAME;
use trust_dns_server::authority::{
    Authority, LookupOptions, UpdateResult, ZoneType, LookupError, MessageRequest,
    UpdateRequest
};
use trust_dns_proto::op::ResponseCode;
use trust_dns_proto::rr::{
    RecordType, RData, Record, RecordSet, LowerName, Name
};
use trust_dns_server::authority::LookupObject;
use crate::store::{FormDnsRecord, SharedStore};
use anyhow::Result;
use trust_dns_client::client::ClientHandle;

#[derive(Clone)]
pub struct SimpleLookup {
    records: RecordSet,
    additionals: Option<RecordSet>,
}

impl SimpleLookup {
    pub fn from_record_set(rrset: RecordSet) -> Self {
        Self { records: rrset, additionals: None }
    }

    pub fn with_additionals(rrset: RecordSet, additionals: RecordSet) -> Self {
        Self { records: rrset, additionals: Some(additionals) }
    }
}

pub struct FormAuthority {
    origin: LowerName,
    zone_type: ZoneType,
    store: SharedStore,
    fallback_client: AsyncClient,
}

impl FormAuthority {
    pub fn new(origin: Name, store: SharedStore, fallback_client: AsyncClient) -> Self {
        let lower_origin = LowerName::new(&origin);
        Self {
            origin: lower_origin,
            zone_type: ZoneType::Primary,
            store,
            fallback_client
        }
    }

    async fn lookup_local(
        &self,
        name: &str,
        rtype: RecordType,
        src: Option<IpAddr>,
    ) -> Option<RecordSet> {
        log::info!("trimming name");
        let key = name.trim_end_matches('.').to_lowercase();
        log::info!("trimmed name: {key}");

        let record_opt = {
            let guard = self.store.blocking_read();
            guard.get(&key)
        };
        log::info!("retreived record {record_opt:?}");

        if let Some(record) = record_opt {
            let is_formnet = {
                match src {
                    Some(IpAddr::V4(addr)) => addr.octets()[0] == 10,
                    Some(IpAddr::V6(_)) => false,
                    None => false,
                }
            };
            log::info!("Request is formnet? {is_formnet}");
            let ips = if is_formnet {
                if !record.formnet_ip.is_empty() {
                    let mut ips = record.formnet_ip.clone();
                    if !record.public_ip.is_empty() {
                        ips.extend(record.public_ip.clone());
                    }
                    ips
                } else if !record.public_ip.is_empty() {
                    record.public_ip.clone()
                } else {
                    vec![]
                }
            } else {
                if !record.public_ip.is_empty() {
                    record.public_ip.clone()
                } else {
                    vec![]
                }
            };

            log::info!("IPS: {ips:?}");

            if let Ok(rr_name) = Name::from_utf8(&key) {
                let mut rrset = RecordSet::new(&rr_name, rtype, 300);
                match rtype {
                    RecordType::A => {
                        for ip in ips { 
                            if let IpAddr::V4(v4) = ip {
                                let mut rec = Record::with(rrset.name().clone(), RecordType::A, 300);
                                rec.set_data(Some(trust_dns_proto::rr::rdata::A(v4)));
                                rrset.add_rdata(rec.into_record_of_rdata().data()?.clone());
                            }
                        }
                    }
                    RecordType::AAAA => {
                        for ip in ips {
                            if let IpAddr::V6(v6) = ip {
                                let mut rec = Record::with(rrset.name().clone(), RecordType::AAAA, 300);
                                rec.set_data(Some(trust_dns_proto::rr::rdata::AAAA(v6)));
                                rrset.add_rdata(rec.into_record_of_rdata().data()?.clone());
                            }
                        }
                    }
                    RecordType::CNAME => {
                        log::info!("Request is for CNAME record");
                        if let Ok(name) = Name::from_utf8(record.cname_target?) {
                            let rdata = RData::CNAME(CNAME(name));
                            let rec: Record<RData> = Record::from_rdata(rrset.name().clone(), 300, rdata);
                            rrset.insert(rec, 300);
                        }
                    }
                    _ => {}
                }

                if !rrset.is_empty() {
                    return Some(rrset);
                }
            }
        }

        None
    }

    async fn lookup_upstream(
        &self,
        name: &LowerName,
        rtype: RecordType,
    ) -> Result<RecordSet, LookupError> {
        let fqdn_name = Name::from_utf8(&name.to_string())
            .map_err(|_| LookupError::ResponseCode(ResponseCode::FormErr))?;

        let mut client = self.fallback_client.clone();
        let response = client.query(
            fqdn_name.clone(),
            trust_dns_proto::rr::DNSClass::IN,
            rtype
        ).await.map_err(|_| LookupError::ResponseCode(ResponseCode::ServFail))?;

        let answers = response.answers();
        if answers.is_empty() {
            return Err(LookupError::ResponseCode(ResponseCode::NXDomain));
        }

        let mut rrset = RecordSet::new(&fqdn_name, rtype, 300);
        for ans in answers {
            if ans.record_type() == rtype {
                rrset.add_rdata(
                    ans.clone()
                        .into_record_of_rdata()
                        .data()
                        .ok_or(
                            LookupError::ResponseCode(
                                ResponseCode::FormErr
                            )
                        )?
                        .clone()
                );
            }
        }

        if rrset.is_empty() {
            return Err(LookupError::ResponseCode(ResponseCode::NXDomain));
        }

        Ok(rrset)
    }

    async fn lookup_fallback(
        &self,
        name: &LowerName,
        rtype: RecordType,
    ) -> Result<RecordSet, LookupError> {
        self.lookup_upstream(name, rtype).await
    }

    async fn apply_update(&self, msg: &MessageRequest) -> Result<bool, ResponseCode> {
        let _zone_name = msg.query().name();

        let updates = msg.updates();
        if updates.is_empty() {
            return Ok(false)
        }

        let mut changed = false;

        let mut store_guard = self.store.write().await;

        for rec in updates {
            let domain = rec.name().to_string().to_lowercase();
            let rtype = rec.record_type();
            let ttl = rec.ttl();

            match (rtype, rec.clone().into_record_of_rdata().data()) {
                (RecordType::A, Some(&RData::A(v4))) => {
                    if ttl == 0 {
                        if store_guard.remove(&domain).is_some() {
                            changed = true;
                        } else {
                            let record = FormDnsRecord {
                                domain: domain.clone(),
                                record_type: rtype,
                                formnet_ip: if v4.octets()[0] == 10 {
                                    vec![IpAddr::V4(v4.into())]
                                } else {
                                    vec![]
                                },
                                public_ip: if !v4.octets()[0] == 10 {
                                    vec![IpAddr::V4(v4.into())]
                                } else {
                                    vec![]
                                },
                                cname_target: None,
                                ttl: 0
                            };
                            store_guard.insert(&domain, record);
                            changed = true;
                        }
                    } else {
                        if let Some(mut record) = store_guard.get(&domain) {
                            let form_record = FormDnsRecord {
                                record_type: rtype,
                                formnet_ip: if v4.octets()[0] == 10 {
                                    record.formnet_ip.push(IpAddr::V4(v4.into()));
                                    record.formnet_ip.clone()
                                } else { 
                                    record.formnet_ip.clone()
                                },
                                public_ip: if !v4.octets()[0] == 10 {
                                    record.public_ip.push(IpAddr::V4(v4.into()));
                                    record.public_ip.clone()
                                } else {
                                    vec![]
                                },
                                ttl,
                                ..record
                            };
                            store_guard.insert(&domain, form_record);
                            changed = true;
                        } else {
                            let record = FormDnsRecord {
                                domain: domain.clone(),
                                record_type: rtype,
                                formnet_ip: if v4.octets()[0] == 10 {
                                    vec![IpAddr::V4(v4.into())]
                                } else {
                                    vec![]
                                },
                                public_ip: if !v4.octets()[0] == 10 {
                                    vec![IpAddr::V4(v4.into())]
                                } else {
                                    vec![]
                                },
                                cname_target: None,
                                ttl
                            };
                            store_guard.insert(&domain, record);
                            changed = true;
                        }
                    }
                },
                (RecordType::AAAA, Some(&RData::AAAA(v6))) => {
                    if ttl == 0 {
                        if store_guard.remove(&domain).is_some() {
                            changed = true;
                        } else {
                            let record = FormDnsRecord {
                                domain: domain.clone(),
                                record_type: rtype,
                                formnet_ip: vec![],
                                public_ip: vec![IpAddr::V6(v6.into())],
                                cname_target: None,
                                ttl: 0
                            };
                            store_guard.insert(&domain, record);
                            changed = true;
                        }
                    } else {
                        if let Some(mut record) = store_guard.get(&domain) {
                            let form_record = FormDnsRecord {
                                record_type: rtype,
                                formnet_ip: vec![],
                                public_ip: {
                                    record.public_ip.push(IpAddr::V6(v6.into()));
                                    record.public_ip
                                },
                                ttl,
                                ..record
                            };
                            store_guard.insert(&domain, form_record);
                            changed = true;
                        } else {
                            let record = FormDnsRecord {
                                domain: domain.clone(),
                                record_type: rtype,
                                formnet_ip: vec![],
                                public_ip: vec![IpAddr::V6(v6.into())],
                                cname_target: None,
                                ttl
                            };
                            store_guard.insert(&domain, record);
                            changed = true;
                        }
                    }
                }
                (RecordType::CNAME, Some(&RData::CNAME(ref target))) => {
                    if ttl == 0 {
                        if store_guard.remove(&domain).is_some() {
                            changed = true;
                        } else {
                            let record = FormDnsRecord {
                                domain: domain.clone(),
                                record_type: rtype,
                                formnet_ip: vec![],
                                public_ip: vec![],
                                cname_target: Some(target.0.to_string()),
                                ttl
                            };
                            store_guard.insert(&domain, record);
                        }
                    } else {
                        if let Some(record) = store_guard.get(&domain) {
                            let form_record = FormDnsRecord {
                                record_type: rtype,
                                cname_target: Some(target.0.to_string()),
                                ttl,
                                ..record
                            };
                            store_guard.insert(&domain, form_record);
                            changed = true;
                        } else {
                            let record = FormDnsRecord {
                                domain: domain.clone(),
                                record_type: rtype,
                                formnet_ip: vec![],
                                public_ip: vec![],
                                cname_target: Some(target.0.to_string()),
                                ttl
                            };
                            store_guard.insert(&domain, record);
                            changed = true;
                        }
                    }
                }
                _ => {
                }
            }
        }

        Ok(changed)
    }
}

impl LookupObject for SimpleLookup {
    fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &'_ Record> + Send + '_> {
        Box::new(
            self.records.records_without_rrsigs()
        )
    }

    fn take_additionals(&mut self) -> Option<Box<dyn LookupObject>> {
        if let Some(adds) = self.additionals.take() {
            return Some(Box::new(SimpleLookup {
                records: adds,
                additionals: None,
            }))
        }
        None
    }
}

impl Authority for FormAuthority {
    type Lookup = SimpleLookup;

    fn zone_type(&self) -> ZoneType {
        self.zone_type
    }

    fn is_axfr_allowed(&self) -> bool {
        false
    }

    fn update<'life0,'life1,'async_trait>(&'life0 self,update: &'life1 MessageRequest) ->  ::core::pin::Pin<Box<dyn ::core::future::Future<Output = UpdateResult<bool> > + ::core::marker::Send+'async_trait> >where 'life0:'async_trait,'life1:'async_trait,Self:'async_trait {
        Box::pin(async move {
            match self.apply_update(update).await {
                Ok(changed) => Ok(changed),
                Err(rcode) => Err(rcode.into())

            }
        })
    }

    fn origin(&self) ->  &LowerName {
        &self.origin
    }

    fn lookup<'life0,'life1,'async_trait>(&'life0 self,name: &'life1 LowerName,rtype:RecordType,_lookup_options:LookupOptions,) ->  ::core::pin::Pin<Box<dyn ::core::future::Future<Output = std::result::Result<Self::Lookup,LookupError> > + ::core::marker::Send+'async_trait> >where 'life0:'async_trait,'life1:'async_trait,Self:'async_trait {
        Box::pin(async move {
            let name_str = name.to_string();
            if let Some(rrset) = self.lookup_local(&name_str, rtype, None).await {
                return Ok(SimpleLookup::from_record_set(rrset));
            }

            match self.lookup_fallback(name, rtype).await {
                Ok(rr) => Ok(SimpleLookup::from_record_set(rr)),
                Err(e) => Err(e),
            }
        })
    }

    fn search<'life0,'life1,'async_trait>(&'life0 self,request:trust_dns_server::server::RequestInfo<'life1> ,_lookup_options:LookupOptions,) ->  ::core::pin::Pin<Box<dyn ::core::future::Future<Output = std::result::Result<Self::Lookup,LookupError> > + ::core::marker::Send+'async_trait> >where 'life0:'async_trait,'life1:'async_trait,Self:'async_trait {
        Box::pin(async move {
            let src = request.src;
            let rtype = request.query.query_type();
            let name = request.query.name();
            if let Some(rrset) = self.lookup_local(&name.to_string(), rtype, Some(src.ip())).await {
                log::info!("Found record in local, returning...");
                return Ok(SimpleLookup::from_record_set(rrset));
            }

            log::info!("Unable to find record in checking fallback...");
            match self.lookup_fallback(name.into(), rtype).await {
                Ok(rr) => Ok(SimpleLookup::from_record_set(rr)),
                Err(e) => Err(e),
            }
        })
    }

    fn get_nsec_records<'life0,'life1,'async_trait>(&'life0 self,_name: &'life1 LowerName,_lookup_options:LookupOptions,) ->  ::core::pin::Pin<Box<dyn ::core::future::Future<Output = std::result::Result<Self::Lookup,LookupError> > + ::core::marker::Send+'async_trait> >where 'life0:'async_trait,'life1:'async_trait,Self:'async_trait {
        Box::pin(async move {
            Err(LookupError::ResponseCode(ResponseCode::NXDomain))
        })
    }
}
