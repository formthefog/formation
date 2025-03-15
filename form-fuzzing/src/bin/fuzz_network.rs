// form-fuzzing/src/bin/fuzz_network.rs
//! Network Fuzzer

use std::env;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};
use std::collections::HashMap;

use form_fuzzing::{self, constants, utils};
use form_fuzzing::generators::Generator;
use form_fuzzing::generators::network::{
    NetworkPacket, NetworkPacketGenerator, Protocol, NATConfig, 
    NATType, MappingBehavior, FilteringBehavior, P2PConnectionRequest,
    NATConfigGenerator, P2PConnectionRequestGenerator
};
use form_fuzzing::harness::network::{NetworkHarness, NetworkResult};
use form_fuzzing::harness::FuzzingHarness;
use form_fuzzing::instrumentation::coverage;
use form_fuzzing::instrumentation::fault_injection;
use form_fuzzing::instrumentation::fault_injection::FaultConfig;
use form_fuzzing::mutators::network::{
    NetworkPacketMutator, NATConfigMutator, P2PConnectionRequestMutator
};
use form_fuzzing::mutators::Mutator;

use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

fn main() {
    // Print banner
    println!("=============================================================");
    println!("|   Formation Network Fuzzer - Network Connectivity Test    |");
    println!("=============================================================");
    
    // Initialize the fuzzing framework
    form_fuzzing::init();
    
    // Initialize coverage tracking
    coverage::init();
    
    // Set up the network harness
    let mut harness = NetworkHarness::new();
    harness.setup();
    
    // Create generators
    let packet_generator = NetworkPacketGenerator::new();
    let nat_generator = NATConfigGenerator::new();
    let p2p_generator = P2PConnectionRequestGenerator::new();
    
    // Create mutators
    let packet_mutator = NetworkPacketMutator::new();
    let nat_mutator = NATConfigMutator::new();
    let p2p_mutator = P2PConnectionRequestMutator::new();
    
    // Load corpus or create a new one
    let corpus_dir = env::var("FUZZING_CORPUS_DIR").unwrap_or_else(|_| "fuzzing-corpus".to_string());
    let corpus_path = Path::new(&corpus_dir).join("network");
    
    // Create corpus directory if it doesn't exist
    fs::create_dir_all(&corpus_path).expect("Failed to create corpus directory");
    
    // Maximum number of iterations
    let max_iterations = env::var("FUZZING_MAX_ITERATIONS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1000);
        
    println!("Running network fuzzer for {} iterations", max_iterations);
    println!("Corpus directory: {}", corpus_path.display());
    
    // Create a deterministic RNG for reproducibility
    let seed = env::var("FUZZING_SEED")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(42);
    let mut rng = StdRng::seed_from_u64(seed);
    
    // Track test results
    let mut total_tests = 0;
    let mut successful_tests = 0;
    let mut nat_failures = 0;
    let mut packet_failures = 0;
    let mut p2p_failures = 0;
    let mut timeout_failures = 0;
    let mut internal_failures = 0;
    
    // Track coverage
    let mut initial_coverage = coverage::get_coverage_count();
    let mut current_coverage = initial_coverage;
    
    // Start timing
    let start_time = Instant::now();
    
    // Run the fuzzing loop
    for i in 0..max_iterations {
        // Reset fault injection for this iteration
        fault_injection::reset();
        
        // Periodically report progress
        if i % 100 == 0 && i > 0 {
            let elapsed = start_time.elapsed().as_secs_f64();
            let tests_per_second = i as f64 / elapsed;
            let new_coverage = coverage::get_coverage_count();
            println!(
                "Progress: {}/{} tests ({:.2} tests/sec), coverage: {} (+{})",
                i, max_iterations, tests_per_second, new_coverage, new_coverage - initial_coverage
            );
            current_coverage = new_coverage;
        }
        
        // Choose a testing strategy
        let strategy = if i % 3 == 0 {
            "packet_routing"
        } else if i % 3 == 1 {
            "nat_traversal"
        } else {
            "p2p_connection"
        };
        
        // Run the selected strategy
        match strategy {
            "packet_routing" => fuzz_packet_routing(&mut harness, &packet_generator, &packet_mutator, &mut rng, &mut total_tests, &mut successful_tests, &mut packet_failures, &mut internal_failures, &mut timeout_failures),
            "nat_traversal" => fuzz_nat_traversal(&mut harness, &nat_generator, &nat_mutator, &mut rng, &mut total_tests, &mut successful_tests, &mut nat_failures, &mut internal_failures, &mut timeout_failures),
            "p2p_connection" => fuzz_p2p_connection(&mut harness, &p2p_generator, &p2p_mutator, &mut rng, &mut total_tests, &mut successful_tests, &mut p2p_failures, &mut internal_failures, &mut timeout_failures),
            _ => unreachable!(),
        };
        
        // Inject random faults occasionally
        if rng.gen_bool(0.1) {
            fault_injection::register_fault_point("packet_routing", FaultConfig::new("packet_routing", 0.5));
        }
        if rng.gen_bool(0.1) {
            fault_injection::register_fault_point("nat_mapping", FaultConfig::new("nat_mapping", 0.5));
        }
        if rng.gen_bool(0.1) {
            fault_injection::register_fault_point("p2p_connect", FaultConfig::new("p2p_connect", 0.5));
        }
        if rng.gen_bool(0.1) {
            fault_injection::register_fault_point("network_packet_routing", FaultConfig::new("network_packet_routing", 0.5));
        }
        if rng.gen_bool(0.1) {
            fault_injection::register_fault_point("network_nat_traversal", FaultConfig::new("network_nat_traversal", 0.5));
        }
        if rng.gen_bool(0.1) {
            fault_injection::register_fault_point("network_p2p_connection", FaultConfig::new("network_p2p_connection", 0.5));
        }
    }
    
    // Calculate elapsed time
    let elapsed = start_time.elapsed();
    let elapsed_secs = elapsed.as_secs_f64();
    
    // Print summary
    println!("\n=================================================================");
    println!("Network Fuzzing Summary:");
    println!("=================================================================");
    println!("Total tests:           {}", total_tests);
    println!("Successful tests:      {} ({:.2}%)", successful_tests, 100.0 * successful_tests as f64 / total_tests as f64);
    println!("Packet routing failures: {}", packet_failures);
    println!("NAT traversal failures: {}", nat_failures);
    println!("P2P connection failures: {}", p2p_failures);
    println!("Timeout failures:      {}", timeout_failures);
    println!("Internal failures:     {}", internal_failures);
    println!("-----------------------------------------------------------------");
    println!("Elapsed time:          {:.2} seconds", elapsed_secs);
    println!("Tests per second:      {:.2}", total_tests as f64 / elapsed_secs);
    println!("Initial coverage:      {}", initial_coverage);
    println!("Final coverage:        {}", current_coverage);
    println!("New edge coverage:     {}", current_coverage - initial_coverage);
    println!("=================================================================");
    
    // Clean up and save coverage data
    harness.teardown();
    coverage::save_coverage("network_fuzzer_coverage.dat");
    form_fuzzing::finalize();
}

/// Fuzz packet routing
fn fuzz_packet_routing(
    harness: &mut NetworkHarness,
    generator: &NetworkPacketGenerator,
    mutator: &NetworkPacketMutator,
    rng: &mut StdRng,
    total_tests: &mut usize,
    successful_tests: &mut usize,
    packet_failures: &mut usize,
    internal_failures: &mut usize,
    timeout_failures: &mut usize,
) {
    *total_tests += 1;
    
    // Generate a packet
    let mut packet = generator.generate();
    
    // Apply mutations to the packet in ~50% of tests
    if rng.gen_bool(0.5) {
        mutator.mutate(&mut packet);
    }
    
    // Test packet routing
    let source_id = "source";
    let destination_id = "destination";
    
    let result = harness.test_packet_routing(source_id, destination_id, &packet);
    
    match result {
        NetworkResult::Success => {
            *successful_tests += 1;
        },
        NetworkResult::ConnectionFailed(_) => {
            *packet_failures += 1;
        },
        NetworkResult::Timeout => {
            *timeout_failures += 1;
        },
        _ => {
            *internal_failures += 1;
        },
    }
}

/// Fuzz NAT traversal
fn fuzz_nat_traversal(
    harness: &mut NetworkHarness,
    generator: &NATConfigGenerator,
    mutator: &NATConfigMutator,
    rng: &mut StdRng,
    total_tests: &mut usize,
    successful_tests: &mut usize,
    nat_failures: &mut usize,
    internal_failures: &mut usize,
    timeout_failures: &mut usize,
) {
    *total_tests += 1;
    
    // Generate NAT configurations
    let mut local_nat = generator.generate();
    let mut remote_nat = generator.generate();
    
    // Apply mutations to the NAT configurations in ~50% of tests
    if rng.gen_bool(0.5) {
        mutator.mutate(&mut local_nat);
    }
    if rng.gen_bool(0.5) {
        mutator.mutate(&mut remote_nat);
    }
    
    // Test NAT traversal
    let result = harness.test_nat_traversal(local_nat, remote_nat);
    
    match result {
        NetworkResult::Success => {
            *successful_tests += 1;
        },
        NetworkResult::NATTraversalFailed(_) => {
            *nat_failures += 1;
        },
        NetworkResult::Timeout => {
            *timeout_failures += 1;
        },
        _ => {
            *internal_failures += 1;
        },
    }
}

/// Fuzz P2P connection
fn fuzz_p2p_connection(
    harness: &mut NetworkHarness,
    generator: &P2PConnectionRequestGenerator,
    mutator: &P2PConnectionRequestMutator,
    rng: &mut StdRng,
    total_tests: &mut usize,
    successful_tests: &mut usize,
    p2p_failures: &mut usize,
    internal_failures: &mut usize,
    timeout_failures: &mut usize,
) {
    *total_tests += 1;
    
    // Generate a P2P connection request
    let mut request = generator.generate();
    
    // Apply mutations to the request in ~50% of tests
    if rng.gen_bool(0.5) {
        mutator.mutate(&mut request);
    }
    
    // Test P2P connection
    let result = harness.test_p2p_connection(&request);
    
    match result {
        NetworkResult::Success => {
            *successful_tests += 1;
        },
        NetworkResult::ConnectionFailed(_) | NetworkResult::NATTraversalFailed(_) => {
            *p2p_failures += 1;
        },
        NetworkResult::Timeout => {
            *timeout_failures += 1;
        },
        _ => {
            *internal_failures += 1;
        },
    }
} 