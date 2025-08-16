#!/bin/bash

echo "Testing Rust OS kernel with bug fixes..."
echo "==========================================="

# Build the kernel
echo "Building kernel..."
cargo bootimage --target x86_64-rust_os.json --release 2>&1 | tail -5

if [ ! -f "target/x86_64-rust_os/release/bootimage-rust_kernel.bin" ]; then
    echo "Failed to build kernel image!"
    exit 1
fi

echo ""
echo "Kernel built successfully!"
echo ""
echo "Running kernel in QEMU (will timeout after 10 seconds)..."
echo "==========================================="

# Run the kernel and capture output
timeout 10 qemu-system-x86_64 \
    -drive format=raw,file=target/x86_64-rust_os/release/bootimage-rust_kernel.bin \
    -serial stdio \
    -display none \
    -no-reboot 2>&1 | tee kernel_test_output.log

echo ""
echo "==========================================="
echo "Checking kernel boot progress..."
echo ""

# Check for successful stages
if grep -q "Stage 11a: Process executor initialized" kernel_test_output.log; then
    echo "✓ Process executor initialized successfully (bug fixed!)"
else
    echo "✗ Process executor failed to initialize"
fi

if grep -q "Stage 12a: Disk drivers initialized" kernel_test_output.log; then
    echo "✓ Disk drivers initialized successfully (bug fixed!)"
else
    echo "✗ Disk drivers failed to initialize"
fi

if grep -q "Stage 16a: Shell initialized and ready" kernel_test_output.log; then
    echo "✓ Shell initialized successfully"
else
    echo "✗ Shell failed to initialize"
fi

echo ""
echo "Last boot stage reached:"
grep "Stage" kernel_test_output.log | tail -1

echo ""
echo "Test complete!"