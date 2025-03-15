// form-fuzzing/src/instrumentation/fault_injection.rs
//! Fault injection utilities for simulating failures and error conditions

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use std::time::Duration;
use rand::Rng;

/// Global state for fault injection
static ENABLED: AtomicBool = AtomicBool::new(false);
static FAULT_COUNTER: AtomicU64 = AtomicU64::new(0);
static mut FAULT_POINTS: Option<Arc<RwLock<HashMap<String, FaultConfig>>>> = None;

/// Initialize the fault injection system
pub fn init() {
    ENABLED.store(true, Ordering::SeqCst);
    unsafe {
        if FAULT_POINTS.is_none() {
            FAULT_POINTS = Some(Arc::new(RwLock::new(HashMap::new())));
            println!("Fault injection initialized");
        }
    }
}

/// Add a fault injection point with specific configuration
pub fn register_fault_point(name: &str, config: FaultConfig) {
    if !ENABLED.load(Ordering::SeqCst) {
        return;
    }
    
    unsafe {
        if let Some(ref points) = FAULT_POINTS {
            if let Ok(mut points) = points.write() {
                points.insert(name.to_string(), config);
                println!("Registered fault point: {}", name);
            }
        }
    }
}

/// Check if a fault should be triggered at a specific point
pub fn should_inject_fault(name: &str) -> bool {
    if !ENABLED.load(Ordering::SeqCst) {
        return false;
    }
    
    let counter = FAULT_COUNTER.fetch_add(1, Ordering::SeqCst);
    
    unsafe {
        if let Some(ref points) = FAULT_POINTS {
            if let Ok(points) = points.read() {
                if let Some(config) = points.get(name) {
                    return config.should_trigger(counter);
                }
            }
        }
    }
    
    false
}

/// Reset all fault injection configuration
pub fn reset() {
    FAULT_COUNTER.store(0, Ordering::SeqCst);
    
    unsafe {
        if let Some(ref points) = FAULT_POINTS {
            if let Ok(mut points) = points.write() {
                points.clear();
                println!("Fault injection configuration reset");
            }
        }
    }
}

/// Disable fault injection
pub fn disable() {
    ENABLED.store(false, Ordering::SeqCst);
    println!("Fault injection disabled");
}

/// Enable fault injection
pub fn enable() {
    ENABLED.store(true, Ordering::SeqCst);
    println!("Fault injection enabled");
}

/// Get a list of all registered fault points
pub fn list_fault_points() -> Vec<String> {
    unsafe {
        if let Some(ref points) = FAULT_POINTS {
            if let Ok(points) = points.read() {
                return points.keys().cloned().collect();
            }
        }
    }
    Vec::new()
}

/// Configuration for a fault injection point
#[derive(Debug, Clone)]
pub struct FaultConfig {
    /// Name of the fault point
    name: String,
    /// Probability of triggering (0.0 - 1.0)
    probability: f64,
    /// Only trigger after this many calls
    after_calls: Option<u64>,
    /// Only trigger every N calls
    every_n_calls: Option<u64>,
    /// Maximum number of times to trigger
    max_triggers: Option<u64>,
    /// Current trigger count
    trigger_count: u64,
    /// Delay before returning (simulates slow operations)
    delay_ms: Option<u64>,
}

impl FaultConfig {
    /// Create a new fault configuration with a specific probability
    pub fn new(name: &str, probability: f64) -> Self {
        Self {
            name: name.to_string(),
            probability,
            after_calls: None,
            every_n_calls: None,
            max_triggers: None,
            trigger_count: 0,
            delay_ms: None,
        }
    }
    
    /// Set to only trigger after a specific number of calls
    pub fn after_calls(mut self, calls: u64) -> Self {
        self.after_calls = Some(calls);
        self
    }
    
    /// Set to only trigger every N calls
    pub fn every_n_calls(mut self, n: u64) -> Self {
        self.every_n_calls = Some(n);
        self
    }
    
    /// Set maximum number of times this fault can trigger
    pub fn max_triggers(mut self, max: u64) -> Self {
        self.max_triggers = Some(max);
        self
    }
    
    /// Set a delay before returning (simulates slow operations)
    pub fn delay_ms(mut self, ms: u64) -> Self {
        self.delay_ms = Some(ms);
        self
    }
    
    /// Check if the fault should trigger on this call
    fn should_trigger(&self, counter: u64) -> bool {
        // Check if we've reached the max trigger count
        if let Some(max) = self.max_triggers {
            if self.trigger_count >= max {
                return false;
            }
        }
        
        // Check if we've reached the minimum call count
        if let Some(after) = self.after_calls {
            if counter < after {
                return false;
            }
        }
        
        // Check if this is an Nth call
        if let Some(every_n) = self.every_n_calls {
            if counter % every_n != 0 {
                return false;
            }
        }
        
        // Apply probability
        let should_trigger = rand::random::<f64>() <= self.probability;
        
        if should_trigger {
            // Apply delay if specified
            if let Some(delay_ms) = self.delay_ms {
                std::thread::sleep(Duration::from_millis(delay_ms));
            }
            
            println!("Triggering fault: {}", self.name);
        }
        
        should_trigger
    }
}

/// Helper macro for common fault injection patterns
#[macro_export]
macro_rules! inject_fault {
    ($name:expr) => {
        if $crate::instrumentation::fault_injection::should_inject_fault($name) {
            return Err($crate::instrumentation::fault_injection::FaultError::new($name).into());
        }
    };
    ($name:expr, $result:expr) => {
        if $crate::instrumentation::fault_injection::should_inject_fault($name) {
            return $result;
        }
    };
}

/// Error type for injected faults
#[derive(Debug)]
pub struct FaultError {
    pub name: String,
}

impl FaultError {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

impl std::fmt::Display for FaultError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Injected fault: {}", self.name)
    }
}

impl std::error::Error for FaultError {} 