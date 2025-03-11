use sysinfo::System;
use serde::{Serialize, Deserialize};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)] 
pub struct MemoryMetrics {
    total: u64,
    free: u64,
    available: u64,
    used: u64,
    total_swap: u64,
    used_swap: u64,
}

impl MemoryMetrics {
    pub fn total(&self) -> u64 {
        self.total
    }
    
    pub fn free(&self) -> u64 {
        self.free
    }
    
    pub fn available(&self) -> u64 {
        self.available
    }
    
    pub fn used(&self) -> u64 {
        self.used
    }
    
    pub fn total_swap(&self) -> u64 {
        self.total_swap
    }
    
    pub fn used_swap(&self) -> u64 {
        self.used_swap
    }
}

pub async fn collect_memory(sys: &mut System) -> MemoryMetrics {
    sys.refresh_all();
    sys.refresh_memory();
    MemoryMetrics {
        total: sys.total_memory(),
        free: sys.free_memory(),
        available: sys.available_memory(),
        used: sys.used_memory(),
        total_swap: sys.total_swap(),
        used_swap: sys.used_swap()
    }
}
