#!/usr/bin/env bash

set -e

echo "Running rustfmt..."
cargo fmt --all --check

echo "Running clippy..."
cargo clippy -- -D warnings

echo "Running tests..."
cargo test

echo "All checks passed!"
