#!/bin/bash
set -e
echo "Building UI..."
cd ui && npm run build && cd ..
echo "Building Rust..."
cargo build --release
echo "Done: target/release/evlogstudio"
