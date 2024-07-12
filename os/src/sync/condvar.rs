use alloc::collections::VecDeque;
use crate::task::schedule::TaskID;
use crate::task::process::try_wakeup_task;
use crate::task::block_task_and_run_next;
use crate::task::schedule::get_current_task;
use crate::sync::inner::OneCoreCell;

pub struct Condvar {
    pub waitque : OneCoreCell<VecDeque<TaskID>>,
}

impl Condvar {
    pub fn new() -> Self {
        Self {
            waitque : unsafe{
                OneCoreCell::new(VecDeque::new())
            },
        }
    }
    //try wake up all threads
    pub fn signal_all(&self) {
        loop {
            let mut queue = self.waitque.exclusive_access();
            let id = queue.pop_front();
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
    #[allow(unused)]
    pub fn signal_one(&self) {
        let mut queue = self.waitque.exclusive_access();
        if let Some(task) = queue.pop_front() {
            try_wakeup_task(task);
        }
    }
    pub fn wait(&self) {
        let mut queue = self.waitque.exclusive_access();
        let task_id = get_current_task();
        queue.push_back(task_id);
        drop(queue);
        block_task_and_run_next();
    }
    pub fn wait_no_schedule(&self) {
        let mut queue = self.waitque.exclusive_access();
        let task_id = get_current_task();
        queue.push_back(task_id);
    }
}