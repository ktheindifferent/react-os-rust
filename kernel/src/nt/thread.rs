use super::{NtStatus, object::{ObjectHeader, ObjectTrait, Handle, ObjectType}};
use super::process::{ProcessId, ThreadId};

// Thread state
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    Initialized = 0,
    Ready = 1,
    Running = 2,
    Standby = 3,
    Terminated = 4,
    Waiting = 5,
    Transition = 6,
    DeferredReady = 7,
    GateWait = 8,
}

// NT Thread structure
#[derive(Debug)]
pub struct NtThread {
    pub header: ObjectHeader,
    pub thread_id: ThreadId,
    pub process_id: ProcessId,
    pub state: ThreadState,
    pub priority: u8,
    pub base_priority: u8,
    pub stack_base: u64,
    pub stack_limit: u64,
    pub kernel_stack_base: u64,
    pub kernel_stack_limit: u64,
    pub context: ThreadContext,
    pub exit_code: Option<u32>,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct ThreadContext {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub rsp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rip: u64,
    pub rflags: u64,
}

impl Default for ThreadContext {
    fn default() -> Self {
        Self {
            rax: 0, rbx: 0, rcx: 0, rdx: 0,
            rsi: 0, rdi: 0, rbp: 0, rsp: 0,
            r8: 0, r9: 0, r10: 0, r11: 0,
            r12: 0, r13: 0, r14: 0, r15: 0,
            rip: 0, rflags: 0x202, // Enable interrupts
        }
    }
}

impl NtThread {
    pub fn new(process_id: ProcessId) -> Self {
        Self {
            header: ObjectHeader::new(ObjectType::Thread),
            thread_id: ThreadId::new(),
            process_id,
            state: ThreadState::Initialized,
            priority: 8, // Normal priority
            base_priority: 8,
            stack_base: 0,
            stack_limit: 0,
            kernel_stack_base: 0,
            kernel_stack_limit: 0,
            context: ThreadContext::default(),
            exit_code: None,
        }
    }
}

impl ObjectTrait for NtThread {
    fn get_header(&self) -> &ObjectHeader {
        &self.header
    }

    fn get_header_mut(&mut self) -> &mut ObjectHeader {
        &mut self.header
    }
}