#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

#[path = "boards/qemu.rs"]
mod board;

#[macro_use]
mod console;
mod sbi;
mod kernel_error;
mod syscall;
mod mm;
mod fs;
mod drivers;
mod net;
pub mod sync;
pub mod timer;
pub mod trap;
pub mod task;
pub mod config;
pub mod common;
extern crate bitflags;
use core::arch::global_asm;
use crate::mm::init_core_memory;
use crate::task::schedule::task_schedule;
extern crate alloc;

global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("kernel_pagetable.S"));


fn zero_bss(){
    extern "C" {
        fn sbss();
        fn ebss();
    }
    (sbss as usize ..ebss as usize).for_each(|a|{
        unsafe{(a as *mut u8).write_volatile(0)}
    })
}

#[no_mangle]
fn entry_os() {
    println!("Entering to my OS");
    zero_bss();
    println!("clear bss sections..");
    init_core_memory();
    println!("Init core memory done..");
    task::init_for_task();
    println!("init for task done..");
    trap::set_kernel_trap_entry();
    println!("set kernel trap first..");
    trap::enable_timer_interrupt();
    timer::set_timer_trigger();
    println!("enable timer done..");
    board::init_qemu_devices();
    println!("all drivers start-up..");
    fs::list_apps();
    println!("All apps have checked..");
    task::process::run_init_process();
    println!("run init process done..");
    config::set_file_non_blocking();
    println!("All file operations switch to non-blocking..");
    task_schedule();
    panic!("ERROR!!!should never get here");
}
