#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
extern crate alloc;

use alloc::vec::Vec;
use alloc::boxed::Box;
use core::cell::UnsafeCell;
use user_lib::{
    condvar_create, condvar_signal, condvar_wait, thread_exit, mutex_create, mutex_lock, mutex_unlock,
    thread_create, waittid,
};

const THREAD_NUM: usize = 3;

struct Barrier {
    mutex_id: usize,
    condvar_id: usize,
    count: UnsafeCell<usize>,
}

impl Barrier {
    pub fn new() -> Self {
        Self {
            mutex_id: mutex_create() as usize,
            condvar_id: condvar_create() as usize,
            count: UnsafeCell::new(0),
        }
    }
    pub fn block(&self) {
        mutex_lock(self.mutex_id);
        let count = self.count.get();
        // SAFETY: Here, the accesses of the count is in the
        // critical section protected by the mutex.
        unsafe {
            *count = *count + 1;
        }
        if unsafe { *count } == THREAD_NUM {
            condvar_signal(self.condvar_id);
        } else {
            condvar_wait(self.condvar_id, self.mutex_id);
            condvar_signal(self.condvar_id);
        }
        mutex_unlock(self.mutex_id);
    }
}

unsafe impl Sync for Barrier {}

static mut BARRIER_AB: Option<&Barrier> = None;
static mut BARRIER_BC: Option<&Barrier> = None;

pub fn init_barrier()
{
    let barrier_ab = Box::new(Barrier::new());
    let barrier_bc = Box::new(Barrier::new());
    unsafe {
        BARRIER_AB = Some(Box::leak(barrier_ab));
        BARRIER_BC = Some(Box::leak(barrier_bc));
    }
}

fn thread_fn() {
    for _ in 0..300 {
        print!("a");
    }
    unsafe {BARRIER_AB.unwrap().block()}
    for _ in 0..300 {
        print!("b");
    }
    unsafe {BARRIER_BC.unwrap().block()}
    for _ in 0..300 {
        print!("c");
    }
    thread_exit(0)
}

#[no_mangle]
pub fn main() -> i32 {
    init_barrier();
    let mut v: Vec<isize> = Vec::new();
    for _ in 0..THREAD_NUM {
        v.push(thread_create(thread_fn as usize, 0));
    }
    for tid in v.iter() {
        waittid(*tid as usize);
    }
    println!("\nOK!");
    0
}
