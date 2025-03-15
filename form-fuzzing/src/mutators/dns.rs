// form-fuzzing/src/mutators/dns.rs
//! Mutators for DNS-related fuzzing

use crate::generators::dns::{DNSRecord, DNSRecordType, DNSZone};
use crate::mutators::Mutator;
use rand::Rng;

/// Mutator for DNS records
pub struct DNSRecordMutator;

impl DNSRecordMutator {
    pub fn new() -> Self {
        Self
    }
}

impl Mutator<DNSRecord> for DNSRecordMutator {
    fn mutate(&self, input: &mut DNSRecord) {
        let mut rng = rand::thread_rng();
        
        // Choose a mutation strategy
        let mutation_type = rng.gen_range(0..5);
        
        match mutation_type {
            0 => {
                // Change TTL to an extreme value
                let extreme_ttls = [0, 1, 5, 10, 86400 * 365];
                input.ttl = extreme_ttls[rng.gen_range(0..extreme_ttls.len())];
            },
            1 => {
                // Change record type (keeping values the same)
                let new_type = match input.record_type {
                    DNSRecordType::A => DNSRecordType::AAAA,
                    DNSRecordType::AAAA => DNSRecordType::A,
                    DNSRecordType::CNAME => DNSRecordType::TXT,
                    DNSRecordType::MX => DNSRecordType::CNAME,
                    DNSRecordType::TXT => DNSRecordType::A,
                    DNSRecordType::NS => DNSRecordType::CNAME,
                    DNSRecordType::SRV => DNSRecordType::MX,
                    DNSRecordType::PTR => DNSRecordType::A,
                    DNSRecordType::CAA => DNSRecordType::TXT,
                };
                input.record_type = new_type;
            },
            2 => {
                // Modify the domain to invalid or extreme cases
                let domain_mutations = [
                    "".to_string(),
                    ".".to_string(),
                    ".com".to_string(),
                    "example.com.".to_string(),
                    "a".repeat(63) + ".com",  // Max label length
                    "a".repeat(64) + ".com",  // Exceeds max label length
                    "a".repeat(253),          // Max domain length
                    "a".repeat(254),          // Exceeds max domain length
                    "😊.example.com".to_string(), // Unicode emoji
                    "xn--h32b.example.com".to_string(), // Punycode
                    "@.example.com".to_string(),  // Invalid character
                    "sub.*.example.com".to_string(), // Wildcard in middle
                    "*.example.com".to_string(),     // Wildcard
                ];
                input.domain = domain_mutations[rng.gen_range(0..domain_mutations.len())].clone();
            },
            3 => {
                // Mutate values to invalid or extreme cases
                if !input.values.is_empty() {
                    let index = rng.gen_range(0..input.values.len());
                    
                    // Create invalid or edge case value based on record type
                    let value_mutation = match input.record_type {
                        DNSRecordType::A => {
                            let ip_mutations = [
                                "".to_string(),
                                "127.0.0.1".to_string(),
                                "0.0.0.0".to_string(),
                                "255.255.255.255".to_string(),
                                "999.999.999.999".to_string(),
                                "1.1.1.1.1".to_string(),
                                "a.b.c.d".to_string(),
                            ];
                            ip_mutations[rng.gen_range(0..ip_mutations.len())].clone()
                        },
                        DNSRecordType::AAAA => {
                            let ipv6_mutations = [
                                "".to_string(),
                                "::1".to_string(),
                                "::".to_string(),
                                ":::".to_string(),
                                "1::2::3".to_string(),
                                "xxxxx".to_string(),
                            ];
                            ipv6_mutations[rng.gen_range(0..ipv6_mutations.len())].clone()
                        },
                        _ => {
                            // General value mutations for other types
                            let general_mutations = [
                                "".to_string(),
                                ".".to_string(),
                                "a".repeat(300),  // Extremely long value
                                "null".to_string(),
                                "0".to_string(),
                                "-1".to_string(),
                                "127.0.0.1".to_string(),
                                "\"unterminated string".to_string(),
                            ];
                            general_mutations[rng.gen_range(0..general_mutations.len())].clone()
                        }
                    };
                    
                    input.values[index] = value_mutation;
                }
            },
            4 => {
                // Modify priority values for MX/SRV records
                if input.record_type == DNSRecordType::MX || input.record_type == DNSRecordType::SRV {
                    let priority_mutations = [None, Some(0), Some(65535)];
                    input.priority = priority_mutations[rng.gen_range(0..priority_mutations.len())];
                } else {
                    // Add priority to records that shouldn't have it
                    input.priority = Some(rng.gen_range(0..1000));
                }
            },
            _ => {} // No mutation
        }
    }
}

/// Mutator for DNS zones
pub struct DNSZoneMutator {
    record_mutator: DNSRecordMutator,
}

impl DNSZoneMutator {
    pub fn new() -> Self {
        Self {
            record_mutator: DNSRecordMutator::new(),
        }
    }
}

impl Mutator<DNSZone> for DNSZoneMutator {
    fn mutate(&self, input: &mut DNSZone) {
        let mut rng = rand::thread_rng();
        
        // Choose a mutation strategy
        let mutation_type = rng.gen_range(0..5);
        
        match mutation_type {
            0 => {
                // Mutate a random record
                if !input.records.is_empty() {
                    let record_idx = rng.gen_range(0..input.records.len());
                    self.record_mutator.mutate(&mut input.records[record_idx]);
                }
            },
            1 => {
                // Swap nameservers or modify them
                if !input.nameservers.is_empty() {
                    let ns_idx = rng.gen_range(0..input.nameservers.len());
                    
                    // Either remove the nameserver or replace it
                    if rng.gen_bool(0.5) {
                        input.nameservers.remove(ns_idx);
                    } else {
                        let ns_mutations = [
                            "".to_string(),
                            ".".to_string(),
                            "a.com".to_string(),
                            "example.com".to_string(),
                            "invalid-ns".to_string(),
                        ];
                        input.nameservers[ns_idx] = ns_mutations[rng.gen_range(0..ns_mutations.len())].clone();
                    }
                }
            },
            2 => {
                // Modify SOA parameters
                let soa_mutations = [
                    (0, 0, 0, 0),            // All zeros
                    (u32::MAX, u32::MAX, u32::MAX, u32::MAX), // Max values
                    (3600, 600, 86400, 60),  // Reasonable values
                    (0, 1, 2, 3),            // Small incremental values
                ];
                
                let soa_idx = rng.gen_range(0..soa_mutations.len());
                let (refresh, retry, expire, min_ttl) = soa_mutations[soa_idx];
                
                input.refresh = refresh;
                input.retry = retry;
                input.expire = expire;
                input.minimum_ttl = min_ttl;
            },
            3 => {
                // Modify admin email
                let email_mutations = [
                    "".to_string(),
                    "@".to_string(),
                    "invalid-email".to_string(),
                    "admin@".to_string(),
                    "@example.com".to_string(),
                    ".@example.com".to_string(),
                    "a".repeat(100) + "@example.com",
                ];
                
                input.admin_email = email_mutations[rng.gen_range(0..email_mutations.len())].clone();
            },
            4 => {
                // Modify zone name
                let zone_mutations = [
                    "".to_string(),
                    ".".to_string(),
                    ".com".to_string(),
                    "example.com.".to_string(),
                    "invalid..domain".to_string(),
                    "a".repeat(63) + ".com",  // Max label length
                    "a".repeat(253),          // Max domain length
                    "a".repeat(254),          // Exceeds max domain length
                ];
                
                input.name = zone_mutations[rng.gen_range(0..zone_mutations.len())].clone();
            },
            _ => {} // No mutation
        }
    }
} 