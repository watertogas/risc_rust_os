use crate::mm::memory_set::UserBuffer;
use crate::task::exit_process_and_run_next;
use crate::task::suspend_task_and_run_next;
use crate::timer::get_time_in_ms;
use crate::timer::add_timer;
use crate::task::process::fork_new_app;
use crate::task::schedule::get_current_task;
use crate::task::process::wait_any_child;
use crate::task::process::wait_single_child;
use alloc::string::String;
use alloc::vec::Vec;
use crate::trap::user_trap_return;
use crate::fs::open_file;
use crate::fs::OpenFlags;
use crate::task::process::exec_an_app;
use crate::task::process::set_signal_mask;
use crate::task::process::set_signal_action;
use crate::task::process::set_signal;
use crate::task::process::return_from_signal;
use crate::task::block_task_and_run_next;


pub fn syscall_yield() ->isize {
    suspend_task_and_run_next();
    0
}

pub fn syscall_exit(code : isize) ->isize {
    exit_process_and_run_next(code);
    0
}

pub fn syscall_fork() -> isize {
    fork_new_app() as isize
}

pub fn syscall_get_pid() -> isize {
    get_current_task().to_pid() as isize
}

pub fn syscall_wait_pid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    if pid == -1 {
        wait_any_child(exit_code_ptr)
    } else {
        wait_single_child(pid, exit_code_ptr)
    }
}

pub fn syscall_exec(args : usize, len : usize, args_num : usize) -> isize {
    let ret = do_exec(args, len, args_num);
    if ret == 0 {
        //exec success, just do trap return
        user_trap_return();
    }
    println!("exec failed: {}", ret);
    ret
}

//&string[0..string.len()-1]
pub fn do_exec(args : usize, len : usize, args_num : usize)->isize
{
    let all_buf = UserBuffer::new(args, len);
    let args_vec : Vec<u8> = Vec::with_capacity(len);
    let args_addr = args_vec.as_slice().as_ptr() as usize;
    all_buf.read_buff_to_kernel_slice(args_addr, len);
    let args_buf = unsafe {core::slice::from_raw_parts(args_addr as *const usize, 2 + args_num*2)};
    let mut string = String::new();
    let user_buf = UserBuffer::new(args_buf[0], args_buf[1]);
    user_buf.read_buff_to_kernel_string(&mut string);
    if let Some(app_inode) = open_file(&string[0..string.len()-1], OpenFlags::RDONLY) {
        let all_data = app_inode.read_all();
        exec_an_app(all_data.as_slice(), args_buf);
        0
    } else {
        -1
    }
}

//signal syscalls
pub fn syscall_setmask(mask : i32) -> isize {
    set_signal_mask(mask)
}

pub fn syscall_signal_action(signum: i32, action: usize, old_action: usize) -> isize {
    set_signal_action(signum, action, old_action)
}

pub fn syscall_sigreturn() -> isize {
    return_from_signal()
}

pub fn syscall_kill(pid: usize, signum: i32) -> isize {
    set_signal(pid, signum)
}


//time
pub fn syscall_sleep_ms(period_ms: usize) -> isize {
    let expire_ms = get_time_in_ms() + period_ms;
    add_timer(expire_ms, get_current_task());
    block_task_and_run_next();
    0
}

pub fn syscall_get_time() ->isize {
    get_time_in_ms() as isize
}