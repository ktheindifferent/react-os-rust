use crate::serial_println;
use alloc::vec::Vec;
use alloc::string::{String, ToString};
use alloc::collections::VecDeque;
use alloc::{vec, format};
use spin::Mutex;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::AuditLevel;

static AUDIT_ENABLED: AtomicBool = AtomicBool::new(false);
static AUDIT_LEVEL: Mutex<AuditLevel> = Mutex::new(AuditLevel::Normal);
static EVENT_COUNTER: AtomicU64 = AtomicU64::new(0);

const MAX_LOG_ENTRIES: usize = 10000;
const MAX_LOG_SIZE: usize = 1024 * 1024; // 1MB

#[derive(Debug, Clone)]
pub struct AuditEvent {
    pub id: u64,
    pub timestamp: u64,
    pub event_type: SecurityEvent,
    pub pid: u64,
    pub uid: u32,
    pub severity: Severity,
    pub message: String,
    pub details: EventDetails,
}

#[derive(Debug, Clone)]
pub enum SecurityEvent {
    // Authentication events
    LoginSuccess,
    LoginFailure,
    LogoutSuccess,
    PrivilegeEscalation,
    
    // Access control events
    FileAccessGranted,
    FileAccessDenied,
    ProcessAccessGranted,
    ProcessAccessDenied,
    
    // System events
    SystemStartup,
    SystemShutdown,
    ServiceStart,
    ServiceStop,
    
    // Security violations
    StackOverflow,
    HeapCorruption,
    InvalidMemoryAccess,
    BufferOverflow,
    IntegerOverflow,
    
    // Capability events
    CapabilityGranted,
    CapabilityDenied,
    CapabilityDropped,
    
    // Sandbox events
    SandboxCreated,
    SandboxViolation,
    SandboxEscapeAttempt,
    
    // Network events
    NetworkConnectionEstablished,
    NetworkConnectionDenied,
    SuspiciousNetworkActivity,
    
    // Cryptographic events
    SignatureVerificationSuccess,
    SignatureVerificationFailure,
    CertificateExpired,
    
    // Mitigation events
    SpectreAttackBlocked,
    MeltdownAttackBlocked,
    RopAttackBlocked,
    
    // Integrity events
    KernelIntegrityCheck,
    ModuleIntegrityFailure,
    ConfigurationChange,
}

#[derive(Debug, Clone)]
pub enum Severity {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone)]
pub struct EventDetails {
    pub source_ip: Option<String>,
    pub target_path: Option<String>,
    pub syscall: Option<u32>,
    pub error_code: Option<u32>,
    pub additional_info: Vec<(String, String)>,
}

impl EventDetails {
    pub fn new() -> Self {
        Self {
            source_ip: None,
            target_path: None,
            syscall: None,
            error_code: None,
            additional_info: Vec::new(),
        }
    }
}

static AUDIT_LOG: Mutex<VecDeque<AuditEvent>> = Mutex::new(VecDeque::new());
static AUDIT_FILTERS: Mutex<Vec<AuditFilter>> = Mutex::new(Vec::new());

#[derive(Debug, Clone)]
pub struct AuditFilter {
    pub event_types: Vec<SecurityEvent>,
    pub min_severity: Severity,
    pub uid_filter: Option<u32>,
    pub pid_filter: Option<u64>,
}

pub fn init(level: AuditLevel) {
    if AUDIT_ENABLED.load(Ordering::SeqCst) {
        return;
    }
    
    *AUDIT_LEVEL.lock() = level;
    
    // Set up default filters based on audit level
    setup_default_filters(level);
    
    AUDIT_ENABLED.store(true, Ordering::SeqCst);
    
    // Log initialization
    log_event(
        SecurityEvent::SystemStartup,
        Severity::Info,
        "Security auditing system initialized",
        EventDetails::new(),
    );
    
    serial_println!("[AUDIT] Security auditing initialized at level: {:?}", level);
}

fn setup_default_filters(level: AuditLevel) {
    let mut filters = AUDIT_FILTERS.lock();
    
    match level {
        AuditLevel::None => {
            // No filtering, no logging
        },
        AuditLevel::Critical => {
            // Only critical security events
            filters.push(AuditFilter {
                event_types: vec![],
                min_severity: Severity::Critical,
                uid_filter: None,
                pid_filter: None,
            });
        },
        AuditLevel::Normal => {
            // Important security events
            filters.push(AuditFilter {
                event_types: vec![],
                min_severity: Severity::Warning,
                uid_filter: None,
                pid_filter: None,
            });
        },
        AuditLevel::Verbose => {
            // All security events
            filters.push(AuditFilter {
                event_types: vec![],
                min_severity: Severity::Debug,
                uid_filter: None,
                pid_filter: None,
            });
        },
    }
}

pub fn log_event(
    event_type: SecurityEvent,
    severity: Severity,
    message: &str,
    details: EventDetails,
) {
    if !AUDIT_ENABLED.load(Ordering::SeqCst) {
        return;
    }
    
    // Check if event should be logged based on filters
    if !should_log_event(&event_type, &severity) {
        return;
    }
    
    let event = AuditEvent {
        id: EVENT_COUNTER.fetch_add(1, Ordering::SeqCst),
        timestamp: get_timestamp(),
        event_type,
        pid: get_current_pid(),
        uid: get_current_uid(),
        severity,
        message: message.to_string(),
        details,
    };
    
    // Store event
    store_event(event.clone());
    
    // Output to serial for critical events
    if matches!(event.severity, Severity::Critical | Severity::Error) {
        serial_println!("[AUDIT] {:?}: {}", event.severity, event.message);
    }
}

fn should_log_event(event_type: &SecurityEvent, severity: &Severity) -> bool {
    let level = AUDIT_LEVEL.lock();
    
    match *level {
        AuditLevel::None => false,
        AuditLevel::Critical => matches!(severity, Severity::Critical),
        AuditLevel::Normal => !matches!(severity, Severity::Debug),
        AuditLevel::Verbose => true,
    }
}

fn store_event(event: AuditEvent) {
    let mut log = AUDIT_LOG.lock();
    
    // Enforce log size limits
    if log.len() >= MAX_LOG_ENTRIES {
        log.pop_front(); // Remove oldest event
    }
    
    log.push_back(event);
}

fn get_timestamp() -> u64 {
    // Get system timestamp
    unsafe {
        let tsc: u64;
        core::arch::asm!("rdtsc", out("rax") tsc, out("rdx") _);
        tsc
    }
}

fn get_current_pid() -> u64 {
    // Get from scheduler
    0 // Placeholder
}

fn get_current_uid() -> u32 {
    // Get from process context
    0 // Placeholder
}

pub fn log_security_event(event_type: SecurityEvent, message: &str) {
    log_event(
        event_type,
        Severity::Warning,
        message,
        EventDetails::new(),
    );
}

pub fn log_access_violation(path: &str, operation: &str) {
    let mut details = EventDetails::new();
    details.target_path = Some(path.to_string());
    details.additional_info.push(("operation".to_string(), operation.to_string()));
    
    log_event(
        SecurityEvent::FileAccessDenied,
        Severity::Warning,
        &format!("Access denied: {} on {}", operation, path),
        details,
    );
}

pub fn log_capability_event(cap: super::capabilities::Capability, granted: bool) {
    let event_type = if granted {
        SecurityEvent::CapabilityGranted
    } else {
        SecurityEvent::CapabilityDenied
    };
    
    let mut details = EventDetails::new();
    details.additional_info.push(("capability".to_string(), format!("{:?}", cap)));
    
    log_event(
        event_type,
        Severity::Info,
        &format!("Capability {:?} {}", cap, if granted { "granted" } else { "denied" }),
        details,
    );
}

pub fn log_sandbox_violation(sandbox_id: u64, violation_type: &str) {
    let mut details = EventDetails::new();
    details.additional_info.push(("sandbox_id".to_string(), sandbox_id.to_string()));
    details.additional_info.push(("violation_type".to_string(), violation_type.to_string()));
    
    log_event(
        SecurityEvent::SandboxViolation,
        Severity::Error,
        &format!("Sandbox {} violation: {}", sandbox_id, violation_type),
        details,
    );
}

pub fn log_attack_blocked(attack_type: &str) {
    let event_type = match attack_type {
        "spectre" => SecurityEvent::SpectreAttackBlocked,
        "meltdown" => SecurityEvent::MeltdownAttackBlocked,
        "rop" => SecurityEvent::RopAttackBlocked,
        _ => SecurityEvent::SuspiciousNetworkActivity,
    };
    
    log_event(
        event_type,
        Severity::Critical,
        &format!("{} attack attempt blocked", attack_type),
        EventDetails::new(),
    );
}

pub fn get_audit_log(filter: Option<AuditFilter>) -> Vec<AuditEvent> {
    let log = AUDIT_LOG.lock();
    
    if let Some(f) = filter {
        log.iter()
            .filter(|e| apply_filter(e, &f))
            .cloned()
            .collect()
    } else {
        log.iter().cloned().collect()
    }
}

fn apply_filter(event: &AuditEvent, filter: &AuditFilter) -> bool {
    // Check severity
    let severity_match = match (&event.severity, &filter.min_severity) {
        (Severity::Debug, _) => true,
        (Severity::Info, Severity::Debug) => false,
        (Severity::Info, _) => true,
        (Severity::Warning, Severity::Debug | Severity::Info) => false,
        (Severity::Warning, _) => true,
        (Severity::Error, Severity::Critical) => false,
        (Severity::Error, _) => true,
        (Severity::Critical, _) => true,
    };
    
    if !severity_match {
        return false;
    }
    
    // Check UID filter
    if let Some(uid) = filter.uid_filter {
        if event.uid != uid {
            return false;
        }
    }
    
    // Check PID filter
    if let Some(pid) = filter.pid_filter {
        if event.pid != pid {
            return false;
        }
    }
    
    true
}

pub fn export_audit_log() -> String {
    let log = AUDIT_LOG.lock();
    let mut output = String::new();
    
    for event in log.iter() {
        output.push_str(&format!(
            "[{}] {:?} - {:?}: {} (PID: {}, UID: {})\n",
            event.timestamp,
            event.severity,
            event.event_type,
            event.message,
            event.pid,
            event.uid
        ));
    }
    
    output
}

pub fn clear_audit_log() {
    if !AUDIT_ENABLED.load(Ordering::SeqCst) {
        return;
    }
    
    AUDIT_LOG.lock().clear();
    EVENT_COUNTER.store(0, Ordering::SeqCst);
    
    log_event(
        SecurityEvent::ConfigurationChange,
        Severity::Info,
        "Audit log cleared",
        EventDetails::new(),
    );
}

pub struct SecurityAuditor;

impl SecurityAuditor {
    pub fn log_security_event(&self, event_type: SecurityEvent, message: &str) {
        log_security_event(event_type, message);
    }
}

pub fn get_auditor() -> Option<SecurityAuditor> {
    if AUDIT_ENABLED.load(Ordering::SeqCst) {
        Some(SecurityAuditor)
    } else {
        None
    }
}