#![no_std]

use alloc::collections::VecDeque;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use core::fmt;
use spin::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
    Fatal = 5,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Fatal => "FATAL",
        }
    }
}

#[derive(Clone)]
pub struct LogEntry {
    pub timestamp: u64,
    pub level: LogLevel,
    pub cpu_id: u32,
    pub process_id: u32,
    pub thread_id: u32,
    pub category: String,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<u32>,
}

pub struct RingBuffer<T> {
    buffer: VecDeque<T>,
    capacity: usize,
}

impl<T> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, item: T) {
        if self.buffer.len() >= self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back(item);
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.buffer.iter()
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

pub struct Logger {
    kernel_logs: Mutex<RingBuffer<LogEntry>>,
    user_logs: Mutex<RingBuffer<LogEntry>>,
    min_level: AtomicUsize,
    log_count: AtomicU64,
    dropped_count: AtomicU64,
    remote_logging_enabled: AtomicUsize,
}

static LOGGER: Logger = Logger {
    kernel_logs: Mutex::new(RingBuffer {
        buffer: VecDeque::new(),
        capacity: 0,
    }),
    user_logs: Mutex::new(RingBuffer {
        buffer: VecDeque::new(),
        capacity: 0,
    }),
    min_level: AtomicUsize::new(LogLevel::Info as usize),
    log_count: AtomicU64::new(0),
    dropped_count: AtomicU64::new(0),
    remote_logging_enabled: AtomicUsize::new(0),
};

const KERNEL_LOG_CAPACITY: usize = 10000;
const USER_LOG_CAPACITY: usize = 5000;

pub fn init() {
    *LOGGER.kernel_logs.lock() = RingBuffer::new(KERNEL_LOG_CAPACITY);
    *LOGGER.user_logs.lock() = RingBuffer::new(USER_LOG_CAPACITY);
    set_min_level(LogLevel::Info);
}

pub fn set_min_level(level: LogLevel) {
    LOGGER.min_level.store(level as usize, Ordering::SeqCst);
}

pub fn get_min_level() -> LogLevel {
    match LOGGER.min_level.load(Ordering::SeqCst) {
        0 => LogLevel::Trace,
        1 => LogLevel::Debug,
        2 => LogLevel::Info,
        3 => LogLevel::Warn,
        4 => LogLevel::Error,
        5 => LogLevel::Fatal,
        _ => LogLevel::Info,
    }
}

pub fn log(
    level: LogLevel,
    category: &str,
    message: &str,
    file: Option<&str>,
    line: Option<u32>,
) {
    if (level as usize) < LOGGER.min_level.load(Ordering::SeqCst) {
        return;
    }

    let timestamp = crate::timer::get_ticks();
    let cpu_id = crate::cpu::current_cpu_id();
    let (process_id, thread_id) = if let Some(proc) = crate::process::current_process() {
        (proc.pid(), proc.current_thread_id())
    } else {
        (0, 0)
    };

    let entry = LogEntry {
        timestamp,
        level,
        cpu_id,
        process_id,
        thread_id,
        category: category.to_string(),
        message: message.to_string(),
        file: file.map(|s| s.to_string()),
        line,
    };

    let is_kernel = process_id == 0;
    
    if is_kernel {
        if let Some(mut logs) = LOGGER.kernel_logs.try_lock() {
            logs.push(entry.clone());
        } else {
            LOGGER.dropped_count.fetch_add(1, Ordering::Relaxed);
        }
    } else {
        if let Some(mut logs) = LOGGER.user_logs.try_lock() {
            logs.push(entry.clone());
        } else {
            LOGGER.dropped_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    LOGGER.log_count.fetch_add(1, Ordering::Relaxed);

    if LOGGER.remote_logging_enabled.load(Ordering::Relaxed) != 0 {
        send_remote_log(&entry);
    }

    if level >= LogLevel::Error {
        crate::serial_println!(
            "[{}] [CPU:{}] [{}] {}:{}: {}",
            level.as_str(),
            cpu_id,
            category,
            file.unwrap_or("unknown"),
            line.unwrap_or(0),
            message
        );
    }
}

fn send_remote_log(entry: &LogEntry) {
    // TODO: Implement syslog protocol
}

pub fn flush() {
    // Force any pending logs to be written
}

pub fn get_kernel_logs(max_count: usize) -> Vec<LogEntry> {
    LOGGER
        .kernel_logs
        .lock()
        .iter()
        .take(max_count)
        .cloned()
        .collect()
}

pub fn get_user_logs(max_count: usize) -> Vec<LogEntry> {
    LOGGER
        .user_logs
        .lock()
        .iter()
        .take(max_count)
        .cloned()
        .collect()
}

pub fn clear_logs() {
    LOGGER.kernel_logs.lock().clear();
    LOGGER.user_logs.lock().clear();
}

pub fn get_stats() -> LogStats {
    LogStats {
        total_logs: LOGGER.log_count.load(Ordering::Relaxed),
        dropped_logs: LOGGER.dropped_count.load(Ordering::Relaxed),
        kernel_log_count: LOGGER.kernel_logs.lock().len(),
        user_log_count: LOGGER.user_logs.lock().len(),
    }
}

pub struct LogStats {
    pub total_logs: u64,
    pub dropped_logs: u64,
    pub kernel_log_count: usize,
    pub user_log_count: usize,
}

#[macro_export]
macro_rules! log_trace {
    ($category:expr, $($arg:tt)*) => {
        $crate::monitoring::logging::log(
            $crate::monitoring::logging::LogLevel::Trace,
            $category,
            &alloc::format!($($arg)*),
            Some(file!()),
            Some(line!()),
        )
    };
}

#[macro_export]
macro_rules! log_debug {
    ($category:expr, $($arg:tt)*) => {
        $crate::monitoring::logging::log(
            $crate::monitoring::logging::LogLevel::Debug,
            $category,
            &alloc::format!($($arg)*),
            Some(file!()),
            Some(line!()),
        )
    };
}

#[macro_export]
macro_rules! log_info {
    ($category:expr, $($arg:tt)*) => {
        $crate::monitoring::logging::log(
            $crate::monitoring::logging::LogLevel::Info,
            $category,
            &alloc::format!($($arg)*),
            Some(file!()),
            Some(line!()),
        )
    };
}

#[macro_export]
macro_rules! log_warn {
    ($category:expr, $($arg:tt)*) => {
        $crate::monitoring::logging::log(
            $crate::monitoring::logging::LogLevel::Warn,
            $category,
            &alloc::format!($($arg)*),
            Some(file!()),
            Some(line!()),
        )
    };
}

#[macro_export]
macro_rules! log_error {
    ($category:expr, $($arg:tt)*) => {
        $crate::monitoring::logging::log(
            $crate::monitoring::logging::LogLevel::Error,
            $category,
            &alloc::format!($($arg)*),
            Some(file!()),
            Some(line!()),
        )
    };
}

#[macro_export]
macro_rules! log_fatal {
    ($category:expr, $($arg:tt)*) => {
        $crate::monitoring::logging::log(
            $crate::monitoring::logging::LogLevel::Fatal,
            $category,
            &alloc::format!($($arg)*),
            Some(file!()),
            Some(line!()),
        )
    };
}