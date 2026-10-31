# Development entry points. Run `just --list` for an overview.

# Format all files.
fmt:
    cargo fmt --all

# Run format all files in check mode.
fmt-check:
    cargo fmt --all -- --check

# Check the workspace to catch common mistakes
clippy:
    cargo clippy --workspace --all-targets --all-features

# Check the workspace to catch common mistakes (treat warnings as errors)
clippy-strict:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# Check the workspace to catch common mistakes (treat warnings as errors)
clippy-strict-ndf:
    cargo clippy --workspace --all-targets --no-default-features -- -D warnings

# Run the Rust test suites.
test:
    cargo test --workspace --all-features --color always

# Compile and open the docs.
doc:
    cargo doc --workspace --open

# Check dependencies' licenses, advisories, bans and sources.
deny:
    cargo deny check

# Check that the pinned toolchain equals the MSRV (workspace rust-version).
toolchain-check:
    #!/usr/bin/env bash
    set -euo pipefail
    pin=$(sed -n 's/^channel = "\(.*\)"/\1/p' rust-toolchain.toml)
    msrv=$(sed -n 's/^rust-version = "\(.*\)"/\1/p' Cargo.toml)
    if [ -z "$pin" ] || [ "$pin" != "$msrv" ]; then
        echo "rust-toolchain.toml pins '$pin' but rust-version is '$msrv'" >&2
        exit 1
    fi

# Run toolchain-check, fmt-check, clippy, deny, and test
ci: toolchain-check fmt-check clippy-strict-ndf clippy-strict deny test

# Save a benchmark baseline named NAME
bench-save NAME:
    cargo bench -p inchworm-units -- --save-baseline {{NAME}}

# Compare the benchmarks against the baseline named NAME
bench-compare NAME:
    cargo bench -p inchworm-units -- --baseline {{NAME}}
