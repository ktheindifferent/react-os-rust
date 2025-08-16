// Power management tests
#[cfg(test)]
mod tests {
    use super::super::*;
    
    #[test]
    fn test_power_profiles() {
        // Test switching between power profiles
        assert!(set_power_profile(PowerProfile::Balanced).is_ok());
        assert_eq!(get_current_profile(), PowerProfile::Balanced);
        
        assert!(set_power_profile(PowerProfile::Performance).is_ok());
        assert_eq!(get_current_profile(), PowerProfile::Performance);
        
        assert!(set_power_profile(PowerProfile::PowerSaver).is_ok());
        assert_eq!(get_current_profile(), PowerProfile::PowerSaver);
    }
    
    #[test]
    fn test_power_states() {
        // Test power state queries
        let state = get_power_state();
        assert_eq!(state, PowerState::S0); // Should be in working state
    }
    
    #[test]
    fn test_cpu_frequency_governor() {
        // Test CPU frequency governor changes
        assert!(cpufreq::set_governor(governor::CpuGovernor::OnDemand).is_ok());
        assert!(cpufreq::set_governor(governor::CpuGovernor::Performance).is_ok());
        assert!(cpufreq::set_governor(governor::CpuGovernor::PowerSave).is_ok());
    }
    
    #[test]
    fn test_device_power_management() {
        // Test device power management policies
        assert!(device::set_runtime_pm_policy(device::RuntimePmPolicy::Auto).is_ok());
        assert!(device::set_runtime_pm_policy(device::RuntimePmPolicy::Aggressive).is_ok());
        assert!(device::set_runtime_pm_policy(device::RuntimePmPolicy::Disabled).is_ok());
    }
    
    #[test]
    fn test_thermal_policies() {
        // Test thermal management policies
        use crate::thermal::ThermalPolicy;
        assert!(crate::thermal::set_thermal_policy(ThermalPolicy::Balanced).is_ok());
        assert!(crate::thermal::set_thermal_policy(ThermalPolicy::Performance).is_ok());
        assert!(crate::thermal::set_thermal_policy(ThermalPolicy::Quiet).is_ok());
    }
}