use crate::task::process::add_new_mutext;
use crate::task::process::operation_lock;
use crate::task::process::operation_semaphore;
use crate::task::process::add_semaphore;
use crate::task::process::add_condvar;
use crate::task::process::operation_condvar;

pub fn syscall_mutex_create(blocking : bool) -> isize {
    add_new_mutext(blocking)
}

pub fn syscall_mutex_lock(lock_id : usize) -> isize {
    operation_lock(lock_id, true)
}

pub fn syscall_mutex_unlock(lock_id : usize) -> isize {
    operation_lock(lock_id, false)
}

pub fn syscall_semaphore_create(res_count: usize) -> isize {
    add_semaphore(res_count)
}

pub fn syscall_semaphore_up(sem_id: usize) -> isize {
    operation_semaphore(sem_id, true)
}

pub fn syscall_semaphore_down(sem_id: usize) -> isize {
    operation_semaphore(sem_id, false)
}

pub fn syscall_condvar_create() -> isize {
    add_condvar()
}
pub fn syscall_condvar_signal(condvar_id: usize) ->isize {
    operation_condvar(condvar_id, 0, true)
}
pub fn syscall_condvar_wait(condvar_id: usize, mutex_id: usize) ->isize {
    operation_condvar(condvar_id, mutex_id, false)
}