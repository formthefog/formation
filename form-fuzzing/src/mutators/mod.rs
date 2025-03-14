// Formation Network Fuzzing Infrastructure
// Mutators Module

pub mod vm;

/// Trait for mutating fuzzable inputs
pub trait Mutator<T> {
    /// Mutate the given input
    fn mutate(&self, input: &mut T);
    
    /// Apply multiple mutations to the input
    fn mutate_multiple(&self, input: &mut T, count: usize) {
        for _ in 0..count {
            self.mutate(input);
        }
    }
}

/// A composite mutator that applies multiple mutators in sequence
pub struct CompositeMutator<T> {
    mutators: Vec<Box<dyn Mutator<T>>>,
}

impl<T> CompositeMutator<T> {
    pub fn new() -> Self {
        Self {
            mutators: Vec::new(),
        }
    }
    
    pub fn add_mutator(&mut self, mutator: Box<dyn Mutator<T>>) {
        self.mutators.push(mutator);
    }
}

impl<T> Mutator<T> for CompositeMutator<T> {
    fn mutate(&self, input: &mut T) {
        // Apply each mutator in sequence
        for mutator in &self.mutators {
            mutator.mutate(input);
        }
    }
}

/// A basic string mutator that modifies strings
pub struct StringMutator;

impl StringMutator {
    pub fn new() -> Self {
        Self
    }
}

impl Mutator<String> for StringMutator {
    fn mutate(&self, input: &mut String) {
        // Simplified mutation - in a real system, this would use proper fuzzing strategies
        if !input.is_empty() {
            if input.len() > 10 {
                *input = input[0..10].to_string();
            } else {
                *input = format!("{}extra", input);
            }
        }
    }
}

/// A basic numeric mutator that modifies numbers
pub struct NumericMutator;

impl NumericMutator {
    pub fn new() -> Self {
        Self
    }
}

impl Mutator<u32> for NumericMutator {
    fn mutate(&self, input: &mut u32) {
        // Simplified mutation - in a real system, this would use proper fuzzing strategies
        *input = *input + 1;
    }
} 