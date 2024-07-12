use alloc::collections::VecDeque;
use crate::task::schedule::get_current_task;
use crate::task::schedule::TaskID;
use crate::task::process::try_wakeup_task;
use crate::task::block_task_and_run_next;
use crate::sync::inner::OneCoreCell;

pub trait Mutex: Sync + Send {
    fn lock(&self)->usize;
    fn unlock(&self);
}

//spinLock for single core, we do not need to care
//about interrupt
pub struct SpinLock {
    pub locked : OneCoreCell<bool>,
}

impl SpinLock {
    pub fn new() -> Self {
        Self {
            locked: unsafe{OneCoreCell::new(false)},
        }
    }
}
//spin lock
impl Mutex for SpinLock {
    fn lock(&self)->usize {
        let mut locked = self.locked.exclusive_access();
        if *locked {
            1
        } else {
            *locked = true;
            0
        }
    }
    fn unlock(&self) {
        let mut locked = self.locked.exclusive_access();
        *locked = false;
    }
}

pub struct MutextInner {
    pub locked : bool,
    pub waitque : VecDeque<TaskID>,
}

pub struct MutexLock {
    pub inner : OneCoreCell<MutextInner>,
}

impl MutexLock {
    pub fn new() -> Self {
        Self {
            inner: unsafe{OneCoreCell::new(
                MutextInner {
                    locked : false,
                    waitque : VecDeque::new(),
                }
            )},
        }
    }
}
//MutexLock
impl Mutex for MutexLock {
    fn lock(&self)->usize{
        let mut mutext = self.inner.exclusive_access();
        if mutext.locked {
            let task_id = get_current_task();
            mutext.waitque.push_back(task_id);
            drop(mutext);
            //just block current task and run next
            block_task_and_run_next();
            1
        } else {
            mutext.locked = true;
            0
        }
    }
    fn unlock(&self) {
        let mut mutext = self.inner.exclusive_access();
        mutext.locked = false;
        //try to wake up all threads
        loop {
            let id = mutext.waitque.pop_front();
            match id {
                Some(task_id) => {
                    try_wakeup_task(task_id);
                },
                None => {
                    break;
                },
            }
        }
    }
}