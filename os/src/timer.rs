use riscv::register::time;
use crate::sbi::set_timer;
use crate::config::CLOCK_FREQ;
use crate::config::SCHEDUL_INTERVAL;
use crate::task::schedule::TaskID;
use crate::task::process::try_wakeup_task;
use core::cmp::Ordering;
use spin::Mutex;
use alloc::collections::BinaryHeap;
use lazy_static::*;

const MSECS_IN_SECS : usize = 1000;

//read current time from machine mode
pub fn get_time()->usize {
    time::read()
}

pub fn get_time_in_ms() -> usize {
    time::read()/(CLOCK_FREQ / MSECS_IN_SECS)
}

pub fn set_timer_trigger() {
    set_timer(get_time() + CLOCK_FREQ/MSECS_IN_SECS*SCHEDUL_INTERVAL);
}

pub struct TimerCondVar {
    pub expire_ms: usize,
    pub task_id: TaskID,
}

impl PartialEq for TimerCondVar {
    fn eq(&self, other: &Self) -> bool {
        self.expire_ms == other.expire_ms
    }
}
impl Eq for TimerCondVar {}
impl PartialOrd for TimerCondVar {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let a = -(self.expire_ms as isize);
        let b = -(other.expire_ms as isize);
        Some(a.cmp(&b))
    }
}

impl Ord for TimerCondVar {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

lazy_static! {
    static ref TIMERS: Mutex<BinaryHeap<TimerCondVar>> = Mutex::new(BinaryHeap::<TimerCondVar>::new());
}

pub fn add_timer(expire_ms: usize, task_id: TaskID) {
    TIMERS.lock().push(TimerCondVar { expire_ms, task_id });
}

//we do not need to remove timer, since we will check if task is ready,
//we will not wake up a task that is not ready
pub fn check_timer() {
    let current_ms = get_time_in_ms();
    let mut expire_ms : usize;
    let mut task_id : TaskID;
    loop {
        match TIMERS.lock().peek() {
            Some(timer) => {
                expire_ms = timer.expire_ms;
                task_id = timer.task_id;
            }
            None => {
                break;
            }
        }
        if expire_ms <= current_ms {
            try_wakeup_task(task_id);
            {
                TIMERS.lock().pop();
            }
        } else {
            break;
        }
    }
}