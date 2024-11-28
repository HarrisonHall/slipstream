# Slipstream justfile.

# List directives.
[private]
default:
    @just --list

# Set up dependencies.
setup:
    rustup default stable
    rustup component add rust-std-x86_64-unknown-linux-musl
    cargo install dioxus-cli

# Run debug slipknot.
debug-slipknot:
    cargo run --bin slipknot -- --debug

# Run debug slipstream.
debug-slipstream:
    dx serve

# Build static release for many versions of linux via musl.
build-many:
    # Req: rustup component add rust-std-x86_64-unknown-linux-musl
    cargo build --target x86_64-unknown-linux-musl --release

# Test the repo.
test:
    cargo test
