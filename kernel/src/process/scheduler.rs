use super::thread::THREAD_MANAGER;
use super::{ThreadId, ProcessId};
use alloc::string::ToString;

pub struct RoundRobinScheduler {
    time_slice: u32,
    current_time_slice: u32,
}

impl RoundRobinScheduler {
    pub fn new(time_slice: u32) -> Self {
        Self {
            time_slice,
            current_time_slice: 0,
        }
    }

    pub fn tick(&mut self) -> Option<ThreadId> {
        self.current_time_slice += 1;
        
        if self.current_time_slice >= self.time_slice {
            self.current_time_slice = 0;
            self.schedule()
        } else {
            None
        }
    }

    pub fn schedule(&mut self) -> Option<ThreadId> {
        let mut thread_manager = THREAD_MANAGER.lock();
        let ready_threads = thread_manager.get_ready_threads();
        
        if ready_threads.is_empty() {
            return None;
        }

        // Simple round-robin: get next thread
        let current = thread_manager.get_current_thread();
        let next_thread = if let Some(current_id) = current {
            // Find current thread in ready list and get next one
            if let Some(pos) = ready_threads.iter().position(|&id| id == current_id) {
                let next_pos = (pos + 1) % ready_threads.len();
                ready_threads[next_pos]
            } else {
                // Current thread not ready, pick first ready thread
                ready_threads[0]
            }
        } else {
            // No current thread, pick first ready thread
            ready_threads[0]
        };

        thread_manager.set_current_thread(next_thread);
        Some(next_thread)
    }

    pub fn yield_thread(&mut self) -> Option<ThreadId> {
        self.current_time_slice = 0;
        self.schedule()
    }
}

// Windows-compatible process creation functions
pub fn create_process_w(
    application_name: &str,
    _command_line: &str,
    _inherit_handles: bool,
) -> Result<ProcessId, &'static str> {
    use super::PROCESS_MANAGER;
    
    let mut process_manager = PROCESS_MANAGER.lock();
    let process_id = process_manager.create_process(
        application_name.to_string(),
        None, // No parent for now
    );

    // Create main thread for the process
    let mut thread_manager = THREAD_MANAGER.lock();
    let thread_id = thread_manager.create_thread(process_id);
    
    // Add thread to process
    drop(thread_manager);
    if let Some(process) = process_manager.get_process_mut(process_id) {
        process.add_thread(thread_id);
    }

    Ok(process_id)
}

pub fn terminate_process_w(process_id: ProcessId, _exit_code: u32) -> Result<(), &'static str> {
    use super::PROCESS_MANAGER;
    
    let mut process_manager = PROCESS_MANAGER.lock();
    process_manager.terminate_process(process_id);
    Ok(())
}