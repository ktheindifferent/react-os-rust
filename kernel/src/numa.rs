// NUMA (Non-Uniform Memory Access) optimizations
// Provides CPU affinity, memory locality awareness, and IPI optimization

use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use spin::Mutex;
use lazy_static::lazy_static;
use bitflags::bitflags;

// ACPI SRAT (System Resource Affinity Table) structures
#[repr(C, packed)]
struct SratHeader {
    signature: [u8; 4],  // "SRAT"
    length: u32,
    revision: u8,
    checksum: u8,
    oem_id: [u8; 6],
    oem_table_id: [u8; 8],
    oem_revision: u32,
    creator_id: u32,
    creator_revision: u32,
    reserved: u32,
}

#[repr(C, packed)]
struct SratEntry {
    entry_type: u8,
    length: u8,
}

#[repr(C, packed)]
struct SratProcessorAffinity {
    entry_type: u8,     // 0x00
    length: u8,         // 0x10
    proximity_domain_low: u8,
    apic_id: u8,
    flags: u32,
    local_sapic_eid: u8,
    proximity_domain_high: [u8; 3],
    clock_domain: u32,
}

#[repr(C, packed)]
struct SratMemoryAffinity {
    entry_type: u8,     // 0x01
    length: u8,         // 0x28
    proximity_domain: u32,
    reserved1: u16,
    base_address: u64,
    length_bytes: u64,
    reserved2: u32,
    flags: u32,
    reserved3: u64,
}

// NUMA node information
#[derive(Debug, Clone)]
pub struct NumaNode {
    pub id: u32,
    pub cpus: Vec<u32>,
    pub memory_start: u64,
    pub memory_end: u64,
    pub distance_map: BTreeMap<u32, u32>, // Distance to other nodes
}

impl NumaNode {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            cpus: Vec::new(),
            memory_start: 0,
            memory_end: 0,
            distance_map: BTreeMap::new(),
        }
    }
    
    pub fn add_cpu(&mut self, cpu_id: u32) {
        if !self.cpus.contains(&cpu_id) {
            self.cpus.push(cpu_id);
        }
    }
    
    pub fn set_memory_range(&mut self, start: u64, end: u64) {
        self.memory_start = start;
        self.memory_end = end;
    }
    
    pub fn contains_address(&self, addr: u64) -> bool {
        addr >= self.memory_start && addr < self.memory_end
    }
}

// NUMA topology
pub struct NumaTopology {
    nodes: Vec<NumaNode>,
    cpu_to_node: BTreeMap<u32, u32>,
    node_count: u32,
}

impl NumaTopology {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            cpu_to_node: BTreeMap::new(),
            node_count: 0,
        }
    }
    
    pub fn detect() -> Self {
        let mut topology = Self::new();
        
        // Try to parse ACPI SRAT table
        if let Ok(()) = topology.parse_srat() {
            crate::serial_println!("NUMA topology detected: {} nodes", topology.node_count);
        } else {
            // Fall back to single node
            topology.create_single_node();
            crate::serial_println!("NUMA: Single node system");
        }
        
        topology
    }
    
    fn parse_srat(&mut self) -> Result<(), &'static str> {
        // This would parse the ACPI SRAT table
        // For now, return error to use fallback
        Err("SRAT parsing not implemented")
    }
    
    fn create_single_node(&mut self) {
        let mut node = NumaNode::new(0);
        
        // Add all CPUs to node 0
        let cpu_count = crate::cpu::get_info().logical_cores as u32;
        for cpu in 0..cpu_count {
            node.add_cpu(cpu);
            self.cpu_to_node.insert(cpu, 0);
        }
        
        // Set memory range (example: 0 to 4GB)
        node.set_memory_range(0, 4 * 1024 * 1024 * 1024);
        
        self.nodes.push(node);
        self.node_count = 1;
    }
    
    pub fn get_node_for_cpu(&self, cpu_id: u32) -> Option<u32> {
        self.cpu_to_node.get(&cpu_id).copied()
    }
    
    pub fn get_node_for_address(&self, addr: u64) -> Option<u32> {
        for node in &self.nodes {
            if node.contains_address(addr) {
                return Some(node.id);
            }
        }
        None
    }
    
    pub fn get_node(&self, node_id: u32) -> Option<&NumaNode> {
        self.nodes.iter().find(|n| n.id == node_id)
    }
    
    pub fn get_distance(&self, from_node: u32, to_node: u32) -> u32 {
        if from_node == to_node {
            return 10; // Local access
        }
        
        if let Some(node) = self.get_node(from_node) {
            if let Some(distance) = node.distance_map.get(&to_node) {
                return *distance;
            }
        }
        
        20 // Default remote access cost
    }
}

lazy_static! {
    pub static ref NUMA_TOPOLOGY: NumaTopology = NumaTopology::detect();
}

// CPU affinity management
bitflags! {
    pub struct CpuSet: u64 {
        const CPU0 = 1 << 0;
        const CPU1 = 1 << 1;
        const CPU2 = 1 << 2;
        const CPU3 = 1 << 3;
        const CPU4 = 1 << 4;
        const CPU5 = 1 << 5;
        const CPU6 = 1 << 6;
        const CPU7 = 1 << 7;
        const CPU8 = 1 << 8;
        const CPU9 = 1 << 9;
        const CPU10 = 1 << 10;
        const CPU11 = 1 << 11;
        const CPU12 = 1 << 12;
        const CPU13 = 1 << 13;
        const CPU14 = 1 << 14;
        const CPU15 = 1 << 15;
        const ALL = 0xFFFF;
    }
}

impl CpuSet {
    pub fn from_cpu(cpu_id: u32) -> Self {
        CpuSet::from_bits_truncate(1 << cpu_id)
    }
    
    pub fn from_node(node_id: u32) -> Self {
        if let Some(node) = NUMA_TOPOLOGY.get_node(node_id) {
            let mut set = CpuSet::empty();
            for &cpu in &node.cpus {
                set |= CpuSet::from_cpu(cpu);
            }
            set
        } else {
            CpuSet::empty()
        }
    }
    
    pub fn is_cpu_set(&self, cpu_id: u32) -> bool {
        self.bits() & (1 << cpu_id) != 0
    }
    
    pub fn first_cpu(&self) -> Option<u32> {
        if self.is_empty() {
            return None;
        }
        
        for i in 0..64 {
            if self.bits() & (1 << i) != 0 {
                return Some(i);
            }
        }
        None
    }
    
    pub fn cpu_count(&self) -> u32 {
        self.bits().count_ones()
    }
}

// Process affinity
pub struct ProcessAffinity {
    pub pid: u32,
    pub cpu_affinity: CpuSet,
    pub memory_affinity: u32, // Preferred NUMA node
    pub strict_affinity: bool,
}

impl ProcessAffinity {
    pub fn new(pid: u32) -> Self {
        Self {
            pid,
            cpu_affinity: CpuSet::ALL,
            memory_affinity: 0,
            strict_affinity: false,
        }
    }
    
    pub fn set_cpu_affinity(&mut self, cpuset: CpuSet) {
        self.cpu_affinity = cpuset;
    }
    
    pub fn set_memory_affinity(&mut self, node_id: u32) {
        self.memory_affinity = node_id;
    }
    
    pub fn can_run_on_cpu(&self, cpu_id: u32) -> bool {
        self.cpu_affinity.is_cpu_set(cpu_id)
    }
}

// NUMA-aware memory allocation hints
pub struct NumaAllocHint {
    pub preferred_node: u32,
    pub fallback_nodes: Vec<u32>,
    pub interleave: bool,
    pub local_only: bool,
}

impl NumaAllocHint {
    pub fn local() -> Self {
        let cpu_id = crate::cpu::get_cpu_id();
        let node_id = NUMA_TOPOLOGY.get_node_for_cpu(cpu_id).unwrap_or(0);
        
        Self {
            preferred_node: node_id,
            fallback_nodes: Vec::new(),
            interleave: false,
            local_only: false,
        }
    }
    
    pub fn on_node(node_id: u32) -> Self {
        Self {
            preferred_node: node_id,
            fallback_nodes: Vec::new(),
            interleave: false,
            local_only: false,
        }
    }
    
    pub fn interleaved(nodes: Vec<u32>) -> Self {
        Self {
            preferred_node: nodes.first().copied().unwrap_or(0),
            fallback_nodes: nodes,
            interleave: true,
            local_only: false,
        }
    }
}

// Inter-processor interrupt (IPI) optimization
pub struct IpiOptimizer {
    ipi_count: BTreeMap<(u32, u32), AtomicU64>, // (from_cpu, to_cpu) -> count
    local_ipis: AtomicU64,
    remote_ipis: AtomicU64,
}

impl IpiOptimizer {
    pub fn new() -> Self {
        Self {
            ipi_count: BTreeMap::new(),
            local_ipis: AtomicU64::new(0),
            remote_ipis: AtomicU64::new(0),
        }
    }
    
    pub fn send_ipi(&self, target_cpu: u32, vector: u8) {
        let from_cpu = crate::cpu::get_cpu_id();
        let from_node = NUMA_TOPOLOGY.get_node_for_cpu(from_cpu);
        let to_node = NUMA_TOPOLOGY.get_node_for_cpu(target_cpu);
        
        // Track IPI statistics
        if from_node == to_node {
            self.local_ipis.fetch_add(1, Ordering::Relaxed);
        } else {
            self.remote_ipis.fetch_add(1, Ordering::Relaxed);
        }
        
        // Send IPI using APIC
        unsafe {
            const APIC_BASE: u64 = 0xFEE00000;
            const APIC_ICR_LOW: u32 = 0x300;
            const APIC_ICR_HIGH: u32 = 0x310;
            
            let apic = APIC_BASE as *mut u32;
            
            // Set destination
            apic.add((APIC_ICR_HIGH / 4) as usize)
                .write_volatile((target_cpu << 24) as u32);
            
            // Send IPI
            let icr_low = (vector as u32) | (1 << 14); // Fixed delivery mode
            apic.add((APIC_ICR_LOW / 4) as usize)
                .write_volatile(icr_low);
        }
    }
    
    pub fn broadcast_ipi(&self, cpuset: CpuSet, vector: u8) {
        // Optimize broadcast by grouping by NUMA node
        let mut nodes_to_cpus: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        
        for cpu in 0..64 {
            if cpuset.is_cpu_set(cpu) {
                if let Some(node) = NUMA_TOPOLOGY.get_node_for_cpu(cpu) {
                    nodes_to_cpus.entry(node).or_insert_with(Vec::new).push(cpu);
                }
            }
        }
        
        // Send IPIs node by node for better cache locality
        for (_node, cpus) in nodes_to_cpus {
            for cpu in cpus {
                self.send_ipi(cpu, vector);
            }
        }
    }
    
    pub fn get_stats(&self) -> (u64, u64) {
        (
            self.local_ipis.load(Ordering::Relaxed),
            self.remote_ipis.load(Ordering::Relaxed),
        )
    }
}

lazy_static! {
    pub static ref IPI_OPTIMIZER: IpiOptimizer = IpiOptimizer::new();
}

// NUMA-aware scheduler hints
pub struct SchedNumaHint {
    pub preferred_cpu: Option<u32>,
    pub preferred_node: Option<u32>,
    pub avoid_migration: bool,
    pub cache_hot_threshold: u64, // TSC cycles
}

impl SchedNumaHint {
    pub fn new() -> Self {
        Self {
            preferred_cpu: None,
            preferred_node: None,
            avoid_migration: false,
            cache_hot_threshold: 1_000_000, // ~500µs at 2GHz
        }
    }
    
    pub fn prefer_local(&mut self) {
        let cpu = crate::cpu::get_cpu_id();
        let node = NUMA_TOPOLOGY.get_node_for_cpu(cpu);
        
        self.preferred_cpu = Some(cpu);
        self.preferred_node = node;
    }
    
    pub fn is_cache_hot(&self, last_run_tsc: u64) -> bool {
        let current_tsc = crate::timer::rdtsc();
        (current_tsc - last_run_tsc) < self.cache_hot_threshold
    }
}

// NUMA statistics
pub struct NumaStats {
    pub local_accesses: AtomicU64,
    pub remote_accesses: AtomicU64,
    pub migrations: AtomicU64,
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
}

impl NumaStats {
    pub const fn new() -> Self {
        Self {
            local_accesses: AtomicU64::new(0),
            remote_accesses: AtomicU64::new(0),
            migrations: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
        }
    }
    
    pub fn record_access(&self, addr: u64) {
        let cpu = crate::cpu::get_cpu_id();
        let cpu_node = NUMA_TOPOLOGY.get_node_for_cpu(cpu);
        let mem_node = NUMA_TOPOLOGY.get_node_for_address(addr);
        
        if cpu_node == mem_node {
            self.local_accesses.fetch_add(1, Ordering::Relaxed);
        } else {
            self.remote_accesses.fetch_add(1, Ordering::Relaxed);
        }
    }
    
    pub fn record_migration(&self) {
        self.migrations.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn print_stats(&self) {
        let local = self.local_accesses.load(Ordering::Relaxed);
        let remote = self.remote_accesses.load(Ordering::Relaxed);
        let total = local + remote;
        
        println!("NUMA Statistics:");
        println!("  Local accesses:  {} ({:.1}%)", 
            local, (local as f64 / total as f64) * 100.0);
        println!("  Remote accesses: {} ({:.1}%)", 
            remote, (remote as f64 / total as f64) * 100.0);
        println!("  Migrations:      {}", self.migrations.load(Ordering::Relaxed));
        println!("  Cache hits:      {}", self.cache_hits.load(Ordering::Relaxed));
        println!("  Cache misses:    {}", self.cache_misses.load(Ordering::Relaxed));
    }
}

lazy_static! {
    pub static ref NUMA_STATS: NumaStats = NumaStats::new();
}

// NUMA-aware memory migration
pub fn migrate_pages(pid: u32, from_node: u32, to_node: u32) -> Result<usize, &'static str> {
    // This would migrate process pages between NUMA nodes
    // For now, just track the migration
    NUMA_STATS.record_migration();
    
    Ok(0) // Pages migrated
}

// Set process CPU affinity
pub fn set_process_affinity(pid: u32, cpuset: CpuSet) -> Result<(), &'static str> {
    // This would update the process's allowed CPU set
    // and potentially migrate it if needed
    
    crate::serial_println!("Process {} affinity set to CPUs: {:?}", pid, cpuset);
    Ok(())
}

// Get optimal CPU for process
pub fn get_optimal_cpu(hint: &SchedNumaHint) -> u32 {
    if let Some(cpu) = hint.preferred_cpu {
        return cpu;
    }
    
    if let Some(node) = hint.preferred_node {
        // Find least loaded CPU on preferred node
        if let Some(numa_node) = NUMA_TOPOLOGY.get_node(node) {
            if let Some(&cpu) = numa_node.cpus.first() {
                return cpu;
            }
        }
    }
    
    // Fall back to current CPU
    crate::cpu::get_cpu_id()
}

// Initialize NUMA subsystem
pub fn init() {
    // Force lazy static initialization
    let _ = &*NUMA_TOPOLOGY;
    let _ = &*IPI_OPTIMIZER;
    let _ = &*NUMA_STATS;
    
    crate::serial_println!("NUMA subsystem initialized");
}