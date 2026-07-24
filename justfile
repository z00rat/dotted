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

update:
    cargo update

# ------------------------------------------------------------------------------
# Testing
# ------------------------------------------------------------------------------

# Run tests using nextest (parallel, isolated test execution)
test *args:
    cargo nextest run {{ args }}

# Build the Arch Linux container sandbox image
sandbox-build: build
    cp target/debug/dotted sandbox/dotted
    docker tag dotted-sandbox dotted-sandbox:old || true
    docker build -t dotted-sandbox sandbox
    rm -f sandbox/dotted

# Open an interactive shell inside an Arch Linux container sandbox
sandbox-shell: sandbox-build
    #!/usr/bin/env bash
    set -euo pipefail
    docker run -it --rm \
      dotted-sandbox \
      fish -c 'echo "Arch Linux sandbox shell. Try running: dotted status"; exec fish'

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
