// Dynamic batch scheduling for inference
use alloc::{vec::Vec, collections::VecDeque};
use core::time::Duration;

pub struct BatchScheduler {
    max_batch_size: usize,
    max_delay: Duration,
    queue: VecDeque<Request>,
}

pub struct Request {
    id: usize,
    data: Vec<u8>,
    timestamp: u64,
}

impl BatchScheduler {
    pub fn new(max_batch_size: usize, max_delay: Duration) -> Self {
        Self {
            max_batch_size,
            max_delay,
            queue: VecDeque::new(),
        }
    }
    
    pub fn add_request(&mut self, request: Request) {
        self.queue.push_back(request);
    }
    
    pub fn get_batch(&mut self) -> Option<Vec<Request>> {
        if self.queue.len() >= self.max_batch_size {
            let mut batch = Vec::new();
            for _ in 0..self.max_batch_size {
                if let Some(req) = self.queue.pop_front() {
                    batch.push(req);
                }
            }
            Some(batch)
        } else {
            None
        }
    }
}