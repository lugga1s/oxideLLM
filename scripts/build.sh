#!/bin/bash
export PATH="/root/.cargo/bin:$PATH"
echo "Building gateway..."
cargo build --release
echo "Building mock..."
cd mock
cargo build --release
