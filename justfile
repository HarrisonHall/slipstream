# Slipstream

# List 
[private]
default:
    just --list

# Set up dependencies
setup:
    cargo install dioxus-cli

serve:
    dx serve

build:
    echo build
