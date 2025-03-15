# Formation Network Fuzzing Infrastructure

This crate provides a comprehensive fuzzing infrastructure for the Formation Network, enabling robust testing of all core components.

## Overview

The Formation Network fuzzing infrastructure is designed to provide thorough testing of all key components, finding edge cases, security vulnerabilities, and reliability issues before they affect production systems.

## Key Features

- **Comprehensive Coverage**: Tests all core components of the Formation Network
- **Modular Architecture**: Easily extensible to new components
- **Multiple Fuzzing Strategies**: Combines various techniques for thorough testing
- **Coverage-Guided**: Uses code coverage to focus on unexplored code paths
- **Fault Injection**: Simulates failures to test error handling
- **CI/CD Integration**: Designed to run in continuous integration pipelines

## Components

The fuzzing infrastructure consists of several key modules:

- **Instrumentation**: Tools for tracking code coverage and monitoring execution
- **Generators**: Smart input generators for various data types and protocols
- **Mutators**: Input mutation strategies for finding edge cases
- **Harnesses**: Test environments for specific components
- **Reporters**: Analysis and reporting of fuzzing results

## Available Fuzzers

The following fuzzers are available:

- **VM Management Fuzzer**: Tests VM creation, deletion, and management operations.
- **DNS Management Fuzzer**: Tests DNS record creation, updates, and resolution.
- **Network Fuzzer**: Tests network configuration, routing, and connectivity.
- **MCP Server Fuzzer**: Tests the Management Control Plane server API.
- **Economic Fuzzer**: Tests the economic infrastructure components.
- **Pack Manager Fuzzer**: Tests the Pack Manager and Image Builder components.
- **BGP/Anycast Routing Fuzzer**: Tests BGP announcements, GeoDNS resolution, health tracking, and anycast routing.
- **P2P Message Queue Fuzzer**: Tests P2P message publishing, topic subscription, message routing, and network conditions.

## Usage

### Environment Variables

- `FORM_FUZZING_CORPUS_DIR`: Specifies the directory to store corpus files (default: `fuzzing-corpus/<component>`)
- `FORM_FUZZING_MAX_ITERATIONS`: Specifies the maximum number of iterations (default: 1000)
- `FORM_FUZZING_SEED`: Specifies the random seed for reproducibility (default: 42)

### Running Fuzzers

```bash
# Build the project
cargo build

# Run a specific fuzzer
cargo run --bin fuzz_vm          # Run VM management fuzzer
cargo run --bin fuzz_dns         # Run DNS management fuzzer
cargo run --bin fuzz_network     # Run network fuzzer
cargo run --bin fuzz_mcp         # Run MCP server fuzzer
cargo run --bin fuzz_economic    # Run economic infrastructure fuzzer
cargo run --bin fuzz_pack        # Run Pack Manager fuzzer
cargo run --bin fuzz_routing     # Run BGP/Anycast routing fuzzer
cargo run --bin fuzz_p2p         # Run P2P message queue fuzzer
```

### Integrating into CI/CD

Add the following to your GitHub Actions workflow:

```yaml
- name: Run Fuzzers
  run: |
    cargo run --bin fuzz_vm
    cargo run --bin fuzz_dns
    cargo run --bin fuzz_network
    cargo run --bin fuzz_mcp
    cargo run --bin fuzz_economic
    cargo run --bin fuzz_pack
    cargo run --bin fuzz_routing
    cargo run --bin fuzz_p2p
```

## Environment Variables

- `FORM_FUZZING_MODE`: Fuzzing mode (quick, standard, thorough, ci, debug)
- `FORM_FUZZING_MAX_ITERATIONS`: Maximum number of fuzzing iterations
- `FORM_FUZZING_LOG_LEVEL`: Log verbosity (0-5)
- `FORM_FUZZING_CORPUS_DIR`: Directory for storing corpus files
- `FORM_FUZZING_ARTIFACTS_DIR`: Directory for storing fuzzing artifacts
- `FORM_FUZZING_COVERAGE_DIR`: Directory for storing coverage information
- `FORM_FUZZING_ENABLE_*`: Enable specific features (e.g., FORM_FUZZING_ENABLE_TIMEOUT=1)

## Implementation Plan

This implementation follows the comprehensive fuzzing plan developed for the Formation Network. For more details, see the fuzzing-tasks directory in the project root.

## Contributing

To add a new fuzzer:

1. Add generators in `src/generators/<component>.rs`
2. Add mutators in `src/mutators/<component>.rs`
3. Add a harness in `src/harness/<component>.rs`
4. Add a fuzzer binary in `src/bin/fuzz_<component>.rs`
5. Update this README.md

## License

MIT OR Apache-2.0 