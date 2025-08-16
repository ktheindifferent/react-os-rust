#!/bin/bash

# Comprehensive Test Suite Runner
# This script runs all test suites for the OS

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test results
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

echo "====================================="
echo "     OS COMPREHENSIVE TEST SUITE    "
echo "====================================="
echo ""

# Function to run a test and capture results
run_test() {
    local test_name=$1
    local test_command=$2
    
    echo -n "Running $test_name... "
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    if eval "$test_command" > /tmp/test_output_$$.log 2>&1; then
        echo -e "${GREEN}[PASS]${NC}"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        echo -e "${RED}[FAIL]${NC}"
        FAILED_TESTS=$((FAILED_TESTS + 1))
        echo "  Error output:"
        tail -n 10 /tmp/test_output_$$.log | sed 's/^/    /'
    fi
    
    rm -f /tmp/test_output_$$.log
}

# Check dependencies
echo "Checking dependencies..."
command -v cargo >/dev/null 2>&1 || { echo "cargo is required but not installed."; exit 1; }
command -v qemu-system-x86_64 >/dev/null 2>&1 || { echo "qemu-system-x86_64 is required but not installed."; exit 1; }

# Build the kernel
echo ""
echo "Building kernel..."
cd "$PROJECT_ROOT"
cargo build --target x86_64-rust_os.json || { echo "Build failed"; exit 1; }

# Run unit tests
echo ""
echo "=== Unit Tests ==="
run_test "Memory Management" "cd kernel && cargo test --lib memory_tests --no-run"
run_test "Scheduler" "cd kernel && cargo test --lib scheduler_tests --no-run"
run_test "File System" "cd kernel && cargo test --lib filesystem_tests --no-run"
run_test "Network Stack" "cd kernel && cargo test --lib network_tests --no-run"

# Run integration tests
echo ""
echo "=== Integration Tests ==="
run_test "Hardware Integration" "$PROJECT_ROOT/run_tests.sh"

# Run benchmarks (if enabled)
if [ "$RUN_BENCHMARKS" = "1" ]; then
    echo ""
    echo "=== Performance Benchmarks ==="
    run_test "Memory Benchmarks" "$PROJECT_ROOT/scripts/benchmark.sh memory"
    run_test "Scheduler Benchmarks" "$PROJECT_ROOT/scripts/benchmark.sh scheduler"
    run_test "I/O Benchmarks" "$PROJECT_ROOT/scripts/benchmark.sh io"
fi

# Run stress tests (if enabled)
if [ "$RUN_STRESS_TESTS" = "1" ]; then
    echo ""
    echo "=== Stress Tests ==="
    echo -e "${YELLOW}Warning: Stress tests may take several minutes${NC}"
    run_test "Memory Stress" "$PROJECT_ROOT/scripts/stress_test.sh memory"
    run_test "Process Stress" "$PROJECT_ROOT/scripts/stress_test.sh process"
    run_test "Network Stress" "$PROJECT_ROOT/scripts/stress_test.sh network"
fi

# Summary
echo ""
echo "====================================="
echo "           TEST SUMMARY              "
echo "====================================="
echo "Total Tests:  $TOTAL_TESTS"
echo -e "Passed:       ${GREEN}$PASSED_TESTS${NC}"
echo -e "Failed:       ${RED}$FAILED_TESTS${NC}"
echo ""

if [ $FAILED_TESTS -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed. Please review the errors above.${NC}"
    exit 1
fi