// Power Management Drivers Subsystem
use super::*;
use alloc::vec::Vec;
use alloc::string::String;
use crate::nt::NtStatus;

pub fn initialize_power_subsystem() -> NtStatus {
    crate::println!("Power: Initializing power management subsystem");
    
    // Initialize advanced power management
    if let Err(e) = crate::power::init() {
        crate::serial_println!("Power: Failed to initialize power management: {}", e);
        return NtStatus::UnsuccessfulDriver;
    }
    
    // Set default power profile
    if let Err(e) = crate::power::set_power_profile(crate::power::PowerProfile::Balanced) {
        crate::serial_println!("Power: Failed to set power profile: {}", e);
    }
    
    // Initialize thermal monitoring
    if let Err(e) = crate::thermal::init() {
        crate::serial_println!("Power: Failed to initialize thermal management: {}", e);
    }
    
    crate::println!("Power: Advanced power management initialized successfully");
    NtStatus::Success
}