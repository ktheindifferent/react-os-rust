use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use spin::Mutex;
use crate::serial_println;

use super::{HypervisorCapabilities, HypervisorError, VmConfig, VirtualizationTechnology};
use super::vcpu::{Vcpu, VcpuState, VmExitReason};
use super::memory::{GuestMemory, EptManager, NptManager};
use super::device::{VirtualDevice, DeviceManager};

static VM_ID_COUNTER: AtomicU32 = AtomicU32::new(1);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VmState {
    Created,
    Running,
    Paused,
    Suspended,
    Shutdown,
}

pub struct Vm {
    id: u32,
    name: String,
    state: Mutex<VmState>,
    vcpus: Vec<Mutex<Vcpu>>,
    memory: GuestMemory,
    ept_manager: Option<EptManager>,
    npt_manager: Option<NptManager>,
    device_manager: DeviceManager,
    stats: VmStats,
    config: VmConfig,
}

#[derive(Debug)]
pub struct VmStats {
    total_exits: AtomicU64,
    io_exits: AtomicU64,
    mmio_exits: AtomicU64,
    interrupt_injections: AtomicU64,
    page_faults: AtomicU64,
}

impl Vm {
    pub fn new(config: VmConfig, capabilities: &HypervisorCapabilities) -> Result<Self, HypervisorError> {
        let id = VM_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
        
        let memory = GuestMemory::new(config.memory_mb * 1024 * 1024)?;
        
        let mut vcpus = Vec::new();
        for vcpu_id in 0..config.vcpu_count {
            let vcpu = Vcpu::new(vcpu_id, capabilities.virt_tech)?;
            vcpus.push(Mutex::new(vcpu));
        }
        
        let ept_manager = if capabilities.ept_supported && config.enable_ept {
            Some(EptManager::new()?)
        } else {
            None
        };
        
        let npt_manager = if capabilities.npt_supported && config.enable_ept {
            Some(NptManager::new()?)
        } else {
            None
        };
        
        let device_manager = DeviceManager::new();
        
        let stats = VmStats {
            total_exits: AtomicU64::new(0),
            io_exits: AtomicU64::new(0),
            mmio_exits: AtomicU64::new(0),
            interrupt_injections: AtomicU64::new(0),
            page_faults: AtomicU64::new(0),
        };
        
        Ok(Self {
            id,
            name: config.name.clone(),
            state: Mutex::new(VmState::Created),
            vcpus,
            memory,
            ept_manager,
            npt_manager,
            device_manager,
            stats,
            config,
        })
    }
    
    pub fn start(&self) -> Result<(), HypervisorError> {
        let mut state = self.state.lock();
        if *state != VmState::Created && *state != VmState::Paused {
            return Err(HypervisorError::InvalidParameter);
        }
        
        *state = VmState::Running;
        
        for vcpu_mutex in &self.vcpus {
            let mut vcpu = vcpu_mutex.lock();
            vcpu.resume();
        }
        
        Ok(())
    }
    
    pub fn pause(&self) -> Result<(), HypervisorError> {
        let mut state = self.state.lock();
        if *state != VmState::Running {
            return Err(HypervisorError::InvalidParameter);
        }
        
        *state = VmState::Paused;
        
        for vcpu_mutex in &self.vcpus {
            let mut vcpu = vcpu_mutex.lock();
            vcpu.halt();
        }
        
        Ok(())
    }
    
    pub fn resume(&self) -> Result<(), HypervisorError> {
        let mut state = self.state.lock();
        if *state != VmState::Paused {
            return Err(HypervisorError::InvalidParameter);
        }
        
        *state = VmState::Running;
        
        for vcpu_mutex in &self.vcpus {
            let mut vcpu = vcpu_mutex.lock();
            vcpu.resume();
        }
        
        Ok(())
    }
    
    pub fn shutdown(&self) -> Result<(), HypervisorError> {
        let mut state = self.state.lock();
        *state = VmState::Shutdown;
        
        for vcpu_mutex in &self.vcpus {
            let mut vcpu = vcpu_mutex.lock();
            vcpu.halt();
        }
        
        Ok(())
    }
    
    pub fn run_vcpu(&self, vcpu_id: u32) -> Result<(), HypervisorError> {
        if vcpu_id >= self.vcpus.len() as u32 {
            return Err(HypervisorError::VcpuNotFound);
        }
        
        let vcpu_mutex = &self.vcpus[vcpu_id as usize];
        
        loop {
            let state = self.state.lock();
            if *state != VmState::Running {
                break;
            }
            drop(state);
            
            let mut vcpu = vcpu_mutex.lock();
            let exit_reason = vcpu.run()?;
            drop(vcpu);
            
            self.stats.total_exits.fetch_add(1, Ordering::Relaxed);
            
            self.handle_vm_exit(vcpu_id, exit_reason)?;
        }
        
        Ok(())
    }
    
    fn handle_vm_exit(&self, vcpu_id: u32, exit_reason: VmExitReason) -> Result<(), HypervisorError> {
        match exit_reason {
            VmExitReason::IoInstruction(qualification) => {
                self.stats.io_exits.fetch_add(1, Ordering::Relaxed);
                self.handle_io_exit(vcpu_id, qualification)?;
            }
            VmExitReason::Cpuid => {
                self.handle_cpuid(vcpu_id)?;
            }
            VmExitReason::Rdmsr => {
                self.handle_rdmsr(vcpu_id)?;
            }
            VmExitReason::Wrmsr => {
                self.handle_wrmsr(vcpu_id)?;
            }
            VmExitReason::EptViolation => {
                self.stats.page_faults.fetch_add(1, Ordering::Relaxed);
                self.handle_ept_violation(vcpu_id)?;
            }
            VmExitReason::ExternalInterrupt => {
                self.handle_external_interrupt(vcpu_id)?;
            }
            VmExitReason::Hlt => {
                self.handle_hlt(vcpu_id)?;
            }
            VmExitReason::Vmcall => {
                self.handle_hypercall(vcpu_id)?;
            }
            _ => {
                serial_println!("Unhandled VM exit: {:?}", exit_reason);
            }
        }
        
        Ok(())
    }
    
    fn handle_io_exit(&self, vcpu_id: u32, qualification: u64) -> Result<(), HypervisorError> {
        let is_input = (qualification & 0x8) != 0;
        let port = ((qualification >> 16) & 0xFFFF) as u16;
        let size = ((qualification & 0x7) + 1) as u32;
        
        let vcpu = self.vcpus[vcpu_id as usize].lock();
        let registers = vcpu.get_registers();
        
        if is_input {
            let value = self.device_manager.io_read(port, size)?;
            
        } else {
            let value = registers.rax & ((1u64 << (size * 8)) - 1);
            self.device_manager.io_write(port, value as u32, size)?;
        }
        
        Ok(())
    }
    
    fn handle_cpuid(&self, vcpu_id: u32) -> Result<(), HypervisorError> {
        let mut vcpu = self.vcpus[vcpu_id as usize].lock();
        let mut registers = *vcpu.get_registers();
        
        let leaf = registers.rax as u32;
        let subleaf = registers.rcx as u32;
        
        unsafe {
            let result = core::arch::x86_64::__cpuid_count(leaf, subleaf);
            registers.rax = result.eax as u64;
            registers.rbx = result.ebx as u64;
            registers.rcx = result.ecx as u64;
            registers.rdx = result.edx as u64;
        }
        
        if leaf == 1 {
            registers.rcx &= !(1 << 5);
            registers.rcx &= !(1 << 3);
        }
        
        vcpu.set_registers(registers);
        Ok(())
    }
    
    fn handle_rdmsr(&self, vcpu_id: u32) -> Result<(), HypervisorError> {
        let mut vcpu = self.vcpus[vcpu_id as usize].lock();
        let mut registers = *vcpu.get_registers();
        
        let msr = registers.rcx as u32;
        
        let value = match msr {
            0x174..=0x176 => 0,
            0xC0000080 => 0x500,
            _ => 0,
        };
        
        registers.rax = value & 0xFFFFFFFF;
        registers.rdx = value >> 32;
        
        vcpu.set_registers(registers);
        Ok(())
    }
    
    fn handle_wrmsr(&self, vcpu_id: u32) -> Result<(), HypervisorError> {
        let vcpu = self.vcpus[vcpu_id as usize].lock();
        let registers = vcpu.get_registers();
        
        let msr = registers.rcx as u32;
        let value = registers.rax | (registers.rdx << 32);
        
        serial_println!("WRMSR: msr={:#x}, value={:#x}", msr, value);
        
        Ok(())
    }
    
    fn handle_ept_violation(&self, vcpu_id: u32) -> Result<(), HypervisorError> {
        if let Some(ref ept_manager) = self.ept_manager {
            ept_manager.handle_violation()?;
        } else if let Some(ref npt_manager) = self.npt_manager {
            npt_manager.handle_violation()?;
        }
        Ok(())
    }
    
    fn handle_external_interrupt(&self, _vcpu_id: u32) -> Result<(), HypervisorError> {
        Ok(())
    }
    
    fn handle_hlt(&self, vcpu_id: u32) -> Result<(), HypervisorError> {
        let mut vcpu = self.vcpus[vcpu_id as usize].lock();
        vcpu.halt();
        Ok(())
    }
    
    fn handle_hypercall(&self, vcpu_id: u32) -> Result<(), HypervisorError> {
        let vcpu = self.vcpus[vcpu_id as usize].lock();
        let registers = vcpu.get_registers();
        
        let call_nr = registers.rax;
        
        match call_nr {
            0x1000 => {
                serial_println!("Hypercall: Get VM info");
            }
            0x1001 => {
                serial_println!("Hypercall: Map memory");
            }
            _ => {
                serial_println!("Unknown hypercall: {:#x}", call_nr);
            }
        }
        
        Ok(())
    }
    
    pub fn add_device(&mut self, device: Box<dyn VirtualDevice>) -> Result<(), HypervisorError> {
        self.device_manager.add_device(device)
    }
    
    pub fn get_memory(&self) -> &GuestMemory {
        &self.memory
    }
    
    pub fn get_memory_mut(&mut self) -> &mut GuestMemory {
        &mut self.memory
    }
    
    pub fn get_stats(&self) -> &VmStats {
        &self.stats
    }
    
    pub fn get_id(&self) -> u32 {
        self.id
    }
    
    pub fn get_name(&self) -> &str {
        &self.name
    }
}