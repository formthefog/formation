// form-fuzzing/src/harness/mod.rs
//! Test harnesses for fuzzing various components

pub mod common;
pub mod dns;
pub mod vm_management;
pub mod network;
pub mod mcp;
pub mod economic;

pub use common::*;
pub use dns::*;
pub use vm_management::*;
pub use network::*;
pub use mcp::*;
pub use economic::*;

/// Trait for fuzzing harnesses
pub trait FuzzingHarness {
    /// Set up the harness for a new fuzzing run
    fn setup(&mut self);
    
    /// Clean up after a fuzzing run
    fn teardown(&mut self);
    
    /// Reset the harness to a clean state between test cases
    fn reset(&mut self);
}

/// Harness configuration options
#[derive(Debug, Clone)]
pub struct HarnessConfig {
    /// Enable verbose logging
    pub verbose: bool,
    /// Maximum time allowed for each test (in ms)
    pub timeout_ms: u64,
    /// Directory for storing test artifacts
    pub artifact_dir: Option<String>,
    /// Enable collection of code coverage
    pub collect_coverage: bool,
}

impl Default for HarnessConfig {
    fn default() -> Self {
        Self {
            verbose: false,
            timeout_ms: 1000,
            artifact_dir: None,
            collect_coverage: true,
        }
    }
}

/// Run a function with timeout protection
#[cfg(feature = "timeout")]
pub fn with_timeout<F, R>(timeout_ms: u64, f: F) -> Result<R, String>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;
    
    let (tx, rx) = mpsc::channel();
    
    let handle = thread::spawn(move || {
        let result = f();
        let _ = tx.send(result);
    });
    
    match rx.recv_timeout(Duration::from_millis(timeout_ms)) {
        Ok(result) => Ok(result),
        Err(_) => {
            // Clean up the thread
            // Note: This is a best-effort approach. In a real implementation,
            // we'd want a more robust way to cancel the operation.
            let _ = handle.join();
            Err(format!("Operation timed out after {}ms", timeout_ms))
        }
    }
}

#[cfg(not(feature = "timeout"))]
pub fn with_timeout<F, R>(timeout_ms: u64, f: F) -> Result<R, String>
where
    F: FnOnce() -> R,
{
    // When timeout feature is disabled, just run the function directly
    Ok(f())
}

/// Capture system state for comparison
#[derive(Debug, Clone, PartialEq)]
pub struct SystemState {
    // A representation of system state that can be compared
    // This would include information about resources, processes, etc.
    // For demonstration purposes, we'll use a simple placeholder
    pub memory_usage: usize,
    pub process_count: usize,
    pub open_files: usize,
    pub network_connections: usize,
}

impl SystemState {
    /// Capture the current system state
    pub fn capture() -> Self {
        // In a real implementation, this would gather actual system metrics
        Self {
            memory_usage: 0,
            process_count: 0,
            open_files: 0,
            network_connections: 0,
        }
    }
    
    /// Check if this state is consistent (no resource leaks, etc.)
    pub fn is_consistent(&self, other: &Self) -> bool {
        // In a real implementation, this would have logic to detect leaks
        // and other inconsistencies
        
        // For now, just check if they're roughly equivalent
        (self.memory_usage as i64 - other.memory_usage as i64).abs() < 1024
            && (self.process_count as i64 - other.process_count as i64).abs() < 2
            && (self.open_files as i64 - other.open_files as i64).abs() < 5
            && (self.network_connections as i64 - other.network_connections as i64).abs() < 2
    }
} 