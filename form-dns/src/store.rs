use std::collections::hash_map::{Entry, Iter};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::collections::HashMap;
use tokio::sync::{RwLock, mpsc::Sender};
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use trust_dns_proto::rr::RecordType;
use trust_dns_proto::rr::{Name, RData};
use trust_dns_client::client::{AsyncClient, ClientHandle};
use trust_dns_client::udp::UdpClientStream;
use trust_dns_client::rr::DNSClass;
use std::str::FromStr;
use std::time::Duration;

use crate::resolvectl_dns;
use crate::health::SharedIpHealthRepository;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FormDnsRecord {
    pub domain: String,
    pub record_type: RecordType,
    pub public_ip: Vec<SocketAddr>,
    pub formnet_ip: Vec<SocketAddr>,
    pub cname_target: Option<String>,
    pub ssl_cert: bool,
    pub ttl: u32,
    pub verification_status: Option<VerificationStatus>,
    pub verification_timestamp: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum VerificationStatus {
    Pending,
    Verified,
    Failed(String),
    NotVerified,
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
    #[serde(skip)]
    health_repository: Option<SharedIpHealthRepository>,
}

impl DnsStore {
    pub fn new(sender: Sender<FormDnsRecord>) -> Self {
        Self {
            servers: Vec::new(),
            records: HashMap::new(),
            sender: Some(sender),
            health_repository: None,
        }
    }

    pub fn with_health_repository(mut self, health_repository: SharedIpHealthRepository) -> Self {
        self.health_repository = Some(health_repository);
        self
    }

    pub fn get_health_repository(&self) -> Option<SharedIpHealthRepository> {
        self.health_repository.clone()
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

    /// Initiates the verification process for a domain
    /// Returns instructions if verification cannot be completed immediately
    pub async fn initiate_verification(&mut self, domain: &str) -> Result<VerificationResult, String> {
        let key = domain.trim_end_matches('.').to_lowercase();
        
        // Check if domain exists in our records
        if let Some(mut record) = self.records.get(&key).cloned() {
            // Set verification status to pending
            record.verification_status = Some(VerificationStatus::Pending);
            record.verification_timestamp = Some(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs()));
            
            // Update record in store
            self.records.insert(key.clone(), record.clone());
            
            // Perform verification check
            match self.check_domain_points_to_network(&domain).await {
                Ok(true) => {
                    // Domain already points to our network
                    let mut verified_record = record;
                    verified_record.verification_status = Some(VerificationStatus::Verified);
                    verified_record.verification_timestamp = Some(std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_or(0, |d| d.as_secs()));
                    
                    // Update record in store
                    self.records.insert(key, verified_record.clone());
                    
                    Ok(VerificationResult::Verified(verified_record))
                },
                Ok(false) => {
                    // Domain doesn't point to our network, return instructions
                    let required_configs = self.get_required_configs(&record);
                    Ok(VerificationResult::RequiresConfig(required_configs))
                },
                Err(e) => {
                    // Verification check failed
                    let mut failed_record = record;
                    failed_record.verification_status = Some(VerificationStatus::Failed(e.clone()));
                    
                    // Update record in store
                    self.records.insert(key, failed_record.clone());
                    
                    Err(e)
                }
            }
        } else {
            Err("Domain not found in records".to_string())
        }
    }

    /// Checks if a domain's DNS records already point to our network
    async fn check_domain_points_to_network(&self, domain: &str) -> Result<bool, String> {
        // Use Google's public DNS server for external lookups
        let google_dns = SocketAddr::from(([8, 8, 8, 8], 53));
        
        // Setup the UDP client connection
        let stream = UdpClientStream::<tokio::net::UdpSocket>::with_timeout(google_dns, Duration::from_secs(5));
        
        // Connect client
        let (mut client, background) = AsyncClient::connect(stream)
            .await
            .map_err(|e| format!("Failed to create DNS client: {}", e))?;
            
        // Spawn background task to handle DNS responses
        tokio::spawn(background);
        
        // Parse domain name
        let name = Name::from_str(domain)
            .map_err(|e| format!("Invalid domain name: {}", e))?;
        
        // Look up A records
        let response = client.query(name.clone(), DNSClass::IN, RecordType::A)
            .await
            .map_err(|e| format!("DNS query failed: {}", e))?;
        
        // Check if any A records point to our network IPs
        for record in response.answers() {
            if let Some(RData::A(ip_addr)) = record.data() {
                // Create a socket address from the IPv4 address
                let socket_addr = SocketAddr::new(IpAddr::V4(**ip_addr), 80);
                
                // Check if this IP matches any of our network nodes' public IPs
                for (_, record) in self.records.iter() {
                    for public_ip in &record.public_ip {
                        if public_ip.ip() == socket_addr.ip() {
                            return Ok(true);
                        }
                    }
                }
            }
        }
        
        // Look up CNAME records if no A record match was found
        let response = client.query(name, DNSClass::IN, RecordType::CNAME)
            .await
            .map_err(|e| format!("DNS query failed: {}", e))?;
        
        // Check if any CNAME records point to our domains
        for record in response.answers() {
            if let Some(RData::CNAME(cname)) = record.data() {
                let cname_str = cname.to_string();
                
                // Check if this CNAME points to any of our domains
                for (domain, _) in self.records.iter() {
                    if cname_str.ends_with(domain) {
                        return Ok(true);
                    }
                }
            }
        }
        
        // No matches found
        Ok(false)
    }
    
    /// Gets the required DNS configurations for a domain to be verified
    fn get_required_configs(&self, record: &FormDnsRecord) -> DnsConfiguration {
        match record.record_type {
            RecordType::A => {
                // Get the first public IP for this record
                let target_ip = record.public_ip.first().map(|addr| addr.ip().to_string());
                
                DnsConfiguration {
                    record_type: "A".to_string(),
                    target: target_ip.unwrap_or_default(),
                    ttl: record.ttl,
                }
            },
            RecordType::CNAME => {
                DnsConfiguration {
                    record_type: "CNAME".to_string(),
                    target: record.cname_target.clone().unwrap_or_default(),
                    ttl: record.ttl,
                }
            },
            _ => DnsConfiguration {
                record_type: record.record_type.to_string(),
                target: "".to_string(),
                ttl: record.ttl,
            }
        }
    }

    /// Check verification status of a domain
    pub async fn check_verification(&mut self, domain: &str) -> Result<VerificationResult, String> {
        let key = domain.trim_end_matches('.').to_lowercase();
        
        // Check if domain exists in our records
        if let Some(record) = self.records.get(&key).cloned() {
            // Perform verification check
            match self.check_domain_points_to_network(&domain).await {
                Ok(true) => {
                    // Domain points to our network, update verification status
                    let mut verified_record = record.clone();
                    verified_record.verification_status = Some(VerificationStatus::Verified);
                    verified_record.verification_timestamp = Some(std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_or(0, |d| d.as_secs()));
                    
                    // Update record in store
                    self.records.insert(key, verified_record.clone());
                    
                    Ok(VerificationResult::Verified(verified_record))
                },
                Ok(false) => {
                    // Domain doesn't point to our network
                    let mut failed_record = record.clone();
                    failed_record.verification_status = Some(VerificationStatus::Failed(
                        "Domain does not point to our network".to_string()
                    ));
                    
                    // Update record in store
                    self.records.insert(key, failed_record);
                    
                    let required_configs = self.get_required_configs(&record);
                    Ok(VerificationResult::RequiresConfig(required_configs))
                },
                Err(e) => {
                    // Verification check failed
                    let mut failed_record = record.clone();
                    failed_record.verification_status = Some(VerificationStatus::Failed(e.clone()));
                    
                    // Update record in store
                    self.records.insert(key, failed_record);
                    
                    Err(e)
                }
            }
        } else {
            Err("Domain not found in records".to_string())
        }
    }
}

/// Result of domain verification attempts
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum VerificationResult {
    /// Domain is verified
    Verified(FormDnsRecord),
    /// Domain requires configuration
    RequiresConfig(DnsConfiguration),
}

/// DNS configuration required for verification
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DnsConfiguration {
    pub record_type: String,
    pub target: String,
    pub ttl: u32,
}

pub type SharedStore = Arc<RwLock<DnsStore>>;
