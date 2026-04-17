#!/bin/bash
set -e
echo "Building UI for dev..."
cd ui && npm run build && cd ..
echo "Building Rust (debug)..."
cargo build
echo "Done: target/debug/evlogstudio"
