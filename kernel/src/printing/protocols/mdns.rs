use alloc::{vec::Vec, string::String};
use super::NetworkPrinterInfo;

const MDNS_PORT: u16 = 5353;
const MDNS_ADDR: &str = "224.0.0.251";

pub fn start_discovery() -> Result<(), &'static str> {
    Ok(())
}

pub fn get_discovered_printers() -> Result<Vec<NetworkPrinterInfo>, &'static str> {
    Ok(Vec::new())
}

pub fn announce_printer(name: &str, service: &str, port: u16) -> Result<(), &'static str> {
    Ok(())
}