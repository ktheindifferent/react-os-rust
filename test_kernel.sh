#!/bin/bash

# Script to test the kernel's test command
echo "Starting kernel with test runner..."

# Build the kernel first
echo "Building kernel..."
cd kernel
cargo bootimage --target ../x86_64-rust_os.json 2>&1 | tail -5
cd ..

# Run QEMU with our kernel and send commands after a delay
(sleep 3; echo -e "test\r"; sleep 2; echo -e "shutdown\r") | timeout 30 qemu-system-x86_64 \
    -drive format=raw,file=target/x86_64-rust_os/debug/bootimage-rust_kernel.bin \
    -serial mon:stdio \
    -display none \
    -no-reboot 2>&1 | tee /tmp/kernel_test_output.txt

# Check if tests ran
if grep -q "Starting Hardware Component Tests" /tmp/kernel_test_output.txt; then
    echo ""
    echo "=== TEST RUNNER OUTPUT ==="
    grep -A 100 "Starting Hardware Component Tests" /tmp/kernel_test_output.txt | head -50
    echo ""
    echo "Tests executed successfully!"
else
    echo "Tests did not run. Kernel output:"
    tail -30 /tmp/kernel_test_output.txt
fi

# Clean up
rm -f /tmp/test_input.txt /tmp/kernel_test_output.txt