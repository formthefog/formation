# Formation Documentation Testing

This directory contains tools and scripts for testing Formation's documentation to ensure accuracy, completeness, and quality.

## Overview

The documentation testing system validates three key areas:

1. **Code Examples** - Verifies all code examples have valid syntax
2. **API Endpoints** - Tests API endpoint documentation for correctness
3. **Links and References** - Validates all internal and external links

## Quick Start

### Install Dependencies

```bash
# Install Python dependencies for testing
pip install -r scripts/requirements-test.txt

# For JavaScript syntax checking (optional)
npm install -g ethers axios node-fetch
```

### Run Tests

```bash
# Run all documentation tests
make docs-test

# Run with verbose output
make docs-test-verbose

# Or run directly
python scripts/test-documentation.py --verbose
```

## Testing Components

### 1. Code Example Validation (Sub-sub-task 2.4.2.1)

The system extracts and validates code blocks from all markdown files:

#### Supported Languages
- **Python**: Syntax checking with `py_compile`
- **JavaScript/Node.js**: Syntax checking with `node --check`
- **Bash/Shell**: Syntax checking with `bash -n`
- **cURL**: Command structure validation

#### What's Tested
- Syntax errors in code blocks
- Missing imports (automatically added for common cases)
- Formation-specific patterns (authentication headers, etc.)
- Command structure and format

#### Example
```python
# This Python code will be syntax-checked
def example_function():
    return "Hello Formation!"
```

### 2. API Endpoint Testing (Sub-sub-task 2.4.2.2)

Validates API endpoints mentioned in documentation:

#### Endpoint Format Validation
- Proper HTTP method usage
- Correct endpoint paths
- Parameter placeholder format
- Formation-specific conventions

#### Formation API Patterns
- `/v1/` versioned endpoints
- `/health` monitoring endpoints
- `/agents/` and `/models/` resource endpoints
- REST convention compliance

#### Example Validations
```http
GET /v1/agents          ✅ Valid
POST /v1/agents/create  ✅ Valid
GET /v1/agents/create   ❌ Invalid (GET shouldn't use /create)
```

### 3. Link and Reference Validation (Sub-sub-task 2.4.2.3)

Comprehensive link checking across all documentation:

#### Internal Links
- **Relative file links**: `./other-file.md`, `../parent/file.md`
- **Anchor links**: `#section-heading`, `file.md#anchor`
- **Cross-references**: Links between documentation sections

#### External Links
- **HTTP/HTTPS links**: Accessibility testing with rate limiting
- **Known domains**: Special handling for GitHub, Discord, etc.
- **Broken link detection**: 404s and connection errors

#### Link Resolution
- Path normalization and resolution
- Anchor target verification
- Case-sensitive link checking

## Test Results

### Output Format

The testing system provides detailed results:

```
FORMATION DOCUMENTATION TEST SUMMARY
================================================================================
Total Tests: 156
Passed: 142
Failed: 14
Success Rate: 91.0%

Results by Test Type:
----------------------------------------
code_syntax          45/ 48 ( 93.8%)
curl_validation      12/ 15 ( 80.0%)
api_endpoint         23/ 25 ( 92.0%)
internal_link        38/ 42 ( 90.5%)
external_link        24/ 26 ( 92.3%)
```

### Failure Details

Failed tests include:
- File path and line number
- Specific error message
- Suggested fixes (when available)

## Configuration

### Test Settings

The `DocumentationTester` class can be configured:

```python
tester = DocumentationTester(
    docs_dir="docs",           # Documentation directory
    verbose=True,              # Detailed output
    fix_links=False           # Attempt to fix broken links
)
```

### External Domains

Known external domains (not tested to avoid rate limiting):
- `github.com`
- `discord.gg`
- `formation.ai`
- `api.openai.com`
- `docs.python.org`
- `nodejs.org`
- `docker.com`
- `rust-lang.org`

## Continuous Integration

### GitHub Actions

The documentation tests run automatically on:
- Pull requests affecting documentation
- Pushes to main/develop branches
- Changes to testing scripts

### Workflow Jobs

1. **test-documentation**: Main test suite
2. **validate-links**: External link checking
3. **check-formatting**: Markdown formatting validation

### Local Pre-commit

Add to your development workflow:

```bash
# Quick validation before committing
make validate

# Full test suite
make test
```

## Advanced Usage

### Custom Test Runs

```bash
# Test specific directory
python scripts/test-documentation.py --docs-dir custom-docs

# Verbose output with link fixing
python scripts/test-documentation.py --verbose --fix-links

# Test only specific file types
python scripts/test-documentation.py --docs-dir docs/users
```

### Integration with IDEs

The testing script outputs standard error formats that can be integrated with:
- VS Code problem matchers
- Vim quickfix lists
- Emacs compilation mode

### Extending Tests

To add new test types:

1. Create a new test method in `DocumentationTester`
2. Add the test type to the results processing
3. Update the summary reporting

Example:
```python
def test_custom_format(self, content: str, file_path: Path) -> TestResult:
    # Custom validation logic
    return TestResult("custom_format", str(file_path), "Custom test", 
                     success, message)
```

## Troubleshooting

### Common Issues

#### Python Syntax Errors
- **Issue**: Missing imports in code examples
- **Solution**: Common imports are automatically added
- **Manual fix**: Add required imports to code blocks

#### JavaScript Errors
- **Issue**: Node.js not available
- **Solution**: Install Node.js or skip JavaScript tests
- **Command**: `npm install -g node`

#### Link Failures
- **Issue**: Relative links not resolving
- **Solution**: Check file paths and anchor names
- **Debug**: Use `--verbose` flag for detailed link resolution

#### External Link Timeouts
- **Issue**: Network connectivity or rate limiting
- **Solution**: Known domains are skipped automatically
- **Manual**: Add domains to `external_domains` set

### Debug Mode

Enable verbose logging:

```bash
python scripts/test-documentation.py --verbose 2>&1 | tee test-output.log
```

### Performance

For large documentation sets:
- Tests run in parallel (4 workers by default)
- External link checking is rate-limited
- Syntax checking uses temporary files efficiently

## Contributing

### Adding New Tests

1. Identify the documentation quality issue
2. Create a test method in the appropriate category
3. Add test cases and validation logic
4. Update documentation and examples

### Improving Existing Tests

1. Review test failure patterns
2. Enhance error messages and suggestions
3. Add support for new file formats or patterns
4. Optimize performance for large documentation sets

### Reporting Issues

When reporting documentation test issues:
1. Include the full error output
2. Specify the file and line number
3. Provide the expected vs. actual behavior
4. Include system information (Python version, OS, etc.)

---

## Files in This Directory

- **`test-documentation.py`**: Main testing script
- **`requirements-test.txt`**: Python dependencies
- **`README.md`**: This documentation
- **`../Makefile`**: Build targets for testing
- **`../.github/workflows/docs-test.yml`**: CI/CD configuration

## Related Documentation

- [Formation Documentation](../docs/README.md) - Main documentation entry point
- [Contributing Guidelines](../CONTRIBUTING.md) - How to contribute to Formation
- [Development Setup](../docs/developers/README.md) - Developer documentation

---

**Ready to ensure Formation's documentation quality?** Run `make docs-test` to get started! 🚀 