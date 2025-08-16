// GPU Command Submission and Ring Buffer Management
use alloc::vec::Vec;
use alloc::collections::VecDeque;
use spin::Mutex;
use x86_64::VirtAddr;
use super::{EngineType, BufferObject};

// GPU Command Types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CommandType {
    // State Management
    PipelineControl,
    StateBaseAddress,
    
    // Rendering Commands
    DrawPrimitive,
    DrawIndexedPrimitive,
    DrawIndirect,
    DrawIndexedIndirect,
    
    // Compute Commands
    ComputeDispatch,
    ComputeDispatchIndirect,
    
    // Memory Commands
    MemoryCopy,
    MemoryFill,
    MemoryBarrier,
    CacheFlush,
    
    // Blitting Commands
    Blit2D,
    BlitScaled,
    ColorFill,
    
    // Synchronization
    Fence,
    Semaphore,
    Event,
    PipelineBarrier,
    
    // Context Management
    ContextSwitch,
    SaveContext,
    RestoreContext,
    
    // Video Commands
    VideoDecode,
    VideoEncode,
    VideoProcess,
}

// GPU Command Packet
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CommandPacket {
    pub opcode: u32,
    pub length: u32,  // In dwords
    pub flags: u32,
    pub data: [u32; 61], // Variable size data
}

// Command Ring Buffer
pub struct CommandRing {
    pub buffer: VirtAddr,
    pub size: usize,
    pub head: usize,
    pub tail: usize,
    pub wrap_count: u64,
}

impl CommandRing {
    pub fn new(buffer: VirtAddr, size: usize) -> Self {
        Self {
            buffer,
            size,
            head: 0,
            tail: 0,
            wrap_count: 0,
        }
    }
    
    pub fn space_available(&self) -> usize {
        if self.tail >= self.head {
            self.size - self.tail + self.head
        } else {
            self.head - self.tail
        }
    }
    
    pub fn is_empty(&self) -> bool {
        self.head == self.tail
    }
    
    pub fn write_command(&mut self, cmd: &CommandPacket) -> Result<(), &'static str> {
        let cmd_size = core::mem::size_of::<CommandPacket>();
        
        if self.space_available() < cmd_size {
            return Err("Command ring buffer full");
        }
        
        unsafe {
            let dst = (self.buffer.as_u64() + self.tail as u64) as *mut CommandPacket;
            *dst = *cmd;
        }
        
        self.tail += cmd_size;
        if self.tail >= self.size {
            self.tail = 0;
            self.wrap_count += 1;
        }
        
        Ok(())
    }
    
    pub fn write_dword(&mut self, value: u32) -> Result<(), &'static str> {
        if self.space_available() < 4 {
            return Err("Command ring buffer full");
        }
        
        unsafe {
            let dst = (self.buffer.as_u64() + self.tail as u64) as *mut u32;
            *dst = value;
        }
        
        self.tail += 4;
        if self.tail >= self.size {
            self.tail = 0;
            self.wrap_count += 1;
        }
        
        Ok(())
    }
    
    pub fn advance_head(&mut self, bytes: usize) {
        self.head += bytes;
        if self.head >= self.size {
            self.head -= self.size;
        }
    }
}

// Command Builder for easier command construction
pub struct CommandBuilder {
    commands: Vec<u32>,
}

impl CommandBuilder {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }
    
    pub fn add_nop(&mut self, count: u32) -> &mut Self {
        for _ in 0..count {
            self.commands.push(0x00000000); // NOP opcode
        }
        self
    }
    
    pub fn add_pipeline_control(&mut self, flags: u32) -> &mut Self {
        self.commands.push(0x7A000003); // PIPE_CONTROL opcode
        self.commands.push(flags);
        self.commands.push(0); // Address low
        self.commands.push(0); // Address high
        self.commands.push(0); // Immediate data
        self
    }
    
    pub fn add_state_base_address(&mut self, base_addresses: &[u64]) -> &mut Self {
        self.commands.push(0x61010000 | ((base_addresses.len() * 2 - 1) as u32)); // STATE_BASE_ADDRESS
        
        for addr in base_addresses {
            self.commands.push(*addr as u32);
            self.commands.push((*addr >> 32) as u32);
        }
        self
    }
    
    pub fn add_2d_blit(&mut self, src_addr: u64, dst_addr: u64, 
                       src_pitch: u32, dst_pitch: u32,
                       src_x: u32, src_y: u32, 
                       dst_x: u32, dst_y: u32,
                       width: u32, height: u32) -> &mut Self {
        // XY_SRC_COPY_BLT command
        self.commands.push(0x53300008); // Opcode + length
        self.commands.push(0xCC << 16 | dst_pitch); // ROP + dst pitch
        self.commands.push((dst_y << 16) | dst_x); // Dst X,Y
        self.commands.push(((dst_y + height) << 16) | (dst_x + width)); // Dst X2,Y2
        self.commands.push(dst_addr as u32); // Dst address low
        self.commands.push((dst_addr >> 32) as u32); // Dst address high
        self.commands.push((src_y << 16) | src_x); // Src X,Y
        self.commands.push(src_pitch); // Src pitch
        self.commands.push(src_addr as u32); // Src address low
        self.commands.push((src_addr >> 32) as u32); // Src address high
        self
    }
    
    pub fn add_color_fill(&mut self, dst_addr: u64, dst_pitch: u32,
                         x: u32, y: u32, width: u32, height: u32,
                         color: u32) -> &mut Self {
        // XY_COLOR_BLT command
        self.commands.push(0x53200006); // Opcode + length
        self.commands.push(0xF0 << 16 | dst_pitch); // ROP + dst pitch
        self.commands.push((y << 16) | x); // Dst X,Y
        self.commands.push(((y + height) << 16) | (x + width)); // Dst X2,Y2
        self.commands.push(dst_addr as u32); // Dst address low
        self.commands.push((dst_addr >> 32) as u32); // Dst address high
        self.commands.push(color); // Fill color
        self
    }
    
    pub fn add_mi_batch_buffer_end(&mut self) -> &mut Self {
        self.commands.push(0x0A000000); // MI_BATCH_BUFFER_END
        self
    }
    
    pub fn add_mi_flush(&mut self) -> &mut Self {
        self.commands.push(0x04000000); // MI_FLUSH
        self
    }
    
    pub fn add_mi_load_register_imm(&mut self, reg: u32, value: u32) -> &mut Self {
        self.commands.push(0x11000001); // MI_LOAD_REGISTER_IMM
        self.commands.push(reg);
        self.commands.push(value);
        self
    }
    
    pub fn add_mi_store_register_mem(&mut self, reg: u32, addr: u64) -> &mut Self {
        self.commands.push(0x24000001); // MI_STORE_REGISTER_MEM
        self.commands.push(reg);
        self.commands.push(addr as u32);
        self.commands.push((addr >> 32) as u32);
        self
    }
    
    pub fn add_mi_wait_for_event(&mut self, event_mask: u32) -> &mut Self {
        self.commands.push(0x03000000 | event_mask); // MI_WAIT_FOR_EVENT
        self
    }
    
    pub fn add_3d_primitive(&mut self, vertex_count: u32, instance_count: u32,
                           start_vertex: u32, start_instance: u32) -> &mut Self {
        // 3DPRIMITIVE command
        self.commands.push(0x7B000005); // Opcode + length
        self.commands.push(0); // Vertex Access Type
        self.commands.push(vertex_count);
        self.commands.push(start_vertex);
        self.commands.push(instance_count);
        self.commands.push(start_instance);
        self.commands.push(0); // Base Vertex Location
        self
    }
    
    pub fn build(&self) -> Vec<u32> {
        self.commands.clone()
    }
    
    pub fn size_bytes(&self) -> usize {
        self.commands.len() * 4
    }
}

// Batch Buffer for command submission
pub struct BatchBuffer {
    pub id: u64,
    pub buffer: BufferObject,
    pub commands: CommandBuilder,
    pub size: usize,
    pub engine: EngineType,
}

impl BatchBuffer {
    pub fn new(id: u64, buffer: BufferObject, engine: EngineType) -> Self {
        Self {
            id,
            buffer,
            commands: CommandBuilder::new(),
            size: 0,
            engine,
        }
    }
    
    pub fn add_commands(&mut self, builder: CommandBuilder) {
        self.commands = builder;
        self.size = self.commands.size_bytes();
    }
    
    pub fn write_to_buffer(&mut self, vaddr: VirtAddr) -> Result<(), &'static str> {
        let cmds = self.commands.build();
        let size_bytes = cmds.len() * 4;
        
        if size_bytes > self.buffer.size as usize {
            return Err("Commands exceed buffer size");
        }
        
        unsafe {
            let dst = vaddr.as_u64() as *mut u32;
            for (i, cmd) in cmds.iter().enumerate() {
                *dst.add(i) = *cmd;
            }
        }
        
        self.size = size_bytes;
        Ok(())
    }
}

// Command Submission Queue
pub struct CommandQueue {
    pub engine: EngineType,
    pub pending: VecDeque<BatchBuffer>,
    pub executing: Option<BatchBuffer>,
    pub completed: VecDeque<BatchBuffer>,
    pub ring: Option<CommandRing>,
}

impl CommandQueue {
    pub fn new(engine: EngineType) -> Self {
        Self {
            engine,
            pending: VecDeque::new(),
            executing: None,
            completed: VecDeque::new(),
            ring: None,
        }
    }
    
    pub fn submit(&mut self, batch: BatchBuffer) {
        self.pending.push_back(batch);
    }
    
    pub fn execute_next(&mut self) -> Option<&BatchBuffer> {
        if self.executing.is_some() {
            return None;
        }
        
        if let Some(batch) = self.pending.pop_front() {
            self.executing = Some(batch);
            self.executing.as_ref()
        } else {
            None
        }
    }
    
    pub fn complete_current(&mut self) {
        if let Some(batch) = self.executing.take() {
            self.completed.push_back(batch);
        }
    }
    
    pub fn is_idle(&self) -> bool {
        self.pending.is_empty() && self.executing.is_none()
    }
}