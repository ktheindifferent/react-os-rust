use core::arch::asm;
use x86_64::registers::control::{Cr0, Cr0Flags, Cr4, Cr4Flags};
use x86_64::registers::model_specific::Msr;
use x86_64::PhysAddr;
use alloc::vec::Vec;
use alloc::boxed::Box;

use super::{HypervisorCapabilities, HypervisorError};

const IA32_VMX_BASIC_MSR: u32 = 0x480;
const IA32_VMX_PINBASED_CTLS_MSR: u32 = 0x481;
const IA32_VMX_PROCBASED_CTLS_MSR: u32 = 0x482;
const IA32_VMX_EXIT_CTLS_MSR: u32 = 0x483;
const IA32_VMX_ENTRY_CTLS_MSR: u32 = 0x484;
const IA32_VMX_MISC_MSR: u32 = 0x485;
const IA32_VMX_CR0_FIXED0_MSR: u32 = 0x486;
const IA32_VMX_CR0_FIXED1_MSR: u32 = 0x487;
const IA32_VMX_CR4_FIXED0_MSR: u32 = 0x488;
const IA32_VMX_CR4_FIXED1_MSR: u32 = 0x489;
const IA32_VMX_PROCBASED_CTLS2_MSR: u32 = 0x48B;
const IA32_VMX_EPT_VPID_CAP_MSR: u32 = 0x48C;
const IA32_FEATURE_CONTROL_MSR: u32 = 0x3A;

const VMCS_SIZE: usize = 4096;

#[repr(C, align(4096))]
pub struct Vmcs {
    data: [u8; VMCS_SIZE],
}

impl Vmcs {
    pub fn new() -> Box<Self> {
        Box::new(Self {
            data: [0; VMCS_SIZE],
        })
    }

    pub fn clear(&mut self) -> Result<(), HypervisorError> {
        let phys_addr = self as *mut _ as u64;
        let result: u64;
        
        unsafe {
            asm!(
                "vmclear [{0}]",
                "pushfq",
                "pop {1}",
                in(reg) &phys_addr,
                out(reg) result,
                options(nostack)
            );
        }
        
        if result & 0x41 != 0 {
            return Err(HypervisorError::InvalidVmcs);
        }
        
        Ok(())
    }

    pub fn load(&mut self) -> Result<(), HypervisorError> {
        let phys_addr = self as *mut _ as u64;
        let result: u64;
        
        unsafe {
            asm!(
                "vmptrld [{0}]",
                "pushfq",
                "pop {1}",
                in(reg) &phys_addr,
                out(reg) result,
                options(nostack)
            );
        }
        
        if result & 0x41 != 0 {
            return Err(HypervisorError::InvalidVmcs);
        }
        
        Ok(())
    }
}

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum VmcsField {
    VirtualProcessorId = 0x00000000,
    PostedIntNotificationVector = 0x00000002,
    EptpIndex = 0x00000004,
    
    GuestEsSelector = 0x00000800,
    GuestCsSelector = 0x00000802,
    GuestSsSelector = 0x00000804,
    GuestDsSelector = 0x00000806,
    GuestFsSelector = 0x00000808,
    GuestGsSelector = 0x0000080a,
    GuestLdtrSelector = 0x0000080c,
    GuestTrSelector = 0x0000080e,
    GuestIntrStatus = 0x00000810,
    GuestPmlIndex = 0x00000812,
    
    HostEsSelector = 0x00000c00,
    HostCsSelector = 0x00000c02,
    HostSsSelector = 0x00000c04,
    HostDsSelector = 0x00000c06,
    HostFsSelector = 0x00000c08,
    HostGsSelector = 0x00000c0a,
    HostTrSelector = 0x00000c0c,
    
    IoBitmapA = 0x00002000,
    IoBitmapB = 0x00002002,
    MsrBitmap = 0x00002004,
    VmExitMsrStoreAddr = 0x00002006,
    VmExitMsrLoadAddr = 0x00002008,
    VmEntryMsrLoadAddr = 0x0000200a,
    ExecutiveVmcsPointer = 0x0000200c,
    PmlAddress = 0x0000200e,
    TscOffset = 0x00002010,
    VirtualApicPageAddr = 0x00002012,
    ApicAccessAddr = 0x00002014,
    PostedIntDescAddr = 0x00002016,
    VmFunctionControls = 0x00002018,
    EptPointer = 0x0000201a,
    EoiExitBitmap0 = 0x0000201c,
    EoiExitBitmap1 = 0x0000201e,
    EoiExitBitmap2 = 0x00002020,
    EoiExitBitmap3 = 0x00002022,
    EptpListAddress = 0x00002024,
    VmreadBitmapAddress = 0x00002026,
    VmwriteBitmapAddress = 0x00002028,
    
    GuestPhysicalAddress = 0x00002400,
    
    VmcsLinkPointer = 0x00002800,
    GuestIa32Debugctl = 0x00002802,
    GuestIa32Pat = 0x00002804,
    GuestIa32Efer = 0x00002806,
    GuestIa32PerfGlobalCtrl = 0x00002808,
    GuestPdptr0 = 0x0000280a,
    GuestPdptr1 = 0x0000280c,
    GuestPdptr2 = 0x0000280e,
    GuestPdptr3 = 0x00002810,
    GuestIa32Bndcfgs = 0x00002812,
    GuestIa32RtitCtl = 0x00002814,
    
    HostIa32Pat = 0x00002c00,
    HostIa32Efer = 0x00002c02,
    HostIa32PerfGlobalCtrl = 0x00002c04,
    
    PinBasedVmExecControl = 0x00004000,
    CpuBasedVmExecControl = 0x00004002,
    ExceptionBitmap = 0x00004004,
    PageFaultErrorCodeMask = 0x00004006,
    PageFaultErrorCodeMatch = 0x00004008,
    Cr3TargetCount = 0x0000400a,
    VmExitControls = 0x0000400c,
    VmExitMsrStoreCount = 0x0000400e,
    VmExitMsrLoadCount = 0x00004010,
    VmEntryControls = 0x00004012,
    VmEntryMsrLoadCount = 0x00004014,
    VmEntryIntrInfoField = 0x00004016,
    VmEntryExceptionErrorCode = 0x00004018,
    VmEntryInstructionLen = 0x0000401a,
    TprThreshold = 0x0000401c,
    SecondaryVmExecControl = 0x0000401e,
    PleGap = 0x00004020,
    PleWindow = 0x00004022,
    
    VmInstructionError = 0x00004400,
    VmExitReason = 0x00004402,
    VmExitIntrInfo = 0x00004404,
    VmExitIntrErrorCode = 0x00004406,
    IdtVectoringInfoField = 0x00004408,
    IdtVectoringErrorCode = 0x0000440a,
    VmExitInstructionLen = 0x0000440c,
    VmxInstructionInfo = 0x0000440e,
    
    GuestEsLimit = 0x00004800,
    GuestCsLimit = 0x00004802,
    GuestSsLimit = 0x00004804,
    GuestDsLimit = 0x00004806,
    GuestFsLimit = 0x00004808,
    GuestGsLimit = 0x0000480a,
    GuestLdtrLimit = 0x0000480c,
    GuestTrLimit = 0x0000480e,
    GuestGdtrLimit = 0x00004810,
    GuestIdtrLimit = 0x00004812,
    GuestEsArBytes = 0x00004814,
    GuestCsArBytes = 0x00004816,
    GuestSsArBytes = 0x00004818,
    GuestDsArBytes = 0x0000481a,
    GuestFsArBytes = 0x0000481c,
    GuestGsArBytes = 0x0000481e,
    GuestLdtrArBytes = 0x00004820,
    GuestTrArBytes = 0x00004822,
    GuestInterruptibilityInfo = 0x00004824,
    GuestActivityState = 0x00004826,
    GuestSysenterCs = 0x0000482a,
    VmxPreemptionTimerValue = 0x0000482e,
    
    HostIa32SysenterCs = 0x00004c00,
    
    Cr0GuestHostMask = 0x00006000,
    Cr4GuestHostMask = 0x00006002,
    Cr0ReadShadow = 0x00006004,
    Cr4ReadShadow = 0x00006006,
    Cr3TargetValue0 = 0x00006008,
    Cr3TargetValue1 = 0x0000600a,
    Cr3TargetValue2 = 0x0000600c,
    Cr3TargetValue3 = 0x0000600e,
    
    ExitQualification = 0x00006400,
    GuestLinearAddress = 0x0000640a,
    
    GuestCr0 = 0x00006800,
    GuestCr3 = 0x00006802,
    GuestCr4 = 0x00006804,
    GuestEsBase = 0x00006806,
    GuestCsBase = 0x00006808,
    GuestSsBase = 0x0000680a,
    GuestDsBase = 0x0000680c,
    GuestFsBase = 0x0000680e,
    GuestGsBase = 0x00006810,
    GuestLdtrBase = 0x00006812,
    GuestTrBase = 0x00006814,
    GuestGdtrBase = 0x00006816,
    GuestIdtrBase = 0x00006818,
    GuestDr7 = 0x0000681a,
    GuestRsp = 0x0000681c,
    GuestRip = 0x0000681e,
    GuestRflags = 0x00006820,
    GuestPendingDbgExceptions = 0x00006822,
    GuestSysenterEsp = 0x00006824,
    GuestSysenterEip = 0x00006826,
    
    HostCr0 = 0x00006c00,
    HostCr3 = 0x00006c02,
    HostCr4 = 0x00006c04,
    HostFsBase = 0x00006c06,
    HostGsBase = 0x00006c08,
    HostTrBase = 0x00006c0a,
    HostGdtrBase = 0x00006c0c,
    HostIdtrBase = 0x00006c0e,
    HostIa32SysenterEsp = 0x00006c10,
    HostIa32SysenterEip = 0x00006c12,
    HostRsp = 0x00006c14,
    HostRip = 0x00006c16,
}

pub fn vmread(field: VmcsField) -> Result<u64, HypervisorError> {
    let value: u64;
    let flags: u64;
    
    unsafe {
        asm!(
            "vmread {0}, {1}",
            "pushfq",
            "pop {2}",
            out(reg) value,
            in(reg) field as u64,
            out(reg) flags,
            options(nostack)
        );
    }
    
    if flags & 0x41 != 0 {
        return Err(HypervisorError::InvalidVmcs);
    }
    
    Ok(value)
}

pub fn vmwrite(field: VmcsField, value: u64) -> Result<(), HypervisorError> {
    let flags: u64;
    
    unsafe {
        asm!(
            "vmwrite {0}, {1}",
            "pushfq",
            "pop {2}",
            in(reg) field as u64,
            in(reg) value,
            out(reg) flags,
            options(nostack)
        );
    }
    
    if flags & 0x41 != 0 {
        return Err(HypervisorError::InvalidVmcs);
    }
    
    Ok(())
}

pub fn detect_vmx_capabilities(mut caps: HypervisorCapabilities) -> HypervisorCapabilities {
    unsafe {
        let basic = Msr::new(IA32_VMX_BASIC_MSR).read();
        
        if basic & (1 << 55) != 0 {
            let proc_ctls2 = Msr::new(IA32_VMX_PROCBASED_CTLS2_MSR).read();
            
            if proc_ctls2 & (1 << 1) != 0 {
                caps.ept_supported = true;
            }
            
            if proc_ctls2 & (1 << 5) != 0 {
                caps.vpid_supported = true;
            }
            
            if proc_ctls2 & (1 << 7) != 0 {
                caps.unrestricted_guest = true;
            }
            
            if proc_ctls2 & (1 << 8) != 0 {
                caps.apicv_supported = true;
            }
            
            if proc_ctls2 & (1 << 13) != 0 {
                caps.vmcs_shadowing = true;
            }
            
            if proc_ctls2 & (1 << 22) != 0 {
                caps.posted_interrupts = true;
            }
            
            if proc_ctls2 & (1 << 26) != 0 {
                caps.nested_virt = true;
            }
        }
    }
    
    caps
}

pub fn enable_vmx() -> Result<(), HypervisorError> {
    unsafe {
        let feature_control = Msr::new(IA32_FEATURE_CONTROL_MSR).read();
        
        if feature_control & 1 != 0 {
            if feature_control & (1 << 2) == 0 {
                return Err(HypervisorError::LockedByBios);
            }
        }
        
        let mut cr0 = Cr0::read();
        let fixed0 = Msr::new(IA32_VMX_CR0_FIXED0_MSR).read();
        let fixed1 = Msr::new(IA32_VMX_CR0_FIXED1_MSR).read();
        cr0 = Cr0::from_bits_truncate((cr0.bits() | fixed0) & fixed1);
        Cr0::write(cr0);
        
        let mut cr4 = Cr4::read();
        cr4 |= Cr4Flags::VIRTUAL_MACHINE_EXTENSIONS;
        let fixed0 = Msr::new(IA32_VMX_CR4_FIXED0_MSR).read();
        let fixed1 = Msr::new(IA32_VMX_CR4_FIXED1_MSR).read();
        cr4 = Cr4::from_bits_truncate((cr4.bits() | fixed0) & fixed1);
        Cr4::write(cr4);
        
        let result: u64;
        asm!(
            "vmxon [{0}]",
            "pushfq",
            "pop {1}",
            in(reg) &alloc_vmxon_region(),
            out(reg) result,
            options(nostack)
        );
        
        if result & 0x41 != 0 {
            return Err(HypervisorError::VmxNotSupported);
        }
    }
    
    Ok(())
}

pub fn disable_vmx() -> Result<(), HypervisorError> {
    unsafe {
        asm!(
            "vmxoff",
            options(nostack)
        );
        
        let mut cr4 = Cr4::read();
        cr4 &= !Cr4Flags::VIRTUAL_MACHINE_EXTENSIONS;
        Cr4::write(cr4);
    }
    
    Ok(())
}

fn alloc_vmxon_region() -> u64 {
    let mut region = alloc::vec![0u8; 4096];
    let basic = unsafe { Msr::new(IA32_VMX_BASIC_MSR).read() };
    let revision_id = basic as u32;
    
    unsafe {
        *(region.as_mut_ptr() as *mut u32) = revision_id;
    }
    
    region.as_ptr() as u64
}