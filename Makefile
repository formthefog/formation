# Formation Project Makefile
# Common development and testing tasks

.PHONY: help docs-test docs-test-verbose docs-install-deps test build clean

# Default target
help:
	@echo "Formation Project - Available Commands:"
	@echo ""
	@echo "Documentation:"
	@echo "  docs-test          Run documentation tests"
	@echo "  docs-test-verbose  Run documentation tests with verbose output"
	@echo "  docs-install-deps  Install documentation testing dependencies"
	@echo ""
	@echo "Development:"
	@echo "  test              Run all tests"
	@echo "  build             Build the project"
	@echo "  clean             Clean build artifacts"
	@echo ""

# Documentation testing
docs-test:
	@echo "🧪 Running Formation documentation tests..."
	@python3 scripts/test-documentation.py

docs-test-verbose:
	@echo "🧪 Running Formation documentation tests (verbose)..."
	@python3 scripts/test-documentation.py --verbose

docs-install-deps:
	@echo "📦 Installing documentation testing dependencies..."
	@pip3 install -r scripts/requirements-test.txt

# Development tasks
test: docs-test
	@echo "✅ All tests completed"

build:
	@echo "🔨 Building Formation..."
	@cargo build

clean:
	@echo "🧹 Cleaning build artifacts..."
	@cargo clean
	@find . -name "*.pyc" -delete
	@find . -name "__pycache__" -type d -exec rm -rf {} + 2>/dev/null || true

# CI/CD targets
ci-docs-test:
	@echo "🤖 CI: Running documentation tests..."
	@python3 scripts/test-documentation.py --verbose

# Quick validation (for pre-commit hooks)
validate: docs-test
	@echo "✅ Validation completed" 