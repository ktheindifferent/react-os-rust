use core::sync::atomic::{AtomicU64, AtomicUsize, AtomicBool, Ordering, fence};
use core::cell::UnsafeCell;
use core::ptr::NonNull;
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;
use crate::smp::{percpu, ipi, MAX_CPUS};

pub struct RcuGracePeriod {
    current_gp: AtomicU64,
    completed_gp: AtomicU64,
    online_cpus: AtomicUsize,
    quiescent_cpus: AtomicUsize,
    gp_in_progress: AtomicBool,
}

impl RcuGracePeriod {
    pub fn new() -> Self {
        Self {
            current_gp: AtomicU64::new(0),
            completed_gp: AtomicU64::new(0),
            online_cpus: AtomicUsize::new(1),
            quiescent_cpus: AtomicUsize::new(0),
            gp_in_progress: AtomicBool::new(false),
        }
    }

    pub fn start_grace_period(&self) -> u64 {
        while self.gp_in_progress.compare_exchange_weak(
            false,
            true,
            Ordering::Acquire,
            Ordering::Relaxed
        ).is_err() {
            core::hint::spin_loop();
        }
        
        let gp = self.current_gp.fetch_add(1, Ordering::Release);
        self.quiescent_cpus.store(0, Ordering::Release);
        
        fence(Ordering::SeqCst);
        
        ipi::send_ipi_all_but_self(0xF6);
        
        gp
    }

    pub fn end_grace_period(&self, gp: u64) {
        while self.quiescent_cpus.load(Ordering::Acquire) < self.online_cpus.load(Ordering::Acquire) {
            core::hint::spin_loop();
        }
        
        self.completed_gp.store(gp, Ordering::Release);
        self.gp_in_progress.store(false, Ordering::Release);
    }

    pub fn report_quiescent_state(&self) {
        self.quiescent_cpus.fetch_add(1, Ordering::Release);
    }

    pub fn wait_for_grace_period(&self, gp: u64) {
        while self.completed_gp.load(Ordering::Acquire) < gp {
            core::hint::spin_loop();
        }
    }
}

lazy_static! {
    static ref RCU_GP: RcuGracePeriod = RcuGracePeriod::new();
}

pub fn rcu_read_lock() {
    percpu::preempt_disable();
    fence(Ordering::Acquire);
}

pub fn rcu_read_unlock() {
    fence(Ordering::Release);
    percpu::preempt_enable();
}

pub fn synchronize_rcu() {
    let gp = RCU_GP.start_grace_period();
    RCU_GP.end_grace_period(gp);
}

pub fn call_rcu<F: FnOnce() + Send + 'static>(callback: F) {
    struct RcuCallback {
        func: Box<dyn FnOnce() + Send>,
        gp: u64,
    }
    
    lazy_static! {
        static ref RCU_CALLBACKS: Mutex<Vec<RcuCallback>> = Mutex::new(Vec::new());
    }
    
    let gp = RCU_GP.current_gp.load(Ordering::Acquire);
    let callback = RcuCallback {
        func: Box::new(callback),
        gp,
    };
    
    RCU_CALLBACKS.lock().push(callback);
}

pub fn rcu_barrier() {
    synchronize_rcu();
}

pub fn rcu_quiescent_state() {
    RCU_GP.report_quiescent_state();
}

pub struct RcuPointer<T> {
    ptr: AtomicUsize,
    _marker: core::marker::PhantomData<T>,
}

impl<T> RcuPointer<T> {
    pub fn new(value: T) -> Self {
        let boxed = Box::new(value);
        let ptr = Box::into_raw(boxed) as usize;
        
        Self {
            ptr: AtomicUsize::new(ptr),
            _marker: core::marker::PhantomData,
        }
    }

    pub fn load(&self) -> Option<&T> {
        let ptr = self.ptr.load(Ordering::Acquire);
        if ptr == 0 {
            None
        } else {
            unsafe { Some(&*(ptr as *const T)) }
        }
    }

    pub fn update(&self, new_value: T) {
        let new_boxed = Box::new(new_value);
        let new_ptr = Box::into_raw(new_boxed) as usize;
        
        let old_ptr = self.ptr.swap(new_ptr, Ordering::Release);
        
        if old_ptr != 0 {
            call_rcu(move || {
                unsafe {
                    let _ = Box::from_raw(old_ptr as *mut T);
                }
            });
        }
    }

    pub fn replace(&self, new_value: Option<T>) -> Option<T> {
        let new_ptr = if let Some(value) = new_value {
            let boxed = Box::new(value);
            Box::into_raw(boxed) as usize
        } else {
            0
        };
        
        let old_ptr = self.ptr.swap(new_ptr, Ordering::AcqRel);
        
        if old_ptr == 0 {
            None
        } else {
            unsafe { Some(*Box::from_raw(old_ptr as *mut T)) }
        }
    }
}

unsafe impl<T: Send> Send for RcuPointer<T> {}
unsafe impl<T: Send + Sync> Sync for RcuPointer<T> {}

pub struct RcuList<T> {
    head: AtomicUsize,
    _marker: core::marker::PhantomData<T>,
}

struct RcuListNode<T> {
    value: T,
    next: AtomicUsize,
}

impl<T> RcuList<T> {
    pub const fn new() -> Self {
        Self {
            head: AtomicUsize::new(0),
            _marker: core::marker::PhantomData,
        }
    }

    pub fn push_front(&self, value: T) {
        let node = Box::new(RcuListNode {
            value,
            next: AtomicUsize::new(0),
        });
        let node_ptr = Box::into_raw(node) as usize;
        
        loop {
            let head = self.head.load(Ordering::Acquire);
            unsafe {
                (*(node_ptr as *mut RcuListNode<T>)).next.store(head, Ordering::Relaxed);
            }
            
            if self.head.compare_exchange_weak(
                head,
                node_ptr,
                Ordering::Release,
                Ordering::Acquire
            ).is_ok() {
                break;
            }
        }
    }

    pub fn iter(&self) -> RcuListIter<T> {
        RcuListIter {
            current: self.head.load(Ordering::Acquire),
            _marker: core::marker::PhantomData,
        }
    }

    pub fn remove<F>(&self, predicate: F) -> Option<T>
    where
        F: Fn(&T) -> bool,
    {
        let mut prev_ptr = &self.head;
        let mut current = prev_ptr.load(Ordering::Acquire);
        
        while current != 0 {
            unsafe {
                let node = &*(current as *const RcuListNode<T>);
                if predicate(&node.value) {
                    let next = node.next.load(Ordering::Acquire);
                    
                    if prev_ptr.compare_exchange(
                        current,
                        next,
                        Ordering::Release,
                        Ordering::Acquire
                    ).is_ok() {
                        let removed = Box::from_raw(current as *mut RcuListNode<T>);
                        return Some(removed.value);
                    }
                }
                
                prev_ptr = &node.next;
                current = node.next.load(Ordering::Acquire);
            }
        }
        
        None
    }
}

pub struct RcuListIter<T> {
    current: usize,
    _marker: core::marker::PhantomData<T>,
}

impl<T> Iterator for RcuListIter<T> {
    type Item = *const T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current == 0 {
            None
        } else {
            unsafe {
                let node = &*(self.current as *const RcuListNode<T>);
                let value_ptr = &node.value as *const T;
                self.current = node.next.load(Ordering::Acquire);
                Some(value_ptr)
            }
        }
    }
}

unsafe impl<T: Send> Send for RcuList<T> {}
unsafe impl<T: Send + Sync> Sync for RcuList<T> {}

#[macro_export]
macro_rules! rcu_dereference {
    ($ptr:expr) => {{
        $crate::sync::rcu::rcu_read_lock();
        let result = $ptr.load();
        result
    }};
}

#[macro_export]
macro_rules! rcu_assign_pointer {
    ($ptr:expr, $value:expr) => {{
        $ptr.update($value);
        $crate::sync::rcu::synchronize_rcu();
    }};
}

pub fn init_rcu() {
    ipi::register_ipi_handler(0xF6, handle_rcu_ipi);
}

fn handle_rcu_ipi() {
    rcu_quiescent_state();
}