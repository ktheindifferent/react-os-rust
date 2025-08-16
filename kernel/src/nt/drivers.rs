use super::{NtStatus, object::{Handle, ObjectHeader, ObjectTrait, ObjectType}};
use super::io::{
    DriverObject, DeviceObject, Irp, FileObject, IoStatusBlock, UnicodeString,
    DeviceType, DeviceCharacteristics, DeviceFlags, ProcessorMode, IoManager, IO_MANAGER,
    IRP_MJ_CREATE, IRP_MJ_CLOSE, IRP_MJ_READ, IRP_MJ_WRITE, IRP_MJ_DEVICE_CONTROL
};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::{Vec, self};
use alloc::sync::Arc;
use alloc::boxed::Box;
use alloc::format;
use spin::Mutex;
use lazy_static::lazy_static;
use core::sync::atomic::{AtomicU32, Ordering};

// Driver types
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverType {
    KernelDriver = 1,
    FileSystemDriver = 2,
    Win32ServiceDriver = 3,
    AdapterDriver = 4,
}

// Driver start type
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverStartType {
    BootStart = 0,
    SystemStart = 1,
    AutoStart = 2,
    DemandStart = 3,
    Disabled = 4,
}

// Driver error control
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverErrorControl {
    Ignore = 0,
    Normal = 1,
    Severe = 2,
    Critical = 3,
}

// Driver registration structure
#[derive(Debug, Clone)]
pub struct DriverRegistration {
    pub name: String,
    pub driver_type: DriverType,
    pub start_type: DriverStartType,
    pub error_control: DriverErrorControl,
    pub image_path: String,
    pub service_key: String,
    pub load_order_group: String,
    pub dependencies: Vec<String>,
    pub display_name: String,
    pub description: String,
}

// Device driver trait - defines interface for all device drivers
pub trait DeviceDriver: Send + Sync {
    fn driver_entry(&mut self, driver_object: *mut DriverObject, registry_path: &str) -> NtStatus;
    fn add_device(&mut self, driver_object: *mut DriverObject, physical_device_object: *mut DeviceObject) -> NtStatus;
    fn dispatch_create(&mut self, device_object: *mut DeviceObject, irp: *mut Irp) -> NtStatus;
    fn dispatch_close(&mut self, device_object: *mut DeviceObject, irp: *mut Irp) -> NtStatus;
    fn dispatch_read(&mut self, device_object: *mut DeviceObject, irp: *mut Irp) -> NtStatus;
    fn dispatch_write(&mut self, device_object: *mut DeviceObject, irp: *mut Irp) -> NtStatus;
    fn dispatch_device_control(&mut self, device_object: *mut DeviceObject, irp: *mut Irp) -> NtStatus;
    fn unload(&mut self, driver_object: *mut DriverObject);
    fn get_name(&self) -> &str;
    fn get_version(&self) -> (u32, u32, u32, u32);
}

// Generic device driver implementation
pub struct GenericDriver {
    name: String,
    version: (u32, u32, u32, u32),
    devices: Vec<*mut DeviceObject>,
    registry_path: String,
}

impl GenericDriver {
    pub fn new(name: String, version: (u32, u32, u32, u32)) -> Self {
        Self {
            name,
            version,
            devices: Vec::new(),
            registry_path: String::new(),
        }
    }
}

impl DeviceDriver for GenericDriver {
    fn driver_entry(&mut self, driver_object: *mut DriverObject, registry_path: &str) -> NtStatus {
        self.registry_path = registry_path.to_string();
        
        // Set up driver object dispatch routines
        unsafe {
            if !driver_object.is_null() {
                (*driver_object).major_function[IRP_MJ_CREATE as usize] = Some(generic_dispatch_create);
                (*driver_object).major_function[IRP_MJ_CLOSE as usize] = Some(generic_dispatch_close);
                (*driver_object).major_function[IRP_MJ_READ as usize] = Some(generic_dispatch_read);
                (*driver_object).major_function[IRP_MJ_WRITE as usize] = Some(generic_dispatch_write);
                (*driver_object).major_function[IRP_MJ_DEVICE_CONTROL as usize] = Some(generic_dispatch_device_control);
                (*driver_object).driver_unload = Some(generic_driver_unload);
                
                if let Some(ref mut ext) = (*driver_object).driver_extension.as_mut() {
                    ext.add_device = Some(generic_add_device);
                }
            }
        }
        
        NtStatus::Success
    }

    fn add_device(&mut self, driver_object: *mut DriverObject, physical_device_object: *mut DeviceObject) -> NtStatus {
        // Create functional device object
        match super::io::io_create_device(
            driver_object,
            0, // No device extension for now
            Some(&format!("\\Device\\{}", self.name)),
            DeviceType::FileDeviceUnknown,
            DeviceCharacteristics::empty(),
            false,
        ) {
            Ok(device_object) => {
                self.devices.push(device_object);
                
                // Clear the initializing flag
                unsafe {
                    if !device_object.is_null() {
                        (*device_object).flags &= !DeviceFlags::DO_DEVICE_INITIALIZING;
                    }
                }
                
                NtStatus::Success
            }
            Err(status) => status,
        }
    }

    fn dispatch_create(&mut self, _device_object: *mut DeviceObject, irp: *mut Irp) -> NtStatus {
        // Handle create request
        unsafe {
            if !irp.is_null() {
                (*irp).io_status.status = NtStatus::Success;
                (*irp).io_status.information = 0;
            }
        }
        super::io::io_complete_request(irp, 0);
        NtStatus::Success
    }

    fn dispatch_close(&mut self, _device_object: *mut DeviceObject, irp: *mut Irp) -> NtStatus {
        // Handle close request
        unsafe {
            if !irp.is_null() {
                (*irp).io_status.status = NtStatus::Success;
                (*irp).io_status.information = 0;
            }
        }
        super::io::io_complete_request(irp, 0);
        NtStatus::Success
    }

    fn dispatch_read(&mut self, _device_object: *mut DeviceObject, irp: *mut Irp) -> NtStatus {
        // Handle read request
        unsafe {
            if !irp.is_null() {
                (*irp).io_status.status = NtStatus::Success;
                (*irp).io_status.information = 0;
            }
        }
        super::io::io_complete_request(irp, 0);
        NtStatus::Success
    }

    fn dispatch_write(&mut self, _device_object: *mut DeviceObject, irp: *mut Irp) -> NtStatus {
        // Handle write request
        unsafe {
            if !irp.is_null() {
                (*irp).io_status.status = NtStatus::Success;
                (*irp).io_status.information = 0;
            }
        }
        super::io::io_complete_request(irp, 0);
        NtStatus::Success
    }

    fn dispatch_device_control(&mut self, _device_object: *mut DeviceObject, irp: *mut Irp) -> NtStatus {
        // Handle device control request
        unsafe {
            if !irp.is_null() {
                (*irp).io_status.status = NtStatus::NotImplemented;
                (*irp).io_status.information = 0;
            }
        }
        super::io::io_complete_request(irp, 0);
        NtStatus::NotImplemented
    }

    fn unload(&mut self, _driver_object: *mut DriverObject) {
        // Clean up driver resources
        for device in &self.devices {
            super::io::io_delete_device(*device);
        }
        self.devices.clear();
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_version(&self) -> (u32, u32, u32, u32) {
        self.version
    }
}

// Keyboard driver implementation
pub struct KeyboardDriver {
    generic: GenericDriver,
    device_object: Option<*mut DeviceObject>,
    interrupt_count: AtomicU32,
}

impl KeyboardDriver {
    pub fn new() -> Self {
        Self {
            generic: GenericDriver::new("Keyboard".to_string(), (1, 0, 0, 0)),
            device_object: None,
            interrupt_count: AtomicU32::new(0),
        }
    }
    
    pub fn handle_interrupt(&self) {
        self.interrupt_count.fetch_add(1, Ordering::SeqCst);
        // In a real implementation, we'd read the keyboard scan code and process it
    }
    
    pub fn get_interrupt_count(&self) -> u32 {
        self.interrupt_count.load(Ordering::SeqCst)
    }
}

impl DeviceDriver for KeyboardDriver {
    fn driver_entry(&mut self, driver_object: *mut DriverObject, registry_path: &str) -> NtStatus {
        self.generic.driver_entry(driver_object, registry_path)
    }

    fn add_device(&mut self, driver_object: *mut DriverObject, physical_device_object: *mut DeviceObject) -> NtStatus {
        match super::io::io_create_device(
            driver_object,
            0,
            Some("\\Device\\KeyboardClass0"),
            DeviceType::FileDeviceKeyboard,
            DeviceCharacteristics::empty(),
            false,
        ) {
            Ok(device_object) => {
                self.device_object = Some(device_object);
                
                unsafe {
                    if !device_object.is_null() {
                        (*device_object).flags &= !DeviceFlags::DO_DEVICE_INITIALIZING;
                    }
                }
                
                NtStatus::Success
            }
            Err(status) => status,
        }
    }

    fn dispatch_create(&mut self, device_object: *mut DeviceObject, irp: *mut Irp) -> NtStatus {
        self.generic.dispatch_create(device_object, irp)
    }

    fn dispatch_close(&mut self, device_object: *mut DeviceObject, irp: *mut Irp) -> NtStatus {
        self.generic.dispatch_close(device_object, irp)
    }

    fn dispatch_read(&mut self, _device_object: *mut DeviceObject, irp: *mut Irp) -> NtStatus {
        // Keyboard-specific read handling
        unsafe {
            if !irp.is_null() {
                (*irp).io_status.status = NtStatus::Success;
                (*irp).io_status.information = 0; // No data available for now
            }
        }
        super::io::io_complete_request(irp, 0);
        NtStatus::Success
    }

    fn dispatch_write(&mut self, device_object: *mut DeviceObject, irp: *mut Irp) -> NtStatus {
        // Keyboards typically don't support write
        unsafe {
            if !irp.is_null() {
                (*irp).io_status.status = NtStatus::InvalidDeviceRequest;
                (*irp).io_status.information = 0;
            }
        }
        super::io::io_complete_request(irp, 0);
        NtStatus::InvalidDeviceRequest
    }

    fn dispatch_device_control(&mut self, _device_object: *mut DeviceObject, irp: *mut Irp) -> NtStatus {
        // Handle keyboard-specific IOCTLs
        unsafe {
            if !irp.is_null() {
                // For now, just return not implemented
                (*irp).io_status.status = NtStatus::NotImplemented;
                (*irp).io_status.information = 0;
            }
        }
        super::io::io_complete_request(irp, 0);
        NtStatus::NotImplemented
    }

    fn unload(&mut self, driver_object: *mut DriverObject) {
        if let Some(device) = self.device_object {
            super::io::io_delete_device(device);
            self.device_object = None;
        }
        self.generic.unload(driver_object);
    }

    fn get_name(&self) -> &str {
        "Keyboard"
    }

    fn get_version(&self) -> (u32, u32, u32, u32) {
        (1, 0, 0, 0)
    }
}

// Display driver implementation
pub struct DisplayDriver {
    generic: GenericDriver,
    device_object: Option<*mut DeviceObject>,
    resolution: (u32, u32),
    bits_per_pixel: u32,
    frame_buffer: Option<*mut u8>,
}

impl DisplayDriver {
    pub fn new() -> Self {
        Self {
            generic: GenericDriver::new("Display".to_string(), (1, 0, 0, 0)),
            device_object: None,
            resolution: (80, 25), // VGA text mode
            bits_per_pixel: 4,     // 16 colors
            frame_buffer: None,
        }
    }
    
    pub fn set_resolution(&mut self, width: u32, height: u32, bpp: u32) -> NtStatus {
        self.resolution = (width, height);
        self.bits_per_pixel = bpp;
        NtStatus::Success
    }
    
    pub fn get_resolution(&self) -> (u32, u32, u32) {
        (self.resolution.0, self.resolution.1, self.bits_per_pixel)
    }
}

impl DeviceDriver for DisplayDriver {
    fn driver_entry(&mut self, driver_object: *mut DriverObject, registry_path: &str) -> NtStatus {
        self.generic.driver_entry(driver_object, registry_path)
    }

    fn add_device(&mut self, driver_object: *mut DriverObject, physical_device_object: *mut DeviceObject) -> NtStatus {
        match super::io::io_create_device(
            driver_object,
            0,
            Some("\\Device\\Video0"),
            DeviceType::FileDeviceVideo,
            DeviceCharacteristics::empty(),
            false,
        ) {
            Ok(device_object) => {
                self.device_object = Some(device_object);
                
                unsafe {
                    if !device_object.is_null() {
                        (*device_object).flags &= !DeviceFlags::DO_DEVICE_INITIALIZING;
                    }
                }
                
                NtStatus::Success
            }
            Err(status) => status,
        }
    }

    fn dispatch_create(&mut self, device_object: *mut DeviceObject, irp: *mut Irp) -> NtStatus {
        self.generic.dispatch_create(device_object, irp)
    }

    fn dispatch_close(&mut self, device_object: *mut DeviceObject, irp: *mut Irp) -> NtStatus {
        self.generic.dispatch_close(device_object, irp)
    }

    fn dispatch_read(&mut self, _device_object: *mut DeviceObject, irp: *mut Irp) -> NtStatus {
        // Display drivers typically don't support read
        unsafe {
            if !irp.is_null() {
                (*irp).io_status.status = NtStatus::InvalidDeviceRequest;
                (*irp).io_status.information = 0;
            }
        }
        super::io::io_complete_request(irp, 0);
        NtStatus::InvalidDeviceRequest
    }

    fn dispatch_write(&mut self, _device_object: *mut DeviceObject, irp: *mut Irp) -> NtStatus {
        // Handle display write (drawing) operations
        unsafe {
            if !irp.is_null() {
                (*irp).io_status.status = NtStatus::Success;
                (*irp).io_status.information = 0;
            }
        }
        super::io::io_complete_request(irp, 0);
        NtStatus::Success
    }

    fn dispatch_device_control(&mut self, _device_object: *mut DeviceObject, irp: *mut Irp) -> NtStatus {
        // Handle display-specific IOCTLs (mode setting, etc.)
        unsafe {
            if !irp.is_null() {
                (*irp).io_status.status = NtStatus::NotImplemented;
                (*irp).io_status.information = 0;
            }
        }
        super::io::io_complete_request(irp, 0);
        NtStatus::NotImplemented
    }

    fn unload(&mut self, driver_object: *mut DriverObject) {
        if let Some(device) = self.device_object {
            super::io::io_delete_device(device);
            self.device_object = None;
        }
        self.generic.unload(driver_object);
    }

    fn get_name(&self) -> &str {
        "Display"
    }

    fn get_version(&self) -> (u32, u32, u32, u32) {
        (1, 0, 0, 0)
    }
}

// Storage driver implementation  
pub struct StorageDriver {
    generic: GenericDriver,
    device_object: Option<*mut DeviceObject>,
    sector_size: u32,
    total_sectors: u64,
    read_only: bool,
}

impl StorageDriver {
    pub fn new(sector_size: u32, total_sectors: u64, read_only: bool) -> Self {
        Self {
            generic: GenericDriver::new("Storage".to_string(), (1, 0, 0, 0)),
            device_object: None,
            sector_size,
            total_sectors,
            read_only,
        }
    }
    
    pub fn get_geometry(&self) -> (u32, u64, bool) {
        (self.sector_size, self.total_sectors, self.read_only)
    }
}

impl DeviceDriver for StorageDriver {
    fn driver_entry(&mut self, driver_object: *mut DriverObject, registry_path: &str) -> NtStatus {
        self.generic.driver_entry(driver_object, registry_path)
    }

    fn add_device(&mut self, driver_object: *mut DriverObject, physical_device_object: *mut DeviceObject) -> NtStatus {
        match super::io::io_create_device(
            driver_object,
            0,
            Some("\\Device\\Harddisk0\\Partition0"),
            DeviceType::FileDeviceDisk,
            if self.read_only { DeviceCharacteristics::FILE_READ_ONLY_DEVICE } else { DeviceCharacteristics::empty() },
            false,
        ) {
            Ok(device_object) => {
                self.device_object = Some(device_object);
                
                unsafe {
                    if !device_object.is_null() {
                        (*device_object).flags &= !DeviceFlags::DO_DEVICE_INITIALIZING;
                        (*device_object).sector_size = self.sector_size as u16;
                    }
                }
                
                NtStatus::Success
            }
            Err(status) => status,
        }
    }

    fn dispatch_create(&mut self, device_object: *mut DeviceObject, irp: *mut Irp) -> NtStatus {
        self.generic.dispatch_create(device_object, irp)
    }

    fn dispatch_close(&mut self, device_object: *mut DeviceObject, irp: *mut Irp) -> NtStatus {
        self.generic.dispatch_close(device_object, irp)
    }

    fn dispatch_read(&mut self, _device_object: *mut DeviceObject, irp: *mut Irp) -> NtStatus {
        // Handle storage read operations
        unsafe {
            if !irp.is_null() {
                (*irp).io_status.status = NtStatus::Success;
                (*irp).io_status.information = 0; // Would be actual bytes read
            }
        }
        super::io::io_complete_request(irp, 0);
        NtStatus::Success
    }

    fn dispatch_write(&mut self, _device_object: *mut DeviceObject, irp: *mut Irp) -> NtStatus {
        if self.read_only {
            unsafe {
                if !irp.is_null() {
                    (*irp).io_status.status = NtStatus::AccessDenied;
                    (*irp).io_status.information = 0;
                }
            }
            super::io::io_complete_request(irp, 0);
            return NtStatus::AccessDenied;
        }
        
        // Handle storage write operations
        unsafe {
            if !irp.is_null() {
                (*irp).io_status.status = NtStatus::Success;
                (*irp).io_status.information = 0; // Would be actual bytes written
            }
        }
        super::io::io_complete_request(irp, 0);
        NtStatus::Success
    }

    fn dispatch_device_control(&mut self, _device_object: *mut DeviceObject, irp: *mut Irp) -> NtStatus {
        // Handle storage-specific IOCTLs (geometry queries, etc.)
        unsafe {
            if !irp.is_null() {
                (*irp).io_status.status = NtStatus::NotImplemented;
                (*irp).io_status.information = 0;
            }
        }
        super::io::io_complete_request(irp, 0);
        NtStatus::NotImplemented
    }

    fn unload(&mut self, driver_object: *mut DriverObject) {
        if let Some(device) = self.device_object {
            super::io::io_delete_device(device);
            self.device_object = None;
        }
        self.generic.unload(driver_object);
    }

    fn get_name(&self) -> &str {
        "Storage"
    }

    fn get_version(&self) -> (u32, u32, u32, u32) {
        (1, 0, 0, 0)
    }
}

// Driver Manager - manages all loaded drivers
pub struct DriverManager {
    drivers: BTreeMap<String, Box<dyn DeviceDriver>>,
    driver_objects: BTreeMap<String, *mut DriverObject>,
    load_order: Vec<String>,
    next_driver_id: AtomicU32,
}

impl DriverManager {
    pub fn new() -> Self {
        Self {
            drivers: BTreeMap::new(),
            driver_objects: BTreeMap::new(),
            load_order: Vec::new(),
            next_driver_id: AtomicU32::new(1),
        }
    }
    
    pub fn register_driver(&mut self, mut driver: Box<dyn DeviceDriver>, registration: DriverRegistration) -> NtStatus {
        let driver_name = driver.get_name().to_string();
        
        // Create driver object (simplified)
        let driver_object = Box::into_raw(Box::new(DriverObject {
            header: ObjectHeader::new(ObjectType::Driver),
            type_: 4, // IO_TYPE_DRIVER
            size: core::mem::size_of::<DriverObject>() as u16,
            device_object: core::ptr::null_mut(),
            flags: 0,
            driver_start: core::ptr::null_mut(),
            driver_size: 0,
            driver_section: core::ptr::null_mut(),
            driver_extension: core::ptr::null_mut(),
            driver_name: UnicodeString {
                length: (driver_name.len() * 2) as u16,
                maximum_length: (driver_name.len() * 2) as u16,
                buffer: core::ptr::null_mut(),
            },
            hardware_database: core::ptr::null_mut(),
            fast_io_dispatch: core::ptr::null_mut(),
            driver_init: None,
            driver_start_io: None,
            driver_unload: None,
            major_function: [None; 28],
        }));
        
        // Call driver entry point
        match driver.driver_entry(driver_object, &registration.service_key) {
            NtStatus::Success => {
                self.drivers.insert(driver_name.clone(), driver);
                self.driver_objects.insert(driver_name.clone(), driver_object);
                self.load_order.push(driver_name);
                NtStatus::Success
            }
            error => {
                // Clean up on failure
                unsafe { Box::from_raw(driver_object) };
                error
            }
        }
    }
    
    pub fn unload_driver(&mut self, driver_name: &str) -> NtStatus {
        if let Some(mut driver) = self.drivers.remove(driver_name) {
            if let Some(&driver_object) = self.driver_objects.get(driver_name) {
                driver.unload(driver_object);
                
                // Clean up driver object
                unsafe { Box::from_raw(driver_object) };
                self.driver_objects.remove(driver_name);
            }
            
            self.load_order.retain(|name| name != driver_name);
            NtStatus::Success
        } else {
            NtStatus::ObjectNameNotFound
        }
    }
    
    pub fn get_driver(&self, driver_name: &str) -> Option<&dyn DeviceDriver> {
        self.drivers.get(driver_name).map(|d| d.as_ref())
    }
    
    pub fn get_driver_mut(&mut self, driver_name: &str) -> Option<&mut Box<dyn DeviceDriver>> {
        self.drivers.get_mut(driver_name)
    }
    
    pub fn enumerate_drivers(&self) -> Vec<String> {
        self.load_order.clone()
    }
    
    pub fn load_builtin_drivers(&mut self) -> NtStatus {
        use crate::serial_println;
        
        serial_println!("Loading built-in drivers...");
        
        // Load keyboard driver
        let keyboard_driver = Box::new(KeyboardDriver::new());
        let keyboard_reg = DriverRegistration {
            name: "keyboard".to_string(),
            driver_type: DriverType::KernelDriver,
            start_type: DriverStartType::SystemStart,
            error_control: DriverErrorControl::Normal,
            image_path: "\\SystemRoot\\System32\\drivers\\keyboard.sys".to_string(),
            service_key: "\\Registry\\Machine\\System\\CurrentControlSet\\Services\\keyboard".to_string(),
            load_order_group: "Keyboard Class".to_string(),
            dependencies: alloc::vec!["i8042prt".to_string()],
            display_name: "Keyboard Class Driver".to_string(),
            description: "Standard keyboard class driver".to_string(),
        };
        
        match self.register_driver(keyboard_driver, keyboard_reg) {
            NtStatus::Success => serial_println!("Keyboard driver loaded successfully"),
            error => serial_println!("Failed to load keyboard driver: {:?}", error),
        }
        
        // Load display driver
        let display_driver = Box::new(DisplayDriver::new());
        let display_reg = DriverRegistration {
            name: "display".to_string(),
            driver_type: DriverType::KernelDriver,
            start_type: DriverStartType::SystemStart,
            error_control: DriverErrorControl::Normal,
            image_path: "\\SystemRoot\\System32\\drivers\\vga.sys".to_string(),
            service_key: "\\Registry\\Machine\\System\\CurrentControlSet\\Services\\vga".to_string(),
            load_order_group: "Video".to_string(),
            dependencies: alloc::vec![],
            display_name: "VGA Display Driver".to_string(),
            description: "Standard VGA display driver".to_string(),
        };
        
        match self.register_driver(display_driver, display_reg) {
            NtStatus::Success => serial_println!("Display driver loaded successfully"),
            error => serial_println!("Failed to load display driver: {:?}", error),
        }
        
        // Load storage driver  
        let storage_driver = Box::new(StorageDriver::new(512, 2048576, false)); // 1GB disk
        let storage_reg = DriverRegistration {
            name: "storage".to_string(),
            driver_type: DriverType::KernelDriver,
            start_type: DriverStartType::BootStart,
            error_control: DriverErrorControl::Critical,
            image_path: "\\SystemRoot\\System32\\drivers\\disk.sys".to_string(),
            service_key: "\\Registry\\Machine\\System\\CurrentControlSet\\Services\\disk".to_string(),
            load_order_group: "SCSI miniport".to_string(),
            dependencies: alloc::vec![],
            display_name: "Disk Driver".to_string(),
            description: "Standard disk driver".to_string(),
        };
        
        match self.register_driver(storage_driver, storage_reg) {
            NtStatus::Success => serial_println!("Storage driver loaded successfully"),
            error => serial_println!("Failed to load storage driver: {:?}", error),
        }
        
        serial_println!("Built-in driver loading complete");
        NtStatus::Success
    }
    
    pub fn get_driver_statistics(&self) -> DriverStatistics {
        DriverStatistics {
            total_drivers: self.drivers.len(),
            loaded_drivers: self.drivers.len(),
            failed_drivers: 0,
            driver_names: self.load_order.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DriverStatistics {
    pub total_drivers: usize,
    pub loaded_drivers: usize,
    pub failed_drivers: usize,
    pub driver_names: Vec<String>,
}

// Global driver manager
lazy_static! {
    pub static ref DRIVER_MANAGER: Mutex<DriverManager> = Mutex::new(DriverManager::new());
}

// C-style dispatch functions for compatibility
extern "C" fn generic_dispatch_create(device_object: *mut DeviceObject, irp: *mut Irp) -> NtStatus {
    // This would need to find the appropriate driver and call its dispatch function
    NtStatus::Success
}

extern "C" fn generic_dispatch_close(device_object: *mut DeviceObject, irp: *mut Irp) -> NtStatus {
    NtStatus::Success
}

extern "C" fn generic_dispatch_read(device_object: *mut DeviceObject, irp: *mut Irp) -> NtStatus {
    NtStatus::Success
}

extern "C" fn generic_dispatch_write(device_object: *mut DeviceObject, irp: *mut Irp) -> NtStatus {
    NtStatus::Success
}

extern "C" fn generic_dispatch_device_control(device_object: *mut DeviceObject, irp: *mut Irp) -> NtStatus {
    NtStatus::NotImplemented
}

extern "C" fn generic_driver_unload(driver_object: *mut DriverObject) {
    // This would need to find the appropriate driver and call its unload function
}

extern "C" fn generic_add_device(driver_object: *mut DriverObject, physical_device_object: *mut DeviceObject) -> NtStatus {
    // This would need to find the appropriate driver and call its add_device function
    NtStatus::Success
}

// Public API functions
pub fn load_builtin_drivers() -> NtStatus {
    let mut dm = DRIVER_MANAGER.lock();
    dm.load_builtin_drivers()
}

pub fn register_driver(driver: Box<dyn DeviceDriver>, registration: DriverRegistration) -> NtStatus {
    let mut dm = DRIVER_MANAGER.lock();
    dm.register_driver(driver, registration)
}

pub fn unload_driver(driver_name: &str) -> NtStatus {
    let mut dm = DRIVER_MANAGER.lock();
    dm.unload_driver(driver_name)
}

pub fn enumerate_drivers() -> Vec<String> {
    let dm = DRIVER_MANAGER.lock();
    dm.enumerate_drivers()
}

pub fn get_driver_statistics() -> DriverStatistics {
    let dm = DRIVER_MANAGER.lock();
    dm.get_driver_statistics()
}