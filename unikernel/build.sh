#!/bin/bash
set -e

echo "Building core engine for Nanos..."
cd /opt/elixide

# Cross-compile for Linux (Nanos requirement)
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl

# If on macOS, use Docker for cross-compilation
if [[ "$OSTYPE" == "darwin"* ]]; then
    docker run --rm -v $(pwd):/workspace -w /workspace \
        clippy/rust-musl-cross:x86_64-unknown-linux-musl \
        cargo build --release --target x86_64-unknown-linux-musl
fi

# Copy binary to unikernel directory
cp target/x86_64-unknown-linux-musl/release/elixide-core unikernel/

# Create unikernel image
cd unikernel
ops image create elixide-core -c config.json

echo "Unikernel image created. Run with: ops run elixide-core -c config.json"