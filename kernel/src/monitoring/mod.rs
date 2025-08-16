#![no_std]

pub mod logging;
pub mod metrics;
pub mod events;
pub mod telemetry;
pub mod health;
pub mod resources;
pub mod diagnostics;

use alloc::vec::Vec;
use alloc::string::String;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

static MONITORING_ENABLED: AtomicBool = AtomicBool::new(true);
static MONITORING_INITIALIZED: AtomicBool = AtomicBool::new(false);

pub struct MonitoringConfig {
    pub logging_enabled: bool,
    pub metrics_enabled: bool,
    pub events_enabled: bool,
    pub telemetry_enabled: bool,
    pub health_checks_enabled: bool,
    pub resource_tracking_enabled: bool,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            logging_enabled: true,
            metrics_enabled: true,
            events_enabled: true,
            telemetry_enabled: false,
            health_checks_enabled: true,
            resource_tracking_enabled: true,
        }
    }
}

static mut MONITORING_CONFIG: MonitoringConfig = MonitoringConfig {
    logging_enabled: true,
    metrics_enabled: true,
    events_enabled: true,
    telemetry_enabled: false,
    health_checks_enabled: true,
    resource_tracking_enabled: true,
};

pub fn init() {
    if MONITORING_INITIALIZED.swap(true, Ordering::SeqCst) {
        return;
    }

    logging::init();
    metrics::init();
    events::init();
    resources::init();
    health::init();
    
    if unsafe { MONITORING_CONFIG.telemetry_enabled } {
        telemetry::init();
    }
    
    crate::println!("[MONITORING] System monitoring initialized");
}

pub fn enable() {
    MONITORING_ENABLED.store(true, Ordering::SeqCst);
}

pub fn disable() {
    MONITORING_ENABLED.store(false, Ordering::SeqCst);
}

pub fn is_enabled() -> bool {
    MONITORING_ENABLED.load(Ordering::SeqCst)
}

pub fn configure(config: MonitoringConfig) {
    unsafe {
        MONITORING_CONFIG = config;
    }
}

pub fn shutdown() {
    logging::flush();
    metrics::flush();
    telemetry::shutdown();
}