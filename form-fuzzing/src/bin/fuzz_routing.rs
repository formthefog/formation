// form-fuzzing/src/bin/fuzz_routing.rs
//! BGP/Anycast Routing Fuzzer
//!
//! This fuzzer tests the BGP/Anycast routing components of the Formation Network,
//! including:
//! - BGP announcements and withdrawals
//! - DNS resolution with GeoDNS support
//! - Health tracking for IP addresses
//! - Anycast routing functionality
//! - Edge cases and error handling
//!
//! The fuzzer uses a combination of valid and invalid inputs to test the robustness
//! of the routing system and its ability to handle various geographic locations,
//! network conditions, and failures.

use form_fuzzing::generators::Generator;
use form_fuzzing::mutators::Mutator;
use form_fuzzing::generators::routing::{
    Region, RegionalIpGenerator, BgpAnnouncementGenerator, HealthStatusGenerator,
    GeoDnsRequestGenerator, AnycastTestGenerator,
    IpAddressInfo, GeoDnsRequest, HealthStatusReport, BgpAnnouncement, AnycastTest
};
use form_fuzzing::harness::routing::{RoutingHarness, RoutingOperationResult};
use form_fuzzing::instrumentation::coverage;
use form_fuzzing::instrumentation::fault_injection;
use form_fuzzing::instrumentation::sanitizer;
use form_fuzzing::mutators::routing::{
    IpAddressMutator, GeoDnsRequestMutator, HealthStatusMutator, BgpAnnouncementMutator,
    AnycastTestMutator,
};

use std::env;
use std::fs::{self, create_dir_all};
use std::net::{IpAddr, Ipv4Addr};
use std::path::Path;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::fmt::Debug;
use rand::{Rng, seq::SliceRandom, thread_rng};
use uuid::Uuid;

/// Default maximum number of iterations
const DEFAULT_MAX_ITERATIONS: usize = 1000;

/// Default corpus directory
const DEFAULT_CORPUS_DIR: &str = "/tmp/form-fuzzing/routing";

/// Main function
fn main() {
    println!("Starting BGP/Anycast Routing Fuzzer");
    
    // Initialize coverage tracking
    coverage::init();
    
    // Initialize fault injection
    fault_injection::init();
    
    // Initialize sanitizer
    sanitizer::init();
    
    // Parse command line args
    let max_iterations = env::var("FUZZ_MAX_ITERATIONS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(DEFAULT_MAX_ITERATIONS);
    
    let corpus_dir = env::var("FUZZ_CORPUS_DIR")
        .unwrap_or_else(|_| DEFAULT_CORPUS_DIR.to_string());
    
    // Ensure corpus directory exists
    if let Err(e) = create_dir_all(&corpus_dir) {
        eprintln!("Warning: Failed to create corpus directory: {}", e);
    }
    
    // Initialize generators
    let ip_gen = RegionalIpGenerator::new(Region::random())
        .with_ipv6(thread_rng().gen_bool(0.5))
        .with_healthy_percentage(0.8);
    
    let bgp_gen = BgpAnnouncementGenerator::new()
        .with_multiple_prefixes(true)
        .with_communities(true);
    
    let dns_gen = GeoDnsRequestGenerator::new()
        .with_client_ip(true)
        .with_coordinates(true)
        .with_ecs(true);
    
    let health_gen = HealthStatusGenerator::new()
        .with_node_range(5, 20)
        .with_healthy_percentage(0.7);
    
    let anycast_gen = AnycastTestGenerator::new()
        .with_request_range(5, 15);
    
    // Initialize mutators
    let ip_mutator = IpAddressMutator::new();
    let bgp_mutator = BgpAnnouncementMutator::new();
    let dns_mutator = GeoDnsRequestMutator::new();
    let health_mutator = HealthStatusMutator::new();
    let anycast_mutator = AnycastTestMutator::new();
    
    // Initialize harness
    let mut harness = RoutingHarness::new();
    
    // Initialize stats
    let mut stats = FuzzingStats::new();
    
    println!("Running for {} iterations", max_iterations);
    
    // Main fuzzing loop
    let start_time = Instant::now();
    
    for i in 0..max_iterations {
        // Occasionally show progress
        if i % 100 == 0 && i > 0 {
            let elapsed = start_time.elapsed().as_secs_f64();
            println!("Progress: {}/{} iterations ({:.2} iter/sec)", 
                     i, max_iterations, i as f64 / elapsed);
        }
        
        // Choose a random strategy for this iteration
        let strategy = match thread_rng().gen_range(0..10) {
            0 => FuzzingStrategy::ValidBgpAnnouncement,
            1 => FuzzingStrategy::InvalidBgpAnnouncement,
            2 => FuzzingStrategy::ValidDnsRequest,
            3 => FuzzingStrategy::InvalidDnsRequest,
            4 => FuzzingStrategy::ValidHealthReport,
            5 => FuzzingStrategy::InvalidHealthReport,
            6 => FuzzingStrategy::AnycastTest,
            7 => FuzzingStrategy::MixedOperations,
            8 => FuzzingStrategy::MutateBgpAnnouncement,
            _ => FuzzingStrategy::MutateDnsAndHealth,
        };
        
        // Execute the chosen strategy
        match strategy {
            FuzzingStrategy::ValidBgpAnnouncement => {
                // Generate a valid BGP announcement
                let announcement = bgp_gen.generate();
                
                // Process the announcement
                match harness.process_bgp_announcement(&announcement) {
                    Ok(result) => {
                        stats.successful_bgp += 1;
                        stats.bgp_announcements += 1;
                        
                        // Save interesting results to corpus
                        if result.propagated_to > 5 {
                            save_to_corpus_debug(&announcement, "bgp_valid", &corpus_dir);
                        }
                    },
                    Err(e) => {
                        stats.failed_bgp += 1;
                        stats.bgp_announcements += 1;
                        
                        // This is unexpected for valid announcements
                        if !matches!(e, RoutingOperationResult::RateLimited) {
                            println!("Valid BGP announcement failed: {:?}", e);
                            save_to_corpus_debug(&announcement, "bgp_valid_failed", &corpus_dir);
                        }
                    },
                }
                
                // Occasionally also test withdrawals
                if thread_rng().gen_bool(0.2) {
                    // Create a withdrawal for one of the prefixes
                    let mut withdrawal = announcement.clone();
                    withdrawal.is_withdrawal = true;
                    
                    // If the announcement had multiple prefixes, only withdraw some
                    if withdrawal.prefixes.len() > 1 {
                        let keep_count = thread_rng().gen_range(1..withdrawal.prefixes.len());
                        withdrawal.prefixes.truncate(keep_count);
                    }
                    
                    match harness.process_bgp_announcement(&withdrawal) {
                        Ok(_) => stats.successful_bgp += 1,
                        Err(_) => stats.failed_bgp += 1,
                    }
                    stats.bgp_announcements += 1;
                }
            },
            FuzzingStrategy::InvalidBgpAnnouncement => {
                // Generate a BGP announcement then make it invalid
                let mut announcement = bgp_gen.generate();
                
                // Now make it invalid in some way
                let invalid_type = thread_rng().gen_range(0..5);
                match invalid_type {
                    0 => {
                        // Empty AS path
                        announcement.as_path.clear();
                    },
                    1 => {
                        // Invalid next hop
                        announcement.next_hop = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
                    },
                    2 => {
                        // Extremely long AS path
                        announcement.as_path = (0..1000).map(|_| thread_rng().gen()).collect();
                    },
                    3 => {
                        // Invalid prefix length
                        if !announcement.prefixes.is_empty() {
                            announcement.prefixes[0].1 = match announcement.prefixes[0].0 {
                                IpAddr::V4(_) => 33, // Invalid for IPv4
                                IpAddr::V6(_) => 129, // Invalid for IPv6
                            };
                        }
                    },
                    _ => {
                        // Empty prefixes
                        announcement.prefixes.clear();
                    },
                }
                
                // Process the invalid announcement - should fail
                match harness.process_bgp_announcement(&announcement) {
                    Ok(_) => {
                        stats.unexpected_success += 1;
                        stats.successful_bgp += 1;
                        
                        // This is unexpected since it should have failed
                        println!("Invalid BGP announcement was accepted!");
                        save_to_corpus_debug(&announcement, "bgp_invalid_accepted", &corpus_dir);
                    },
                    Err(_) => {
                        stats.expected_failures += 1;
                        stats.failed_bgp += 1;
                    },
                }
                stats.bgp_announcements += 1;
            },
            FuzzingStrategy::ValidDnsRequest => {
                // First, add some DNS records to query
                let domain = generate_domain();
                let record_count = thread_rng().gen_range(1..8);
                let mut added_records = 0;
                
                // Add different records in different regions
                for _ in 0..record_count {
                    let region = Region::random();
                    let ip_info = ip_gen.generate();
                    
                    match harness.add_dns_record(&domain, ip_info) {
                        RoutingOperationResult::Success => added_records += 1,
                        _ => {}
                    }
                }
                
                // Generate a valid DNS request
                let request = dns_gen.generate();
                
                // Process the request
                match harness.resolve_dns(&request) {
                    Ok(result) => {
                        stats.successful_dns += 1;
                        stats.dns_requests += 1;
                        
                        // Save interesting results to corpus
                        if result.addresses.len() > 2 {
                            save_to_corpus_debug(&request, "dns_valid", &corpus_dir);
                        }
                    },
                    Err(e) => {
                        stats.failed_dns += 1;
                        stats.dns_requests += 1;
                        
                        // This may be expected if we're requesting a domain without records
                        if let RoutingOperationResult::DomainNotFound = e {
                            // Expected
                        } else {
                            println!("Valid DNS request failed: {:?}", e);
                            save_to_corpus_debug(&request, "dns_valid_failed", &corpus_dir);
                        }
                    },
                }
            },
            FuzzingStrategy::InvalidDnsRequest => {
                // Generate a DNS request then make it invalid
                let mut request = dns_gen.generate();
                
                // Make it invalid in some way
                let invalid_type = thread_rng().gen_range(0..3);
                match invalid_type {
                    0 => {
                        // Invalid domain
                        request.domain = ".invalid-domain.".to_string();
                    },
                    1 => {
                        // Very long domain
                        request.domain = (0..1000).map(|_| "a").collect::<String>() + ".com";
                    },
                    _ => {
                        // Invalid ECS prefix
                        if request.ecs_prefix.is_some() {
                            request.ecs_prefix = Some(129); // Invalid prefix
                        }
                    },
                }
                
                // Process the invalid request - should fail
                match harness.resolve_dns(&request) {
                    Ok(_) => {
                        stats.unexpected_success += 1;
                        stats.successful_dns += 1;
                        
                        println!("Invalid DNS request was accepted!");
                        save_to_corpus_debug(&request, "dns_invalid_accepted", &corpus_dir);
                    },
                    Err(_) => {
                        stats.expected_failures += 1;
                        stats.failed_dns += 1;
                    },
                }
                stats.dns_requests += 1;
            },
            FuzzingStrategy::ValidHealthReport => {
                // Generate a valid health report
                let report = health_gen.generate();
                
                // Process the health report
                match harness.update_health(&report) {
                    RoutingOperationResult::Success => {
                        stats.successful_health += 1;
                        
                        // Save successful reports to corpus
                        if report.nodes.len() > 10 {
                            save_to_corpus_debug(&report, "health_valid", &corpus_dir);
                        }
                    },
                    _ => {
                        stats.failed_health += 1;
                        save_to_corpus_debug(&report, "health_valid_failed", &corpus_dir);
                    },
                }
                stats.health_reports += 1;
            },
            FuzzingStrategy::InvalidHealthReport => {
                // Generate a health report then make it invalid
                let mut report = health_gen.generate();
                
                // Make it invalid in some way
                let invalid_type = thread_rng().gen_range(0..3);
                match invalid_type {
                    0 => {
                        // Empty report
                        report.nodes.clear();
                    },
                    1 => {
                        // Invalid health values
                        for (_, health) in report.nodes.iter_mut() {
                            health.health = 2.0; // Health should be 0.0-1.0
                        }
                    },
                    _ => {
                        // Timestamp in the future
                        report.timestamp = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() + 10000000;
                    },
                }
                
                // Process the report - should fail
                match harness.update_health(&report) {
                    RoutingOperationResult::Success => {
                        stats.unexpected_success += 1;
                        stats.successful_health += 1;
                        
                        // This is unexpected since it should have failed
                        println!("Invalid health report was accepted!");
                        save_to_corpus_debug(&report, "health_invalid_accepted", &corpus_dir);
                    },
                    _ => {
                        stats.expected_failures += 1;
                        stats.failed_health += 1;
                    },
                }
                stats.health_reports += 1;
            },
            FuzzingStrategy::AnycastTest => {
                // Ensure we have some DNS records first
                let domain = generate_domain();
                let record_count = thread_rng().gen_range(5..10);
                
                for _ in 0..record_count {
                    let ip_info = ip_gen.generate();
                    // Ignore errors, just trying to add some records
                    let _ = harness.add_dns_record(&domain, ip_info);
                }
                
                // Generate an anycast test
                let test = anycast_gen.generate();
                
                // Run the test
                match harness.run_anycast_test(&test) {
                    Ok(results) => {
                        stats.successful_anycast += 1;
                        
                        // Verify results
                        let verified = harness.verify_anycast_test(&test.test_id, &results);
                        if verified {
                            // Good result
                        } else {
                            println!("Anycast test results did not match expectations");
                        }
                        
                        // Save to corpus
                        save_to_corpus_debug(&test, "anycast_test", &corpus_dir);
                    },
                    Err(_) => {
                        stats.failed_anycast += 1;
                    },
                }
                stats.anycast_tests += 1;
            },
            FuzzingStrategy::MixedOperations => {
                // Perform a random mix of operations
                let operation_count = thread_rng().gen_range(5..15);
                
                for _ in 0..operation_count {
                    match thread_rng().gen_range(0..4) {
                        0 => {
                            // BGP announcement
                            let announcement = bgp_gen.generate();
                            match harness.process_bgp_announcement(&announcement) {
                                Ok(_) => stats.successful_bgp += 1,
                                Err(_) => stats.failed_bgp += 1,
                            }
                            stats.bgp_announcements += 1;
                        },
                        1 => {
                            // Health report
                            let report = health_gen.generate();
                            match harness.update_health(&report) {
                                RoutingOperationResult::Success => stats.successful_health += 1,
                                _ => stats.failed_health += 1,
                            }
                            stats.health_reports += 1;
                        },
                        2 => {
                            // DNS request
                            let domain = generate_domain();
                            let ip_info = ip_gen.generate();
                            // Ignore errors, just trying to add a record
                            let _ = harness.add_dns_record(&domain, ip_info);
                            
                            let request = dns_gen.generate();
                            match harness.resolve_dns(&request) {
                                Ok(_) => stats.successful_dns += 1,
                                Err(_) => stats.failed_dns += 1,
                            }
                            stats.dns_requests += 1;
                        },
                        _ => {
                            // Anycast test
                            let test = anycast_gen.generate();
                            match harness.run_anycast_test(&test) {
                                Ok(_) => stats.successful_anycast += 1,
                                Err(_) => stats.failed_anycast += 1,
                            }
                            stats.anycast_tests += 1;
                        },
                    }
                }
            },
            FuzzingStrategy::MutateBgpAnnouncement => {
                // Generate a BGP announcement and then mutate it several times
                let mut announcement = bgp_gen.generate();
                
                // Apply multiple mutations
                let mutation_count = thread_rng().gen_range(1..5);
                
                for _ in 0..mutation_count {
                    // Mutate the announcement
                    bgp_mutator.mutate(&mut announcement);
                }
                
                // Process the mutated announcement
                match harness.process_bgp_announcement(&announcement) {
                    Ok(_) => stats.successful_bgp += 1,
                    Err(_) => stats.failed_bgp += 1,
                }
                stats.bgp_announcements += 1;
            },
            FuzzingStrategy::MutateDnsAndHealth => {
                // Mutate both DNS requests and health reports
                {
                    // DNS request
                    let mut request = dns_gen.generate();
                    
                    // Apply mutations
                    let mutation_count = thread_rng().gen_range(1..3);
                    
                    for _ in 0..mutation_count {
                        dns_mutator.mutate(&mut request);
                    }
                    
                    // Process the mutated request
                    match harness.resolve_dns(&request) {
                        Ok(_) => stats.successful_dns += 1,
                        Err(_) => stats.failed_dns += 1,
                    }
                    stats.dns_requests += 1;
                }
                
                {
                    // Health report
                    let mut report = health_gen.generate();
                    
                    // Apply mutations
                    let mutation_count = thread_rng().gen_range(1..3);
                    
                    for _ in 0..mutation_count {
                        health_mutator.mutate(&mut report);
                    }
                    
                    // Process the mutated report
                    match harness.update_health(&report) {
                        RoutingOperationResult::Success => stats.successful_health += 1,
                        _ => stats.failed_health += 1,
                    }
                    stats.health_reports += 1;
                }
            },
        }
    }
    
    // Print stats
    let elapsed = start_time.elapsed();
    println!("\nFuzzing completed in {:.2} seconds", elapsed.as_secs_f64());
    stats.print();
    
    // Coverage information
    let coverage_count = coverage::get_coverage_count();
    println!("\nCoverage Information:");
    println!("  Code blocks covered: {}", coverage_count);
    
    // Save results
    println!("\nSaving results to {}", corpus_dir);
}

/// Save an interesting input to the corpus using debug formatting
fn save_to_corpus_debug<T: Debug>(
    item: &T,
    prefix: &str,
    corpus_dir: &str
) {
    // Use debug formatting instead of serialization
    let debug_str = format!("{:#?}", item);
    let filename = format!("{}/{}-{}.txt", corpus_dir, prefix, Uuid::new_v4());
    if let Err(e) = fs::write(&filename, debug_str) {
        println!("Warning: Failed to write to corpus: {}", e);
    }
}

/// Generate a random domain name
fn generate_domain() -> String {
    let mut rng = thread_rng();
    
    // Random length for subdomain
    let len = rng.gen_range(3..15);
    
    // Generate random alphanumeric string
    let subdomain: String = (0..len)
        .map(|_| {
            let ch = rng.gen_range(0..36);
            if ch < 10 {
                // digit
                (b'0' + ch) as char
            } else {
                // lowercase letter
                (b'a' + ch - 10) as char
            }
        })
        .collect();
    
    let tlds = [".com", ".net", ".org", ".io", ".dev"];
    let tld = tlds.choose(&mut rng).unwrap();
    
    format!("{}{}", subdomain, tld)
}

/// Fuzzing strategies
enum FuzzingStrategy {
    /// Test valid BGP announcements
    ValidBgpAnnouncement,
    /// Test invalid BGP announcements
    InvalidBgpAnnouncement,
    /// Test valid DNS requests
    ValidDnsRequest,
    /// Test invalid DNS requests
    InvalidDnsRequest,
    /// Test valid health reports
    ValidHealthReport,
    /// Test invalid health reports
    InvalidHealthReport,
    /// Test anycast functionality
    AnycastTest,
    /// Test mixed operations
    MixedOperations,
    /// Test mutated BGP announcements
    MutateBgpAnnouncement,
    /// Test mutated DNS requests and health reports
    MutateDnsAndHealth,
}

/// Fuzzing statistics
struct FuzzingStats {
    /// Number of BGP announcements processed
    bgp_announcements: usize,
    /// Number of successful BGP operations
    successful_bgp: usize,
    /// Number of failed BGP operations
    failed_bgp: usize,
    
    /// Number of DNS requests processed
    dns_requests: usize,
    /// Number of successful DNS operations
    successful_dns: usize,
    /// Number of failed DNS operations
    failed_dns: usize,
    
    /// Number of health reports processed
    health_reports: usize,
    /// Number of successful health operations
    successful_health: usize,
    /// Number of failed health operations
    failed_health: usize,
    
    /// Number of anycast tests processed
    anycast_tests: usize,
    /// Number of successful anycast operations
    successful_anycast: usize,
    /// Number of failed anycast operations
    failed_anycast: usize,
    
    /// Number of cases where invalid input was unexpectedly accepted
    unexpected_success: usize,
    /// Number of cases where invalid input failed as expected
    expected_failures: usize,
}

impl FuzzingStats {
    /// Create new stats
    fn new() -> Self {
        Self {
            bgp_announcements: 0,
            successful_bgp: 0,
            failed_bgp: 0,
            
            dns_requests: 0,
            successful_dns: 0,
            failed_dns: 0,
            
            health_reports: 0,
            successful_health: 0,
            failed_health: 0,
            
            anycast_tests: 0,
            successful_anycast: 0,
            failed_anycast: 0,
            
            unexpected_success: 0,
            expected_failures: 0,
        }
    }
    
    /// Print stats
    fn print(&self) {
        println!("=== Routing Fuzzing Statistics ===");
        println!("BGP Announcements: {} (Success: {}, Failed: {})", 
                 self.bgp_announcements, self.successful_bgp, self.failed_bgp);
        
        println!("DNS Requests: {} (Success: {}, Failed: {})", 
                 self.dns_requests, self.successful_dns, self.failed_dns);
        
        println!("Health Reports: {} (Success: {}, Failed: {})", 
                 self.health_reports, self.successful_health, self.failed_health);
        
        println!("Anycast Tests: {} (Success: {}, Failed: {})", 
                 self.anycast_tests, self.successful_anycast, self.failed_anycast);
        
        println!("Invalid inputs accepted: {}", self.unexpected_success);
        println!("Invalid inputs properly rejected: {}", self.expected_failures);
    }
} 