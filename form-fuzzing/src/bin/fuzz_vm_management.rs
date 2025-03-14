// form-fuzzing/src/bin/fuzz_vm_management.rs
//! VM Management Fuzzer

use form_fuzzing::{self, constants, utils};
use form_fuzzing::generators::vm::{VMCreateRequest, VMCreateRequestGenerator};
use form_fuzzing::harness::vm_management::{VMManagementHarness, Signature, VMOperationResult};
use form_fuzzing::mutators::vm::{VMMutator, VMResourceMutator};
use form_fuzzing::mutators::Mutator;
use form_fuzzing::harness::FuzzingHarness;
use form_fuzzing::instrumentation;
use form_fuzzing::reporters;

use std::time::Instant;
use std::collections::HashMap;

fn main() {
    // Print banner
    println!("=============================================================");
    println!("| Formation Network Fuzzer - VM Management & Ownership Test |");
    println!("=============================================================");
    
    // Initialize fuzzing framework
    form_fuzzing::init();
    
    // Initialize instrumentation for coverage tracking
    let _coverage_guard = instrumentation::coverage::init_coverage_tracking(constants::targets::VM_MANAGEMENT);
    
    println!("Setting up VM Management fuzzing harness...");
    
    // Create harness
    let mut harness = VMManagementHarness::new();
    harness.setup();
    
    // Create generators and mutators
    let vm_generator = VMCreateRequestGenerator::new();
    let vm_mutator = VMMutator::new();
    let vm_resource_mutator = VMResourceMutator::new();
    
    // Get or generate corpus
    let corpus = utils::load_corpus(constants::targets::VM_MANAGEMENT);
    println!("Loaded {} corpus items", corpus.len());
    
    // Get max iterations
    let max_iterations = utils::get_max_iterations();
    println!("Running {} iterations", max_iterations);
    
    // Statistics
    let mut total_tests = 0;
    let mut success_count = 0;
    let mut signature_failures = 0;
    let mut permission_failures = 0;
    let mut resource_failures = 0;
    let mut timeout_failures = 0;
    let mut other_failures = 0;
    
    // Track time
    let start_time = Instant::now();
    
    // Test iterations
    for (i, corpus_item) in corpus.iter().enumerate().take(max_iterations) {
        if i > 0 && i % 100 == 0 {
            println!("Completed {} iterations...", i);
        }
        
        // 1. Fuzz signature verification with valid requests but invalid signatures
        {
            // Create a valid request
            let mut request = vm_generator.generate();
            
            // Create a signature but make it invalid
            let signature = Signature {
                key_id: request.user_id.clone(),
                algorithm: "ed25519".to_string(),
                timestamp: request.timestamp,
                value: b"invalid-signature".to_vec(),
            };
            
            // Test signature verification
            let result = harness.test_signature_verification(request.clone(), signature);
            
            // Track results
            total_tests += 1;
            match result {
                VMOperationResult::InvalidSignature => signature_failures += 1,
                VMOperationResult::Success => success_count += 1,
                _ => other_failures += 1,
            }
        }
        
        // 2. Fuzz VM creation with valid signatures
        {
            // Create a valid request
            let mut request = vm_generator.generate();
            
            // Create a valid signature
            let signature = Signature {
                key_id: request.user_id.clone(),
                algorithm: "ed25519".to_string(),
                timestamp: request.timestamp,
                value: format!("sig-{}", request.user_id).into_bytes(),
            };
            
            // Test VM creation
            let result = harness.test_vm_creation(request, signature);
            
            // Track results
            total_tests += 1;
            match result {
                VMOperationResult::Success => success_count += 1,
                VMOperationResult::InvalidSignature => signature_failures += 1,
                VMOperationResult::PermissionDenied => permission_failures += 1,
                VMOperationResult::ResourceError(_) => resource_failures += 1,
                VMOperationResult::Timeout => timeout_failures += 1,
                _ => other_failures += 1,
            }
        }
        
        // 3. Fuzz permission checks
        {
            let user_ids = ["user-12345", "user-67890", "invalid-user"];
            let permissions = ["vm.create", "vm.delete", "vm.invalid"];
            
            for user_id in &user_ids {
                for permission in &permissions {
                    let result = harness.test_permission_checks(user_id, permission);
                    
                    // Track results
                    total_tests += 1;
                    match result {
                        VMOperationResult::Success => success_count += 1,
                        VMOperationResult::PermissionDenied => permission_failures += 1,
                        _ => other_failures += 1,
                    }
                }
            }
        }
        
        // 4. Fuzz VM lifecycle (create and delete)
        {
            // Create requests for a VM lifecycle test
            let mut request = vm_generator.generate();
            let vm_name = format!("vm-test-{}", i);
            
            // Create signatures
            let create_signature = Signature {
                key_id: request.user_id.clone(),
                algorithm: "ed25519".to_string(),
                timestamp: request.timestamp,
                value: format!("sig-{}", request.user_id).into_bytes(),
            };
            
            let delete_signature = Signature {
                key_id: request.user_id.clone(),
                algorithm: "ed25519".to_string(),
                timestamp: request.timestamp + 1,
                value: format!("sig-delete-{}", request.user_id).into_bytes(),
            };
            
            // Convert to the internal CreateVmRequest type for the lifecycle test
            let create_vm_request = form_fuzzing::harness::vm_management::CreateVmRequest {
                name: vm_name.clone(),
                cpu_count: request.cpu_cores,
                memory_mb: request.memory_mb,
                user_id: request.user_id.clone(),
            };
            
            // Test lifecycle
            let operations = vec![
                ("create".to_string(), create_vm_request.clone(), create_signature),
                ("delete".to_string(), create_vm_request.clone(), delete_signature),
            ];
            
            let results = harness.test_vm_lifecycle(operations);
            
            // Track results
            total_tests += results.len();
            for result in results {
                match result {
                    VMOperationResult::Success => success_count += 1,
                    VMOperationResult::InvalidSignature => signature_failures += 1,
                    VMOperationResult::PermissionDenied => permission_failures += 1,
                    VMOperationResult::ResourceError(_) => resource_failures += 1,
                    VMOperationResult::Timeout => timeout_failures += 1,
                    _ => other_failures += 1,
                }
            }
        }
        
        // 5. Fuzz VM creation with extreme resource values
        {
            // Create a request with extreme resource values
            let mut request = vm_generator.generate();
            
            // Apply the resource mutator
            vm_resource_mutator.mutate(&mut request);
            
            // Create a valid signature (though resources are extreme)
            let signature = Signature {
                key_id: request.user_id.clone(),
                algorithm: "ed25519".to_string(),
                timestamp: request.timestamp,
                value: format!("sig-{}", request.user_id).into_bytes(),
            };
            
            // Test VM creation
            let result = harness.test_vm_creation(request, signature);
            
            // Track results
            total_tests += 1;
            match result {
                VMOperationResult::Success => success_count += 1,
                VMOperationResult::InvalidSignature => signature_failures += 1,
                VMOperationResult::PermissionDenied => permission_failures += 1,
                VMOperationResult::ResourceError(_) => resource_failures += 1,
                VMOperationResult::Timeout => timeout_failures += 1,
                _ => other_failures += 1,
            }
        }
    }
    
    // Calculate elapsed time
    let elapsed = start_time.elapsed();
    
    // Print summary
    println!("\n=== VM Management Fuzzing Summary ===");
    println!("Total tests:         {}", total_tests);
    println!("Successful tests:    {} ({:.1}%)", success_count, 100.0 * success_count as f64 / total_tests as f64);
    println!("Signature failures:  {} ({:.1}%)", signature_failures, 100.0 * signature_failures as f64 / total_tests as f64);
    println!("Permission failures: {} ({:.1}%)", permission_failures, 100.0 * permission_failures as f64 / total_tests as f64);
    println!("Resource failures:   {} ({:.1}%)", resource_failures, 100.0 * resource_failures as f64 / total_tests as f64);
    println!("Timeout failures:    {} ({:.1}%)", timeout_failures, 100.0 * timeout_failures as f64 / total_tests as f64);
    println!("Other failures:      {} ({:.1}%)", other_failures, 100.0 * other_failures as f64 / total_tests as f64);
    println!("Elapsed time:        {:.2?}", elapsed);
    println!("Tests per second:    {:.1}", total_tests as f64 / elapsed.as_secs_f64());
    
    // Print coverage info
    let new_coverage = _coverage_guard.new_coverage();
    println!("New edge coverage:   {}", new_coverage);
    
    // Clean up
    harness.teardown();
    
    // Save coverage data
    if let Err(e) = instrumentation::coverage::save_coverage(constants::targets::VM_MANAGEMENT) {
        eprintln!("Failed to save coverage data: {}", e);
    }
    
    // Finalize fuzzing
    form_fuzzing::finalize();
} 