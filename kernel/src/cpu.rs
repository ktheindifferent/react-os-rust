use core::arch::x86_64::{__cpuid, __cpuid_count};
use bitflags::bitflags;

bitflags! {
    pub struct CpuFeatures: u64 {
        const SSE = 1 << 0;
        const SSE2 = 1 << 1;
        const SSE3 = 1 << 2;
        const SSSE3 = 1 << 3;
        const SSE41 = 1 << 4;
        const SSE42 = 1 << 5;
        const AVX = 1 << 6;
        const AVX2 = 1 << 7;
        const FMA = 1 << 8;
        const AES = 1 << 9;
        const RDRAND = 1 << 10;
        const RDSEED = 1 << 11;
        const POPCNT = 1 << 12;
        const BMI1 = 1 << 13;
        const BMI2 = 1 << 14;
        const FSGSBASE = 1 << 15;
        const XSAVE = 1 << 16;
        const OSXSAVE = 1 << 17;
        const AVX512F = 1 << 18;
        const AVX512DQ = 1 << 19;
        const AVX512BW = 1 << 20;
        const AVX512VL = 1 << 21;
        const HYPERVISOR = 1 << 22;
        const TSC = 1 << 23;
        const TSC_DEADLINE = 1 << 24;
        const APIC = 1 << 25;
        const X2APIC = 1 << 26;
        const SMEP = 1 << 27;
        const SMAP = 1 << 28;
        const PCID = 1 << 29;
        const INVPCID = 1 << 30;
    }
}

pub struct CpuInfo {
    pub vendor: [u8; 12],
    pub max_cpuid: u32,
    pub max_extended_cpuid: u32,
    pub features: CpuFeatures,
    pub processor_brand: [u8; 48],
    pub physical_cores: u8,
    pub logical_cores: u8,
    pub cache_line_size: u16,
    pub l1_data_cache: u32,
    pub l1_inst_cache: u32,
    pub l2_cache: u32,
    pub l3_cache: u32,
}

impl CpuInfo {
    pub fn detect() -> Self {
        let mut info = Self {
            vendor: [0; 12],
            max_cpuid: 0,
            max_extended_cpuid: 0,
            features: CpuFeatures::empty(),
            processor_brand: [0; 48],
            physical_cores: 1,
            logical_cores: 1,
            cache_line_size: 64,
            l1_data_cache: 0,
            l1_inst_cache: 0,
            l2_cache: 0,
            l3_cache: 0,
        };

        unsafe {
            // Get vendor string and max CPUID
            let cpuid = __cpuid(0);
            info.max_cpuid = cpuid.eax;
            
            // Vendor string is in ebx, edx, ecx
            info.vendor[0..4].copy_from_slice(&cpuid.ebx.to_le_bytes());
            info.vendor[4..8].copy_from_slice(&cpuid.edx.to_le_bytes());
            info.vendor[8..12].copy_from_slice(&cpuid.ecx.to_le_bytes());

            // Get extended CPUID max
            let cpuid = __cpuid(0x80000000);
            info.max_extended_cpuid = cpuid.eax;

            // Feature detection
            if info.max_cpuid >= 1 {
                let cpuid = __cpuid(1);
                
                // EDX features
                if cpuid.edx & (1 << 25) != 0 { info.features |= CpuFeatures::SSE; }
                if cpuid.edx & (1 << 26) != 0 { info.features |= CpuFeatures::SSE2; }
                if cpuid.edx & (1 << 4) != 0 { info.features |= CpuFeatures::TSC; }
                if cpuid.edx & (1 << 9) != 0 { info.features |= CpuFeatures::APIC; }
                
                // ECX features
                if cpuid.ecx & (1 << 0) != 0 { info.features |= CpuFeatures::SSE3; }
                if cpuid.ecx & (1 << 9) != 0 { info.features |= CpuFeatures::SSSE3; }
                if cpuid.ecx & (1 << 19) != 0 { info.features |= CpuFeatures::SSE41; }
                if cpuid.ecx & (1 << 20) != 0 { info.features |= CpuFeatures::SSE42; }
                if cpuid.ecx & (1 << 25) != 0 { info.features |= CpuFeatures::AES; }
                if cpuid.ecx & (1 << 26) != 0 { info.features |= CpuFeatures::XSAVE; }
                if cpuid.ecx & (1 << 27) != 0 { info.features |= CpuFeatures::OSXSAVE; }
                if cpuid.ecx & (1 << 28) != 0 { info.features |= CpuFeatures::AVX; }
                if cpuid.ecx & (1 << 12) != 0 { info.features |= CpuFeatures::FMA; }
                if cpuid.ecx & (1 << 30) != 0 { info.features |= CpuFeatures::RDRAND; }
                if cpuid.ecx & (1 << 23) != 0 { info.features |= CpuFeatures::POPCNT; }
                if cpuid.ecx & (1 << 21) != 0 { info.features |= CpuFeatures::X2APIC; }
                if cpuid.ecx & (1 << 24) != 0 { info.features |= CpuFeatures::TSC_DEADLINE; }
                if cpuid.ecx & (1 << 31) != 0 { info.features |= CpuFeatures::HYPERVISOR; }
                
                // Get logical core count
                info.logical_cores = ((cpuid.ebx >> 16) & 0xFF) as u8;
            }

            // Extended features
            if info.max_cpuid >= 7 {
                let cpuid = __cpuid_count(7, 0);
                
                // EBX features
                if cpuid.ebx & (1 << 0) != 0 { info.features |= CpuFeatures::FSGSBASE; }
                if cpuid.ebx & (1 << 3) != 0 { info.features |= CpuFeatures::BMI1; }
                if cpuid.ebx & (1 << 5) != 0 { info.features |= CpuFeatures::AVX2; }
                if cpuid.ebx & (1 << 7) != 0 { info.features |= CpuFeatures::SMEP; }
                if cpuid.ebx & (1 << 8) != 0 { info.features |= CpuFeatures::BMI2; }
                if cpuid.ebx & (1 << 10) != 0 { info.features |= CpuFeatures::INVPCID; }
                if cpuid.ebx & (1 << 16) != 0 { info.features |= CpuFeatures::AVX512F; }
                if cpuid.ebx & (1 << 17) != 0 { info.features |= CpuFeatures::AVX512DQ; }
                if cpuid.ebx & (1 << 18) != 0 { info.features |= CpuFeatures::RDSEED; }
                if cpuid.ebx & (1 << 20) != 0 { info.features |= CpuFeatures::SMAP; }
                if cpuid.ebx & (1 << 30) != 0 { info.features |= CpuFeatures::AVX512BW; }
                if cpuid.ebx & (1 << 31) != 0 { info.features |= CpuFeatures::AVX512VL; }
                
                // ECX features
                if cpuid.ecx & (1 << 17) != 0 { info.features |= CpuFeatures::PCID; }
            }

            // Get processor brand string
            if info.max_extended_cpuid >= 0x80000004 {
                let mut brand_idx = 0;
                for i in 0x80000002..=0x80000004 {
                    let cpuid = __cpuid(i);
                    info.processor_brand[brand_idx..brand_idx+4].copy_from_slice(&cpuid.eax.to_le_bytes());
                    info.processor_brand[brand_idx+4..brand_idx+8].copy_from_slice(&cpuid.ebx.to_le_bytes());
                    info.processor_brand[brand_idx+8..brand_idx+12].copy_from_slice(&cpuid.ecx.to_le_bytes());
                    info.processor_brand[brand_idx+12..brand_idx+16].copy_from_slice(&cpuid.edx.to_le_bytes());
                    brand_idx += 16;
                }
            }

            // Get cache information
            if info.max_cpuid >= 4 {
                for i in 0.. {
                    let cpuid = __cpuid_count(4, i);
                    let cache_type = cpuid.eax & 0x1F;
                    
                    if cache_type == 0 {
                        break;
                    }
                    
                    let cache_level = (cpuid.eax >> 5) & 0x7;
                    let cache_size = ((cpuid.ebx >> 22) + 1) * 
                                    (((cpuid.ebx >> 12) & 0x3FF) + 1) *
                                    ((cpuid.ebx & 0xFFF) + 1) *
                                    (cpuid.ecx + 1);
                    
                    match (cache_level, cache_type) {
                        (1, 1) => info.l1_data_cache = cache_size,
                        (1, 2) => info.l1_inst_cache = cache_size,
                        (2, _) => info.l2_cache = cache_size,
                        (3, _) => info.l3_cache = cache_size,
                        _ => {}
                    }
                    
                    if i == 0 {
                        info.cache_line_size = ((cpuid.ebx & 0xFFF) + 1) as u16;
                        info.physical_cores = ((cpuid.eax >> 26) + 1) as u8;
                    }
                }
            }
        }

        info
    }

    pub fn print_info(&self) {
        use crate::println;
        
        println!("CPU Information:");
        println!("  Vendor: {}", core::str::from_utf8(&self.vendor).unwrap_or("Unknown"));
        
        let brand = core::str::from_utf8(&self.processor_brand)
            .unwrap_or("Unknown")
            .trim_end_matches('\0');
        println!("  Brand: {}", brand);
        
        println!("  Physical Cores: {}", self.physical_cores);
        println!("  Logical Cores: {}", self.logical_cores);
        println!("  Cache Line Size: {} bytes", self.cache_line_size);
        
        if self.l1_data_cache > 0 {
            println!("  L1 Data Cache: {} KB", self.l1_data_cache / 1024);
        }
        if self.l1_inst_cache > 0 {
            println!("  L1 Instruction Cache: {} KB", self.l1_inst_cache / 1024);
        }
        if self.l2_cache > 0 {
            println!("  L2 Cache: {} KB", self.l2_cache / 1024);
        }
        if self.l3_cache > 0 {
            println!("  L3 Cache: {} KB", self.l3_cache / 1024);
        }
        
        println!("  Features:");
        if self.features.contains(CpuFeatures::SSE) { println!("    - SSE"); }
        if self.features.contains(CpuFeatures::SSE2) { println!("    - SSE2"); }
        if self.features.contains(CpuFeatures::SSE3) { println!("    - SSE3"); }
        if self.features.contains(CpuFeatures::SSSE3) { println!("    - SSSE3"); }
        if self.features.contains(CpuFeatures::SSE41) { println!("    - SSE4.1"); }
        if self.features.contains(CpuFeatures::SSE42) { println!("    - SSE4.2"); }
        if self.features.contains(CpuFeatures::AVX) { println!("    - AVX"); }
        if self.features.contains(CpuFeatures::AVX2) { println!("    - AVX2"); }
        if self.features.contains(CpuFeatures::AVX512F) { println!("    - AVX-512F"); }
        if self.features.contains(CpuFeatures::FMA) { println!("    - FMA"); }
        if self.features.contains(CpuFeatures::AES) { println!("    - AES-NI"); }
        if self.features.contains(CpuFeatures::RDRAND) { println!("    - RDRAND"); }
        if self.features.contains(CpuFeatures::RDSEED) { println!("    - RDSEED"); }
        if self.features.contains(CpuFeatures::POPCNT) { println!("    - POPCNT"); }
        if self.features.contains(CpuFeatures::BMI1) { println!("    - BMI1"); }
        if self.features.contains(CpuFeatures::BMI2) { println!("    - BMI2"); }
        if self.features.contains(CpuFeatures::SMEP) { println!("    - SMEP"); }
        if self.features.contains(CpuFeatures::SMAP) { println!("    - SMAP"); }
        if self.features.contains(CpuFeatures::HYPERVISOR) { println!("    - Running in VM"); }
    }

    #[inline]
    pub fn has_sse2(&self) -> bool {
        self.features.contains(CpuFeatures::SSE2)
    }

    #[inline]
    pub fn has_avx(&self) -> bool {
        self.features.contains(CpuFeatures::AVX)
    }

    #[inline]
    pub fn has_avx2(&self) -> bool {
        self.features.contains(CpuFeatures::AVX2)
    }

    #[inline]
    pub fn has_rdrand(&self) -> bool {
        self.features.contains(CpuFeatures::RDRAND)
    }
}

static mut CPU_INFO: Option<CpuInfo> = None;

pub fn init() {
    unsafe {
        CPU_INFO = Some(CpuInfo::detect());
    }
}

pub fn get_info() -> &'static CpuInfo {
    unsafe {
        CPU_INFO.as_ref().expect("CPU info not initialized")
    }
}

pub fn rdtsc() -> u64 {
    unsafe {
        core::arch::x86_64::_rdtsc()
    }
}

pub fn rdrand() -> Option<u64> {
    if get_info().has_rdrand() {
        let mut val: u64;
        let mut success: u8;
        unsafe {
            core::arch::asm!(
                "rdrand {}",
                "setc {}",
                out(reg) val,
                out(reg_byte) success,
                options(nomem, nostack, preserves_flags)
            );
        }
        if success != 0 {
            Some(val)
        } else {
            None
        }
    } else {
        None
    }
}

// Get current CPU ID (for multi-core support)
pub fn get_cpu_id() -> u32 {
    // For now, return 0 (single core)
    // In future, read from LAPIC or use core ID from CPUID
    0
}

// Enable/disable CPU features
pub fn enable_sse() {
    unsafe {
        // Enable SSE by setting CR4.OSFXSR (bit 9)
        let mut cr4: u64;
        core::arch::asm!("mov {}, cr4", out(reg) cr4);
        cr4 |= 1 << 9;
        core::arch::asm!("mov cr4, {}", in(reg) cr4);
    }
}

pub fn enable_avx() {
    if get_info().has_avx() {
        unsafe {
            // Enable XSAVE by setting CR4.OSXSAVE (bit 18)
            let mut cr4: u64;
            core::arch::asm!("mov {}, cr4", out(reg) cr4);
            cr4 |= 1 << 18;
            core::arch::asm!("mov cr4, {}", in(reg) cr4);
            
            // Enable AVX in XCR0
            let mut xcr0: u64;
            core::arch::asm!(
                "xor ecx, ecx",
                "xgetbv",
                out("eax") xcr0,
                out("edx") _,
            );
            xcr0 |= 0x7; // Enable x87, SSE, and AVX state
            core::arch::asm!(
                "xor ecx, ecx",
                "xsetbv",
                in("eax") xcr0 as u32,
                in("edx") (xcr0 >> 32) as u32,
            );
        }
    }
}

// MSR (Model Specific Register) access
pub fn read_msr(msr: u32) -> u64 {
    let low: u32;
    let high: u32;
    unsafe {
        core::arch::asm!(
            "rdmsr",
            in("ecx") msr,
            out("eax") low,
            out("edx") high,
        );
    }
    ((high as u64) << 32) | (low as u64)
}

pub fn write_msr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    unsafe {
        core::arch::asm!(
            "wrmsr",
            in("ecx") msr,
            in("eax") low,
            in("edx") high,
        );
    }
}

// Performance monitoring
pub fn enable_performance_counters() {
    // Enable performance monitoring in CR4
    unsafe {
        let mut cr4: u64;
        core::arch::asm!("mov {}, cr4", out(reg) cr4);
        cr4 |= 1 << 8; // Set PCE (Performance-monitoring counter enable)
        core::arch::asm!("mov cr4, {}", in(reg) cr4);
    }
}