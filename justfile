# Slipstream justfile

# List directives.
[private]
default:
    @just --list

# Set up dependencies
setup:
    cargo install dioxus-cli

# Debug
debug:
    dx serve

# Build
build:
    echo build

# Test the repo.
test:
    cargo test
