use core::arch::asm;
use crate::SignalAction;

const SYSCALL_EXIT_ID : usize = 0;
const SYSCALL_WRITE_ID : usize = 1;
const SYSCALL_YIELD_ID : usize = 2;
const SYSCALL_GET_TIME_ID : usize = 3;
//process sysacalls
const SYSCALL_FORK_ID : usize = 4;
const SYSCALL_GET_PID : usize = 5;
const SYSCALL_WAIT_PID : usize = 6;
const SYSCALL_EXEC : usize = 7;
//file syscalls
const SYSCALL_READ : usize = 8;
const SYSCALL_OPEN : usize = 9;
const SYSCALL_CLOSE : usize = 10;
//pipe syscalls
const SYSCALL_PIPE : usize = 11;
const SYSCALL_DUP : usize = 12;
//signal syscalls
const SYSCALL_SIGKILL : usize = 13;
const SYSCALL_SIGACTION : usize = 14;
const SYSCALL_SIGMASK : usize = 15;
const SYSCALL_SIGRETURN : usize = 16;
//thread
const SYSCALL_THREAD_CREATE : usize = 17;
const SYSCALL_GET_THREAD_ID : usize = 18;
const SYSCALL_WAIT_THREAD : usize = 19;
const SYSCALL_THREAD_EXIT : usize = 20;
//timer
const SYSCALL_SLEEP_MS : usize = 21;
//mutext
const SYSCALL_MUTEXT_CREATE : usize = 22;
const SYSCALL_MUTEXT_LOCK : usize = 23;
const SYSCALL_MUTEXT_UNLOCK : usize = 24;
//semaphore
const SYSCALL_SEMAPHORE_CREATE : usize = 25;
const SYSCALL_SEMAPHORE_DOWN : usize = 26;
const SYSCALL_SEMAPHORE_UP : usize = 27;
//condvar
const SYSCALL_CONDVAR_CREATE : usize = 28;
const SYSCALL_CONDVAR_SIGANL : usize = 29;
const SYSCALL_CONDVAR_WAIT : usize = 30;
//network
const SYSCALL_ACCEPT : usize = 31;
const SYSCALL_LISTEN : usize = 32;
const SYSCALL_CONNECT : usize = 33;
//display & input
const SYSCALL_MAP_FRAMEBUFFER : usize = 34;
const SYSCALL_FLUSH_FRAMEBUFFER : usize = 35;
const SYSCALL_GET_EVENT : usize = 36;
const SYSCALL_KEY_PRESSED : usize = 37;

fn syscall_fn(sys_id : usize, args: [usize; 3]) ->isize {
    let mut ret: isize;
    unsafe {
        asm!(
            "ecall",
            inlateout("x10") args[0] => ret,
            in("x11") args[1],
            in("x12") args[2],
            in("x17") sys_id
        );
    }
    ret
}

pub fn syscall_write(fd: usize, buffer: &[u8]) -> isize {
    syscall_fn(SYSCALL_WRITE_ID, [fd, buffer.as_ptr() as usize, buffer.len()])
}

pub fn syscall_exit(exit_code : i32) -> ! {
    syscall_fn(SYSCALL_EXIT_ID, [exit_code as usize, 0, 0]);
    panic!("unreachable after sys_exit!")
}

pub fn syscall_yield() -> isize {
    syscall_fn(SYSCALL_YIELD_ID, [0, 0, 0])
}

pub fn syscall_get_time() -> isize {
    syscall_fn(SYSCALL_GET_TIME_ID, [0, 0, 0])
}

pub fn syscall_fork() -> isize {
    syscall_fn(SYSCALL_FORK_ID, [0, 0, 0])
}

pub fn syscall_getpid() -> isize {
    syscall_fn(SYSCALL_GET_PID, [0, 0, 0])
}

pub fn syscall_exec(buf_start: usize, buf_len: usize, args_num : usize) -> isize {
    syscall_fn(SYSCALL_EXEC, [buf_start, buf_len, args_num])
}

pub fn syscall_waitpid(pid: isize, exit_code: *mut i32) -> isize {
    syscall_fn(SYSCALL_WAIT_PID, [pid as usize, exit_code as usize, 0])
}

pub fn syscall_read(fd: usize, buffer: &mut [u8]) -> isize {
    syscall_fn(SYSCALL_READ,[fd, buffer.as_mut_ptr() as usize, buffer.len()])
}

pub fn syscall_open(path: &str, flags: u32) -> isize {
    syscall_fn(SYSCALL_OPEN, [path.as_ptr() as usize, path.len(), flags as usize])
}

pub fn syscall_close(fd: usize) -> isize {
    syscall_fn(SYSCALL_CLOSE, [fd, 0, 0])
}

pub fn syscall_pipe(buffer: &mut [usize]) -> isize {
    syscall_fn(SYSCALL_PIPE,[buffer.as_mut_ptr() as usize, 0, 0])
}

pub fn syscall_dup(fd: usize) -> isize {
    syscall_fn(SYSCALL_DUP,[fd, 0, 0])
}

pub fn syscall_kill(pid: usize, signal: i32) -> isize {
    syscall_fn(SYSCALL_SIGKILL,[pid, signal as usize, 0])
}

pub fn syscall_action(signal: i32, action: *const SignalAction, old_action: *mut SignalAction) -> isize {
    syscall_fn(SYSCALL_SIGACTION,[signal as usize, action as usize, old_action as usize])
}

pub fn syscall_sigmask(mask: u32) -> isize {
    syscall_fn(SYSCALL_SIGMASK,[mask as usize, 0, 0])
}

pub fn syscall_sigreturn() -> isize {
    syscall_fn(SYSCALL_SIGRETURN,[0, 0, 0])
}

pub fn syscall_thread_create(thread_func: usize, start_func: usize, arg_addr: usize) -> isize {
    syscall_fn(SYSCALL_THREAD_CREATE,[thread_func, start_func, arg_addr])
}
pub fn syscall_gettid() -> isize {
    syscall_fn(SYSCALL_GET_THREAD_ID,[0, 0, 0])
}
pub fn syscall_waittid(tid: usize) -> isize {
    syscall_fn(SYSCALL_WAIT_THREAD,[tid, 0, 0])
}
pub fn syscall_thread_exit(exit_code: i32) -> isize {
    syscall_fn(SYSCALL_THREAD_EXIT,[exit_code as usize, 0, 0])
}

pub fn syscall_sleep(period_ms: usize) -> isize {
    syscall_fn(SYSCALL_SLEEP_MS,[period_ms, 0, 0])
}

pub fn syscall_mutex_create(blocking: bool) -> isize {
    syscall_fn(SYSCALL_MUTEXT_CREATE,[blocking as usize, 0, 0])
}

pub fn syscall_mutex_unlock(mutex_id: usize) -> isize {
    syscall_fn(SYSCALL_MUTEXT_UNLOCK,[mutex_id as usize, 0, 0])
}

pub fn syscall_mutex_lock(mutex_id: usize) -> isize {
    syscall_fn(SYSCALL_MUTEXT_LOCK,[mutex_id as usize, 0, 0])
}

pub fn syscall_semaphore_create(res_count: usize) -> isize {
    syscall_fn(SYSCALL_SEMAPHORE_CREATE,[res_count, 0, 0])
}

pub fn syscall_semaphore_up(sem_id: usize) -> isize {
    syscall_fn(SYSCALL_SEMAPHORE_UP,[sem_id, 0, 0])
}

pub fn syscall_semaphore_down(sem_id: usize) -> isize {
    syscall_fn(SYSCALL_SEMAPHORE_DOWN,[sem_id, 0, 0])
}

//condvar
pub fn syscall_condvar_create() -> isize {
    syscall_fn(SYSCALL_CONDVAR_CREATE,[0, 0, 0])
}
pub fn syscall_condvar_signal(condvar_id: usize) ->isize {
    syscall_fn(SYSCALL_CONDVAR_SIGANL,[condvar_id, 0, 0])
}
pub fn syscall_condvar_wait(condvar_id: usize, mutex_id: usize) ->isize {
    syscall_fn(SYSCALL_CONDVAR_WAIT,[condvar_id, mutex_id, 0])
}

//i/o devices
pub fn syscall_get_fb_addr() ->isize {
    syscall_fn(SYSCALL_MAP_FRAMEBUFFER,[0, 0, 0])
}
pub fn syscall_framebuffer_flush() ->isize {
    syscall_fn(SYSCALL_FLUSH_FRAMEBUFFER,[0, 0, 0])
}
pub fn syscall_event_get() -> isize {
    syscall_fn(SYSCALL_GET_EVENT, [0, 0, 0])
}

pub fn syscall_key_pressed() -> isize {
    syscall_fn(SYSCALL_KEY_PRESSED, [0, 0, 0])
}

//newtwork
pub fn syscall_connect(ip: u32, sport: u16, dport: u16) -> isize {
    syscall_fn(SYSCALL_CONNECT, [ip as usize, sport as usize, dport as usize])
}

pub fn syscall_listen(sport: u16) -> isize {
    syscall_fn(SYSCALL_LISTEN, [sport as usize, 0, 0])
}

pub fn syscall_accept(socket_fd: usize) -> isize {
    syscall_fn(SYSCALL_ACCEPT, [socket_fd as usize, 0, 0])
}

