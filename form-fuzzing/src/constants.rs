// form-fuzzing/src/constants.rs
//! Shared constants for the fuzzing infrastructure

/// Default timeout for fuzzing operations (in milliseconds)
pub const DEFAULT_TIMEOUT_MS: u64 = 1000;

/// Default maximum memory allowed (in MB)
pub const DEFAULT_MAX_MEMORY_MB: usize = 1024;

/// Maximum number of iterations for a fuzzing run
pub const MAX_ITERATIONS: usize = 10000;

/// Default corpus directory
pub const DEFAULT_CORPUS_DIR: &str = "./fuzzing-corpus";

/// Default artifact directory
pub const DEFAULT_ARTIFACT_DIR: &str = "./fuzzing-artifacts";

/// Default coverage directory
pub const DEFAULT_COVERAGE_DIR: &str = "./fuzzing-coverage";

/// Log levels
pub mod log_levels {
    /// No logging
    pub const NONE: u8 = 0;
    /// Error logging only
    pub const ERROR: u8 = 1;
    /// Warning and error logging
    pub const WARN: u8 = 2;
    /// Info, warning, and error logging
    pub const INFO: u8 = 3;
    /// Debug and above logging
    pub const DEBUG: u8 = 4;
    /// Trace and above logging (most verbose)
    pub const TRACE: u8 = 5;
}

/// Fuzzing targets
pub mod targets {
    /// VM Management target
    pub const VM_MANAGEMENT: &str = "vm_management";
    /// Network target
    pub const NETWORK: &str = "network";
    /// DNS target
    pub const DNS: &str = "dns";
    /// MCP Server target
    pub const MCP: &str = "mcp";
    /// State Management target
    pub const STATE: &str = "state";
    /// Economic Infrastructure target
    pub const ECONOMIC: &str = "economic";
    /// P2P target
    pub const P2P: &str = "p2p";
    /// CLI target
    pub const CLI: &str = "cli";
    /// Metrics target
    pub const METRICS: &str = "metrics";
    /// Configuration target
    pub const CONFIG: &str = "config";
}

/// Fuzzing modes
pub mod modes {
    /// Standard fuzzing mode
    pub const STANDARD: &str = "standard";
    /// Quick fuzzing mode (fewer iterations)
    pub const QUICK: &str = "quick";
    /// Thorough fuzzing mode (more iterations, more checks)
    pub const THOROUGH: &str = "thorough";
    /// CI mode (optimized for continuous integration)
    pub const CI: &str = "ci";
    /// Debug mode (extra logging and checks)
    pub const DEBUG: &str = "debug";
} 