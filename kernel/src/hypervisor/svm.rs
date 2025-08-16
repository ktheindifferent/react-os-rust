use core::arch::asm;
use x86_64::registers::control::{Cr0, Cr4, Cr4Flags};
use x86_64::registers::model_specific::Msr;
use x86_64::PhysAddr;
use alloc::boxed::Box;

use super::{HypervisorCapabilities, HypervisorError};

const MSR_VM_CR: u32 = 0xC0010114;
const MSR_VM_HSAVE_PA: u32 = 0xC0010117;
const MSR_EFER: u32 = 0xC0000080;

const VMCB_SIZE: usize = 4096;

#[repr(C, align(4096))]
pub struct Vmcb {
    control_area: VmcbControlArea,
    _reserved1: [u8; 0x400 - core::mem::size_of::<VmcbControlArea>()],
    state_save_area: VmcbStateSaveArea,
    _reserved2: [u8; 0x1000 - 0x400 - core::mem::size_of::<VmcbStateSaveArea>()],
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct VmcbControlArea {
    pub intercept_cr: u32,
    pub intercept_dr: u32,
    pub intercept_exceptions: u32,
    pub intercept_instructions1: u32,
    pub intercept_instructions2: u32,
    pub _reserved1: [u8; 0x28],
    pub pause_filter_threshold: u16,
    pub pause_filter_count: u16,
    pub iopm_base_pa: u64,
    pub msrpm_base_pa: u64,
    pub tsc_offset: u64,
    pub guest_asid: u32,
    pub tlb_control: u8,
    pub _reserved2: [u8; 3],
    pub v_tpr: u8,
    pub v_irq: u8,
    pub v_intr_prio: u8,
    pub v_ign_tpr: u8,
    pub v_intr_masking: u8,
    pub v_intr_vector: u8,
    pub _reserved3: [u8; 2],
    pub interrupt_shadow: u8,
    pub _reserved4: [u8; 7],
    pub exitcode: u64,
    pub exitinfo1: u64,
    pub exitinfo2: u64,
    pub exit_int_info: u64,
    pub enable_nested_paging: u8,
    pub _reserved5: [u8; 7],
    pub event_injection: u64,
    pub n_cr3: u64,
    pub lbr_virtualization_enable: u8,
    pub _reserved6: [u8; 7],
    pub vmcb_clean: u32,
    pub _reserved7: [u8; 4],
    pub next_rip: u64,
    pub number_of_bytes_fetched: u8,
    pub guest_instruction_bytes: [u8; 15],
    pub avic_apic_bar: u64,
    pub _reserved8: [u8; 8],
    pub avic_backing_page_pointer: u64,
    pub _reserved9: [u8; 8],
    pub avic_logical_table_pointer: u64,
    pub avic_physical_table_pointer: u64,
    pub _reserved10: [u8; 8],
    pub vmcb_pointer: u64,
    pub _reserved11: [u8; 0x100 - 0xE8],
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct VmcbStateSaveArea {
    pub es: SegmentRegister,
    pub cs: SegmentRegister,
    pub ss: SegmentRegister,
    pub ds: SegmentRegister,
    pub fs: SegmentRegister,
    pub gs: SegmentRegister,
    pub gdtr: SegmentRegister,
    pub ldtr: SegmentRegister,
    pub idtr: SegmentRegister,
    pub tr: SegmentRegister,
    pub _reserved1: [u8; 0xCB - 0xA0],
    pub cpl: u8,
    pub _reserved2: [u8; 4],
    pub efer: u64,
    pub _reserved3: [u8; 0x148 - 0xD8],
    pub cr4: u64,
    pub cr3: u64,
    pub cr0: u64,
    pub dr7: u64,
    pub dr6: u64,
    pub rflags: u64,
    pub rip: u64,
    pub _reserved4: [u8; 0x1D8 - 0x180],
    pub rsp: u64,
    pub _reserved5: [u8; 0x1F8 - 0x1E0],
    pub rax: u64,
    pub star: u64,
    pub lstar: u64,
    pub cstar: u64,
    pub sfmask: u64,
    pub kernel_gs_base: u64,
    pub sysenter_cs: u64,
    pub sysenter_esp: u64,
    pub sysenter_eip: u64,
    pub cr2: u64,
    pub _reserved6: [u8; 0x268 - 0x248],
    pub pat: u64,
    pub dbgctl: u64,
    pub br_from: u64,
    pub br_to: u64,
    pub last_excp_from: u64,
    pub last_excp_to: u64,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct SegmentRegister {
    pub selector: u16,
    pub attrib: u16,
    pub limit: u32,
    pub base: u64,
}

impl Vmcb {
    pub fn new() -> Box<Self> {
        let mut vmcb = Box::new(unsafe { core::mem::zeroed::<Self>() });
        vmcb.init();
        vmcb
    }

    fn init(&mut self) {
        self.control_area.intercept_exceptions = 0xFFFFFFFF;
        
        self.control_area.intercept_instructions1 = 
            (1 << 0) |  
            (1 << 1) |  
            (1 << 2) |  
            (1 << 3) |  
            (1 << 28);  
        
        self.control_area.intercept_instructions2 = 
            (1 << 0) |  
            (1 << 1) |  
            (1 << 2) |  
            (1 << 3);   
        
        self.control_area.guest_asid = 1;
        
        self.state_save_area.efer = unsafe { Msr::new(MSR_EFER).read() };
        self.state_save_area.cr0 = Cr0::read().bits();
        self.state_save_area.cr4 = Cr4::read().bits();
        
        self.state_save_area.cs = SegmentRegister {
            selector: 0x08,
            attrib: 0x029B,
            limit: 0xFFFFFFFF,
            base: 0,
        };
        
        self.state_save_area.ds = SegmentRegister {
            selector: 0x10,
            attrib: 0x0293,
            limit: 0xFFFFFFFF,
            base: 0,
        };
        
        self.state_save_area.es = self.state_save_area.ds;
        self.state_save_area.fs = self.state_save_area.ds;
        self.state_save_area.gs = self.state_save_area.ds;
        self.state_save_area.ss = self.state_save_area.ds;
        
        self.state_save_area.gdtr = SegmentRegister {
            selector: 0,
            attrib: 0,
            limit: 0xFFFF,
            base: 0,
        };
        
        self.state_save_area.idtr = SegmentRegister {
            selector: 0,
            attrib: 0,
            limit: 0xFFFF,
            base: 0,
        };
        
        self.state_save_area.rflags = 0x2;
        self.state_save_area.rsp = 0;
        self.state_save_area.rip = 0;
    }

    pub fn run(&mut self) -> Result<SvmExitReason, HypervisorError> {
        let vmcb_phys_addr = self as *mut _ as u64;
        let mut rax = self.state_save_area.rax;
        
        unsafe {
            asm!(
                "push rbx",
                "push rcx",
                "push rdx",
                "push rsi",
                "push rdi",
                "push rbp",
                "push r8",
                "push r9",
                "push r10",
                "push r11",
                "push r12",
                "push r13",
                "push r14",
                "push r15",
                
                "vmload rax",
                "vmrun rax",
                "vmsave rax",
                
                "pop r15",
                "pop r14",
                "pop r13",
                "pop r12",
                "pop r11",
                "pop r10",
                "pop r9",
                "pop r8",
                "pop rbp",
                "pop rdi",
                "pop rsi",
                "pop rdx",
                "pop rcx",
                "pop rbx",
                
                inout("rax") vmcb_phys_addr => rax,
                options(nostack)
            );
        }
        
        self.state_save_area.rax = rax;
        
        match self.control_area.exitcode {
            0x60 => Ok(SvmExitReason::Interrupt),
            0x61 => Ok(SvmExitReason::Nmi),
            0x62 => Ok(SvmExitReason::Smi),
            0x63 => Ok(SvmExitReason::Init),
            0x64 => Ok(SvmExitReason::VirtualInterrupt),
            0x65 => Ok(SvmExitReason::Cr0Write),
            0x72 => Ok(SvmExitReason::Cpuid),
            0x73 => Ok(SvmExitReason::Hlt),
            0x7B => Ok(SvmExitReason::IoIn),
            0x7C => Ok(SvmExitReason::IoOut),
            0x7F => Ok(SvmExitReason::Msr),
            0x400 => Ok(SvmExitReason::NestedPageFault),
            _ => Ok(SvmExitReason::Unknown(self.control_area.exitcode)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SvmExitReason {
    Interrupt,
    Nmi,
    Smi,
    Init,
    VirtualInterrupt,
    Cr0Write,
    Cpuid,
    Hlt,
    IoIn,
    IoOut,
    Msr,
    NestedPageFault,
    Unknown(u64),
}

pub fn detect_svm_capabilities(mut caps: HypervisorCapabilities) -> HypervisorCapabilities {
    unsafe {
        let cpuid = core::arch::x86_64::__cpuid_count(0x8000000A, 0);
        
        if cpuid.edx & (1 << 0) != 0 {
            caps.npt_supported = true;
        }
        
        if cpuid.edx & (1 << 2) != 0 {
            caps.nested_virt = true;
        }
        
        if cpuid.edx & (1 << 13) != 0 {
            caps.apicv_supported = true;
        }
    }
    
    caps
}

pub fn enable_svm() -> Result<(), HypervisorError> {
    unsafe {
        let vm_cr = Msr::new(MSR_VM_CR).read();
        if vm_cr & (1 << 4) != 0 {
            return Err(HypervisorError::LockedByBios);
        }
        
        let mut efer = Msr::new(MSR_EFER);
        let efer_val = efer.read();
        efer.write(efer_val | (1 << 12));
        
        let hsave_pa = alloc::vec![0u8; 4096];
        let hsave_msr = Msr::new(MSR_VM_HSAVE_PA);
        hsave_msr.write(hsave_pa.as_ptr() as u64);
    }
    
    Ok(())
}

pub fn disable_svm() -> Result<(), HypervisorError> {
    unsafe {
        let mut efer = Msr::new(MSR_EFER);
        let efer_val = efer.read();
        efer.write(efer_val & !(1 << 12));
    }
    
    Ok(())
}