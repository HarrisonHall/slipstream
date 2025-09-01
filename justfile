# Slipstream justfile.

# List directives.
[private]
default:
    @just --list

# Set up dependencies.
setup:
    rustup default stable
    rustup component add rust-std-x86_64-unknown-linux-musl

# Install locally.
install:
    cargo install --path ./crates/slipstream

# Run debug slipreader.
debug-slipreader local:
    #!/usr/bin/env sh
    if [ {{local}} ]; then
        cargo run --bin slipstream -- --debug -c ~/.config/slipstream/slipreader.toml read
    else
        cargo run --bin slipstream -- --debug -c ./examples/config/slipreader.toml read
    fi

# Run debug slipstream.
debug-slipstream:
    #!/usr/bin/env sh
    cargo run --bin slipstream -- --debug -c ./examples/config/slipstream.toml serve

# Build static release for many versions of linux via musl.
build-many-correct:
    # Req: rustup component add rust-std-x86_64-unknown-linux-musl
    cargo build --package slipstream --target x86_64-unknown-linux-musl --release

# Build static release for many versions of linux via musl.
# This is a hacky build while sqlx figures out how to disable fts to support musl.
build-many:
    # Req: rustup component add rust-std-x86_64-unknown-linux-musl
    cargo build --package slipstream --target x86_64-unknown-linux-gnu --release
    # patchelf --set-interpreter /usr/lib64/ld-linux-x86-64.so.2 target/x86_64-unknown-linux-gnu/release/slipstream
    patchelf --set-interpreter /lib64/ld-linux-x86-64.so.2 target/x86_64-unknown-linux-gnu/release/slipstream

# Test the repo.
test:
    cargo test
