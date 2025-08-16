use core::sync::atomic::{AtomicU32, AtomicBool, Ordering};
use core::arch::asm;
use x86_64::{PhysAddr, VirtAddr};
use x86_64::structures::paging::PageTable;
use crate::memory::PHYS_MEM_OFFSET;
use crate::acpi::apic::LOCAL_APIC;
use super::{SMP_MANAGER, CpuState};

const AP_BOOT_CODE_ADDR: u64 = 0x8000;
const AP_BOOT_STACK_SIZE: usize = 4096;
const AP_STARTUP_TIMEOUT_MS: u64 = 100;

static AP_BOOT_FLAG: AtomicBool = AtomicBool::new(false);
static AP_CPU_COUNT: AtomicU32 = AtomicU32::new(0);
static AP_BOOT_ERROR: AtomicBool = AtomicBool::new(false);

#[repr(C, packed)]
struct ApBootInfo {
    cr3: u64,
    gdt_ptr: u64,
    idt_ptr: u64,
    stack_top: u64,
    entry_point: u64,
    cpu_id: u32,
    apic_id: u8,
    _padding: [u8; 3],
}

static mut AP_BOOT_INFO: ApBootInfo = ApBootInfo {
    cr3: 0,
    gdt_ptr: 0,
    idt_ptr: 0,
    stack_top: 0,
    entry_point: 0,
    cpu_id: 0,
    apic_id: 0,
    _padding: [0; 3],
};

#[link_section = ".ap_boot"]
#[no_mangle]
pub static AP_BOOT_CODE: [u8; 512] = *include_bytes!("ap_trampoline.bin");

pub fn start_application_processors() -> Result<(), &'static str> {
    let smp = SMP_MANAGER.lock();
    let ap_count = smp.cpu_count() - 1;
    
    if ap_count == 0 {
        return Ok(());
    }
    
    crate::serial_println!("SMP: Starting {} application processors", ap_count);
    
    unsafe {
        copy_ap_boot_code();
        
        setup_ap_boot_info();
    }
    
    let mut started_count = 0;
    for cpu in smp.cpus.iter() {
        if cpu.is_bsp {
            continue;
        }
        
        if start_single_ap(cpu.cpu_id, cpu.apic_id)? {
            started_count += 1;
        }
    }
    
    drop(smp);
    
    if started_count != ap_count {
        crate::serial_println!(
            "SMP: Warning: Only {}/{} APs started successfully",
            started_count,
            ap_count
        );
    } else {
        crate::serial_println!("SMP: All {} APs started successfully", ap_count);
    }
    
    SMP_MANAGER.lock().mark_boot_complete();
    
    Ok(())
}

unsafe fn copy_ap_boot_code() {
    let boot_code_addr = (PHYS_MEM_OFFSET + AP_BOOT_CODE_ADDR) as *mut u8;
    
    core::ptr::copy_nonoverlapping(
        AP_BOOT_CODE.as_ptr(),
        boot_code_addr,
        AP_BOOT_CODE.len()
    );
    
    asm!("mfence");
}

unsafe fn setup_ap_boot_info() {
    let mut cr3: u64;
    asm!("mov {}, cr3", out(reg) cr3);
    AP_BOOT_INFO.cr3 = cr3;
    
    let gdt_ptr = crate::gdt::get_gdt_ptr();
    AP_BOOT_INFO.gdt_ptr = gdt_ptr as u64;
    
    let idt_ptr = crate::interrupts::get_idt_ptr();
    AP_BOOT_INFO.idt_ptr = idt_ptr as u64;
    
    AP_BOOT_INFO.entry_point = ap_entry_point as u64;
    
    let boot_info_addr = (PHYS_MEM_OFFSET + AP_BOOT_CODE_ADDR + 0x500) as *mut ApBootInfo;
    core::ptr::write_volatile(boot_info_addr, AP_BOOT_INFO);
    
    asm!("mfence");
}

fn start_single_ap(cpu_id: u32, apic_id: u8) -> Result<bool, &'static str> {
    crate::serial_println!("SMP: Starting AP {} (APIC ID {})", cpu_id, apic_id);
    
    unsafe {
        let stack_size = AP_BOOT_STACK_SIZE * 16;
        let stack_bottom = crate::allocator::alloc_pages(stack_size / 4096)
            .ok_or("Failed to allocate AP stack")?;
        let stack_top = stack_bottom as u64 + stack_size as u64;
        
        let boot_info_addr = (PHYS_MEM_OFFSET + AP_BOOT_CODE_ADDR + 0x500) as *mut ApBootInfo;
        (*boot_info_addr).stack_top = stack_top;
        (*boot_info_addr).cpu_id = cpu_id;
        (*boot_info_addr).apic_id = apic_id;
        
        asm!("mfence");
    }
    
    AP_BOOT_FLAG.store(false, Ordering::SeqCst);
    AP_BOOT_ERROR.store(false, Ordering::SeqCst);
    
    {
        let mut smp = SMP_MANAGER.lock();
        if let Some(cpu) = smp.get_cpu_mut(cpu_id) {
            cpu.state = CpuState::Booting;
        }
    }
    
    send_init_ipi(apic_id);
    crate::timer::delay_ms(10);
    
    send_startup_ipi(apic_id, AP_BOOT_CODE_ADDR);
    crate::timer::delay_ms(1);
    
    send_startup_ipi(apic_id, AP_BOOT_CODE_ADDR);
    
    let timeout = crate::timer::get_ticks() + (AP_STARTUP_TIMEOUT_MS * 1000);
    while crate::timer::get_ticks() < timeout {
        if AP_BOOT_FLAG.load(Ordering::Acquire) {
            {
                let mut smp = SMP_MANAGER.lock();
                if let Some(cpu) = smp.get_cpu_mut(cpu_id) {
                    cpu.state = CpuState::Online;
                }
                smp.mark_cpu_online(cpu_id);
            }
            
            crate::serial_println!("SMP: AP {} online", cpu_id);
            return Ok(true);
        }
        
        if AP_BOOT_ERROR.load(Ordering::Acquire) {
            crate::serial_println!("SMP: AP {} reported boot error", cpu_id);
            return Ok(false);
        }
        
        crate::timer::delay_us(100);
    }
    
    {
        let mut smp = SMP_MANAGER.lock();
        if let Some(cpu) = smp.get_cpu_mut(cpu_id) {
            cpu.state = CpuState::Offline;
        }
    }
    
    crate::serial_println!("SMP: AP {} startup timeout", cpu_id);
    Ok(false)
}

fn send_init_ipi(apic_id: u8) {
    if let Some(ref lapic) = *LOCAL_APIC.lock() {
        lapic.send_ipi(apic_id, 0, 0x5);
    }
}

fn send_startup_ipi(apic_id: u8, start_addr: u64) {
    if let Some(ref lapic) = *LOCAL_APIC.lock() {
        let vector = (start_addr >> 12) as u8;
        lapic.send_ipi(apic_id, vector, 0x6);
    }
}

#[no_mangle]
pub extern "C" fn ap_entry_point() {
    unsafe {
        super::percpu::init_percpu();
        
        crate::gdt::load_gdt();
        crate::interrupts::load_idt();
        
        if let Some(ref lapic) = *LOCAL_APIC.lock() {
            lapic.init();
        }
        
        super::percpu::set_cpu_id(AP_CPU_COUNT.fetch_add(1, Ordering::SeqCst) + 1);
        
        crate::timer::init_ap_timer();
        
        AP_BOOT_FLAG.store(true, Ordering::Release);
        
        ap_idle_loop();
    }
}

fn ap_idle_loop() -> ! {
    let cpu_id = super::current_cpu_id();
    crate::serial_println!("AP {}: Entering idle loop", cpu_id);
    
    loop {
        unsafe {
            asm!("hlt");
        }
        
        crate::process::scheduler::schedule();
    }
}