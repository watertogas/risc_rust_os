#![no_std]
#![feature(linkage)]
#![feature(alloc_error_handler)]


extern crate alloc;

#[macro_use]
pub mod console;
mod user_errors;
mod syscall;
mod io;
mod net;
mod sync;
mod threads;

use syscall::*;
pub use io::*;
pub use sync::*;
pub use net::*;
pub use threads::*;

use alloc::vec::Vec;
//add a 64KB heap for every process
use buddy_system_allocator::LockedHeap;
const USER_FIXED_HEAP_SIZE : usize = 64 * 1024;

#[global_allocator]
/// heap allocator instance
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();

#[alloc_error_handler]
/// panic when heap allocation error occurs
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

static mut USER_HEAP : [u8; USER_FIXED_HEAP_SIZE] = [0; USER_FIXED_HEAP_SIZE];

fn init_heap() {
    unsafe {
        HEAP_ALLOCATOR.lock().init(USER_HEAP.as_ptr() as usize, USER_FIXED_HEAP_SIZE);
    }
}

#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn _start(argc: usize, argv: usize) -> ! {
    zero_bss();
    init_heap();
    let mut start_addr = argv;
    let mut v: Vec<&'static str> = Vec::new();
    for _ in 0..argc {
        let len_buf = unsafe {core::slice::from_raw_parts_mut(start_addr as *mut usize, 1)};
        let string_len = len_buf[0] + 1;
        start_addr += core::mem::size_of::<usize>();
        let cur_buf = unsafe {core::slice::from_raw_parts(start_addr as *const u8, string_len)};
        let str = core::str::from_utf8(cur_buf).unwrap();
        v.push(str);
        start_addr += string_len;
    }
    exit(main(argc, v.as_slice()));
    panic!("unreachable after sys_exit!");
}

#[linkage = "weak"]
#[no_mangle]
fn main(_argc: usize, _argv: &[&str]) -> i32 {
    panic!("Cannot find main!");
}

fn zero_bss(){
    extern "C" {
        fn start_bss();
        fn end_bss();
    }
    (start_bss as usize ..end_bss as usize).for_each(|a|{
        unsafe{(a as *mut u8).write_volatile(0)}
    })
}
