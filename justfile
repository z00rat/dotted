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

# Build release packages for Arch Linux (.pkg.tar.zst), Debian/Ubuntu (.deb), Fedora/RHEL (.rpm), and shell script installer
publish-dist TAG:
    #!/usr/bin/env bash
    set -euo pipefail

    export PATH="$HOME/.local/share/cargo/bin:$HOME/.cargo/bin:$PATH"

    VERSION=$(cargo metadata --no-deps --format-version 1 | grep -oP '"name":"dotted".*?"version":"\K[^"]+' | head -n 1)
    echo "Syncing PKGBUILD pkgver=$VERSION from Cargo.toml..."
    sed -i "s/^pkgver=.*/pkgver=$VERSION/" PKGBUILD

    echo "Building release binary..."
    cargo build --release

    echo "Cleaning packaging output directories..."
    rm -rf target/distrib target/debian target/generate-rpm
    mkdir -p target/distrib
    tar -czf "target/distrib/dotted-{{ TAG }}-x86_64-linux.tar.gz" -C target/release dotted

    echo "Building Debian (.deb) & Fedora (.rpm) packages if generators installed..."
    command -v cargo-deb >/dev/null 2>&1 && cargo deb || echo "cargo-deb not installed, skipping .deb"
    command -v cargo-generate-rpm >/dev/null 2>&1 && cargo generate-rpm || echo "cargo-generate-rpm not installed, skipping .rpm"

    # Build Arch Linux package if makepkg is installed
    if command -v makepkg >/dev/null 2>&1; then
        echo "Building native Arch Linux package (.pkg.tar.zst)..."
        PKGDEST="$PWD/target/distrib" makepkg -f --nodeps
        # Ensure package is zstd compressed to .pkg.tar.zst regardless of host makepkg.conf
        for f in target/distrib/*.pkg.tar; do
            if [ -f "$f" ]; then
                echo "Compressing $f with zstd..."
                zstd -z -f --rm "$f" -o "$f.zst"
            fi
        done
    fi

    # Collect deb, rpm, and arch packages into distrib directory
    mkdir -p target/distrib
    cp -f target/debian/*.deb target/distrib/ 2>/dev/null || true
    cp -f target/generate-rpm/*.rpm target/distrib/ 2>/dev/null || true

    echo "Uploading dist artifacts to GitHub Release {{ TAG }}..."
    git push origin "{{ TAG }}" 2>/dev/null || true
    gh release create "{{ TAG }}" target/distrib/* --generate-notes --title "Release {{ TAG }}" 2>/dev/null || gh release upload "{{ TAG }}" target/distrib/* --clobber
