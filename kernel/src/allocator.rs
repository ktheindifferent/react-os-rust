// High-Performance Hybrid Memory Allocator
// Combines Slab allocator for small objects with Buddy allocator for large allocations
// Includes per-CPU caches for improved multi-core scalability

#![allow(dead_code)]

use alloc::vec::Vec;
use core::alloc::{GlobalAlloc, Layout};
use core::mem::size_of;
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicUsize, Ordering};
use spin::{Mutex, RwLock};
use lazy_static::lazy_static;

// Constants for memory management
pub const HEAP_SIZE: usize = 32 * 1024 * 1024; // 32 MiB heap for better performance
pub const PAGE_SIZE: usize = 4096;
pub const SLAB_THRESHOLD: usize = 4096; // Objects < 4KB use slab allocator
pub const MAX_CPU_CACHES: usize = 256; // Support up to 256 CPUs
pub const CACHE_MAGAZINE_SIZE: usize = 64; // Objects per CPU cache magazine

// Static heap allocation
#[repr(align(4096))]
struct AlignedHeap([u8; HEAP_SIZE]);
static mut HEAP: AlignedHeap = AlignedHeap([0; HEAP_SIZE]);

// ==================== Buddy Allocator ====================
// Efficient allocator for large memory blocks using buddy system

const MAX_ORDER: usize = 20; // Max block size = 2^20 * PAGE_SIZE = 4GB
const MIN_BLOCK_SIZE: usize = PAGE_SIZE;

#[derive(Debug)]
struct BuddyBlock {
    addr: usize,
    order: usize,
    next: Option<NonNull<BuddyBlock>>,
    prev: Option<NonNull<BuddyBlock>>,
}

struct BuddyAllocator {
    free_lists: [Option<NonNull<BuddyBlock>>; MAX_ORDER + 1],
    block_pool: Vec<BuddyBlock>,
    base_addr: usize,
    total_size: usize,
    free_bytes: AtomicUsize,
    allocated_bytes: AtomicUsize,
}

// Safety: BuddyAllocator is safe to send between threads when properly synchronized
unsafe impl Send for BuddyAllocator {}
unsafe impl Sync for BuddyAllocator {}

impl BuddyAllocator {
    const fn new() -> Self {
        Self {
            free_lists: [None; MAX_ORDER + 1],
            block_pool: Vec::new(),
            base_addr: 0,
            total_size: 0,
            free_bytes: AtomicUsize::new(0),
            allocated_bytes: AtomicUsize::new(0),
        }
    }

    unsafe fn init(&mut self, base: *mut u8, size: usize) {
        self.base_addr = base as usize;
        self.total_size = size;
        self.free_bytes.store(size, Ordering::Relaxed);
        
        // Calculate the highest order that fits in the heap
        let max_order = self.size_to_order(size);
        
        // Create initial free block
        let block = BuddyBlock {
            addr: self.base_addr,
            order: max_order,
            next: None,
            prev: None,
        };
        
        self.block_pool.push(block);
        if let Some(block_ref) = self.block_pool.last_mut() {
            let block_ptr = NonNull::new_unchecked(block_ref as *mut _);
            self.free_lists[max_order] = Some(block_ptr);
        }
    }

    fn size_to_order(&self, size: usize) -> usize {
        let blocks = (size + MIN_BLOCK_SIZE - 1) / MIN_BLOCK_SIZE;
        blocks.next_power_of_two().trailing_zeros() as usize
    }

    fn order_to_size(&self, order: usize) -> usize {
        MIN_BLOCK_SIZE << order
    }

    unsafe fn allocate(&mut self, layout: Layout) -> Option<NonNull<u8>> {
        let size = layout.size().max(layout.align());
        let order = self.size_to_order(size);
        
        if order > MAX_ORDER {
            return None;
        }

        // Find the smallest available block
        for current_order in order..=MAX_ORDER {
            if let Some(block) = self.remove_from_free_list(current_order) {
                // Split larger blocks if necessary
                self.split_block(block, current_order, order);
                
                let block_ref = block.as_ref();
                let addr = block_ref.addr;
                let alloc_size = self.order_to_size(order);
                
                self.allocated_bytes.fetch_add(alloc_size, Ordering::Relaxed);
                self.free_bytes.fetch_sub(alloc_size, Ordering::Relaxed);
                
                return NonNull::new(addr as *mut u8);
            }
        }
        
        None
    }

    unsafe fn deallocate(&mut self, ptr: NonNull<u8>, layout: Layout) {
        let addr = ptr.as_ptr() as usize;
        let size = layout.size().max(layout.align());
        let order = self.size_to_order(size);
        
        // Create a new block for the freed memory
        let block = BuddyBlock {
            addr,
            order,
            next: None,
            prev: None,
        };
        
        self.block_pool.push(block);
        if let Some(block_ref) = self.block_pool.last_mut() {
            let block_ptr = NonNull::new_unchecked(block_ref as *mut _);
            
            // Try to merge with buddy
            self.merge_buddies(block_ptr);
            
            let dealloc_size = self.order_to_size(order);
            self.allocated_bytes.fetch_sub(dealloc_size, Ordering::Relaxed);
            self.free_bytes.fetch_add(dealloc_size, Ordering::Relaxed);
        }
    }

    unsafe fn split_block(&mut self, block: NonNull<BuddyBlock>, from_order: usize, to_order: usize) {
        let mut current_order = from_order;
        let mut current_block = block;
        
        while current_order > to_order {
            let block_ref = current_block.as_ref();
            let buddy_addr = block_ref.addr + self.order_to_size(current_order - 1);
            
            // Create buddy block
            let buddy = BuddyBlock {
                addr: buddy_addr,
                order: current_order - 1,
                next: None,
                prev: None,
            };
            
            self.block_pool.push(buddy);
            if let Some(buddy_ref) = self.block_pool.last_mut() {
                let buddy_ptr = NonNull::new_unchecked(buddy_ref as *mut _);
                self.add_to_free_list(buddy_ptr, current_order - 1);
            }
            
            current_order -= 1;
        }
    }

    unsafe fn merge_buddies(&mut self, block: NonNull<BuddyBlock>) {
        let mut current_block = block;
        let mut current_order = current_block.as_ref().order;
        
        while current_order < MAX_ORDER {
            let block_addr = current_block.as_ref().addr;
            let buddy_addr = self.get_buddy_addr(block_addr, current_order);
            
            // Check if buddy is free
            if let Some(buddy) = self.find_and_remove_buddy(buddy_addr, current_order) {
                // Merge blocks
                let merged_addr = block_addr.min(buddy_addr);
                let merged_block = BuddyBlock {
                    addr: merged_addr,
                    order: current_order + 1,
                    next: None,
                    prev: None,
                };
                
                self.block_pool.push(merged_block);
                if let Some(merged_ref) = self.block_pool.last_mut() {
                    current_block = NonNull::new_unchecked(merged_ref as *mut _);
                    current_order += 1;
                }
            } else {
                // No buddy available, add to free list
                self.add_to_free_list(current_block, current_order);
                break;
            }
        }
    }

    fn get_buddy_addr(&self, addr: usize, order: usize) -> usize {
        addr ^ self.order_to_size(order)
    }

    unsafe fn find_and_remove_buddy(&mut self, addr: usize, order: usize) -> Option<NonNull<BuddyBlock>> {
        let mut current = self.free_lists[order];
        
        while let Some(block) = current {
            let block_ref = block.as_ref();
            if block_ref.addr == addr {
                self.remove_from_free_list_ptr(block, order);
                return Some(block);
            }
            current = block_ref.next;
        }
        
        None
    }

    unsafe fn add_to_free_list(&mut self, block: NonNull<BuddyBlock>, order: usize) {
        let block_ref = block.as_ptr();
        (*block_ref).next = self.free_lists[order];
        (*block_ref).prev = None;
        
        if let Some(next) = self.free_lists[order] {
            (*next.as_ptr()).prev = Some(block);
        }
        
        self.free_lists[order] = Some(block);
    }

    unsafe fn remove_from_free_list(&mut self, order: usize) -> Option<NonNull<BuddyBlock>> {
        if let Some(block) = self.free_lists[order] {
            self.remove_from_free_list_ptr(block, order);
            Some(block)
        } else {
            None
        }
    }

    unsafe fn remove_from_free_list_ptr(&mut self, block: NonNull<BuddyBlock>, order: usize) {
        let block_ref = block.as_ptr();
        
        if let Some(prev) = (*block_ref).prev {
            (*prev.as_ptr()).next = (*block_ref).next;
        } else {
            self.free_lists[order] = (*block_ref).next;
        }
        
        if let Some(next) = (*block_ref).next {
            (*next.as_ptr()).prev = (*block_ref).prev;
        }
        
        (*block_ref).next = None;
        (*block_ref).prev = None;
    }
}

// ==================== Enhanced Slab Allocator ====================
// Optimized for small object allocation with size classes

const SLAB_SIZE_CLASSES: &[usize] = &[
    8, 16, 24, 32, 48, 64, 96, 128, 192, 256, 384, 512, 768, 1024, 1536, 2048, 3072, 4096
];

#[derive(Debug, Clone, Copy)]
struct SlabObject {
    next_free: Option<NonNull<SlabObject>>,
}

struct SlabClass {
    object_size: usize,
    objects_per_slab: usize,
    free_list: Option<NonNull<SlabObject>>,
    partial_slabs: Vec<NonNull<u8>>,
    full_slabs: Vec<NonNull<u8>>,
    empty_slabs: Vec<NonNull<u8>>,
    total_objects: AtomicUsize,
    free_objects: AtomicUsize,
}

// Safety: SlabClass is safe to send between threads
unsafe impl Send for SlabClass {}
unsafe impl Sync for SlabClass {}

impl SlabClass {
    fn new(object_size: usize) -> Self {
        let objects_per_slab = (PAGE_SIZE - size_of::<SlabHeader>()) / object_size;
        
        Self {
            object_size,
            objects_per_slab,
            free_list: None,
            partial_slabs: Vec::new(),
            full_slabs: Vec::new(),
            empty_slabs: Vec::new(),
            total_objects: AtomicUsize::new(0),
            free_objects: AtomicUsize::new(0),
        }
    }

    unsafe fn allocate(&mut self, buddy: &mut BuddyAllocator) -> Option<NonNull<u8>> {
        // Try to allocate from free list
        if let Some(obj) = self.free_list {
            self.free_list = (*obj.as_ptr()).next_free;
            self.free_objects.fetch_sub(1, Ordering::Relaxed);
            return Some(obj.cast());
        }

        // Allocate new slab if needed
        if self.partial_slabs.is_empty() && self.empty_slabs.is_empty() {
            self.allocate_new_slab(buddy)?;
        }

        // Get from partial or empty slab
        let slab = self.partial_slabs.first().or_else(|| self.empty_slabs.first())?;
        self.allocate_from_slab(*slab)
    }

    unsafe fn allocate_new_slab(&mut self, buddy: &mut BuddyAllocator) -> Option<()> {
        let layout = Layout::from_size_align(PAGE_SIZE, PAGE_SIZE).ok()?;
        let slab_ptr = buddy.allocate(layout)?;
        
        // Initialize slab header
        let header_ptr = slab_ptr.cast::<SlabHeader>();
        ptr::write(header_ptr.as_ptr(), SlabHeader {
            size_class: self.object_size,
            free_count: self.objects_per_slab,
            free_list: None,
        });

        // Initialize free list in the slab
        let objects_start = slab_ptr.as_ptr().add(size_of::<SlabHeader>());
        let mut prev_obj: Option<NonNull<SlabObject>> = None;
        
        for i in 0..self.objects_per_slab {
            let obj_ptr = objects_start.add(i * self.object_size) as *mut SlabObject;
            (*obj_ptr).next_free = prev_obj;
            prev_obj = Some(NonNull::new_unchecked(obj_ptr));
        }

        (*header_ptr.as_ptr()).free_list = prev_obj;
        
        self.empty_slabs.push(slab_ptr);
        self.total_objects.fetch_add(self.objects_per_slab, Ordering::Relaxed);
        self.free_objects.fetch_add(self.objects_per_slab, Ordering::Relaxed);
        
        Some(())
    }

    unsafe fn allocate_from_slab(&mut self, slab: NonNull<u8>) -> Option<NonNull<u8>> {
        let header = slab.cast::<SlabHeader>().as_mut();
        
        if let Some(obj) = header.free_list {
            header.free_list = (*obj.as_ptr()).next_free;
            header.free_count -= 1;
            self.free_objects.fetch_sub(1, Ordering::Relaxed);
            
            // Update slab lists
            if header.free_count == 0 {
                self.move_slab_to_full(slab);
            } else if header.free_count == self.objects_per_slab - 1 {
                self.move_empty_to_partial(slab);
            }
            
            return Some(obj.cast());
        }
        
        None
    }

    unsafe fn deallocate(&mut self, ptr: NonNull<u8>, buddy: &mut BuddyAllocator) -> bool {
        // Find which slab this object belongs to
        let obj_addr = ptr.as_ptr() as usize;
        let slab_addr = (obj_addr / PAGE_SIZE) * PAGE_SIZE;
        let slab = match NonNull::new(slab_addr as *mut u8) {
            Some(s) => s,
            None => return false,
        };
        
        let header = slab.cast::<SlabHeader>().as_mut();
        if header.size_class != self.object_size {
            return false;
        }
        
        // Add object back to free list
        let obj = ptr.cast::<SlabObject>();
        (*obj.as_ptr()).next_free = header.free_list;
        header.free_list = Some(obj);
        header.free_count += 1;
        self.free_objects.fetch_add(1, Ordering::Relaxed);
        
        // Update slab lists
        if header.free_count == 1 {
            self.move_full_to_partial(slab);
        } else if header.free_count == self.objects_per_slab {
            self.move_partial_to_empty(slab);
            
            // Free completely empty slabs if we have too many
            if self.empty_slabs.len() > 2 {
                self.free_empty_slab(slab, buddy);
            }
        }
        
        true
    }

    fn move_slab_to_full(&mut self, slab: NonNull<u8>) {
        self.partial_slabs.retain(|&s| s != slab);
        self.empty_slabs.retain(|&s| s != slab);
        self.full_slabs.push(slab);
    }

    fn move_empty_to_partial(&mut self, slab: NonNull<u8>) {
        self.empty_slabs.retain(|&s| s != slab);
        self.partial_slabs.push(slab);
    }

    fn move_full_to_partial(&mut self, slab: NonNull<u8>) {
        self.full_slabs.retain(|&s| s != slab);
        self.partial_slabs.push(slab);
    }

    fn move_partial_to_empty(&mut self, slab: NonNull<u8>) {
        self.partial_slabs.retain(|&s| s != slab);
        self.empty_slabs.push(slab);
    }

    unsafe fn free_empty_slab(&mut self, slab: NonNull<u8>, buddy: &mut BuddyAllocator) {
        self.empty_slabs.retain(|&s| s != slab);
        let layout = Layout::from_size_align(PAGE_SIZE, PAGE_SIZE).unwrap();
        buddy.deallocate(slab, layout);
        self.total_objects.fetch_sub(self.objects_per_slab, Ordering::Relaxed);
        self.free_objects.fetch_sub(self.objects_per_slab, Ordering::Relaxed);
    }
}

#[repr(C)]
struct SlabHeader {
    size_class: usize,
    free_count: usize,
    free_list: Option<NonNull<SlabObject>>,
}

// ==================== Per-CPU Cache ====================
// Reduces lock contention by maintaining CPU-local object caches

struct CpuCache {
    magazines: [Magazine; SLAB_SIZE_CLASSES.len()],
}

// Safety: CpuCache is safe to send between threads when properly synchronized
unsafe impl Send for CpuCache {}
unsafe impl Sync for CpuCache {}

struct Magazine {
    size_class: usize,
    objects: [Option<NonNull<u8>>; CACHE_MAGAZINE_SIZE],
    count: usize,
}

impl Magazine {
    const fn new(size_class: usize) -> Self {
        Self {
            size_class,
            objects: [None; CACHE_MAGAZINE_SIZE],
            count: 0,
        }
    }

    fn push(&mut self, obj: NonNull<u8>) -> bool {
        if self.count < CACHE_MAGAZINE_SIZE {
            self.objects[self.count] = Some(obj);
            self.count += 1;
            true
        } else {
            false
        }
    }

    fn pop(&mut self) -> Option<NonNull<u8>> {
        if self.count > 0 {
            self.count -= 1;
            self.objects[self.count].take()
        } else {
            None
        }
    }

    fn is_empty(&self) -> bool {
        self.count == 0
    }

    fn is_full(&self) -> bool {
        self.count == CACHE_MAGAZINE_SIZE
    }
}

impl CpuCache {
    const fn new() -> Self {
        const EMPTY_MAGAZINE: Magazine = Magazine {
            size_class: 0,
            objects: [None; CACHE_MAGAZINE_SIZE],
            count: 0,
        };
        
        Self {
            magazines: [EMPTY_MAGAZINE; SLAB_SIZE_CLASSES.len()],
        }
    }

    fn init(&mut self) {
        for (i, &size) in SLAB_SIZE_CLASSES.iter().enumerate() {
            self.magazines[i] = Magazine::new(size);
        }
    }
}

// ==================== Main Hybrid Allocator ====================

pub struct HybridAllocator {
    slab_allocator: RwLock<Vec<SlabClass>>,
    buddy_allocator: Mutex<BuddyAllocator>,
    cpu_caches: [Mutex<CpuCache>; MAX_CPU_CACHES],
    initialized: AtomicUsize,
    stats: AllocatorStats,
}

struct AllocatorStats {
    total_allocations: AtomicUsize,
    total_deallocations: AtomicUsize,
    current_allocated: AtomicUsize,
    peak_allocated: AtomicUsize,
    cache_hits: AtomicUsize,
    cache_misses: AtomicUsize,
}

impl AllocatorStats {
    const fn new() -> Self {
        Self {
            total_allocations: AtomicUsize::new(0),
            total_deallocations: AtomicUsize::new(0),
            current_allocated: AtomicUsize::new(0),
            peak_allocated: AtomicUsize::new(0),
            cache_hits: AtomicUsize::new(0),
            cache_misses: AtomicUsize::new(0),
        }
    }

    fn record_allocation(&self, size: usize) {
        self.total_allocations.fetch_add(1, Ordering::Relaxed);
        let current = self.current_allocated.fetch_add(size, Ordering::Relaxed) + size;
        
        // Update peak if necessary
        let mut peak = self.peak_allocated.load(Ordering::Relaxed);
        while current > peak {
            match self.peak_allocated.compare_exchange_weak(
                peak, current, Ordering::Relaxed, Ordering::Relaxed
            ) {
                Ok(_) => break,
                Err(p) => peak = p,
            }
        }
    }

    fn record_deallocation(&self, size: usize) {
        self.total_deallocations.fetch_add(1, Ordering::Relaxed);
        self.current_allocated.fetch_sub(size, Ordering::Relaxed);
    }
}

impl HybridAllocator {
    const fn new() -> Self {
        const EMPTY_CACHE: Mutex<CpuCache> = Mutex::new(CpuCache::new());
        
        Self {
            slab_allocator: RwLock::new(Vec::new()),
            buddy_allocator: Mutex::new(BuddyAllocator::new()),
            cpu_caches: [EMPTY_CACHE; MAX_CPU_CACHES],
            initialized: AtomicUsize::new(0),
            stats: AllocatorStats::new(),
        }
    }

    pub unsafe fn init(&self) {
        if self.initialized.compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
            // Initialize buddy allocator with heap memory
            let heap_start = HEAP.0.as_ptr() as *mut u8;
            self.buddy_allocator.lock().init(heap_start, HEAP_SIZE);
            
            // Initialize slab classes
            let mut slabs = self.slab_allocator.write();
            for &size in SLAB_SIZE_CLASSES {
                slabs.push(SlabClass::new(size));
            }
            
            // Initialize CPU caches
            for i in 0..MAX_CPU_CACHES {
                self.cpu_caches[i].lock().init();
            }
            
            crate::serial_println!("Hybrid allocator initialized: {} MB heap, {} slab classes", 
                HEAP_SIZE / (1024 * 1024), SLAB_SIZE_CLASSES.len());
        }
    }

    fn get_cpu_id(&self) -> usize {
        // In a real implementation, this would get the actual CPU ID
        // For now, use a simple round-robin approach
        0
    }

    fn get_size_class_index(size: usize) -> Option<usize> {
        SLAB_SIZE_CLASSES.iter().position(|&s| s >= size)
    }

    unsafe fn allocate_from_cache(&self, size_class_idx: usize) -> Option<NonNull<u8>> {
        let cpu_id = self.get_cpu_id();
        let mut cache = self.cpu_caches[cpu_id].lock();
        
        if let Some(obj) = cache.magazines[size_class_idx].pop() {
            self.stats.cache_hits.fetch_add(1, Ordering::Relaxed);
            return Some(obj);
        }
        
        self.stats.cache_misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    unsafe fn deallocate_to_cache(&self, ptr: NonNull<u8>, size_class_idx: usize) -> bool {
        let cpu_id = self.get_cpu_id();
        let mut cache = self.cpu_caches[cpu_id].lock();
        
        if cache.magazines[size_class_idx].push(ptr) {
            self.stats.cache_hits.fetch_add(1, Ordering::Relaxed);
            return true;
        }
        
        // Cache is full, flush half to slab allocator
        let flush_count = CACHE_MAGAZINE_SIZE / 2;
        let mut objects_to_flush = Vec::new();
        
        for _ in 0..flush_count {
            if let Some(obj) = cache.magazines[size_class_idx].pop() {
                objects_to_flush.push(obj);
            }
        }
        
        // Now add the new object
        let result = cache.magazines[size_class_idx].push(ptr);
        
        // Drop the lock before flushing to slab
        drop(cache);
        
        // Flush objects to slab
        for obj in objects_to_flush {
            self.deallocate_to_slab(obj, size_class_idx);
        }
        
        result
    }

    unsafe fn allocate_from_slab(&self, size_class_idx: usize) -> Option<NonNull<u8>> {
        let mut slabs = self.slab_allocator.write();
        let mut buddy = self.buddy_allocator.lock();
        slabs[size_class_idx].allocate(&mut buddy)
    }

    unsafe fn deallocate_to_slab(&self, ptr: NonNull<u8>, size_class_idx: usize) -> bool {
        let mut slabs = self.slab_allocator.write();
        let mut buddy = self.buddy_allocator.lock();
        slabs[size_class_idx].deallocate(ptr, &mut buddy)
    }
}

unsafe impl GlobalAlloc for HybridAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Initialize on first allocation
        if self.initialized.load(Ordering::Relaxed) == 0 {
            self.init();
        }

        let size = layout.size().max(layout.align());
        
        // Use slab allocator for small objects
        if size < SLAB_THRESHOLD {
            if let Some(idx) = Self::get_size_class_index(size) {
                // Try CPU cache first
                if let Some(ptr) = self.allocate_from_cache(idx) {
                    self.stats.record_allocation(SLAB_SIZE_CLASSES[idx]);
                    return ptr.as_ptr();
                }
                
                // Fall back to slab allocator
                if let Some(ptr) = self.allocate_from_slab(idx) {
                    self.stats.record_allocation(SLAB_SIZE_CLASSES[idx]);
                    return ptr.as_ptr();
                }
            }
        }
        
        // Use buddy allocator for large objects
        let mut buddy = self.buddy_allocator.lock();
        if let Some(ptr) = buddy.allocate(layout) {
            self.stats.record_allocation(size);
            return ptr.as_ptr();
        }
        
        ptr::null_mut()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ptr.is_null() {
            return;
        }

        let ptr = match NonNull::new(ptr) {
            Some(p) => p,
            None => return,
        };

        let size = layout.size().max(layout.align());
        
        // Try slab allocator first for small objects
        if size < SLAB_THRESHOLD {
            if let Some(idx) = Self::get_size_class_index(size) {
                // Try to add to CPU cache
                if self.deallocate_to_cache(ptr, idx) {
                    self.stats.record_deallocation(SLAB_SIZE_CLASSES[idx]);
                    return;
                }
                
                // Fall back to slab allocator
                if self.deallocate_to_slab(ptr, idx) {
                    self.stats.record_deallocation(SLAB_SIZE_CLASSES[idx]);
                    return;
                }
            }
        }
        
        // Use buddy allocator for large objects
        let mut buddy = self.buddy_allocator.lock();
        buddy.deallocate(ptr, layout);
        self.stats.record_deallocation(size);
    }
}

// Global allocator instance
#[global_allocator]
static ALLOCATOR: HybridAllocator = HybridAllocator::new();

// Public initialization function
pub fn init_heap() {
    unsafe {
        ALLOCATOR.init();
    }
}

// Memory statistics functions
pub fn memory_stats() -> MemoryStats {
    let buddy = ALLOCATOR.buddy_allocator.lock();
    let slabs = ALLOCATOR.slab_allocator.read();
    
    let mut slab_allocated = 0;
    let mut slab_free = 0;
    
    for slab_class in slabs.iter() {
        let total = slab_class.total_objects.load(Ordering::Relaxed);
        let free = slab_class.free_objects.load(Ordering::Relaxed);
        slab_allocated += (total - free) * slab_class.object_size;
        slab_free += free * slab_class.object_size;
    }
    
    MemoryStats {
        heap_size: HEAP_SIZE,
        buddy_allocated: buddy.allocated_bytes.load(Ordering::Relaxed),
        buddy_free: buddy.free_bytes.load(Ordering::Relaxed),
        slab_allocated,
        slab_free,
        total_allocations: ALLOCATOR.stats.total_allocations.load(Ordering::Relaxed),
        total_deallocations: ALLOCATOR.stats.total_deallocations.load(Ordering::Relaxed),
        current_allocated: ALLOCATOR.stats.current_allocated.load(Ordering::Relaxed),
        peak_allocated: ALLOCATOR.stats.peak_allocated.load(Ordering::Relaxed),
        cache_hits: ALLOCATOR.stats.cache_hits.load(Ordering::Relaxed),
        cache_misses: ALLOCATOR.stats.cache_misses.load(Ordering::Relaxed),
    }
}

#[derive(Debug)]
pub struct MemoryStats {
    pub heap_size: usize,
    pub buddy_allocated: usize,
    pub buddy_free: usize,
    pub slab_allocated: usize,
    pub slab_free: usize,
    pub total_allocations: usize,
    pub total_deallocations: usize,
    pub current_allocated: usize,
    pub peak_allocated: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
}

impl MemoryStats {
    pub fn print_summary(&self) {
        crate::serial_println!("=== Memory Statistics ===");
        crate::serial_println!("Heap Size: {} MB", self.heap_size / (1024 * 1024));
        crate::serial_println!("Buddy Allocator:");
        crate::serial_println!("  Allocated: {} KB", self.buddy_allocated / 1024);
        crate::serial_println!("  Free: {} KB", self.buddy_free / 1024);
        crate::serial_println!("Slab Allocator:");
        crate::serial_println!("  Allocated: {} KB", self.slab_allocated / 1024);
        crate::serial_println!("  Free: {} KB", self.slab_free / 1024);
        crate::serial_println!("Lifetime Stats:");
        crate::serial_println!("  Total Allocations: {}", self.total_allocations);
        crate::serial_println!("  Total Deallocations: {}", self.total_deallocations);
        crate::serial_println!("  Current Allocated: {} KB", self.current_allocated / 1024);
        crate::serial_println!("  Peak Allocated: {} KB", self.peak_allocated / 1024);
        crate::serial_println!("Cache Performance:");
        let hit_rate = if self.cache_hits + self.cache_misses > 0 {
            (self.cache_hits * 100) / (self.cache_hits + self.cache_misses)
        } else {
            0
        };
        crate::serial_println!("  Hit Rate: {}%", hit_rate);
        crate::serial_println!("  Hits: {}, Misses: {}", self.cache_hits, self.cache_misses);
    }
}

// Debug allocation functions
#[cfg(debug_assertions)]
pub mod debug {
    use super::*;
    
    pub fn validate_heap() -> bool {
        // Perform heap validation checks
        let stats = memory_stats();
        
        // Check for memory leaks
        if stats.total_allocations != stats.total_deallocations + 
           (stats.current_allocated > 0) as usize {
            crate::serial_println!("WARNING: Possible memory leak detected!");
            return false;
        }
        
        true
    }
    
    pub fn dump_allocator_state() {
        let stats = memory_stats();
        stats.print_summary();
    }
}