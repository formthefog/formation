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

The following fuzzers are currently implemented:

- **VM Management**: Tests VM creation, deletion, and state transitions.
- **DNS Management**: Tests DNS record management, certificate handling, wildcard domains, and DNS propagation.
- **Network**: Tests network configuration, topology management, and firewall rules.
- **MCP Server**: Tests the Management Control Plane API including authentication, VM operations, workload building and deployment.
- **Economic Infrastructure**: Tests resource usage tracking, threshold detection, event emission, and the API layer for economic infrastructure.
- **Pack Manager and Image Builder**: Tests formfile validation, image building, package deployment, and lifecycle operations for containerized workloads.

## Usage

### Environment Variables

- `FORM_FUZZING_CORPUS_DIR`: Specifies the directory to store corpus files (default: `fuzzing-corpus/<component>`)
- `FORM_FUZZING_MAX_ITERATIONS`: Specifies the maximum number of iterations (default: 1000)
- `FORM_FUZZING_SEED`: Specifies the random seed for reproducibility (default: 42)

### Running Fuzzers

```bash
# Run the VM Management fuzzer
cargo run --bin fuzz_vm

# Run the DNS Management fuzzer
cargo run --bin fuzz_dns

# Run the Network fuzzer
cargo run --bin fuzz_network

# Run the MCP Server fuzzer
cargo run --bin fuzz_mcp

# Run the Economic Infrastructure fuzzer
cargo run --bin fuzz_economic

# Run the Pack Manager and Image Builder fuzzer
cargo run --bin fuzz_pack
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