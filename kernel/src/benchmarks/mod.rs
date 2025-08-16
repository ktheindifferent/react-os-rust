// Performance Benchmarking Framework

use crate::{serial_println, timer};
use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;
use core::arch::x86_64::{_rdtsc, __cpuid};

pub struct BenchmarkResult {
    pub name: String,
    pub iterations: u64,
    pub total_cycles: u64,
    pub min_cycles: u64,
    pub max_cycles: u64,
    pub avg_cycles: u64,
    pub throughput: Option<f64>, // Operations per second
}

pub struct BenchmarkRunner {
    results: Vec<BenchmarkResult>,
    warmup_iterations: u64,
    benchmark_iterations: u64,
}

impl BenchmarkRunner {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            warmup_iterations: 100,
            benchmark_iterations: 1000,
        }
    }
    
    pub fn set_iterations(&mut self, warmup: u64, benchmark: u64) {
        self.warmup_iterations = warmup;
        self.benchmark_iterations = benchmark;
    }
    
    pub fn run_benchmark<F>(&mut self, name: &str, mut bench_fn: F)
    where
        F: FnMut(),
    {
        serial_println!("Benchmarking {}...", name);
        
        // Warmup phase
        for _ in 0..self.warmup_iterations {
            bench_fn();
        }
        
        // Measurement phase
        let mut cycles = Vec::new();
        let mut total_cycles = 0u64;
        let mut min_cycles = u64::MAX;
        let mut max_cycles = 0u64;
        
        for _ in 0..self.benchmark_iterations {
            let start = unsafe { _rdtsc() };
            bench_fn();
            let end = unsafe { _rdtsc() };
            
            let elapsed = end - start;
            cycles.push(elapsed);
            total_cycles += elapsed;
            
            if elapsed < min_cycles {
                min_cycles = elapsed;
            }
            if elapsed > max_cycles {
                max_cycles = elapsed;
            }
        }
        
        let avg_cycles = total_cycles / self.benchmark_iterations;
        
        self.results.push(BenchmarkResult {
            name: String::from(name),
            iterations: self.benchmark_iterations,
            total_cycles,
            min_cycles,
            max_cycles,
            avg_cycles,
            throughput: None,
        });
    }
    
    pub fn run_throughput_benchmark<F>(
        &mut self,
        name: &str,
        operations_per_iteration: u64,
        mut bench_fn: F,
    ) where
        F: FnMut(),
    {
        serial_println!("Benchmarking {} (throughput)...", name);
        
        // Warmup phase
        for _ in 0..self.warmup_iterations {
            bench_fn();
        }
        
        // Measurement phase
        let start = unsafe { _rdtsc() };
        
        for _ in 0..self.benchmark_iterations {
            bench_fn();
        }
        
        let end = unsafe { _rdtsc() };
        let total_cycles = end - start;
        let total_operations = self.benchmark_iterations * operations_per_iteration;
        
        // Estimate CPU frequency (simplified)
        let cpu_freq = estimate_cpu_frequency();
        let throughput = (total_operations as f64 * cpu_freq) / total_cycles as f64;
        
        self.results.push(BenchmarkResult {
            name: String::from(name),
            iterations: self.benchmark_iterations,
            total_cycles,
            min_cycles: 0,
            max_cycles: 0,
            avg_cycles: total_cycles / self.benchmark_iterations,
            throughput: Some(throughput),
        });
    }
    
    pub fn summary(&self) {
        serial_println!("\n===== Benchmark Results =====");
        serial_println!("{:<40} {:>15} {:>15} {:>15} {:>15}",
            "Benchmark", "Avg Cycles", "Min Cycles", "Max Cycles", "Throughput");
        serial_println!("{:-<100}", "");
        
        for result in &self.results {
            if let Some(throughput) = result.throughput {
                serial_println!("{:<40} {:>15} {:>15} {:>15} {:>12.2} ops/s",
                    result.name, result.avg_cycles, result.min_cycles,
                    result.max_cycles, throughput);
            } else {
                serial_println!("{:<40} {:>15} {:>15} {:>15} {:>15}",
                    result.name, result.avg_cycles, result.min_cycles,
                    result.max_cycles, "N/A");
            }
        }
    }
}

// Estimate CPU frequency using TSC
fn estimate_cpu_frequency() -> f64 {
    // Simplified frequency estimation
    // In a real implementation, this would use CPUID or calibration
    2_400_000_000.0 // 2.4 GHz default
}

// Memory benchmarks
pub mod memory_benchmarks {
    use super::*;
    use alloc::vec;
    
    pub fn run_memory_benchmarks(runner: &mut BenchmarkRunner) {
        // Small allocation benchmark
        runner.run_benchmark("memory::small_alloc", || {
            let _data = vec![0u8; 64];
        });
        
        // Medium allocation benchmark
        runner.run_benchmark("memory::medium_alloc", || {
            let _data = vec![0u8; 4096];
        });
        
        // Large allocation benchmark
        runner.run_benchmark("memory::large_alloc", || {
            let _data = vec![0u8; 1024 * 1024];
        });
        
        // Memory copy benchmark
        runner.run_throughput_benchmark("memory::copy_1kb", 1024, || {
            let src = vec![0xAA_u8; 1024];
            let mut dst = vec![0_u8; 1024];
            dst.copy_from_slice(&src);
        });
        
        // Memory fill benchmark
        runner.run_throughput_benchmark("memory::fill_4kb", 4096, || {
            let mut buffer = vec![0_u8; 4096];
            for byte in &mut buffer {
                *byte = 0xFF;
            }
        });
        
        // Random access benchmark
        runner.run_benchmark("memory::random_access", || {
            let mut data = vec![0_u64; 1024];
            let indices = [13, 511, 256, 777, 100, 900, 42, 666];
            
            for &idx in &indices {
                data[idx] = idx as u64;
            }
        });
    }
}

// Context switch benchmarks
pub mod scheduler_benchmarks {
    use super::*;
    
    pub fn run_scheduler_benchmarks(runner: &mut BenchmarkRunner) {
        // Context save benchmark
        runner.run_benchmark("scheduler::context_save", || {
            let mut context = CpuContext::default();
            save_context(&mut context);
        });
        
        // Context restore benchmark
        runner.run_benchmark("scheduler::context_restore", || {
            let context = CpuContext::default();
            restore_context(&context);
        });
        
        // Thread switch simulation
        runner.run_benchmark("scheduler::thread_switch", || {
            let mut ctx1 = CpuContext::default();
            let ctx2 = CpuContext::default();
            
            save_context(&mut ctx1);
            restore_context(&ctx2);
        });
    }
    
    #[derive(Default, Clone)]
    struct CpuContext {
        regs: [u64; 16],
        rip: u64,
        rflags: u64,
    }
    
    fn save_context(ctx: &mut CpuContext) {
        // Simulated context save
        for i in 0..16 {
            ctx.regs[i] = i as u64;
        }
        ctx.rip = 0xFFFF800000001000;
        ctx.rflags = 0x202;
    }
    
    fn restore_context(ctx: &CpuContext) {
        // Simulated context restore
        let _ = ctx.regs[0];
        let _ = ctx.rip;
        let _ = ctx.rflags;
    }
}

// File system benchmarks
pub mod fs_benchmarks {
    use super::*;
    
    pub fn run_fs_benchmarks(runner: &mut BenchmarkRunner) {
        // Path parsing benchmark
        runner.run_benchmark("fs::path_parse", || {
            let path = "/home/user/documents/file.txt";
            let components: Vec<&str> = path.split('/').collect();
            let _ = components.len();
        });
        
        // Directory lookup simulation
        runner.run_benchmark("fs::dir_lookup", || {
            let mut entries = vec![
                (".", 1),
                ("..", 2),
                ("file1.txt", 100),
                ("file2.txt", 101),
                ("subdir", 200),
            ];
            
            let target = "file2.txt";
            let _ = entries.iter().find(|(name, _)| *name == target);
        });
        
        // Buffer cache lookup
        runner.run_throughput_benchmark("fs::cache_lookup", 100, || {
            let cache = vec![(0, vec![0u8; 512]); 100];
            let block_id = 42;
            let _ = cache.iter().find(|(id, _)| *id == block_id);
        });
    }
}

// Network benchmarks
pub mod network_benchmarks {
    use super::*;
    
    pub fn run_network_benchmarks(runner: &mut BenchmarkRunner) {
        // Checksum calculation
        runner.run_benchmark("net::checksum_1kb", || {
            let data = vec![0xFF_u8; 1024];
            calculate_checksum(&data);
        });
        
        // Packet parsing
        runner.run_benchmark("net::parse_ipv4", || {
            let packet = vec![
                0x45, 0x00, 0x00, 0x3C, // Version, IHL, TOS, Length
                0x1C, 0x46, 0x40, 0x00, // ID, Flags, Fragment
                0x40, 0x06, 0xB1, 0xE6, // TTL, Protocol, Checksum
                0xC0, 0xA8, 0x01, 0x01, // Source IP
                0xC0, 0xA8, 0x01, 0x02, // Dest IP
            ];
            parse_ipv4_header(&packet);
        });
        
        // TCP state machine
        runner.run_benchmark("net::tcp_state_transition", || {
            let mut state = TcpState::Listen;
            state = match state {
                TcpState::Listen => TcpState::SynReceived,
                TcpState::SynReceived => TcpState::Established,
                TcpState::Established => TcpState::FinWait1,
                _ => TcpState::Closed,
            };
        });
    }
    
    fn calculate_checksum(data: &[u8]) {
        let mut sum = 0u32;
        let mut i = 0;
        
        while i < data.len() - 1 {
            sum += ((data[i] as u32) << 8) | (data[i + 1] as u32);
            i += 2;
        }
        
        if i < data.len() {
            sum += (data[i] as u32) << 8;
        }
        
        while (sum >> 16) != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }
        
        let _ = !sum as u16;
    }
    
    fn parse_ipv4_header(packet: &[u8]) {
        if packet.len() >= 20 {
            let _version = packet[0] >> 4;
            let _ihl = packet[0] & 0x0F;
            let _total_length = ((packet[2] as u16) << 8) | (packet[3] as u16);
            let _protocol = packet[9];
        }
    }
    
    enum TcpState {
        Closed,
        Listen,
        SynReceived,
        Established,
        FinWait1,
    }
}

// Interrupt handling benchmarks
pub mod interrupt_benchmarks {
    use super::*;
    
    pub fn run_interrupt_benchmarks(runner: &mut BenchmarkRunner) {
        // Interrupt entry overhead
        runner.run_benchmark("interrupt::entry_overhead", || {
            simulate_interrupt_entry();
        });
        
        // Interrupt exit overhead
        runner.run_benchmark("interrupt::exit_overhead", || {
            simulate_interrupt_exit();
        });
        
        // IRQ handling
        runner.run_benchmark("interrupt::irq_handling", || {
            handle_irq(32); // Timer IRQ
        });
    }
    
    fn simulate_interrupt_entry() {
        // Save registers
        let mut saved_regs = [0u64; 16];
        for i in 0..16 {
            saved_regs[i] = i as u64;
        }
    }
    
    fn simulate_interrupt_exit() {
        // Restore registers
        let saved_regs = [0u64; 16];
        for i in 0..16 {
            let _ = saved_regs[i];
        }
    }
    
    fn handle_irq(irq: u8) {
        match irq {
            32 => {}, // Timer
            33 => {}, // Keyboard
            _ => {},
        }
    }
}

// Main benchmark entry point
pub fn run_all_benchmarks() {
    serial_println!("\n===== Starting Performance Benchmarks =====\n");
    
    let mut runner = BenchmarkRunner::new();
    runner.set_iterations(100, 1000);
    
    // Run memory benchmarks
    serial_println!("\n[Memory Benchmarks]");
    memory_benchmarks::run_memory_benchmarks(&mut runner);
    
    // Run scheduler benchmarks
    serial_println!("\n[Scheduler Benchmarks]");
    scheduler_benchmarks::run_scheduler_benchmarks(&mut runner);
    
    // Run file system benchmarks
    serial_println!("\n[File System Benchmarks]");
    fs_benchmarks::run_fs_benchmarks(&mut runner);
    
    // Run network benchmarks
    serial_println!("\n[Network Benchmarks]");
    network_benchmarks::run_network_benchmarks(&mut runner);
    
    // Run interrupt benchmarks
    serial_println!("\n[Interrupt Benchmarks]");
    interrupt_benchmarks::run_interrupt_benchmarks(&mut runner);
    
    // Display summary
    runner.summary();
}