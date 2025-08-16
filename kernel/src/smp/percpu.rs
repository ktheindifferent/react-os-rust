use core::sync::atomic::{AtomicU32, AtomicPtr, Ordering};
use core::arch::asm;
use core::mem;
use alloc::boxed::Box;
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;
use super::MAX_CPUS;

#[repr(C)]
pub struct PerCpuData {
    pub cpu_id: u32,
    pub apic_id: u8,
    pub kernel_stack: u64,
    pub user_stack: u64,
    pub tss_addr: u64,
    pub current_task: AtomicPtr<()>,
    pub idle_task: AtomicPtr<()>,
    pub irq_count: AtomicU32,
    pub softirq_pending: AtomicU32,
    pub preempt_count: AtomicU32,
    pub need_resched: AtomicU32,
    pub in_interrupt: AtomicU32,
    pub cpu_usage: CpuUsageStats,
    pub tlb_flush_count: AtomicU32,
    pub ipi_count: AtomicU32,
    _padding: [u8; 64],
}

#[derive(Default)]
pub struct CpuUsageStats {
    pub user_time: AtomicU32,
    pub kernel_time: AtomicU32,
    pub idle_time: AtomicU32,
    pub irq_time: AtomicU32,
    pub softirq_time: AtomicU32,
}

impl PerCpuData {
    pub fn new(cpu_id: u32, apic_id: u8) -> Self {
        Self {
            cpu_id,
            apic_id,
            kernel_stack: 0,
            user_stack: 0,
            tss_addr: 0,
            current_task: AtomicPtr::new(core::ptr::null_mut()),
            idle_task: AtomicPtr::new(core::ptr::null_mut()),
            irq_count: AtomicU32::new(0),
            softirq_pending: AtomicU32::new(0),
            preempt_count: AtomicU32::new(0),
            need_resched: AtomicU32::new(0),
            in_interrupt: AtomicU32::new(0),
            cpu_usage: CpuUsageStats::default(),
            tlb_flush_count: AtomicU32::new(0),
            ipi_count: AtomicU32::new(0),
            _padding: [0; 64],
        }
    }

    pub fn inc_irq_count(&self) {
        self.irq_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_ipi_count(&self) {
        self.ipi_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn enter_interrupt(&self) {
        self.in_interrupt.fetch_add(1, Ordering::Acquire);
    }

    pub fn leave_interrupt(&self) {
        self.in_interrupt.fetch_sub(1, Ordering::Release);
    }

    pub fn in_interrupt(&self) -> bool {
        self.in_interrupt.load(Ordering::Acquire) > 0
    }

    pub fn inc_preempt(&self) {
        self.preempt_count.fetch_add(1, Ordering::Acquire);
    }

    pub fn dec_preempt(&self) {
        self.preempt_count.fetch_sub(1, Ordering::Release);
    }

    pub fn preemptible(&self) -> bool {
        self.preempt_count.load(Ordering::Acquire) == 0
    }

    pub fn set_need_resched(&self) {
        self.need_resched.store(1, Ordering::Release);
    }

    pub fn clear_need_resched(&self) {
        self.need_resched.store(0, Ordering::Relaxed);
    }

    pub fn need_resched(&self) -> bool {
        self.need_resched.load(Ordering::Acquire) != 0
    }
}

lazy_static! {
    static ref PER_CPU_DATA: Mutex<Vec<Option<Box<PerCpuData>>>> = {
        let mut vec = Vec::with_capacity(MAX_CPUS);
        for _ in 0..MAX_CPUS {
            vec.push(None);
        }
        Mutex::new(vec)
    };
}

const MSR_GS_BASE: u32 = 0xC0000101;
const MSR_KERNEL_GS_BASE: u32 = 0xC0000102;

pub fn init_percpu() {
    let cpu_id = get_apic_id() as u32;
    
    let percpu = Box::new(PerCpuData::new(cpu_id, get_apic_id()));
    let percpu_ptr = Box::into_raw(percpu);
    
    unsafe {
        wrmsr(MSR_GS_BASE, percpu_ptr as u64);
        wrmsr(MSR_KERNEL_GS_BASE, percpu_ptr as u64);
    }
    
    let mut percpu_vec = PER_CPU_DATA.lock();
    percpu_vec[cpu_id as usize] = Some(unsafe { Box::from_raw(percpu_ptr) });
}

pub fn get_percpu() -> &'static PerCpuData {
    unsafe {
        let ptr: *const PerCpuData;
        asm!("mov {}, gs:0", out(reg) ptr);
        &*ptr
    }
}

pub fn get_percpu_mut() -> &'static mut PerCpuData {
    unsafe {
        let ptr: *mut PerCpuData;
        asm!("mov {}, gs:0", out(reg) ptr);
        &mut *ptr
    }
}

pub fn get_cpu_id() -> u32 {
    get_percpu().cpu_id
}

pub fn set_cpu_id(id: u32) {
    get_percpu_mut().cpu_id = id;
}

pub fn get_apic_id() -> u8 {
    unsafe {
        if let Some(ref lapic) = *crate::acpi::apic::LOCAL_APIC.lock() {
            lapic.id()
        } else {
            0
        }
    }
}

pub fn this_cpu_inc_irq() {
    get_percpu().inc_irq_count();
}

pub fn this_cpu_inc_ipi() {
    get_percpu().inc_ipi_count();
}

pub fn this_cpu_enter_irq() {
    get_percpu().enter_interrupt();
}

pub fn this_cpu_leave_irq() {
    get_percpu().leave_interrupt();
}

pub fn this_cpu_in_irq() -> bool {
    get_percpu().in_interrupt()
}

pub fn preempt_disable() {
    get_percpu().inc_preempt();
}

pub fn preempt_enable() {
    let percpu = get_percpu();
    percpu.dec_preempt();
    
    if percpu.preemptible() && percpu.need_resched() {
        crate::process::scheduler::schedule();
    }
}

pub fn set_need_resched() {
    get_percpu().set_need_resched();
}

pub fn clear_need_resched() {
    get_percpu().clear_need_resched();
}

unsafe fn wrmsr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    asm!(
        "wrmsr",
        in("ecx") msr,
        in("eax") low,
        in("edx") high,
    );
}

unsafe fn rdmsr(msr: u32) -> u64 {
    let low: u32;
    let high: u32;
    asm!(
        "rdmsr",
        in("ecx") msr,
        out("eax") low,
        out("edx") high,
    );
    ((high as u64) << 32) | (low as u64)
}

pub struct PerCpuVar<T> {
    data: [Option<T>; MAX_CPUS],
}

impl<T: Default + Clone> PerCpuVar<T> {
    pub const fn new() -> Self {
        Self {
            data: [None; MAX_CPUS],
        }
    }

    pub fn init(&mut self, value: T) {
        let cpu_id = get_cpu_id() as usize;
        self.data[cpu_id] = Some(value);
    }

    pub fn get(&self) -> &T {
        let cpu_id = get_cpu_id() as usize;
        self.data[cpu_id].as_ref().unwrap()
    }

    pub fn get_mut(&mut self) -> &mut T {
        let cpu_id = get_cpu_id() as usize;
        self.data[cpu_id].as_mut().unwrap()
    }

    pub fn get_for_cpu(&self, cpu_id: u32) -> Option<&T> {
        self.data[cpu_id as usize].as_ref()
    }

    pub fn set(&mut self, value: T) {
        let cpu_id = get_cpu_id() as usize;
        self.data[cpu_id] = Some(value);
    }
}

#[macro_export]
macro_rules! define_per_cpu {
    ($name:ident, $type:ty, $init:expr) => {
        lazy_static::lazy_static! {
            static ref $name: spin::Mutex<$crate::smp::percpu::PerCpuVar<$type>> = {
                let mut var = $crate::smp::percpu::PerCpuVar::new();
                spin::Mutex::new(var)
            };
        }
    };
}

#[macro_export]
macro_rules! this_cpu {
    ($var:ident) => {{
        $var.lock().get()
    }};
}

#[macro_export]
macro_rules! this_cpu_mut {
    ($var:ident) => {{
        $var.lock().get_mut()
    }};
}