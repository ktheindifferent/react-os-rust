use super::{ThreadId, ProcessId};
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;

#[derive(Debug, Clone, Copy)]
pub enum ThreadState {
    Running,
    Ready,
    Blocked,
    Terminated,
}

#[derive(Debug)]
pub struct Thread {
    pub id: ThreadId,
    pub process_id: ProcessId,
    pub state: ThreadState,
    pub stack_pointer: u64,
    pub instruction_pointer: u64,
    pub priority: u8,
}

impl Thread {
    pub fn new(id: ThreadId, process_id: ProcessId) -> Self {
        Self {
            id,
            process_id,
            state: ThreadState::Ready,
            stack_pointer: 0,
            instruction_pointer: 0,
            priority: 0,
        }
    }
}

pub struct ThreadManager {
    threads: Vec<Thread>,
    next_thread_id: u32,
    current_thread: Option<ThreadId>,
}

impl ThreadManager {
    pub fn new() -> Self {
        Self {
            threads: Vec::new(),
            next_thread_id: 1,
            current_thread: None,
        }
    }

    pub fn create_thread(&mut self, process_id: ProcessId) -> ThreadId {
        let id = ThreadId(self.next_thread_id);
        self.next_thread_id += 1;

        let thread = Thread::new(id, process_id);
        self.threads.push(thread);

        id
    }

    pub fn get_thread(&self, id: ThreadId) -> Option<&Thread> {
        self.threads.iter().find(|t| t.id == id)
    }

    pub fn get_thread_mut(&mut self, id: ThreadId) -> Option<&mut Thread> {
        self.threads.iter_mut().find(|t| t.id == id)
    }

    pub fn set_current_thread(&mut self, id: ThreadId) {
        self.current_thread = Some(id);
    }

    pub fn get_current_thread(&self) -> Option<ThreadId> {
        self.current_thread
    }

    pub fn get_ready_threads(&self) -> Vec<ThreadId> {
        self.threads
            .iter()
            .filter(|t| matches!(t.state, ThreadState::Ready))
            .map(|t| t.id)
            .collect()
    }
}

lazy_static! {
    pub static ref THREAD_MANAGER: Mutex<ThreadManager> = Mutex::new(ThreadManager::new());
}