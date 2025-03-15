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

use form_fuzzing::generators::routing::{
    Region, IpAddressGenerator, BgpAnnouncementGenerator, HealthStatusReportGenerator,
    GeoDnsRequestGenerator, AnycastTestGenerator, InvalidBgpAnnouncementGenerator,
    InvalidGeoDnsRequestGenerator, InvalidHealthStatusReportGenerator, InvalidIpAddressGenerator,
};
use form_fuzzing::harness::routing::RoutingHarness;
use form_fuzzing::instrumentation::coverage;
use form_fuzzing::instrumentation::fault_injection;
use form_fuzzing::instrumentation::sanitizer;
use form_fuzzing::mutators::routing::{
    IpAddressMutator, GeoDnsRequestMutator, HealthStatusMutator, BgpAnnouncementMutator,
    AnycastTestMutator,
};

use std::env;
use std::fs::{self, create_dir_all};
use std::net::IpAddr;
use std::path::Path;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
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
    coverage::init("routing_fuzzer");
    
    // Initialize fault injection
    fault_injection::init();
    fault_injection::set_failure_probability(0.02);
    
    // Initialize sanitizer
    sanitizer::init();
    
    // Read configuration from environment variables
    let max_iterations = env::var("FORM_FUZZING_MAX_ITERATIONS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_MAX_ITERATIONS);
        
    let corpus_dir = env::var("FORM_FUZZING_CORPUS_DIR")
        .unwrap_or_else(|_| DEFAULT_CORPUS_DIR.to_string());
        
    let seed = env::var("FORM_FUZZING_SEED")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or_else(|| {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            now
        });
        
    println!("Configuration:");
    println!("  Max Iterations: {}", max_iterations);
    println!("  Corpus Directory: {}", corpus_dir);
    println!("  Seed: {}", seed);
    
    // Create corpus directory if it doesn't exist
    let corpus_path = Path::new(&corpus_dir);
    if !corpus_path.exists() {
        create_dir_all(&corpus_path).expect("Failed to create corpus directory");
    }
    
    // Create subdirectories for different types of corpus
    let bgp_corpus_dir = corpus_path.join("bgp");
    let dns_corpus_dir = corpus_path.join("dns");
    let health_corpus_dir = corpus_path.join("health");
    let anycast_corpus_dir = corpus_path.join("anycast");
    
    for dir in [&bgp_corpus_dir, &dns_corpus_dir, &health_corpus_dir, &anycast_corpus_dir] {
        if !dir.exists() {
            create_dir_all(dir).expect("Failed to create corpus subdirectory");
        }
    }
    
    // Create the routing harness
    let mut harness = RoutingHarness::new();
    
    // Create generators
    let ip_gen = IpAddressGenerator::new();
    let bgp_gen = BgpAnnouncementGenerator::new();
    let dns_gen = GeoDnsRequestGenerator::new();
    let health_gen = HealthStatusReportGenerator::new();
    let anycast_gen = AnycastTestGenerator::new();
    
    // Create invalid generators
    let invalid_ip_gen = InvalidIpAddressGenerator::new();
    let invalid_bgp_gen = InvalidBgpAnnouncementGenerator::new();
    let invalid_dns_gen = InvalidGeoDnsRequestGenerator::new();
    let invalid_health_gen = InvalidHealthStatusReportGenerator::new();
    
    // Create mutators
    let ip_mutator = IpAddressMutator::new();
    let bgp_mutator = BgpAnnouncementMutator::new();
    let dns_mutator = GeoDnsRequestMutator::new();
    let health_mutator = HealthStatusMutator::new();
    let anycast_mutator = AnycastTestMutator::new();
    
    // Statistics tracking
    let mut stats = FuzzingStats::new();
    let start_time = Instant::now();
    
    println!("Starting fuzzing loop for {} iterations", max_iterations);
    
    // Main fuzzing loop
    for i in 0..max_iterations {
        // Print progress every 100 iterations
        if i % 100 == 0 && i > 0 {
            let elapsed = start_time.elapsed();
            println!("Completed {} iterations in {:.2}s ({:.2} iter/s)",
                i,
                elapsed.as_secs_f64(),
                i as f64 / elapsed.as_secs_f64()
            );
            
            // Print stats
            stats.print();
        }
        
        // Pick a fuzzing strategy based on iteration index
        let strategy = match i % 10 {
            0 => FuzzingStrategy::ValidBgpAnnouncement,
            1 => FuzzingStrategy::InvalidBgpAnnouncement,
            2 => FuzzingStrategy::ValidDnsRequest,
            3 => FuzzingStrategy::InvalidDnsRequest,
            4 => FuzzingStrategy::ValidHealthReport,
            5 => FuzzingStrategy::InvalidHealthReport,
            6 => FuzzingStrategy::AnycastTest,
            7 => FuzzingStrategy::MixedOperations,
            8 => FuzzingStrategy::MutateBgpAnnouncement,
            9 => FuzzingStrategy::MutateDnsAndHealth,
            _ => unreachable!(),
        };
        
        // Execute the selected strategy
        match strategy {
            FuzzingStrategy::ValidBgpAnnouncement => {
                stats.bgp_announcements += 1;
                
                // Generate a valid BGP announcement
                let announcement = bgp_gen
                    .prefix_count(thread_rng().gen_range(1..5))
                    .as_path_length(thread_rng().gen_range(1..10))
                    .generate();
                
                // Process the announcement
                match harness.process_bgp_announcement(&announcement) {
                    Ok(result) => {
                        stats.successful_bgp += 1;
                        
                        // Save successful announcements to corpus
                        if result.accepted {
                            let filename = format!("bgp_valid_{}.log", Uuid::new_v4());
                            let file_path = bgp_corpus_dir.join(filename);
                            let data = format!("{:?}", announcement);
                            fs::write(file_path, data).ok();
                        }
                    },
                    Err(err) => {
                        stats.failed_bgp += 1;
                        println!("BGP announcement error: {:?}", err);
                    }
                }
                
                // Also try a withdrawal sometimes
                if thread_rng().gen_bool(0.3) {
                    // Get list of all announced prefixes
                    let prefixes = harness.bgp_router.get_announced_prefixes();
                    
                    if !prefixes.is_empty() {
                        // Generate a withdrawal for some random prefixes
                        let mut selected_prefixes = Vec::new();
                        let count = thread_rng().gen_range(1..=prefixes.len());
                        
                        for i in 0..count {
                            if let Some(prefix) = prefixes.get(i) {
                                selected_prefixes.push(*prefix);
                            }
                        }
                        
                        // Generate and process withdrawal
                        let withdrawal = bgp_gen.generate_withdrawal(selected_prefixes);
                        
                        match harness.process_bgp_announcement(&withdrawal) {
                            Ok(_) => stats.successful_bgp += 1,
                            Err(_) => stats.failed_bgp += 1,
                        }
                    }
                }
            },
            
            FuzzingStrategy::InvalidBgpAnnouncement => {
                stats.bgp_announcements += 1;
                
                // Generate an invalid BGP announcement
                let announcement = invalid_bgp_gen.generate();
                
                // Process the announcement - should fail
                match harness.process_bgp_announcement(&announcement) {
                    Ok(_) => {
                        stats.unexpected_success += 1;
                        
                        // This should have failed but didn't - save to corpus
                        let filename = format!("bgp_unexpected_success_{}.log", Uuid::new_v4());
                        let file_path = bgp_corpus_dir.join(filename);
                        let data = format!("{:?}", announcement);
                        fs::write(file_path, data).ok();
                        
                        println!("Warning: Invalid BGP announcement was accepted: {:?}", announcement);
                    },
                    Err(_) => {
                        stats.expected_failures += 1;
                    }
                }
            },
            
            FuzzingStrategy::ValidDnsRequest => {
                stats.dns_requests += 1;
                
                // First add some DNS records if needed
                if thread_rng().gen_bool(0.5) || harness.dns_server.get_records("example.com").is_empty() {
                    let domain = "example.com";
                    let mut added_records = 0;
                    
                    // Add records for different regions
                    for region in Region::all() {
                        let ip_info = ip_gen
                            .region(region)
                            .generate_ip_info();
                            
                        match harness.add_dns_record(domain, ip_info) {
                            RoutingHarness::RoutingOperationResult::Success => added_records += 1,
                            _ => {}
                        }
                    }
                    
                    println!("Added {} DNS records for example.com", added_records);
                }
                
                // Generate a valid DNS request
                let request = dns_gen
                    .domain("example.com")
                    .include_ecs(thread_rng().gen_bool(0.7))
                    .include_coordinates(thread_rng().gen_bool(0.3))
                    .client_region(Region::random())
                    .generate();
                
                // Process the DNS request
                match harness.resolve_dns(&request) {
                    Ok(result) => {
                        stats.successful_dns += 1;
                        
                        // Save successful requests to corpus
                        let filename = format!("dns_valid_{}.log", Uuid::new_v4());
                        let file_path = dns_corpus_dir.join(filename);
                        let data = format!("{:?}\nResult: {:?}", request, result);
                        fs::write(file_path, data).ok();
                    },
                    Err(err) => {
                        stats.failed_dns += 1;
                        println!("DNS request error: {:?}", err);
                    }
                }
            },
            
            FuzzingStrategy::InvalidDnsRequest => {
                stats.dns_requests += 1;
                
                // Generate an invalid DNS request
                let request = invalid_dns_gen.generate();
                
                // Process the request - should fail
                match harness.resolve_dns(&request) {
                    Ok(result) => {
                        stats.unexpected_success += 1;
                        
                        // This should have failed but didn't - save to corpus
                        let filename = format!("dns_unexpected_success_{}.log", Uuid::new_v4());
                        let file_path = dns_corpus_dir.join(filename);
                        let data = format!("{:?}\nResult: {:?}", request, result);
                        fs::write(file_path, data).ok();
                        
                        println!("Warning: Invalid DNS request was accepted: {:?}", request);
                    },
                    Err(_) => {
                        stats.expected_failures += 1;
                    }
                }
            },
            
            FuzzingStrategy::ValidHealthReport => {
                stats.health_reports += 1;
                
                // Generate a valid health report
                let report = health_gen
                    .node_count(thread_rng().gen_range(1..10))
                    .generate();
                
                // Sometimes include unhealthy nodes
                let report = if thread_rng().gen_bool(0.3) {
                    health_gen
                        .node_count(thread_rng().gen_range(5..15))
                        .generate_with_unhealthy(thread_rng().gen_range(1..5))
                } else {
                    report
                };
                
                // Process the health report
                match harness.update_health(&report) {
                    RoutingHarness::RoutingOperationResult::Success => {
                        stats.successful_health += 1;
                        
                        // Save successful reports to corpus
                        let filename = format!("health_valid_{}.log", Uuid::new_v4());
                        let file_path = health_corpus_dir.join(filename);
                        let data = format!("{:?}", report);
                        fs::write(file_path, data).ok();
                    },
                    err => {
                        stats.failed_health += 1;
                        println!("Health report error: {:?}", err);
                    }
                }
            },
            
            FuzzingStrategy::InvalidHealthReport => {
                stats.health_reports += 1;
                
                // Generate an invalid health report
                let report = invalid_health_gen.generate();
                
                // Process the report - should fail
                match harness.update_health(&report) {
                    RoutingHarness::RoutingOperationResult::Success => {
                        stats.unexpected_success += 1;
                        
                        // This should have failed but didn't - save to corpus
                        let filename = format!("health_unexpected_success_{}.log", Uuid::new_v4());
                        let file_path = health_corpus_dir.join(filename);
                        let data = format!("{:?}", report);
                        fs::write(file_path, data).ok();
                        
                        println!("Warning: Invalid health report was accepted: {:?}", report);
                    },
                    _ => {
                        stats.expected_failures += 1;
                    }
                }
            },
            
            FuzzingStrategy::AnycastTest => {
                stats.anycast_tests += 1;
                
                // First set up the DNS records for the test if needed
                let domain = "anycast-test.com";
                let regions_with_records = HashSet::new();
                
                // Add records for all regions
                for region in Region::all() {
                    // Add multiple records per region
                    for _ in 0..thread_rng().gen_range(1..4) {
                        let ip_info = ip_gen
                            .region(region)
                            .generate_ip_info();
                            
                        harness.add_dns_record(domain, ip_info);
                    }
                }
                
                // Generate an anycast test
                let test = anycast_gen
                    .domain(domain)
                    .request_count(thread_rng().gen_range(3..10))
                    .generate();
                
                // Run the anycast test
                match harness.run_anycast_test(&test) {
                    Ok(results) => {
                        stats.successful_anycast += 1;
                        
                        // Verify that the test results are valid
                        let is_valid = harness.verify_anycast_test(&test.test_id, &results);
                        
                        if is_valid {
                            // Save successful tests to corpus
                            let filename = format!("anycast_valid_{}.log", Uuid::new_v4());
                            let file_path = anycast_corpus_dir.join(filename);
                            let data = format!("{:?}\nResults: {:?}", test, results);
                            fs::write(file_path, data).ok();
                        } else {
                            stats.failed_anycast += 1;
                        }
                    },
                    Err(err) => {
                        stats.failed_anycast += 1;
                        println!("Anycast test error: {:?}", err);
                    }
                }
            },
            
            FuzzingStrategy::MixedOperations => {
                // Perform a series of mixed operations
                let mut rng = thread_rng();
                let op_count = rng.gen_range(5..15);
                
                for _ in 0..op_count {
                    match rng.gen_range(0..4) {
                        0 => {
                            // BGP announcement
                            stats.bgp_announcements += 1;
                            let announcement = bgp_gen.generate();
                            match harness.process_bgp_announcement(&announcement) {
                                Ok(_) => stats.successful_bgp += 1,
                                Err(_) => stats.failed_bgp += 1,
                            }
                        },
                        1 => {
                            // DNS request
                            stats.dns_requests += 1;
                            let request = dns_gen.generate();
                            match harness.resolve_dns(&request) {
                                Ok(_) => stats.successful_dns += 1,
                                Err(_) => stats.failed_dns += 1,
                            }
                        },
                        2 => {
                            // Health report
                            stats.health_reports += 1;
                            let report = health_gen.generate();
                            match harness.update_health(&report) {
                                RoutingHarness::RoutingOperationResult::Success => stats.successful_health += 1,
                                _ => stats.failed_health += 1,
                            }
                        },
                        3 => {
                            // Add DNS record
                            let domain = match rng.gen_range(0..3) {
                                0 => "example.com",
                                1 => "test.org",
                                _ => "anycast.net",
                            };
                            
                            let ip_info = ip_gen.generate_ip_info();
                            match harness.add_dns_record(domain, ip_info) {
                                RoutingHarness::RoutingOperationResult::Success => stats.successful_dns += 1,
                                _ => stats.failed_dns += 1,
                            }
                        },
                        _ => unreachable!(),
                    }
                }
            },
            
            FuzzingStrategy::MutateBgpAnnouncement => {
                stats.bgp_announcements += 1;
                
                // Generate a valid BGP announcement
                let mut announcement = bgp_gen.generate();
                
                // Mutate it several times
                let mutation_count = thread_rng().gen_range(1..5);
                
                for _ in 0..mutation_count {
                    announcement = match thread_rng().gen_range(0..5) {
                        0 => bgp_mutator.mutate_prefixes(&announcement),
                        1 => bgp_mutator.mutate_as_path(&announcement),
                        2 => bgp_mutator.mutate_communities(&announcement),
                        3 => bgp_mutator.mutate_next_hop(&announcement),
                        _ => bgp_mutator.mutate_attributes(&announcement),
                    };
                }
                
                // Process the mutated announcement
                match harness.process_bgp_announcement(&announcement) {
                    Ok(_) => stats.successful_bgp += 1,
                    Err(_) => stats.failed_bgp += 1,
                }
            },
            
            FuzzingStrategy::MutateDnsAndHealth => {
                if thread_rng().gen_bool(0.5) {
                    stats.dns_requests += 1;
                    
                    // Generate a DNS request and mutate it
                    let mut request = dns_gen.generate();
                    
                    let mutation_count = thread_rng().gen_range(1..3);
                    
                    for _ in 0..mutation_count {
                        request = match thread_rng().gen_range(0..3) {
                            0 => dns_mutator.mutate_domain(&request),
                            1 => dns_mutator.mutate_client_ip(&request),
                            _ => dns_mutator.mutate_ecs_prefix(&request),
                        };
                    }
                    
                    // Process the mutated request
                    match harness.resolve_dns(&request) {
                        Ok(_) => stats.successful_dns += 1,
                        Err(_) => stats.failed_dns += 1,
                    }
                } else {
                    stats.health_reports += 1;
                    
                    // Generate a health report and mutate it
                    let mut report = health_gen.generate();
                    
                    let mutation_count = thread_rng().gen_range(1..3);
                    
                    for _ in 0..mutation_count {
                        report = match thread_rng().gen_range(0..3) {
                            0 => health_mutator.mutate_nodes(&report),
                            1 => health_mutator.mutate_reporter(&report),
                            _ => health_mutator.mutate_timestamp(&report),
                        };
                    }
                    
                    // Process the mutated report
                    match harness.update_health(&report) {
                        RoutingHarness::RoutingOperationResult::Success => stats.successful_health += 1,
                        _ => stats.failed_health += 1,
                    }
                }
            },
        }
        
        // Occasionally clear state to start fresh
        if thread_rng().gen_ratio(1, 50) {
            harness.clear_all();
            println!("Cleared harness state");
        }
    }
    
    // Print final statistics
    let elapsed = start_time.elapsed();
    println!("\nFuzzing completed!");
    println!("Executed {} iterations in {:.2}s ({:.2} iter/s)",
        max_iterations,
        elapsed.as_secs_f64(),
        max_iterations as f64 / elapsed.as_secs_f64()
    );
    
    stats.print();
    
    // Print coverage information
    let coverage_info = coverage::get_coverage_info();
    println!("\nCoverage Information:");
    println!("  Covered blocks: {}", coverage_info.covered_blocks);
    println!("  Total blocks: {}", coverage_info.total_blocks);
    println!("  Coverage percentage: {:.2}%", coverage_info.percentage);
    
    // Print sanitizer information
    let sanitizer_info = sanitizer::get_sanitizer_info();
    println!("\nSanitizer Information:");
    println!("  Memory issues detected: {}", sanitizer_info.memory_issues);
    println!("  Other issues detected: {}", sanitizer_info.other_issues);
    
    // Print fault injection information
    let fault_info = fault_injection::get_fault_info();
    println!("\nFault Injection Information:");
    println!("  Faults injected: {}", fault_info.faults_injected);
    println!("  Faults handled: {}", fault_info.faults_handled);
}

/// Fuzzing strategy
#[derive(Debug, Clone, Copy)]
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

/// Statistics for fuzzing
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
    /// Create new statistics tracker
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
    
    /// Print statistics
    fn print(&self) {
        println!("\nFuzzing Statistics:");
        println!("  BGP Announcements:");
        println!("    Total: {}", self.bgp_announcements);
        println!("    Successful: {}", self.successful_bgp);
        println!("    Failed: {}", self.failed_bgp);
        
        println!("  DNS Requests:");
        println!("    Total: {}", self.dns_requests);
        println!("    Successful: {}", self.successful_dns);
        println!("    Failed: {}", self.failed_dns);
        
        println!("  Health Reports:");
        println!("    Total: {}", self.health_reports);
        println!("    Successful: {}", self.successful_health);
        println!("    Failed: {}", self.failed_health);
        
        println!("  Anycast Tests:");
        println!("    Total: {}", self.anycast_tests);
        println!("    Successful: {}", self.successful_anycast);
        println!("    Failed: {}", self.failed_anycast);
        
        println!("  Error Handling:");
        println!("    Unexpected Successes: {}", self.unexpected_success);
        println!("    Expected Failures: {}", self.expected_failures);
    }
} 