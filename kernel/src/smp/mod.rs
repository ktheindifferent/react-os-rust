pub mod ap_boot;
pub mod percpu;
pub mod ipi;
pub mod topology;
pub mod numa;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use spin::{Mutex, Once};
use lazy_static::lazy_static;
use crate::acpi::apic::{LocalApicInfo, ApicInfo};

pub const MAX_CPUS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuState {
    Offline,
    Booting,
    Online,
    Halted,
}

#[derive(Debug, Clone)]
pub struct CpuInfo {
    pub cpu_id: u32,
    pub apic_id: u8,
    pub package_id: u8,
    pub core_id: u8,
    pub thread_id: u8,
    pub numa_node: u8,
    pub state: CpuState,
    pub is_bsp: bool,
}

impl CpuInfo {
    pub fn new(cpu_id: u32, apic_id: u8, is_bsp: bool) -> Self {
        Self {
            cpu_id,
            apic_id,
            package_id: 0,
            core_id: 0,
            thread_id: 0,
            numa_node: 0,
            state: if is_bsp { CpuState::Online } else { CpuState::Offline },
            is_bsp,
        }
    }
}

pub struct SmpManager {
    cpus: Vec<CpuInfo>,
    online_count: AtomicU32,
    boot_complete: AtomicBool,
}

impl SmpManager {
    pub fn new() -> Self {
        Self {
            cpus: Vec::new(),
            online_count: AtomicU32::new(1),
            boot_complete: AtomicBool::new(false),
        }
    }

    pub fn register_cpu(&mut self, cpu_info: CpuInfo) {
        self.cpus.push(cpu_info);
    }

    pub fn get_cpu(&self, cpu_id: u32) -> Option<&CpuInfo> {
        self.cpus.iter().find(|cpu| cpu.cpu_id == cpu_id)
    }

    pub fn get_cpu_mut(&mut self, cpu_id: u32) -> Option<&mut CpuInfo> {
        self.cpus.iter_mut().find(|cpu| cpu.cpu_id == cpu_id)
    }

    pub fn get_cpu_by_apic_id(&self, apic_id: u8) -> Option<&CpuInfo> {
        self.cpus.iter().find(|cpu| cpu.apic_id == apic_id)
    }

    pub fn cpu_count(&self) -> usize {
        self.cpus.len()
    }

    pub fn online_cpu_count(&self) -> u32 {
        self.online_count.load(Ordering::Acquire)
    }

    pub fn mark_cpu_online(&self, cpu_id: u32) {
        self.online_count.fetch_add(1, Ordering::Release);
    }

    pub fn mark_cpu_offline(&self, cpu_id: u32) {
        self.online_count.fetch_sub(1, Ordering::Release);
    }

    pub fn is_boot_complete(&self) -> bool {
        self.boot_complete.load(Ordering::Acquire)
    }

    pub fn mark_boot_complete(&self) {
        self.boot_complete.store(true, Ordering::Release);
    }

    pub fn get_online_cpus(&self) -> Vec<u32> {
        self.cpus
            .iter()
            .filter(|cpu| cpu.state == CpuState::Online)
            .map(|cpu| cpu.cpu_id)
            .collect()
    }

    pub fn get_bsp(&self) -> Option<&CpuInfo> {
        self.cpus.iter().find(|cpu| cpu.is_bsp)
    }
}

lazy_static! {
    pub static ref SMP_MANAGER: Mutex<SmpManager> = Mutex::new(SmpManager::new());
}

static BSP_INIT: Once = Once::new();

pub fn init_bsp() {
    BSP_INIT.call_once(|| {
        use crate::cpu::get_cpu_id;
        
        let mut smp = SMP_MANAGER.lock();
        let bsp_info = CpuInfo::new(0, 0, true);
        smp.register_cpu(bsp_info);
        
        topology::detect_topology();
        
        crate::serial_println!("SMP: BSP initialized (CPU 0)");
    });
}

pub fn init_ap_cpus(apic_info: &ApicInfo) -> Result<(), &'static str> {
    let mut smp = SMP_MANAGER.lock();
    
    let mut cpu_id = 1;
    for lapic in &apic_info.local_apics {
        if !lapic.enabled {
            continue;
        }
        
        if lapic.apic_id == 0 {
            continue;
        }
        
        let cpu_info = CpuInfo::new(cpu_id, lapic.apic_id, false);
        smp.register_cpu(cpu_info);
        cpu_id += 1;
    }
    
    let total_cpus = smp.cpu_count();
    drop(smp);
    
    crate::serial_println!("SMP: Found {} CPUs total", total_cpus);
    
    if total_cpus > 1 {
        ap_boot::start_application_processors()?;
    }
    
    Ok(())
}

pub fn current_cpu_id() -> u32 {
    percpu::get_cpu_id()
}

pub fn cpu_online(cpu_id: u32) -> bool {
    let smp = SMP_MANAGER.lock();
    smp.get_cpu(cpu_id)
        .map(|cpu| cpu.state == CpuState::Online)
        .unwrap_or(false)
}

pub fn send_ipi_to_cpu(target_cpu: u32, vector: u8) {
    let smp = SMP_MANAGER.lock();
    if let Some(cpu) = smp.get_cpu(target_cpu) {
        ipi::send_ipi(cpu.apic_id, vector);
    }
}

pub fn send_ipi_to_all(vector: u8, exclude_self: bool) {
    let current = current_cpu_id();
    let smp = SMP_MANAGER.lock();
    
    for cpu in &smp.cpus {
        if cpu.state != CpuState::Online {
            continue;
        }
        
        if exclude_self && cpu.cpu_id == current {
            continue;
        }
        
        ipi::send_ipi(cpu.apic_id, vector);
    }
}

pub fn send_ipi_to_others(vector: u8) {
    send_ipi_to_all(vector, true);
}

pub fn yield_cpu() {
    unsafe {
        core::arch::x86_64::_mm_pause();
    }
}

pub fn cpu_relax() {
    for _ in 0..100 {
        yield_cpu();
    }
}