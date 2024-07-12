pub mod heap_allocator;
pub mod address;
pub mod page_table;
pub mod frame_allocator;
pub mod memory_set;
pub mod kernel_set;

use crate::mm::frame_allocator::init_frame_allocator;
use crate::mm::heap_allocator::init_heap;
use crate::mm::kernel_set::init_first_kernel_mapping;
use crate::mm::kernel_set::init_second_kernel_mapping;

pub fn init_core_memory()
{
    //init physical page table
    init_first_kernel_mapping();
    init_heap();
    init_frame_allocator();
    init_second_kernel_mapping();
}