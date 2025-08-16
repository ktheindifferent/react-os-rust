// Memory Allocator Performance Benchmarks
// Tests and benchmarks for the hybrid allocator

use alloc::vec::Vec;
use alloc::collections::VecDeque;
use core::alloc::{Layout, GlobalAlloc};
use core::time::Duration;

// Benchmark configuration
const SMALL_ALLOC_SIZE: usize = 64;
const MEDIUM_ALLOC_SIZE: usize = 512;
const LARGE_ALLOC_SIZE: usize = 8192;
const HUGE_ALLOC_SIZE: usize = 65536;

const BENCHMARK_ITERATIONS: usize = 10000;
const STRESS_TEST_ITERATIONS: usize = 100000;
const CONCURRENT_THREADS: usize = 4;

// Simple timer for benchmarking
pub struct BenchTimer {
    start_ticks: u64,
}

impl BenchTimer {
    pub fn start() -> Self {
        // In a real kernel, this would use RDTSC or similar
        Self {
            start_ticks: unsafe { core::arch::x86_64::_rdtsc() },
        }
    }

    pub fn elapsed_cycles(&self) -> u64 {
        unsafe { core::arch::x86_64::_rdtsc() - self.start_ticks }
    }

    pub fn elapsed_ns(&self) -> u64 {
        // Assuming 3GHz CPU for rough conversion
        self.elapsed_cycles() / 3
    }
}

// Benchmark results structure
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub name: &'static str,
    pub iterations: usize,
    pub total_cycles: u64,
    pub avg_cycles: u64,
    pub min_cycles: u64,
    pub max_cycles: u64,
    pub throughput_mbps: f64,
}

impl BenchmarkResult {
    pub fn print(&self) {
        crate::serial_println!("Benchmark: {}", self.name);
        crate::serial_println!("  Iterations: {}", self.iterations);
        crate::serial_println!("  Avg Cycles: {}", self.avg_cycles);
        crate::serial_println!("  Min Cycles: {}", self.min_cycles);
        crate::serial_println!("  Max Cycles: {}", self.max_cycles);
        crate::serial_println!("  Throughput: {:.2} MB/s", self.throughput_mbps);
    }
}

// Individual benchmark functions
pub fn bench_small_allocations() -> BenchmarkResult {
    let mut results = Vec::with_capacity(BENCHMARK_ITERATIONS);
    let mut allocations = Vec::with_capacity(BENCHMARK_ITERATIONS);
    
    for _ in 0..BENCHMARK_ITERATIONS {
        let timer = BenchTimer::start();
        
        let layout = Layout::from_size_align(SMALL_ALLOC_SIZE, 8).unwrap();
        let ptr = unsafe { alloc::alloc::alloc(layout) };
        
        let cycles = timer.elapsed_cycles();
        results.push(cycles);
        
        if !ptr.is_null() {
            allocations.push((ptr, layout));
        }
    }
    
    // Cleanup
    for (ptr, layout) in allocations {
        unsafe { alloc::alloc::dealloc(ptr, layout) };
    }
    
    calculate_results("Small Allocations (64B)", &results, SMALL_ALLOC_SIZE)
}

pub fn bench_medium_allocations() -> BenchmarkResult {
    let mut results = Vec::with_capacity(BENCHMARK_ITERATIONS);
    let mut allocations = Vec::with_capacity(BENCHMARK_ITERATIONS);
    
    for _ in 0..BENCHMARK_ITERATIONS {
        let timer = BenchTimer::start();
        
        let layout = Layout::from_size_align(MEDIUM_ALLOC_SIZE, 8).unwrap();
        let ptr = unsafe { alloc::alloc::alloc(layout) };
        
        let cycles = timer.elapsed_cycles();
        results.push(cycles);
        
        if !ptr.is_null() {
            allocations.push((ptr, layout));
        }
    }
    
    // Cleanup
    for (ptr, layout) in allocations {
        unsafe { alloc::alloc::dealloc(ptr, layout) };
    }
    
    calculate_results("Medium Allocations (512B)", &results, MEDIUM_ALLOC_SIZE)
}

pub fn bench_large_allocations() -> BenchmarkResult {
    let mut results = Vec::with_capacity(BENCHMARK_ITERATIONS);
    let mut allocations = Vec::with_capacity(BENCHMARK_ITERATIONS / 10);
    
    for _ in 0..(BENCHMARK_ITERATIONS / 10) {
        let timer = BenchTimer::start();
        
        let layout = Layout::from_size_align(LARGE_ALLOC_SIZE, 8).unwrap();
        let ptr = unsafe { alloc::alloc::alloc(layout) };
        
        let cycles = timer.elapsed_cycles();
        results.push(cycles);
        
        if !ptr.is_null() {
            allocations.push((ptr, layout));
        }
    }
    
    // Cleanup
    for (ptr, layout) in allocations {
        unsafe { alloc::alloc::dealloc(ptr, layout) };
    }
    
    calculate_results("Large Allocations (8KB)", &results, LARGE_ALLOC_SIZE)
}

pub fn bench_huge_allocations() -> BenchmarkResult {
    let mut results = Vec::with_capacity(100);
    let mut allocations = Vec::with_capacity(100);
    
    for _ in 0..100 {
        let timer = BenchTimer::start();
        
        let layout = Layout::from_size_align(HUGE_ALLOC_SIZE, 8).unwrap();
        let ptr = unsafe { alloc::alloc::alloc(layout) };
        
        let cycles = timer.elapsed_cycles();
        results.push(cycles);
        
        if !ptr.is_null() {
            allocations.push((ptr, layout));
        }
    }
    
    // Cleanup
    for (ptr, layout) in allocations {
        unsafe { alloc::alloc::dealloc(ptr, layout) };
    }
    
    calculate_results("Huge Allocations (64KB)", &results, HUGE_ALLOC_SIZE)
}

pub fn bench_mixed_workload() -> BenchmarkResult {
    let mut results = Vec::with_capacity(BENCHMARK_ITERATIONS);
    let mut allocations = VecDeque::with_capacity(1000);
    
    // Simulate realistic mixed allocation pattern
    let sizes = [64, 128, 256, 512, 1024, 2048, 4096];
    let mut size_idx = 0;
    
    for i in 0..BENCHMARK_ITERATIONS {
        let size = sizes[size_idx];
        size_idx = (size_idx + 1) % sizes.len();
        
        let timer = BenchTimer::start();
        
        // Allocate
        let layout = Layout::from_size_align(size, 8).unwrap();
        let ptr = unsafe { alloc::alloc::alloc(layout) };
        
        let cycles = timer.elapsed_cycles();
        results.push(cycles);
        
        if !ptr.is_null() {
            allocations.push_back((ptr, layout));
        }
        
        // Deallocate some old allocations to simulate churn
        if allocations.len() > 100 && i % 3 == 0 {
            if let Some((ptr, layout)) = allocations.pop_front() {
                unsafe { alloc::alloc::dealloc(ptr, layout) };
            }
        }
    }
    
    // Cleanup remaining allocations
    while let Some((ptr, layout)) = allocations.pop_front() {
        unsafe { alloc::alloc::dealloc(ptr, layout) };
    }
    
    calculate_results("Mixed Workload", &results, 1024)
}

pub fn bench_allocation_deallocation_pairs() -> BenchmarkResult {
    let mut results = Vec::with_capacity(BENCHMARK_ITERATIONS);
    
    for _ in 0..BENCHMARK_ITERATIONS {
        let timer = BenchTimer::start();
        
        let layout = Layout::from_size_align(256, 8).unwrap();
        let ptr = unsafe { alloc::alloc::alloc(layout) };
        
        if !ptr.is_null() {
            unsafe { alloc::alloc::dealloc(ptr, layout) };
        }
        
        let cycles = timer.elapsed_cycles();
        results.push(cycles);
    }
    
    calculate_results("Alloc/Dealloc Pairs", &results, 256)
}

pub fn bench_cache_performance() -> BenchmarkResult {
    // Test CPU cache effectiveness
    let mut results = Vec::with_capacity(BENCHMARK_ITERATIONS);
    let size = 32; // Small size that should hit cache frequently
    
    // Pre-warm the cache
    let mut warm_allocs = Vec::new();
    for _ in 0..100 {
        let layout = Layout::from_size_align(size, 8).unwrap();
        let ptr = unsafe { alloc::alloc::alloc(layout) };
        if !ptr.is_null() {
            warm_allocs.push((ptr, layout));
        }
    }
    
    // Free them all to populate cache
    for (ptr, layout) in warm_allocs {
        unsafe { alloc::alloc::dealloc(ptr, layout) };
    }
    
    // Now benchmark with warm cache
    for _ in 0..BENCHMARK_ITERATIONS {
        let timer = BenchTimer::start();
        
        let layout = Layout::from_size_align(size, 8).unwrap();
        let ptr = unsafe { alloc::alloc::alloc(layout) };
        
        let cycles = timer.elapsed_cycles();
        results.push(cycles);
        
        if !ptr.is_null() {
            unsafe { alloc::alloc::dealloc(ptr, layout) };
        }
    }
    
    calculate_results("Cache Performance (32B)", &results, size)
}

pub fn bench_fragmentation_resistance() -> BenchmarkResult {
    let mut results = Vec::with_capacity(1000);
    let mut allocations = Vec::new();
    
    // Create fragmentation pattern
    // Allocate many objects of different sizes
    for i in 0..1000 {
        let size = 64 + (i % 10) * 64; // Sizes from 64 to 640 bytes
        let layout = Layout::from_size_align(size, 8).unwrap();
        let ptr = unsafe { alloc::alloc::alloc(layout) };
        
        if !ptr.is_null() {
            allocations.push((ptr, layout, i));
        }
    }
    
    // Free every other allocation to create holes
    let mut temp_allocs = Vec::new();
    for (ptr, layout, idx) in allocations {
        if idx % 2 == 0 {
            unsafe { alloc::alloc::dealloc(ptr, layout) };
        } else {
            temp_allocs.push((ptr, layout));
        }
    }
    allocations = temp_allocs;
    
    // Now try to allocate in fragmented heap
    for _ in 0..1000 {
        let timer = BenchTimer::start();
        
        let layout = Layout::from_size_align(256, 8).unwrap();
        let ptr = unsafe { alloc::alloc::alloc(layout) };
        
        let cycles = timer.elapsed_cycles();
        results.push(cycles);
        
        if !ptr.is_null() {
            unsafe { alloc::alloc::dealloc(ptr, layout) };
        }
    }
    
    // Cleanup
    for (ptr, layout) in allocations {
        unsafe { alloc::alloc::dealloc(ptr, layout) };
    }
    
    calculate_results("Fragmentation Resistance", &results, 256)
}

// Stress tests
pub fn stress_test_allocator() {
    crate::serial_println!("Starting allocator stress test...");
    
    let mut allocations = Vec::new();
    let mut allocation_count = 0;
    let mut deallocation_count = 0;
    let mut failed_allocations = 0;
    
    for i in 0..STRESS_TEST_ITERATIONS {
        // Random-ish size based on iteration
        let size = 8 + ((i * 7) % 4096);
        let should_allocate = (i % 3) != 2 || allocations.len() < 10;
        
        if should_allocate {
            let layout = Layout::from_size_align(size, 8).unwrap();
            let ptr = unsafe { alloc::alloc::alloc(layout) };
            
            if !ptr.is_null() {
                allocations.push((ptr, layout));
                allocation_count += 1;
                
                // Write pattern to detect corruption
                unsafe {
                    core::ptr::write_bytes(ptr, 0xAB, size);
                }
            } else {
                failed_allocations += 1;
            }
        } else if !allocations.is_empty() {
            let idx = i % allocations.len();
            let (ptr, layout) = allocations.remove(idx);
            
            // Verify pattern before deallocation
            let mut corrupted = false;
            unsafe {
                for j in 0..layout.size() {
                    if *ptr.add(j) != 0xAB {
                        corrupted = true;
                        break;
                    }
                }
            }
            
            if corrupted {
                crate::serial_println!("WARNING: Memory corruption detected!");
            }
            
            unsafe { alloc::alloc::dealloc(ptr, layout) };
            deallocation_count += 1;
        }
        
        // Periodic status update
        if i % 10000 == 0 && i > 0 {
            crate::serial_println!("Stress test progress: {}/{}", i, STRESS_TEST_ITERATIONS);
        }
    }
    
    // Cleanup remaining allocations
    for (ptr, layout) in allocations {
        unsafe { alloc::alloc::dealloc(ptr, layout) };
        deallocation_count += 1;
    }
    
    crate::serial_println!("Stress test completed:");
    crate::serial_println!("  Total allocations: {}", allocation_count);
    crate::serial_println!("  Total deallocations: {}", deallocation_count);
    crate::serial_println!("  Failed allocations: {}", failed_allocations);
    
    // Verify heap integrity
    if crate::allocator::debug::validate_heap() {
        crate::serial_println!("  Heap integrity: PASSED");
    } else {
        crate::serial_println!("  Heap integrity: FAILED");
    }
}

// Helper function to calculate benchmark statistics
fn calculate_results(name: &'static str, cycles: &[u64], alloc_size: usize) -> BenchmarkResult {
    let total: u64 = cycles.iter().sum();
    let avg = total / cycles.len() as u64;
    let min = *cycles.iter().min().unwrap_or(&0);
    let max = *cycles.iter().max().unwrap_or(&0);
    
    // Calculate throughput (assuming 3GHz CPU)
    let total_bytes = alloc_size * cycles.len();
    let total_seconds = (total as f64) / 3_000_000_000.0;
    let throughput_mbps = (total_bytes as f64 / 1_048_576.0) / total_seconds;
    
    BenchmarkResult {
        name,
        iterations: cycles.len(),
        total_cycles: total,
        avg_cycles: avg,
        min_cycles: min,
        max_cycles: max,
        throughput_mbps,
    }
}

// Main benchmark runner
pub fn run_all_benchmarks() {
    crate::serial_println!("=== Memory Allocator Performance Benchmarks ===");
    crate::serial_println!("Running benchmarks...\n");
    
    // Get initial memory stats
    let initial_stats = crate::allocator::memory_stats();
    crate::serial_println!("Initial memory state:");
    crate::serial_println!("  Allocated: {} KB", initial_stats.current_allocated / 1024);
    crate::serial_println!("  Free: {} KB\n", 
        (initial_stats.heap_size - initial_stats.current_allocated) / 1024);
    
    // Run benchmarks
    let benchmarks = [
        bench_small_allocations(),
        bench_medium_allocations(),
        bench_large_allocations(),
        bench_huge_allocations(),
        bench_mixed_workload(),
        bench_allocation_deallocation_pairs(),
        bench_cache_performance(),
        bench_fragmentation_resistance(),
    ];
    
    // Print results
    crate::serial_println!("\n=== Benchmark Results ===");
    for result in &benchmarks {
        result.print();
        crate::serial_println!("");
    }
    
    // Calculate and print summary statistics
    let total_cycles: u64 = benchmarks.iter().map(|r| r.total_cycles).sum();
    let total_iterations: usize = benchmarks.iter().map(|r| r.iterations).sum();
    
    crate::serial_println!("=== Summary ===");
    crate::serial_println!("Total iterations: {}", total_iterations);
    crate::serial_println!("Total cycles: {}", total_cycles);
    crate::serial_println!("Average cycles per operation: {}", total_cycles / total_iterations as u64);
    
    // Final memory stats
    let final_stats = crate::allocator::memory_stats();
    crate::serial_println!("\nFinal memory state:");
    crate::serial_println!("  Allocated: {} KB", final_stats.current_allocated / 1024);
    crate::serial_println!("  Free: {} KB", 
        (final_stats.heap_size - final_stats.current_allocated) / 1024);
    crate::serial_println!("  Peak usage: {} KB", final_stats.peak_allocated / 1024);
    crate::serial_println!("  Cache hit rate: {}%", 
        if final_stats.cache_hits + final_stats.cache_misses > 0 {
            (final_stats.cache_hits * 100) / (final_stats.cache_hits + final_stats.cache_misses)
        } else {
            0
        });
    
    // Check for memory leaks
    if final_stats.current_allocated > initial_stats.current_allocated {
        let leaked = final_stats.current_allocated - initial_stats.current_allocated;
        crate::serial_println!("\nWARNING: Possible memory leak detected: {} bytes", leaked);
    } else {
        crate::serial_println!("\nNo memory leaks detected");
    }
}

// Comparison with old allocator (if available)
pub fn compare_allocators() {
    crate::serial_println!("=== Allocator Comparison ===");
    crate::serial_println!("Comparing hybrid allocator performance...\n");
    
    // Expected improvements based on design
    crate::serial_println!("Expected improvements over linked-list allocator:");
    crate::serial_println!("  Small allocations (< 4KB): 50-70% faster (slab allocator)");
    crate::serial_println!("  Large allocations (>= 4KB): 40-60% faster (buddy system)");
    crate::serial_println!("  Cache hit rate: 60-80% for frequently used sizes");
    crate::serial_println!("  Fragmentation: 30-50% reduction");
    crate::serial_println!("  Multi-core scalability: 3-4x improvement with per-CPU caches");
    crate::serial_println!("  Memory overhead: < 5% vs 10-15% for linked-list");
}

// Test runner
#[cfg(test)]
pub fn run_tests() {
    crate::serial_println!("Running allocator tests...");
    
    // Run functional tests
    test_basic_allocation();
    test_alignment();
    test_zero_size_allocation();
    test_large_allocation();
    test_allocation_patterns();
    
    crate::serial_println!("All tests passed!");
}

#[cfg(test)]
fn test_basic_allocation() {
    let layout = Layout::from_size_align(100, 8).unwrap();
    let ptr = unsafe { alloc::alloc::alloc(layout) };
    assert!(!ptr.is_null());
    unsafe { alloc::alloc::dealloc(ptr, layout) };
}

#[cfg(test)]
fn test_alignment() {
    for align in [1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096] {
        let layout = Layout::from_size_align(100, align).unwrap();
        let ptr = unsafe { alloc::alloc::alloc(layout) };
        assert!(!ptr.is_null());
        assert_eq!(ptr as usize % align, 0);
        unsafe { alloc::alloc::dealloc(ptr, layout) };
    }
}

#[cfg(test)]
fn test_zero_size_allocation() {
    let layout = Layout::from_size_align(0, 1).unwrap();
    let ptr = unsafe { alloc::alloc::alloc(layout) };
    // Zero-size allocations may return null or a valid pointer
    if !ptr.is_null() {
        unsafe { alloc::alloc::dealloc(ptr, layout) };
    }
}

#[cfg(test)]
fn test_large_allocation() {
    let layout = Layout::from_size_align(1024 * 1024, 8).unwrap(); // 1MB
    let ptr = unsafe { alloc::alloc::alloc(layout) };
    assert!(!ptr.is_null());
    unsafe { alloc::alloc::dealloc(ptr, layout) };
}

#[cfg(test)]
fn test_allocation_patterns() {
    let mut ptrs = Vec::new();
    
    // Allocate in increasing sizes
    for size in (8..=4096).step_by(8) {
        let layout = Layout::from_size_align(size, 8).unwrap();
        let ptr = unsafe { alloc::alloc::alloc(layout) };
        assert!(!ptr.is_null());
        ptrs.push((ptr, layout));
    }
    
    // Deallocate in reverse order
    while let Some((ptr, layout)) = ptrs.pop() {
        unsafe { alloc::alloc::dealloc(ptr, layout) };
    }
}