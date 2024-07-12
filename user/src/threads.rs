
use super::*;

pub fn exit(exit_code: i32) -> ! {
    syscall_exit(exit_code)
}
pub fn yield_() -> isize {
    syscall_yield()
}

pub fn get_time()->isize {
    syscall_get_time()
}

pub fn get_time_in_ms() -> isize {
    syscall_get_time()
}
pub fn fork() -> isize {
    syscall_fork()
}
pub fn getpid() -> isize {
    syscall_getpid()
}
pub fn waitpid(pid: usize, exit_code: &mut i32) -> isize {
    loop {
        match syscall_waitpid(pid as isize, exit_code as _) {
            -2 => {
                //this means that child process is still running, just waiting
                yield_();
            },
            pid => return pid,
        }
    }
}

pub fn wait(exit_code: &mut i32) -> isize {
    loop {
        match syscall_waitpid(-1, exit_code as _) {
            -2 => {
                //this means that child process is still running, just waiting
                yield_();
            },
            pid => return pid,
        }
    }
}

pub fn sleep(period_ms: usize) {
    syscall_sleep(period_ms);
}

pub const RISV_MAX_ARGUMENT_SIZE : usize = 256;

pub fn exec(path: &str, args: &[*const u8]) -> isize {
    let mut all_args : Vec<usize> = Vec::new();
    all_args.push(path.as_ptr() as usize);
    all_args.push(path.len());
    //last value is zero
    let mut args_num : usize = 0;
    for i in 0..args.len(){
        let args_start = args[i];
        if args_start == core::ptr::null::<u8>() {
            break;
        }
        let mut len = RISV_MAX_ARGUMENT_SIZE;
        let cur_buf = unsafe {core::slice::from_raw_parts(args_start, len)};
        for j in 0..len{
            if cur_buf[j] == 0 {
                len = j;
                break;
            }
        }
        all_args.push(args_start as usize);
        all_args.push(len);
        args_num += 1;
    }
    syscall_exec(all_args.as_slice().as_ptr() as usize, all_args.len() * core::mem::size_of::<usize>(), args_num)
}

pub fn waitpid_nb(pid: usize, exit_code: &mut i32) -> isize {
    syscall_waitpid(pid as isize, exit_code as *mut _)
}


//These API are user for threads
#[no_mangle]
extern "C" fn thread_start(start_func: extern "C" fn(arg : usize), arg_addr: usize) -> isize {
    start_func(arg_addr);
    thread_exit(0)
}

//thread_create: creates an new thread
pub fn thread_create(start_func: usize, arg_addr: usize) -> isize {
    extern "C" {
        fn thread_start();
    }
    syscall_thread_create(thread_start as usize, start_func, arg_addr)
}
//gettid: get current thread id
pub fn gettid() -> isize {
    syscall_gettid()
}

pub fn thread_join(tid: usize, exit_code : &mut isize) {
    *exit_code = waittid(tid);
}

//waittid: wait a thread to exit, works like pthread_join
pub fn waittid(tid: usize) -> isize {
    loop {
        match syscall_waittid(tid) {
            -2 => {
                yield_();
            }
            exit_code => return exit_code,
        }
    }
}
//gettid: exit current thread
pub fn thread_exit(exit_code: i32) -> ! {
    syscall_thread_exit(exit_code);
    panic!("unreachable after thread_exit!")
}
