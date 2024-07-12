use alloc::collections::VecDeque;
use crate::task::schedule::TaskID;
use crate::task::process::try_wakeup_task;
use crate::task::block_task_and_run_next;
use crate::task::schedule::get_current_task;
use crate::sync::inner::OneCoreCell;

pub struct SemaphoreInner {
    pub count : isize,
    pub waitque : VecDeque<TaskID>,
}

pub struct Semaphore {
    pub inner : OneCoreCell<SemaphoreInner>,
}

impl Semaphore {
    pub fn new(res_count: usize) -> Self {
        Self {
            inner : unsafe{
                OneCoreCell::new(
                    SemaphoreInner {
                        count: res_count as isize,
                        waitque : VecDeque::new(),
                    }
                )
            },
        }
    }
    pub fn up(&self) {
        let mut semaphore =  self.inner.exclusive_access();
        semaphore.count += 1;
        if semaphore.count <= 0 {
            if let Some(task) = semaphore.waitque.pop_front() {
                try_wakeup_task(task);
            }
        }
    }
    pub fn down(&self) {
        let mut semaphore =  self.inner.exclusive_access();
        semaphore.count -= 1;
        if semaphore.count < 0 {
            let task_id = get_current_task();
            semaphore.waitque.push_back(task_id);
            drop(semaphore);
            block_task_and_run_next();
        }
    }
}