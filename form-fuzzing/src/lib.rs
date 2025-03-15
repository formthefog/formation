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

/// Generator modules for creating valid and invalid test inputs
pub mod generators {
    /// Generators for VM management
    pub mod vm;
    /// Generators for DNS management
    pub mod dns;
    /// Generators for network management
    pub mod network;
    /// Generators for MCP server
    pub mod mcp;
    /// Generators for economic infrastructure
    pub mod economic;
    /// Generators for pack manager and image builder
    pub mod pack;
    /// Generators for BGP/Anycast routing
    pub mod routing;
}

/// Mutator modules for modifying test inputs
pub mod mutators {
    /// Mutators for VM management
    pub mod vm;
    /// Mutators for DNS management
    pub mod dns;
    /// Mutators for network management
    pub mod network;
    /// Mutators for MCP server
    pub mod mcp;
    /// Mutators for economic infrastructure
    pub mod economic;
    /// Mutators for pack manager and image builder
    pub mod pack;
    /// Mutators for BGP/Anycast routing
    pub mod routing;
}

/// Harness modules for testing components
pub mod harness {
    /// Harness for VM management
    pub mod vm;
    /// Harness for DNS management
    pub mod dns;
    /// Harness for network management
    pub mod network;
    /// Harness for MCP server
    pub mod mcp;
    /// Harness for economic infrastructure
    pub mod economic;
    /// Harness for pack manager and image builder
    pub mod pack;
    /// Harness for BGP/Anycast routing
    pub mod routing;
}

/// Instrumentation for tracking code coverage and injecting faults
pub mod instrumentation {
    /// Code coverage tracking
    pub mod coverage;
    /// Fault injection
    pub mod fault_injection;
    /// Memory and undefined behavior sanitizer
    pub mod sanitizer;
} 