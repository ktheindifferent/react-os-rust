use crate::serial_println;
use x86_64::{VirtAddr, PhysAddr};
use core::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use spin::Mutex;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;

static KASLR_ENABLED: AtomicBool = AtomicBool::new(false);
static KERNEL_SLIDE: AtomicU64 = AtomicU64::new(0);
static MODULE_SLIDE_BASE: AtomicU64 = AtomicU64::new(0);

const MAX_KERNEL_SLIDE: u64 = 0x1000_0000;  // 256MB maximum slide
const MODULE_SLIDE_RANGE: u64 = 0x800_0000; // 128MB range for modules
const ENTROPY_POOL_SIZE: usize = 64;

pub struct KaslrConfig {
    pub kernel_base: VirtAddr,
    pub kernel_size: u64,
    pub module_base: VirtAddr,
    pub module_size: u64,
}

struct EntropySource {
    pool: [u64; ENTROPY_POOL_SIZE],
    index: usize,
}

impl EntropySource {
    fn new() -> Self {
        let mut pool = [0u64; ENTROPY_POOL_SIZE];
        
        // Initialize with various entropy sources
        for i in 0..ENTROPY_POOL_SIZE {
            pool[i] = Self::gather_entropy(i);
        }
        
        Self { pool, index: 0 }
    }
    
    fn gather_entropy(seed: usize) -> u64 {
        let mut entropy = 0u64;
        
        // Use RDTSC for timing entropy
        unsafe {
            let tsc: u64;
            core::arch::asm!("rdtsc", out("rax") tsc, out("rdx") _);
            entropy ^= tsc;
        }
        
        // Use RDRAND if available
        if crate::cpu::get_info().has_rdrand() {
            unsafe {
                let mut random: u64;
                core::arch::asm!(
                    "rdrand {}",
                    out(reg) random,
                    options(nomem, nostack)
                );
                entropy ^= random;
            }
        }
        
        // Mix with seed and memory addresses
        entropy ^= seed as u64;
        entropy ^= &seed as *const _ as u64;
        
        // Simple mixing function
        entropy = entropy.wrapping_mul(0x9e3779b97f4a7c15);
        entropy ^= entropy >> 30;
        entropy = entropy.wrapping_mul(0xbf58476d1ce4e5b9);
        entropy ^= entropy >> 27;
        entropy = entropy.wrapping_mul(0x94d049bb133111eb);
        entropy ^= entropy >> 31;
        
        entropy
    }
    
    fn get_random(&mut self) -> u64 {
        let result = self.pool[self.index];
        self.index = (self.index + 1) % ENTROPY_POOL_SIZE;
        
        // Refresh the used entry
        self.pool[self.index] = Self::gather_entropy(self.index);
        
        result
    }
}

static ENTROPY: Mutex<Option<EntropySource>> = Mutex::new(None);

pub fn init() -> bool {
    if KASLR_ENABLED.load(Ordering::SeqCst) {
        return true;
    }
    
    // Initialize entropy source
    *ENTROPY.lock() = Some(EntropySource::new());
    
    // Calculate random kernel slide
    let slide = calculate_kernel_slide();
    KERNEL_SLIDE.store(slide, Ordering::SeqCst);
    
    // Calculate module base slide
    let module_slide = calculate_module_slide();
    MODULE_SLIDE_BASE.store(module_slide, Ordering::SeqCst);
    
    serial_println!("[KASLR] Kernel slide: 0x{:x}", slide);
    serial_println!("[KASLR] Module base: 0x{:x}", module_slide);
    
    KASLR_ENABLED.store(true, Ordering::SeqCst);
    
    true
}

fn calculate_kernel_slide() -> u64 {
    let mut entropy_guard = ENTROPY.lock();
    if let Some(ref mut entropy) = *entropy_guard {
        let random = entropy.get_random();
        // Align to 2MB boundary for large page support
        let slide = (random % (MAX_KERNEL_SLIDE / 0x200000)) * 0x200000;
        slide
    } else {
        0
    }
}

fn calculate_module_slide() -> u64 {
    let mut entropy_guard = ENTROPY.lock();
    if let Some(ref mut entropy) = *entropy_guard {
        let random = entropy.get_random();
        // Align to 4KB boundary
        let slide = (random % (MODULE_SLIDE_RANGE / 0x1000)) * 0x1000;
        slide
    } else {
        0
    }
}

pub fn randomize_address(base: VirtAddr, is_module: bool) -> VirtAddr {
    if !KASLR_ENABLED.load(Ordering::SeqCst) {
        return base;
    }
    
    if is_module {
        let slide = MODULE_SLIDE_BASE.load(Ordering::SeqCst);
        VirtAddr::new(base.as_u64() + slide)
    } else {
        let slide = KERNEL_SLIDE.load(Ordering::SeqCst);
        VirtAddr::new(base.as_u64() + slide)
    }
}

pub fn get_kernel_slide() -> u64 {
    KERNEL_SLIDE.load(Ordering::SeqCst)
}

pub fn hide_kernel_addresses() -> bool {
    KASLR_ENABLED.load(Ordering::SeqCst)
}

pub fn sanitize_kernel_address(addr: VirtAddr) -> VirtAddr {
    if hide_kernel_addresses() {
        // Return a sanitized address for userspace
        VirtAddr::new(0xFFFF_FFFF_FFFF_FFFF)
    } else {
        addr
    }
}

pub struct RelocatedSymbol {
    pub name: &'static str,
    pub original_addr: VirtAddr,
    pub relocated_addr: VirtAddr,
}

static SYMBOL_TABLE: Mutex<BTreeMap<&'static str, RelocatedSymbol>> = Mutex::new(BTreeMap::new());

pub fn register_symbol(name: &'static str, addr: VirtAddr) {
    if !KASLR_ENABLED.load(Ordering::SeqCst) {
        return;
    }
    
    let relocated = randomize_address(addr, false);
    let symbol = RelocatedSymbol {
        name,
        original_addr: addr,
        relocated_addr: relocated,
    };
    
    SYMBOL_TABLE.lock().insert(name, symbol);
}

pub fn resolve_symbol(name: &str) -> Option<VirtAddr> {
    SYMBOL_TABLE.lock().get(name).map(|s| s.relocated_addr)
}

pub fn apply_relocations(image_base: VirtAddr, relocations: &[(u64, RelocationType)]) {
    let slide = get_kernel_slide();
    
    for (offset, reloc_type) in relocations {
        let addr = image_base + *offset;
        
        match reloc_type {
            RelocationType::Absolute64 => {
                unsafe {
                    let ptr = addr.as_mut_ptr::<u64>();
                    *ptr = (*ptr).wrapping_add(slide);
                }
            },
            RelocationType::Relative32 => {
                // Relative relocations don't need adjustment for KASLR
            },
            RelocationType::BaseReloc => {
                unsafe {
                    let ptr = addr.as_mut_ptr::<u64>();
                    let original = *ptr;
                    if original >= 0xFFFF_8000_0000_0000 {
                        *ptr = original.wrapping_add(slide);
                    }
                }
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum RelocationType {
    Absolute64,
    Relative32,
    BaseReloc,
}

pub fn randomize_stack_base() -> VirtAddr {
    if !KASLR_ENABLED.load(Ordering::SeqCst) {
        return VirtAddr::new(0xFFFF_FF00_0000_0000);
    }
    
    let mut entropy_guard = ENTROPY.lock();
    if let Some(ref mut entropy) = *entropy_guard {
        let random = entropy.get_random();
        // Randomize within a 1GB range, aligned to 16 bytes
        let offset = (random % (0x4000_0000 / 16)) * 16;
        VirtAddr::new(0xFFFF_FF00_0000_0000 - offset)
    } else {
        VirtAddr::new(0xFFFF_FF00_0000_0000)
    }
}

pub fn randomize_heap_base() -> VirtAddr {
    if !KASLR_ENABLED.load(Ordering::SeqCst) {
        return VirtAddr::new(0xFFFF_8800_0000_0000);
    }
    
    let mut entropy_guard = ENTROPY.lock();
    if let Some(ref mut entropy) = *entropy_guard {
        let random = entropy.get_random();
        // Randomize within a 256MB range, aligned to page size
        let offset = (random % (0x1000_0000 / 0x1000)) * 0x1000;
        VirtAddr::new(0xFFFF_8800_0000_0000 + offset)
    } else {
        VirtAddr::new(0xFFFF_8800_0000_0000)
    }
}