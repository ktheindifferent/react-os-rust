use linked_list_allocator::LockedHeap;

pub const HEAP_SIZE: usize = 1024 * 1024; // 1 MiB - larger heap for all subsystems

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

// Static heap allocation for simplicity
static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

pub fn init_heap() {
    unsafe {
        let mut allocator = ALLOCATOR.lock();
        allocator.init(HEAP.as_mut_ptr(), HEAP_SIZE);
    }
}