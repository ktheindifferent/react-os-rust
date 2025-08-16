# OS Testing Documentation

## Overview

This document describes the comprehensive testing infrastructure for our OS, including unit tests, integration tests, performance benchmarks, and stress tests.

## Table of Contents

1. [Test Architecture](#test-architecture)
2. [Running Tests](#running-tests)
3. [Unit Tests](#unit-tests)
4. [Integration Tests](#integration-tests)
5. [Performance Benchmarks](#performance-benchmarks)
6. [Stress Tests](#stress-tests)
7. [Continuous Integration](#continuous-integration)
8. [Writing New Tests](#writing-new-tests)
9. [Coverage Reports](#coverage-reports)

## Test Architecture

The testing infrastructure is organized into several layers:

```
kernel/src/
├── tests/              # Unit tests
│   ├── memory_tests.rs
│   ├── scheduler_tests.rs
│   ├── filesystem_tests.rs
│   └── network_tests.rs
├── benchmarks/         # Performance benchmarks
│   └── mod.rs
├── stress_tests/       # Stress tests
│   └── mod.rs
└── test_runner.rs      # Test framework

scripts/
├── test_all.sh         # Run all tests
├── benchmark.sh        # Run benchmarks
└── stress_test.sh      # Run stress tests
```

## Running Tests

### Quick Start

Run all tests:
```bash
./scripts/test_all.sh
```

### Individual Test Suites

#### Unit Tests
```bash
cd kernel
cargo test --lib
```

#### Integration Tests
```bash
./run_tests.sh
```

#### Performance Benchmarks
```bash
./scripts/benchmark.sh all
# Or specific category:
./scripts/benchmark.sh memory
./scripts/benchmark.sh scheduler
./scripts/benchmark.sh io
```

#### Stress Tests
```bash
./scripts/stress_test.sh all
# Or specific category:
./scripts/stress_test.sh memory
./scripts/stress_test.sh process
./scripts/stress_test.sh network
```

### Environment Variables

- `RUN_BENCHMARKS=1` - Include benchmarks in test suite
- `RUN_STRESS_TESTS=1` - Include stress tests in test suite
- `STRESS_DURATION=60` - Set stress test duration in seconds

## Unit Tests

### Memory Management Tests

Tests for memory allocation, deallocation, and management:

- **Heap Allocation**: Basic heap operations
- **Slab Allocator**: Fixed-size allocation tests
- **Frame Allocator**: Physical memory management
- **Virtual Memory**: Page table operations
- **Demand Paging**: Lazy allocation and COW

Example:
```rust
runner.run_test("memory::heap_allocation", || {
    let data = Box::new(42u32);
    if *data != 42 {
        return Err(format!("Allocation failed"));
    }
    Ok(())
});
```

### Scheduler Tests

Process and thread scheduling tests:

- **Round Robin**: Time-slice based scheduling
- **Priority Scheduling**: Priority queue management
- **Context Switching**: Register save/restore
- **Thread Management**: Thread creation and synchronization
- **CPU Affinity**: Multi-core scheduling

### File System Tests

File system implementation tests:

- **Path Parsing**: Path resolution and normalization
- **Inode Operations**: File metadata management
- **Directory Entries**: Directory traversal
- **FAT32**: FAT file system operations
- **NTFS**: NTFS file system operations
- **VFS**: Virtual file system layer

### Network Stack Tests

Network protocol implementation:

- **Ethernet**: Frame construction and validation
- **IP**: Packet routing and fragmentation
- **TCP**: Connection management and flow control
- **UDP**: Datagram handling
- **ARP**: Address resolution
- **DNS**: Name resolution
- **DHCP**: Dynamic configuration

## Integration Tests

Integration tests verify component interactions:

```rust
runner.run_test("integration::memory_alignment", || {
    let nvme_addr = 0x10000000u64;
    if nvme_addr & 0xFFF != 0 {
        return Err(String::from("NVMe alignment failed"));
    }
    Ok(())
});
```

Key areas:
- Hardware driver integration
- Memory-mapped I/O
- Interrupt handling
- DMA operations

## Performance Benchmarks

### Benchmark Framework

The benchmark framework measures:
- **Cycles**: CPU cycles per operation
- **Throughput**: Operations per second
- **Latency**: Time per operation

### Running Benchmarks

```rust
let mut runner = BenchmarkRunner::new();
runner.set_iterations(100, 1000); // warmup, benchmark

runner.run_benchmark("memory::small_alloc", || {
    let _data = vec![0u8; 64];
});

runner.summary();
```

### Benchmark Categories

1. **Memory Benchmarks**
   - Allocation speed
   - Memory copy throughput
   - Cache performance

2. **Scheduler Benchmarks**
   - Context switch latency
   - Thread creation overhead
   - Lock contention

3. **I/O Benchmarks**
   - Disk read/write throughput
   - Network packet processing
   - File system operations

4. **Interrupt Benchmarks**
   - Interrupt latency
   - IRQ handling overhead

### Results Interpretation

```
Benchmark                     Avg Cycles    Min Cycles    Max Cycles    Throughput
----------------------------------------------------------------------------------
memory::small_alloc                 1250           980          1520           N/A
memory::copy_1kb                    3200          3100          3500    750000 ops/s
scheduler::context_switch           2100          1900          2400           N/A
```

## Stress Tests

### Stress Test Framework

Stress tests push the system to its limits:

```rust
runner.run_stress_test("memory::allocation_storm", 5000, || {
    // Rapid allocation/deallocation
    for _ in 0..100 {
        let data = vec![0u8; random_size()];
        verify_data(&data)?;
    }
    Ok(())
});
```

### Stress Test Categories

1. **Memory Stress**
   - Allocation storms
   - Fragmentation testing
   - Memory pressure
   - Rapid alloc/free cycles

2. **Process Stress**
   - Process spawn storms
   - Fork bomb simulation
   - Context switch storms

3. **File System Stress**
   - File creation/deletion storms
   - Deep directory nesting
   - Concurrent access
   - Mount/unmount cycles

4. **Network Stress**
   - Packet floods
   - Connection storms
   - Buffer overflow testing
   - SYN flood simulation

5. **Corruption Recovery**
   - File system recovery
   - Memory corruption detection
   - Stack overflow protection

### Monitoring During Stress Tests

The framework tracks:
- Operation count
- Error rate
- Peak memory usage
- System stability

## Continuous Integration

### GitHub Actions Workflow

While GitHub Actions cannot be directly modified, here's the recommended workflow:

```yaml
name: OS Test Suite

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      
      - name: Install Dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y qemu-system-x86_64
          
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: nightly
          components: rust-src, llvm-tools-preview
          
      - name: Run Tests
        run: ./scripts/test_all.sh
        
      - name: Run Benchmarks
        run: RUN_BENCHMARKS=1 ./scripts/test_all.sh
        if: github.event_name == 'push'
```

### Local CI Simulation

Run the full CI pipeline locally:
```bash
# Install dependencies
make install-deps

# Run full test suite
RUN_BENCHMARKS=1 RUN_STRESS_TESTS=1 ./scripts/test_all.sh
```

## Writing New Tests

### Unit Test Template

```rust
pub fn run_my_component_tests(runner: &mut TestRunner) {
    runner.run_test("component::test_name", || {
        // Setup
        let data = setup_test_data();
        
        // Execute
        let result = perform_operation(&data)?;
        
        // Verify
        if result != expected {
            return Err(format!("Expected {:?}, got {:?}", expected, result));
        }
        
        // Cleanup (automatic with RAII)
        Ok(())
    });
}
```

### Benchmark Template

```rust
runner.run_benchmark("component::operation", || {
    // Operation to benchmark
    perform_operation();
});

runner.run_throughput_benchmark("component::throughput", 1000, || {
    // Operation that processes 1000 items
    process_batch();
});
```

### Stress Test Template

```rust
runner.run_stress_test("component::stress", 5000, || {
    // Stress operation
    for _ in 0..STRESS_ITERATIONS {
        stress_operation()?;
    }
    Ok(())
});
```

## Coverage Reports

### Generating Coverage

```bash
# Build with coverage
RUSTFLAGS="-C instrument-coverage" cargo build --tests

# Run tests
./scripts/test_all.sh

# Generate report
grcov . --binary-path ./target/debug/ \
    -s . -t html --branch --ignore-not-existing \
    -o ./target/coverage/
```

### Coverage Goals

- **Unit Tests**: >80% line coverage
- **Integration Tests**: >60% coverage
- **Critical Components**: >90% coverage
  - Memory management
  - Scheduler
  - File system core
  - Network stack

### Viewing Reports

Open `target/coverage/index.html` in a browser to view detailed coverage reports.

## Test Best Practices

1. **Isolation**: Tests should not depend on each other
2. **Determinism**: Tests should produce consistent results
3. **Speed**: Unit tests should complete quickly (<100ms)
4. **Coverage**: Aim for high code coverage
5. **Error Messages**: Provide clear failure messages
6. **Documentation**: Document complex test scenarios

## Troubleshooting

### Common Issues

1. **QEMU not found**
   ```bash
   sudo apt-get install qemu-system-x86_64
   ```

2. **Build failures**
   ```bash
   cargo clean
   rustup update nightly
   rustup component add rust-src llvm-tools-preview
   ```

3. **Test timeouts**
   - Increase timeout in test scripts
   - Check for infinite loops
   - Verify QEMU configuration

4. **Flaky tests**
   - Add retries for network tests
   - Increase timing tolerances
   - Use deterministic random seeds

## Performance Regression Detection

### Baseline Establishment

1. Run benchmarks on main branch:
   ```bash
   ./scripts/benchmark.sh all > baseline.txt
   ```

2. Compare with feature branch:
   ```bash
   ./scripts/benchmark.sh all > feature.txt
   diff baseline.txt feature.txt
   ```

### Automated Detection

The CI pipeline can automatically detect performance regressions:

```bash
# In CI script
BASELINE_CYCLES=1000
ACTUAL_CYCLES=$(grep "memory::small_alloc" benchmark.log | awk '{print $2}')

if [ $ACTUAL_CYCLES -gt $((BASELINE_CYCLES * 110 / 100)) ]; then
    echo "Performance regression detected!"
    exit 1
fi
```

## Future Improvements

1. **Fuzzing**: Add fuzzing for system calls and network protocols
2. **Property-based Testing**: Use quickcheck for invariant testing
3. **Chaos Engineering**: Introduce random failures
4. **Hardware Testing**: Support for real hardware testing
5. **Distributed Testing**: Parallel test execution
6. **Visual Regression**: GUI testing framework

## Contributing

When contributing tests:

1. Follow the existing test structure
2. Add tests for new features
3. Update this documentation
4. Ensure all tests pass before submitting PR
5. Include benchmark results for performance-critical changes

## Contact

For questions about testing:
- Review existing test examples
- Check the troubleshooting section
- Open an issue with the `testing` label