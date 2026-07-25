# Development entry points. Run `just --list` for an overview.

# Format all files.
fmt:
    cargo fmt --all

# Run the Rust test suites.
test:
    cargo test --workspace --all-features --color always

# Compile and open the docs.
doc:
    cargo doc --workspace --open
