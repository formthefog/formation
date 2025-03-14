// form-fuzzing/src/instrumentation/sanitizer.rs
//! Sanitizer utilities for detecting common programming errors

use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::HashMap;
use std::sync::{Arc, RwLock, Mutex};
use std::thread;

/// Global sanitizer state
static SANITIZERS_ENABLED: AtomicBool = AtomicBool::new(false);
static mut MEMORY_TRACKER: Option<Arc<RwLock<MemoryTracker>>> = None;
static mut THREAD_TRACKER: Option<Arc<Mutex<ThreadTracker>>> = None;

/// Initialize sanitizers
pub fn init() {
    SANITIZERS_ENABLED.store(true, Ordering::SeqCst);
    
    // Note: In a real implementation, this would hook into LLVM sanitizers
    // like AddressSanitizer, ThreadSanitizer, etc. But for this example
    // we'll use a lightweight approach for demonstration.
    
    unsafe {
        if MEMORY_TRACKER.is_none() {
            MEMORY_TRACKER = Some(Arc::new(RwLock::new(MemoryTracker::new())));
        }
        
        if THREAD_TRACKER.is_none() {
            THREAD_TRACKER = Some(Arc::new(Mutex::new(ThreadTracker::new())));
        }
    }
    
    println!("Sanitizers initialized");
}

/// Enable all sanitizers
pub fn enable() {
    SANITIZERS_ENABLED.store(true, Ordering::SeqCst);
    println!("Sanitizers enabled");
}

/// Disable all sanitizers
pub fn disable() {
    SANITIZERS_ENABLED.store(false, Ordering::SeqCst);
    println!("Sanitizers disabled");
}

/// Check if sanitizers are enabled
pub fn is_enabled() -> bool {
    SANITIZERS_ENABLED.load(Ordering::Relaxed)
}

/// Register a memory allocation
pub fn register_allocation(ptr: *mut u8, size: usize) {
    if !is_enabled() {
        return;
    }
    
    unsafe {
        if let Some(ref tracker) = MEMORY_TRACKER {
            if let Ok(mut tracker) = tracker.write() {
                tracker.register_allocation(ptr, size);
            }
        }
    }
}

/// Register a memory deallocation
pub fn register_deallocation(ptr: *mut u8) {
    if !is_enabled() {
        return;
    }
    
    unsafe {
        if let Some(ref tracker) = MEMORY_TRACKER {
            if let Ok(mut tracker) = tracker.write() {
                tracker.register_deallocation(ptr);
            }
        }
    }
}

/// Check for memory-related issues
pub fn check_memory(ptr: *const u8, len: usize) -> Result<(), SanitizerError> {
    if !is_enabled() {
        return Ok(());
    }
    
    // This would be a more sophisticated check in a real implementation
    if ptr.is_null() {
        return Err(SanitizerError::NullPointer);
    }
    
    unsafe {
        if let Some(ref tracker) = MEMORY_TRACKER {
            if let Ok(tracker) = tracker.read() {
                return tracker.check_memory(ptr, len);
            }
        }
    }
    
    Ok(())
}

/// Register a thread access to shared data
pub fn register_thread_access(addr: *const u8, is_write: bool) {
    if !is_enabled() {
        return;
    }
    
    let thread_id = thread::current().id();
    
    unsafe {
        if let Some(ref tracker) = THREAD_TRACKER {
            if let Ok(mut tracker) = tracker.lock() {
                tracker.register_access(addr as usize, thread_id, is_write);
            }
        }
    }
}

/// Check for thread-safety issues
pub fn check_thread_safety<T>(data: &T) -> Result<(), SanitizerError> {
    if !is_enabled() {
        return Ok(());
    }
    
    // This would do real thread-safety checks in a more sophisticated implementation
    let addr = data as *const T as *const u8;
    register_thread_access(addr, false);
    
    Ok(())
}

/// Memory tracker for detecting memory errors
#[derive(Debug)]
struct MemoryTracker {
    // Map of pointers to allocation info
    allocations: HashMap<usize, AllocationInfo>,
    // Set of freed pointers
    freed: HashMap<usize, AllocationInfo>,
}

#[derive(Debug, Clone)]
struct AllocationInfo {
    size: usize,
    stack_trace: Vec<String>, // Simplified representation
}

impl MemoryTracker {
    fn new() -> Self {
        Self {
            allocations: HashMap::new(),
            freed: HashMap::new(),
        }
    }
    
    fn register_allocation(&mut self, ptr: *mut u8, size: usize) {
        let addr = ptr as usize;
        
        // Check if this pointer was previously freed
        if self.freed.contains_key(&addr) {
            println!("Warning: Reallocating previously freed memory at {:?}", ptr);
            self.freed.remove(&addr);
        }
        
        // Add to allocations
        self.allocations.insert(addr, AllocationInfo {
            size,
            stack_trace: vec!["<stack trace not available>".to_string()],
        });
    }
    
    fn register_deallocation(&mut self, ptr: *mut u8) {
        let addr = ptr as usize;
        
        // Check if this pointer was allocated
        if let Some(info) = self.allocations.remove(&addr) {
            // Add to freed list
            self.freed.insert(addr, info);
        } else if self.freed.contains_key(&addr) {
            // Double free
            println!("Error: Double free detected at {:?}", ptr);
        } else {
            // Free of unallocated memory
            println!("Error: Freeing unallocated memory at {:?}", ptr);
        }
    }
    
    fn check_memory(&self, ptr: *const u8, len: usize) -> Result<(), SanitizerError> {
        let addr = ptr as usize;
        
        // Check if this is a valid allocation
        if let Some(info) = self.allocations.get(&addr) {
            if len > info.size {
                return Err(SanitizerError::OutOfBounds {
                    ptr,
                    len: info.size,
                    access: len,
                });
            }
        } else if self.freed.contains_key(&addr) {
            // Use after free
            return Err(SanitizerError::UseAfterFree { ptr });
        }
        
        // Check for overlapping with any freed memory
        for (freed_addr, info) in &self.freed {
            let freed_end = freed_addr + info.size;
            let access_end = addr + len;
            
            if (addr >= *freed_addr && addr < freed_end) ||
               (access_end > *freed_addr && access_end <= freed_end) {
                return Err(SanitizerError::UseAfterFree { ptr });
            }
        }
        
        Ok(())
    }
    
    fn check_leaks(&self) -> Vec<(*const u8, usize)> {
        let mut leaks = Vec::new();
        
        for (addr, info) in &self.allocations {
            leaks.push((*addr as *const u8, info.size));
        }
        
        leaks
    }
}

/// Thread tracker for detecting data races
#[derive(Debug)]
struct ThreadTracker {
    // Map of memory addresses to access info
    accesses: HashMap<usize, Vec<ThreadAccess>>,
}

#[derive(Debug, Clone)]
struct ThreadAccess {
    thread_id: thread::ThreadId,
    is_write: bool,
}

impl ThreadTracker {
    fn new() -> Self {
        Self {
            accesses: HashMap::new(),
        }
    }
    
    fn register_access(&mut self, addr: usize, thread_id: thread::ThreadId, is_write: bool) {
        let access = ThreadAccess {
            thread_id,
            is_write,
        };
        
        self.accesses.entry(addr).or_insert_with(Vec::new).push(access);
        
        // Check for data races (simplified)
        if is_write {
            let accesses = self.accesses.get(&addr).unwrap();
            if accesses.len() > 1 {
                let mut unique_threads = std::collections::HashSet::new();
                for a in accesses {
                    unique_threads.insert(format!("{:?}", a.thread_id));
                }
                
                if unique_threads.len() > 1 {
                    println!("Warning: Potential data race detected at address {:?}", addr);
                }
            }
        }
    }
}

/// Error from sanitizer checks
#[derive(Debug)]
pub enum SanitizerError {
    /// Null pointer dereference
    NullPointer,
    /// Out of bounds memory access
    OutOfBounds { ptr: *const u8, len: usize, access: usize },
    /// Use after free
    UseAfterFree { ptr: *const u8 },
    /// Double free
    DoubleFree { ptr: *const u8 },
    /// Memory leak
    MemoryLeak { ptr: *const u8, size: usize },
    /// Data race
    DataRace { addr: *const u8, thread1: String, thread2: String },
    /// Deadlock
    Deadlock { threads: Vec<String> },
}

impl std::fmt::Display for SanitizerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SanitizerError::NullPointer => write!(f, "Null pointer dereference"),
            SanitizerError::OutOfBounds { ptr, len, access } => {
                write!(f, "Out of bounds access: ptr={:?}, len={}, access={}", ptr, len, access)
            }
            SanitizerError::UseAfterFree { ptr } => {
                write!(f, "Use after free: ptr={:?}", ptr)
            }
            SanitizerError::DoubleFree { ptr } => {
                write!(f, "Double free: ptr={:?}", ptr)
            }
            SanitizerError::MemoryLeak { ptr, size } => {
                write!(f, "Memory leak: ptr={:?}, size={}", ptr, size)
            }
            SanitizerError::DataRace { addr, thread1, thread2 } => {
                write!(f, "Data race: addr={:?}, thread1={}, thread2={}", addr, thread1, thread2)
            }
            SanitizerError::Deadlock { threads } => {
                write!(f, "Deadlock detected involving threads: {:?}", threads)
            }
        }
    }
}

impl std::error::Error for SanitizerError {}

/// Wrapper function to run a code block with sanitizer checks
pub fn with_sanitizers<F, R>(f: F) -> R 
where 
    F: FnOnce() -> R 
{
    // Enable sanitizers
    let was_enabled = is_enabled();
    if !was_enabled {
        enable();
    }
    
    // Execute the function
    let result = f();
    
    // Check for leaks
    unsafe {
        if let Some(ref tracker) = MEMORY_TRACKER {
            if let Ok(tracker) = tracker.read() {
                let leaks = tracker.check_leaks();
                if !leaks.is_empty() {
                    println!("Warning: Memory leaks detected:");
                    for (ptr, size) in leaks {
                        println!("  Leak: {:?} (size: {})", ptr, size);
                    }
                }
            }
        }
    }
    
    // Restore previous state
    if !was_enabled {
        disable();
    }
    
    result
} 