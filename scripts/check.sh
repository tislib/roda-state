#!/usr/bin/env bash

set -e

echo "Running rustfmt..."
cargo fmt --all --check

echo "Running clippy..."
cargo clippy --all-targets -- -D warnings

echo "Running tests..."



echo "All checks passed!"
