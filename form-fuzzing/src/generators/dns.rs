// form-fuzzing/src/generators/dns.rs
//! Generators for DNS-related fuzzing

use crate::generators::Generator;
use rand::{Rng, distributions::Alphanumeric};
use std::iter;

/// Represents a DNS record for fuzzing
#[derive(Debug, Clone)]
pub struct DNSRecord {
    pub domain: String,
    pub record_type: DNSRecordType,
    pub ttl: u32,
    pub values: Vec<String>,
    pub priority: Option<u16>,
}

/// Common DNS record types
#[derive(Debug, Clone, PartialEq)]
pub enum DNSRecordType {
    A,
    AAAA,
    CNAME,
    MX,
    TXT,
    NS,
    SRV,
    PTR,
    CAA,
}

impl DNSRecordType {
    /// Convert record type to string
    pub fn as_str(&self) -> &'static str {
        match self {
            DNSRecordType::A => "A",
            DNSRecordType::AAAA => "AAAA",
            DNSRecordType::CNAME => "CNAME",
            DNSRecordType::MX => "MX",
            DNSRecordType::TXT => "TXT", 
            DNSRecordType::NS => "NS",
            DNSRecordType::SRV => "SRV",
            DNSRecordType::PTR => "PTR",
            DNSRecordType::CAA => "CAA",
        }
    }
    
    /// Get all record types
    pub fn all() -> Vec<DNSRecordType> {
        vec![
            DNSRecordType::A,
            DNSRecordType::AAAA,
            DNSRecordType::CNAME,
            DNSRecordType::MX,
            DNSRecordType::TXT,
            DNSRecordType::NS,
            DNSRecordType::SRV,
            DNSRecordType::PTR,
            DNSRecordType::CAA,
        ]
    }
}

/// Represents a DNS zone for fuzzing
#[derive(Debug, Clone)]
pub struct DNSZone {
    pub name: String,
    pub records: Vec<DNSRecord>,
    pub nameservers: Vec<String>,
    pub admin_email: String,
    pub refresh: u32,
    pub retry: u32,
    pub expire: u32,
    pub minimum_ttl: u32,
}

/// Generator for DNS records
pub struct DNSRecordGenerator {
    min_ttl: u32,
    max_ttl: u32,
    domains: Vec<String>,
}

impl DNSRecordGenerator {
    /// Create a new DNS record generator with default settings
    pub fn new() -> Self {
        Self {
            min_ttl: 60,           // 1 minute
            max_ttl: 86400 * 7,    // 1 week
            domains: vec![
                "example.com".to_string(),
                "test.domain".to_string(),
                "formation.network".to_string(),
                "fuzzing.test".to_string(),
                "subdomain.example.com".to_string(),
            ],
        }
    }
    
    /// Generate values for a specific record type
    fn generate_values_for_type(&self, record_type: &DNSRecordType) -> Vec<String> {
        match record_type {
            DNSRecordType::A => vec![generate_ipv4()],
            DNSRecordType::AAAA => vec![generate_ipv6()],
            DNSRecordType::CNAME => vec![format!("cname.{}", self.random_domain())],
            DNSRecordType::MX => vec![format!("mail.{}", self.random_domain())],
            DNSRecordType::TXT => vec![generate_txt_record()],
            DNSRecordType::NS => vec![format!("ns1.{}", self.random_domain()), format!("ns2.{}", self.random_domain())],
            DNSRecordType::SRV => vec![format!("srv.{}", self.random_domain())],
            DNSRecordType::PTR => vec![generate_reverse_dns()],
            DNSRecordType::CAA => vec!["0 issue \"letsencrypt.org\"".to_string()]
        }
    }
    
    /// Select a random domain from the list
    fn random_domain(&self) -> &String {
        let mut rng = rand::thread_rng();
        let index = rng.gen_range(0..self.domains.len());
        &self.domains[index]
    }
}

impl Generator<DNSRecord> for DNSRecordGenerator {
    fn generate(&self) -> DNSRecord {
        let mut rng = rand::thread_rng();
        
        // Select a random record type
        let record_types = DNSRecordType::all();
        let record_type_idx = rng.gen_range(0..record_types.len());
        let record_type = record_types[record_type_idx].clone();
        
        // Generate a TTL
        let ttl = rng.gen_range(self.min_ttl..=self.max_ttl);
        
        // Generate domain (use subdomain for variety)
        let domain = if rng.gen_bool(0.3) {
            format!("sub-{}.{}", generate_random_string(5), self.random_domain())
        } else {
            self.random_domain().clone()
        };
        
        // Generate values based on record type
        let values = self.generate_values_for_type(&record_type);
        
        // Generate priority for MX/SRV records
        let priority = if record_type == DNSRecordType::MX || record_type == DNSRecordType::SRV {
            Some(rng.gen_range(0..100))
        } else {
            None
        };
        
        DNSRecord {
            domain,
            record_type,
            ttl,
            values,
            priority,
        }
    }
}

/// Generator for DNS zones
pub struct DNSZoneGenerator {
    min_records: usize,
    max_records: usize,
    record_generator: DNSRecordGenerator,
}

impl DNSZoneGenerator {
    /// Create a new DNS zone generator with default settings
    pub fn new() -> Self {
        Self {
            min_records: 3,
            max_records: 20,
            record_generator: DNSRecordGenerator::new(),
        }
    }
}

impl Generator<DNSZone> for DNSZoneGenerator {
    fn generate(&self) -> DNSZone {
        let mut rng = rand::thread_rng();
        
        // Select a domain for the zone
        let zone_name = self.record_generator.random_domain().clone();
        
        // Generate records
        let record_count = rng.gen_range(self.min_records..=self.max_records);
        let mut records = Vec::with_capacity(record_count);
        
        for _ in 0..record_count {
            let record = self.record_generator.generate();
            records.push(record);
        }
        
        // Generate nameservers
        let nameservers = vec![
            format!("ns1.{}", zone_name),
            format!("ns2.{}", zone_name),
        ];
        
        // Generate admin email
        let admin_email = format!("admin@{}", zone_name);
        
        // Generate SOA parameters
        let refresh = 14400;    // 4 hours
        let retry = 3600;       // 1 hour
        let expire = 604800;    // 1 week
        let minimum_ttl = 3600; // 1 hour
        
        DNSZone {
            name: zone_name,
            records,
            nameservers,
            admin_email,
            refresh,
            retry,
            expire,
            minimum_ttl,
        }
    }
}

/// Generate a random IPv4 address
fn generate_ipv4() -> String {
    let mut rng = rand::thread_rng();
    format!(
        "{}.{}.{}.{}",
        rng.gen_range(1..=254),
        rng.gen_range(0..=255),
        rng.gen_range(0..=255),
        rng.gen_range(1..=254)
    )
}

/// Generate a random IPv6 address
fn generate_ipv6() -> String {
    let mut rng = rand::thread_rng();
    let segments: Vec<String> = (0..8)
        .map(|_| format!("{:04x}", rng.gen::<u16>()))
        .collect();
    segments.join(":")
}

/// Generate a random TXT record
fn generate_txt_record() -> String {
    let mut rng = rand::thread_rng();
    let length = rng.gen_range(10..200);
    
    let txt: String = iter::repeat(())
        .map(|()| rng.sample(Alphanumeric) as char)
        .take(length)
        .collect();
    
    format!("\"{}\"", txt)
}

/// Generate a random reverse DNS entry
fn generate_reverse_dns() -> String {
    let ip = generate_ipv4();
    let parts: Vec<&str> = ip.split('.').collect();
    
    format!("{}.{}.{}.{}.in-addr.arpa.",
        parts[3], parts[2], parts[1], parts[0])
}

/// Generate a random string of given length
fn generate_random_string(length: usize) -> String {
    let mut rng = rand::thread_rng();
    
    iter::repeat(())
        .map(|()| rng.sample(Alphanumeric) as char)
        .take(length)
        .collect()
} 