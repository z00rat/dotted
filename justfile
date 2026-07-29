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

# Run unit tests only
test-unit:
    cargo test --lib

# Run integration tests only
test-integration:
    cargo test --test cli

# Run every local quality check
quality: check test-unit test-integration test-docs

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

# Stage release binary + completions and build Arch .pkg.tar.zst via PKGBUILD/makepkg
package-arch:
    #!/usr/bin/env bash
    set -euo pipefail
    export CARGO_TARGET_DIR="$PWD/target"

    command -v makepkg >/dev/null 2>&1 || { echo "makepkg not found (Arch packaging requires pacman/makepkg)" >&2; exit 1; }

    VERSION=$(cargo metadata --no-deps --format-version 1 | grep -oP '"name":"dotted".*?"version":"\K[^"]+' | head -n 1)
    echo "Syncing PKGBUILD pkgver=$VERSION from Cargo.toml..."
    sed -i "s/^pkgver=.*/pkgver=$VERSION/" PKGBUILD

    if [[ ! -x target/release/dotted ]]; then
        echo "Building release binary..."
        cargo build --release
    fi

    echo "Staging Arch package inputs..."
    # Run makepkg from an isolated directory so its src/pkg workdirs can never
    # collide with this Rust project's src/ tree.
    ROOT="$PWD"
    rm -rf target/arch-package
    mkdir -p target/arch-package target/completions target/distrib

    install -Dm644 PKGBUILD target/arch-package/PKGBUILD
    install -Dm755 target/release/dotted target/arch-package/dotted
    install -Dm644 LICENSE target/arch-package/LICENSE

    target/release/dotted shell completions bash > target/completions/dotted.bash
    target/release/dotted shell completions zsh > target/completions/_dotted
    target/release/dotted shell completions fish > target/completions/dotted.fish
    install -Dm644 target/completions/dotted.bash target/arch-package/dotted.bash
    install -Dm644 target/completions/_dotted target/arch-package/_dotted
    install -Dm644 target/completions/dotted.fish target/arch-package/dotted.fish

    echo "Building Arch Linux package (.pkg.tar.zst)..."
    # Force zstd package extension regardless of host makepkg.conf.
    # --nodeps: packaging a prebuilt binary; runtime depends are declared in PKGBUILD for installers.
    (
      cd target/arch-package
      PKGEXT='.pkg.tar.zst' PKGDEST="$ROOT/target/distrib" makepkg -f --clean --nodeps
    )
    rm -rf target/arch-package

    echo "Arch package(s):"
    ls -1 target/distrib/*.pkg.tar.zst

# Build release packages for Arch Linux (.pkg.tar.zst), Debian/Ubuntu (.deb), Fedora/RHEL (.rpm), and tarball
publish-dist TAG:
    #!/usr/bin/env bash
    set -euo pipefail

    export PATH="$HOME/.local/share/cargo/bin:$HOME/.cargo/bin:$PATH"
    export CARGO_TARGET_DIR="$PWD/target"

    echo "Building release binary..."
    cargo build --release

    echo "Cleaning packaging output directories..."
    rm -rf target/distrib target/debian target/generate-rpm target/completions target/arch-package
    mkdir -p target/distrib target/completions

    echo "Generating shell completion scripts..."
    target/release/dotted shell completions bash > target/completions/dotted.bash
    target/release/dotted shell completions zsh > target/completions/_dotted
    target/release/dotted shell completions fish > target/completions/dotted.fish

    tar -czf "target/distrib/dotted-{{ TAG }}-x86_64-linux.tar.gz" -C target/release dotted -C ../completions dotted.bash _dotted dotted.fish

    echo "Building Arch Linux package..."
    just package-arch

    echo "Building Debian (.deb) & Fedora (.rpm) packages if generators installed..."
    command -v cargo-deb >/dev/null 2>&1 && cargo deb || echo "cargo-deb not installed, skipping .deb"
    command -v cargo-generate-rpm >/dev/null 2>&1 && cargo generate-rpm || echo "cargo-generate-rpm not installed, skipping .rpm"

    mkdir -p target/distrib
    cp -f target/debian/*.deb target/distrib/ 2>/dev/null || true
    cp -f target/generate-rpm/*.rpm target/distrib/ 2>/dev/null || true

    echo "Release artifacts are ready in target/distrib:"
    ls -lh target/distrib/*
    printf "Push the current branch and tag {{ TAG }}, then upload these artifacts to GitHub? [y/N] "
    read -r CONFIRM </dev/tty
    if [[ ! "$CONFIRM" =~ ^[Yy]$ ]]; then
        echo "GitHub publishing cancelled; local artifacts were kept."
        exit 0
    fi

    echo "Uploading dist artifacts to GitHub Release {{ TAG }}..."
    git push origin HEAD "{{ TAG }}"
    gh release view "{{ TAG }}" >/dev/null 2>&1 \
        && gh release upload "{{ TAG }}" target/distrib/* --clobber \
        || gh release create "{{ TAG }}" target/distrib/* --generate-notes --title "Release {{ TAG }}"
