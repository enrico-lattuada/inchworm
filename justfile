# Development entry points. Run `just --list` for an overview.

# Format all files.
fmt:
    cargo fmt --all

# Run the Rust test suites.
test:
    cargo test --workspace --all-features --color always
