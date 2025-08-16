pub mod usb;
pub mod ipp;
pub mod lpd;
pub mod smb;
pub mod mdns;

use alloc::{vec::Vec, string::String, collections::BTreeMap};
use spin::RwLock;

pub struct ProtocolManager {
    usb_printers: RwLock<Vec<USBPrinterInfo>>,
    network_printers: RwLock<Vec<NetworkPrinterInfo>>,
    active_connections: RwLock<BTreeMap<u32, PrinterConnection>>,
}

#[derive(Debug, Clone)]
pub struct USBPrinterInfo {
    pub vendor_id: u16,
    pub product_id: u16,
    pub serial: String,
    pub manufacturer: String,
    pub product: String,
    pub device_class: u8,
    pub interface: u8,
    pub endpoint_in: u8,
    pub endpoint_out: u8,
}

#[derive(Debug, Clone)]
pub struct NetworkPrinterInfo {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub protocol: NetworkProtocol,
    pub uri: String,
    pub manufacturer: String,
    pub model: String,
    pub location: String,
    pub info: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkProtocol {
    IPP,
    IPPS,
    LPD,
    SMB,
    Socket,
    HTTP,
}

#[derive(Debug)]
pub enum PrinterConnection {
    USB(USBConnection),
    Network(NetworkConnection),
}

#[derive(Debug)]
pub struct USBConnection {
    device: USBPrinterInfo,
    is_open: bool,
    buffer: Vec<u8>,
}

#[derive(Debug)]
pub struct NetworkConnection {
    printer: NetworkPrinterInfo,
    socket: Option<u32>,
    is_connected: bool,
    buffer: Vec<u8>,
}

impl ProtocolManager {
    pub fn new() -> Self {
        Self {
            usb_printers: RwLock::new(Vec::new()),
            network_printers: RwLock::new(Vec::new()),
            active_connections: RwLock::new(BTreeMap::new()),
        }
    }

    pub fn init_protocols(&mut self) -> Result<(), &'static str> {
        usb::init_usb_subsystem()?;
        ipp::init_ipp_client()?;
        lpd::init_lpd_client()?;
        mdns::start_discovery()?;
        Ok(())
    }

    pub fn discover_usb_printers(&mut self) -> Result<(), &'static str> {
        let printers = usb::scan_usb_printers()?;
        *self.usb_printers.write() = printers;
        Ok(())
    }

    pub fn discover_network_printers(&mut self) -> Result<(), &'static str> {
        let mut printers = Vec::new();
        
        printers.extend(ipp::discover_ipp_printers()?);
        printers.extend(lpd::discover_lpd_printers()?);
        printers.extend(mdns::get_discovered_printers()?);
        
        *self.network_printers.write() = printers;
        Ok(())
    }

    pub fn connect_to_printer(&mut self, printer_id: u32) -> Result<(), &'static str> {
        let subsystem = super::get_subsystem().read();
        if let Some(subsystem) = subsystem.as_ref() {
            if let Some(printer) = subsystem.get_printer(printer_id) {
                let connection = self.create_connection(&printer)?;
                self.active_connections.write().insert(printer_id, connection);
                Ok(())
            } else {
                Err("Printer not found")
            }
        } else {
            Err("Print subsystem not initialized")
        }
    }

    fn create_connection(&self, printer: &super::Printer) -> Result<PrinterConnection, &'static str> {
        if printer.location.starts_with("usb://") {
            let usb_printers = self.usb_printers.read();
            if let Some(usb_info) = usb_printers.first() {
                Ok(PrinterConnection::USB(USBConnection {
                    device: usb_info.clone(),
                    is_open: false,
                    buffer: Vec::new(),
                }))
            } else {
                Err("USB printer not found")
            }
        } else if printer.location.starts_with("ipp://") || printer.location.starts_with("ipps://") {
            Ok(PrinterConnection::Network(NetworkConnection {
                printer: NetworkPrinterInfo {
                    name: printer.name.clone(),
                    host: String::from("localhost"),
                    port: 631,
                    protocol: NetworkProtocol::IPP,
                    uri: printer.location.clone(),
                    manufacturer: String::new(),
                    model: String::new(),
                    location: printer.location.clone(),
                    info: printer.description.clone(),
                },
                socket: None,
                is_connected: false,
                buffer: Vec::new(),
            }))
        } else {
            Err("Unsupported printer protocol")
        }
    }

    pub fn send_data(&mut self, printer_id: u32, data: Vec<u8>) -> Result<(), &'static str> {
        let mut connections = self.active_connections.write();
        if let Some(connection) = connections.get_mut(&printer_id) {
            match connection {
                PrinterConnection::USB(usb_conn) => {
                    usb::send_to_usb_printer(&usb_conn.device, &data)
                }
                PrinterConnection::Network(net_conn) => {
                    match net_conn.printer.protocol {
                        NetworkProtocol::IPP | NetworkProtocol::IPPS => {
                            ipp::send_print_job(&net_conn.printer, &data)
                        }
                        NetworkProtocol::LPD => {
                            lpd::send_lpd_job(&net_conn.printer, &data)
                        }
                        _ => Err("Protocol not implemented")
                    }
                }
            }
        } else {
            self.connect_to_printer(printer_id)?;
            self.send_data(printer_id, data)
        }
    }

    pub fn get_printer_status(&self, printer_id: u32) -> Result<super::PrinterStatus, &'static str> {
        let connections = self.active_connections.read();
        if let Some(connection) = connections.get(&printer_id) {
            match connection {
                PrinterConnection::USB(usb_conn) => {
                    usb::get_usb_printer_status(&usb_conn.device)
                }
                PrinterConnection::Network(net_conn) => {
                    match net_conn.printer.protocol {
                        NetworkProtocol::IPP | NetworkProtocol::IPPS => {
                            ipp::get_printer_status(&net_conn.printer)
                        }
                        _ => Ok(super::PrinterStatus::Idle)
                    }
                }
            }
        } else {
            Ok(super::PrinterStatus::Offline)
        }
    }

    pub fn disconnect_printer(&mut self, printer_id: u32) -> Result<(), &'static str> {
        self.active_connections.write().remove(&printer_id);
        Ok(())
    }
}

pub fn send_data(printer_id: u32, data: Vec<u8>) -> Result<(), &'static str> {
    let subsystem = super::get_subsystem().read();
    if let Some(subsystem) = subsystem.as_ref() {
        subsystem.protocols.send_data(printer_id, data)
    } else {
        Err("Print subsystem not initialized")
    }
}