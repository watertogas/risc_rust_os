use crate::task::exit_task_and_run_next;
use crate::task::schedule::get_current_task;
use crate::task::process::create_new_thread;
use crate::task::process::wait_thread;

pub fn syscall_thread_create(thread_func: usize, start_func: usize, arg_addr: usize) -> isize {
    create_new_thread(thread_func, start_func, arg_addr)
}

pub fn syscall_get_tid() -> isize {
    get_current_task().to_tid() as isize
}

pub fn syscall_wait_tid(tid: usize) -> isize {
    wait_thread(tid)
}
pub fn syscall_thread_exit(exit_code: i32) -> isize {
    exit_task_and_run_next(exit_code);
    0
}