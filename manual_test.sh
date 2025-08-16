#!/bin/bash

echo "Starting kernel manually - type 'test' to run tests, 'shutdown' to exit"
echo "Press Ctrl+A then X to exit QEMU"

qemu-system-x86_64 \
    -drive format=raw,file=target/x86_64-rust_os/debug/bootimage-rust_kernel.bin \
    -serial mon:stdio \
    -no-reboot