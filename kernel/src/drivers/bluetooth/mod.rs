pub mod usb;
pub mod uart;
pub mod sdio;
pub mod intel;
pub mod broadcom;
pub mod realtek;

use crate::bluetooth::BluetoothAddress;

pub trait BluetoothDriver: Send + Sync {
    fn init(&mut self) -> Result<(), DriverError>;
    fn reset(&mut self) -> Result<(), DriverError>;
    fn get_address(&self) -> BluetoothAddress;
    fn send_command(&mut self, data: &[u8]) -> Result<(), DriverError>;
    fn send_acl_data(&mut self, data: &[u8]) -> Result<(), DriverError>;
    fn send_sco_data(&mut self, data: &[u8]) -> Result<(), DriverError>;
    fn receive_data(&mut self, buffer: &mut [u8]) -> Result<usize, DriverError>;
    fn load_firmware(&mut self) -> Result<(), DriverError>;
    fn set_power(&mut self, on: bool) -> Result<(), DriverError>;
}

#[derive(Debug)]
pub enum DriverError {
    InitFailed,
    NotFound,
    Unsupported,
    IoError,
    Timeout,
    FirmwareError,
    InvalidResponse,
}

impl From<DriverError> for crate::bluetooth::BluetoothError {
    fn from(err: DriverError) -> Self {
        match err {
            DriverError::InitFailed => crate::bluetooth::BluetoothError::AdapterError,
            DriverError::NotFound => crate::bluetooth::BluetoothError::NoAdapter,
            DriverError::Unsupported => crate::bluetooth::BluetoothError::NotSupported,
            DriverError::IoError => crate::bluetooth::BluetoothError::IoError,
            DriverError::Timeout => crate::bluetooth::BluetoothError::Timeout,
            DriverError::FirmwareError => crate::bluetooth::BluetoothError::AdapterError,
            DriverError::InvalidResponse => crate::bluetooth::BluetoothError::ProtocolError,
        }
    }
}