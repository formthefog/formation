// form-fuzzing/src/instrumentation/mod.rs
//! Instrumentation utilities for code coverage tracking and performance analysis

pub mod coverage;
pub mod fault_injection;
pub mod sanitizer;

use std::sync::Once;

static INIT: Once = Once::new();

/// Initialize instrumentation with default settings
pub fn init() -> Result<(), Box<dyn std::error::Error>> {
    init_with_config()
}

/// Initialize instrumentation with custom configuration
pub fn init_with_config() -> Result<(), Box<dyn std::error::Error>> {
    INIT.call_once(|| {
        // Initialize coverage tracking
        coverage::init();
        
        // Initialize fault injection
        fault_injection::init();
        
        // Initialize sanitizers
        sanitizer::init();
    });
    
    Ok(())
}

/// Create and return a tracing guard for detailed logging
pub fn init_tracing() -> impl Drop {
    struct TracingGuard;
    
    impl Drop for TracingGuard {
        fn drop(&mut self) {
            // Clean up tracing when guard is dropped
            println!("Tracing completed");
        }
    }
    
    // Configure tracing
    println!("Initializing tracing");
    
    TracingGuard
}

/// Create an instrumentation guard for a specific code section
pub fn guard() -> impl Drop {
    struct InstrumentationGuard;
    
    impl Drop for InstrumentationGuard {
        fn drop(&mut self) {
            // Cleanup any instrumentation resources
            println!("Instrumentation guard dropped");
        }
    }
    
    println!("Creating instrumentation guard");
    
    InstrumentationGuard
} 