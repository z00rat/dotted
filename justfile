# Set the shell to bash with strict error handling
set shell := ["bash", "-uc"]

# Print available recipes by default
default:
    @just --list

# ------------------------------------------------------------------------------
# Development & Validation
# ------------------------------------------------------------------------------

# Run the project binary
run *args:
    cargo run {{ args }}

# Build the project in debug mode
build:
    cargo build

# Run formatting checks and clippy lints
check:
    cargo fmt -- --check
    cargo clippy --all-targets -- -D warnings

# Format all source code strictly
fmt:
    cargo fmt

# ------------------------------------------------------------------------------
# Testing
# ------------------------------------------------------------------------------

# Run tests using nextest (parallel, isolated test execution)
test *args:
    cargo nextest run {{ args }}

# Run doc tests (nextest doesn't run doc tests, so standard cargo is used here)
test-docs:
    cargo test --doc

# ------------------------------------------------------------------------------
# CI / Release Pipeline
# ------------------------------------------------------------------------------

# Build optimized release binaries
release:
    cargo build --release

# Clean compilation artifacts
clean:
    cargo clean

# Run full local validation before committing
validate: fmt check test test-docs
