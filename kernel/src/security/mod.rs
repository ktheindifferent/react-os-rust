pub mod kaslr;
pub mod stack_protection;
pub mod memory_protection;
pub mod secure_boot;
pub mod capabilities;
pub mod sandbox;
pub mod mitigations;
pub mod audit;
pub mod integrity;

use crate::serial_println;

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use spin::Mutex;
use alloc::vec::Vec;
use alloc::string::String;

static SECURITY_INITIALIZED: AtomicBool = AtomicBool::new(false);
static SECURITY_FEATURES: AtomicU64 = AtomicU64::new(0);

#[repr(u64)]
#[derive(Debug, Clone, Copy)]
pub enum SecurityFeature {
    Kaslr = 1 << 0,
    StackCanaries = 1 << 1,
    StackGuardPages = 1 << 2,
    ShadowStack = 1 << 3,
    WriteXorExecute = 1 << 4,
    Smep = 1 << 5,
    Smap = 1 << 6,
    HeapHardening = 1 << 7,
    ControlFlowIntegrity = 1 << 8,
    SecureBoot = 1 << 9,
    Capabilities = 1 << 10,
    Sandboxing = 1 << 11,
    SpectreMitigation = 1 << 12,
    MeltdownMitigation = 1 << 13,
    RopProtection = 1 << 14,
    BoundsChecking = 1 << 15,
    IntegerOverflowDetection = 1 << 16,
    AuditingEnabled = 1 << 17,
    IntegrityChecking = 1 << 18,
}

#[derive(Clone, Copy)]
pub struct SecurityConfig {
    pub kaslr_enabled: bool,
    pub stack_protection_level: StackProtectionLevel,
    pub memory_protection_strict: bool,
    pub secure_boot_required: bool,
    pub audit_level: AuditLevel,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StackProtectionLevel {
    None,
    Basic,      // Just canaries
    Enhanced,   // Canaries + guard pages
    Maximum,    // Canaries + guard pages + shadow stack
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AuditLevel {
    None,
    Critical,   // Only critical security events
    Normal,     // Important security events
    Verbose,    // All security events
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            kaslr_enabled: true,
            stack_protection_level: StackProtectionLevel::Enhanced,
            memory_protection_strict: true,
            secure_boot_required: false,
            audit_level: AuditLevel::Normal,
        }
    }
}

static SECURITY_CONFIG: Mutex<Option<SecurityConfig>> = Mutex::new(None);

pub fn init(config: SecurityConfig) {
    if SECURITY_INITIALIZED.load(Ordering::SeqCst) {
        serial_println!("[SECURITY] Already initialized");
        return;
    }

    serial_println!("[SECURITY] Initializing security subsystem");
    
    *SECURITY_CONFIG.lock() = Some(config);
    
    let mut features = 0u64;
    
    // Initialize KASLR if enabled
    if config.kaslr_enabled {
        if kaslr::init() {
            features |= SecurityFeature::Kaslr as u64;
            serial_println!("[SECURITY] KASLR enabled");
        }
    }
    
    // Initialize stack protection
    match config.stack_protection_level {
        StackProtectionLevel::None => {},
        StackProtectionLevel::Basic => {
            stack_protection::init_canaries();
            features |= SecurityFeature::StackCanaries as u64;
            serial_println!("[SECURITY] Stack canaries enabled");
        },
        StackProtectionLevel::Enhanced => {
            stack_protection::init_canaries();
            stack_protection::init_guard_pages();
            features |= SecurityFeature::StackCanaries as u64;
            features |= SecurityFeature::StackGuardPages as u64;
            serial_println!("[SECURITY] Stack canaries and guard pages enabled");
        },
        StackProtectionLevel::Maximum => {
            stack_protection::init_canaries();
            stack_protection::init_guard_pages();
            if stack_protection::init_shadow_stack() {
                features |= SecurityFeature::ShadowStack as u64;
            }
            features |= SecurityFeature::StackCanaries as u64;
            features |= SecurityFeature::StackGuardPages as u64;
            serial_println!("[SECURITY] Maximum stack protection enabled");
        },
    }
    
    // Initialize memory protection
    if config.memory_protection_strict {
        memory_protection::init();
        features |= SecurityFeature::WriteXorExecute as u64;
        
        if memory_protection::enable_smep() {
            features |= SecurityFeature::Smep as u64;
            serial_println!("[SECURITY] SMEP enabled");
        }
        
        if memory_protection::enable_smap() {
            features |= SecurityFeature::Smap as u64;
            serial_println!("[SECURITY] SMAP enabled");
        }
        
        memory_protection::enable_heap_hardening();
        features |= SecurityFeature::HeapHardening as u64;
        serial_println!("[SECURITY] Heap hardening enabled");
    }
    
    // Initialize vulnerability mitigations
    mitigations::init();
    if mitigations::enable_spectre_mitigation() {
        features |= SecurityFeature::SpectreMitigation as u64;
        serial_println!("[SECURITY] Spectre mitigation enabled");
    }
    
    if mitigations::enable_meltdown_mitigation() {
        features |= SecurityFeature::MeltdownMitigation as u64;
        serial_println!("[SECURITY] Meltdown mitigation enabled");
    }
    
    if mitigations::enable_rop_protection() {
        features |= SecurityFeature::RopProtection as u64;
        serial_println!("[SECURITY] ROP protection enabled");
    }
    
    // Initialize auditing
    if config.audit_level != AuditLevel::None {
        audit::init(config.audit_level);
        features |= SecurityFeature::AuditingEnabled as u64;
        serial_println!("[SECURITY] Auditing enabled at level: {:?}", config.audit_level);
    }
    
    // Initialize integrity checking
    integrity::init();
    features |= SecurityFeature::IntegrityChecking as u64;
    serial_println!("[SECURITY] Kernel integrity checking enabled");
    
    SECURITY_FEATURES.store(features, Ordering::SeqCst);
    SECURITY_INITIALIZED.store(true, Ordering::SeqCst);
    
    serial_println!("[SECURITY] Security subsystem initialized with features: 0x{:x}", features);
}

pub fn is_feature_enabled(feature: SecurityFeature) -> bool {
    let features = SECURITY_FEATURES.load(Ordering::SeqCst);
    (features & (feature as u64)) != 0
}

pub fn get_enabled_features() -> Vec<SecurityFeature> {
    let features = SECURITY_FEATURES.load(Ordering::SeqCst);
    let mut result = Vec::new();
    
    for i in 0..19 {
        let feature_bit = 1u64 << i;
        if (features & feature_bit) != 0 {
            use SecurityFeature::*;
            let feature = match feature_bit {
                f if f == Kaslr as u64 => Kaslr,
                f if f == StackCanaries as u64 => StackCanaries,
                f if f == StackGuardPages as u64 => StackGuardPages,
                f if f == ShadowStack as u64 => ShadowStack,
                f if f == WriteXorExecute as u64 => WriteXorExecute,
                f if f == Smep as u64 => Smep,
                f if f == Smap as u64 => Smap,
                f if f == HeapHardening as u64 => HeapHardening,
                f if f == ControlFlowIntegrity as u64 => ControlFlowIntegrity,
                f if f == SecureBoot as u64 => SecureBoot,
                f if f == Capabilities as u64 => Capabilities,
                f if f == Sandboxing as u64 => Sandboxing,
                f if f == SpectreMitigation as u64 => SpectreMitigation,
                f if f == MeltdownMitigation as u64 => MeltdownMitigation,
                f if f == RopProtection as u64 => RopProtection,
                f if f == BoundsChecking as u64 => BoundsChecking,
                f if f == IntegerOverflowDetection as u64 => IntegerOverflowDetection,
                f if f == AuditingEnabled as u64 => AuditingEnabled,
                f if f == IntegrityChecking as u64 => IntegrityChecking,
                _ => continue,
            };
            result.push(feature);
        }
    }
    
    result
}