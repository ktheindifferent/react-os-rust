use alloc::{vec::Vec, string::{String, ToString}, format};
use super::NetworkPrinterInfo;

const LPD_PORT: u16 = 515;

pub fn init_lpd_client() -> Result<(), &'static str> {
    Ok(())
}

pub fn discover_lpd_printers() -> Result<Vec<NetworkPrinterInfo>, &'static str> {
    Ok(Vec::new())
}

pub fn send_lpd_job(printer: &NetworkPrinterInfo, data: &[u8]) -> Result<(), &'static str> {
    let control_file = build_control_file("user", "Print Job", data.len());
    send_lpd_command(printer, b"\x02", &printer.name)?;
    send_lpd_data(printer, &control_file)?;
    send_lpd_data(printer, data)?;
    Ok(())
}

fn build_control_file(user: &str, title: &str, size: usize) -> Vec<u8> {
    let mut cf = String::new();
    cf.push_str(&format!("H{}\n", "localhost"));
    cf.push_str(&format!("P{}\n", user));
    cf.push_str(&format!("J{}\n", title));
    cf.push_str(&format!("ldfA000localhost\n"));
    cf.push_str(&format!("UdfA000localhost\n"));
    cf.push_str(&format!("N{}\n", title));
    cf.into_bytes()
}

fn send_lpd_command(printer: &NetworkPrinterInfo, command: &[u8], queue: &str) -> Result<(), &'static str> {
    Ok(())
}

fn send_lpd_data(printer: &NetworkPrinterInfo, data: &[u8]) -> Result<(), &'static str> {
    Ok(())
}