// form-fuzzing/src/bin/fuzz_economic.rs

//! Economic Infrastructure Fuzzer

use std::env;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};
use std::collections::HashMap;

use form_fuzzing::generators::economic::{
    AuthTokenGenerator, ApiKeyGenerator, InvalidAuthTokenGenerator, InvalidApiKeyGenerator,
    ResourceUsageReportGenerator, HighResourceUsageReportGenerator, CriticalResourceUsageReportGenerator,
    ResourceThresholdGenerator, WebhookUrlGenerator, InvalidWebhookUrlGenerator,
    AuthToken, ApiKey, ResourceUsageReport,
};
use form_fuzzing::harness::economic::{EconomicHarness, EconomicOperationResult, ResourceType, ResourceThreshold};
use form_fuzzing::instrumentation::coverage;
use form_fuzzing::instrumentation::fault_injection;
use form_fuzzing::mutators::economic::{
    AuthTokenMutator, ApiKeyMutator, ResourceUsageReportMutator,
    ResourceThresholdMutator, WebhookUrlMutator, ResourceMapMutator
};

use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;

fn main() {
    println!("=================================================================");
    println!("Formation Network Economic Infrastructure Fuzzer");
    println!("=================================================================");
    
    // Initialize the fuzzing framework
    form_fuzzing::init();
    
    // Initialize coverage tracking
    coverage::init();
    
    // Set up the Economic Infrastructure harness
    let harness = EconomicHarness::new();
    
    // Create generators
    let auth_token_generator = AuthTokenGenerator::new();
    let api_key_generator = ApiKeyGenerator::new();
    let invalid_auth_token_generator = InvalidAuthTokenGenerator::new();
    let invalid_api_key_generator = InvalidApiKeyGenerator::new();
    let resource_usage_generator = ResourceUsageReportGenerator::new();
    let high_resource_usage_generator = HighResourceUsageReportGenerator::new();
    let critical_resource_usage_generator = CriticalResourceUsageReportGenerator::new();
    let resource_threshold_generator = ResourceThresholdGenerator::new();
    let webhook_url_generator = WebhookUrlGenerator::new();
    let invalid_webhook_url_generator = InvalidWebhookUrlGenerator::new();
    
    // Create mutators
    let auth_token_mutator = AuthTokenMutator::new();
    let api_key_mutator = ApiKeyMutator::new();
    let resource_usage_mutator = ResourceUsageReportMutator::new();
    let resource_threshold_mutator = ResourceThresholdMutator::new();
    let webhook_url_mutator = WebhookUrlMutator::new();
    let resource_map_mutator = ResourceMapMutator::new();
    
    // Load corpus or create a new one
    let corpus_dir = env::var("FUZZING_CORPUS_DIR").unwrap_or_else(|_| "fuzzing-corpus".to_string());
    let corpus_path = Path::new(&corpus_dir).join("economic");
    
    // Create corpus directory if it doesn't exist
    fs::create_dir_all(&corpus_path).expect("Failed to create corpus directory");
    
    // Maximum number of iterations
    let max_iterations = env::var("FUZZING_MAX_ITERATIONS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1000);
    
    println!("Running Economic Infrastructure fuzzer for {} iterations", max_iterations);
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
    let mut operation_failures = 0;
    let mut internal_failures = 0;
    let mut timeout_failures = 0;
    
    // Track coverage
    let mut initial_coverage = coverage::get_coverage_count();
    let mut current_coverage = initial_coverage;
    
    // Keep track of valid tokens and resources for later tests
    let mut valid_tokens = Vec::new();
    let mut valid_api_keys = Vec::new();
    let mut vm_ids = Vec::new();
    
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
        
        // Strategy 1: Register API keys and tokens (10% of tests)
        if i % 10 == 0 {
            total_tests += 1;
            
            // Generate a new auth token
            let auth_token = auth_token_generator.generate();
            
            // Register the token
            let result = harness.register_token(&auth_token.user_id, &auth_token.token);
            
            match result {
                EconomicOperationResult::Success(_) => {
                    successful_tests += 1;
                    
                    // Save token for later tests
                    valid_tokens.push(auth_token);
                    
                    // Limit the number of tokens we store
                    if valid_tokens.len() > 10 {
                        valid_tokens.remove(0);
                    }
                    
                    // Also generate and register an API key
                    let api_key = api_key_generator.generate();
                    let api_key_result = harness.register_api_key(&api_key.user_id, &api_key.api_key);
                    
                    if let EconomicOperationResult::Success(_) = api_key_result {
                        // Save API key for later tests
                        valid_api_keys.push(api_key);
                        
                        // Limit the number of API keys we store
                        if valid_api_keys.len() > 5 {
                            valid_api_keys.remove(0);
                        }
                    }
                }
                _ => {
                    operation_failures += 1;
                    println!("Unexpected failure when registering token: {:?}", result);
                }
            }
        }
        
        // Strategy 2: Report normal resource usage (20% of tests)
        else if i % 10 == 1 || i % 10 == 2 {
            total_tests += 1;
            
            if valid_tokens.is_empty() {
                // Can't proceed without a valid token, try registering one
                let auth_token = auth_token_generator.generate();
                harness.register_token(&auth_token.user_id, &auth_token.token);
                valid_tokens.push(auth_token);
                continue;
            }
            
            // Choose a random token
            let token = &valid_tokens[rng.gen_range(0..valid_tokens.len())];
            
            // Generate a resource usage report
            let report = resource_usage_generator.generate();
            
            // Save VM ID for later tests
            vm_ids.push(report.vm_id.clone());
            
            // Limit the number of VM IDs we store
            if vm_ids.len() > 20 {
                vm_ids.remove(0);
            }
            
            // Report resource usage
            let result = harness.report_resource_usage(&token.token, &report.vm_id, report.resources.clone());
            
            match result {
                EconomicOperationResult::Success(_) => {
                    successful_tests += 1;
                }
                EconomicOperationResult::AuthenticationFailed => {
                    auth_failures += 1;
                    println!("Authentication failed for token: {:?}", token);
                }
                EconomicOperationResult::RateLimited => {
                    rate_limit_failures += 1;
                    println!("Rate limited when reporting resource usage");
                }
                EconomicOperationResult::InternalError(_) => {
                    // This can happen due to simulated random failures
                    internal_failures += 1;
                }
                _ => {
                    operation_failures += 1;
                    println!("Unexpected failure when reporting resource usage: {:?}", result);
                }
            }
        }
        
        // Strategy 3: Report high resource usage (warning threshold) (10% of tests)
        else if i % 10 == 3 {
            total_tests += 1;
            
            if valid_tokens.is_empty() {
                // Can't proceed without a valid token, try registering one
                let auth_token = auth_token_generator.generate();
                harness.register_token(&auth_token.user_id, &auth_token.token);
                valid_tokens.push(auth_token);
                continue;
            }
            
            // Choose a random token
            let token = &valid_tokens[rng.gen_range(0..valid_tokens.len())];
            
            // Generate a high resource usage report
            let report = high_resource_usage_generator.generate();
            
            // Save VM ID for later tests
            vm_ids.push(report.vm_id.clone());
            
            // Report resource usage
            let result = harness.report_resource_usage(&token.token, &report.vm_id, report.resources.clone());
            
            match result {
                EconomicOperationResult::Success(data) => {
                    successful_tests += 1;
                    
                    // Check if any threshold violations were detected
                    if let Some(violations) = data.get("threshold_violations").and_then(|v| v.as_u64()) {
                        if violations == 0 {
                            println!("Warning: No threshold violations detected for high resource usage");
                        }
                    }
                }
                EconomicOperationResult::AuthenticationFailed => {
                    auth_failures += 1;
                }
                EconomicOperationResult::RateLimited => {
                    rate_limit_failures += 1;
                }
                EconomicOperationResult::InternalError(_) => {
                    // This can happen due to simulated random failures
                    internal_failures += 1;
                }
                _ => {
                    operation_failures += 1;
                    println!("Unexpected failure when reporting high resource usage: {:?}", result);
                }
            }
        }
        
        // Strategy 4: Report critical resource usage (10% of tests)
        else if i % 10 == 4 {
            total_tests += 1;
            
            if valid_tokens.is_empty() {
                // Can't proceed without a valid token, try registering one
                let auth_token = auth_token_generator.generate();
                harness.register_token(&auth_token.user_id, &auth_token.token);
                valid_tokens.push(auth_token);
                continue;
            }
            
            // Choose a random token
            let token = &valid_tokens[rng.gen_range(0..valid_tokens.len())];
            
            // Generate a critical resource usage report
            let report = critical_resource_usage_generator.generate();
            
            // Save VM ID for later tests
            vm_ids.push(report.vm_id.clone());
            
            // Report resource usage
            let result = harness.report_resource_usage(&token.token, &report.vm_id, report.resources.clone());
            
            match result {
                EconomicOperationResult::Success(data) => {
                    successful_tests += 1;
                    
                    // Check if any threshold violations were detected
                    if let Some(violations) = data.get("threshold_violations").and_then(|v| v.as_u64()) {
                        if violations == 0 {
                            println!("Warning: No threshold violations detected for critical resource usage");
                        }
                    }
                }
                EconomicOperationResult::AuthenticationFailed => {
                    auth_failures += 1;
                }
                EconomicOperationResult::RateLimited => {
                    rate_limit_failures += 1;
                }
                EconomicOperationResult::InternalError(_) => {
                    // This can happen due to simulated random failures
                    internal_failures += 1;
                }
                _ => {
                    operation_failures += 1;
                    println!("Unexpected failure when reporting critical resource usage: {:?}", result);
                }
            }
        }
        
        // Strategy 5: Report invalid resource usage (10% of tests)
        else if i % 10 == 5 {
            total_tests += 1;
            
            if valid_tokens.is_empty() {
                // Can't proceed without a valid token, try registering one
                let auth_token = auth_token_generator.generate();
                harness.register_token(&auth_token.user_id, &auth_token.token);
                valid_tokens.push(auth_token);
                continue;
            }
            
            // Choose a random token
            let token = &valid_tokens[rng.gen_range(0..valid_tokens.len())];
            
            // Generate a resource usage report and mutate it
            let mut report = resource_usage_generator.generate();
            resource_usage_mutator.mutate(&mut report);
            
            // Report resource usage
            let result = harness.report_resource_usage(&token.token, &report.vm_id, report.resources.clone());
            
            match result {
                EconomicOperationResult::Success(_) => {
                    // This might be unexpected success for invalid data
                    successful_tests += 1;
                }
                EconomicOperationResult::InvalidInput(_) => {
                    // Expected response for invalid data
                    successful_tests += 1;
                    invalid_input_failures += 1;
                }
                EconomicOperationResult::ResourceNotFound => {
                    // Expected if VM ID was mutated to be empty or invalid
                    successful_tests += 1;
                    not_found_failures += 1;
                }
                EconomicOperationResult::InternalError(_) => {
                    // This can happen due to simulated random failures
                    internal_failures += 1;
                }
                _ => {
                    // Other error types are acceptable for invalid data
                    successful_tests += 1;
                }
            }
        }
        
        // Strategy 6: Get VM usage with valid and invalid tokens (10% of tests)
        else if i % 10 == 6 {
            total_tests += 1;
            
            let use_valid_token = rng.gen_bool(0.7);
            let token = if use_valid_token && !valid_tokens.is_empty() {
                valid_tokens[rng.gen_range(0..valid_tokens.len())].token.clone()
            } else {
                // Generate an invalid token
                let mut invalid_token = invalid_auth_token_generator.generate();
                auth_token_mutator.mutate(&mut invalid_token);
                invalid_token.token
            };
            
            let vm_id = if !vm_ids.is_empty() {
                vm_ids[rng.gen_range(0..vm_ids.len())].clone()
            } else {
                format!("vm-{}", rng.gen::<u32>())
            };
            
            // Get VM usage
            let result = harness.get_vm_usage(&token, &vm_id);
            
            match result {
                EconomicOperationResult::Success(_) => {
                    if use_valid_token {
                        successful_tests += 1;
                    } else {
                        println!("Unexpected success with invalid token");
                        auth_failures += 1;
                    }
                }
                EconomicOperationResult::AuthenticationFailed => {
                    if !use_valid_token {
                        successful_tests += 1; // Expected for invalid token
                    } else {
                        auth_failures += 1;
                        println!("Authentication failed for valid token");
                    }
                }
                EconomicOperationResult::ResourceNotFound => {
                    if vm_ids.is_empty() {
                        successful_tests += 1; // Expected if we don't have any real VM IDs
                    } else {
                        not_found_failures += 1;
                        println!("Resource not found for VM ID: {}", vm_id);
                    }
                }
                EconomicOperationResult::RateLimited => {
                    rate_limit_failures += 1;
                }
                _ => {
                    operation_failures += 1;
                    println!("Unexpected failure when getting VM usage: {:?}", result);
                }
            }
        }
        
        // Strategy 7: Get usage events with valid and invalid tokens (10% of tests)
        else if i % 10 == 7 {
            total_tests += 1;
            
            let use_valid_token = rng.gen_bool(0.7);
            let token = if use_valid_token && !valid_tokens.is_empty() {
                valid_tokens[rng.gen_range(0..valid_tokens.len())].token.clone()
            } else {
                // Generate an invalid token
                let mut invalid_token = invalid_auth_token_generator.generate();
                auth_token_mutator.mutate(&mut invalid_token);
                invalid_token.token
            };
            
            let limit = if rng.gen_bool(0.5) {
                Some(rng.gen_range(1..100))
            } else {
                None
            };
            
            // Get recent usage events
            let result = harness.get_recent_usage_events(&token, limit);
            
            match result {
                EconomicOperationResult::Success(_) => {
                    if use_valid_token {
                        successful_tests += 1;
                    } else {
                        println!("Unexpected success with invalid token");
                        auth_failures += 1;
                    }
                }
                EconomicOperationResult::AuthenticationFailed => {
                    if !use_valid_token {
                        successful_tests += 1; // Expected for invalid token
                    } else {
                        auth_failures += 1;
                        println!("Authentication failed for valid token");
                    }
                }
                EconomicOperationResult::RateLimited => {
                    rate_limit_failures += 1;
                }
                _ => {
                    operation_failures += 1;
                    println!("Unexpected failure when getting usage events: {:?}", result);
                }
            }
        }
        
        // Strategy 8: Get threshold events with valid and invalid tokens (10% of tests)
        else if i % 10 == 8 {
            total_tests += 1;
            
            let use_valid_token = rng.gen_bool(0.7);
            let token = if use_valid_token && !valid_tokens.is_empty() {
                valid_tokens[rng.gen_range(0..valid_tokens.len())].token.clone()
            } else {
                // Generate an invalid token
                let mut invalid_token = invalid_auth_token_generator.generate();
                auth_token_mutator.mutate(&mut invalid_token);
                invalid_token.token
            };
            
            let limit = if rng.gen_bool(0.5) {
                Some(rng.gen_range(1..100))
            } else {
                None
            };
            
            let critical_only = if rng.gen_bool(0.5) {
                Some(rng.gen_bool(0.5))
            } else {
                None
            };
            
            // Get recent threshold events
            let result = harness.get_recent_threshold_events(&token, limit, critical_only);
            
            match result {
                EconomicOperationResult::Success(_) => {
                    if use_valid_token {
                        successful_tests += 1;
                    } else {
                        println!("Unexpected success with invalid token");
                        auth_failures += 1;
                    }
                }
                EconomicOperationResult::AuthenticationFailed => {
                    if !use_valid_token {
                        successful_tests += 1; // Expected for invalid token
                    } else {
                        auth_failures += 1;
                        println!("Authentication failed for valid token");
                    }
                }
                EconomicOperationResult::RateLimited => {
                    rate_limit_failures += 1;
                }
                _ => {
                    operation_failures += 1;
                    println!("Unexpected failure when getting threshold events: {:?}", result);
                }
            }
        }
        
        // Strategy 9: Register webhooks and update thresholds (10% of tests)
        else {
            total_tests += 1;
            
            // Choose subtest randomly
            let subtest = rng.gen_range(0..2);
            
            match subtest {
                0 => {
                    // Register webhook
                    let use_valid_token = rng.gen_bool(0.7);
                    let token = if use_valid_token && !valid_tokens.is_empty() {
                        valid_tokens[rng.gen_range(0..valid_tokens.len())].token.clone()
                    } else {
                        // Generate an invalid token
                        let mut invalid_token = invalid_auth_token_generator.generate();
                        auth_token_mutator.mutate(&mut invalid_token);
                        invalid_token.token
                    };
                    
                    let use_valid_url = rng.gen_bool(0.7);
                    let mut webhook_url = if use_valid_url {
                        webhook_url_generator.generate()
                    } else {
                        invalid_webhook_url_generator.generate()
                    };
                    
                    // Occasionally mutate the URL even if it's valid
                    if use_valid_url && rng.gen_bool(0.3) {
                        webhook_url_mutator.mutate(&mut webhook_url);
                    }
                    
                    // Register webhook
                    let result = harness.register_webhook(&token, &webhook_url);
                    
                    match result {
                        EconomicOperationResult::Success(_) => {
                            if use_valid_token && use_valid_url {
                                successful_tests += 1;
                            } else {
                                println!("Unexpected success with invalid token or URL");
                                invalid_input_failures += 1;
                            }
                        }
                        EconomicOperationResult::AuthenticationFailed => {
                            if !use_valid_token {
                                successful_tests += 1; // Expected for invalid token
                            } else {
                                auth_failures += 1;
                                println!("Authentication failed for valid token");
                            }
                        }
                        EconomicOperationResult::InvalidInput(_) => {
                            if !use_valid_url {
                                successful_tests += 1; // Expected for invalid URL
                            } else {
                                invalid_input_failures += 1;
                                println!("Invalid input for valid URL: {}", webhook_url);
                            }
                        }
                        EconomicOperationResult::RateLimited => {
                            rate_limit_failures += 1;
                        }
                        _ => {
                            operation_failures += 1;
                            println!("Unexpected failure when registering webhook: {:?}", result);
                        }
                    }
                },
                1 => {
                    // Update threshold
                    let use_valid_token = rng.gen_bool(0.7);
                    let token = if use_valid_token && !valid_tokens.is_empty() {
                        valid_tokens[rng.gen_range(0..valid_tokens.len())].token.clone()
                    } else {
                        // Generate an invalid token
                        let mut invalid_token = invalid_auth_token_generator.generate();
                        auth_token_mutator.mutate(&mut invalid_token);
                        invalid_token.token
                    };
                    
                    // Generate a resource threshold
                    let (resource_type, mut threshold) = resource_threshold_generator.generate();
                    
                    // Occasionally mutate the threshold
                    if rng.gen_bool(0.3) {
                        resource_threshold_mutator.mutate(&mut threshold);
                    }
                    
                    let warning = Some(threshold.warning_threshold);
                    let critical = Some(threshold.critical_threshold);
                    let enabled = Some(threshold.enabled);
                    
                    // Update threshold
                    let result = harness.update_threshold(
                        &token,
                        resource_type.clone(),
                        warning,
                        critical,
                        enabled
                    );
                    
                    match result {
                        EconomicOperationResult::Success(_) => {
                            if use_valid_token {
                                successful_tests += 1;
                            } else {
                                println!("Unexpected success with invalid token");
                                auth_failures += 1;
                            }
                        }
                        EconomicOperationResult::AuthenticationFailed => {
                            if !use_valid_token {
                                successful_tests += 1; // Expected for invalid token
                            } else {
                                auth_failures += 1;
                                println!("Authentication failed for valid token");
                            }
                        }
                        EconomicOperationResult::InvalidInput(_) => {
                            if !threshold.enabled {
                                successful_tests += 1; // Sometimes expected for mutated thresholds
                            } else {
                                invalid_input_failures += 1;
                                println!("Invalid input for threshold: {:?}", threshold);
                            }
                        }
                        EconomicOperationResult::RateLimited => {
                            rate_limit_failures += 1;
                        }
                        _ => {
                            operation_failures += 1;
                            println!("Unexpected failure when updating threshold: {:?}", result);
                        }
                    }
                },
                _ => {}
            }
        }
        
        // Occasionally inject faults
        if rng.gen_bool(0.05) {
            let fault_points = [
                "economic_auth",
                "economic_db",
                "economic_threshold",
                "economic_event",
                "economic_webhook",
            ];
            
            let fault_point = fault_points.choose(&mut rng).unwrap();
            fault_injection::register_fault_point(fault_point, 0.5);
        }
    }
    
    // Calculate elapsed time
    let elapsed = start_time.elapsed();
    let elapsed_secs = elapsed.as_secs_f64();
    
    // Print summary
    println!("\n=================================================================");
    println!("Economic Infrastructure Fuzzing Summary:");
    println!("=================================================================");
    println!("Total tests:            {}", total_tests);
    println!("Successful tests:       {} ({:.2}%)", successful_tests, 100.0 * successful_tests as f64 / total_tests as f64);
    println!("Authentication failures: {}", auth_failures);
    println!("Permission failures:    {}", permission_failures);
    println!("Rate limit failures:    {}", rate_limit_failures);
    println!("Not found failures:     {}", not_found_failures);
    println!("Invalid input failures: {}", invalid_input_failures);
    println!("Operation failures:     {}", operation_failures);
    println!("Internal failures:      {}", internal_failures);
    println!("Timeout failures:       {}", timeout_failures);
    println!("-----------------------------------------------------------------");
    println!("Elapsed time:           {:.2} seconds", elapsed_secs);
    println!("Tests per second:       {:.2}", total_tests as f64 / elapsed_secs);
    println!("Initial coverage:       {}", initial_coverage);
    println!("Final coverage:         {}", current_coverage);
    println!("New edge coverage:      {}", current_coverage - initial_coverage);
    println!("=================================================================");
    
    // Clean up and save coverage data
    coverage::save_coverage_data("economic_fuzzer_coverage.dat");
    form_fuzzing::finalize();
} 