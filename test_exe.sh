#!/bin/bash

echo "Building kernel with .exe execution support..."
cargo bootimage --target x86_64-rust_os.json 2>&1 | tail -5

echo ""
echo "Starting kernel with test commands..."
echo "Commands to test:"
echo "  help - Show available commands"
echo "  exec test.exe - Execute a test PE file"
echo ""

# Create a simple input script
cat > /tmp/test_input.txt << EOF
help
exec test.exe
ver
EOF

echo "Running kernel test..."
timeout 5 qemu-system-x86_64 \
    -drive format=raw,file=target/x86_64-rust_os/debug/bootimage-rust_kernel.bin \
    -serial mon:stdio \
    -display none \
    -no-reboot < /tmp/test_input.txt 2>&1 | tee /tmp/kernel_output.txt

echo ""
echo "=== SHELL OUTPUT ==="
grep -A50 "ReactOS Rust Shell" /tmp/kernel_output.txt || tail -30 /tmp/kernel_output.txt