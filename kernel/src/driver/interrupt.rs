//! Interrupt Handling Framework for Drivers

use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::String,
    sync::Arc,
    vec::Vec,
};
use core::{
    sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
};
use spin::{Mutex, RwLock};

use super::{Device, DriverError, Result};

/// Interrupt handler function type
pub type InterruptHandler = Box<dyn Fn() -> InterruptReturn + Send + Sync>;

/// Threaded interrupt handler type
pub type ThreadedHandler = Box<dyn Fn() -> InterruptReturn + Send + Sync>;

/// Interrupt return value
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptReturn {
    /// Interrupt was not from this device
    None,
    /// Interrupt was handled
    Handled,
    /// Wake up interrupt thread
    WakeThread,
}

/// Interrupt request line
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Irq(u32);

impl Irq {
    /// Create new IRQ
    pub const fn new(irq: u32) -> Self {
        Self(irq)
    }
    
    /// Get IRQ number
    pub const fn number(&self) -> u32 {
        self.0
    }
}

/// Interrupt flags
#[derive(Debug, Clone, Copy, Default)]
pub struct IrqFlags {
    /// Interrupt is shared
    pub shared: bool,
    /// Use threaded handler
    pub threaded: bool,
    /// Interrupt is edge triggered
    pub edge_triggered: bool,
    /// Interrupt is active low
    pub active_low: bool,
    /// Disable interrupt during handler
    pub oneshot: bool,
    /// Can wake system from sleep
    pub can_wake: bool,
    /// No automatic enable after handler
    pub no_autoen: bool,
}

/// Interrupt descriptor
pub struct IrqDesc {
    /// IRQ number
    irq: Irq,
    /// Interrupt name
    name: String,
    /// Device owning this interrupt
    device: Option<Arc<Device>>,
    /// Interrupt handlers
    handlers: Vec<IrqHandler>,
    /// Interrupt flags
    flags: IrqFlags,
    /// Interrupt enabled
    enabled: AtomicBool,
    /// Interrupt pending
    pending: AtomicBool,
    /// Interrupt count
    count: AtomicU64,
    /// Spurious interrupt count
    spurious: AtomicU32,
    /// CPU affinity mask
    affinity: AtomicU64,
}

/// Individual interrupt handler
struct IrqHandler {
    handler: InterruptHandler,
    thread_fn: Option<ThreadedHandler>,
    device: Arc<Device>,
    name: String,
}

/// MSI/MSI-X descriptor
pub struct MsiDesc {
    /// MSI vector
    vector: u32,
    /// MSI address
    address: u64,
    /// MSI data
    data: u32,
    /// Is MSI-X
    is_msix: bool,
    /// Entry number for MSI-X
    entry: Option<u32>,
}

/// Interrupt controller operations
pub trait InterruptController: Send + Sync {
    /// Enable interrupt
    fn enable(&self, irq: Irq) -> Result<()>;
    
    /// Disable interrupt
    fn disable(&self, irq: Irq) -> Result<()>;
    
    /// Mask interrupt
    fn mask(&self, irq: Irq) -> Result<()>;
    
    /// Unmask interrupt
    fn unmask(&self, irq: Irq) -> Result<()>;
    
    /// Set interrupt affinity
    fn set_affinity(&self, irq: Irq, cpumask: u64) -> Result<()>;
    
    /// Acknowledge interrupt
    fn ack(&self, irq: Irq) -> Result<()>;
    
    /// End of interrupt
    fn eoi(&self, irq: Irq) -> Result<()>;
}

/// Global interrupt manager
pub struct InterruptManager {
    /// Interrupt descriptors
    irq_descs: RwLock<BTreeMap<Irq, Arc<Mutex<IrqDesc>>>>,
    /// MSI descriptors
    msi_descs: RwLock<BTreeMap<u32, MsiDesc>>,
    /// Interrupt controllers
    controllers: RwLock<Vec<Arc<dyn InterruptController>>>,
    /// Thread pool for threaded handlers
    thread_pool: Mutex<Option<WorkQueue>>,
    /// Statistics
    stats: InterruptStats,
}

/// Interrupt statistics
struct InterruptStats {
    total_interrupts: AtomicU64,
    handled_interrupts: AtomicU64,
    spurious_interrupts: AtomicU64,
    threaded_interrupts: AtomicU64,
}

/// Work queue for threaded interrupts
struct WorkQueue {
    items: Mutex<Vec<WorkItem>>,
    workers: Vec<WorkerThread>,
}

/// Work item for threaded handler
struct WorkItem {
    handler: ThreadedHandler,
    device: Arc<Device>,
}

/// Worker thread for handling threaded interrupts
struct WorkerThread {
    id: u32,
    running: Arc<AtomicBool>,
}

impl InterruptManager {
    /// Create new interrupt manager
    pub const fn new() -> Self {
        Self {
            irq_descs: RwLock::new(BTreeMap::new()),
            msi_descs: RwLock::new(BTreeMap::new()),
            controllers: RwLock::new(Vec::new()),
            thread_pool: Mutex::new(None),
            stats: InterruptStats {
                total_interrupts: AtomicU64::new(0),
                handled_interrupts: AtomicU64::new(0),
                spurious_interrupts: AtomicU64::new(0),
                threaded_interrupts: AtomicU64::new(0),
            },
        }
    }
    
    /// Register interrupt controller
    pub fn register_controller(&self, controller: Arc<dyn InterruptController>) {
        self.controllers.write().push(controller);
    }
    
    /// Request interrupt
    pub fn request_irq(
        &self,
        irq: Irq,
        handler: InterruptHandler,
        flags: IrqFlags,
        name: String,
        device: Arc<Device>,
    ) -> Result<()> {
        let mut irq_descs = self.irq_descs.write();
        
        let desc = irq_descs.entry(irq).or_insert_with(|| {
            Arc::new(Mutex::new(IrqDesc {
                irq,
                name: name.clone(),
                device: Some(device.clone()),
                handlers: Vec::new(),
                flags,
                enabled: AtomicBool::new(false),
                pending: AtomicBool::new(false),
                count: AtomicU64::new(0),
                spurious: AtomicU32::new(0),
                affinity: AtomicU64::new(0xFFFFFFFFFFFFFFFF), // All CPUs
            }))
        });
        
        let mut desc = desc.lock();
        
        // Check if sharing is allowed
        if !desc.handlers.is_empty() && !flags.shared {
            return Err(DriverError::ResourceConflict);
        }
        
        // Add handler
        desc.handlers.push(IrqHandler {
            handler,
            thread_fn: None,
            device,
            name,
        });
        
        // Enable interrupt
        self.enable_irq(irq)?;
        
        Ok(())
    }
    
    /// Request threaded interrupt
    pub fn request_threaded_irq(
        &self,
        irq: Irq,
        handler: InterruptHandler,
        thread_fn: ThreadedHandler,
        flags: IrqFlags,
        name: String,
        device: Arc<Device>,
    ) -> Result<()> {
        let mut irq_descs = self.irq_descs.write();
        
        let desc = irq_descs.entry(irq).or_insert_with(|| {
            Arc::new(Mutex::new(IrqDesc {
                irq,
                name: name.clone(),
                device: Some(device.clone()),
                handlers: Vec::new(),
                flags: IrqFlags { threaded: true, ..flags },
                enabled: AtomicBool::new(false),
                pending: AtomicBool::new(false),
                count: AtomicU64::new(0),
                spurious: AtomicU32::new(0),
                affinity: AtomicU64::new(0xFFFFFFFFFFFFFFFF),
            }))
        });
        
        let mut desc = desc.lock();
        
        // Add handler with thread function
        desc.handlers.push(IrqHandler {
            handler,
            thread_fn: Some(thread_fn),
            device,
            name,
        });
        
        // Enable interrupt
        self.enable_irq(irq)?;
        
        Ok(())
    }
    
    /// Free interrupt
    pub fn free_irq(&self, irq: Irq, device: &Device) -> Result<()> {
        let irq_descs = self.irq_descs.read();
        
        if let Some(desc) = irq_descs.get(&irq) {
            let mut desc = desc.lock();
            
            // Remove handlers for this device
            desc.handlers.retain(|h| h.device.id() != device.id());
            
            // Disable if no more handlers
            if desc.handlers.is_empty() {
                self.disable_irq(irq)?;
            }
            
            Ok(())
        } else {
            Err(DriverError::NotFound)
        }
    }
    
    /// Enable interrupt
    pub fn enable_irq(&self, irq: Irq) -> Result<()> {
        let controllers = self.controllers.read();
        
        for controller in controllers.iter() {
            controller.enable(irq)?;
        }
        
        if let Some(desc) = self.irq_descs.read().get(&irq) {
            desc.lock().enabled.store(true, Ordering::Release);
        }
        
        Ok(())
    }
    
    /// Disable interrupt
    pub fn disable_irq(&self, irq: Irq) -> Result<()> {
        let controllers = self.controllers.read();
        
        for controller in controllers.iter() {
            controller.disable(irq)?;
        }
        
        if let Some(desc) = self.irq_descs.read().get(&irq) {
            desc.lock().enabled.store(false, Ordering::Release);
        }
        
        Ok(())
    }
    
    /// Disable interrupt (wait for completion)
    pub fn disable_irq_sync(&self, irq: Irq) -> Result<()> {
        self.disable_irq(irq)?;
        
        // Wait for any running handlers to complete
        while self.is_irq_in_progress(irq) {
            core::hint::spin_loop();
        }
        
        Ok(())
    }
    
    /// Check if interrupt is in progress
    fn is_irq_in_progress(&self, irq: Irq) -> bool {
        if let Some(desc) = self.irq_descs.read().get(&irq) {
            desc.lock().pending.load(Ordering::Acquire)
        } else {
            false
        }
    }
    
    /// Handle interrupt (called from low-level handler)
    pub fn handle_irq(&self, irq: Irq) -> InterruptReturn {
        self.stats.total_interrupts.fetch_add(1, Ordering::Relaxed);
        
        let irq_descs = self.irq_descs.read();
        
        if let Some(desc) = irq_descs.get(&irq) {
            let mut desc = desc.lock();
            
            if !desc.enabled.load(Ordering::Acquire) {
                return InterruptReturn::None;
            }
            
            desc.pending.store(true, Ordering::Release);
            desc.count.fetch_add(1, Ordering::Relaxed);
            
            let mut handled = false;
            let mut wake_thread = false;
            
            // Call all handlers
            for handler in &desc.handlers {
                match (handler.handler)() {
                    InterruptReturn::Handled => {
                        handled = true;
                        self.stats.handled_interrupts.fetch_add(1, Ordering::Relaxed);
                    }
                    InterruptReturn::WakeThread => {
                        handled = true;
                        wake_thread = true;
                        
                        // Queue threaded handler
                        if let Some(ref thread_fn) = handler.thread_fn {
                            self.queue_threaded_handler(
                                thread_fn.clone(),
                                handler.device.clone(),
                            );
                            self.stats.threaded_interrupts.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    InterruptReturn::None => {}
                }
            }
            
            if !handled {
                desc.spurious.fetch_add(1, Ordering::Relaxed);
                self.stats.spurious_interrupts.fetch_add(1, Ordering::Relaxed);
            }
            
            desc.pending.store(false, Ordering::Release);
            
            if wake_thread {
                InterruptReturn::WakeThread
            } else if handled {
                InterruptReturn::Handled
            } else {
                InterruptReturn::None
            }
        } else {
            InterruptReturn::None
        }
    }
    
    /// Queue threaded handler for execution
    fn queue_threaded_handler(&self, handler: ThreadedHandler, device: Arc<Device>) {
        // Would queue to thread pool
        // For now, just execute directly (not ideal)
        (handler)();
    }
    
    /// Set interrupt affinity
    pub fn set_irq_affinity(&self, irq: Irq, cpumask: u64) -> Result<()> {
        let controllers = self.controllers.read();
        
        for controller in controllers.iter() {
            controller.set_affinity(irq, cpumask)?;
        }
        
        if let Some(desc) = self.irq_descs.read().get(&irq) {
            desc.lock().affinity.store(cpumask, Ordering::Release);
        }
        
        Ok(())
    }
    
    /// Allocate MSI vectors
    pub fn alloc_msi_vectors(&self, device: &Device, count: u32) -> Result<Vec<u32>> {
        let mut vectors = Vec::new();
        let mut msi_descs = self.msi_descs.write();
        
        // Find free vectors
        let mut next_vector = 0;
        for _ in 0..count {
            while msi_descs.contains_key(&next_vector) {
                next_vector += 1;
            }
            
            vectors.push(next_vector);
            msi_descs.insert(next_vector, MsiDesc {
                vector: next_vector,
                address: 0xFEE00000, // Default MSI address
                data: next_vector,
                is_msix: false,
                entry: None,
            });
            
            next_vector += 1;
        }
        
        Ok(vectors)
    }
    
    /// Free MSI vectors
    pub fn free_msi_vectors(&self, vectors: &[u32]) -> Result<()> {
        let mut msi_descs = self.msi_descs.write();
        
        for vector in vectors {
            msi_descs.remove(vector);
        }
        
        Ok(())
    }
    
    /// Get interrupt statistics
    pub fn statistics(&self) -> InterruptStatistics {
        InterruptStatistics {
            total_interrupts: self.stats.total_interrupts.load(Ordering::Relaxed),
            handled_interrupts: self.stats.handled_interrupts.load(Ordering::Relaxed),
            spurious_interrupts: self.stats.spurious_interrupts.load(Ordering::Relaxed),
            threaded_interrupts: self.stats.threaded_interrupts.load(Ordering::Relaxed),
            registered_irqs: self.irq_descs.read().len() as u32,
        }
    }
}

/// Interrupt statistics
#[derive(Debug, Clone, Copy)]
pub struct InterruptStatistics {
    pub total_interrupts: u64,
    pub handled_interrupts: u64,
    pub spurious_interrupts: u64,
    pub threaded_interrupts: u64,
    pub registered_irqs: u32,
}

/// Global interrupt manager instance
static INTERRUPT_MANAGER: InterruptManager = InterruptManager::new();

/// Get global interrupt manager
pub fn interrupt_manager() -> &'static InterruptManager {
    &INTERRUPT_MANAGER
}

/// Deferred work for bottom-half processing
pub struct DeferredWork {
    work_fn: Box<dyn Fn() + Send + Sync>,
    device: Arc<Device>,
}

impl DeferredWork {
    /// Create new deferred work
    pub fn new<F>(work_fn: F, device: Arc<Device>) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        Self {
            work_fn: Box::new(work_fn),
            device,
        }
    }
    
    /// Schedule work for execution
    pub fn schedule(&self) {
        // Would queue to work queue
        (self.work_fn)();
    }
}

/// Tasklet for lightweight bottom-half processing
pub struct Tasklet {
    func: Box<dyn Fn() + Send + Sync>,
    scheduled: AtomicBool,
}

impl Tasklet {
    /// Create new tasklet
    pub fn new<F>(func: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        Self {
            func: Box::new(func),
            scheduled: AtomicBool::new(false),
        }
    }
    
    /// Schedule tasklet
    pub fn schedule(&self) {
        if !self.scheduled.swap(true, Ordering::AcqRel) {
            // Would add to tasklet queue
            (self.func)();
            self.scheduled.store(false, Ordering::Release);
        }
    }
    
    /// Kill tasklet
    pub fn kill(&self) {
        self.scheduled.store(false, Ordering::Release);
    }
}

/// Helper macros for interrupt handling
#[macro_export]
macro_rules! request_irq {
    ($irq:expr, $handler:expr, $flags:expr, $name:expr, $dev:expr) => {
        $crate::interrupt_manager().request_irq(
            $irq,
            Box::new($handler),
            $flags,
            $name.into(),
            $dev,
        )
    };
}

#[macro_export]
macro_rules! request_threaded_irq {
    ($irq:expr, $handler:expr, $thread:expr, $flags:expr, $name:expr, $dev:expr) => {
        $crate::interrupt_manager().request_threaded_irq(
            $irq,
            Box::new($handler),
            Box::new($thread),
            $flags,
            $name.into(),
            $dev,
        )
    };
}