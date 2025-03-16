//! Node Metrics Fuzzer

use clap::{Parser, Subcommand};
use form_fuzzing::{
    generators::Generator,
    harness::{FuzzingHarness, NodeMetricsFuzzHarness, NodeMetricsResponse},
    instrumentation::coverage,
    mutators::node_metrics::{NodeMetricsRequestMutator, NodeCapabilitiesMutator, NodeCapacityMutator, NodeMetricsMutator},
    mutators::Mutator,
};
use form_node_metrics::{
    capabilities::NodeCapabilities,
    capacity::NodeCapacity,
    metrics::NodeMetrics,
    NodeMetricsRequest,
};
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use uuid::Uuid;

/// Command-line arguments for the node metrics fuzzer
#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Cli {
    /// Number of fuzzing iterations to run
    #[clap(short, long, default_value = "100")]
    iterations: usize,

    /// Path to the corpus directory
    #[clap(short, long, default_value = "fuzzing-corpus/node-metrics")]
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

/// Subcommands for the fuzzer
#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate a valid sample
    Generate {
        /// Output file to save the generated sample
        #[clap(short, long)]
        output: Option<String>,
    },
}

fn main() {
    println!("=== Formation Network Node Metrics Fuzzer ===");
    
    // Initialize instrumentation for coverage tracking
    let _coverage_guard = coverage::init_coverage_tracking("node_metrics");
    
    // Parse command-line arguments
    let cli = Cli::parse();
    
    // Create harness
    let mut harness = NodeMetricsFuzzHarness::new();
    harness.setup();
    
    // Create mutator
    let mutator = NodeMetricsRequestMutator::new();
    
    match &cli.command {
        Some(Commands::Generate { output }) => {
            // Generate a valid sample and save it
            let req = create_random_request();
            if let Some(output_path) = output {
                save_sample(&req, output_path);
                println!("Generated sample saved to {}", output_path);
            } else {
                println!("Generated sample (not saved)");
            }
        }
        None => {
            // Run the fuzzer for specified iterations
            run_fuzzer(&mut harness, &mutator, cli.iterations, &cli.corpus_path, cli.save_interesting, cli.save_errors);
        }
    }
    
    // Clean up
    harness.teardown();
}

fn run_fuzzer(
    harness: &mut NodeMetricsFuzzHarness,
    mutator: &NodeMetricsRequestMutator,
    iterations: usize,
    corpus_path: &str,
    save_interesting: bool,
    save_errors: bool,
) {
    println!("Running fuzzer for {} iterations", iterations);
    println!("Corpus path: {}", corpus_path);
    
    // Create corpus directory if it doesn't exist
    fs::create_dir_all(corpus_path).expect("Failed to create corpus directory");
    
    // Load existing samples from corpus
    let mut samples = load_samples(corpus_path);
    println!("Loaded {} samples from corpus", samples.len());
    
    // Statistics
    let mut total_requests = 0;
    let mut successful = 0;
    let mut errors = 0;
    let mut error_types: HashMap<String, usize> = HashMap::new();
    let mut interesting_cases = 0;
    let mut start_time = Instant::now();
    
    // Create a random number generator
    let mut rng = StdRng::from_entropy();
    
    // Run fuzzing iterations
    for i in 0..iterations {
        // Reset harness every 50 iterations
        if i > 0 && i % 50 == 0 {
            harness.reset();
            println!("Reset harness after {} iterations", i);
            // Clear tracked nodes
            samples.clear();
        }
        
        // Load samples from corpus every 10 iterations
        if i > 0 && i % 10 == 0 {
            let new_samples = load_samples(corpus_path);
            if !new_samples.is_empty() {
                samples.extend(new_samples);
            }
        }
        
        // Create a request - either mutate an existing one or generate a new one
        let req = if !samples.is_empty() && rng.gen_bool(0.7) {
            // Mutate an existing sample
            let sample_idx = rng.gen_range(0..samples.len());
            let mut req = samples[sample_idx].clone();
            mutator.mutate(&mut req);
            req
        } else {
            // Create a random request
            create_random_request()
        };
        
        // Process the request
        let result = harness.process_request(&req);
        total_requests += 1;
        
        // Handle the result
        match &result {
            NodeMetricsResponse::Success => {
                successful += 1;
                
                // If this is an interesting case, save it
                if is_interesting(&req) && save_interesting {
                    interesting_cases += 1;
                    let filename = format!("{}/interesting_{}.bin", corpus_path, Uuid::new_v4());
                    save_sample(&req, &filename);
                }
                
                // Add successful requests to our corpus for mutation
                samples.push(req);
                if samples.len() > 100 {
                    // Limit the number of samples to prevent excessive memory usage
                    samples.remove(0);
                }
            }
            NodeMetricsResponse::Error { error } => {
                errors += 1;
                
                // Track error types
                *error_types.entry(error.clone()).or_insert(0) += 1;
                
                // Save error-triggering samples if requested
                if save_errors {
                    let filename = format!("{}/error_{}.bin", corpus_path, Uuid::new_v4());
                    save_sample(&req, &filename);
                }
            }
        }
        
        // Progress reporting
        if i > 0 && i % 100 == 0 {
            let elapsed = start_time.elapsed();
            let reqs_per_sec = 100.0 / elapsed.as_secs_f64();
            println!(
                "Completed {} iterations ({:.1} reqs/sec): {} successful, {} errors",
                i, reqs_per_sec, successful, errors
            );
            start_time = Instant::now();
        }
    }
    
    // Print summary
    let total_time = Instant::now() - start_time + Duration::from_secs(1);
    println!("\n=== Fuzzing Summary ===");
    println!("Total requests: {}", total_requests);
    println!("Successful requests: {} ({:.1}%)", successful, 100.0 * successful as f64 / total_requests as f64);
    println!("Error requests: {} ({:.1}%)", errors, 100.0 * errors as f64 / total_requests as f64);
    println!("Interesting cases found: {}", interesting_cases);
    println!("Average requests/sec: {:.1}", total_requests as f64 / total_time.as_secs_f64());
    
    // Print error types if any
    if !error_types.is_empty() {
        println!("\nError Types:");
        for (error, count) in error_types.iter() {
            println!("  {} occurrences of: {}", count, error);
        }
    }
}

/// Create a random NodeMetricsRequest
fn create_random_request() -> NodeMetricsRequest {
    let mut rng = rand::thread_rng();
    
    // Generate a random node ID
    let node_id = if rng.gen_bool(0.7) {
        // Use an existing node ID pattern
        format!("node-{}", rng.gen_range(1000..10000))
    } else {
        // Generate a completely random ID
        Uuid::new_v4().to_string()
    };
    
    // Randomly choose a request type
    match rng.gen_range(0..3) {
        0 => {
            // Initial metrics
            NodeMetricsRequest::SetInitialMetrics {
                node_id,
                node_capabilities: NodeCapabilities::default(),
                node_capacity: NodeCapacity::default(),
            }
        }
        1 => {
            // Update metrics
            NodeMetricsRequest::UpdateMetrics {
                node_id,
                node_capacity: NodeCapacity::default(),
                node_metrics: NodeMetrics::default(),
            }
        }
        _ => {
            // Heartbeat
            NodeMetricsRequest::Heartbeat {
                node_id,
                timestamp: chrono::Utc::now().timestamp(),
            }
        }
    }
}

/// Determine if a request is "interesting" based on fuzzing goals
fn is_interesting(req: &NodeMetricsRequest) -> bool {
    match req {
        NodeMetricsRequest::SetInitialMetrics { node_capabilities, node_capacity, .. } => {
            // Interesting if it has unusual resource combinations
            node_capacity.cpu_total_cores > 32 || 
            node_capacity.memory_total_bytes > 1024 * 1024 * 1024 * 64 || // 64 GB
            node_capacity.gpu_total_memory_bytes > 1024 * 1024 * 1024 * 16 // 16 GB
        }
        NodeMetricsRequest::UpdateMetrics { node_metrics, .. } => {
            // Interesting if it has high utilization
            node_metrics.load_avg_1 > 5000 || // Load > 5.0
            node_metrics.cpu_temperature.unwrap_or(0) > 9000 || // 90°C
            node_metrics.disk_read_bytes_per_sec > 1024 * 1024 * 1024 || // 1 GB/s
            node_metrics.network_in_bytes_per_sec > 1024 * 1024 * 1024 // 1 GB/s
        }
        NodeMetricsRequest::Heartbeat { timestamp, .. } => {
            // Interesting if timestamp is unusual
            *timestamp <= 0 || *timestamp > chrono::Utc::now().timestamp() + 86400
        }
    }
}

/// Load samples from corpus directory
fn load_samples(corpus_path: &str) -> Vec<NodeMetricsRequest> {
    let mut samples = Vec::new();
    
    if let Ok(entries) = fs::read_dir(corpus_path) {
        for entry in entries.filter_map(Result::ok) {
            if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                continue;
            }
            
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext != "bin" {
                    continue;
                }
            } else {
                continue;
            }
            
            // In a real implementation, we would deserialize the binary file
            // but for this example, we'll just create a random request
            samples.push(create_random_request());
        }
    }
    
    samples
}

/// Save a sample to disk
fn save_sample(req: &NodeMetricsRequest, path: &str) {
    // In a real implementation, we would serialize the request to binary
    // For this example, we'll just create an empty file
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .expect("Failed to create sample file");
    
    // Write some placeholder content
    writeln!(file, "Node Metrics Request Placeholder").expect("Failed to write to file");
} 