use super::{ThreadId, ProcessId};
use super::thread::{Thread, ThreadState, THREAD_MANAGER};
use alloc::vec::Vec;
use alloc::collections::VecDeque;
use core::sync::atomic::{AtomicU32, AtomicBool, Ordering};
use spin::Mutex;
use lazy_static::lazy_static;
use crate::smp::{MAX_CPUS, percpu, ipi};

const DEFAULT_TIME_SLICE: u32 = 10;
const MIN_TIME_SLICE: u32 = 1;
const MAX_TIME_SLICE: u32 = 100;
const LOAD_BALANCE_PERIOD: u32 = 100;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SchedulerPolicy {
    RoundRobin,
    Priority,
    CFS,
    RealTime,
}

pub struct RunQueue {
    ready_queue: VecDeque<ThreadId>,
    expired_queue: VecDeque<ThreadId>,
    rt_queue: VecDeque<ThreadId>,
    idle_thread: Option<ThreadId>,
    nr_running: AtomicU32,
    cpu_load: AtomicU32,
    last_balance: AtomicU32,
}

impl RunQueue {
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
            expired_queue: VecDeque::new(),
            rt_queue: VecDeque::new(),
            idle_thread: None,
            nr_running: AtomicU32::new(0),
            cpu_load: AtomicU32::new(0),
            last_balance: AtomicU32::new(0),
        }
    }

    pub fn enqueue(&mut self, thread_id: ThreadId, priority: u8) {
        if priority >= 100 {
            self.rt_queue.push_back(thread_id);
        } else {
            self.ready_queue.push_back(thread_id);
        }
        self.nr_running.fetch_add(1, Ordering::Relaxed);
    }

    pub fn dequeue(&mut self) -> Option<ThreadId> {
        if let Some(thread) = self.rt_queue.pop_front() {
            self.nr_running.fetch_sub(1, Ordering::Relaxed);
            return Some(thread);
        }
        
        if let Some(thread) = self.ready_queue.pop_front() {
            self.nr_running.fetch_sub(1, Ordering::Relaxed);
            return Some(thread);
        }
        
        if self.ready_queue.is_empty() && !self.expired_queue.is_empty() {
            core::mem::swap(&mut self.ready_queue, &mut self.expired_queue);
            if let Some(thread) = self.ready_queue.pop_front() {
                self.nr_running.fetch_sub(1, Ordering::Relaxed);
                return Some(thread);
            }
        }
        
        self.idle_thread
    }

    pub fn requeue(&mut self, thread_id: ThreadId) {
        self.expired_queue.push_back(thread_id);
    }

    pub fn remove(&mut self, thread_id: ThreadId) -> bool {
        if let Some(pos) = self.rt_queue.iter().position(|&id| id == thread_id) {
            self.rt_queue.remove(pos);
            self.nr_running.fetch_sub(1, Ordering::Relaxed);
            return true;
        }
        
        if let Some(pos) = self.ready_queue.iter().position(|&id| id == thread_id) {
            self.ready_queue.remove(pos);
            self.nr_running.fetch_sub(1, Ordering::Relaxed);
            return true;
        }
        
        if let Some(pos) = self.expired_queue.iter().position(|&id| id == thread_id) {
            self.expired_queue.remove(pos);
            return true;
        }
        
        false
    }

    pub fn is_empty(&self) -> bool {
        self.ready_queue.is_empty() && 
        self.expired_queue.is_empty() && 
        self.rt_queue.is_empty()
    }

    pub fn load(&self) -> u32 {
        self.nr_running.load(Ordering::Relaxed)
    }

    pub fn update_load(&self) {
        let current_load = self.nr_running.load(Ordering::Relaxed);
        let old_load = self.cpu_load.load(Ordering::Relaxed);
        let new_load = (old_load * 3 + current_load) / 4;
        self.cpu_load.store(new_load, Ordering::Relaxed);
    }
}

pub struct SmpScheduler {
    run_queues: Vec<Mutex<RunQueue>>,
    current_threads: Vec<AtomicU32>,
    time_slices: Vec<AtomicU32>,
    policy: SchedulerPolicy,
    load_balance_tick: AtomicU32,
}

impl SmpScheduler {
    pub fn new() -> Self {
        let mut run_queues = Vec::with_capacity(MAX_CPUS);
        let mut current_threads = Vec::with_capacity(MAX_CPUS);
        let mut time_slices = Vec::with_capacity(MAX_CPUS);
        
        for _ in 0..MAX_CPUS {
            run_queues.push(Mutex::new(RunQueue::new()));
            current_threads.push(AtomicU32::new(0));
            time_slices.push(AtomicU32::new(DEFAULT_TIME_SLICE));
        }
        
        Self {
            run_queues,
            current_threads,
            time_slices,
            policy: SchedulerPolicy::Priority,
            load_balance_tick: AtomicU32::new(0),
        }
    }

    pub fn tick(&self, cpu_id: u32) -> Option<ThreadId> {
        let time_slice = &self.time_slices[cpu_id as usize];
        let remaining = time_slice.fetch_sub(1, Ordering::Relaxed);
        
        if remaining <= 1 {
            time_slice.store(DEFAULT_TIME_SLICE, Ordering::Relaxed);
            self.schedule(cpu_id)
        } else {
            if self.load_balance_tick.fetch_add(1, Ordering::Relaxed) % LOAD_BALANCE_PERIOD == 0 {
                self.load_balance(cpu_id);
            }
            None
        }
    }

    pub fn schedule(&self, cpu_id: u32) -> Option<ThreadId> {
        let current = self.current_threads[cpu_id as usize].load(Ordering::Relaxed);
        
        if current != 0 {
            let mut rq = self.run_queues[cpu_id as usize].lock();
            rq.requeue(current);
        }
        
        let mut rq = self.run_queues[cpu_id as usize].lock();
        if let Some(next) = rq.dequeue() {
            self.current_threads[cpu_id as usize].store(next, Ordering::Relaxed);
            
            percpu::clear_need_resched();
            
            Some(next)
        } else {
            self.current_threads[cpu_id as usize].store(0, Ordering::Relaxed);
            None
        }
    }

    pub fn enqueue_thread(&self, thread_id: ThreadId, cpu_affinity: Option<u32>) {
        let target_cpu = if let Some(cpu) = cpu_affinity {
            cpu
        } else {
            self.find_least_loaded_cpu()
        };
        
        let mut rq = self.run_queues[target_cpu as usize].lock();
        rq.enqueue(thread_id, 50);
        
        if target_cpu != percpu::get_cpu_id() {
            ipi::send_reschedule_ipi(target_cpu);
        }
    }

    pub fn dequeue_thread(&self, thread_id: ThreadId) {
        for cpu in 0..MAX_CPUS {
            let mut rq = self.run_queues[cpu].lock();
            if rq.remove(thread_id) {
                break;
            }
        }
    }

    pub fn yield_thread(&self, cpu_id: u32) {
        self.time_slices[cpu_id as usize].store(0, Ordering::Relaxed);
        percpu::set_need_resched();
    }

    pub fn set_thread_affinity(&self, thread_id: ThreadId, cpu_mask: u64) {
        let current_cpu = self.find_thread_cpu(thread_id);
        
        if let Some(current) = current_cpu {
            if (cpu_mask & (1 << current)) == 0 {
                let new_cpu = self.find_first_cpu_in_mask(cpu_mask);
                if let Some(new) = new_cpu {
                    self.migrate_thread(thread_id, current, new);
                }
            }
        }
    }

    pub fn get_thread_affinity(&self, thread_id: ThreadId) -> u64 {
        let thread_manager = THREAD_MANAGER.lock();
        if let Some(thread) = thread_manager.get_thread(thread_id) {
            thread.cpu_affinity
        } else {
            0
        }
    }

    fn find_least_loaded_cpu(&self) -> u32 {
        let mut min_load = u32::MAX;
        let mut best_cpu = 0;
        
        for cpu in 0..crate::smp::SMP_MANAGER.lock().online_cpu_count() {
            let rq = self.run_queues[cpu as usize].lock();
            let load = rq.load();
            if load < min_load {
                min_load = load;
                best_cpu = cpu;
            }
        }
        
        best_cpu
    }

    fn find_thread_cpu(&self, thread_id: ThreadId) -> Option<u32> {
        for cpu in 0..MAX_CPUS {
            let rq = self.run_queues[cpu].lock();
            if rq.ready_queue.contains(&thread_id) || 
               rq.expired_queue.contains(&thread_id) ||
               rq.rt_queue.contains(&thread_id) {
                return Some(cpu as u32);
            }
        }
        None
    }

    fn find_first_cpu_in_mask(&self, cpu_mask: u64) -> Option<u32> {
        for cpu in 0..64 {
            if (cpu_mask & (1 << cpu)) != 0 {
                if crate::smp::cpu_online(cpu) {
                    return Some(cpu);
                }
            }
        }
        None
    }

    fn migrate_thread(&self, thread_id: ThreadId, from_cpu: u32, to_cpu: u32) {
        let mut from_rq = self.run_queues[from_cpu as usize].lock();
        if from_rq.remove(thread_id) {
            drop(from_rq);
            
            let mut to_rq = self.run_queues[to_cpu as usize].lock();
            to_rq.enqueue(thread_id, 50);
            
            if to_cpu != percpu::get_cpu_id() {
                ipi::send_reschedule_ipi(to_cpu);
            }
        }
    }

    fn load_balance(&self, cpu_id: u32) {
        let local_rq = self.run_queues[cpu_id as usize].lock();
        let local_load = local_rq.load();
        drop(local_rq);
        
        if local_load == 0 {
            for other_cpu in 0..crate::smp::SMP_MANAGER.lock().online_cpu_count() {
                if other_cpu == cpu_id {
                    continue;
                }
                
                let other_rq = self.run_queues[other_cpu as usize].lock();
                let other_load = other_rq.load();
                drop(other_rq);
                
                if other_load > 1 {
                    self.pull_task(cpu_id, other_cpu);
                    break;
                }
            }
        } else {
            let avg_load = self.calculate_average_load();
            
            if local_load > avg_load + 1 {
                for other_cpu in 0..crate::smp::SMP_MANAGER.lock().online_cpu_count() {
                    if other_cpu == cpu_id {
                        continue;
                    }
                    
                    let other_rq = self.run_queues[other_cpu as usize].lock();
                    let other_load = other_rq.load();
                    drop(other_rq);
                    
                    if other_load < avg_load {
                        self.push_task(cpu_id, other_cpu);
                        break;
                    }
                }
            }
        }
    }

    fn calculate_average_load(&self) -> u32 {
        let mut total_load = 0;
        let cpu_count = crate::smp::SMP_MANAGER.lock().online_cpu_count();
        
        for cpu in 0..cpu_count {
            let rq = self.run_queues[cpu as usize].lock();
            total_load += rq.load();
        }
        
        if cpu_count > 0 {
            total_load / cpu_count
        } else {
            0
        }
    }

    fn pull_task(&self, to_cpu: u32, from_cpu: u32) {
        let mut from_rq = self.run_queues[from_cpu as usize].lock();
        if let Some(thread_id) = from_rq.ready_queue.pop_back() {
            from_rq.nr_running.fetch_sub(1, Ordering::Relaxed);
            drop(from_rq);
            
            let mut to_rq = self.run_queues[to_cpu as usize].lock();
            to_rq.enqueue(thread_id, 50);
        }
    }

    fn push_task(&self, from_cpu: u32, to_cpu: u32) {
        let mut from_rq = self.run_queues[from_cpu as usize].lock();
        if let Some(thread_id) = from_rq.ready_queue.pop_back() {
            from_rq.nr_running.fetch_sub(1, Ordering::Relaxed);
            drop(from_rq);
            
            let mut to_rq = self.run_queues[to_cpu as usize].lock();
            to_rq.enqueue(thread_id, 50);
            
            if to_cpu != percpu::get_cpu_id() {
                ipi::send_reschedule_ipi(to_cpu);
            }
        }
    }
}

lazy_static! {
    pub static ref SMP_SCHEDULER: SmpScheduler = SmpScheduler::new();
}

pub fn schedule() {
    let cpu_id = percpu::get_cpu_id();
    
    if let Some(next_thread) = SMP_SCHEDULER.schedule(cpu_id) {
        crate::process::context_switch::switch_to_thread(next_thread);
    }
}

pub fn tick() {
    let cpu_id = percpu::get_cpu_id();
    
    if let Some(next_thread) = SMP_SCHEDULER.tick(cpu_id) {
        crate::process::context_switch::switch_to_thread(next_thread);
    }
}

pub fn yield_current() {
    let cpu_id = percpu::get_cpu_id();
    SMP_SCHEDULER.yield_thread(cpu_id);
    schedule();
}

pub fn enqueue_thread(thread_id: ThreadId) {
    SMP_SCHEDULER.enqueue_thread(thread_id, None);
}

pub fn dequeue_thread(thread_id: ThreadId) {
    SMP_SCHEDULER.dequeue_thread(thread_id);
}

pub fn set_thread_cpu_affinity(thread_id: ThreadId, cpu_mask: u64) {
    SMP_SCHEDULER.set_thread_affinity(thread_id, cpu_mask);
}