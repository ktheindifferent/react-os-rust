use alloc::{vec::Vec, string::String};
use super::NetworkPrinterInfo;

pub fn init_smb_client() -> Result<(), &'static str> {
    Ok(())
}

pub fn discover_smb_printers() -> Result<Vec<NetworkPrinterInfo>, &'static str> {
    Ok(Vec::new())
}

pub fn send_smb_job(printer: &NetworkPrinterInfo, data: &[u8]) -> Result<(), &'static str> {
    Ok(())
}