use std::collections::hash_map::{Entry, Iter};
use std::net::IpAddr;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use serde::{Serialize, Deserialize};
use trust_dns_proto::rr::RecordType;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FormDnsRecord {
    pub domain: String,
    pub record_type: RecordType,
    pub public_ip: Vec<IpAddr>,
    pub formnet_ip: Vec<IpAddr>,
    pub cname_target: Option<String>,
    pub ttl: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FormTarget {
    A(Vec<IpAddr>),
    AAAA(Vec<IpAddr>),
    CNAME(String),
    None
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct DnsStore {
    records: HashMap<String, FormDnsRecord>
}

impl DnsStore {
    pub fn new() -> Self {
        Self {
            records: HashMap::new()
        }
    }

    pub fn insert(
        &mut self,
        domain: &str,
        record: FormDnsRecord
    ) {
        let key = domain.trim_end_matches('.').to_lowercase();
        self.records.insert(key, record);
    }

    pub fn lookup(&self, domain: &str, src: IpAddr) -> FormTarget {
        let key = domain.trim_end_matches('.').to_lowercase(); 
        let record = self.records.get(&key);
        if let Some(rec) = record {
            match rec.record_type {
                RecordType::A => {
                    match src {
                        IpAddr::V4(addr) => {
                            if addr.octets()[0] == 10 {
                                if !rec.formnet_ip.is_empty() {
                                    return FormTarget::A(rec.formnet_ip.clone())
                                } else if !rec.public_ip.is_empty() {
                                    return FormTarget::A(rec.public_ip.clone())
                                }
                            } else if !rec.public_ip.is_empty() {
                                return FormTarget::A(rec.public_ip.clone())
                            }
                        }
                        IpAddr::V6(_) => if !rec.public_ip.is_empty() {
                            return FormTarget::A(rec.public_ip.clone())
                        }
                    }
                }
                RecordType::CNAME => {
                    if let Some(ct) = &rec.cname_target {
                        return FormTarget::CNAME(ct.to_string())
                    }
                }
                RecordType::AAAA => {
                    match src {
                        IpAddr::V4(addr) => {
                            if addr.octets()[0] == 10 {
                                if !rec.formnet_ip.is_empty() {
                                    let mut ips = rec.formnet_ip.clone();
                                    if !rec.public_ip.is_empty(){
                                        ips.extend(rec.public_ip.clone());
                                    }
                                    return FormTarget::AAAA(ips)
                                } else if !rec.public_ip.is_empty() {
                                    return FormTarget::AAAA(rec.public_ip.clone())
                                }
                            } else {
                                if !rec.public_ip.is_empty() {
                                    return FormTarget::AAAA(rec.public_ip.clone())
                                }
                            }
                        }
                        IpAddr::V6(_) => {
                            if !rec.public_ip.is_empty() {
                                return FormTarget::AAAA(rec.public_ip.clone())
                            }
                        }
                    }
                }
                _ => return FormTarget::None
            }
        }
        FormTarget::None
    }

    pub fn get(&self, domain: &str) -> Option<FormDnsRecord> {
        self.records.get(domain).cloned()
    }

    pub fn remove(&mut self, domain: &str) -> Option<FormDnsRecord> {
        self.records.remove(domain)
    }

    pub fn entry(&mut self, domain: &str) -> Entry<'_, String, FormDnsRecord> {
        self.records.entry(domain.to_string())
    }

    pub fn iter(&self) -> Iter<String, FormDnsRecord> {
        self.records.iter()
    }
}

pub type SharedStore = Arc<RwLock<DnsStore>>;
