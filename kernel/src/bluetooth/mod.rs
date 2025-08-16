pub mod core;
pub mod profiles;
pub mod ble;
pub mod security;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;
use spin::RwLock;
use core::sync::atomic::{AtomicU32, Ordering};

pub use core::hci::{HciController, HciCommand, HciEvent};
pub use core::l2cap::{L2capChannel, L2capPacket};
pub use security::{PairingMode, SecurityLevel};
pub use ble::{BleAdvertiser, BleScanner};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BluetoothVersion {
    V1_0,
    V1_1,
    V1_2,
    V2_0,
    V2_1,
    V3_0,
    V4_0,
    V4_1,
    V4_2,
    V5_0,
    V5_1,
    V5_2,
    V5_3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BluetoothAddress([u8; 6]);

impl BluetoothAddress {
    pub const fn new(addr: [u8; 6]) -> Self {
        Self(addr)
    }

    pub fn from_str(s: &str) -> Result<Self, BluetoothError> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 6 {
            return Err(BluetoothError::InvalidAddress);
        }

        let mut addr = [0u8; 6];
        for (i, part) in parts.iter().enumerate() {
            addr[i] = u8::from_str_radix(part, 16)
                .map_err(|_| BluetoothError::InvalidAddress)?;
        }

        Ok(Self(addr))
    }

    pub fn to_string(&self) -> String {
        format!("{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                self.0[0], self.0[1], self.0[2],
                self.0[3], self.0[4], self.0[5])
    }
}

#[derive(Debug, Clone)]
pub struct BluetoothDevice {
    pub address: BluetoothAddress,
    pub name: Option<String>,
    pub class: u32,
    pub rssi: Option<i8>,
    pub paired: bool,
    pub connected: bool,
    pub trusted: bool,
    pub blocked: bool,
    pub profiles: Vec<BluetoothProfile>,
    pub version: BluetoothVersion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BluetoothProfile {
    A2DP,
    AVRCP,
    HFP,
    HSP,
    HID,
    PAN,
    SPP,
    OBEX,
    MAP,
    PBAP,
    BLE,
}

#[derive(Debug)]
pub enum BluetoothError {
    NotSupported,
    NotReady,
    NoAdapter,
    AdapterError,
    InvalidAddress,
    ConnectionFailed,
    AuthenticationFailed,
    PairingFailed,
    Timeout,
    InvalidParameter,
    ResourceBusy,
    NoMemory,
    IoError,
    ProtocolError,
}

pub struct BluetoothAdapter {
    id: u32,
    address: BluetoothAddress,
    name: String,
    powered: bool,
    discoverable: bool,
    discovering: bool,
    devices: RwLock<BTreeMap<BluetoothAddress, BluetoothDevice>>,
    controller: Option<HciController>,
}

impl BluetoothAdapter {
    pub fn new(id: u32, address: BluetoothAddress) -> Self {
        Self {
            id,
            address,
            name: format!("hci{}", id),
            powered: false,
            discoverable: false,
            discovering: false,
            devices: RwLock::new(BTreeMap::new()),
            controller: None,
        }
    }

    pub fn power_on(&mut self) -> Result<(), BluetoothError> {
        if let Some(ref mut controller) = self.controller {
            controller.reset()?;
            controller.set_event_mask(0xFFFFFFFFFFFFFFFF)?;
            controller.set_local_name(&self.name)?;
            self.powered = true;
            Ok(())
        } else {
            Err(BluetoothError::NoAdapter)
        }
    }

    pub fn power_off(&mut self) -> Result<(), BluetoothError> {
        self.powered = false;
        if let Some(ref mut controller) = self.controller {
            controller.reset()?;
        }
        Ok(())
    }

    pub fn start_discovery(&mut self) -> Result<(), BluetoothError> {
        if !self.powered {
            return Err(BluetoothError::NotReady);
        }

        if let Some(ref mut controller) = self.controller {
            controller.inquiry(10, 10)?;
            self.discovering = true;
            Ok(())
        } else {
            Err(BluetoothError::NoAdapter)
        }
    }

    pub fn stop_discovery(&mut self) -> Result<(), BluetoothError> {
        if let Some(ref mut controller) = self.controller {
            controller.cancel_inquiry()?;
            self.discovering = false;
            Ok(())
        } else {
            Err(BluetoothError::NoAdapter)
        }
    }

    pub fn pair_device(&mut self, address: BluetoothAddress) -> Result<(), BluetoothError> {
        if !self.powered {
            return Err(BluetoothError::NotReady);
        }

        // Implement pairing logic
        Ok(())
    }

    pub fn connect_device(&mut self, address: BluetoothAddress) -> Result<(), BluetoothError> {
        if !self.powered {
            return Err(BluetoothError::NotReady);
        }

        // Implement connection logic
        Ok(())
    }

    pub fn disconnect_device(&mut self, address: BluetoothAddress) -> Result<(), BluetoothError> {
        // Implement disconnection logic
        Ok(())
    }

    pub fn get_devices(&self) -> Vec<BluetoothDevice> {
        self.devices.read().values().cloned().collect()
    }

    pub fn get_device(&self, address: BluetoothAddress) -> Option<BluetoothDevice> {
        self.devices.read().get(&address).cloned()
    }
}

pub struct BluetoothManager {
    adapters: RwLock<BTreeMap<u32, BluetoothAdapter>>,
    next_adapter_id: AtomicU32,
}

impl BluetoothManager {
    pub const fn new() -> Self {
        Self {
            adapters: RwLock::new(BTreeMap::new()),
            next_adapter_id: AtomicU32::new(0),
        }
    }

    pub fn register_adapter(&self, address: BluetoothAddress) -> u32 {
        let id = self.next_adapter_id.fetch_add(1, Ordering::SeqCst);
        let adapter = BluetoothAdapter::new(id, address);
        self.adapters.write().insert(id, adapter);
        id
    }

    pub fn unregister_adapter(&self, id: u32) {
        self.adapters.write().remove(&id);
    }

    pub fn get_adapter(&self, id: u32) -> Option<BluetoothAdapter> {
        self.adapters.read().get(&id).cloned()
    }

    pub fn get_default_adapter(&self) -> Option<BluetoothAdapter> {
        self.adapters.read().values().next().cloned()
    }

    pub fn list_adapters(&self) -> Vec<u32> {
        self.adapters.read().keys().cloned().collect()
    }
}

pub static BLUETOOTH_MANAGER: BluetoothManager = BluetoothManager::new();

pub fn init() {
    log::info!("Initializing Bluetooth subsystem");
    
    // Initialize HCI layer
    core::hci::init();
    
    // Initialize L2CAP layer
    core::l2cap::init();
    
    // Initialize security manager
    security::init();
    
    // Initialize BLE subsystem
    ble::init();
    
    // Scan for Bluetooth adapters
    scan_for_adapters();
    
    log::info!("Bluetooth subsystem initialized");
}

fn scan_for_adapters() {
    // Scan for USB Bluetooth adapters
    if let Some(addr) = crate::drivers::bluetooth::usb::scan() {
        BLUETOOTH_MANAGER.register_adapter(addr);
    }
    
    // Scan for UART Bluetooth modules
    if let Some(addr) = crate::drivers::bluetooth::uart::scan() {
        BLUETOOTH_MANAGER.register_adapter(addr);
    }
    
    // Scan for SDIO Bluetooth chips
    if let Some(addr) = crate::drivers::bluetooth::sdio::scan() {
        BLUETOOTH_MANAGER.register_adapter(addr);
    }
}