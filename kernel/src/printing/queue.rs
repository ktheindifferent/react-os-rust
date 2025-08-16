use alloc::{collections::VecDeque, vec::Vec, string::String};
use spin::RwLock;
use super::job::{PrintJob, JobStatus, JobPriority};

pub struct PrintQueue {
    queue_id: u32,
    name: String,
    jobs: RwLock<VecDeque<PrintJob>>,
    paused: RwLock<bool>,
    max_jobs: usize,
    total_processed: RwLock<u64>,
}

impl PrintQueue {
    pub fn new(queue_id: u32, name: String) -> Self {
        Self {
            queue_id,
            name,
            jobs: RwLock::new(VecDeque::new()),
            paused: RwLock::new(false),
            max_jobs: 1000,
            total_processed: RwLock::new(0),
        }
    }

    pub fn add_job(&self, job: PrintJob) -> Result<(), &'static str> {
        let mut queue = self.jobs.write();
        
        if queue.len() >= self.max_jobs {
            return Err("Queue is full");
        }
        
        match job.priority {
            JobPriority::High => {
                let pos = queue.iter().position(|j| j.priority != JobPriority::High)
                    .unwrap_or(queue.len());
                queue.insert(pos, job);
            }
            JobPriority::Normal => {
                let pos = queue.iter().position(|j| j.priority == JobPriority::Low)
                    .unwrap_or(queue.len());
                queue.insert(pos, job);
            }
            JobPriority::Low => {
                queue.push_back(job);
            }
        }
        
        Ok(())
    }

    pub fn get_next_job(&self) -> Option<PrintJob> {
        if *self.paused.read() {
            return None;
        }
        
        let mut queue = self.jobs.write();
        let job = queue.pop_front();
        
        if job.is_some() {
            *self.total_processed.write() += 1;
        }
        
        job
    }

    pub fn peek_next_job(&self) -> Option<PrintJob> {
        self.jobs.read().front().cloned()
    }

    pub fn remove_job(&self, job_id: u32) -> Result<(), &'static str> {
        let mut queue = self.jobs.write();
        if let Some(pos) = queue.iter().position(|j| j.id == job_id) {
            queue.remove(pos);
            Ok(())
        } else {
            Err("Job not found in queue")
        }
    }

    pub fn pause(&self) {
        *self.paused.write() = true;
    }

    pub fn resume(&self) {
        *self.paused.write() = false;
    }

    pub fn is_paused(&self) -> bool {
        *self.paused.read()
    }

    pub fn clear(&self) {
        self.jobs.write().clear();
    }

    pub fn get_job_count(&self) -> usize {
        self.jobs.read().len()
    }

    pub fn get_jobs(&self) -> Vec<PrintJob> {
        self.jobs.read().iter().cloned().collect()
    }

    pub fn get_total_processed(&self) -> u64 {
        *self.total_processed.read()
    }

    pub fn reorder_job(&self, job_id: u32, new_position: usize) -> Result<(), &'static str> {
        let mut queue = self.jobs.write();
        
        if let Some(current_pos) = queue.iter().position(|j| j.id == job_id) {
            if new_position >= queue.len() {
                return Err("Invalid position");
            }
            
            let job = queue.remove(current_pos).unwrap();
            queue.insert(new_position, job);
            Ok(())
        } else {
            Err("Job not found")
        }
    }

    pub fn promote_job(&self, job_id: u32) -> Result<(), &'static str> {
        let mut queue = self.jobs.write();
        
        if let Some(pos) = queue.iter().position(|j| j.id == job_id) {
            if pos > 0 {
                queue.swap(pos, pos - 1);
            }
            Ok(())
        } else {
            Err("Job not found")
        }
    }

    pub fn demote_job(&self, job_id: u32) -> Result<(), &'static str> {
        let mut queue = self.jobs.write();
        
        if let Some(pos) = queue.iter().position(|j| j.id == job_id) {
            if pos < queue.len() - 1 {
                queue.swap(pos, pos + 1);
            }
            Ok(())
        } else {
            Err("Job not found")
        }
    }
}

pub struct QueueManager {
    queues: RwLock<Vec<PrintQueue>>,
    default_queue: RwLock<Option<u32>>,
}

impl QueueManager {
    pub fn new() -> Self {
        Self {
            queues: RwLock::new(Vec::new()),
            default_queue: RwLock::new(None),
        }
    }

    pub fn create_queue(&self, name: String) -> u32 {
        let queue_id = self.queues.read().len() as u32;
        let queue = PrintQueue::new(queue_id, name);
        
        if self.default_queue.read().is_none() {
            *self.default_queue.write() = Some(queue_id);
        }
        
        self.queues.write().push(queue);
        queue_id
    }

    pub fn get_queue(&self, queue_id: u32) -> Option<&PrintQueue> {
        let queues = unsafe { &*self.queues.data_ptr() };
        queues.get(queue_id as usize)
    }

    pub fn get_default_queue(&self) -> Option<&PrintQueue> {
        if let Some(id) = *self.default_queue.read() {
            self.get_queue(id)
        } else {
            None
        }
    }

    pub fn set_default_queue(&self, queue_id: u32) -> Result<(), &'static str> {
        if (queue_id as usize) < self.queues.read().len() {
            *self.default_queue.write() = Some(queue_id);
            Ok(())
        } else {
            Err("Queue not found")
        }
    }

    pub fn list_queues(&self) -> Vec<(u32, String)> {
        self.queues.read()
            .iter()
            .map(|q| (q.queue_id, q.name.clone()))
            .collect()
    }

    pub fn get_total_job_count(&self) -> usize {
        self.queues.read()
            .iter()
            .map(|q| q.get_job_count())
            .sum()
    }
}