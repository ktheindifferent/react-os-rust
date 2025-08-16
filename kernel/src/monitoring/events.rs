#![no_std]

use alloc::collections::VecDeque;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    ProcessLifecycle,
    Security,
    Hardware,
    Network,
    FileSystem,
    Power,
    Error,
    Warning,
    Performance,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Clone)]
pub struct SystemEvent {
    pub id: u64,
    pub timestamp: u64,
    pub event_type: EventType,
    pub severity: EventSeverity,
    pub source: String,
    pub description: String,
    pub data: EventData,
}

#[derive(Debug, Clone)]
pub enum EventData {
    ProcessEvent(ProcessEventData),
    SecurityEvent(SecurityEventData),
    HardwareEvent(HardwareEventData),
    NetworkEvent(NetworkEventData),
    FileSystemEvent(FileSystemEventData),
    PowerEvent(PowerEventData),
    ErrorEvent(ErrorEventData),
    PerformanceEvent(PerformanceEventData),
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct ProcessEventData {
    pub pid: u32,
    pub parent_pid: u32,
    pub name: String,
    pub action: ProcessAction,
}

#[derive(Debug, Clone, Copy)]
pub enum ProcessAction {
    Created,
    Terminated,
    Suspended,
    Resumed,
    Crashed,
}

#[derive(Debug, Clone)]
pub struct SecurityEventData {
    pub user_id: u32,
    pub action: SecurityAction,
    pub target: String,
    pub result: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum SecurityAction {
    Login,
    Logout,
    PermissionChange,
    AccessDenied,
    AuthenticationFailed,
}

#[derive(Debug, Clone)]
pub struct HardwareEventData {
    pub device_type: String,
    pub device_id: String,
    pub action: HardwareAction,
}

#[derive(Debug, Clone, Copy)]
pub enum HardwareAction {
    Attached,
    Detached,
    Failed,
    Recovered,
    ThresholdExceeded,
}

#[derive(Debug, Clone)]
pub struct NetworkEventData {
    pub interface: String,
    pub action: NetworkAction,
    pub remote_addr: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum NetworkAction {
    Connected,
    Disconnected,
    LinkUp,
    LinkDown,
    PacketDropped,
}

#[derive(Debug, Clone)]
pub struct FileSystemEventData {
    pub path: String,
    pub action: FileSystemAction,
}

#[derive(Debug, Clone, Copy)]
pub enum FileSystemAction {
    Mounted,
    Unmounted,
    Full,
    CorruptionDetected,
    QuotaExceeded,
}

#[derive(Debug, Clone)]
pub struct PowerEventData {
    pub action: PowerAction,
    pub battery_level: Option<u8>,
}

#[derive(Debug, Clone, Copy)]
pub enum PowerAction {
    Suspend,
    Resume,
    Shutdown,
    Reboot,
    BatteryLow,
    ACConnected,
    ACDisconnected,
}

#[derive(Debug, Clone)]
pub struct ErrorEventData {
    pub error_code: u32,
    pub component: String,
    pub message: String,
    pub recoverable: bool,
}

#[derive(Debug, Clone)]
pub struct PerformanceEventData {
    pub metric: String,
    pub threshold: u64,
    pub actual: u64,
    pub duration_ms: u64,
}

pub struct EventManager {
    events: Mutex<VecDeque<SystemEvent>>,
    event_counter: AtomicU64,
    max_events: usize,
    subscribers: Mutex<Vec<EventSubscriber>>,
}

pub struct EventSubscriber {
    pub id: u64,
    pub event_types: Vec<EventType>,
    pub min_severity: EventSeverity,
    pub callback: fn(&SystemEvent),
}

static EVENT_MANAGER: EventManager = EventManager {
    events: Mutex::new(VecDeque::new()),
    event_counter: AtomicU64::new(0),
    max_events: 0,
    subscribers: Mutex::new(Vec::new()),
};

const MAX_EVENTS: usize = 5000;

pub fn init() {
    // Initialize event storage
    *EVENT_MANAGER.events.lock() = VecDeque::with_capacity(MAX_EVENTS);
}

pub fn emit_event(
    event_type: EventType,
    severity: EventSeverity,
    source: &str,
    description: &str,
    data: EventData,
) -> u64 {
    let id = EVENT_MANAGER.event_counter.fetch_add(1, Ordering::SeqCst);
    let timestamp = crate::timer::get_ticks();
    
    let event = SystemEvent {
        id,
        timestamp,
        event_type,
        severity,
        source: source.to_string(),
        description: description.to_string(),
        data,
    };
    
    // Store event
    {
        let mut events = EVENT_MANAGER.events.lock();
        if events.len() >= MAX_EVENTS {
            events.pop_front();
        }
        events.push_back(event.clone());
    }
    
    // Notify subscribers
    notify_subscribers(&event);
    
    // Log critical events
    if severity == EventSeverity::Critical {
        crate::log_error!("EVENTS", "Critical event: {} - {}", source, description);
    }
    
    id
}

fn notify_subscribers(event: &SystemEvent) {
    let subscribers = EVENT_MANAGER.subscribers.lock();
    
    for subscriber in subscribers.iter() {
        if subscriber.event_types.contains(&event.event_type) &&
           event.severity as u8 <= subscriber.min_severity as u8 {
            (subscriber.callback)(event);
        }
    }
}

pub fn subscribe(
    event_types: Vec<EventType>,
    min_severity: EventSeverity,
    callback: fn(&SystemEvent),
) -> u64 {
    static mut SUBSCRIBER_ID: u64 = 0;
    
    let id = unsafe {
        SUBSCRIBER_ID += 1;
        SUBSCRIBER_ID
    };
    
    let subscriber = EventSubscriber {
        id,
        event_types,
        min_severity,
        callback,
    };
    
    EVENT_MANAGER.subscribers.lock().push(subscriber);
    id
}

pub fn unsubscribe(subscriber_id: u64) {
    let mut subscribers = EVENT_MANAGER.subscribers.lock();
    subscribers.retain(|s| s.id != subscriber_id);
}

// Helper functions for common events
pub fn emit_process_created(pid: u32, parent_pid: u32, name: &str) {
    emit_event(
        EventType::ProcessLifecycle,
        EventSeverity::Info,
        "process",
        &format!("Process {} created", name),
        EventData::ProcessEvent(ProcessEventData {
            pid,
            parent_pid,
            name: name.to_string(),
            action: ProcessAction::Created,
        }),
    );
}

pub fn emit_process_terminated(pid: u32, name: &str) {
    emit_event(
        EventType::ProcessLifecycle,
        EventSeverity::Info,
        "process",
        &format!("Process {} terminated", name),
        EventData::ProcessEvent(ProcessEventData {
            pid,
            parent_pid: 0,
            name: name.to_string(),
            action: ProcessAction::Terminated,
        }),
    );
}

pub fn emit_security_login(user_id: u32, success: bool) {
    let severity = if success {
        EventSeverity::Info
    } else {
        EventSeverity::High
    };
    
    emit_event(
        EventType::Security,
        severity,
        "auth",
        if success { "User login successful" } else { "User login failed" },
        EventData::SecurityEvent(SecurityEventData {
            user_id,
            action: SecurityAction::Login,
            target: String::new(),
            result: success,
        }),
    );
}

pub fn emit_hardware_attached(device_type: &str, device_id: &str) {
    emit_event(
        EventType::Hardware,
        EventSeverity::Info,
        "hardware",
        &format!("{} device attached", device_type),
        EventData::HardwareEvent(HardwareEventData {
            device_type: device_type.to_string(),
            device_id: device_id.to_string(),
            action: HardwareAction::Attached,
        }),
    );
}

pub fn emit_network_connected(interface: &str, remote_addr: Option<&str>) {
    emit_event(
        EventType::Network,
        EventSeverity::Info,
        "network",
        &format!("Network connection established on {}", interface),
        EventData::NetworkEvent(NetworkEventData {
            interface: interface.to_string(),
            action: NetworkAction::Connected,
            remote_addr: remote_addr.map(|s| s.to_string()),
        }),
    );
}

pub fn emit_fs_mounted(path: &str) {
    emit_event(
        EventType::FileSystem,
        EventSeverity::Info,
        "filesystem",
        &format!("File system mounted at {}", path),
        EventData::FileSystemEvent(FileSystemEventData {
            path: path.to_string(),
            action: FileSystemAction::Mounted,
        }),
    );
}

pub fn emit_power_state_change(action: PowerAction) {
    emit_event(
        EventType::Power,
        EventSeverity::Medium,
        "power",
        &format!("Power state change: {:?}", action),
        EventData::PowerEvent(PowerEventData {
            action,
            battery_level: None,
        }),
    );
}

pub fn emit_error(component: &str, error_code: u32, message: &str, recoverable: bool) {
    let severity = if recoverable {
        EventSeverity::Medium
    } else {
        EventSeverity::Critical
    };
    
    emit_event(
        EventType::Error,
        severity,
        component,
        message,
        EventData::ErrorEvent(ErrorEventData {
            error_code,
            component: component.to_string(),
            message: message.to_string(),
            recoverable,
        }),
    );
}

pub fn emit_performance_threshold(metric: &str, threshold: u64, actual: u64) {
    emit_event(
        EventType::Performance,
        EventSeverity::Medium,
        "performance",
        &format!("{} exceeded threshold", metric),
        EventData::PerformanceEvent(PerformanceEventData {
            metric: metric.to_string(),
            threshold,
            actual,
            duration_ms: 0,
        }),
    );
}

pub fn get_recent_events(count: usize) -> Vec<SystemEvent> {
    EVENT_MANAGER
        .events
        .lock()
        .iter()
        .rev()
        .take(count)
        .cloned()
        .collect()
}

pub fn get_events_by_type(event_type: EventType, count: usize) -> Vec<SystemEvent> {
    EVENT_MANAGER
        .events
        .lock()
        .iter()
        .rev()
        .filter(|e| e.event_type == event_type)
        .take(count)
        .cloned()
        .collect()
}

pub fn clear_events() {
    EVENT_MANAGER.events.lock().clear();
}