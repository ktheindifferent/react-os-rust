use alloc::boxed::Box;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, AtomicBool, Ordering};
use core::arch::asm;
use x86_64::structures::idt::InterruptStackFrame;

use super::{HypervisorError, VirtualizationTechnology};
use super::vmx::{Vmcs, VmcsField, vmread, vmwrite};
use super::svm::{Vmcb, SvmExitReason};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VcpuState {
    Created,
    Ready,
    Running,
    Blocked,
    Halted,
    Terminated,
}

#[derive(Debug, Clone, Copy)]
pub struct VcpuRegisters {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub rsp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rip: u64,
    pub rflags: u64,
    pub cr0: u64,
    pub cr2: u64,
    pub cr3: u64,
    pub cr4: u64,
    pub cr8: u64,
    pub dr0: u64,
    pub dr1: u64,
    pub dr2: u64,
    pub dr3: u64,
    pub dr6: u64,
    pub dr7: u64,
}

impl Default for VcpuRegisters {
    fn default() -> Self {
        Self {
            rax: 0,
            rbx: 0,
            rcx: 0,
            rdx: 0,
            rsi: 0,
            rdi: 0,
            rbp: 0,
            rsp: 0,
            r8: 0,
            r9: 0,
            r10: 0,
            r11: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            rip: 0,
            rflags: 0x2,
            cr0: 0x60000010,
            cr2: 0,
            cr3: 0,
            cr4: 0x2000,
            cr8: 0,
            dr0: 0,
            dr1: 0,
            dr2: 0,
            dr3: 0,
            dr6: 0xFFFF0FF0,
            dr7: 0x400,
        }
    }
}

pub struct Vcpu {
    id: u32,
    state: VcpuState,
    registers: VcpuRegisters,
    virt_tech: VirtualizationTechnology,
    vmcs: Option<Box<Vmcs>>,
    vmcb: Option<Box<Vmcb>>,
    vpid: u16,
    asid: u32,
    tsc_offset: i64,
    pending_interrupts: Vec<u8>,
    interrupt_window_open: AtomicBool,
    nmi_pending: AtomicBool,
}

impl Vcpu {
    pub fn new(id: u32, virt_tech: VirtualizationTechnology) -> Result<Self, HypervisorError> {
        let mut vcpu = Self {
            id,
            state: VcpuState::Created,
            registers: VcpuRegisters::default(),
            virt_tech,
            vmcs: None,
            vmcb: None,
            vpid: (id + 1) as u16,
            asid: id + 1,
            tsc_offset: 0,
            pending_interrupts: Vec::new(),
            interrupt_window_open: AtomicBool::new(false),
            nmi_pending: AtomicBool::new(false),
        };

        match virt_tech {
            VirtualizationTechnology::IntelVmx => {
                vcpu.vmcs = Some(Vmcs::new());
                vcpu.init_vmcs()?;
            }
            VirtualizationTechnology::AmdSvm => {
                vcpu.vmcb = Some(Vmcb::new());
                vcpu.init_vmcb()?;
            }
            VirtualizationTechnology::None => {
                return Err(HypervisorError::NotSupported);
            }
        }

        vcpu.state = VcpuState::Ready;
        Ok(vcpu)
    }

    fn init_vmcs(&mut self) -> Result<(), HypervisorError> {
        if let Some(ref mut vmcs) = self.vmcs {
            vmcs.clear()?;
            vmcs.load()?;

            vmwrite(VmcsField::VirtualProcessorId, self.vpid as u64)?;
            
            vmwrite(VmcsField::PinBasedVmExecControl, 0x16)?;
            vmwrite(VmcsField::CpuBasedVmExecControl, 0x8401E9F2)?;
            vmwrite(VmcsField::SecondaryVmExecControl, 0x22)?;
            vmwrite(VmcsField::VmExitControls, 0x336FFF)?;
            vmwrite(VmcsField::VmEntryControls, 0x93FF)?;
            
            vmwrite(VmcsField::GuestCr0, self.registers.cr0)?;
            vmwrite(VmcsField::GuestCr3, self.registers.cr3)?;
            vmwrite(VmcsField::GuestCr4, self.registers.cr4)?;
            
            vmwrite(VmcsField::GuestCsSelector, 0x8)?;
            vmwrite(VmcsField::GuestCsBase, 0)?;
            vmwrite(VmcsField::GuestCsLimit, 0xFFFFFFFF)?;
            vmwrite(VmcsField::GuestCsArBytes, 0xC09B)?;
            
            vmwrite(VmcsField::GuestDsSelector, 0x10)?;
            vmwrite(VmcsField::GuestDsBase, 0)?;
            vmwrite(VmcsField::GuestDsLimit, 0xFFFFFFFF)?;
            vmwrite(VmcsField::GuestDsArBytes, 0xC093)?;
            
            vmwrite(VmcsField::GuestEsSelector, 0x10)?;
            vmwrite(VmcsField::GuestSsSelector, 0x10)?;
            vmwrite(VmcsField::GuestFsSelector, 0)?;
            vmwrite(VmcsField::GuestGsSelector, 0)?;
            
            vmwrite(VmcsField::GuestRsp, self.registers.rsp)?;
            vmwrite(VmcsField::GuestRip, self.registers.rip)?;
            vmwrite(VmcsField::GuestRflags, self.registers.rflags)?;
            
            vmwrite(VmcsField::VmcsLinkPointer, 0xFFFFFFFFFFFFFFFF)?;
            
            Ok(())
        } else {
            Err(HypervisorError::InvalidVmcs)
        }
    }

    fn init_vmcb(&mut self) -> Result<(), HypervisorError> {
        if let Some(ref mut vmcb) = self.vmcb {
            vmcb.control_area.guest_asid = self.asid;
            vmcb.control_area.tsc_offset = self.tsc_offset as u64;
            
            vmcb.state_save_area.cr0 = self.registers.cr0;
            vmcb.state_save_area.cr3 = self.registers.cr3;
            vmcb.state_save_area.cr4 = self.registers.cr4;
            
            vmcb.state_save_area.rsp = self.registers.rsp;
            vmcb.state_save_area.rip = self.registers.rip;
            vmcb.state_save_area.rflags = self.registers.rflags;
            vmcb.state_save_area.rax = self.registers.rax;
            
            Ok(())
        } else {
            Err(HypervisorError::InvalidVmcb)
        }
    }

    pub fn run(&mut self) -> Result<VmExitReason, HypervisorError> {
        self.state = VcpuState::Running;
        
        match self.virt_tech {
            VirtualizationTechnology::IntelVmx => self.run_vmx(),
            VirtualizationTechnology::AmdSvm => self.run_svm(),
            VirtualizationTechnology::None => Err(HypervisorError::NotSupported),
        }
    }

    fn run_vmx(&mut self) -> Result<VmExitReason, HypervisorError> {
        if let Some(ref mut vmcs) = self.vmcs {
            vmcs.load()?;
            
            self.inject_pending_interrupts_vmx()?;
            
            unsafe {
                asm!(
                    "vmlaunch",
                    options(nostack)
                );
            }
            
            let exit_reason = vmread(VmcsField::VmExitReason)?;
            let exit_qualification = vmread(VmcsField::ExitQualification)?;
            
            self.registers.rip = vmread(VmcsField::GuestRip)?;
            self.registers.rsp = vmread(VmcsField::GuestRsp)?;
            self.registers.rflags = vmread(VmcsField::GuestRflags)?;
            
            Ok(self.decode_vmx_exit(exit_reason, exit_qualification))
        } else {
            Err(HypervisorError::InvalidVmcs)
        }
    }

    fn run_svm(&mut self) -> Result<VmExitReason, HypervisorError> {
        if let Some(ref mut vmcb) = self.vmcb {
            self.inject_pending_interrupts_svm()?;
            
            let exit_reason = vmcb.run()?;
            
            self.registers.rip = vmcb.state_save_area.rip;
            self.registers.rsp = vmcb.state_save_area.rsp;
            self.registers.rflags = vmcb.state_save_area.rflags;
            self.registers.rax = vmcb.state_save_area.rax;
            
            Ok(self.decode_svm_exit(exit_reason))
        } else {
            Err(HypervisorError::InvalidVmcb)
        }
    }

    fn inject_pending_interrupts_vmx(&mut self) -> Result<(), HypervisorError> {
        if self.nmi_pending.load(Ordering::Acquire) {
            vmwrite(VmcsField::VmEntryIntrInfoField, 0x80000202)?;
            self.nmi_pending.store(false, Ordering::Release);
        } else if !self.pending_interrupts.is_empty() && 
                  self.interrupt_window_open.load(Ordering::Acquire) {
            let vector = self.pending_interrupts.remove(0);
            let intr_info = 0x80000000 | (vector as u64);
            vmwrite(VmcsField::VmEntryIntrInfoField, intr_info)?;
        }
        Ok(())
    }

    fn inject_pending_interrupts_svm(&mut self) -> Result<(), HypervisorError> {
        if let Some(ref mut vmcb) = self.vmcb {
            if self.nmi_pending.load(Ordering::Acquire) {
                vmcb.control_area.event_injection = 0x80000002;
                self.nmi_pending.store(false, Ordering::Release);
            } else if !self.pending_interrupts.is_empty() && 
                      self.interrupt_window_open.load(Ordering::Acquire) {
                let vector = self.pending_interrupts.remove(0);
                vmcb.control_area.event_injection = 0x80000000 | (vector as u64);
            }
        }
        Ok(())
    }

    fn decode_vmx_exit(&self, reason: u64, qualification: u64) -> VmExitReason {
        match reason & 0xFFFF {
            0 => VmExitReason::ExceptionOrNmi,
            1 => VmExitReason::ExternalInterrupt,
            2 => VmExitReason::TripleFault,
            3 => VmExitReason::Init,
            7 => VmExitReason::InterruptWindow,
            9 => VmExitReason::TaskSwitch,
            10 => VmExitReason::Cpuid,
            12 => VmExitReason::Hlt,
            14 => VmExitReason::Invlpg,
            15 => VmExitReason::Rdpmc,
            16 => VmExitReason::Rdtsc,
            18 => VmExitReason::Vmcall,
            28 => VmExitReason::CrAccess(qualification),
            30 => VmExitReason::IoInstruction(qualification),
            31 => VmExitReason::Rdmsr,
            32 => VmExitReason::Wrmsr,
            48 => VmExitReason::EptViolation,
            49 => VmExitReason::EptMisconfig,
            _ => VmExitReason::Unknown(reason),
        }
    }

    fn decode_svm_exit(&self, reason: SvmExitReason) -> VmExitReason {
        match reason {
            SvmExitReason::Interrupt => VmExitReason::ExternalInterrupt,
            SvmExitReason::Nmi => VmExitReason::ExceptionOrNmi,
            SvmExitReason::Cpuid => VmExitReason::Cpuid,
            SvmExitReason::Hlt => VmExitReason::Hlt,
            SvmExitReason::IoIn | SvmExitReason::IoOut => VmExitReason::IoInstruction(0),
            SvmExitReason::Msr => VmExitReason::Rdmsr,
            SvmExitReason::NestedPageFault => VmExitReason::EptViolation,
            _ => VmExitReason::Unknown(0),
        }
    }

    pub fn inject_interrupt(&mut self, vector: u8) {
        self.pending_interrupts.push(vector);
    }

    pub fn inject_nmi(&mut self) {
        self.nmi_pending.store(true, Ordering::Release);
    }

    pub fn get_registers(&self) -> &VcpuRegisters {
        &self.registers
    }

    pub fn set_registers(&mut self, registers: VcpuRegisters) {
        self.registers = registers;
    }

    pub fn get_state(&self) -> VcpuState {
        self.state
    }

    pub fn halt(&mut self) {
        self.state = VcpuState::Halted;
    }

    pub fn resume(&mut self) {
        if self.state == VcpuState::Halted {
            self.state = VcpuState::Ready;
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum VmExitReason {
    ExceptionOrNmi,
    ExternalInterrupt,
    TripleFault,
    Init,
    InterruptWindow,
    TaskSwitch,
    Cpuid,
    Hlt,
    Invlpg,
    Rdpmc,
    Rdtsc,
    Vmcall,
    CrAccess(u64),
    IoInstruction(u64),
    Rdmsr,
    Wrmsr,
    EptViolation,
    EptMisconfig,
    Unknown(u64),
}