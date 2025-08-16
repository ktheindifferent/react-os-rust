#!/bin/bash

echo "Building kernel..."
cargo bootimage --target x86_64-rust_os.json 2>&1 | tail -5

echo "Starting kernel interactively (Press Ctrl-A X to exit)..."
qemu-system-x86_64 \
    -drive format=raw,file=target/x86_64-rust_os/debug/bootimage-rust_kernel.bin \
    -serial mon:stdio \
    -display none \
    -no-reboot \
    -no-shutdown