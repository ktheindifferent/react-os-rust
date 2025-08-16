#!/bin/bash
# Build AP boot trampoline

# Assemble the trampoline code
nasm -f bin -o ap_trampoline.bin ap_trampoline.asm

# Check if the binary is small enough (must fit in < 4KB)
size=$(stat -c%s ap_trampoline.bin)
if [ $size -gt 4096 ]; then
    echo "Error: AP trampoline too large ($size bytes, max 4096)"
    exit 1
fi

# Pad to exactly 512 bytes for simplicity
dd if=/dev/zero of=ap_trampoline.bin bs=1 count=$((512-$size)) seek=$size 2>/dev/null

echo "AP trampoline built successfully ($size bytes, padded to 512)"