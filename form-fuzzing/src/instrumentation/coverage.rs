// form-fuzzing/src/instrumentation/coverage.rs
//! Code coverage tracking utilities for measuring fuzzing effectiveness

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, RwLock};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use crate::utils;

/// Global coverage map to track which code paths have been executed
static mut COVERAGE_MAP: Option<Arc<RwLock<CoverageMap>>> = None;

/// Initialize the coverage tracking system
pub fn init() {
    unsafe {
        if COVERAGE_MAP.is_none() {
            COVERAGE_MAP = Some(Arc::new(RwLock::new(CoverageMap::new())));
            println!("Coverage tracking initialized");
        }
    }
}

/// Initialize coverage tracking for a specific fuzzing session
pub fn init_coverage_tracking(target: &str) -> CoverageGuard {
    unsafe {
        if COVERAGE_MAP.is_none() {
            init();
        }
        
        CoverageGuard {
            target: target.to_string(),
            coverage: COVERAGE_MAP.as_ref().unwrap().clone(),
            initial_count: get_coverage_count(),
        }
    }
}

/// Get the current coverage count
pub fn get_coverage_count() -> usize {
    unsafe {
        if let Some(ref map) = COVERAGE_MAP {
            if let Ok(map) = map.read() {
                return map.total_edges();
            }
        }
        0
    }
}

/// Reset the coverage map for a new fuzzing session
pub fn reset_coverage() {
    unsafe {
        if let Some(ref map) = COVERAGE_MAP {
            if let Ok(mut map) = map.write() {
                map.reset();
                println!("Coverage map reset");
            }
        }
    }
}

/// Save coverage data to a file
pub fn save_coverage(target: &str) -> io::Result<()> {
    let coverage_dir = utils::get_coverage_dir(target);
    let filename = utils::create_timestamped_filename("coverage", "json");
    let path = coverage_dir.join(filename);
    
    unsafe {
        if let Some(ref map) = COVERAGE_MAP {
            if let Ok(map) = map.read() {
                let mut file = File::create(path)?;
                let json = serde_json::to_string_pretty(&map.report())?;
                file.write_all(json.as_bytes())?;
                println!("Coverage data saved to {}", filename);
                return Ok(());
            }
        }
    }
    
    Err(io::Error::new(io::ErrorKind::Other, "Failed to access coverage map"))
}

/// Record a branch execution with unique identifier
pub fn record_branch(from: u32, to: u32) {
    unsafe {
        if let Some(ref map) = COVERAGE_MAP {
            if let Ok(mut map) = map.write() {
                map.add_edge(from, to);
            }
        }
    }
}

/// Tracks coverage information for a specific fuzzing run
pub struct CoverageGuard {
    target: String,
    coverage: Arc<RwLock<CoverageMap>>,
    initial_count: usize,
}

impl CoverageGuard {
    /// Get new coverage added during this guard's lifetime
    pub fn new_coverage(&self) -> usize {
        if let Ok(map) = self.coverage.read() {
            map.total_edges().saturating_sub(self.initial_count)
        } else {
            0
        }
    }
    
    /// Save the coverage data
    pub fn save(&self) -> io::Result<()> {
        save_coverage(&self.target)
    }
}

impl Drop for CoverageGuard {
    fn drop(&mut self) {
        let new_coverage = self.new_coverage();
        println!("Coverage guard dropped: +{} new edges", new_coverage);
        
        // Save coverage on drop if configured
        if utils::is_feature_enabled("save_coverage_on_drop") {
            if let Err(e) = self.save() {
                eprintln!("Failed to save coverage: {}", e);
            }
        }
    }
}

/// Coverage map that tracks execution paths
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CoverageMap {
    /// Maps from code locations to their destinations
    edges: HashMap<u32, HashSet<u32>>,
    /// Unique paths encountered
    paths: HashSet<(u32, u32)>,
}

impl CoverageMap {
    /// Create a new empty coverage map
    pub fn new() -> Self {
        Self {
            edges: HashMap::new(),
            paths: HashSet::new(),
        }
    }
    
    /// Add an edge to the coverage map
    pub fn add_edge(&mut self, from: u32, to: u32) {
        self.paths.insert((from, to));
        self.edges.entry(from).or_insert_with(HashSet::new).insert(to);
    }
    
    /// Get the total number of unique edges
    pub fn total_edges(&self) -> usize {
        self.paths.len()
    }
    
    /// Reset the coverage map
    pub fn reset(&mut self) {
        self.edges.clear();
        self.paths.clear();
    }
    
    /// Get a report of all covered edges
    pub fn report(&self) -> Vec<(u32, u32)> {
        self.paths.iter().copied().collect()
    }
    
    /// Merge another coverage map into this one
    pub fn merge(&mut self, other: &CoverageMap) {
        for (from, to) in &other.paths {
            self.add_edge(*from, *to);
        }
    }
    
    /// Get the coverage percentage
    pub fn coverage_percentage(&self, total_possible_edges: usize) -> f64 {
        if total_possible_edges == 0 {
            return 0.0;
        }
        (self.total_edges() as f64 / total_possible_edges as f64) * 100.0
    }
} 