// Magic SysRq Key Support
// Emergency keyboard commands for system recovery and debugging

use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicBool, Ordering};
use spin::Mutex;
use lazy_static::lazy_static;

type SysRqHandler = fn();

pub struct SysRqSystem {
    enabled: AtomicBool,
    handlers: Mutex<BTreeMap<char, SysRqCommand>>,
    in_progress: AtomicBool,
}

struct SysRqCommand {
    key: char,
    description: &'static str,
    handler: SysRqHandler,
    enabled: bool,
}

lazy_static! {
    pub static ref SYSRQ: SysRqSystem = SysRqSystem::new();
}

impl SysRqSystem {
    pub fn new() -> Self {
        let mut system = Self {
            enabled: AtomicBool::new(true),
            handlers: Mutex::new(BTreeMap::new()),
            in_progress: AtomicBool::new(false),
        };
        
        system.register_default_handlers();
        system
    }
    
    fn register_default_handlers(&mut self) {
        // Register all standard SysRq commands
        self.register('b', "Immediately reboot the system", sysrq_reboot);
        self.register('c', "Trigger a kernel crash (for testing kdump)", sysrq_crash);
        self.register('d', "Show all locks that are held", sysrq_show_locks);
        self.register('e', "Send SIGTERM to all processes", sysrq_term_all);
        self.register('f', "Call OOM killer", sysrq_oom_kill);
        self.register('g', "Enter kernel debugger (KDB/KGDB)", sysrq_kgdb);
        self.register('h', "Display help (this message)", sysrq_help);
        self.register('i', "Send SIGKILL to all processes", sysrq_kill_all);
        self.register('k', "Secure Access Key (SAK) - kill all on current terminal", sysrq_sak);
        self.register('l', "Show backtrace for all CPUs", sysrq_show_backtrace);
        self.register('m', "Show memory usage", sysrq_show_memory);
        self.register('n', "Make RT tasks nice-able", sysrq_nice_rt);
        self.register('o', "Power off the system", sysrq_poweroff);
        self.register('p', "Show current registers and flags", sysrq_show_regs);
        self.register('q', "Show all timers", sysrq_show_timers);
        self.register('r', "Turn off keyboard raw mode", sysrq_unraw);
        self.register('s', "Sync all filesystems", sysrq_sync);
        self.register('t', "Show task states", sysrq_show_tasks);
        self.register('u', "Remount all filesystems read-only", sysrq_remount_ro);
        self.register('v', "Forcefully restore framebuffer console", sysrq_restore_fb);
        self.register('w', "Show blocked (D state) tasks", sysrq_show_blocked);
        self.register('x', "Dump ftrace buffer", sysrq_dump_ftrace);
        self.register('z', "Dump kernel ring buffer", sysrq_dump_dmesg);
        self.register('0', "Set console log level to 0", || sysrq_loglevel(0));
        self.register('1', "Set console log level to 1", || sysrq_loglevel(1));
        self.register('2', "Set console log level to 2", || sysrq_loglevel(2));
        self.register('3', "Set console log level to 3", || sysrq_loglevel(3));
        self.register('4', "Set console log level to 4", || sysrq_loglevel(4));
        self.register('5', "Set console log level to 5", || sysrq_loglevel(5));
        self.register('6', "Set console log level to 6", || sysrq_loglevel(6));
        self.register('7', "Set console log level to 7", || sysrq_loglevel(7));
        self.register('8', "Set console log level to 8", || sysrq_loglevel(8));
        self.register('9', "Set console log level to 9", || sysrq_loglevel(9));
    }
    
    pub fn register(&mut self, key: char, description: &'static str, handler: SysRqHandler) {
        self.handlers.lock().insert(key, SysRqCommand {
            key,
            description,
            handler,
            enabled: true,
        });
    }
    
    pub fn handle_sysrq(&self, key: char) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        
        // Prevent re-entrance
        if self.in_progress.swap(true, Ordering::SeqCst) {
            crate::serial_println!("SysRq: Already processing a command");
            return;
        }
        
        crate::serial_println!("\nSysRq: {}", key);
        
        if let Some(cmd) = self.handlers.lock().get(&key) {
            if cmd.enabled {
                crate::serial_println!("SysRq: {}", cmd.description);
                (cmd.handler)();
            } else {
                crate::serial_println!("SysRq: Command '{}' is disabled", key);
            }
        } else {
            crate::serial_println!("SysRq: Unknown command '{}'", key);
            crate::serial_println!("SysRq: Type 'h' for help");
        }
        
        self.in_progress.store(false, Ordering::SeqCst);
    }
    
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::SeqCst);
        crate::serial_println!("SysRq: Enabled");
    }
    
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::SeqCst);
        crate::serial_println!("SysRq: Disabled");
    }
}

// SysRq command implementations
fn sysrq_help() {
    crate::serial_println!("\nSysRq Help:");
    crate::serial_println!("============");
    
    let handlers = SYSRQ.handlers.lock();
    let mut keys: Vec<_> = handlers.keys().collect();
    keys.sort();
    
    for key in keys {
        if let Some(cmd) = handlers.get(key) {
            crate::serial_println!("  {} - {}", key, cmd.description);
        }
    }
    
    crate::serial_println!("\nUsage: Alt+SysRq+<key>");
}

fn sysrq_reboot() {
    crate::serial_println!("SysRq: Emergency Reboot");
    crate::serial_println!("SysRq: Rebooting NOW!");
    
    // Sync filesystems first
    sysrq_sync();
    
    // Trigger immediate reboot
    unsafe {
        // Triple fault to force reboot
        core::arch::asm!("xor %eax, %eax");
        core::arch::asm!("mov %eax, %cr3");
    }
}

fn sysrq_crash() {
    crate::serial_println!("SysRq: Triggering kernel crash");
    crate::serial_println!("SysRq: This is a test crash for kdump");
    
    // Trigger a deliberate panic
    panic!("SysRq triggered crash");
}

fn sysrq_show_locks() {
    crate::serial_println!("SysRq: Held Locks:");
    crate::serial_println!("SysRq: Lock debugging not yet implemented");
    // Would show all held locks and their holders
}

fn sysrq_term_all() {
    crate::serial_println!("SysRq: Sending SIGTERM to all processes");
    // Would send SIGTERM to all processes except init
}

fn sysrq_kill_all() {
    crate::serial_println!("SysRq: Sending SIGKILL to all processes");
    // Would send SIGKILL to all processes except init
}

fn sysrq_oom_kill() {
    crate::serial_println!("SysRq: Invoking OOM killer");
    // Would trigger out-of-memory killer
}

fn sysrq_kgdb() {
    crate::serial_println!("SysRq: Entering kernel debugger");
    
    if super::DEBUG_STATE.kdb_enabled.load(Ordering::Relaxed) {
        super::kdb::enter_debugger("SysRq");
    } else if super::kgdb::is_connected() {
        super::kgdb::force_entry();
    } else {
        crate::serial_println!("SysRq: No debugger available");
    }
}

fn sysrq_sak() {
    crate::serial_println!("SysRq: Secure Access Key");
    // Would kill all processes on current terminal
}

fn sysrq_show_backtrace() {
    crate::serial_println!("SysRq: Backtrace for all CPUs:");
    
    // Show current CPU backtrace
    if let Some(trace) = super::generate_stack_trace() {
        crate::serial_println!("CPU 0:");
        for frame in trace {
            crate::serial_println!("  {}", frame);
        }
    }
    
    // Would show backtraces for all other CPUs via IPI
}

fn sysrq_show_memory() {
    crate::serial_println!("SysRq: Memory Info:");
    crate::serial_println!("SysRq: Free: 0 KB");  // Would get from allocator
    crate::serial_println!("SysRq: Used: 0 KB");
    crate::serial_println!("SysRq: Total: 0 KB");
    
    // Show KASAN stats if enabled
    if super::DEBUG_STATE.kasan_enabled.load(Ordering::Relaxed) {
        super::kasan::print_stats();
    }
}

fn sysrq_nice_rt() {
    crate::serial_println!("SysRq: Making RT tasks nice-able");
    // Would adjust real-time task priorities
}

fn sysrq_poweroff() {
    crate::serial_println!("SysRq: Power Off");
    crate::serial_println!("SysRq: System will power off NOW!");
    
    // Sync filesystems first
    sysrq_sync();
    
    // Would trigger ACPI poweroff
}

fn sysrq_show_regs() {
    crate::serial_println!("SysRq: CPU Registers:");
    
    let mut rax: u64;
    let mut rbx: u64;
    let mut rcx: u64;
    let mut rdx: u64;
    let mut rsi: u64;
    let mut rdi: u64;
    let mut rbp: u64;
    let mut rsp: u64;
    
    unsafe {
        core::arch::asm!(
            "mov {}, rax",
            "mov {}, rbx",
            "mov {}, rcx",
            "mov {}, rdx",
            "mov {}, rsi",
            "mov {}, rdi",
            "mov {}, rbp",
            "mov {}, rsp",
            out(reg) rax,
            out(reg) rbx,
            out(reg) rcx,
            out(reg) rdx,
            out(reg) rsi,
            out(reg) rdi,
            out(reg) rbp,
            out(reg) rsp,
        );
    }
    
    crate::serial_println!("RAX: {:#018x}  RBX: {:#018x}", rax, rbx);
    crate::serial_println!("RCX: {:#018x}  RDX: {:#018x}", rcx, rdx);
    crate::serial_println!("RSI: {:#018x}  RDI: {:#018x}", rsi, rdi);
    crate::serial_println!("RBP: {:#018x}  RSP: {:#018x}", rbp, rsp);
}

fn sysrq_show_timers() {
    crate::serial_println!("SysRq: Active Timers:");
    // Would show all active kernel timers
}

fn sysrq_unraw() {
    crate::serial_println!("SysRq: Keyboard mode reset");
    // Would reset keyboard to normal mode
}

fn sysrq_sync() {
    crate::serial_println!("SysRq: Syncing filesystems");
    // Would sync all mounted filesystems
}

fn sysrq_show_tasks() {
    crate::serial_println!("SysRq: Task States:");
    crate::serial_println!("SysRq: PID   State   Command");
    // Would show all task states
}

fn sysrq_remount_ro() {
    crate::serial_println!("SysRq: Remounting filesystems read-only");
    // Would remount all filesystems as read-only
}

fn sysrq_restore_fb() {
    crate::serial_println!("SysRq: Restoring framebuffer console");
    // Would restore VGA/framebuffer console
}

fn sysrq_show_blocked() {
    crate::serial_println!("SysRq: Blocked Tasks:");
    // Would show all tasks in uninterruptible sleep
}

fn sysrq_dump_ftrace() {
    crate::serial_println!("SysRq: Dumping trace buffer");
    super::trace::print_trace(100);
}

fn sysrq_dump_dmesg() {
    crate::serial_println!("SysRq: Kernel ring buffer:");
    // Would dump kernel message buffer
}

fn sysrq_loglevel(level: u32) {
    crate::serial_println!("SysRq: Setting console log level to {}", level);
    // Would set kernel log level
}

// Public API
pub fn init() {
    crate::serial_println!("[SYSRQ] Magic SysRq key support initialized");
}

pub fn handle_sysrq(key: char) {
    SYSRQ.handle_sysrq(key);
}

pub fn is_enabled() -> bool {
    SYSRQ.enabled.load(Ordering::Relaxed)
}

pub fn enable() {
    SYSRQ.enable();
}

pub fn disable() {
    SYSRQ.disable();
}