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
debug-slipreader:
    cargo run --bin slipstream -- --debug -c ./examples/config/slipreader.toml read

# Run debug slipreader with local config.
debug-slipreader-local:
    cargo run --bin slipstream -- --debug -c ~/.config/slipstream/slipreader.toml read

# Run debug slipstream.
debug-slipstream:
    cargo run --bin slipstream -- --debug -c ./examples/config/slipstream.toml serve

# Run debug slipstream with local config.
debug-slipstream-local:
    cargo run --bin slipstream -- --debug -c ~/.config/slipstream/slipstream.toml serve

# Build static release for many versions of linux via musl.
build-many:
    # This is a hacky build while sqlx figures out how to disable fts to support musl.
    # Req: rustup component add rust-std-x86_64-unknown-linux-musl
    # cargo build --package slipstream --target x86_64-unknown-linux-gnu --release
    # patchelf --set-interpreter /usr/lib64/ld-linux-x86-64.so.2 target/x86_64-unknown-linux-gnu/release/slipstream
    # patchelf --set-interpreter /lib64/ld-linux-x86-64.so.2 target/x86_64-unknown-linux-gnu/release/slipstream
    cargo zigbuild --package slipstream-cli --target x86_64-unknown-linux-gnu.2.32 --release

# Build static release for many versions of linux via musl.
[private]
build-many-correct:
    # This does not work with the current release.
    # Req: rustup component add rust-std-x86_64-unknown-linux-musl
    cargo build --package slipstream-cli --target x86_64-unknown-linux-musl --release

# Test the repo.
test:
    RUST_LOG=info cargo test -- --nocapture
