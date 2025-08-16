use core::sync::atomic::{AtomicU32, AtomicBool, Ordering};
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;
use crate::acpi::apic::LOCAL_APIC;
use super::percpu;

pub const IPI_VECTOR_RESCHEDULE: u8 = 0xFC;
pub const IPI_VECTOR_CALL_FUNCTION: u8 = 0xFB;
pub const IPI_VECTOR_CALL_FUNCTION_SINGLE: u8 = 0xFA;
pub const IPI_VECTOR_TLB_FLUSH: u8 = 0xF9;
pub const IPI_VECTOR_PANIC: u8 = 0xF8;
pub const IPI_VECTOR_TIMER: u8 = 0xF7;

type IpiHandler = fn();

#[derive(Clone)]
struct CallFunctionData {
    func: fn(*mut u8),
    data: *mut u8,
    wait: bool,
    cpu_mask: u64,
    started: AtomicU32,
    finished: AtomicU32,
}

unsafe impl Send for CallFunctionData {}
unsafe impl Sync for CallFunctionData {}

lazy_static! {
    static ref IPI_HANDLERS: Mutex<[Option<IpiHandler>; 256]> = 
        Mutex::new([None; 256]);
    
    static ref CALL_FUNCTION_DATA: Mutex<Option<CallFunctionData>> = 
        Mutex::new(None);
    
    static ref CALL_SINGLE_DATA: Mutex<Vec<Option<CallFunctionData>>> = {
        let mut vec = Vec::with_capacity(super::MAX_CPUS);
        for _ in 0..super::MAX_CPUS {
            vec.push(None);
        }
        Mutex::new(vec)
    };
}

pub fn init_ipi() {
    register_ipi_handler(IPI_VECTOR_RESCHEDULE, handle_reschedule_ipi);
    register_ipi_handler(IPI_VECTOR_CALL_FUNCTION, handle_call_function_ipi);
    register_ipi_handler(IPI_VECTOR_CALL_FUNCTION_SINGLE, handle_call_function_single_ipi);
    register_ipi_handler(IPI_VECTOR_TLB_FLUSH, handle_tlb_flush_ipi);
    register_ipi_handler(IPI_VECTOR_PANIC, handle_panic_ipi);
    register_ipi_handler(IPI_VECTOR_TIMER, handle_timer_ipi);
}

pub fn register_ipi_handler(vector: u8, handler: IpiHandler) {
    let mut handlers = IPI_HANDLERS.lock();
    handlers[vector as usize] = Some(handler);
}

pub fn handle_ipi(vector: u8) {
    percpu::this_cpu_inc_ipi();
    
    let handlers = IPI_HANDLERS.lock();
    if let Some(handler) = handlers[vector as usize] {
        drop(handlers);
        handler();
    } else {
        crate::serial_println!("IPI: Unhandled IPI vector {:#x}", vector);
    }
    
    if let Some(ref lapic) = *LOCAL_APIC.lock() {
        lapic.eoi();
    }
}

pub fn send_ipi(apic_id: u8, vector: u8) {
    if let Some(ref lapic) = *LOCAL_APIC.lock() {
        lapic.send_ipi(apic_id, vector, 0);
    }
}

pub fn send_ipi_mask(cpu_mask: u64, vector: u8) {
    let smp = super::SMP_MANAGER.lock();
    
    for cpu_id in 0..64 {
        if (cpu_mask & (1 << cpu_id)) == 0 {
            continue;
        }
        
        if let Some(cpu) = smp.get_cpu(cpu_id) {
            if cpu.state == super::CpuState::Online {
                send_ipi(cpu.apic_id, vector);
            }
        }
    }
}

pub fn send_ipi_all(vector: u8) {
    super::send_ipi_to_all(vector, false);
}

pub fn send_ipi_all_but_self(vector: u8) {
    super::send_ipi_to_others(vector);
}

pub fn send_reschedule_ipi(cpu_id: u32) {
    super::send_ipi_to_cpu(cpu_id, IPI_VECTOR_RESCHEDULE);
}

pub fn smp_call_function(func: fn(*mut u8), data: *mut u8, wait: bool) {
    let current_cpu = percpu::get_cpu_id();
    let smp = super::SMP_MANAGER.lock();
    let online_cpus = smp.get_online_cpus();
    drop(smp);
    
    let mut cpu_mask = 0u64;
    for cpu_id in online_cpus {
        if cpu_id != current_cpu && cpu_id < 64 {
            cpu_mask |= 1 << cpu_id;
        }
    }
    
    if cpu_mask == 0 {
        return;
    }
    
    let cpu_count = cpu_mask.count_ones();
    
    let call_data = CallFunctionData {
        func,
        data,
        wait,
        cpu_mask,
        started: AtomicU32::new(0),
        finished: AtomicU32::new(0),
    };
    
    {
        let mut global_data = CALL_FUNCTION_DATA.lock();
        *global_data = Some(call_data.clone());
    }
    
    core::sync::atomic::fence(Ordering::Release);
    
    send_ipi_mask(cpu_mask, IPI_VECTOR_CALL_FUNCTION);
    
    while call_data.started.load(Ordering::Acquire) < cpu_count {
        core::hint::spin_loop();
    }
    
    if wait {
        while call_data.finished.load(Ordering::Acquire) < cpu_count {
            core::hint::spin_loop();
        }
    }
    
    {
        let mut global_data = CALL_FUNCTION_DATA.lock();
        *global_data = None;
    }
}

pub fn smp_call_function_single(cpu_id: u32, func: fn(*mut u8), data: *mut u8, wait: bool) {
    if cpu_id == percpu::get_cpu_id() {
        func(data);
        return;
    }
    
    let call_data = CallFunctionData {
        func,
        data,
        wait,
        cpu_mask: 1 << cpu_id,
        started: AtomicU32::new(0),
        finished: AtomicU32::new(0),
    };
    
    {
        let mut single_data = CALL_SINGLE_DATA.lock();
        single_data[cpu_id as usize] = Some(call_data.clone());
    }
    
    core::sync::atomic::fence(Ordering::Release);
    
    super::send_ipi_to_cpu(cpu_id, IPI_VECTOR_CALL_FUNCTION_SINGLE);
    
    while call_data.started.load(Ordering::Acquire) == 0 {
        core::hint::spin_loop();
    }
    
    if wait {
        while call_data.finished.load(Ordering::Acquire) == 0 {
            core::hint::spin_loop();
        }
    }
    
    {
        let mut single_data = CALL_SINGLE_DATA.lock();
        single_data[cpu_id as usize] = None;
    }
}

fn handle_reschedule_ipi() {
    percpu::set_need_resched();
}

fn handle_call_function_ipi() {
    let call_data = {
        let global_data = CALL_FUNCTION_DATA.lock();
        global_data.clone()
    };
    
    if let Some(data) = call_data {
        let cpu_id = percpu::get_cpu_id();
        if (data.cpu_mask & (1 << cpu_id)) != 0 {
            data.started.fetch_add(1, Ordering::Release);
            
            (data.func)(data.data);
            
            data.finished.fetch_add(1, Ordering::Release);
        }
    }
}

fn handle_call_function_single_ipi() {
    let cpu_id = percpu::get_cpu_id();
    
    let call_data = {
        let single_data = CALL_SINGLE_DATA.lock();
        single_data[cpu_id as usize].clone()
    };
    
    if let Some(data) = call_data {
        data.started.fetch_add(1, Ordering::Release);
        
        (data.func)(data.data);
        
        data.finished.fetch_add(1, Ordering::Release);
    }
}

fn handle_tlb_flush_ipi() {
    unsafe {
        core::arch::x86_64::_mm_mfence();
        core::arch::asm!("mov rax, cr3; mov cr3, rax", out("rax") _);
    }
    
    percpu::get_percpu().tlb_flush_count.fetch_add(1, Ordering::Relaxed);
}

fn handle_panic_ipi() {
    crate::serial_println!("CPU {}: Received panic IPI, halting", percpu::get_cpu_id());
    loop {
        unsafe {
            core::arch::asm!("cli; hlt");
        }
    }
}

fn handle_timer_ipi() {
    crate::timer::handle_timer_interrupt();
}

pub fn send_tlb_flush_ipi() {
    send_ipi_all_but_self(IPI_VECTOR_TLB_FLUSH);
}

pub fn send_panic_ipi() {
    send_ipi_all_but_self(IPI_VECTOR_PANIC);
}

pub fn flush_tlb_all() {
    send_tlb_flush_ipi();
    
    unsafe {
        core::arch::x86_64::_mm_mfence();
        core::arch::asm!("mov rax, cr3; mov cr3, rax", out("rax") _);
    }
}

pub fn flush_tlb_single(addr: usize) {
    struct FlushData {
        addr: usize,
    }
    
    let data = FlushData { addr };
    
    fn flush_handler(data: *mut u8) {
        let flush_data = unsafe { &*(data as *const FlushData) };
        unsafe {
            core::arch::asm!("invlpg [{}]", in(reg) flush_data.addr);
        }
    }
    
    smp_call_function(flush_handler, &data as *const _ as *mut u8, true);
    
    unsafe {
        core::arch::asm!("invlpg [{}]", in(reg) addr);
    }
}