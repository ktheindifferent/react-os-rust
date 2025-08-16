use alloc::vec::Vec;
use alloc::sync::Arc;
use spin::RwLock;
use core::time::Duration;
use super::MediaType;

#[derive(Debug)]
pub struct Buffer {
    pub data: Vec<u8>,
    pub pts: Option<i64>,
    pub dts: Option<i64>,
    pub duration: Option<Duration>,
    pub offset: Option<u64>,
    pub flags: BufferFlags,
    pub media_type: MediaType,
    metadata: RwLock<BufferMetadata>,
}

bitflags::bitflags! {
    pub struct BufferFlags: u32 {
        const LIVE = 1 << 0;
        const DECODE_ONLY = 1 << 1;
        const DISCONT = 1 << 2;
        const RESYNC = 1 << 3;
        const CORRUPTED = 1 << 4;
        const MARKER = 1 << 5;
        const HEADER = 1 << 6;
        const GAP = 1 << 7;
        const DROPPABLE = 1 << 8;
        const DELTA_UNIT = 1 << 9;
        const TAG_MEMORY = 1 << 10;
        const SYNC_AFTER = 1 << 11;
    }
}

#[derive(Debug, Default)]
struct BufferMetadata {
    reference_count: usize,
    pool: Option<Arc<BufferPool>>,
}

impl Buffer {
    pub fn new(data: Vec<u8>, media_type: MediaType) -> Self {
        Self {
            data,
            pts: None,
            dts: None,
            duration: None,
            offset: None,
            flags: BufferFlags::empty(),
            media_type,
            metadata: RwLock::new(BufferMetadata::default()),
        }
    }

    pub fn with_pts(mut self, pts: i64) -> Self {
        self.pts = Some(pts);
        self
    }

    pub fn with_dts(mut self, dts: i64) -> Self {
        self.dts = Some(dts);
        self
    }

    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    pub fn with_flags(mut self, flags: BufferFlags) -> Self {
        self.flags = flags;
        self
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }

    pub fn is_writable(&self) -> bool {
        self.metadata.read().reference_count <= 1
    }

    pub fn make_writable(&mut self) -> Result<(), BufferError> {
        if !self.is_writable() {
            let new_data = self.data.clone();
            self.data = new_data;
            self.metadata.write().reference_count = 1;
        }
        Ok(())
    }

    pub fn copy_deep(&self) -> Self {
        Self {
            data: self.data.clone(),
            pts: self.pts,
            dts: self.dts,
            duration: self.duration,
            offset: self.offset,
            flags: self.flags,
            media_type: self.media_type,
            metadata: RwLock::new(BufferMetadata::default()),
        }
    }

    pub fn copy_metadata(&self, other: &Buffer) {
        self.metadata.write().pool = other.metadata.read().pool.clone();
    }
}

pub struct BufferPool {
    name: String,
    buffer_size: usize,
    min_buffers: usize,
    max_buffers: usize,
    free_buffers: RwLock<Vec<Arc<Buffer>>>,
    allocated_count: RwLock<usize>,
}

impl BufferPool {
    pub fn new(name: &str, buffer_size: usize, min_buffers: usize, max_buffers: usize) -> Self {
        let mut pool = Self {
            name: String::from(name),
            buffer_size,
            min_buffers,
            max_buffers,
            free_buffers: RwLock::new(Vec::new()),
            allocated_count: RwLock::new(0),
        };
        
        pool.preallocate();
        pool
    }

    fn preallocate(&mut self) {
        let mut buffers = self.free_buffers.write();
        for _ in 0..self.min_buffers {
            let buffer = Buffer::new(vec![0; self.buffer_size], MediaType::Data);
            buffers.push(Arc::new(buffer));
        }
        *self.allocated_count.write() = self.min_buffers;
    }

    pub fn acquire(&self) -> Result<Arc<Buffer>, BufferError> {
        if let Some(buffer) = self.free_buffers.write().pop() {
            return Ok(buffer);
        }

        let allocated = *self.allocated_count.read();
        if allocated < self.max_buffers {
            *self.allocated_count.write() += 1;
            let buffer = Buffer::new(vec![0; self.buffer_size], MediaType::Data);
            Ok(Arc::new(buffer))
        } else {
            Err(BufferError::PoolExhausted)
        }
    }

    pub fn release(&self, buffer: Arc<Buffer>) {
        if Arc::strong_count(&buffer) == 1 {
            self.free_buffers.write().push(buffer);
        }
    }

    pub fn get_stats(&self) -> PoolStats {
        PoolStats {
            allocated: *self.allocated_count.read(),
            free: self.free_buffers.read().len(),
            min_buffers: self.min_buffers,
            max_buffers: self.max_buffers,
            buffer_size: self.buffer_size,
        }
    }
}

#[derive(Debug)]
pub struct PoolStats {
    pub allocated: usize,
    pub free: usize,
    pub min_buffers: usize,
    pub max_buffers: usize,
    pub buffer_size: usize,
}

pub struct BufferList {
    buffers: Vec<Arc<Buffer>>,
}

impl BufferList {
    pub fn new() -> Self {
        Self {
            buffers: Vec::new(),
        }
    }

    pub fn push(&mut self, buffer: Arc<Buffer>) {
        self.buffers.push(buffer);
    }

    pub fn pop(&mut self) -> Option<Arc<Buffer>> {
        if !self.buffers.is_empty() {
            Some(self.buffers.remove(0))
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.buffers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffers.is_empty()
    }

    pub fn clear(&mut self) {
        self.buffers.clear();
    }

    pub fn calculate_size(&self) -> usize {
        self.buffers.iter().map(|b| b.size()).sum()
    }

    pub fn calculate_duration(&self) -> Option<Duration> {
        let mut total = Duration::from_nanos(0);
        for buffer in &self.buffers {
            if let Some(duration) = buffer.duration {
                total += duration;
            } else {
                return None;
            }
        }
        Some(total)
    }
}

#[derive(Debug)]
pub enum BufferError {
    PoolExhausted,
    InvalidSize,
    AllocationFailed,
}