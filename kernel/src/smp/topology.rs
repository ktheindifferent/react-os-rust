use core::arch::x86_64::{__cpuid, __cpuid_count};
use alloc::vec::Vec;
use super::{SMP_MANAGER, CpuInfo};

#[derive(Debug, Clone, Copy)]
pub struct CpuTopology {
    pub smt_bits: u8,
    pub core_bits: u8,
    pub package_bits: u8,
    pub max_logical_processors: u32,
    pub max_cores_per_package: u32,
    pub max_threads_per_core: u32,
}

impl CpuTopology {
    pub fn new() -> Self {
        Self {
            smt_bits: 0,
            core_bits: 0,
            package_bits: 0,
            max_logical_processors: 1,
            max_cores_per_package: 1,
            max_threads_per_core: 1,
        }
    }
}

pub fn detect_topology() {
    let topology = detect_cpu_topology();
    apply_topology_to_cpus(topology);
    
    crate::serial_println!("SMP: CPU Topology detected:");
    crate::serial_println!("  Max logical processors: {}", topology.max_logical_processors);
    crate::serial_println!("  Max cores per package: {}", topology.max_cores_per_package);
    crate::serial_println!("  Max threads per core: {}", topology.max_threads_per_core);
}

fn detect_cpu_topology() -> CpuTopology {
    let mut topology = CpuTopology::new();
    
    unsafe {
        let cpuid_0 = __cpuid(0);
        let max_cpuid = cpuid_0.eax;
        
        if max_cpuid >= 0xB {
            detect_extended_topology(&mut topology);
        } else if max_cpuid >= 4 {
            detect_legacy_topology(&mut topology);
        } else {
            detect_basic_topology(&mut topology);
        }
    }
    
    topology
}

unsafe fn detect_extended_topology(topology: &mut CpuTopology) {
    let mut smt_width = 0;
    let mut core_width = 0;
    
    for level in 0.. {
        let cpuid = __cpuid_count(0xB, level);
        
        if cpuid.eax == 0 && cpuid.ebx == 0 {
            break;
        }
        
        let level_type = (cpuid.ecx >> 8) & 0xFF;
        let level_width = cpuid.eax & 0x1F;
        
        match level_type {
            1 => {
                smt_width = level_width as u8;
                topology.max_threads_per_core = cpuid.ebx;
            }
            2 => {
                core_width = level_width as u8;
                topology.max_logical_processors = cpuid.ebx;
            }
            _ => {}
        }
    }
    
    topology.smt_bits = smt_width;
    topology.core_bits = core_width.saturating_sub(smt_width);
    topology.package_bits = 8;
    
    if topology.max_threads_per_core > 0 {
        topology.max_cores_per_package = 
            topology.max_logical_processors / topology.max_threads_per_core;
    }
}

unsafe fn detect_legacy_topology(topology: &mut CpuTopology) {
    let cpuid_1 = __cpuid(1);
    topology.max_logical_processors = ((cpuid_1.ebx >> 16) & 0xFF) as u32;
    
    let cpuid_4 = __cpuid_count(4, 0);
    topology.max_cores_per_package = ((cpuid_4.eax >> 26) & 0x3F) as u32 + 1;
    
    if topology.max_cores_per_package > 0 {
        topology.max_threads_per_core = 
            topology.max_logical_processors / topology.max_cores_per_package;
    }
    
    topology.smt_bits = (topology.max_threads_per_core - 1).leading_zeros() as u8;
    topology.core_bits = (topology.max_cores_per_package - 1).leading_zeros() as u8;
    topology.package_bits = 8;
}

unsafe fn detect_basic_topology(topology: &mut CpuTopology) {
    let cpuid_1 = __cpuid(1);
    
    let logical_cpus = ((cpuid_1.ebx >> 16) & 0xFF) as u32;
    topology.max_logical_processors = if logical_cpus > 0 { logical_cpus } else { 1 };
    
    if cpuid_1.edx & (1 << 28) != 0 {
        topology.max_threads_per_core = 2;
        topology.max_cores_per_package = topology.max_logical_processors / 2;
    } else {
        topology.max_threads_per_core = 1;
        topology.max_cores_per_package = topology.max_logical_processors;
    }
    
    topology.smt_bits = if topology.max_threads_per_core > 1 { 1 } else { 0 };
    topology.core_bits = (topology.max_cores_per_package - 1).leading_zeros() as u8;
    topology.package_bits = 8;
}

fn apply_topology_to_cpus(topology: CpuTopology) {
    let mut smp = SMP_MANAGER.lock();
    
    for cpu in &mut smp.cpus {
        let apic_id = cpu.apic_id;
        
        cpu.thread_id = (apic_id & ((1 << topology.smt_bits) - 1)) as u8;
        
        let core_id_shift = topology.smt_bits;
        let core_id_mask = (1 << topology.core_bits) - 1;
        cpu.core_id = ((apic_id >> core_id_shift) & core_id_mask) as u8;
        
        let package_id_shift = topology.smt_bits + topology.core_bits;
        cpu.package_id = (apic_id >> package_id_shift) as u8;
    }
}

#[derive(Debug, Clone)]
pub struct CacheInfo {
    pub level: u8,
    pub cache_type: CacheType,
    pub size: u32,
    pub ways: u32,
    pub line_size: u32,
    pub sets: u32,
    pub shared_by_threads: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CacheType {
    Data,
    Instruction,
    Unified,
}

pub fn detect_cache_topology() -> Vec<CacheInfo> {
    let mut caches = Vec::new();
    
    unsafe {
        for index in 0.. {
            let cpuid = __cpuid_count(4, index);
            
            let cache_type = cpuid.eax & 0x1F;
            if cache_type == 0 {
                break;
            }
            
            let level = ((cpuid.eax >> 5) & 0x7) as u8;
            let ways = ((cpuid.ebx >> 22) & 0x3FF) + 1;
            let partitions = ((cpuid.ebx >> 12) & 0x3FF) + 1;
            let line_size = (cpuid.ebx & 0xFFF) + 1;
            let sets = cpuid.ecx + 1;
            let threads = ((cpuid.eax >> 14) & 0xFFF) + 1;
            
            let size = ways * partitions * line_size * sets;
            
            let cache_type = match cache_type {
                1 => CacheType::Data,
                2 => CacheType::Instruction,
                3 => CacheType::Unified,
                _ => continue,
            };
            
            caches.push(CacheInfo {
                level,
                cache_type,
                size,
                ways,
                line_size,
                sets,
                shared_by_threads: threads,
            });
        }
    }
    
    caches
}

pub fn get_cpu_siblings(cpu_id: u32) -> Vec<u32> {
    let smp = SMP_MANAGER.lock();
    let mut siblings = Vec::new();
    
    if let Some(cpu) = smp.get_cpu(cpu_id) {
        let target_core = cpu.core_id;
        let target_package = cpu.package_id;
        
        for other_cpu in &smp.cpus {
            if other_cpu.cpu_id != cpu_id &&
               other_cpu.package_id == target_package &&
               other_cpu.core_id == target_core {
                siblings.push(other_cpu.cpu_id);
            }
        }
    }
    
    siblings
}

pub fn get_core_siblings(cpu_id: u32) -> Vec<u32> {
    let smp = SMP_MANAGER.lock();
    let mut siblings = Vec::new();
    
    if let Some(cpu) = smp.get_cpu(cpu_id) {
        let target_package = cpu.package_id;
        
        for other_cpu in &smp.cpus {
            if other_cpu.cpu_id != cpu_id &&
               other_cpu.package_id == target_package {
                siblings.push(other_cpu.cpu_id);
            }
        }
    }
    
    siblings
}

pub fn get_package_cpus(package_id: u8) -> Vec<u32> {
    let smp = SMP_MANAGER.lock();
    smp.cpus
        .iter()
        .filter(|cpu| cpu.package_id == package_id)
        .map(|cpu| cpu.cpu_id)
        .collect()
}

pub fn get_core_cpus(package_id: u8, core_id: u8) -> Vec<u32> {
    let smp = SMP_MANAGER.lock();
    smp.cpus
        .iter()
        .filter(|cpu| cpu.package_id == package_id && cpu.core_id == core_id)
        .map(|cpu| cpu.cpu_id)
        .collect()
}