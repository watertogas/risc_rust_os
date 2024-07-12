// os/src/mm/heap_allocator.rs

use buddy_system_allocator::LockedHeap;
use crate::config::KERNEL_HEAP_SIZE;

#[global_allocator]
/// heap allocator instance
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();

#[alloc_error_handler]
/// panic when heap allocation error occurs
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

static mut OS_HEAP : [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

#[no_mangle]
#[inline(never)]
pub fn init_heap() {
    unsafe {
        HEAP_ALLOCATOR.lock().init(OS_HEAP.as_ptr() as usize, KERNEL_HEAP_SIZE);
        println! ("Heap start:0x{:x} len=0x{:x}", OS_HEAP.as_ptr() as usize, KERNEL_HEAP_SIZE);
        //println! ("Heap allocator:0x{:x}", &HEAP_ALLOCATOR as *const LockedHeap as usize);
    }
}