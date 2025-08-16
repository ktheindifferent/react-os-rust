use alloc::{collections::VecDeque, vec::Vec, string::String};
use spin::RwLock;
use crate::task::{Task, spawn};
use super::job::{PrintJob, JobStatus, JobPriority};

pub struct PrintSpooler {
    job_queue: RwLock<VecDeque<PrintJob>>,
    active_jobs: RwLock<Vec<PrintJob>>,
    completed_jobs: RwLock<Vec<PrintJob>>,
    paused: RwLock<bool>,
    max_concurrent_jobs: usize,
}

impl PrintSpooler {
    pub fn new() -> Self {
        Self {
            job_queue: RwLock::new(VecDeque::new()),
            active_jobs: RwLock::new(Vec::new()),
            completed_jobs: RwLock::new(Vec::new()),
            paused: RwLock::new(false),
            max_concurrent_jobs: 4,
        }
    }

    pub fn start(&self) -> Result<(), &'static str> {
        spawn(Task::new("print_spooler", || {
            loop {
                self.process_queue();
                crate::arch::x86_64::hlt();
            }
        }));
        Ok(())
    }

    pub fn add_job(&self, mut job: PrintJob) -> Result<(), &'static str> {
        job.status = JobStatus::Queued;
        job.queued_time = crate::time::get_timestamp();
        
        let mut queue = self.job_queue.write();
        
        match job.priority {
            JobPriority::High => {
                queue.push_front(job);
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

    pub fn cancel_job(&self, job_id: u32) -> Result<(), &'static str> {
        {
            let mut queue = self.job_queue.write();
            if let Some(pos) = queue.iter().position(|j| j.id == job_id) {
                queue.remove(pos);
                return Ok(());
            }
        }
        
        {
            let mut active = self.active_jobs.write();
            if let Some(pos) = active.iter().position(|j| j.id == job_id) {
                active[pos].status = JobStatus::Cancelled;
                return Ok(());
            }
        }
        
        Err("Job not found")
    }

    pub fn pause_job(&self, job_id: u32) -> Result<(), &'static str> {
        let mut queue = self.job_queue.write();
        if let Some(job) = queue.iter_mut().find(|j| j.id == job_id) {
            job.status = JobStatus::Paused;
            Ok(())
        } else {
            Err("Job not found or already processing")
        }
    }

    pub fn resume_job(&self, job_id: u32) -> Result<(), &'static str> {
        let mut queue = self.job_queue.write();
        if let Some(job) = queue.iter_mut().find(|j| j.id == job_id) {
            job.status = JobStatus::Queued;
            Ok(())
        } else {
            Err("Job not found")
        }
    }

    pub fn reorder_job(&self, job_id: u32, new_position: usize) -> Result<(), &'static str> {
        let mut queue = self.job_queue.write();
        if let Some(pos) = queue.iter().position(|j| j.id == job_id) {
            if new_position >= queue.len() {
                return Err("Invalid position");
            }
            let job = queue.remove(pos).unwrap();
            queue.insert(new_position, job);
            Ok(())
        } else {
            Err("Job not found")
        }
    }

    pub fn get_job_status(&self, job_id: u32) -> Option<JobStatus> {
        if let Some(job) = self.job_queue.read().iter().find(|j| j.id == job_id) {
            return Some(job.status);
        }
        
        if let Some(job) = self.active_jobs.read().iter().find(|j| j.id == job_id) {
            return Some(job.status);
        }
        
        if let Some(job) = self.completed_jobs.read().iter().find(|j| j.id == job_id) {
            return Some(job.status);
        }
        
        None
    }

    pub fn get_queue_length(&self) -> usize {
        self.job_queue.read().len()
    }

    pub fn get_active_jobs(&self) -> Vec<PrintJob> {
        self.active_jobs.read().clone()
    }

    pub fn get_queued_jobs(&self) -> Vec<PrintJob> {
        self.job_queue.read().iter().cloned().collect()
    }

    pub fn get_completed_jobs(&self) -> Vec<PrintJob> {
        self.completed_jobs.read().clone()
    }

    pub fn clear_completed_jobs(&self) {
        self.completed_jobs.write().clear();
    }

    pub fn pause_spooler(&self) {
        *self.paused.write() = true;
    }

    pub fn resume_spooler(&self) {
        *self.paused.write() = false;
    }

    pub fn is_paused(&self) -> bool {
        *self.paused.read()
    }

    fn process_queue(&self) {
        if self.is_paused() {
            return;
        }
        
        let active_count = self.active_jobs.read().len();
        if active_count >= self.max_concurrent_jobs {
            return;
        }
        
        let mut queue = self.job_queue.write();
        if let Some(mut job) = queue.pop_front() {
            if job.status == JobStatus::Paused {
                queue.push_front(job);
                return;
            }
            
            job.status = JobStatus::Processing;
            job.start_time = Some(crate::time::get_timestamp());
            
            let job_clone = job.clone();
            self.active_jobs.write().push(job);
            
            spawn(Task::new("print_job", move || {
                self.process_job(job_clone);
            }));
        }
    }

    fn process_job(&self, mut job: PrintJob) {
        let printer = match super::get_subsystem().read().as_ref() {
            Some(subsystem) => subsystem.get_printer(job.printer_id),
            None => None,
        };
        
        if printer.is_none() {
            job.status = JobStatus::Failed;
            job.error_message = Some(String::from("Printer not found"));
            self.complete_job(job);
            return;
        }
        
        let result = self.render_job(&job);
        
        match result {
            Ok(data) => {
                if let Err(e) = self.send_to_printer(&job, data) {
                    job.status = JobStatus::Failed;
                    job.error_message = Some(String::from(e));
                } else {
                    job.status = JobStatus::Completed;
                    job.pages_printed = job.total_pages;
                }
            }
            Err(e) => {
                job.status = JobStatus::Failed;
                job.error_message = Some(String::from(e));
            }
        }
        
        job.end_time = Some(crate::time::get_timestamp());
        self.complete_job(job);
    }

    fn render_job(&self, job: &PrintJob) -> Result<Vec<u8>, &'static str> {
        let filter_chain = super::filter::FilterChain::new();
        filter_chain.process(job)
    }

    fn send_to_printer(&self, job: &PrintJob, data: Vec<u8>) -> Result<(), &'static str> {
        let subsystem = super::get_subsystem().read();
        if let Some(subsystem) = subsystem.as_ref() {
            subsystem.drivers.send_to_printer(job.printer_id, data)
        } else {
            Err("Print subsystem not initialized")
        }
    }

    fn complete_job(&self, job: PrintJob) {
        let mut active = self.active_jobs.write();
        active.retain(|j| j.id != job.id);
        
        let mut completed = self.completed_jobs.write();
        completed.push(job);
        
        if completed.len() > 100 {
            completed.drain(0..50);
        }
    }

    pub fn get_job_progress(&self, job_id: u32) -> Option<(u32, u32)> {
        self.active_jobs.read()
            .iter()
            .find(|j| j.id == job_id)
            .map(|j| (j.pages_printed, j.total_pages))
    }

    pub fn get_estimated_wait_time(&self, job_id: u32) -> Option<u64> {
        let queue = self.job_queue.read();
        let position = queue.iter().position(|j| j.id == job_id)?;
        
        let estimated_time = queue.iter()
            .take(position)
            .map(|j| j.estimated_print_time())
            .sum();
        
        Some(estimated_time)
    }
}