mod fs;
mod net;
mod gui;
mod process;
mod thread;
mod sync;
mod input;

use fs::*;
use net::*;
use gui::*;
use process::*;
use thread::*;
use sync::*;
use input::*;

const SYSCALL_EXIT_ID : usize = 0;
const SYSCALL_WRITE_ID : usize = 1;
const SYSCALL_YIELD_ID : usize = 2;
const SYSCALL_GET_TIME_ID : usize = 3;
const SYSCALL_FORK_ID : usize = 4;
const SYSCALL_GET_PID : usize = 5;
const SYSCALL_WAIT_PID : usize = 6;
const SYSCALL_EXEC : usize = 7;
const SYSCALL_READ : usize = 8;
const SYSCALL_OPEN : usize = 9;
const SYSCALL_CLOSE : usize = 10;
const SYSCALL_PIPE : usize = 11;
const SYSCALL_DUP : usize = 12;
const SYSCALL_SIGKILL : usize = 13;
const SYSCALL_SIGACTION : usize = 14;
const SYSCALL_SIGMASK : usize = 15;
const SYSCALL_SIGRETURN : usize = 16;
const SYSCALL_THREAD_CREATE : usize = 17;
const SYSCALL_GET_THREAD_ID : usize = 18;
const SYSCALL_WAIT_THREAD : usize = 19;
const SYSCALL_THREAD_EXIT : usize = 20;
const SYSCALL_SLEEP_MS : usize = 21;
const SYSCALL_MUTEXT_CREATE : usize = 22;
const SYSCALL_MUTEXT_LOCK : usize = 23;
const SYSCALL_MUTEXT_UNLOCK : usize = 24;
const SYSCALL_SEMAPHORE_CREATE : usize = 25;
const SYSCALL_SEMAPHORE_DOWN : usize = 26;
const SYSCALL_SEMAPHORE_UP : usize = 27;
const SYSCALL_CONDVAR_CREATE : usize = 28;
const SYSCALL_CONDVAR_SIGANL : usize = 29;
const SYSCALL_CONDVAR_WAIT : usize = 30;
const SYSCALL_ACCEPT : usize = 31;
const SYSCALL_LISTEN : usize = 32;
const SYSCALL_CONNECT : usize = 33;
const SYSCALL_MAP_FRAMEBUFFER : usize = 34;
const SYSCALL_FLUSH_FRAMEBUFFER : usize = 35;
const SYSCALL_GET_EVENT : usize = 36;
const SYSCALL_KEY_PRESSED : usize = 37;

pub fn syscall_fn(syscall_id : usize, args: [usize; 3]) ->isize {
    match syscall_id {
        SYSCALL_EXIT_ID => syscall_exit(args[0] as isize),
        SYSCALL_WRITE_ID =>syscall_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_YIELD_ID =>syscall_yield(),
        SYSCALL_GET_TIME_ID =>syscall_get_time(),
        SYSCALL_FORK_ID =>syscall_fork(),
        SYSCALL_GET_PID =>syscall_get_pid(),
        SYSCALL_WAIT_PID =>syscall_wait_pid(args[0] as isize, args[1] as *mut i32),
        SYSCALL_EXEC =>syscall_exec(args[0] as usize, args[1] as usize, args[2] as usize),
        SYSCALL_READ => syscall_read(args[0], args[1] as *const u8, args[2]),
        SYSCALL_OPEN =>syscall_open(args[0] as *const u8, args[1] as usize, args[2] as u32),
        SYSCALL_CLOSE => syscall_close(args[0]),
        SYSCALL_PIPE => syscall_pipe(args[0] as *mut u8),
        SYSCALL_DUP => syscall_dup(args[0]),
        SYSCALL_SIGKILL => syscall_kill(args[0], args[1] as i32),
        SYSCALL_SIGACTION => syscall_signal_action(args[0] as i32, args[1], args[2]),
        SYSCALL_SIGMASK => syscall_setmask(args[0] as i32),
        SYSCALL_SIGRETURN => syscall_sigreturn(),
        SYSCALL_THREAD_CREATE => syscall_thread_create(args[0], args[1], args[2]),
        SYSCALL_GET_THREAD_ID => syscall_get_tid(),
        SYSCALL_WAIT_THREAD => syscall_wait_tid(args[0]),
        SYSCALL_THREAD_EXIT => syscall_thread_exit(args[0] as i32),
        SYSCALL_SLEEP_MS => syscall_sleep_ms(args[0]),
        SYSCALL_MUTEXT_CREATE => syscall_mutex_create(args[0] != 0),
        SYSCALL_MUTEXT_LOCK => syscall_mutex_lock(args[0]),
        SYSCALL_MUTEXT_UNLOCK => syscall_mutex_unlock(args[0]),
        SYSCALL_SEMAPHORE_CREATE =>syscall_semaphore_create(args[0]),
        SYSCALL_SEMAPHORE_UP =>syscall_semaphore_up(args[0]),
        SYSCALL_SEMAPHORE_DOWN =>syscall_semaphore_down(args[0]),
        SYSCALL_CONDVAR_CREATE => syscall_condvar_create(),
        SYSCALL_CONDVAR_SIGANL => syscall_condvar_signal(args[0]),
        SYSCALL_CONDVAR_WAIT => syscall_condvar_wait(args[0], args[1]),
        SYSCALL_ACCEPT =>syscall_accept(args[0]),
        SYSCALL_LISTEN =>syscall_listen(args[0] as u16),
        SYSCALL_CONNECT => syscall_connect(args[0] as u32, args[1] as u16, args[2] as u16),
        SYSCALL_MAP_FRAMEBUFFER => syscall_map_framebuffer(),
        SYSCALL_FLUSH_FRAMEBUFFER => syscall_framebuffer_flush(),
        SYSCALL_GET_EVENT => syscall_event_get(),
        SYSCALL_KEY_PRESSED => syscall_key_pressed(),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}