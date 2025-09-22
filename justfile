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
    cargo install --path ./crates/slipstream-cli

# Run debug slipstream.
debug-slipstream:
    cargo run --bin slipstream -- --debug -c ./examples/config/slipstream.toml serve

# Run debug slipstream with local config.
debug-slipstream-local:
    cargo run --bin slipstream -- --debug serve

# Run debug slipreader.
debug-slipreader:
    cargo run --bin slipstream -- --debug -c ./examples/config/slipreader.toml read

# Run debug slipreader with local config.
debug-slipreader-local:
    cargo run --bin slipstream -- --debug -c ~/.config/slipstream/slipreader.toml read

# Run debug slipstream with local config.
debug-verify:
    cargo run --bin slipstream -- --debug -c ./examples/config/slipstream.toml config verify
    cargo run --bin slipstream -- --debug -c ./examples/config/slipreader.toml config verify

# Run debug slipstream with local config.
debug-verify-local:
    cargo run --bin slipstream -- --debug config verify

# Test slipstream config export/import.
test-import-export:
    mkdir -p test
    cargo run --bin slipstream -- --debug -c ./examples/config/slipstream.toml config export slipstream ./test/slipstream.export.toml
    cargo run --bin slipstream -- --debug -c ./examples/config/slipstream.toml config export opml ./test/slipstream.export.opml
    cargo run --bin slipstream -- --debug -c ./examples/config/slipstream.toml config export list ./test/slipstream.export.list
    cargo run --bin slipstream -- --debug -c ./examples/config/slipreader.toml config export slipstream ./test/slipreader.export.toml
    cargo run --bin slipstream -- --debug -c ./examples/config/slipreader.toml config export opml ./test/slipreader.export.opml
    cargo run --bin slipstream -- --debug -c ./examples/config/slipreader.toml config export list ./test/slipreader.export.list
    cargo run --bin slipstream -- --debug -c ./test/slipreader.export.toml config import slipstream ./test/slipstream.export.toml ./test/slipreader.import.slip.toml
    cargo run --bin slipstream -- --debug -c ./test/slipreader.export.toml config import opml ./test/slipstream.export.opml ./test/slipreader.import.opml.toml
    cargo run --bin slipstream -- --debug -c ./test/slipreader.export.toml config import list ./test/slipstream.export.list ./test/slipreader.import.list.toml

# Build (mostly) static release for many versions of linux.
build-many:
    # This is a hacky build while sqlx figures out how to disable sqlite fts to support musl.
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
    RUST_LOG=trace cargo test -- --nocapture

# Publish to crates.io.
publish:
    cargo publish --workspace
