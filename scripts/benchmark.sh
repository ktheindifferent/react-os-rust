#!/bin/bash

# Performance Benchmark Runner
# Usage: ./benchmark.sh [category]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Benchmark category
CATEGORY=${1:-all}

echo "====================================="
echo "    PERFORMANCE BENCHMARK SUITE     "
echo "====================================="
echo ""

# Function to run benchmark in QEMU
run_benchmark() {
    local bench_type=$1
    
    echo "Running $bench_type benchmarks..."
    
    # Create a temporary script to send commands to QEMU
    cat > /tmp/bench_commands.txt << EOF
benchmark $bench_type
shutdown
EOF
    
    # Run QEMU with benchmark commands
    timeout 30 qemu-system-x86_64 \
        -drive format=raw,file="$PROJECT_ROOT/target/x86_64-rust_os/debug/bootimage-rust_kernel.bin" \
        -serial mon:stdio \
        -display none \
        -m 512M \
        -cpu host,+x2apic \
        -enable-kvm \
        -no-reboot < /tmp/bench_commands.txt 2>&1 | tee /tmp/benchmark_output.log
    
    # Parse and display results
    echo ""
    echo "Results for $bench_type:"
    grep -A 20 "Benchmark Results" /tmp/benchmark_output.log || echo "No results found"
    echo ""
    
    rm -f /tmp/bench_commands.txt /tmp/benchmark_output.log
}

# Build kernel with optimizations
echo "Building optimized kernel..."
cd "$PROJECT_ROOT/kernel"
RUSTFLAGS="-C target-cpu=native -C opt-level=3" cargo build --release --target ../x86_64-rust_os.json

# Create boot image
cd "$PROJECT_ROOT"
cargo bootimage --release --target x86_64-rust_os.json

# Run benchmarks based on category
case $CATEGORY in
    memory)
        run_benchmark "memory"
        ;;
    scheduler)
        run_benchmark "scheduler"
        ;;
    io)
        run_benchmark "filesystem"
        run_benchmark "network"
        ;;
    all)
        run_benchmark "memory"
        run_benchmark "scheduler"
        run_benchmark "filesystem"
        run_benchmark "network"
        run_benchmark "interrupt"
        ;;
    *)
        echo "Unknown category: $CATEGORY"
        echo "Available categories: memory, scheduler, io, all"
        exit 1
        ;;
esac

echo "Benchmark suite completed!"