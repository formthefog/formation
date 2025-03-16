// form-fuzzing/src/bin/fuzz_vm_metrics.rs
//! VM Metrics Fuzzer

use form_fuzzing::harness::vm_metrics::VmMetricsFuzzHarness;
use form_fuzzing::mutators::vm_metrics::VmMetricsMutator;
use form_fuzzing::mutators::Mutator;
use form_fuzzing::harness::FuzzingHarness;
use form_fuzzing::instrumentation::coverage;
use form_fuzzing::instrumentation::fault_injection;

use std::fs;
use std::path::Path;
use std::time::Instant;
use rand::prelude::*;
use clap::{Parser, Subcommand};
use form_vm_metrics::system::SystemMetrics;
use log::{info, warn, error};

/// VM Metrics fuzzer CLI
#[derive(Parser, Debug)]
#[clap(author, version, about = "Fuzzing tool for VM metrics")]
struct Cli {
    /// Number of fuzzing iterations to run
    #[clap(short, long, default_value = "100")]
    iterations: usize,

    /// Path to the corpus directory
    #[clap(short, long, default_value = "fuzzing-corpus/vm-metrics")]
    corpus_path: String,

    /// Save interesting samples
    #[clap(short, long)]
    save_interesting: bool,

    /// Save error-triggering samples
    #[clap(short, long)]
    save_errors: bool,

    /// Subcommands
    #[clap(subcommand)]
    command: Option<Commands>,
}

/// Available subcommands
#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate a valid sample
    Generate {
        /// Output file to save the generated sample
        #[clap(short, long)]
        output: Option<String>,
    },
}

/// Main entry point
fn main() {
    // Initialize logging
    env_logger::init();
    info!("Starting VM metrics fuzzer");
    
    // Parse command-line arguments
    let cli = Cli::parse();
    
    // Handle subcommands
    if let Some(commands) = cli.command {
        match commands {
            Commands::Generate { output } => {
                let harness = VmMetricsFuzzHarness::new();
                let metrics = harness.create_valid_metrics();
                
                if let Some(path) = output {
                    save_sample(&metrics, &path);
                    println!("Valid sample saved to: {}", path);
                } else {
                    println!("{:#?}", metrics);
                }
                return;
            }
        }
    }
    
    // Create corpus directory if it doesn't exist
    if !Path::new(&cli.corpus_path).exists() {
        fs::create_dir_all(&cli.corpus_path).expect("Failed to create corpus directory");
    }
    
    // Create and configure the fuzzing harness
    let mut harness = VmMetricsFuzzHarness::new();
    harness.setup();
    
    // Create the mutator
    let mutator = VmMetricsMutator::new();
    
    // Run the fuzzer
    run_fuzzer(
        &mut harness,
        &mutator,
        cli.iterations,
        &cli.corpus_path,
        cli.save_interesting,
        cli.save_errors,
    );
    
    // Clean up
    harness.teardown();
    info!("VM metrics fuzzing complete");
}

/// Run the fuzzer with the given parameters
fn run_fuzzer(
    harness: &mut VmMetricsFuzzHarness,
    mutator: &VmMetricsMutator,
    iterations: usize,
    corpus_path: &str,
    save_interesting: bool,
    save_errors: bool,
) {
    let start_time = Instant::now();
    let mut rng = rand::thread_rng();
    
    // Statistics
    let mut valid_count = 0;
    let mut error_count = 0;
    let mut interesting_count = 0;
    
    // Load existing samples from corpus
    let mut samples = load_samples(corpus_path);
    info!("Loaded {} samples from corpus", samples.len());
    
    // Track nodes seen in this run
    let mut seen_instance_ids = std::collections::HashSet::new();
    
    for i in 0..iterations {
        // Every 50 iterations, reset the harness to clear tracked state
        if i > 0 && i % 50 == 0 {
            harness.reset();
            seen_instance_ids.clear();
            info!("Reset harness at iteration {}", i);
        }
        
        // Every 10 iterations, reload samples from corpus
        if i > 0 && i % 10 == 0 {
            samples = load_samples(corpus_path);
            info!("Reloaded {} samples from corpus at iteration {}", samples.len(), i);
        }
        
        // Create metrics to test with
        let mut metrics = if !samples.is_empty() && rng.gen::<f32>() < 0.7 {
            // 70% chance to mutate an existing sample
            let sample_idx = rng.gen_range(0..samples.len());
            let mut metrics = samples[sample_idx].clone();
            mutator.mutate(&mut metrics);
            metrics
        } else {
            // 30% chance to create a new random metrics object
            create_random_metrics()
        };
        
        // Initialize coverage tracking
        let coverage_guard = coverage::init_coverage_tracking("vm_metrics");
        
        // Test the metrics
        let response = harness.publish_metrics(metrics.clone());
        
        // End code coverage region
        // Coverage guard will be dropped automatically here
        
        match response {
            form_fuzzing::harness::vm_metrics::MetricsResponse::Success => {
                valid_count += 1;
                
                // Check if we've seen this instance ID before
                if let Some(instance_id) = &metrics.instance_id {
                    if !instance_id.is_empty() {
                        seen_instance_ids.insert(instance_id.clone());
                    }
                }
                
                // Save interesting samples
                if save_interesting && is_interesting(&metrics) {
                    let path = format!("{}/interesting_{}.json", corpus_path, interesting_count);
                    save_sample(&metrics, &path);
                    interesting_count += 1;
                }
                
                // Sometimes save valid samples to corpus
                if rng.gen::<f32>() < 0.1 {
                    let path = format!("{}/valid_{}.json", corpus_path, valid_count);
                    save_sample(&metrics, &path);
                }
            }
            form_fuzzing::harness::vm_metrics::MetricsResponse::Error(msg) => {
                error_count += 1;
                
                if error_count < 10 || error_count % 100 == 0 {
                    error!("Error at iteration {}: {}", i, msg);
                }
                
                // Save error-triggering samples
                if save_errors {
                    let path = format!("{}/error_{}.json", corpus_path, error_count);
                    save_sample(&metrics, &path);
                }
            }
        }
        
        // Progress reporting
        if (i + 1) % 100 == 0 || i == iterations - 1 {
            let elapsed = start_time.elapsed().as_secs_f64();
            let rate = (i as f64 + 1.0) / elapsed;
            info!(
                "Progress: {}/{} iterations ({:.2} iter/sec), Valid: {}, Errors: {}, Interesting: {}, Unique Instance IDs: {}",
                i + 1,
                iterations,
                rate,
                valid_count,
                error_count,
                interesting_count,
                seen_instance_ids.len()
            );
        }
        
        // Introduce some randomized fault injection
        if i > 0 && i % 25 == 0 {
            // Instead of maybe_inject_fault, use should_inject_fault with a specific name
            if fault_injection::should_inject_fault("vm_metrics_fuzzer") {
                // Apply some fault behavior
                harness.set_error(Some("Injected fault".to_string()));
            }
        }
    }
    
    // Final statistics
    let total_time = start_time.elapsed().as_secs_f64();
    println!("=== VM Metrics Fuzzing Statistics ===");
    println!("Total runtime: {:.2} seconds", total_time);
    println!("Iterations: {}", iterations);
    println!("Rate: {:.2} iterations/second", iterations as f64 / total_time);
    println!("Valid inputs: {} ({:.1}%)", valid_count, 100.0 * valid_count as f64 / iterations as f64);
    println!("Error inputs: {} ({:.1}%)", error_count, 100.0 * error_count as f64 / iterations as f64);
    println!("Interesting inputs: {}", interesting_count);
    println!("Unique instance IDs: {}", seen_instance_ids.len());
}

/// Create a random metrics object
fn create_random_metrics() -> SystemMetrics {
    let mut rng = thread_rng();
    
    // Start with a valid metrics object from the harness
    let harness = VmMetricsFuzzHarness::new();
    let mut metrics = harness.create_valid_metrics();
    
    // Apply some randomization
    
    // Timestamp - use current or random past
    metrics.timestamp = if rng.gen_bool(0.8) {
        // 80% current timestamp
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    } else {
        // 20% random past time (up to 30 days ago)
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
            - rng.gen_range(0..60 * 60 * 24 * 30)
    };
    
    // Instance and account IDs
    if rng.gen_bool(0.7) {
        // 70% valid UUIDs
        metrics.instance_id = Some(uuid::Uuid::new_v4().to_string());
        metrics.account_id = Some(uuid::Uuid::new_v4().to_string());
    } else if rng.gen_bool(0.5) {
        // 15% empty
        metrics.instance_id = None;
        metrics.account_id = None;
    } else {
        // 15% invalid format
        metrics.instance_id = Some(format!("invalid-{}", rng.gen::<u32>()));
        metrics.account_id = Some(format!("invalid-{}", rng.gen::<u32>()));
    }
    
    // Randomize number of disks (0-5)
    let disk_count = rng.gen_range(0..6);
    metrics.disks.clear();
    for i in 0..disk_count {
        metrics.disks.push(form_vm_metrics::disk::DiskMetrics {
            device_name: format!("/dev/sd{}", (b'a' + i as u8) as char),
            reads_completed: rng.gen(),
            reads_merged: rng.gen(),
            sectors_read: rng.gen(),
            time_reading: rng.gen(),
            writes_completed: rng.gen(),
            writes_merged: rng.gen(),
            sectors_written: rng.gen(),
            time_writing: rng.gen(),
            io_in_progress: rng.gen_range(0..100),
            time_doing_io: rng.gen(),
            weighted_time_doing_io: rng.gen(),
        });
    }
    
    metrics
}

/// Determine if a metrics object is "interesting" for fuzzing purposes
fn is_interesting(metrics: &SystemMetrics) -> bool {
    // Check for interesting properties:
    
    // 1. Metrics from a very old time (more than 7 days)
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    
    let week_ago = now - (7 * 24 * 60 * 60);
    if metrics.timestamp < week_ago {
        return true;
    }
    
    // 2. Very high number of disks
    if metrics.disks.len() > 3 {
        return true;
    }
    
    // 3. Very high load
    if metrics.load.load1 > 500 || metrics.load.load5 > 500 || metrics.load.load15 > 500 {
        return true;
    }
    
    // 4. Metrics with multiple GPUs
    if metrics.gpus.len() > 1 {
        return true;
    }
    
    // 5. High-temperature GPUs
    for gpu in &metrics.gpus {
        if gpu.temperature_deci_c > 800 { // 80°C
            return true;
        }
    }
    
    // 6. High network activity
    for interface in &metrics.network.interfaces {
        if interface.bytes_sent > 1_000_000_000 || interface.bytes_received > 1_000_000_000 {
            return true;
        }
    }
    
    false
}

/// Load samples from the corpus directory
fn load_samples(corpus_path: &str) -> Vec<SystemMetrics> {
    let mut samples = Vec::new();
    
    let corpus_dir = Path::new(corpus_path);
    if !corpus_dir.exists() {
        return samples;
    }
    
    match fs::read_dir(corpus_dir) {
        Ok(entries) => {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("json") {
                    match fs::read_to_string(&path) {
                        Ok(contents) => {
                            match serde_json::from_str::<SystemMetrics>(&contents) {
                                Ok(metrics) => {
                                    samples.push(metrics);
                                }
                                Err(e) => {
                                    warn!("Failed to parse sample from {}: {}", path.display(), e);
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to read sample from {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }
        Err(e) => {
            error!("Failed to read corpus directory: {}", e);
        }
    }
    
    samples
}

/// Save a metrics sample to a file
fn save_sample(metrics: &SystemMetrics, path: &str) {
    match serde_json::to_string_pretty(metrics) {
        Ok(json) => {
            if let Err(e) = fs::write(path, json) {
                error!("Failed to write sample to {}: {}", path, e);
            }
        }
        Err(e) => {
            error!("Failed to serialize metrics: {}", e);
        }
    }
} 