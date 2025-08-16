use core::sync::atomic::{AtomicU32, AtomicBool, Ordering};
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use crate::smp::percpu;

const READER_BIAS: u32 = 0x00000001;
const WRITER_BIAS: u32 = 0x10000000;

pub struct RwLock<T> {
    lock: AtomicU32,
    writer_waiting: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for RwLock<T> {}
unsafe impl<T: Send + Sync> Sync for RwLock<T> {}

impl<T> RwLock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            lock: AtomicU32::new(0),
            writer_waiting: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    pub fn read(&self) -> RwLockReadGuard<T> {
        loop {
            if !self.writer_waiting.load(Ordering::Acquire) {
                let old = self.lock.fetch_add(READER_BIAS, Ordering::Acquire);
                
                if old < WRITER_BIAS {
                    return RwLockReadGuard { lock: self };
                }
                
                self.lock.fetch_sub(READER_BIAS, Ordering::Release);
            }
            
            while self.writer_waiting.load(Ordering::Relaxed) {
                core::hint::spin_loop();
            }
        }
    }

    pub fn try_read(&self) -> Option<RwLockReadGuard<T>> {
        if !self.writer_waiting.load(Ordering::Acquire) {
            let old = self.lock.fetch_add(READER_BIAS, Ordering::Acquire);
            
            if old < WRITER_BIAS {
                return Some(RwLockReadGuard { lock: self });
            }
            
            self.lock.fetch_sub(READER_BIAS, Ordering::Release);
        }
        
        None
    }

    pub fn write(&self) -> RwLockWriteGuard<T> {
        self.writer_waiting.store(true, Ordering::Release);
        
        loop {
            if self.lock.compare_exchange_weak(
                0,
                WRITER_BIAS,
                Ordering::Acquire,
                Ordering::Relaxed
            ).is_ok() {
                self.writer_waiting.store(false, Ordering::Release);
                return RwLockWriteGuard { lock: self };
            }
            
            core::hint::spin_loop();
        }
    }

    pub fn try_write(&self) -> Option<RwLockWriteGuard<T>> {
        if self.lock.compare_exchange(
            0,
            WRITER_BIAS,
            Ordering::Acquire,
            Ordering::Relaxed
        ).is_ok() {
            Some(RwLockWriteGuard { lock: self })
        } else {
            None
        }
    }

    pub fn get_mut(&mut self) -> &mut T {
        unsafe { &mut *self.data.get() }
    }

    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }
}

pub struct RwLockReadGuard<'a, T> {
    lock: &'a RwLock<T>,
}

impl<'a, T> Drop for RwLockReadGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.lock.fetch_sub(READER_BIAS, Ordering::Release);
    }
}

impl<'a, T> Deref for RwLockReadGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

pub struct RwLockWriteGuard<'a, T> {
    lock: &'a RwLock<T>,
}

impl<'a, T> Drop for RwLockWriteGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.lock.store(0, Ordering::Release);
    }
}

impl<'a, T> Deref for RwLockWriteGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T> DerefMut for RwLockWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

pub struct SeqLock<T: Copy> {
    sequence: AtomicU32,
    data: UnsafeCell<T>,
}

unsafe impl<T: Copy + Send> Send for SeqLock<T> {}
unsafe impl<T: Copy + Send + Sync> Sync for SeqLock<T> {}

impl<T: Copy> SeqLock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            sequence: AtomicU32::new(0),
            data: UnsafeCell::new(data),
        }
    }

    pub fn read(&self) -> T {
        loop {
            let seq1 = self.sequence.load(Ordering::Acquire);
            
            if seq1 & 1 != 0 {
                core::hint::spin_loop();
                continue;
            }
            
            core::sync::atomic::fence(Ordering::Acquire);
            
            let data = unsafe { *self.data.get() };
            
            core::sync::atomic::fence(Ordering::Acquire);
            
            let seq2 = self.sequence.load(Ordering::Acquire);
            
            if seq1 == seq2 {
                return data;
            }
            
            core::hint::spin_loop();
        }
    }

    pub fn write(&self, data: T) {
        let seq = self.sequence.fetch_add(1, Ordering::Release);
        
        core::sync::atomic::fence(Ordering::Release);
        
        unsafe {
            *self.data.get() = data;
        }
        
        core::sync::atomic::fence(Ordering::Release);
        
        self.sequence.store(seq + 2, Ordering::Release);
    }
}

pub struct TicketLock {
    next_ticket: AtomicU32,
    now_serving: AtomicU32,
}

impl TicketLock {
    pub const fn new() -> Self {
        Self {
            next_ticket: AtomicU32::new(0),
            now_serving: AtomicU32::new(0),
        }
    }

    pub fn lock(&self) -> TicketLockGuard {
        let ticket = self.next_ticket.fetch_add(1, Ordering::Relaxed);
        
        while self.now_serving.load(Ordering::Acquire) != ticket {
            core::hint::spin_loop();
        }
        
        TicketLockGuard { lock: self }
    }

    pub fn try_lock(&self) -> Option<TicketLockGuard> {
        let serving = self.now_serving.load(Ordering::Acquire);
        
        if self.next_ticket.compare_exchange(
            serving,
            serving + 1,
            Ordering::Acquire,
            Ordering::Relaxed
        ).is_ok() {
            Some(TicketLockGuard { lock: self })
        } else {
            None
        }
    }
}

pub struct TicketLockGuard<'a> {
    lock: &'a TicketLock,
}

impl<'a> Drop for TicketLockGuard<'a> {
    fn drop(&mut self) {
        self.lock.now_serving.fetch_add(1, Ordering::Release);
    }
}

pub struct McsLock {
    tail: AtomicU64,
}

pub struct McsNode {
    next: AtomicU64,
    locked: AtomicBool,
}

impl McsLock {
    pub const fn new() -> Self {
        Self {
            tail: AtomicU64::new(0),
        }
    }

    pub fn lock(&self, node: &McsNode) -> McsLockGuard {
        node.next.store(0, Ordering::Relaxed);
        node.locked.store(true, Ordering::Relaxed);
        
        let node_ptr = node as *const _ as u64;
        let prev = self.tail.swap(node_ptr, Ordering::AcqRel);
        
        if prev != 0 {
            unsafe {
                let prev_node = &*(prev as *const McsNode);
                prev_node.next.store(node_ptr, Ordering::Release);
            }
            
            while node.locked.load(Ordering::Acquire) {
                core::hint::spin_loop();
            }
        }
        
        McsLockGuard { 
            lock: self,
            node,
        }
    }
}

pub struct McsLockGuard<'a> {
    lock: &'a McsLock,
    node: &'a McsNode,
}

impl<'a> Drop for McsLockGuard<'a> {
    fn drop(&mut self) {
        let node_ptr = self.node as *const _ as u64;
        let next = self.node.next.load(Ordering::Acquire);
        
        if next == 0 {
            if self.lock.tail.compare_exchange(
                node_ptr,
                0,
                Ordering::Release,
                Ordering::Acquire
            ).is_err() {
                while self.node.next.load(Ordering::Acquire) == 0 {
                    core::hint::spin_loop();
                }
                
                let next = self.node.next.load(Ordering::Acquire);
                unsafe {
                    let next_node = &*(next as *const McsNode);
                    next_node.locked.store(false, Ordering::Release);
                }
            }
        } else {
            unsafe {
                let next_node = &*(next as *const McsNode);
                next_node.locked.store(false, Ordering::Release);
            }
        }
    }
}

impl McsNode {
    pub const fn new() -> Self {
        Self {
            next: AtomicU64::new(0),
            locked: AtomicBool::new(false),
        }
    }
}