#!/bin/bash

echo "Running kernel and executing tests..."

# Run QEMU and send 'test' command after boot
(
    # Wait for kernel to boot and shell to be ready
    sleep 5
    # Send test command
    echo -e "test\r"
    # Wait for tests to complete
    sleep 10
    # Send shutdown command
    echo -e "shutdown\r"
) | qemu-system-x86_64 \
    -drive format=raw,file=target/x86_64-rust_os/debug/bootimage-rust_kernel.bin \
    -serial mon:stdio \
    -display none \
    -no-reboot 2>&1 | tee kernel_output.log

echo ""
echo "=== Test Results ==="
if grep -q "Starting Hardware Component Tests" kernel_output.log; then
    echo "Tests were executed!"
    grep -A 50 "Starting Hardware Component Tests" kernel_output.log | head -40
else
    echo "Tests did not run. Check the shell integration."
    echo "Last 20 lines of output:"
    tail -20 kernel_output.log
fi

rm -f kernel_output.log