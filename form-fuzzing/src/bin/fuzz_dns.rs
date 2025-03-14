// form-fuzzing/src/bin/fuzz_dns.rs
//! DNS Management Fuzzer

use std::env;
use std::fs;
use std::path::Path;
use std::time::Instant;

use form_fuzzing::generators::dns::{
    DNSRecord, DNSRecordType, DNSZone, DNSRecordGenerator, DNSZoneGenerator
};
use form_fuzzing::harness::dns::{DNSHarness, DNSOperationResult};
use form_fuzzing::instrumentation::coverage;
use form_fuzzing::instrumentation::fault_injection;
use form_fuzzing::mutators::dns::{DNSRecordMutator, DNSZoneMutator};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

fn main() {
    println!("=================================================================");
    println!("Formation Network DNS Management Fuzzer");
    println!("=================================================================");

    // Initialize the fuzzing framework
    form_fuzzing::init();
    
    // Initialize coverage tracking
    coverage::init();
    
    // Set up the DNS management harness
    let harness = DNSHarness::new();
    
    // Create generators
    let record_generator = DNSRecordGenerator::new();
    let zone_generator = DNSZoneGenerator::new(10); // Generate zones with up to 10 records
    
    // Create mutators
    let record_mutator = DNSRecordMutator::new();
    let zone_mutator = DNSZoneMutator::new();
    
    // Load corpus or create a new one
    let corpus_dir = env::var("FUZZING_CORPUS_DIR").unwrap_or_else(|_| "fuzzing-corpus".to_string());
    let corpus_path = Path::new(&corpus_dir).join("dns");
    
    // Create corpus directory if it doesn't exist
    fs::create_dir_all(&corpus_path).expect("Failed to create corpus directory");
    
    // Maximum number of iterations
    let max_iterations = env::var("FUZZING_MAX_ITERATIONS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1000);
    
    println!("Running DNS fuzzer for {} iterations", max_iterations);
    println!("Corpus directory: {}", corpus_path.display());
    
    // Create a deterministic RNG for reproducibility
    let seed = env::var("FUZZING_SEED")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(42);
    let mut rng = StdRng::seed_from_u64(seed);
    
    // Track test results
    let mut total_tests = 0;
    let mut successful_tests = 0;
    let mut auth_failures = 0;
    let mut permission_failures = 0;
    let mut rate_limit_failures = 0;
    let mut not_found_failures = 0;
    let mut invalid_input_failures = 0;
    let mut internal_failures = 0;
    let mut timeout_failures = 0;
    
    // Track coverage
    let mut initial_coverage = coverage::get_coverage_count();
    let mut current_coverage = initial_coverage;
    
    // Start timing
    let start_time = Instant::now();
    
    // Run the fuzzing loop
    for i in 0..max_iterations {
        // Reset fault injection for this iteration
        fault_injection::reset();
        
        // Periodically report progress
        if i % 100 == 0 && i > 0 {
            let elapsed = start_time.elapsed().as_secs_f64();
            let tests_per_second = i as f64 / elapsed;
            let new_coverage = coverage::get_coverage_count();
            println!(
                "Progress: {}/{} tests ({:.2} tests/sec), coverage: {} (+{})",
                i, max_iterations, tests_per_second, new_coverage, new_coverage - initial_coverage
            );
            current_coverage = new_coverage;
        }
        
        // Strategy 1: Fuzz authentication with valid zones but invalid API keys
        if i % 5 == 0 {
            total_tests += 1;
            
            // Generate a valid zone
            let zone = zone_generator.generate();
            
            // Use invalid API key
            let user_id = "user1";
            let api_key = "invalid_key";
            
            // Try to create a zone with invalid API key
            let result = harness.create_zone(user_id, api_key, zone);
            
            match result {
                DNSOperationResult::AuthenticationFailed => {
                    successful_tests += 1;
                }
                _ => {
                    auth_failures += 1;
                    println!("Authentication failure test failed: {:?}", result);
                }
            }
        }
        
        // Strategy 2: Fuzz zone creation with valid authentication
        else if i % 5 == 1 {
            total_tests += 1;
            
            // Generate a valid zone
            let mut zone = zone_generator.generate();
            
            // Use valid API key
            let user_id = "user1";
            let api_key = "key1";
            
            // Try to create a zone
            let result = harness.create_zone(user_id, api_key, zone.clone());
            
            match result {
                DNSOperationResult::Success => {
                    successful_tests += 1;
                    
                    // Now try to create the same zone again (should fail)
                    let duplicate_result = harness.create_zone(user_id, api_key, zone);
                    if duplicate_result != DNSOperationResult::InvalidInput("Zone already exists".to_string()) {
                        invalid_input_failures += 1;
                        println!("Duplicate zone test failed: {:?}", duplicate_result);
                    } else {
                        successful_tests += 1;
                    }
                }
                _ => {
                    invalid_input_failures += 1;
                    println!("Zone creation test failed: {:?}", result);
                }
            }
        }
        
        // Strategy 3: Fuzz permission checks with different user IDs
        else if i % 5 == 2 {
            total_tests += 1;
            
            // Generate a valid zone
            let zone = zone_generator.generate();
            
            // Create zone as user1
            let user1_id = "user1";
            let user1_key = "key1";
            let result = harness.create_zone(user1_id, user1_key, zone.clone());
            
            if result == DNSOperationResult::Success {
                // Try to delete the zone as user2 (should fail)
                let user2_id = "user2";
                let user2_key = "key2";
                let delete_result = harness.delete_zone(user2_id, user2_key, &zone.name);
                
                match delete_result {
                    DNSOperationResult::PermissionDenied => {
                        successful_tests += 1;
                    }
                    _ => {
                        permission_failures += 1;
                        println!("Permission test failed: {:?}", delete_result);
                    }
                }
            } else {
                invalid_input_failures += 1;
                println!("Zone creation for permission test failed: {:?}", result);
            }
        }
        
        // Strategy 4: Fuzz zone lifecycle (create, add records, update records, delete records, delete zone)
        else if i % 5 == 3 {
            total_tests += 1;
            
            // Generate a valid zone with no records
            let mut zone = zone_generator.generate();
            zone.records.clear();
            
            // Use valid API key
            let user_id = "user1";
            let api_key = "key1";
            
            // Create zone
            let create_result = harness.create_zone(user_id, api_key, zone.clone());
            
            if create_result == DNSOperationResult::Success {
                // Add a record
                let record = record_generator.generate();
                let add_result = harness.add_record(user_id, api_key, &zone.name, record.clone());
                
                if add_result == DNSOperationResult::Success {
                    // Update the record
                    let mut updated_record = record.clone();
                    updated_record.ttl = 3600;
                    let update_result = harness.update_record(
                        user_id, api_key, &zone.name, &record.domain, record.record_type, updated_record
                    );
                    
                    if update_result == DNSOperationResult::Success {
                        // Delete the record
                        let delete_record_result = harness.delete_record(
                            user_id, api_key, &zone.name, &record.domain, record.record_type
                        );
                        
                        if delete_record_result == DNSOperationResult::Success {
                            // Delete the zone
                            let delete_zone_result = harness.delete_zone(user_id, api_key, &zone.name);
                            
                            if delete_zone_result == DNSOperationResult::Success {
                                successful_tests += 1;
                            } else {
                                not_found_failures += 1;
                                println!("Zone deletion test failed: {:?}", delete_zone_result);
                            }
                        } else {
                            not_found_failures += 1;
                            println!("Record deletion test failed: {:?}", delete_record_result);
                        }
                    } else {
                        not_found_failures += 1;
                        println!("Record update test failed: {:?}", update_result);
                    }
                } else {
                    invalid_input_failures += 1;
                    println!("Record addition test failed: {:?}", add_result);
                }
            } else {
                invalid_input_failures += 1;
                println!("Zone creation for lifecycle test failed: {:?}", create_result);
            }
        }
        
        // Strategy 5: Fuzz with mutated zones and records
        else {
            total_tests += 1;
            
            // Generate a valid zone
            let mut zone = zone_generator.generate();
            
            // Mutate the zone
            zone_mutator.mutate(&mut zone);
            
            // Use valid API key
            let user_id = "user1";
            let api_key = "key1";
            
            // Try to create the mutated zone
            let result = harness.create_zone(user_id, api_key, zone.clone());
            
            // Any result is valid here, we're just testing that it doesn't crash
            match result {
                DNSOperationResult::Success => {
                    successful_tests += 1;
                    
                    // If successful, try to add a mutated record
                    if !zone.records.is_empty() {
                        let mut record = record_generator.generate();
                        record_mutator.mutate(&mut record);
                        
                        let add_result = harness.add_record(user_id, api_key, &zone.name, record);
                        
                        // Any result is valid here too
                        if add_result == DNSOperationResult::Success {
                            successful_tests += 1;
                        }
                    }
                }
                DNSOperationResult::InvalidInput(_) => {
                    // This is expected for some mutations
                    successful_tests += 1;
                }
                DNSOperationResult::InternalError(_) => {
                    internal_failures += 1;
                }
                DNSOperationResult::Timeout => {
                    timeout_failures += 1;
                }
                _ => {
                    // Other results are also acceptable
                    successful_tests += 1;
                }
            }
        }
        
        // Inject random faults occasionally
        if rng.gen_bool(0.1) {
            fault_injection::register_fault_point("dns_create_zone", 0.5);
        }
        if rng.gen_bool(0.1) {
            fault_injection::register_fault_point("dns_delete_zone", 0.5);
        }
        if rng.gen_bool(0.1) {
            fault_injection::register_fault_point("dns_add_record", 0.5);
        }
        if rng.gen_bool(0.1) {
            fault_injection::register_fault_point("dns_auth", 0.5);
        }
    }
    
    // Calculate elapsed time
    let elapsed = start_time.elapsed();
    let elapsed_secs = elapsed.as_secs_f64();
    
    // Print summary
    println!("\n=================================================================");
    println!("DNS Fuzzing Summary:");
    println!("=================================================================");
    println!("Total tests:           {}", total_tests);
    println!("Successful tests:      {} ({:.2}%)", successful_tests, 100.0 * successful_tests as f64 / total_tests as f64);
    println!("Authentication failures: {}", auth_failures);
    println!("Permission failures:   {}", permission_failures);
    println!("Rate limit failures:   {}", rate_limit_failures);
    println!("Not found failures:    {}", not_found_failures);
    println!("Invalid input failures: {}", invalid_input_failures);
    println!("Internal failures:     {}", internal_failures);
    println!("Timeout failures:      {}", timeout_failures);
    println!("-----------------------------------------------------------------");
    println!("Elapsed time:          {:.2} seconds", elapsed_secs);
    println!("Tests per second:      {:.2}", total_tests as f64 / elapsed_secs);
    println!("Initial coverage:      {}", initial_coverage);
    println!("Final coverage:        {}", current_coverage);
    println!("New edge coverage:     {}", current_coverage - initial_coverage);
    println!("=================================================================");
    
    // Clean up and save coverage data
    coverage::save_coverage_data("dns_fuzzer_coverage.dat");
    form_fuzzing::finalize();
} 