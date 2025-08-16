//! Device Model - Core device tree representation and management

use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::{String, ToString},
    sync::{Arc, Weak},
    vec::Vec,
};
use core::{
    fmt,
    sync::atomic::{AtomicU32, AtomicU64, Ordering},
};
use spin::{Mutex, RwLock};

use super::{Driver, DriverError, Result};

/// Unique device identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeviceId(u64);

impl DeviceId {
    /// Generate new unique device ID
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
    
    /// Create from raw value (for persistent IDs)
    pub const fn from_raw(id: u64) -> Self {
        Self(id)
    }
    
    /// Get raw value
    pub const fn as_raw(&self) -> u64 {
        self.0
    }
}

/// Device class for categorization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceClass {
    // Storage devices
    BlockDevice,
    CharDevice,
    NvmeDevice,
    ScsiDevice,
    
    // Network devices
    NetworkInterface,
    WirelessInterface,
    
    // Display/Graphics
    DisplayController,
    GraphicsAdapter,
    
    // Input devices
    Keyboard,
    Mouse,
    Touchpad,
    Touchscreen,
    
    // Audio devices
    AudioController,
    AudioCodec,
    
    // Bus controllers
    PciBridge,
    UsbController,
    I2cController,
    SpiController,
    
    // System devices
    SystemController,
    InterruptController,
    DmaController,
    TimerDevice,
    
    // Generic
    Unknown,
    Platform,
    Virtual,
}

/// Device state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceState {
    Uninitialized,
    Probing,
    Bound,
    Active,
    Suspended,
    Failed,
    Removed,
}

/// Device resources
#[derive(Debug, Clone)]
pub enum DeviceResource {
    Memory {
        base: u64,
        size: usize,
        flags: MemoryFlags,
    },
    Io {
        base: u16,
        size: u16,
    },
    Interrupt {
        irq: u32,
        flags: InterruptFlags,
    },
    Dma {
        channel: u32,
        mask: u64,
    },
}

/// Memory resource flags
#[derive(Debug, Clone, Copy)]
pub struct MemoryFlags {
    pub prefetchable: bool,
    pub cacheable: bool,
    pub readonly: bool,
    pub mmio: bool,
}

/// Interrupt flags
#[derive(Debug, Clone, Copy)]
pub struct InterruptFlags {
    pub edge_triggered: bool,
    pub active_low: bool,
    pub shared: bool,
    pub msi: bool,
}

/// Device properties for configuration
pub struct DeviceProperty {
    pub name: String,
    pub value: PropertyValue,
}

/// Property values
pub enum PropertyValue {
    String(String),
    U32(u32),
    U64(u64),
    Bool(bool),
    Binary(Vec<u8>),
}

/// Core device structure
pub struct Device {
    /// Unique device ID
    id: DeviceId,
    
    /// Device name
    name: String,
    
    /// Device class
    class: DeviceClass,
    
    /// Current state
    state: RwLock<DeviceState>,
    
    /// Parent device (weak reference to prevent cycles)
    parent: RwLock<Option<Weak<Device>>>,
    
    /// Child devices
    children: RwLock<Vec<Arc<Device>>>,
    
    /// Bound driver name
    driver: RwLock<Option<String>>,
    
    /// Device resources
    resources: RwLock<Vec<DeviceResource>>,
    
    /// Device properties
    properties: RwLock<BTreeMap<String, PropertyValue>>,
    
    /// Reference count
    refcount: AtomicU32,
    
    /// Device-specific data
    private_data: Mutex<Option<Box<dyn Any + Send + Sync>>>,
    
    /// Device capabilities
    capabilities: DeviceCapabilities,
}

/// Device capabilities
#[derive(Debug, Clone, Copy, Default)]
pub struct DeviceCapabilities {
    pub dma_capable: bool,
    pub bus_master: bool,
    pub wake_capable: bool,
    pub power_manageable: bool,
    pub hot_pluggable: bool,
    pub iommu_mapped: bool,
}

impl Device {
    /// Create new device
    pub fn new(name: String, class: DeviceClass) -> Self {
        Self {
            id: DeviceId::new(),
            name,
            class,
            state: RwLock::new(DeviceState::Uninitialized),
            parent: RwLock::new(None),
            children: RwLock::new(Vec::new()),
            driver: RwLock::new(None),
            resources: RwLock::new(Vec::new()),
            properties: RwLock::new(BTreeMap::new()),
            refcount: AtomicU32::new(1),
            private_data: Mutex::new(None),
            capabilities: DeviceCapabilities::default(),
        }
    }
    
    /// Get device ID
    pub fn id(&self) -> DeviceId {
        self.id
    }
    
    /// Get device name
    pub fn name(&self) -> &str {
        &self.name
    }
    
    /// Get device class
    pub fn class(&self) -> DeviceClass {
        self.class
    }
    
    /// Get current state
    pub fn state(&self) -> DeviceState {
        *self.state.read()
    }
    
    /// Set device state
    pub fn set_state(&self, state: DeviceState) {
        *self.state.write() = state;
    }
    
    /// Check if device is bound to a driver
    pub fn is_bound(&self) -> bool {
        self.driver.read().is_some()
    }
    
    /// Get driver name if bound
    pub fn driver_name(&self) -> Option<String> {
        self.driver.read().clone()
    }
    
    /// Bind driver to device
    pub fn bind_driver(&self, driver_name: String) -> Result<()> {
        let mut driver = self.driver.write();
        
        if driver.is_some() {
            return Err(DriverError::AlreadyExists);
        }
        
        *driver = Some(driver_name);
        self.set_state(DeviceState::Bound);
        
        Ok(())
    }
    
    /// Unbind driver from device
    pub fn unbind_driver(&self) -> Result<()> {
        let mut driver = self.driver.write();
        
        if driver.is_none() {
            return Err(DriverError::NotFound);
        }
        
        *driver = None;
        self.set_state(DeviceState::Uninitialized);
        
        Ok(())
    }
    
    /// Set parent device
    pub fn set_parent(&self, parent: Weak<Device>) {
        *self.parent.write() = Some(parent);
    }
    
    /// Get parent device
    pub fn parent(&self) -> Option<Arc<Device>> {
        self.parent.read().as_ref()?.upgrade()
    }
    
    /// Add child device
    pub fn add_child(&self, child: Arc<Device>) {
        child.set_parent(Arc::downgrade(&Arc::new(self.clone())));
        self.children.write().push(child);
    }
    
    /// Remove child device
    pub fn remove_child(&self, id: DeviceId) -> Result<()> {
        let mut children = self.children.write();
        
        if let Some(pos) = children.iter().position(|c| c.id() == id) {
            children.remove(pos);
            Ok(())
        } else {
            Err(DriverError::NotFound)
        }
    }
    
    /// Get children
    pub fn children(&self) -> Vec<Arc<Device>> {
        self.children.read().clone()
    }
    
    /// Add resource
    pub fn add_resource(&self, resource: DeviceResource) {
        self.resources.write().push(resource);
    }
    
    /// Get resources
    pub fn resources(&self) -> Vec<DeviceResource> {
        self.resources.read().clone()
    }
    
    /// Find resource by type
    pub fn find_resource<F>(&self, predicate: F) -> Option<DeviceResource>
    where
        F: Fn(&DeviceResource) -> bool,
    {
        self.resources.read().iter().find(|r| predicate(r)).cloned()
    }
    
    /// Set property
    pub fn set_property(&self, name: String, value: PropertyValue) {
        self.properties.write().insert(name, value);
    }
    
    /// Get property
    pub fn get_property(&self, name: &str) -> Option<PropertyValue> {
        self.properties.read().get(name).cloned()
    }
    
    /// Set private data
    pub fn set_private_data<T: Any + Send + Sync + 'static>(&self, data: T) {
        *self.private_data.lock() = Some(Box::new(data));
    }
    
    /// Get private data
    pub fn get_private_data<T: Any + Send + Sync + 'static>(&self) -> Option<Box<T>> {
        let mut data = self.private_data.lock();
        
        if let Some(any_box) = data.take() {
            if let Ok(concrete) = any_box.downcast::<T>() {
                return Some(concrete);
            }
        }
        
        None
    }
    
    /// Increment reference count
    pub fn get(&self) {
        self.refcount.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Decrement reference count
    pub fn put(&self) {
        self.refcount.fetch_sub(1, Ordering::Relaxed);
    }
    
    /// Get reference count
    pub fn refcount(&self) -> u32 {
        self.refcount.load(Ordering::Relaxed)
    }
    
    /// Get device capabilities
    pub fn capabilities(&self) -> &DeviceCapabilities {
        &self.capabilities
    }
    
    /// Walk device tree from this node
    pub fn walk_tree<F>(&self, mut visitor: F) -> Result<()>
    where
        F: FnMut(&Device) -> Result<()>,
    {
        visitor(self)?;
        
        for child in self.children() {
            child.walk_tree(&mut visitor)?;
        }
        
        Ok(())
    }
    
    /// Find device in tree by ID
    pub fn find_device(&self, id: DeviceId) -> Option<Arc<Device>> {
        if self.id == id {
            return Some(Arc::new(self.clone()));
        }
        
        for child in self.children() {
            if let Some(found) = child.find_device(id) {
                return Some(found);
            }
        }
        
        None
    }
    
    /// Get device path in tree
    pub fn device_path(&self) -> String {
        let mut path = Vec::new();
        let mut current = Some(Arc::new(self.clone()));
        
        while let Some(dev) = current {
            path.push(dev.name().to_string());
            current = dev.parent();
        }
        
        path.reverse();
        path.join("/")
    }
}

impl Clone for Device {
    fn clone(&self) -> Self {
        self.get(); // Increment refcount
        
        Self {
            id: self.id,
            name: self.name.clone(),
            class: self.class,
            state: RwLock::new(*self.state.read()),
            parent: RwLock::new(self.parent.read().clone()),
            children: RwLock::new(self.children.read().clone()),
            driver: RwLock::new(self.driver.read().clone()),
            resources: RwLock::new(self.resources.read().clone()),
            properties: RwLock::new(self.properties.read().clone()),
            refcount: AtomicU32::new(self.refcount.load(Ordering::Relaxed)),
            private_data: Mutex::new(None), // Don't clone private data
            capabilities: self.capabilities,
        }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        // Cleanup when last reference is dropped
        if self.refcount.load(Ordering::Relaxed) == 0 {
            // Unbind driver if bound
            let _ = self.unbind_driver();
            
            // Clear children
            self.children.write().clear();
        }
    }
}

impl fmt::Debug for Device {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Device")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("class", &self.class)
            .field("state", &self.state())
            .field("driver", &self.driver_name())
            .field("children", &self.children().len())
            .finish()
    }
}

impl Clone for PropertyValue {
    fn clone(&self) -> Self {
        match self {
            Self::String(s) => Self::String(s.clone()),
            Self::U32(v) => Self::U32(*v),
            Self::U64(v) => Self::U64(*v),
            Self::Bool(v) => Self::Bool(*v),
            Self::Binary(v) => Self::Binary(v.clone()),
        }
    }
}

/// Device builder for convenient construction
pub struct DeviceBuilder {
    device: Device,
}

impl DeviceBuilder {
    /// Create new device builder
    pub fn new(name: String, class: DeviceClass) -> Self {
        Self {
            device: Device::new(name, class),
        }
    }
    
    /// Set capabilities
    pub fn capabilities(mut self, caps: DeviceCapabilities) -> Self {
        self.device.capabilities = caps;
        self
    }
    
    /// Add resource
    pub fn resource(self, resource: DeviceResource) -> Self {
        self.device.add_resource(resource);
        self
    }
    
    /// Add property
    pub fn property(self, name: String, value: PropertyValue) -> Self {
        self.device.set_property(name, value);
        self
    }
    
    /// Build device
    pub fn build(self) -> Device {
        self.device
    }
}

/// Device tree walker for tree operations
pub struct DeviceTreeWalker;

impl DeviceTreeWalker {
    /// Count devices in tree
    pub fn count_devices(root: &Device) -> usize {
        let mut count = 1;
        
        for child in root.children() {
            count += Self::count_devices(&child);
        }
        
        count
    }
    
    /// Find devices by class
    pub fn find_by_class(root: &Device, class: DeviceClass) -> Vec<Arc<Device>> {
        let mut devices = Vec::new();
        
        if root.class() == class {
            devices.push(Arc::new(root.clone()));
        }
        
        for child in root.children() {
            devices.extend(Self::find_by_class(&child, class));
        }
        
        devices
    }
    
    /// Find devices by driver
    pub fn find_by_driver(root: &Device, driver_name: &str) -> Vec<Arc<Device>> {
        let mut devices = Vec::new();
        
        if root.driver_name().as_deref() == Some(driver_name) {
            devices.push(Arc::new(root.clone()));
        }
        
        for child in root.children() {
            devices.extend(Self::find_by_driver(&child, driver_name));
        }
        
        devices
    }
    
    /// Print device tree
    pub fn print_tree(root: &Device, indent: usize) {
        let spaces = " ".repeat(indent);
        println!("{}[{}] {} ({:?})", spaces, root.id().as_raw(), root.name(), root.class());
        
        if let Some(driver) = root.driver_name() {
            println!("{}  Driver: {}", spaces, driver);
        }
        
        println!("{}  State: {:?}", spaces, root.state());
        
        for child in root.children() {
            Self::print_tree(&child, indent + 2);
        }
    }
}