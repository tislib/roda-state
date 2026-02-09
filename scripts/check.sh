#!/usr/bin/env bash

set -e

echo "Running rustfmt..."
cargo fmt --all --check

echo "Running clippy..." // temporary disabled, slows down active development, will be reenabled
cargo clippy --workspace -- -D warnings

echo "Running tests..."
cargo test --workspace

echo "All checks passed!"
