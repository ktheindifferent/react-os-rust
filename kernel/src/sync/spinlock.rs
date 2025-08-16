use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::fmt;
use core::time::Duration;

// Deadlock detection threshold (in TSC cycles)
const DEADLOCK_THRESHOLD_CYCLES: u64 = 10_000_000_000; // ~5 seconds on a 2GHz CPU

pub struct SpinLock<T> {
    lock: AtomicBool,
    value: UnsafeCell<T>,
    owner: AtomicU32,
    recursion_count: AtomicU32,
    #[cfg(debug_assertions)]
    lock_file: AtomicU64,  // Store file:line info for debugging
    #[cfg(debug_assertions)]
    lock_time: AtomicU64,  // Store when lock was acquired
}

unsafe impl<T: Send> Sync for SpinLock<T> {}
unsafe impl<T: Send> Send for SpinLock<T> {}

impl<T> SpinLock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            lock: AtomicBool::new(false),
            value: UnsafeCell::new(value),
            owner: AtomicU32::new(0),
            recursion_count: AtomicU32::new(0),
            #[cfg(debug_assertions)]
            lock_file: AtomicU64::new(0),
            #[cfg(debug_assertions)]
            lock_time: AtomicU64::new(0),
        }
    }

    pub fn lock(&self) -> SpinLockGuard<T> {
        let cpu_id = get_cpu_id();
        
        // Check for recursive lock
        if self.owner.load(Ordering::Relaxed) == cpu_id {
            self.recursion_count.fetch_add(1, Ordering::Relaxed);
            return SpinLockGuard {
                lock: self,
                is_recursive: true,
            };
        }
        
        #[cfg(debug_assertions)]
        let start_time = crate::cpu::rdtsc();
        #[cfg(debug_assertions)]
        let mut spin_count = 0u64;
        
        // Spin until we acquire the lock
        while self.lock.compare_exchange_weak(
            false,
            true,
            Ordering::Acquire,
            Ordering::Relaxed
        ).is_err() {
            #[cfg(debug_assertions)]
            {
                spin_count += 1;
                // Check for potential deadlock every 1000 spins
                if spin_count % 1000 == 0 {
                    let elapsed = crate::cpu::rdtsc() - start_time;
                    if elapsed > DEADLOCK_THRESHOLD_CYCLES {
                        let owner = self.owner.load(Ordering::Relaxed);
                        crate::serial_println!(
                            "WARNING: Potential deadlock detected! Lock held by CPU {} for {} cycles",
                            owner, elapsed
                        );
                        // In debug mode, panic to help identify the issue
                        panic!("Deadlock detected in SpinLock");
                    }
                }
            }
            core::hint::spin_loop();
        }
        
        self.owner.store(cpu_id, Ordering::Relaxed);
        
        #[cfg(debug_assertions)]
        {
            self.lock_time.store(crate::cpu::rdtsc(), Ordering::Relaxed);
        }
        
        SpinLockGuard {
            lock: self,
            is_recursive: false,
        }
    }

    pub fn try_lock(&self) -> Option<SpinLockGuard<T>> {
        let cpu_id = get_cpu_id();
        
        // Check for recursive lock
        if self.owner.load(Ordering::Relaxed) == cpu_id {
            self.recursion_count.fetch_add(1, Ordering::Relaxed);
            return Some(SpinLockGuard {
                lock: self,
                is_recursive: true,
            });
        }
        
        if self.lock.compare_exchange(
            false,
            true,
            Ordering::Acquire,
            Ordering::Relaxed
        ).is_ok() {
            self.owner.store(cpu_id, Ordering::Relaxed);
            Some(SpinLockGuard {
                lock: self,
                is_recursive: false,
            })
        } else {
            None
        }
    }

    pub fn try_lock_for(&self, timeout: Duration) -> Option<SpinLockGuard<T>> {
        let start = crate::cpu::rdtsc();
        let timeout_cycles = timeout.as_nanos() as u64 * get_tsc_frequency() / 1_000_000_000;
        
        loop {
            if let Some(guard) = self.try_lock() {
                return Some(guard);
            }
            
            if crate::cpu::rdtsc() - start > timeout_cycles {
                return None;
            }
            
            core::hint::spin_loop();
        }
    }

    pub fn is_locked(&self) -> bool {
        self.lock.load(Ordering::Relaxed)
    }

    pub unsafe fn force_unlock(&self) {
        self.lock.store(false, Ordering::Release);
        self.owner.store(0, Ordering::Relaxed);
        self.recursion_count.store(0, Ordering::Relaxed);
    }
}

pub struct SpinLockGuard<'a, T> {
    lock: &'a SpinLock<T>,
    is_recursive: bool,
}

impl<'a, T> Drop for SpinLockGuard<'a, T> {
    fn drop(&mut self) {
        if self.is_recursive {
            let count = self.lock.recursion_count.fetch_sub(1, Ordering::Relaxed);
            if count > 1 {
                return;
            }
        }
        
        self.lock.owner.store(0, Ordering::Relaxed);
        self.lock.lock.store(false, Ordering::Release);
    }
}

impl<'a, T> Deref for SpinLockGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.value.get() }
    }
}

impl<'a, T> DerefMut for SpinLockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.value.get() }
    }
}

impl<'a, T: fmt::Debug> fmt::Debug for SpinLockGuard<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

pub struct RwSpinLock<T> {
    readers: AtomicU32,
    writer: AtomicBool,
    value: UnsafeCell<T>,
}

unsafe impl<T: Send + Sync> Sync for RwSpinLock<T> {}
unsafe impl<T: Send> Send for RwSpinLock<T> {}

impl<T> RwSpinLock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            readers: AtomicU32::new(0),
            writer: AtomicBool::new(false),
            value: UnsafeCell::new(value),
        }
    }

    pub fn read(&self) -> RwSpinLockReadGuard<T> {
        loop {
            // Wait for writer to finish
            while self.writer.load(Ordering::Relaxed) {
                core::hint::spin_loop();
            }
            
            // Increment reader count
            self.readers.fetch_add(1, Ordering::Acquire);
            
            // Check if writer grabbed lock while we were incrementing
            if !self.writer.load(Ordering::Relaxed) {
                break;
            }
            
            // Writer got lock, back off
            self.readers.fetch_sub(1, Ordering::Release);
        }
        
        RwSpinLockReadGuard { lock: self }
    }

    pub fn write(&self) -> RwSpinLockWriteGuard<T> {
        // Acquire writer lock
        while self.writer.compare_exchange_weak(
            false,
            true,
            Ordering::Acquire,
            Ordering::Relaxed
        ).is_err() {
            core::hint::spin_loop();
        }
        
        // Wait for all readers to finish
        while self.readers.load(Ordering::Relaxed) != 0 {
            core::hint::spin_loop();
        }
        
        RwSpinLockWriteGuard { lock: self }
    }

    pub fn try_read(&self) -> Option<RwSpinLockReadGuard<T>> {
        if self.writer.load(Ordering::Relaxed) {
            return None;
        }
        
        self.readers.fetch_add(1, Ordering::Acquire);
        
        if self.writer.load(Ordering::Relaxed) {
            self.readers.fetch_sub(1, Ordering::Release);
            None
        } else {
            Some(RwSpinLockReadGuard { lock: self })
        }
    }

    pub fn try_write(&self) -> Option<RwSpinLockWriteGuard<T>> {
        if self.writer.compare_exchange(
            false,
            true,
            Ordering::Acquire,
            Ordering::Relaxed
        ).is_ok() {
            if self.readers.load(Ordering::Relaxed) == 0 {
                Some(RwSpinLockWriteGuard { lock: self })
            } else {
                self.writer.store(false, Ordering::Release);
                None
            }
        } else {
            None
        }
    }
}

pub struct RwSpinLockReadGuard<'a, T> {
    lock: &'a RwSpinLock<T>,
}

impl<'a, T> Drop for RwSpinLockReadGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.readers.fetch_sub(1, Ordering::Release);
    }
}

impl<'a, T> Deref for RwSpinLockReadGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.value.get() }
    }
}

pub struct RwSpinLockWriteGuard<'a, T> {
    lock: &'a RwSpinLock<T>,
}

impl<'a, T> Drop for RwSpinLockWriteGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.writer.store(false, Ordering::Release);
    }
}

impl<'a, T> Deref for RwSpinLockWriteGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.value.get() }
    }
}

impl<'a, T> DerefMut for RwSpinLockWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.value.get() }
    }
}

fn get_cpu_id() -> u32 {
    unsafe {
        let mut id: u32;
        core::arch::asm!(
            "mov {}, gs:0",
            out(reg) id,
            options(nomem, nostack, preserves_flags)
        );
        id
    }
}

fn get_tsc_frequency() -> u64 {
    // Approximate TSC frequency - would need calibration in real system
    2_000_000_000 // 2 GHz default
}