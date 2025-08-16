// NVMe Queue Management
use super::*;
use core::sync::atomic::{AtomicU16, Ordering};

pub struct NvmeQueuePair {
    pub submission: NvmeQueue,
    pub completion: NvmeQueue,
    pub queue_id: u16,
    pub max_commands: u16,
    command_id_counter: AtomicU16,
}

impl NvmeQueuePair {
    pub fn new(queue_id: u16, sq_size: u16, cq_size: u16) -> Result<Self, &'static str> {
        let submission = NvmeQueue::new(queue_id, sq_size)?;
        let completion = NvmeQueue::new(queue_id, cq_size)?;
        
        Ok(Self {
            submission,
            completion,
            queue_id,
            max_commands: sq_size.min(cq_size),
            command_id_counter: AtomicU16::new(0),
        })
    }
    
    pub fn get_next_command_id(&self) -> u16 {
        self.command_id_counter.fetch_add(1, Ordering::SeqCst) % self.max_commands
    }
    
    pub fn submit_and_wait(&mut self, cmd: &mut NvmeCommand, base_addr: u64) -> Result<NvmeCompletion, &'static str> {
        // Set command ID
        cmd.command_id = self.get_next_command_id();
        
        // Submit command
        self.submission.submit_command(cmd, base_addr)?;
        
        // Wait for specific completion
        self.wait_for_completion(cmd.command_id, base_addr)
    }
    
    pub fn wait_for_completion(&mut self, command_id: u16, base_addr: u64) -> Result<NvmeCompletion, &'static str> {
        let timeout = 1000; // 10 seconds with 10ms intervals
        
        for _ in 0..timeout {
            if let Some(completion) = self.check_completion(command_id, base_addr)? {
                return Ok(completion);
            }
            
            // Wait 10ms
            for _ in 0..10000 {
                core::hint::spin_loop();
            }
        }
        
        Err("Command timeout")
    }
    
    fn check_completion(&mut self, command_id: u16, base_addr: u64) -> Result<Option<NvmeCompletion>, &'static str> {
        unsafe {
            let cq_ptr = (PHYS_MEM_OFFSET + self.completion.completion_queue + 
                         (self.completion.cq_head as u64 * mem::size_of::<NvmeCompletion>() as u64)) as *mut NvmeCompletion;
            let entry = cq_ptr.read_volatile();
            
            // Check phase bit
            if entry.get_phase() != self.completion.cq_phase {
                return Ok(None); // No new completion
            }
            
            // Check if this is the completion we're waiting for
            if entry.command_id != command_id {
                // Not our completion, but we still need to acknowledge it
                self.acknowledge_completion(base_addr);
                return Ok(None);
            }
            
            // Found our completion
            self.acknowledge_completion(base_addr);
            
            // Check for errors
            if entry.is_error() {
                let status_code = (entry.status >> 1) & 0x7FF;
                serial_println!("NVMe: Command failed with status 0x{:x}", status_code);
                return Err("NVMe command failed");
            }
            
            Ok(Some(entry))
        }
    }
    
    fn acknowledge_completion(&mut self, base_addr: u64) {
        unsafe {
            // Update head and phase
            self.completion.cq_head = (self.completion.cq_head + 1) % self.completion.size;
            if self.completion.cq_head == 0 {
                self.completion.cq_phase = !self.completion.cq_phase;
            }
            
            // Update completion queue head doorbell
            let doorbell = (PHYS_MEM_OFFSET + base_addr + self.completion.doorbell_addr + 4) as *mut u32;
            doorbell.write_volatile(self.completion.cq_head as u32);
        }
    }
}

// Queue utilities
impl NvmeQueue {
    pub fn reset(&mut self) {
        self.sq_tail = 0;
        self.cq_head = 0;
        self.cq_phase = true;
        
        // Clear queue memory
        unsafe {
            let sq_size = self.size as usize * mem::size_of::<NvmeCommand>();
            let cq_size = self.size as usize * mem::size_of::<NvmeCompletion>();
            
            core::ptr::write_bytes((PHYS_MEM_OFFSET + self.submission_queue) as *mut u8, 0, sq_size);
            core::ptr::write_bytes((PHYS_MEM_OFFSET + self.completion_queue) as *mut u8, 0, cq_size);
        }
    }
    
    pub fn is_full(&self) -> bool {
        ((self.sq_tail + 1) % self.size) == self.cq_head
    }
    
    pub fn available_slots(&self) -> u16 {
        if self.sq_tail >= self.cq_head {
            self.size - (self.sq_tail - self.cq_head) - 1
        } else {
            self.cq_head - self.sq_tail - 1
        }
    }
}