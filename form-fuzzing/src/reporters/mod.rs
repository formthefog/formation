// form-fuzzing/src/reporters/mod.rs
//! Reporters for analyzing and reporting fuzzing results

pub mod coverage;
pub mod crash;
pub mod performance;
pub mod visualization;

use crate::harness::vm_management::VMOperationResult;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

/// Report results from VM management fuzzing
pub fn report_vm_management_results(operations: &[(String, VMOperationResult)]) {
    // Count operation types and results
    let mut op_counts: HashMap<String, usize> = HashMap::new();
    let mut result_counts: HashMap<String, usize> = HashMap::new();
    let mut failures: Vec<(String, String)> = Vec::new();
    
    for (op, result) in operations {
        // Count operation types
        *op_counts.entry(op.clone()).or_insert(0) += 1;
        
        // Count result types and track failures
        match result {
            VMOperationResult::Success => {
                *result_counts.entry("Success".to_string()).or_insert(0) += 1;
            }
            VMOperationResult::InvalidSignature => {
                *result_counts.entry("InvalidSignature".to_string()).or_insert(0) += 1;
                failures.push((op.clone(), "InvalidSignature".to_string()));
            }
            VMOperationResult::PermissionDenied => {
                *result_counts.entry("PermissionDenied".to_string()).or_insert(0) += 1;
                failures.push((op.clone(), "PermissionDenied".to_string()));
            }
            VMOperationResult::ResourceError(err) => {
                *result_counts.entry("ResourceError".to_string()).or_insert(0) += 1;
                failures.push((op.clone(), format!("ResourceError: {}", err)));
            }
            VMOperationResult::Timeout => {
                *result_counts.entry("Timeout".to_string()).or_insert(0) += 1;
                failures.push((op.clone(), "Timeout".to_string()));
            }
            VMOperationResult::InternalError(err) => {
                *result_counts.entry("InternalError".to_string()).or_insert(0) += 1;
                failures.push((op.clone(), format!("InternalError: {}", err)));
            }
        }
    }
    
    // Print summary
    println!("\n=== VM Management Fuzzing Results ===");
    println!("Total operations: {}", operations.len());
    
    if !op_counts.is_empty() {
        println!("\nOperation counts:");
        for (op, count) in &op_counts {
            println!("  {}: {}", op, count);
        }
    }
    
    if !result_counts.is_empty() {
        println!("\nResult counts:");
        for (result, count) in &result_counts {
            println!("  {}: {}", result, count);
        }
    }
    
    if !failures.is_empty() {
        println!("\nFailures:");
        for (op, error) in &failures {
            println!("  {}: {}", op, error);
        }
    }
    
    // Save to artifact directory if provided
    if let Some(dir) = get_artifact_dir() {
        let path = Path::new(&dir).join("vm_management_results.txt");
        if let Ok(mut file) = File::create(&path) {
            // Write summary
            writeln!(file, "=== VM Management Fuzzing Results ===").unwrap();
            writeln!(file, "Total operations: {}", operations.len()).unwrap();
            
            if !op_counts.is_empty() {
                writeln!(file, "\nOperation counts:").unwrap();
                for (op, count) in &op_counts {
                    writeln!(file, "  {}: {}", op, count).unwrap();
                }
            }
            
            if !result_counts.is_empty() {
                writeln!(file, "\nResult counts:").unwrap();
                for (result, count) in &result_counts {
                    writeln!(file, "  {}: {}", result, count).unwrap();
                }
            }
            
            if !failures.is_empty() {
                writeln!(file, "\nFailures:").unwrap();
                for (op, error) in &failures {
                    writeln!(file, "  {}: {}", op, error).unwrap();
                }
            }
            
            println!("Results saved to {}", path.display());
        }
    }
}

/// Record verification result for analysis
pub fn record_verification_result(
    request: impl std::fmt::Debug,
    signature: impl std::fmt::Debug,
    result: impl std::fmt::Debug
) {
    // In a real implementation, this would store the result for later analysis
    // For now, just log it
    if cfg!(debug_assertions) {
        println!("Verification result: {:?}", result);
    }
    
    // Save to artifact directory if provided
    if let Some(dir) = get_artifact_dir() {
        if let Ok(mut file) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(Path::new(&dir).join("verification_results.txt")) 
        {
            writeln!(file, "Request: {:?}", request).unwrap();
            writeln!(file, "Signature: {:?}", signature).unwrap();
            writeln!(file, "Result: {:?}", result).unwrap();
            writeln!(file, "-----------------------------------").unwrap();
        }
    }
}

/// Get the artifact directory, creating it if necessary
fn get_artifact_dir() -> Option<String> {
    // Check environment variable first
    let dir = std::env::var("FORM_FUZZING_ARTIFACTS").ok()
        .or_else(|| {
            // Default to ./fuzzing-artifacts
            Some("./fuzzing-artifacts".to_string())
        })?;
    
    // Create directory if it doesn't exist
    let path = Path::new(&dir);
    if !path.exists() {
        if let Err(err) = fs::create_dir_all(path) {
            eprintln!("Failed to create artifact directory {}: {}", dir, err);
            return None;
        }
    }
    
    Some(dir)
} 