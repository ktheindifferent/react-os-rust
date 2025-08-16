#!/bin/bash

# Stress Test Runner
# Usage: ./stress_test.sh [category]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Stress test category
CATEGORY=${1:-all}

# Duration for each stress test (in seconds)
STRESS_DURATION=${STRESS_DURATION:-30}

echo "====================================="
echo "       STRESS TEST SUITE            "
echo "====================================="
echo ""
echo "Duration per test: ${STRESS_DURATION}s"
echo ""

# Function to run stress test in QEMU
run_stress_test() {
    local test_type=$1
    
    echo "Running $test_type stress test..."
    echo "This may take up to $STRESS_DURATION seconds..."
    
    # Create a temporary script to send commands to QEMU
    cat > /tmp/stress_commands.txt << EOF
stress $test_type $((STRESS_DURATION * 1000))
shutdown
EOF
    
    # Run QEMU with stress test commands
    timeout $((STRESS_DURATION + 10)) qemu-system-x86_64 \
        -drive format=raw,file="$PROJECT_ROOT/target/x86_64-rust_os/debug/bootimage-rust_kernel.bin" \
        -serial mon:stdio \
        -display none \
        -m 1024M \
        -smp 2 \
        -cpu qemu64,+x2apic \
        -no-reboot < /tmp/stress_commands.txt 2>&1 | tee /tmp/stress_output.log
    
    # Check for errors
    if grep -q "FAIL" /tmp/stress_output.log; then
        echo "⚠️  Stress test detected failures!"
        grep "FAIL" /tmp/stress_output.log
    else
        echo "✓ Stress test completed successfully"
    fi
    
    # Display summary
    echo ""
    grep -A 10 "Stress Test Results" /tmp/stress_output.log || echo "No results found"
    echo ""
    
    rm -f /tmp/stress_commands.txt /tmp/stress_output.log
}

# Build kernel
echo "Building kernel..."
cd "$PROJECT_ROOT/kernel"
cargo build --target ../x86_64-rust_os.json

# Create boot image
cd "$PROJECT_ROOT"
cargo bootimage --target x86_64-rust_os.json

# Run stress tests based on category
case $CATEGORY in
    memory)
        run_stress_test "memory"
        ;;
    process)
        run_stress_test "process"
        ;;
    filesystem)
        run_stress_test "filesystem"
        ;;
    network)
        run_stress_test "network"
        ;;
    all)
        run_stress_test "memory"
        run_stress_test "process"
        run_stress_test "filesystem"
        run_stress_test "network"
        run_stress_test "interrupt"
        run_stress_test "corruption"
        ;;
    *)
        echo "Unknown category: $CATEGORY"
        echo "Available categories: memory, process, filesystem, network, all"
        exit 1
        ;;
esac

echo "Stress test suite completed!"