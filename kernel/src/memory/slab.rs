use alloc::vec::Vec;
use core::alloc::Layout;
use core::mem;
use core::ptr::{self, NonNull};
use spin::Mutex;
use lazy_static::lazy_static;

const SLAB_SIZES: &[usize] = &[8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096];
const OBJECTS_PER_SLAB: usize = 64;

#[derive(Debug)]
struct SlabHeader {
    size: usize,
    free_count: usize,
    next_free: Option<usize>,
    bitmap: u64,
}

struct Slab {
    header: SlabHeader,
    data: Vec<u8>,
}

impl Slab {
    fn new(object_size: usize) -> Self {
        let total_size = object_size * OBJECTS_PER_SLAB;
        let mut slab = Slab {
            header: SlabHeader {
                size: object_size,
                free_count: OBJECTS_PER_SLAB,
                next_free: Some(0),
                bitmap: !0u64,
            },
            data: Vec::with_capacity(total_size),
        };
        unsafe {
            slab.data.set_len(total_size);
        }
        slab
    }

    fn allocate(&mut self) -> Option<NonNull<u8>> {
        if self.header.free_count == 0 {
            return None;
        }

        if let Some(index) = self.header.next_free {
            if index >= OBJECTS_PER_SLAB {
                return None;
            }

            self.header.bitmap &= !(1u64 << index);
            self.header.free_count -= 1;

            self.header.next_free = None;
            for i in (index + 1)..OBJECTS_PER_SLAB {
                if self.header.bitmap & (1u64 << i) != 0 {
                    self.header.next_free = Some(i);
                    break;
                }
            }

            let offset = index * self.header.size;
            let ptr = unsafe { self.data.as_mut_ptr().add(offset) };
            NonNull::new(ptr)
        } else {
            None
        }
    }

    fn deallocate(&mut self, ptr: NonNull<u8>) -> bool {
        let base = self.data.as_ptr();
        let addr = ptr.as_ptr() as usize;
        let base_addr = base as usize;

        if addr < base_addr || addr >= base_addr + self.data.len() {
            return false;
        }

        let offset = addr - base_addr;
        if offset % self.header.size != 0 {
            return false;
        }

        let index = offset / self.header.size;
        if index >= OBJECTS_PER_SLAB {
            return false;
        }

        if self.header.bitmap & (1u64 << index) != 0 {
            return false;
        }

        self.header.bitmap |= 1u64 << index;
        self.header.free_count += 1;

        if self.header.next_free.is_none() || self.header.next_free.unwrap() > index {
            self.header.next_free = Some(index);
        }

        true
    }

    fn is_full(&self) -> bool {
        self.header.free_count == 0
    }

    fn is_empty(&self) -> bool {
        self.header.free_count == OBJECTS_PER_SLAB
    }
}

struct SlabCache {
    object_size: usize,
    slabs: Vec<Slab>,
    partial_slabs: Vec<usize>,
    full_slabs: Vec<usize>,
}

impl SlabCache {
    fn new(object_size: usize) -> Self {
        SlabCache {
            object_size,
            slabs: Vec::new(),
            partial_slabs: Vec::new(),
            full_slabs: Vec::new(),
        }
    }

    fn allocate(&mut self) -> Option<NonNull<u8>> {
        for &idx in &self.partial_slabs {
            if let Some(ptr) = self.slabs[idx].allocate() {
                if self.slabs[idx].is_full() {
                    self.partial_slabs.retain(|&x| x != idx);
                    self.full_slabs.push(idx);
                }
                return Some(ptr);
            }
        }

        let new_slab = Slab::new(self.object_size);
        let idx = self.slabs.len();
        self.slabs.push(new_slab);
        self.partial_slabs.push(idx);

        self.slabs[idx].allocate()
    }

    fn deallocate(&mut self, ptr: NonNull<u8>) -> bool {
        for (idx, slab) in self.slabs.iter_mut().enumerate() {
            if slab.deallocate(ptr) {
                if let Some(pos) = self.full_slabs.iter().position(|&x| x == idx) {
                    self.full_slabs.remove(pos);
                    self.partial_slabs.push(idx);
                }

                if slab.is_empty() && self.slabs.len() > 1 {
                    self.partial_slabs.retain(|&x| x != idx);
                }
                
                return true;
            }
        }
        false
    }
}

pub struct SlabAllocator {
    caches: Vec<SlabCache>,
    large_allocations: Vec<(NonNull<u8>, Layout)>,
}

unsafe impl Send for SlabAllocator {}
unsafe impl Sync for SlabAllocator {}

impl SlabAllocator {
    pub const fn new() -> Self {
        SlabAllocator {
            caches: Vec::new(),
            large_allocations: Vec::new(),
        }
    }

    pub fn init(&mut self) {
        for &size in SLAB_SIZES {
            self.caches.push(SlabCache::new(size));
        }
    }

    fn get_slab_index(size: usize) -> Option<usize> {
        SLAB_SIZES.iter().position(|&s| s >= size)
    }

    pub unsafe fn allocate(&mut self, layout: Layout) -> Option<NonNull<u8>> {
        let size = layout.size();
        let align = layout.align();

        if align > size {
            return None;
        }

        if let Some(idx) = Self::get_slab_index(size) {
            if idx < self.caches.len() {
                return self.caches[idx].allocate();
            }
        }

        let ptr = unsafe {
            let raw = alloc::alloc::alloc(layout);
            NonNull::new(raw)
        };

        if let Some(ptr) = ptr {
            self.large_allocations.push((ptr, layout));
        }

        ptr
    }

    pub unsafe fn deallocate(&mut self, ptr: NonNull<u8>, layout: Layout) {
        let size = layout.size();

        if let Some(idx) = Self::get_slab_index(size) {
            if idx < self.caches.len() {
                if self.caches[idx].deallocate(ptr) {
                    return;
                }
            }
        }

        if let Some(pos) = self.large_allocations.iter().position(|(p, _)| *p == ptr) {
            let (ptr, layout) = self.large_allocations.remove(pos);
            unsafe {
                alloc::alloc::dealloc(ptr.as_ptr(), layout);
            }
        }
    }

    pub fn stats(&self) -> SlabStats {
        let mut total_allocated = 0;
        let mut total_free = 0;
        let mut slab_count = 0;

        for cache in &self.caches {
            slab_count += cache.slabs.len();
            for slab in &cache.slabs {
                total_allocated += (OBJECTS_PER_SLAB - slab.header.free_count) * slab.header.size;
                total_free += slab.header.free_count * slab.header.size;
            }
        }

        SlabStats {
            total_allocated,
            total_free,
            slab_count,
            large_allocations: self.large_allocations.len(),
        }
    }
}

#[derive(Debug)]
pub struct SlabStats {
    pub total_allocated: usize,
    pub total_free: usize,
    pub slab_count: usize,
    pub large_allocations: usize,
}

lazy_static! {
    pub static ref SLAB_ALLOCATOR: Mutex<SlabAllocator> = Mutex::new(SlabAllocator::new());
}

pub fn init() {
    let mut allocator = SLAB_ALLOCATOR.lock();
    allocator.init();
    crate::serial_println!("SLAB allocator initialized with {} size classes", SLAB_SIZES.len());
}