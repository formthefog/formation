// form-fuzzing/src/bin/fuzz_pack.rs
//! Pack Manager and Image Builder Fuzzer

use std::env;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use form_fuzzing::generators::pack::{
    ApiKeyGenerator, InvalidApiKeyGenerator, FormfileGenerator, InvalidFormfileGenerator,
    BuildIdGenerator, VmIdGenerator, DeploymentIdGenerator,
};
use form_fuzzing::harness::pack::{
    PackHarness, PackOperationResult, Formfile, BuildStatus, DeploymentStatus,
};
use form_fuzzing::instrumentation::coverage;
use form_fuzzing::instrumentation::fault_injection;
use form_fuzzing::mutators::pack::{
    FormfileMutator, ResourcesMutator, NetworkMutator, UserMutator, 
    BuildIdMutator, VmIdMutator, DeploymentIdMutator, ApiKeyMutator,
};

use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

fn main() {
    println!("Starting Pack Manager and Image Builder Fuzzer");
    
    // Initialize coverage tracking
    coverage::init();
    
    // Initialize fault injection
    fault_injection::init();
    
    // Create pack harness
    let mut harness = PackHarness::new();
    
    // Create generators
    let api_key_generator = ApiKeyGenerator::new();
    let invalid_api_key_generator = InvalidApiKeyGenerator::new();
    let formfile_generator = FormfileGenerator::new();
    let invalid_formfile_generator = InvalidFormfileGenerator::new();
    let build_id_generator = BuildIdGenerator::new();
    let vm_id_generator = VmIdGenerator::new();
    let deployment_id_generator = DeploymentIdGenerator::new();
    
    // Create mutators
    let formfile_mutator = FormfileMutator::new();
    let resources_mutator = ResourcesMutator::new();
    let network_mutator = NetworkMutator::new();
    let user_mutator = UserMutator::new();
    let build_id_mutator = BuildIdMutator::new();
    let vm_id_mutator = VmIdMutator::new();
    let deployment_id_mutator = DeploymentIdMutator::new();
    let api_key_mutator = ApiKeyMutator::new();
    
    // Load or create corpus directory
    let corpus_dir = env::var("FORM_FUZZING_CORPUS_DIR")
        .unwrap_or_else(|_| "fuzzing-corpus/pack".to_string());
    
    if !Path::new(&corpus_dir).exists() {
        fs::create_dir_all(&corpus_dir).expect("Failed to create corpus directory");
    }
    
    // Get maximum number of iterations
    let max_iterations = env::var("FORM_FUZZING_MAX_ITERATIONS")
        .map(|s| s.parse::<usize>().unwrap_or(1000))
        .unwrap_or(1000);
    
    // Initialize RNG with a fixed seed for reproducibility
    let seed = env::var("FORM_FUZZING_SEED")
        .map(|s| s.parse::<u64>().unwrap_or(42))
        .unwrap_or(42);
    
    let mut rng = StdRng::seed_from_u64(seed);
    
    // Tracking interesting IDs for reuse
    let mut registered_users = Vec::new();
    let mut build_ids = Vec::new();
    let mut vm_ids = Vec::new();
    let mut deployment_ids = Vec::new();
    
    // Register some known users
    for _ in 0..5 {
        let (user_id, api_key) = api_key_generator.generate();
        harness.register_api_key(&user_id, &api_key);
        registered_users.push((user_id, api_key));
    }
    
    // Counters for reporting
    let mut total_tests = 0;
    let mut successful_tests = 0;
    let mut auth_failures = 0;
    let mut permission_failures = 0;
    let mut not_found_failures = 0;
    let mut invalid_input_failures = 0;
    let mut build_failures = 0;
    let mut deployment_failures = 0;
    let mut rate_limit_failures = 0;
    let mut internal_failures = 0;
    let mut timeout_failures = 0;
    
    // Start fuzzing
    let start_time = Instant::now();
    
    for i in 0..max_iterations {
        // Reset fault injection for this iteration
        fault_injection::reset();
        
        // Report progress every 100 iterations
        if i > 0 && i % 100 == 0 {
            let elapsed = start_time.elapsed();
            let tests_per_second = total_tests as f64 / elapsed.as_secs_f64();
            
            println!("Iteration {}/{} ({:.2} tests/sec)", i, max_iterations, tests_per_second);
            println!("  Total tests: {}", total_tests);
            println!("  Successes: {}", successful_tests);
            println!("  Failures: Auth={}, Permission={}, NotFound={}, InvalidInput={}, Build={}, Deployment={}, RateLimit={}, Internal={}, Timeout={}",
                auth_failures, permission_failures, not_found_failures, invalid_input_failures,
                build_failures, deployment_failures, rate_limit_failures, internal_failures, timeout_failures);
                
            // Report coverage metrics
            let (branches_hit, branches_total) = coverage::get_branch_coverage();
            let (lines_hit, lines_total) = coverage::get_line_coverage();
            
            println!("  Branch coverage: {}/{} ({:.2}%)", 
                branches_hit, branches_total, 
                (branches_hit as f64 / branches_total as f64) * 100.0);
                
            println!("  Line coverage: {}/{} ({:.2}%)", 
                lines_hit, lines_total, 
                (lines_hit as f64 / lines_total as f64) * 100.0);
        }
        
        // Choose a fuzzing strategy
        // 0: Authentication tests
        // 1: Formfile validation tests
        // 2: Build requests
        // 3: Build status requests
        // 4: Build listing
        // 5: Deploy requests
        // 6: Deployment status requests
        // 7: Deployment listing
        // 8: Build cancellation
        // 9: Build deletion
        let strategy = i % 10;
        
        match strategy {
            // Strategy 0: Authentication tests
            0 => {
                total_tests += 1;
                
                // Choose between valid and invalid credentials
                if rng.gen_bool(0.3) {
                    // Use invalid credentials
                    let (user_id, api_key) = invalid_api_key_generator.generate();
                    let formfile = formfile_generator.generate();
                    
                    let result = harness.build(&user_id, &api_key, formfile);
                    
                    if let PackOperationResult::AuthenticationFailed = result {
                        successful_tests += 1;
                    } else {
                        println!("Expected auth failure, got {:?}", result);
                    }
                } else {
                    // Use valid credentials but mutate them
                    let user_idx = rng.gen_range(0..registered_users.len());
                    let mut user_creds = registered_users[user_idx].clone();
                    
                    api_key_mutator.mutate(&mut user_creds);
                    
                    let formfile = formfile_generator.generate();
                    let result = harness.build(&user_creds.0, &user_creds.1, formfile);
                    
                    if let PackOperationResult::AuthenticationFailed = result {
                        successful_tests += 1;
                    } else {
                        println!("Mutated credentials should fail, but got {:?}", result);
                    }
                }
            },
            
            // Strategy 1: Formfile validation tests
            1 => {
                total_tests += 1;
                
                // Choose between valid, invalid, and mutated formfiles
                let test_type = rng.gen_range(0..3);
                
                let formfile = match test_type {
                    0 => formfile_generator.generate(),
                    1 => {
                        // Generate invalid formfile
                        invalid_formfile_generator.generate()
                    },
                    _ => {
                        // Generate valid formfile and mutate it
                        let mut formfile = formfile_generator.generate();
                        formfile_mutator.mutate(&mut formfile);
                        formfile
                    }
                };
                
                // Validate the formfile
                let result = harness.validate_formfile(&formfile);
                
                if test_type == 0 {
                    // Valid formfile should pass validation
                    if result.is_ok() {
                        successful_tests += 1;
                    } else {
                        println!("Valid formfile failed validation: {:?}", result);
                        invalid_input_failures += 1;
                    }
                } else {
                    // Invalid or mutated formfile should fail validation
                    if result.is_err() {
                        successful_tests += 1;
                    } else {
                        println!("Invalid formfile passed validation");
                        invalid_input_failures += 1;
                    }
                }
            },
            
            // Strategy 2: Build requests
            2 => {
                total_tests += 1;
                
                // Get a valid user
                let user_idx = rng.gen_range(0..registered_users.len());
                let (user_id, api_key) = &registered_users[user_idx];
                
                // Choose formfile type
                let test_type = rng.gen_range(0..3);
                
                let formfile = match test_type {
                    0 => formfile_generator.generate(),
                    1 => invalid_formfile_generator.generate(),
                    _ => {
                        let mut formfile = formfile_generator.generate();
                        formfile_mutator.mutate(&mut formfile);
                        formfile
                    }
                };
                
                // Make build request
                let result = harness.build(user_id, api_key, formfile);
                
                match result {
                    PackOperationResult::Success => {
                        if test_type == 0 {
                            successful_tests += 1;
                            
                            // Get builds and save build IDs for later use
                            if let Ok(builds) = harness.list_builds(user_id, api_key) {
                                if !builds.is_empty() {
                                    let build = &builds[builds.len() - 1];
                                    build_ids.push(build.build_id.clone());
                                    
                                    // Limit number of saved build IDs
                                    if build_ids.len() > 10 {
                                        build_ids.remove(0);
                                    }
                                }
                            }
                        } else {
                            println!("Invalid formfile accepted for build");
                            invalid_input_failures += 1;
                        }
                    },
                    PackOperationResult::InvalidInput(_) => {
                        if test_type != 0 {
                            successful_tests += 1;
                        } else {
                            println!("Valid formfile rejected: {:?}", result);
                            invalid_input_failures += 1;
                        }
                    },
                    PackOperationResult::AuthenticationFailed => {
                        auth_failures += 1;
                        println!("Authentication failed for valid user");
                    },
                    PackOperationResult::PermissionDenied => {
                        permission_failures += 1;
                        println!("Permission denied for valid user");
                    },
                    PackOperationResult::RateLimited => {
                        rate_limit_failures += 1;
                        successful_tests += 1; // Expected result in some cases
                    },
                    PackOperationResult::BuildFailed(reason) => {
                        build_failures += 1;
                        if test_type != 0 {
                            successful_tests += 1; // Expected for invalid inputs
                        } else {
                            println!("Build failed for valid formfile: {}", reason);
                        }
                    },
                    _ => {
                        internal_failures += 1;
                        println!("Unexpected result: {:?}", result);
                    }
                }
            },
            
            // Strategy 3: Build status requests
            3 => {
                total_tests += 1;
                
                // Get a valid user
                let user_idx = rng.gen_range(0..registered_users.len());
                let (user_id, api_key) = &registered_users[user_idx];
                
                // Choose build ID type
                let build_id = if !build_ids.is_empty() && rng.gen_bool(0.7) {
                    // Use existing build ID
                    let idx = rng.gen_range(0..build_ids.len());
                    build_ids[idx].clone()
                } else {
                    // Generate new or mutated build ID
                    if rng.gen_bool(0.5) {
                        build_id_generator.generate()
                    } else {
                        let mut build_id = build_id_generator.generate();
                        build_id_mutator.mutate(&mut build_id);
                        build_id
                    }
                };
                
                // Get build status
                let result = harness.get_build_status(user_id, api_key, &build_id);
                
                match result {
                    Ok(status) => {
                        if build_ids.contains(&build_id) {
                            successful_tests += 1;
                            
                            // Save build ID for later if it's completed
                            if status == BuildStatus::Completed {
                                if !build_ids.contains(&build_id) {
                                    build_ids.push(build_id);
                                    
                                    if build_ids.len() > 10 {
                                        build_ids.remove(0);
                                    }
                                }
                            }
                        } else {
                            println!("Got status for nonexistent build ID: {:?}", status);
                            not_found_failures += 1;
                        }
                    },
                    Err(PackOperationResult::ResourceNotFound) => {
                        if !build_ids.contains(&build_id) {
                            successful_tests += 1;
                        } else {
                            println!("Build not found for existing ID: {}", build_id);
                            not_found_failures += 1;
                        }
                    },
                    Err(PackOperationResult::AuthenticationFailed) => {
                        auth_failures += 1;
                        println!("Authentication failed for valid user");
                    },
                    Err(PackOperationResult::PermissionDenied) => {
                        permission_failures += 1;
                        // Could be expected if the build belongs to another user
                        successful_tests += 1;
                    },
                    Err(e) => {
                        internal_failures += 1;
                        println!("Unexpected error: {:?}", e);
                    }
                }
            },
            
            // Strategy 4: Build listing
            4 => {
                total_tests += 1;
                
                // Get a valid user
                let user_idx = rng.gen_range(0..registered_users.len());
                let (user_id, api_key) = &registered_users[user_idx];
                
                // List builds
                let result = harness.list_builds(user_id, api_key);
                
                match result {
                    Ok(builds) => {
                        successful_tests += 1;
                        
                        // Save build IDs for later use
                        for build in builds {
                            if !build_ids.contains(&build.build_id) {
                                build_ids.push(build.build_id);
                                
                                if build_ids.len() > 10 {
                                    build_ids.remove(0);
                                }
                            }
                        }
                    },
                    Err(PackOperationResult::AuthenticationFailed) => {
                        auth_failures += 1;
                        println!("Authentication failed for valid user");
                    },
                    Err(e) => {
                        internal_failures += 1;
                        println!("Unexpected error: {:?}", e);
                    }
                }
            },
            
            // Strategy 5: Deploy requests
            5 => {
                total_tests += 1;
                
                // Get a valid user
                let user_idx = rng.gen_range(0..registered_users.len());
                let (user_id, api_key) = &registered_users[user_idx];
                
                // Choose build ID
                let build_id = if !build_ids.is_empty() && rng.gen_bool(0.7) {
                    // Use existing build ID
                    let idx = rng.gen_range(0..build_ids.len());
                    build_ids[idx].clone()
                } else {
                    // Generate new or invalid build ID
                    if rng.gen_bool(0.5) {
                        build_id_generator.generate()
                    } else {
                        let mut build_id = build_id_generator.generate();
                        build_id_mutator.mutate(&mut build_id);
                        build_id
                    }
                };
                
                // Generate VM ID
                let vm_id = if !vm_ids.is_empty() && rng.gen_bool(0.3) {
                    // Use existing VM ID
                    let idx = rng.gen_range(0..vm_ids.len());
                    vm_ids[idx].clone()
                } else {
                    // Generate new VM ID
                    let vm_id = vm_id_generator.generate();
                    
                    // Add to list
                    if !vm_ids.contains(&vm_id) {
                        vm_ids.push(vm_id.clone());
                        
                        if vm_ids.len() > 10 {
                            vm_ids.remove(0);
                        }
                    }
                    
                    vm_id
                };
                
                // Make deployment request
                let result = harness.deploy(user_id, api_key, &build_id, &vm_id);
                
                match result {
                    PackOperationResult::Success => {
                        if build_ids.contains(&build_id) {
                            successful_tests += 1;
                            
                            // Get deployments and save IDs for later use
                            if let Ok(deployments) = harness.list_deployments(user_id, api_key) {
                                if !deployments.is_empty() {
                                    let deployment = &deployments[deployments.len() - 1];
                                    deployment_ids.push(deployment.deployment_id.clone());
                                    
                                    // Limit number of saved deployment IDs
                                    if deployment_ids.len() > 10 {
                                        deployment_ids.remove(0);
                                    }
                                }
                            }
                        } else {
                            println!("Deployed with nonexistent build ID");
                            not_found_failures += 1;
                        }
                    },
                    PackOperationResult::ResourceNotFound => {
                        if !build_ids.contains(&build_id) {
                            successful_tests += 1;
                        } else {
                            println!("Build not found for existing ID: {}", build_id);
                            not_found_failures += 1;
                        }
                    },
                    PackOperationResult::InvalidInput(_) => {
                        if build_id.is_empty() || vm_id.is_empty() {
                            successful_tests += 1;
                        } else {
                            invalid_input_failures += 1;
                            println!("Invalid input for valid build/VM IDs");
                        }
                    },
                    PackOperationResult::AuthenticationFailed => {
                        auth_failures += 1;
                        println!("Authentication failed for valid user");
                    },
                    PackOperationResult::PermissionDenied => {
                        permission_failures += 1;
                        // Could be expected if the build belongs to another user
                        successful_tests += 1;
                    },
                    PackOperationResult::DeploymentFailed(reason) => {
                        deployment_failures += 1;
                        println!("Deployment failed: {}", reason);
                    },
                    _ => {
                        internal_failures += 1;
                        println!("Unexpected result: {:?}", result);
                    }
                }
            },
            
            // Strategy 6: Deployment status requests
            6 => {
                total_tests += 1;
                
                // Get a valid user
                let user_idx = rng.gen_range(0..registered_users.len());
                let (user_id, api_key) = &registered_users[user_idx];
                
                // Choose deployment ID
                let deployment_id = if !deployment_ids.is_empty() && rng.gen_bool(0.7) {
                    // Use existing deployment ID
                    let idx = rng.gen_range(0..deployment_ids.len());
                    deployment_ids[idx].clone()
                } else {
                    // Generate new or invalid deployment ID
                    if rng.gen_bool(0.5) {
                        deployment_id_generator.generate()
                    } else {
                        let mut deployment_id = deployment_id_generator.generate();
                        deployment_id_mutator.mutate(&mut deployment_id);
                        deployment_id
                    }
                };
                
                // Get deployment status
                let result = harness.get_deployment_status(user_id, api_key, &deployment_id);
                
                match result {
                    Ok(status) => {
                        if deployment_ids.contains(&deployment_id) {
                            successful_tests += 1;
                            
                            // Save deployment ID for later if it's completed
                            if status == DeploymentStatus::Completed {
                                if !deployment_ids.contains(&deployment_id) {
                                    deployment_ids.push(deployment_id);
                                    
                                    if deployment_ids.len() > 10 {
                                        deployment_ids.remove(0);
                                    }
                                }
                            }
                        } else {
                            println!("Got status for nonexistent deployment ID: {:?}", status);
                            not_found_failures += 1;
                        }
                    },
                    Err(PackOperationResult::ResourceNotFound) => {
                        if !deployment_ids.contains(&deployment_id) {
                            successful_tests += 1;
                        } else {
                            println!("Deployment not found for existing ID: {}", deployment_id);
                            not_found_failures += 1;
                        }
                    },
                    Err(PackOperationResult::AuthenticationFailed) => {
                        auth_failures += 1;
                        println!("Authentication failed for valid user");
                    },
                    Err(PackOperationResult::PermissionDenied) => {
                        permission_failures += 1;
                        // Could be expected if the deployment belongs to another user
                        successful_tests += 1;
                    },
                    Err(e) => {
                        internal_failures += 1;
                        println!("Unexpected error: {:?}", e);
                    }
                }
            },
            
            // Strategy 7: Deployment listing
            7 => {
                total_tests += 1;
                
                // Get a valid user
                let user_idx = rng.gen_range(0..registered_users.len());
                let (user_id, api_key) = &registered_users[user_idx];
                
                // List deployments
                let result = harness.list_deployments(user_id, api_key);
                
                match result {
                    Ok(deployments) => {
                        successful_tests += 1;
                        
                        // Save deployment IDs for later use
                        for deployment in deployments {
                            if !deployment_ids.contains(&deployment.deployment_id) {
                                deployment_ids.push(deployment.deployment_id);
                                
                                if deployment_ids.len() > 10 {
                                    deployment_ids.remove(0);
                                }
                            }
                        }
                    },
                    Err(PackOperationResult::AuthenticationFailed) => {
                        auth_failures += 1;
                        println!("Authentication failed for valid user");
                    },
                    Err(e) => {
                        internal_failures += 1;
                        println!("Unexpected error: {:?}", e);
                    }
                }
            },
            
            // Strategy 8: Build cancellation
            8 => {
                total_tests += 1;
                
                // Get a valid user
                let user_idx = rng.gen_range(0..registered_users.len());
                let (user_id, api_key) = &registered_users[user_idx];
                
                // Choose build ID
                let build_id = if !build_ids.is_empty() && rng.gen_bool(0.7) {
                    // Use existing build ID
                    let idx = rng.gen_range(0..build_ids.len());
                    build_ids[idx].clone()
                } else {
                    // Generate new or invalid build ID
                    if rng.gen_bool(0.5) {
                        build_id_generator.generate()
                    } else {
                        let mut build_id = build_id_generator.generate();
                        build_id_mutator.mutate(&mut build_id);
                        build_id
                    }
                };
                
                // Cancel build
                let result = harness.cancel_build(user_id, api_key, &build_id);
                
                match result {
                    PackOperationResult::Success => {
                        if build_ids.contains(&build_id) {
                            successful_tests += 1;
                        } else {
                            println!("Cancelled nonexistent build ID");
                            not_found_failures += 1;
                        }
                    },
                    PackOperationResult::ResourceNotFound => {
                        if !build_ids.contains(&build_id) {
                            successful_tests += 1;
                        } else {
                            println!("Build not found for existing ID: {}", build_id);
                            not_found_failures += 1;
                        }
                    },
                    PackOperationResult::InvalidInput(_) => {
                        // Could be valid if the build is already completed
                        successful_tests += 1;
                    },
                    PackOperationResult::AuthenticationFailed => {
                        auth_failures += 1;
                        println!("Authentication failed for valid user");
                    },
                    PackOperationResult::PermissionDenied => {
                        permission_failures += 1;
                        // Could be expected if the build belongs to another user
                        successful_tests += 1;
                    },
                    _ => {
                        internal_failures += 1;
                        println!("Unexpected result: {:?}", result);
                    }
                }
            },
            
            // Strategy 9: Build deletion
            9 => {
                total_tests += 1;
                
                // Get a valid user
                let user_idx = rng.gen_range(0..registered_users.len());
                let (user_id, api_key) = &registered_users[user_idx];
                
                // Choose build ID
                let build_id = if !build_ids.is_empty() && rng.gen_bool(0.7) {
                    // Use existing build ID
                    let idx = rng.gen_range(0..build_ids.len());
                    let id = build_ids[idx].clone();
                    
                    // Remove from list if used
                    if rng.gen_bool(0.8) {
                        build_ids.remove(idx);
                    }
                    
                    id
                } else {
                    // Generate new or invalid build ID
                    if rng.gen_bool(0.5) {
                        build_id_generator.generate()
                    } else {
                        let mut build_id = build_id_generator.generate();
                        build_id_mutator.mutate(&mut build_id);
                        build_id
                    }
                };
                
                // Delete build
                let result = harness.delete_build(user_id, api_key, &build_id);
                
                match result {
                    PackOperationResult::Success => {
                        if build_ids.contains(&build_id) {
                            successful_tests += 1;
                            
                            // Remove from build_ids if still there
                            if let Some(idx) = build_ids.iter().position(|id| id == &build_id) {
                                build_ids.remove(idx);
                            }
                        } else {
                            println!("Deleted nonexistent build ID");
                            not_found_failures += 1;
                        }
                    },
                    PackOperationResult::ResourceNotFound => {
                        if !build_ids.contains(&build_id) {
                            successful_tests += 1;
                        } else {
                            println!("Build not found for existing ID: {}", build_id);
                            not_found_failures += 1;
                        }
                    },
                    PackOperationResult::AuthenticationFailed => {
                        auth_failures += 1;
                        println!("Authentication failed for valid user");
                    },
                    PackOperationResult::PermissionDenied => {
                        permission_failures += 1;
                        // Could be expected if the build belongs to another user
                        successful_tests += 1;
                    },
                    _ => {
                        internal_failures += 1;
                        println!("Unexpected result: {:?}", result);
                    }
                }
            },
            
            _ => {}
        }
        
        // Occasionally inject faults for each type of operation
        if rng.gen_bool(0.05) {
            fault_injection::register_fault_point("pack_auth", 0.5);
        }
        if rng.gen_bool(0.05) {
            fault_injection::register_fault_point("pack_build", 0.5);
        }
        if rng.gen_bool(0.05) {
            fault_injection::register_fault_point("pack_deploy", 0.5);
        }
    }
    
    // Final report
    let elapsed = start_time.elapsed();
    let tests_per_second = total_tests as f64 / elapsed.as_secs_f64();
    
    println!("Fuzzing completed in {:.2?}", elapsed);
    println!("Total tests: {}", total_tests);
    println!("Successes: {}", successful_tests);
    println!("Failures: Auth={}, Permission={}, NotFound={}, InvalidInput={}, Build={}, Deployment={}, RateLimit={}, Internal={}, Timeout={}",
        auth_failures, permission_failures, not_found_failures, invalid_input_failures,
        build_failures, deployment_failures, rate_limit_failures, internal_failures, timeout_failures);
    println!("Tests per second: {:.2}", tests_per_second);
    
    // Report coverage metrics
    let (branches_hit, branches_total) = coverage::get_branch_coverage();
    let (lines_hit, lines_total) = coverage::get_line_coverage();
    
    println!("Branch coverage: {}/{} ({:.2}%)", 
        branches_hit, branches_total, 
        (branches_hit as f64 / branches_total as f64) * 100.0);
        
    println!("Line coverage: {}/{} ({:.2}%)", 
        lines_hit, lines_total, 
        (lines_hit as f64 / lines_total as f64) * 100.0);
    
    // Save coverage data
    coverage::save_to_file("pack_fuzzer_coverage.dat").expect("Failed to save coverage data");
    
    // Finalize fuzzing framework
    coverage::finalize();
    fault_injection::finalize();
} 