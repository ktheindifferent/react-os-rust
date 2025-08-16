pub mod iwlwifi;
pub mod realtek;

use alloc::boxed::Box;
use alloc::vec::Vec;

pub enum WifiDriver {
    Intel(iwlwifi::IwlWifi),
    Realtek(realtek::RealtekWifi),
}

impl WifiDriver {
    pub fn probe_and_init(vendor_id: u16, device_id: u16, base_addr: u64) -> Result<Box<Self>, ()> {
        match vendor_id {
            0x8086 => {
                if iwlwifi::probe_iwlwifi_devices().contains(&device_id) {
                    let mut driver = iwlwifi::IwlWifi::new(device_id, base_addr)?;
                    driver.init()?;
                    return Ok(Box::new(WifiDriver::Intel(driver)));
                }
            }
            0x10EC => {
                if realtek::probe_realtek_devices().contains(&device_id) {
                    let mut driver = realtek::RealtekWifi::new(device_id, base_addr)?;
                    driver.init()?;
                    return Ok(Box::new(WifiDriver::Realtek(driver)));
                }
            }
            _ => {}
        }
        
        Err(())
    }
}