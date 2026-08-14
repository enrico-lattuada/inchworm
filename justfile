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

# Run the Rust test suites.
test:
    cargo test --workspace --all-features --color always

# Compile and open the docs.
doc:
    cargo doc --workspace --open

# Run fmt-check, clippy, and test
ci: fmt-check clippy test
