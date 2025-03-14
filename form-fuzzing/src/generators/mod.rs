// form-fuzzing/src/generators/mod.rs
//! Input generators for fuzz testing different components

pub mod vm_management;
pub mod network;
pub mod permissions;
pub mod vm_operations;
pub mod configuration;
pub mod api;
pub mod common;
pub mod dns;

use rand::Rng;

/// Trait for generators that can create inputs from raw bytes
pub trait BytesGenerator<T> {
    /// Generate a value from raw bytes
    fn from_bytes(data: &[u8]) -> T;
}

/// Trait for generators that can produce arbitrary values
pub trait ArbitraryGenerator<T> {
    /// Generate an arbitrary value, potentially using the provided RNG
    fn arbitrary(rng: &mut impl Rng) -> T;
}

/// Helper function to get a u8 from bytes or a default
pub fn get_u8_or_default(data: &[u8], index: usize, default: u8) -> u8 {
    data.get(index).copied().unwrap_or(default)
}

/// Helper function to get a u16 from bytes or a default
pub fn get_u16_or_default(data: &[u8], index: usize, default: u16) -> u16 {
    if index + 1 < data.len() {
        let bytes = [data[index], data[index + 1]];
        u16::from_le_bytes(bytes)
    } else {
        default
    }
}

/// Helper function to get a u32 from bytes or a default
pub fn get_u32_or_default(data: &[u8], index: usize, default: u32) -> u32 {
    if index + 3 < data.len() {
        let bytes = [data[index], data[index + 1], data[index + 2], data[index + 3]];
        u32::from_le_bytes(bytes)
    } else {
        default
    }
}

/// Helper function to get a u64 from bytes or a default
pub fn get_u64_or_default(data: &[u8], index: usize, default: u64) -> u64 {
    if index + 7 < data.len() {
        let bytes = [
            data[index], data[index + 1], data[index + 2], data[index + 3],
            data[index + 4], data[index + 5], data[index + 6], data[index + 7],
        ];
        u64::from_le_bytes(bytes)
    } else {
        default
    }
}

/// Helper function to get a boolean from a byte
pub fn get_bool(data: &[u8], index: usize, default: bool) -> bool {
    data.get(index).map(|b| *b % 2 == 0).unwrap_or(default)
}

/// Helper function to get a string from bytes
pub fn get_string(data: &[u8], start: usize, max_len: usize) -> String {
    if start >= data.len() {
        return String::new();
    }
    
    let end = std::cmp::min(start + max_len, data.len());
    let slice = &data[start..end];
    
    // Try to convert to UTF-8, falling back to a safe subset for invalid sequences
    String::from_utf8(slice.to_vec())
        .unwrap_or_else(|_| {
            slice.iter()
                .map(|b| {
                    // Map to ASCII printable range (32-126)
                    (b % 95 + 32) as char
                })
                .collect()
        })
}

// Formation Network Fuzzing Infrastructure
// Generators Module

// Submodules for different components
pub mod vm;

/// Generator trait for creating fuzzable inputs
pub trait Generator<T> {
    /// Generate a new random instance of T
    fn generate(&self) -> T;
    
    /// Generate a set of new random instances of T
    fn generate_set(&self, count: usize) -> Vec<T> {
        (0..count).map(|_| self.generate()).collect()
    }
}

/// A basic string generator that produces random strings
pub struct StringGenerator {
    min_length: usize,
    max_length: usize,
}

impl StringGenerator {
    pub fn new(min_length: usize, max_length: usize) -> Self {
        Self { min_length, max_length }
    }
}

impl Generator<String> for StringGenerator {
    fn generate(&self) -> String {
        // Simplified implementation - in a real system, this would use proper random generation
        let length = self.min_length + (self.max_length - self.min_length) / 2;
        "A".repeat(length)
    }
}

/// A basic numeric generator that produces random numbers
pub struct NumericGenerator<T> {
    min: T,
    max: T,
}

impl<T> NumericGenerator<T>
where
    T: Clone,
{
    pub fn new(min: T, max: T) -> Self {
        Self { min, max }
    }
}

impl Generator<u32> for NumericGenerator<u32> {
    fn generate(&self) -> u32 {
        // Simplified implementation - in a real system, this would use proper random generation
        (self.min + self.max) / 2
    }
} 