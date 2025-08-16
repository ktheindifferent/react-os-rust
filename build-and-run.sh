#!/bin/bash

set -e

echo "========================================="
echo "Rust-based ReactOS Build and Test System"
echo "========================================="

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

cd "$SCRIPT_DIR"

if ! command -v rustup &> /dev/null; then
    echo "Error: Rust is not installed. Please install Rust first:"
    echo "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

if ! command -v qemu-system-x86_64 &> /dev/null; then
    echo "Error: QEMU is not installed. Please install QEMU first:"
    echo "brew install qemu (on macOS)"
    echo "sudo apt-get install qemu-system-x86 (on Ubuntu/Debian)"
    exit 1
fi

echo "Installing Rust nightly and required components..."
rustup override set nightly
rustup component add rust-src llvm-tools-preview

if ! command -v bootimage &> /dev/null; then
    echo "Installing bootimage tool..."
    cargo install bootimage
fi

echo "Building Rust OS kernel..."
cd kernel
cargo build --target ../x86_64-rust_os.json
cd ..

echo "Creating bootable image..."
cd kernel
cargo bootimage --target ../x86_64-rust_os.json
cd ..

echo "Starting QEMU..."
echo "Click in the QEMU window and then try typing..."
qemu-system-x86_64 \
    -drive format=raw,file=target/x86_64-rust_os/debug/bootimage-rust_kernel.bin \
    -serial stdio \
    -m 512M \
    -cpu qemu64,+x2apic \
    || true

echo "Done!"