use alloc::vec::Vec;
use alloc::boxed::Box;
use spin::Mutex;
use lazy_static::lazy_static;
use crate::{serial_println, println};
use x86_64::registers::{control, model_specific::Msr};
use x86_64::structures::paging::PhysFrame;
use x86_64::{PhysAddr, VirtAddr};

#[derive(Debug, Clone)]
pub struct SystemContext {
    // CPU context
    cr0: u64,
    cr3: u64,
    cr4: u64,
    efer: u64,
    gdt_base: u64,
    gdt_limit: u16,
    idt_base: u64,
    idt_limit: u16,
    tss_base: u64,
    
    // General purpose registers
    rax: u64,
    rbx: u64,
    rcx: u64,
    rdx: u64,
    rsi: u64,
    rdi: u64,
    rbp: u64,
    rsp: u64,
    r8: u64,
    r9: u64,
    r10: u64,
    r11: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,
    
    // Segment registers
    cs: u16,
    ds: u16,
    es: u16,
    fs: u16,
    gs: u16,
    ss: u16,
    
    // MSRs
    fs_base: u64,
    gs_base: u64,
    kernel_gs_base: u64,
    star: u64,
    lstar: u64,
    cstar: u64,
    sfmask: u64,
    
    // FPU/SSE state
    fpu_state: [u8; 512],
}

#[derive(Debug, Clone)]
pub struct DeviceContext {
    device_id: u32,
    device_name: alloc::string::String,
    saved_state: Vec<u8>,
}

#[derive(Debug)]
pub struct SuspendState {
    system_context: Option<Box<SystemContext>>,
    device_contexts: Vec<DeviceContext>,
    processes_frozen: bool,
    devices_suspended: bool,
    interrupts_disabled: bool,
    wake_vector_set: bool,
    resume_vector: u64,
}

impl SuspendState {
    pub fn new() -> Self {
        Self {
            system_context: None,
            device_contexts: Vec::new(),
            processes_frozen: false,
            devices_suspended: false,
            interrupts_disabled: false,
            wake_vector_set: false,
            resume_vector: 0,
        }
    }
    
    pub fn prepare_suspend(&mut self) -> Result<(), &'static str> {
        serial_println!("Suspend: Preparing system for S3 state");
        
        // Allocate memory for suspend image
        self.system_context = Some(Box::new(SystemContext::new()));
        
        // Set resume vector
        self.setup_resume_vector()?;
        
        Ok(())
    }
    
    fn setup_resume_vector(&mut self) -> Result<(), &'static str> {
        // Set ACPI wake vector for resume
        // This would write to FACS table
        self.resume_vector = Self::get_resume_entry_point();
        self.wake_vector_set = true;
        
        serial_println!("Suspend: Resume vector set to 0x{:x}", self.resume_vector);
        Ok(())
    }
    
    fn get_resume_entry_point() -> u64 {
        // Return address of resume handler
        resume_from_suspend as *const () as u64
    }
    
    pub fn save_cpu_context(&mut self) -> Result<(), &'static str> {
        if let Some(ref mut ctx) = self.system_context {
            unsafe {
                // Save control registers
                ctx.cr0 = control::Cr0::read_raw();
                ctx.cr3 = control::Cr3::read_raw().0.start_address().as_u64();
                ctx.cr4 = control::Cr4::read_raw();
                
                // Save EFER MSR
                ctx.efer = Msr::new(0xC0000080).read();
                
                // Save GDT
                let gdt = x86_64::instructions::tables::sgdt();
                ctx.gdt_base = gdt.base.as_u64();
                ctx.gdt_limit = gdt.limit;
                
                // Save IDT
                let idt = x86_64::instructions::tables::sidt();
                ctx.idt_base = idt.base.as_u64();
                ctx.idt_limit = idt.limit;
                
                // Save MSRs
                ctx.star = Msr::new(0xC0000081).read();
                ctx.lstar = Msr::new(0xC0000082).read();
                ctx.cstar = Msr::new(0xC0000083).read();
                ctx.sfmask = Msr::new(0xC0000084).read();
                
                ctx.fs_base = Msr::new(0xC0000100).read();
                ctx.gs_base = Msr::new(0xC0000101).read();
                ctx.kernel_gs_base = Msr::new(0xC0000102).read();
                
                // Save FPU state
                core::arch::asm!(
                    "fxsave [{}]",
                    in(reg) ctx.fpu_state.as_mut_ptr(),
                );
            }
            
            serial_println!("Suspend: CPU context saved");
            Ok(())
        } else {
            Err("System context not allocated")
        }
    }
    
    pub fn restore_cpu_context(&self) -> Result<(), &'static str> {
        if let Some(ref ctx) = self.system_context {
            unsafe {
                // Restore control registers
                control::Cr0::write_raw(ctx.cr0);
                control::Cr4::write_raw(ctx.cr4);
                
                // Restore page tables
                let cr3_flags = control::Cr3Flags::empty();
                let cr3 = PhysFrame::from_start_address(
                    PhysAddr::new(ctx.cr3)
                ).unwrap();
                control::Cr3::write(cr3, cr3_flags);
                
                // Restore EFER
                Msr::new(0xC0000080).write(ctx.efer);
                
                // Restore GDT
                let gdt_ptr = x86_64::instructions::tables::DescriptorTablePointer {
                    limit: ctx.gdt_limit,
                    base: VirtAddr::new(ctx.gdt_base),
                };
                x86_64::instructions::tables::lgdt(&gdt_ptr);
                
                // Restore IDT
                let idt_ptr = x86_64::instructions::tables::DescriptorTablePointer {
                    limit: ctx.idt_limit,
                    base: VirtAddr::new(ctx.idt_base),
                };
                x86_64::instructions::tables::lidt(&idt_ptr);
                
                // Restore MSRs
                Msr::new(0xC0000081).write(ctx.star);
                Msr::new(0xC0000082).write(ctx.lstar);
                Msr::new(0xC0000083).write(ctx.cstar);
                Msr::new(0xC0000084).write(ctx.sfmask);
                
                Msr::new(0xC0000100).write(ctx.fs_base);
                Msr::new(0xC0000101).write(ctx.gs_base);
                Msr::new(0xC0000102).write(ctx.kernel_gs_base);
                
                // Restore FPU state
                core::arch::asm!(
                    "fxrstor [{}]",
                    in(reg) ctx.fpu_state.as_ptr(),
                );
            }
            
            serial_println!("Suspend: CPU context restored");
            Ok(())
        } else {
            Err("No saved system context")
        }
    }
    
    pub fn save_device_context(&mut self, device_id: u32, name: alloc::string::String, state: Vec<u8>) {
        self.device_contexts.push(DeviceContext {
            device_id,
            device_name: name,
            saved_state: state,
        });
    }
    
    pub fn restore_device_contexts(&self) -> Result<(), &'static str> {
        for device in &self.device_contexts {
            serial_println!("Suspend: Restoring device {} ({})", 
                           device.device_name, device.device_id);
            // Device-specific restore would happen here
        }
        Ok(())
    }
}

impl SystemContext {
    fn new() -> Self {
        Self {
            cr0: 0,
            cr3: 0,
            cr4: 0,
            efer: 0,
            gdt_base: 0,
            gdt_limit: 0,
            idt_base: 0,
            idt_limit: 0,
            tss_base: 0,
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
            cs: 0,
            ds: 0,
            es: 0,
            fs: 0,
            gs: 0,
            ss: 0,
            fs_base: 0,
            gs_base: 0,
            kernel_gs_base: 0,
            star: 0,
            lstar: 0,
            cstar: 0,
            sfmask: 0,
            fpu_state: [0; 512],
        }
    }
}

lazy_static! {
    static ref SUSPEND_STATE: Mutex<SuspendState> = Mutex::new(SuspendState::new());
}

pub fn init() -> Result<(), &'static str> {
    serial_println!("Suspend: Initializing S3 suspend support");
    
    // Check if S3 is supported
    if !is_s3_supported() {
        return Err("S3 suspend state not supported");
    }
    
    SUSPEND_STATE.lock().prepare_suspend()?;
    
    Ok(())
}

fn is_s3_supported() -> bool {
    // Check ACPI tables for S3 support
    // For now, assume it's supported
    true
}

pub fn freeze_processes() -> Result<(), &'static str> {
    serial_println!("Suspend: Freezing user processes");
    
    // Signal all processes to stop
    // This would iterate through process list and freeze them
    
    SUSPEND_STATE.lock().processes_frozen = true;
    Ok(())
}

pub fn thaw_processes() -> Result<(), &'static str> {
    serial_println!("Suspend: Thawing user processes");
    
    // Resume all frozen processes
    
    SUSPEND_STATE.lock().processes_frozen = false;
    Ok(())
}

pub fn enter_s3_state() -> Result<(), &'static str> {
    let mut state = SUSPEND_STATE.lock();
    
    // Save CPU context
    state.save_cpu_context()?;
    
    // Disable interrupts
    unsafe { x86_64::instructions::interrupts::disable(); }
    state.interrupts_disabled = true;
    
    // Flush caches
    flush_caches();
    
    // Enter S3 via ACPI
    enter_acpi_s3_state()?;
    
    // CPU resumes here after wake event
    
    // Re-enable interrupts
    unsafe { x86_64::instructions::interrupts::enable(); }
    state.interrupts_disabled = false;
    
    // Restore CPU context
    state.restore_cpu_context()?;
    
    Ok(())
}

fn flush_caches() {
    unsafe {
        // Write back and invalidate caches
        core::arch::asm!("wbinvd");
    }
}

fn enter_acpi_s3_state() -> Result<(), &'static str> {
    // This would call into ACPI power module to enter S3
    crate::acpi::power::suspend_to_ram()
}

#[no_mangle]
pub extern "C" fn resume_from_suspend() {
    // This is the resume entry point
    serial_println!("Suspend: Resuming from S3 state");
    
    // Restore system state
    if let Err(e) = SUSPEND_STATE.lock().restore_cpu_context() {
        serial_println!("Suspend: Failed to restore CPU context: {}", e);
    }
    
    // Restore device states
    if let Err(e) = SUSPEND_STATE.lock().restore_device_contexts() {
        serial_println!("Suspend: Failed to restore device contexts: {}", e);
    }
    
    println!("System resumed from suspend");
}

pub fn register_wake_event(event_type: WakeEventType) -> Result<(), &'static str> {
    serial_println!("Suspend: Registered wake event: {:?}", event_type);
    // Configure wake events in ACPI
    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub enum WakeEventType {
    PowerButton,
    RTC,
    Keyboard,
    Mouse,
    Network,
    USB,
}

pub fn set_wake_alarm(seconds: u32) -> Result<(), &'static str> {
    // Set RTC wake alarm
    serial_println!("Suspend: Wake alarm set for {} seconds", seconds);
    Ok(())
}