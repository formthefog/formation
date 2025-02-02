use std::collections::hash_map::{Entry, Iter};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::collections::HashMap;
use tokio::sync::{RwLock, mpsc::Sender};
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use trust_dns_proto::rr::RecordType;

use crate::resolvectl_dns;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FormDnsRecord {
    pub domain: String,
    pub record_type: RecordType,
    pub public_ip: Vec<SocketAddr>,
    pub formnet_ip: Vec<SocketAddr>,
    pub cname_target: Option<String>,
    pub ssl_cert: bool,
    pub ttl: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FormTarget {
    A(Vec<SocketAddr>),
    AAAA(Vec<SocketAddr>),
    CNAME(String),
    None
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct DnsStore {
    servers: Vec<Ipv4Addr>,
    records: HashMap<String, FormDnsRecord>,
    #[serde(skip)]
    sender: Option<Sender<FormDnsRecord>>,
}

impl DnsStore {
    pub fn new(sender: Sender<FormDnsRecord>) -> Self {
        Self {
            servers: Vec::new(),
            records: HashMap::new(),
            sender: Some(sender),
        }
    }

    pub fn add_server(&mut self, server: Ipv4Addr) -> Result<(), Box<dyn std::error::Error>> {
        self.servers.push(server);
        let all_servers = self.servers.clone();
        resolvectl_dns(all_servers)?;
        Ok(())
    }

    pub async fn insert(
        &mut self,
        domain: &str,
        record: FormDnsRecord
    ) {
        let key = domain.trim_end_matches('.').to_lowercase();
        if let Some(ref mut sender) = &mut self.sender {
            let _ = sender.send(record.clone()).await;
        }
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
