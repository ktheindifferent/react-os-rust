use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;
use x86_64::PhysAddr;

#[derive(Debug, Clone)]
pub struct NumaNode {
    pub node_id: u32,
    pub cpus: Vec<u32>,
    pub memory_ranges: Vec<MemoryRange>,
    pub distance_map: Vec<u8>,
    pub total_memory: u64,
    pub free_memory: u64,
}

#[derive(Debug, Clone)]
pub struct MemoryRange {
    pub base: PhysAddr,
    pub size: u64,
    pub flags: u32,
}

#[derive(Debug)]
pub struct NumaTopology {
    pub nodes: Vec<NumaNode>,
    pub distance_matrix: Vec<Vec<u8>>,
    pub node_count: usize,
}

impl NumaTopology {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            distance_matrix: Vec::new(),
            node_count: 0,
        }
    }

    pub fn add_node(&mut self, node: NumaNode) {
        self.nodes.push(node);
        self.node_count = self.nodes.len();
    }

    pub fn get_node(&self, node_id: u32) -> Option<&NumaNode> {
        self.nodes.iter().find(|n| n.node_id == node_id)
    }

    pub fn get_node_mut(&mut self, node_id: u32) -> Option<&mut NumaNode> {
        self.nodes.iter_mut().find(|n| n.node_id == node_id)
    }

    pub fn get_node_for_cpu(&self, cpu_id: u32) -> Option<u32> {
        for node in &self.nodes {
            if node.cpus.contains(&cpu_id) {
                return Some(node.node_id);
            }
        }
        None
    }

    pub fn get_distance(&self, from_node: u32, to_node: u32) -> u8 {
        if from_node as usize >= self.node_count || to_node as usize >= self.node_count {
            return 255;
        }
        
        if from_node == to_node {
            return 10;
        }
        
        if let Some(row) = self.distance_matrix.get(from_node as usize) {
            if let Some(&distance) = row.get(to_node as usize) {
                return distance;
            }
        }
        
        20
    }

    pub fn set_distance(&mut self, from_node: u32, to_node: u32, distance: u8) {
        let max_node = from_node.max(to_node) as usize + 1;
        
        while self.distance_matrix.len() < max_node {
            self.distance_matrix.push(vec![20; max_node]);
        }
        
        for row in &mut self.distance_matrix {
            while row.len() < max_node {
                row.push(20);
            }
        }
        
        self.distance_matrix[from_node as usize][to_node as usize] = distance;
    }

    pub fn get_closest_node_with_memory(&self, from_node: u32, min_size: u64) -> Option<u32> {
        let mut candidates: Vec<(u32, u8)> = Vec::new();
        
        for node in &self.nodes {
            if node.free_memory >= min_size {
                let distance = self.get_distance(from_node, node.node_id);
                candidates.push((node.node_id, distance));
            }
        }
        
        candidates.sort_by_key(|&(_, distance)| distance);
        candidates.first().map(|&(node_id, _)| node_id)
    }
}

lazy_static! {
    pub static ref NUMA_TOPOLOGY: Mutex<NumaTopology> = Mutex::new(NumaTopology::new());
}

pub fn parse_srat(table: *const u8) -> Result<(), &'static str> {
    unsafe {
        let header = table as *const AcpiSratHeader;
        let table_end = table.add((*header).length as usize);
        let mut current = table.add(core::mem::size_of::<AcpiSratHeader>());
        
        let mut topology = NUMA_TOPOLOGY.lock();
        
        while current < table_end {
            let entry_type = *current;
            let entry_length = *(current.add(1));
            
            match entry_type {
                0 => {
                    let lapic_affinity = current as *const SratProcessorAffinity;
                    if (*lapic_affinity).flags & 1 != 0 {
                        let node_id = (*lapic_affinity).proximity_domain_low as u32;
                        let apic_id = (*lapic_affinity).apic_id;
                        
                        add_cpu_to_node(&mut topology, node_id, apic_id);
                    }
                }
                1 => {
                    let mem_affinity = current as *const SratMemoryAffinity;
                    if (*mem_affinity).flags & 1 != 0 {
                        let node_id = (*mem_affinity).proximity_domain;
                        let base = PhysAddr::new((*mem_affinity).base_address);
                        let size = (*mem_affinity).length;
                        
                        add_memory_to_node(&mut topology, node_id, base, size);
                    }
                }
                2 => {
                    let x2apic_affinity = current as *const SratX2ApicAffinity;
                    if (*x2apic_affinity).flags & 1 != 0 {
                        let node_id = (*x2apic_affinity).proximity_domain;
                        let apic_id = (*x2apic_affinity).x2apic_id as u8;
                        
                        add_cpu_to_node(&mut topology, node_id, apic_id);
                    }
                }
                _ => {}
            }
            
            current = current.add(entry_length as usize);
        }
    }
    
    Ok(())
}

pub fn parse_slit(table: *const u8) -> Result<(), &'static str> {
    unsafe {
        let header = table as *const AcpiSlitHeader;
        let locality_count = (*header).locality_count as usize;
        let distances = table.add(core::mem::size_of::<AcpiSlitHeader>());
        
        let mut topology = NUMA_TOPOLOGY.lock();
        
        for i in 0..locality_count {
            for j in 0..locality_count {
                let distance = *(distances.add(i * locality_count + j));
                topology.set_distance(i as u32, j as u32, distance);
            }
        }
    }
    
    Ok(())
}

fn add_cpu_to_node(topology: &mut NumaTopology, node_id: u32, apic_id: u8) {
    let smp = super::SMP_MANAGER.lock();
    if let Some(cpu) = smp.get_cpu_by_apic_id(apic_id) {
        let cpu_id = cpu.cpu_id;
        drop(smp);
        
        if let Some(node) = topology.get_node_mut(node_id) {
            if !node.cpus.contains(&cpu_id) {
                node.cpus.push(cpu_id);
            }
        } else {
            let mut node = NumaNode {
                node_id,
                cpus: vec![cpu_id],
                memory_ranges: Vec::new(),
                distance_map: Vec::new(),
                total_memory: 0,
                free_memory: 0,
            };
            topology.add_node(node);
        }
    }
}

fn add_memory_to_node(topology: &mut NumaTopology, node_id: u32, base: PhysAddr, size: u64) {
    let memory_range = MemoryRange {
        base,
        size,
        flags: 0,
    };
    
    if let Some(node) = topology.get_node_mut(node_id) {
        node.memory_ranges.push(memory_range);
        node.total_memory += size;
        node.free_memory += size;
    } else {
        let node = NumaNode {
            node_id,
            cpus: Vec::new(),
            memory_ranges: vec![memory_range],
            distance_map: Vec::new(),
            total_memory: size,
            free_memory: size,
        };
        topology.add_node(node);
    }
}

#[repr(C, packed)]
struct AcpiSratHeader {
    signature: [u8; 4],
    length: u32,
    revision: u8,
    checksum: u8,
    oem_id: [u8; 6],
    oem_table_id: [u8; 8],
    oem_revision: u32,
    creator_id: u32,
    creator_revision: u32,
    table_revision: u32,
    reserved: u64,
}

#[repr(C, packed)]
struct SratProcessorAffinity {
    entry_type: u8,
    length: u8,
    proximity_domain_low: u8,
    apic_id: u8,
    flags: u32,
    sapic_eid: u8,
    proximity_domain_high: [u8; 3],
    clock_domain: u32,
}

#[repr(C, packed)]
struct SratMemoryAffinity {
    entry_type: u8,
    length: u8,
    proximity_domain: u32,
    reserved1: u16,
    base_address: u64,
    length: u64,
    reserved2: u32,
    flags: u32,
    reserved3: u64,
}

#[repr(C, packed)]
struct SratX2ApicAffinity {
    entry_type: u8,
    length: u8,
    reserved1: u16,
    proximity_domain: u32,
    x2apic_id: u32,
    flags: u32,
    clock_domain: u32,
    reserved2: u32,
}

#[repr(C, packed)]
struct AcpiSlitHeader {
    signature: [u8; 4],
    length: u32,
    revision: u8,
    checksum: u8,
    oem_id: [u8; 6],
    oem_table_id: [u8; 8],
    oem_revision: u32,
    creator_id: u32,
    creator_revision: u32,
    locality_count: u64,
}

pub fn get_node_for_cpu(cpu_id: u32) -> u32 {
    NUMA_TOPOLOGY.lock()
        .get_node_for_cpu(cpu_id)
        .unwrap_or(0)
}

pub fn get_current_node() -> u32 {
    get_node_for_cpu(super::current_cpu_id())
}

pub fn allocate_on_node(node_id: u32, size: u64) -> Option<PhysAddr> {
    let mut topology = NUMA_TOPOLOGY.lock();
    
    if let Some(node) = topology.get_node_mut(node_id) {
        if node.free_memory >= size {
            node.free_memory -= size;
            
            for range in &node.memory_ranges {
                if range.size >= size {
                    return Some(range.base);
                }
            }
        }
    }
    
    if let Some(closest_node) = topology.get_closest_node_with_memory(node_id, size) {
        if let Some(node) = topology.get_node_mut(closest_node) {
            node.free_memory -= size;
            for range in &node.memory_ranges {
                if range.size >= size {
                    return Some(range.base);
                }
            }
        }
    }
    
    None
}

pub fn print_numa_topology() {
    let topology = NUMA_TOPOLOGY.lock();
    
    crate::println!("NUMA Topology:");
    crate::println!("  Nodes: {}", topology.node_count);
    
    for node in &topology.nodes {
        crate::println!("  Node {}:", node.node_id);
        crate::println!("    CPUs: {:?}", node.cpus);
        crate::println!("    Memory: {} MB", node.total_memory / (1024 * 1024));
        crate::println!("    Free: {} MB", node.free_memory / (1024 * 1024));
    }
    
    if topology.node_count > 1 {
        crate::println!("  Distance Matrix:");
        for i in 0..topology.node_count {
            let mut distances = Vec::new();
            for j in 0..topology.node_count {
                distances.push(topology.get_distance(i as u32, j as u32));
            }
            crate::println!("    Node {}: {:?}", i, distances);
        }
    }
}