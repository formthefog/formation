// Formation Network Fuzzing Infrastructure
// Core library definition

pub mod constants;
pub mod utils;

// Core modules
pub mod generators;
pub mod harness;
pub mod instrumentation;
pub mod mutators;
pub mod reporters;

// Re-exports for convenience
pub use constants::*;
pub use utils::*;

/// Initialize the fuzzing infrastructure
/// 
/// Sets up logging, coverage tracking, and other global state for the fuzzing process.
/// Should be called at the start of each fuzzing binary.
pub fn init() {
    println!("Initializing Formation Network Fuzzing Infrastructure");
    
    // Set up logging based on environment variables
    let log_level = utils::get_log_level();
    println!("Log level: {}", log_level);
    
    // Initialize coverage tracking if enabled
    if utils::is_feature_enabled("coverage") {
        println!("Coverage tracking enabled");
        // In a real implementation, this would set up coverage tracking
    }
    
    // Print fuzzing mode
    let mode = utils::get_fuzzing_mode();
    println!("Fuzzing mode: {}", mode);
    
    println!("Fuzzing infrastructure initialized successfully");
}

/// Finalize the fuzzing process
/// 
/// Writes coverage reports, saves artifacts, and performs cleanup.
/// Should be called at the end of each fuzzing binary.
pub fn finalize() {
    println!("Finalizing fuzzing process");
    
    // Write coverage reports if enabled
    if utils::is_feature_enabled("coverage") {
        println!("Writing coverage reports");
        // In a real implementation, this would write coverage data
    }
    
    println!("Fuzzing process finalized successfully");
} 