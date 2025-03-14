// form-fuzzing/src/utils.rs
//! Utility functions for the fuzzing infrastructure

use crate::constants;
use std::path::{Path, PathBuf};
use std::fs;
use std::time::{Duration, SystemTime};
use std::env;

/// Get the fuzzing mode from environment or default to standard
pub fn get_fuzzing_mode() -> String {
    env::var("FORM_FUZZING_MODE")
        .unwrap_or_else(|_| constants::modes::STANDARD.to_string())
}

/// Get the log level from environment or default based on mode
pub fn get_log_level() -> u8 {
    let mode = get_fuzzing_mode();
    let default_level = match mode.as_str() {
        constants::modes::DEBUG => constants::log_levels::DEBUG,
        constants::modes::CI => constants::log_levels::ERROR,
        _ => constants::log_levels::INFO,
    };
    
    env::var("FORM_FUZZING_LOG_LEVEL")
        .ok()
        .and_then(|s| s.parse::<u8>().ok())
        .unwrap_or(default_level)
}

/// Get the maximum iterations based on mode
pub fn get_max_iterations() -> usize {
    let mode = get_fuzzing_mode();
    let default_iterations = match mode.as_str() {
        constants::modes::QUICK => 1000,
        constants::modes::THOROUGH => 50000,
        constants::modes::CI => 5000,
        _ => constants::MAX_ITERATIONS,
    };
    
    env::var("FORM_FUZZING_MAX_ITERATIONS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(default_iterations)
}

/// Get the corpus directory path
pub fn get_corpus_dir(target: &str) -> PathBuf {
    let base_dir = env::var("FORM_FUZZING_CORPUS_DIR")
        .unwrap_or_else(|_| constants::DEFAULT_CORPUS_DIR.to_string());
    
    let path = Path::new(&base_dir).join(target);
    
    // Create if it doesn't exist
    if !path.exists() {
        fs::create_dir_all(&path).expect("Failed to create corpus directory");
    }
    
    path
}

/// Get the artifacts directory path
pub fn get_artifacts_dir(target: &str) -> PathBuf {
    let base_dir = env::var("FORM_FUZZING_ARTIFACTS_DIR")
        .unwrap_or_else(|_| constants::DEFAULT_ARTIFACT_DIR.to_string());
    
    let path = Path::new(&base_dir).join(target);
    
    // Create if it doesn't exist
    if !path.exists() {
        fs::create_dir_all(&path).expect("Failed to create artifacts directory");
    }
    
    path
}

/// Get the coverage directory path
pub fn get_coverage_dir(target: &str) -> PathBuf {
    let base_dir = env::var("FORM_FUZZING_COVERAGE_DIR")
        .unwrap_or_else(|_| constants::DEFAULT_COVERAGE_DIR.to_string());
    
    let path = Path::new(&base_dir).join(target);
    
    // Create if it doesn't exist
    if !path.exists() {
        fs::create_dir_all(&path).expect("Failed to create coverage directory");
    }
    
    path
}

/// Get a timestamp string for file naming
pub fn get_timestamp_string() -> String {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs();
    
    format!("{}", now)
}

/// Create a unique file name with timestamp
pub fn create_timestamped_filename(prefix: &str, extension: &str) -> String {
    format!("{}_{}.{}", prefix, get_timestamp_string(), extension)
}

/// Check if a particular feature is enabled via environment variables
pub fn is_feature_enabled(feature: &str) -> bool {
    let var_name = format!("FORM_FUZZING_ENABLE_{}", feature.to_uppercase());
    env::var(var_name).map(|v| v != "0" && v != "false").unwrap_or(false)
}

/// Saves data to a corpus file for later use
pub fn save_to_corpus(target: &str, data: &[u8]) -> Result<PathBuf, std::io::Error> {
    let corpus_dir = get_corpus_dir(target);
    let filename = create_timestamped_filename("corpus", "bin");
    let path = corpus_dir.join(filename);
    
    fs::write(&path, data)?;
    Ok(path)
}

/// Load corpus files for a target
pub fn load_corpus(target: &str) -> Vec<Vec<u8>> {
    let corpus_dir = get_corpus_dir(target);
    
    // Try to read directory
    let dir = match fs::read_dir(corpus_dir) {
        Ok(dir) => dir,
        Err(_) => return Vec::new(),
    };
    
    // Load each file
    let mut corpus = Vec::new();
    for entry in dir {
        if let Ok(entry) = entry {
            if let Ok(data) = fs::read(entry.path()) {
                corpus.push(data);
            }
        }
    }
    
    if corpus.is_empty() {
        // Return a default corpus item if nothing was found
        vec![vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]]
    } else {
        corpus
    }
}

/// Calculate a size estimate for a VM based on its configuration
pub fn calculate_vm_size(cpu: u32, memory_mb: u32, disk_gb: u32) -> u64 {
    // Simple estimate formula
    let cpu_cost = (cpu as u64) * 100;
    let memory_cost = (memory_mb as u64) * 1;
    let disk_cost = (disk_gb as u64) * 10;
    
    cpu_cost + memory_cost + disk_cost
} 