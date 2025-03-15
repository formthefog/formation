// form-fuzzing/src/bin/fuzz_mcp.rs

//! MCP Server Fuzzer

use std::env;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use form_fuzzing::generators::Generator;
use form_fuzzing::generators::mcp::{
    LoginRequestGenerator, InvalidLoginRequestGenerator,
    VMCreateRequestGenerator, VMListRequestGenerator,
    PackBuildRequestGenerator, PackShipRequestGenerator,
    InvalidPackBuildRequestGenerator,
};
use form_fuzzing::harness::mcp::{MCPHarness, MCPOperationResult};
use form_fuzzing::instrumentation::coverage;
use form_fuzzing::instrumentation::fault_injection;
use form_fuzzing::instrumentation::fault_injection::FaultConfig;
use form_fuzzing::mutators::Mutator;
use form_fuzzing::mutators::mcp::{
    LoginRequestMutator, VMCreateRequestMutator, VMListRequestMutator,
    PackBuildRequestMutator, PackShipRequestMutator, JsonValueMutator,
};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use serde_json::json;

fn main() {
    println!("=================================================================");
    println!("Formation Network MCP Server Fuzzer");
    println!("=================================================================");

    // Initialize the fuzzing framework
    form_fuzzing::init();
    
    // Initialize coverage tracking
    coverage::init();
    
    // Set up the MCP harness
    let harness = MCPHarness::new();
    
    // Create generators
    let login_generator = LoginRequestGenerator::new();
    let invalid_login_generator = InvalidLoginRequestGenerator::new();
    let vm_create_generator = VMCreateRequestGenerator::new();
    let vm_list_generator = VMListRequestGenerator::new();
    let pack_build_generator = PackBuildRequestGenerator::new();
    let pack_ship_generator = PackShipRequestGenerator::new();
    let invalid_pack_build_generator = InvalidPackBuildRequestGenerator::new();
    
    // Create mutators
    let login_mutator = LoginRequestMutator::new();
    let vm_create_mutator = VMCreateRequestMutator::new();
    let vm_list_mutator = VMListRequestMutator::new();
    let pack_build_mutator = PackBuildRequestMutator::new();
    let pack_ship_mutator = PackShipRequestMutator::new();
    let json_value_mutator = JsonValueMutator::new();
    
    // Load corpus or create a new one
    let corpus_dir = env::var("FUZZING_CORPUS_DIR").unwrap_or_else(|_| "fuzzing-corpus".to_string());
    let corpus_path = Path::new(&corpus_dir).join("mcp");
    
    // Create corpus directory if it doesn't exist
    fs::create_dir_all(&corpus_path).expect("Failed to create corpus directory");
    
    // Maximum number of iterations
    let max_iterations = env::var("FUZZING_MAX_ITERATIONS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1000);
    
    println!("Running MCP Server fuzzer for {} iterations", max_iterations);
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
    let mut vm_ids = Vec::new();
    let mut build_ids = Vec::new();
    let mut operation_ids = Vec::new();
    
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
        
        // Strategy 1: Fuzz authentication with valid credentials
        if i % 7 == 0 {
            total_tests += 1;
            
            // Generate a valid login request
            let login_request = login_generator.generate();
            
            // Try to login
            let result = harness.login(
                &login_request.address,
                &login_request.signed_message,
                &login_request.signature
            );
            
            match result {
                MCPOperationResult::Success(data) => {
                    successful_tests += 1;
                    
                    // Extract the token and save it for later tests
                    if let Some(token) = data.get("token").and_then(|t| t.as_str()) {
                        valid_tokens.push(token.to_string());
                        
                        // Limit the number of tokens we store
                        if valid_tokens.len() > 10 {
                            valid_tokens.remove(0);
                        }
                    }
                }
                MCPOperationResult::AuthenticationFailed => {
                    auth_failures += 1;
                    println!("Valid login request failed authentication: {:?}", login_request);
                }
                MCPOperationResult::RateLimited => {
                    rate_limit_failures += 1;
                    println!("Valid login request was rate limited");
                }
                _ => {
                    internal_failures += 1;
                    println!("Unexpected result for valid login: {:?}", result);
                }
            }
        }
        
        // Strategy 2: Fuzz authentication with invalid credentials
        else if i % 7 == 1 {
            total_tests += 1;
            
            // Generate an invalid login request or mutate a valid one
            let login_request = if rng.gen_bool(0.5) {
                invalid_login_generator.generate()
            } else {
                let mut request = login_generator.generate();
                login_mutator.mutate(&mut request);
                request
            };
            
            // Try to login
            let result = harness.login(
                &login_request.address,
                &login_request.signed_message,
                &login_request.signature
            );
            
            match result {
                MCPOperationResult::AuthenticationFailed => {
                    successful_tests += 1; // Expected result for invalid credentials
                }
                MCPOperationResult::Success(_) => {
                    auth_failures += 1;
                    println!("Invalid login request succeeded: {:?}", login_request);
                }
                _ => {
                    // Other results are acceptable for invalid credentials
                    successful_tests += 1;
                }
            }
        }
        
        // Strategy 3: Fuzz tool listing with valid token
        else if i % 7 == 2 {
            total_tests += 1;
            
            let token = if !valid_tokens.is_empty() {
                // Use a previously obtained valid token
                valid_tokens[rng.gen_range(0..valid_tokens.len())].clone()
            } else {
                // Generate a fake token (will likely fail)
                format!("fake_token_{}", rng.gen::<u32>())
            };
            
            // Decide whether to filter by category
            let use_category = rng.gen_bool(0.3);
            let category = if use_category {
                let categories = ["vm", "pack", "network", "invalid_category"];
                let idx = rng.gen_range(0..categories.len());
                Some(categories[idx])
            } else {
                None
            };
            
            // List tools
            let result = harness.list_tools(&token, category);
            
            match result {
                MCPOperationResult::Success(_) => {
                    successful_tests += 1;
                }
                MCPOperationResult::AuthenticationFailed => {
                    if valid_tokens.is_empty() {
                        successful_tests += 1; // Expected for fake token
                    } else {
                        auth_failures += 1;
                        println!("Valid token rejected for tool listing: {}", token);
                    }
                }
                _ => {
                    // For invalid categories, we might get ResourceNotFound which is fine
                    if use_category && category == Some("invalid_category") {
                        successful_tests += 1;
                    } else {
                        not_found_failures += 1;
                        println!("Unexpected result for tool listing: {:?}", result);
                    }
                }
            }
        }
        
        // Strategy 4: Fuzz VM creation
        else if i % 7 == 3 {
            total_tests += 1;
            
            let token = if !valid_tokens.is_empty() {
                // Use a previously obtained valid token
                valid_tokens[rng.gen_range(0..valid_tokens.len())].clone()
            } else {
                // Generate a fake token (will likely fail)
                format!("fake_token_{}", rng.gen::<u32>())
            };
            
            // Decide whether to use a valid request or a mutated one
            let use_valid = rng.gen_bool(0.7);
            
            let params = if use_valid {
                // Generate a valid VM creation request
                let request = vm_create_generator.generate();
                json!({
                    "name": request.name,
                    "vcpus": request.vcpus,
                    "memory_mb": request.memory_mb,
                    "disk_gb": request.disk_gb,
                })
            } else {
                // Generate a mutated request
                let mut request = vm_create_generator.generate();
                vm_create_mutator.mutate(&mut request);
                json!({
                    "name": request.name,
                    "vcpus": request.vcpus,
                    "memory_mb": request.memory_mb,
                    "disk_gb": request.disk_gb,
                })
            };
            
            // Execute VM create tool
            let result = harness.execute_tool(&token, "vm.create", params.clone());
            
            match result {
                MCPOperationResult::Success(data) => {
                    successful_tests += 1;
                    
                    // Extract the operation ID and save it for later
                    if let Some(op_id) = data.get("operation_id").and_then(|id| id.as_str()) {
                        operation_ids.push(op_id.to_string());
                        
                        // Limit the number of operation IDs we store
                        if operation_ids.len() > 20 {
                            operation_ids.remove(0);
                        }
                        
                        // Now get the operation status to get the VM ID
                        let status_result = harness.get_operation_status(&token, op_id);
                        
                        if let MCPOperationResult::Success(status_data) = status_result {
                            if let Some(result) = status_data.get("result") {
                                if let Some(vm_id) = result.get("vm_id").and_then(|id| id.as_str()) {
                                    vm_ids.push(vm_id.to_string());
                                    
                                    // Limit the number of VM IDs we store
                                    if vm_ids.len() > 10 {
                                        vm_ids.remove(0);
                                    }
                                }
                            }
                        }
                    }
                }
                MCPOperationResult::AuthenticationFailed => {
                    if valid_tokens.is_empty() {
                        successful_tests += 1; // Expected for fake token
                    } else {
                        auth_failures += 1;
                        println!("Valid token rejected for VM creation: {}", token);
                    }
                }
                MCPOperationResult::InvalidInput(_) => {
                    if !use_valid {
                        successful_tests += 1; // Expected for invalid request
                    } else {
                        invalid_input_failures += 1;
                        println!("Valid VM creation request rejected: {:?}", params);
                    }
                }
                _ => {
                    // If it was a valid request, this is a failure
                    if use_valid {
                        operation_failures += 1;
                        println!("Unexpected result for VM creation: {:?}", result);
                    } else {
                        successful_tests += 1; // Expected various errors for invalid requests
                    }
                }
            }
        }
        
        // Strategy 5: Fuzz VM listing
        else if i % 7 == 4 {
            total_tests += 1;
            
            let token = if !valid_tokens.is_empty() {
                // Use a previously obtained valid token
                valid_tokens[rng.gen_range(0..valid_tokens.len())].clone()
            } else {
                // Generate a fake token (will likely fail)
                format!("fake_token_{}", rng.gen::<u32>())
            };
            
            // Generate VM listing request
            let mut request = vm_list_generator.generate();
            
            // Decide whether to mutate the request
            if rng.gen_bool(0.3) {
                vm_list_mutator.mutate(&mut request);
            }
            
            // Execute VM list tool
            let params = json!({
                "status": request.status,
            });
            
            let result = harness.execute_tool(&token, "vm.list", params.clone());
            
            match result {
                MCPOperationResult::Success(_) => {
                    successful_tests += 1;
                }
                MCPOperationResult::AuthenticationFailed => {
                    if valid_tokens.is_empty() {
                        successful_tests += 1; // Expected for fake token
                    } else {
                        auth_failures += 1;
                        println!("Valid token rejected for VM listing: {}", token);
                    }
                }
                MCPOperationResult::InvalidInput(_) => {
                    // Could be valid for mutated requests with invalid status
                    if request.status.as_ref().map_or(false, |s| !["creating", "running", "stopped"].contains(&s.as_str())) {
                        successful_tests += 1;
                    } else {
                        invalid_input_failures += 1;
                        println!("Valid VM listing request rejected: {:?}", params);
                    }
                }
                _ => {
                    operation_failures += 1;
                    println!("Unexpected result for VM listing: {:?}", result);
                }
            }
        }
        
        // Strategy 6: Fuzz Pack Build
        else if i % 7 == 5 {
            total_tests += 1;
            
            let token = if !valid_tokens.is_empty() {
                // Use a previously obtained valid token
                valid_tokens[rng.gen_range(0..valid_tokens.len())].clone()
            } else {
                // Generate a fake token (will likely fail)
                format!("fake_token_{}", rng.gen::<u32>())
            };
            
            // Decide whether to use a valid request, invalid request, or mutated one
            let request_type = rng.gen_range(0..3);
            
            let params = match request_type {
                0 => {
                    // Valid request
                    let request = pack_build_generator.generate();
                    json!({
                        "formfile_content": request.formfile_content,
                    })
                },
                1 => {
                    // Invalid request
                    let request = invalid_pack_build_generator.generate();
                    json!({
                        "formfile_content": request.formfile_content,
                    })
                },
                _ => {
                    // Mutated request
                    let mut request = pack_build_generator.generate();
                    pack_build_mutator.mutate(&mut request);
                    json!({
                        "formfile_content": request.formfile_content,
                    })
                }
            };
            
            // Execute Pack Build tool
            let result = harness.execute_tool(&token, "form_pack_build", params.clone());
            
            match result {
                MCPOperationResult::Success(data) => {
                    successful_tests += 1;
                    
                    // Extract the operation ID and save it for later
                    if let Some(op_id) = data.get("operation_id").and_then(|id| id.as_str()) {
                        operation_ids.push(op_id.to_string());
                        
                        // Limit the number of operation IDs we store
                        if operation_ids.len() > 20 {
                            operation_ids.remove(0);
                        }
                        
                        // Now get the operation status to get the build ID
                        let status_result = harness.get_operation_status(&token, op_id);
                        
                        if let MCPOperationResult::Success(status_data) = status_result {
                            if let Some(result) = status_data.get("result") {
                                if let Some(build_id) = result.get("build_id").and_then(|id| id.as_str()) {
                                    build_ids.push(build_id.to_string());
                                    
                                    // Limit the number of build IDs we store
                                    if build_ids.len() > 10 {
                                        build_ids.remove(0);
                                    }
                                }
                            }
                        }
                    }
                }
                MCPOperationResult::AuthenticationFailed => {
                    if valid_tokens.is_empty() {
                        successful_tests += 1; // Expected for fake token
                    } else {
                        auth_failures += 1;
                        println!("Valid token rejected for Pack Build: {}", token);
                    }
                }
                MCPOperationResult::InvalidInput(_) => {
                    if request_type != 0 {
                        successful_tests += 1; // Expected for invalid or mutated request
                    } else {
                        invalid_input_failures += 1;
                        println!("Valid Pack Build request rejected: {:?}", params);
                    }
                }
                _ => {
                    // If it was a valid request, this is a failure
                    if request_type == 0 {
                        operation_failures += 1;
                        println!("Unexpected result for Pack Build: {:?}", result);
                    } else {
                        successful_tests += 1; // Expected various errors for invalid requests
                    }
                }
            }
        }
        
        // Strategy 7: Fuzz Pack Ship
        else {
            total_tests += 1;
            
            let token = if !valid_tokens.is_empty() {
                // Use a previously obtained valid token
                valid_tokens[rng.gen_range(0..valid_tokens.len())].clone()
            } else {
                // Generate a fake token (will likely fail)
                format!("fake_token_{}", rng.gen::<u32>())
            };
            
            // Decide whether to use real IDs or generated ones
            let use_real_build_id = !build_ids.is_empty() && rng.gen_bool(0.7);
            let use_real_vm_id = !vm_ids.is_empty() && rng.gen_bool(0.7);
            
            let build_id = if use_real_build_id {
                build_ids[rng.gen_range(0..build_ids.len())].clone()
            } else {
                format!("build-{}", rng.gen::<u32>())
            };
            
            let instance_id = if use_real_vm_id {
                vm_ids[rng.gen_range(0..vm_ids.len())].clone()
            } else {
                format!("vm-{}", rng.gen::<u32>())
            };
            
            // Create the request
            let mut request = pack_ship_generator.generate();
            request.build_id = build_id;
            request.instance_id = instance_id;
            
            // Decide whether to mutate the request
            if rng.gen_bool(0.3) {
                pack_ship_mutator.mutate(&mut request);
            }
            
            let params = json!({
                "build_id": request.build_id,
                "instance_id": request.instance_id,
            });
            
            // Execute Pack Ship tool
            let result = harness.execute_tool(&token, "form_pack_ship", params.clone());
            
            match result {
                MCPOperationResult::Success(_) => {
                    successful_tests += 1;
                }
                MCPOperationResult::AuthenticationFailed => {
                    if valid_tokens.is_empty() {
                        successful_tests += 1; // Expected for fake token
                    } else {
                        auth_failures += 1;
                        println!("Valid token rejected for Pack Ship: {}", token);
                    }
                }
                MCPOperationResult::ResourceNotFound => {
                    if !use_real_build_id || !use_real_vm_id {
                        successful_tests += 1; // Expected if using fake IDs
                    } else {
                        not_found_failures += 1;
                        println!("Pack Ship could not find resources: build_id={}, instance_id={}", 
                                 request.build_id, request.instance_id);
                    }
                }
                MCPOperationResult::InvalidInput(_) => {
                    // Could be valid for mutated requests
                    if request.build_id.is_empty() || request.instance_id.is_empty() {
                        successful_tests += 1;
                    } else {
                        invalid_input_failures += 1;
                        println!("Valid Pack Ship request rejected: {:?}", params);
                    }
                }
                _ => {
                    // Other results could be valid for invalid or mutated requests
                    successful_tests += 1;
                }
            }
        }
        
        // Occasionally inject faults for each type of operation
        if rng.gen_bool(0.05) {
            fault_injection::register_fault_point("mcp_auth", FaultConfig::new("mcp_auth", 0.5));
        }
        if rng.gen_bool(0.05) {
            fault_injection::register_fault_point("mcp_vm_create", FaultConfig::new("mcp_vm_create", 0.5));
        }
        if rng.gen_bool(0.05) {
            fault_injection::register_fault_point("mcp_vm_list", FaultConfig::new("mcp_vm_list", 0.5));
        }
        if rng.gen_bool(0.05) {
            fault_injection::register_fault_point("mcp_pack_build", FaultConfig::new("mcp_pack_build", 0.5));
        }
        if rng.gen_bool(0.05) {
            fault_injection::register_fault_point("mcp_pack_ship", FaultConfig::new("mcp_pack_ship", 0.5));
        }
    }
    
    // Calculate elapsed time
    let elapsed = start_time.elapsed();
    let elapsed_secs = elapsed.as_secs_f64();
    
    // Print summary
    println!("\n=================================================================");
    println!("MCP Server Fuzzing Summary:");
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
    coverage::save_coverage("mcp_fuzzer_coverage.dat");
    form_fuzzing::finalize();
} 