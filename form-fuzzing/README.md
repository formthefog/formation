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

Currently implemented fuzzers:

- **VM Management**: Tests VM creation, deletion, and ownership verification
- **DNS Management**: Tests DNS zone creation, record management, certificate handling, wildcard domains, and DNS propagation
- **Network**: Tests packet routing, NAT traversal, and P2P connectivity with various network conditions
- **MCP Server**: Tests the Management Control Plane API, including authentication, VM operations, workload building, and deployment
- **Economic Infrastructure**: Tests resource usage tracking, threshold detection, event emission, and the API layer for economic infrastructure
- *(More will be added according to the implementation plan)*

## Usage

### Running a Specific Fuzzer

```bash
# Set environment variables (optional)
export FORM_FUZZING_MODE=quick  # Options: quick, standard, thorough, ci, debug
export FORM_FUZZING_MAX_ITERATIONS=1000
export FORM_FUZZING_CORPUS_DIR=./my-corpus
export FORM_FUZZING_ARTIFACTS_DIR=./my-artifacts

# Run the VM management fuzzer
cargo run --bin fuzz_vm_management

# Run the DNS management fuzzer
cargo run --bin fuzz_dns

# Run the Network fuzzer
cargo run --bin fuzz_network

# Run the MCP Server fuzzer
cargo run --bin fuzz_mcp

# Run the Economic Infrastructure fuzzer
cargo run --bin fuzz_economic
```

### Adding to CI/CD Pipeline

Add this to your GitHub Actions workflow:

```yaml
- name: Run fuzzers
  run: |
    export FORM_FUZZING_MODE=ci
    export FORM_FUZZING_ARTIFACTS_DIR=./fuzzing-artifacts
    cargo run --bin fuzz_vm_management
    cargo run --bin fuzz_dns
    cargo run --bin fuzz_network
    cargo run --bin fuzz_mcp
    cargo run --bin fuzz_economic
    # Add more fuzzers as they are implemented
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

When adding new fuzzers:

1. Create a generator module for your component
2. Implement a harness for your component
3. Add a binary that uses the generator and harness
4. Update the README to document the new fuzzer

## License

MIT OR Apache-2.0 