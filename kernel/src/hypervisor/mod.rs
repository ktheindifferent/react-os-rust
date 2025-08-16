#![allow(dead_code)]

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use x86_64::registers::control::{Cr0, Cr0Flags, Cr4, Cr4Flags};
use x86_64::registers::model_specific::Msr;
use crate::serial_println;

pub mod vmx;
pub mod svm;
pub mod vcpu;
pub mod memory;
pub mod device;
pub mod vm;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VirtualizationTechnology {
    None,
    IntelVmx,
    AmdSvm,
}

#[derive(Debug)]
pub struct HypervisorCapabilities {
    pub virt_tech: VirtualizationTechnology,
    pub ept_supported: bool,
    pub npt_supported: bool,
    pub unrestricted_guest: bool,
    pub vpid_supported: bool,
    pub vmcs_shadowing: bool,
    pub posted_interrupts: bool,
    pub apicv_supported: bool,
    pub nested_virt: bool,
}

impl Default for HypervisorCapabilities {
    fn default() -> Self {
        Self {
            virt_tech: VirtualizationTechnology::None,
            ept_supported: false,
            npt_supported: false,
            unrestricted_guest: false,
            vpid_supported: false,
            vmcs_shadowing: false,
            posted_interrupts: false,
            apicv_supported: false,
            nested_virt: false,
        }
    }
}

pub struct Hypervisor {
    capabilities: HypervisorCapabilities,
    enabled: bool,
}

impl Hypervisor {
    pub fn new() -> Self {
        let capabilities = Self::detect_capabilities();
        Self {
            capabilities,
            enabled: false,
        }
    }

    pub fn detect_capabilities() -> HypervisorCapabilities {
        let mut caps = HypervisorCapabilities::default();
        
        unsafe {
            let cpuid = core::arch::x86_64::__cpuid(1);
            
            if cpuid.ecx & (1 << 5) != 0 {
                caps.virt_tech = VirtualizationTechnology::IntelVmx;
                caps = vmx::detect_vmx_capabilities(caps);
            } else {
                let cpuid = core::arch::x86_64::__cpuid_count(0x80000001, 0);
                if cpuid.ecx & (1 << 2) != 0 {
                    caps.virt_tech = VirtualizationTechnology::AmdSvm;
                    caps = svm::detect_svm_capabilities(caps);
                }
            }
        }
        
        caps
    }

    pub fn enable(&mut self) -> Result<(), HypervisorError> {
        if self.enabled {
            return Ok(());
        }

        match self.capabilities.virt_tech {
            VirtualizationTechnology::IntelVmx => {
                vmx::enable_vmx()?;
            }
            VirtualizationTechnology::AmdSvm => {
                svm::enable_svm()?;
            }
            VirtualizationTechnology::None => {
                return Err(HypervisorError::NotSupported);
            }
        }

        self.enabled = true;
        Ok(())
    }

    pub fn disable(&mut self) -> Result<(), HypervisorError> {
        if !self.enabled {
            return Ok(());
        }

        match self.capabilities.virt_tech {
            VirtualizationTechnology::IntelVmx => {
                vmx::disable_vmx()?;
            }
            VirtualizationTechnology::AmdSvm => {
                svm::disable_svm()?;
            }
            VirtualizationTechnology::None => {}
        }

        self.enabled = false;
        Ok(())
    }

    pub fn create_vm(&self, config: VmConfig) -> Result<vm::Vm, HypervisorError> {
        if !self.enabled {
            return Err(HypervisorError::NotEnabled);
        }

        vm::Vm::new(config, &self.capabilities)
    }

    pub fn capabilities(&self) -> &HypervisorCapabilities {
        &self.capabilities
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

#[derive(Debug)]
pub enum HypervisorError {
    NotSupported,
    NotEnabled,
    AlreadyEnabled,
    VmxNotSupported,
    SvmNotSupported,
    LockedByBios,
    InvalidVmcs,
    InvalidVmcb,
    VmEntryFailed,
    VmExitUnhandled,
    MemoryAllocationFailed,
    InvalidParameter,
    VcpuNotFound,
    DeviceError,
    IoError,
}

impl fmt::Display for HypervisorError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::NotSupported => write!(f, "Virtualization not supported"),
            Self::NotEnabled => write!(f, "Hypervisor not enabled"),
            Self::AlreadyEnabled => write!(f, "Hypervisor already enabled"),
            Self::VmxNotSupported => write!(f, "Intel VT-x not supported"),
            Self::SvmNotSupported => write!(f, "AMD-V not supported"),
            Self::LockedByBios => write!(f, "Virtualization locked by BIOS"),
            Self::InvalidVmcs => write!(f, "Invalid VMCS structure"),
            Self::InvalidVmcb => write!(f, "Invalid VMCB structure"),
            Self::VmEntryFailed => write!(f, "VM entry failed"),
            Self::VmExitUnhandled => write!(f, "Unhandled VM exit"),
            Self::MemoryAllocationFailed => write!(f, "Memory allocation failed"),
            Self::InvalidParameter => write!(f, "Invalid parameter"),
            Self::VcpuNotFound => write!(f, "VCPU not found"),
            Self::DeviceError => write!(f, "Device error"),
            Self::IoError => write!(f, "I/O error"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct VmConfig {
    pub name: alloc::string::String,
    pub vcpu_count: u32,
    pub memory_mb: u64,
    pub enable_ept: bool,
    pub enable_vpid: bool,
    pub enable_unrestricted: bool,
    pub enable_apicv: bool,
}

impl Default for VmConfig {
    fn default() -> Self {
        Self {
            name: alloc::string::String::from("default"),
            vcpu_count: 1,
            memory_mb: 512,
            enable_ept: true,
            enable_vpid: true,
            enable_unrestricted: true,
            enable_apicv: false,
        }
    }
}

pub fn init() -> Result<(), HypervisorError> {
    serial_println!("Initializing hypervisor...");
    
    let mut hypervisor = Hypervisor::new();
    
    match hypervisor.capabilities().virt_tech {
        VirtualizationTechnology::IntelVmx => {
            serial_println!("Intel VT-x detected");
        }
        VirtualizationTechnology::AmdSvm => {
            serial_println!("AMD-V detected");
        }
        VirtualizationTechnology::None => {
            serial_println!("No virtualization support detected");
            return Err(HypervisorError::NotSupported);
        }
    }
    
    hypervisor.enable()?;
    serial_println!("Hypervisor enabled successfully");
    
    Ok(())
}