use core::arch::global_asm;
use core::ptr::addr_of;
use crate::task::task_context::TaskContext;
use alloc::collections::VecDeque;
use alloc::boxed::Box;
use crate::task::process::get_app_context;
use crate::sync::InterruptMask;

const TASK_ID_MASK : usize = (1 << 32) - 1;  

global_asm!(include_str!("switch.S"));
extern "C" {
    /// Switch to the context of idle, saving the current context
    pub fn __switch_from(current_task_cx_ptr: *const TaskContext, idle_task_cx_ptr: *const Idlecontext);
    /// Switch to the context of dst, restoring the dst context
    pub fn __switch_to(dst_task_cx_ptr: *const TaskContext);
}

#[repr(C)]
pub struct Idlecontext{
    pub stack : usize,
    pub ra : usize,
}
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct TaskID(pub usize);
impl TaskID {
    pub fn new() -> Self {
        Self {
            0: 0,
        }
    }
    pub fn set_value(&mut self, pid : usize, tid : usize) {
        self.0 = (pid << 32) + (tid & TASK_ID_MASK)
    }
    pub fn to_pid(&self)->usize {
        self.0 >> 32
    }
    pub fn to_tid(&self)->usize {
        self.0 & TASK_ID_MASK
    }
    pub fn to_pid_tid(&self)->(usize, usize) {
        (self.0 >> 32, self.0 & TASK_ID_MASK)
    }
}

pub struct TaskContoller{
    pub readylist : VecDeque<TaskID>,
    pub cur_app : TaskID,
    pub int_ctl : InterruptMask,
}

impl TaskContoller {
    pub fn new() ->Self {
        Self { 
            readylist: VecDeque::new(),
            cur_app : TaskID::new(),
            int_ctl : InterruptMask::new(),
        }
    }
    pub fn fetch(&mut self) -> Option<TaskID> {
        self.readylist.pop_front()
    }
    pub fn add(&mut self, task : TaskID) {
        self.readylist.push_back(task);
    }
    pub fn remove(&mut self, task_id : TaskID) {
        for i in 0..self.readylist.len() {
            if self.readylist[i].0 == task_id.0 {
                self.readylist.remove(i);
                break;
            }
        }
    }
}

static mut IDLECONTEXT : Idlecontext = Idlecontext{
    stack : 0,
    ra : 0,
};

static mut TASKLIST: Option<&mut TaskContoller> = None;

pub fn init_tasklist()
{
    extern "C" {
        /// kernel stack top
        pub fn kernel_stack_top();
    } 
    let tasks = Box::new(TaskContoller::new());
    unsafe {
        TASKLIST = Some(Box::leak(tasks));
        IDLECONTEXT.stack = kernel_stack_top as usize;
        IDLECONTEXT.ra = task_schedule as usize;
    }
}

#[no_mangle]
#[inline(never)]
pub fn enter_schedule()
{
    unsafe { 
        let task_id = TASKLIST.as_mut().unwrap().cur_app;
        let cur_context_prt = get_app_context(task_id.to_pid(), task_id.to_tid());
        //Disable all interrupts before switch to schedule task, and recover interrupts as we wish
        let mut int_ctl = InterruptMask::new();
        int_ctl.mask_interrupt();
        __switch_from(cur_context_prt, addr_of!(IDLECONTEXT) as *const Idlecontext);
        int_ctl.unmask_interrupt();
    }
}

pub fn get_current_task()->TaskID {
    unsafe {
        TASKLIST.as_mut().unwrap().cur_app
    }
}

pub fn add_current_task()
{
    unsafe {
        TASKLIST.as_mut().unwrap().int_ctl.mask_interrupt();
        let task_id = TASKLIST.as_mut().unwrap().cur_app;
        TASKLIST.as_mut().unwrap().add(task_id);
        TASKLIST.as_mut().unwrap().int_ctl.unmask_interrupt();
    }
}

pub fn add_schedule_task(pid : usize, tid : usize)
{
    let mut task_id = TaskID::new();
    task_id.set_value(pid, tid);
    unsafe {
        TASKLIST.as_mut().unwrap().int_ctl.mask_interrupt();
        TASKLIST.as_mut().unwrap().add(task_id);
        TASKLIST.as_mut().unwrap().int_ctl.unmask_interrupt();
    }
}

pub fn remove_schedule_task(pid : usize, tid : usize)
{
    let mut task_id = TaskID::new();
    task_id.set_value(pid, tid);
    unsafe {
        TASKLIST.as_mut().unwrap().int_ctl.mask_interrupt();
        TASKLIST.as_mut().unwrap().remove(task_id);
        TASKLIST.as_mut().unwrap().int_ctl.unmask_interrupt();
    }
}

pub fn dump_ready_lists()
{
    let (cur_pid, cur_tid) = get_current_task().to_pid_tid();
    println! ("current pid={}, tid={}", cur_pid, cur_tid);
    unsafe {
        let list = &TASKLIST.as_mut().unwrap().readylist;
        for id in list {
            let (pid, tid) = id.to_pid_tid();
            println! ("ready pid={}, tid={}", pid, tid);
        }
    }
}

fn fetch_schedule_task()->TaskID
{
    unsafe {
        let task = TASKLIST.as_mut().unwrap().fetch();
        match task{
            None => {
                panic!("readylist is empty? this should never happen!");
            },
            Some(task_id) => {
                task_id
            },
        }
    }
}

pub fn check_list_is_empty(){
    unsafe {
        if TASKLIST.as_mut().unwrap().readylist.is_empty() {
            panic!("readylist is empty!!!");
        }
    }
}

#[no_mangle]
#[inline(never)]
pub fn task_schedule()
{
    //unsafe{println!("switch from process:{} to idle", TASKLIST.as_mut().unwrap().cur_app.to_pid());}
    check_list_is_empty();
    //do some schedule
    let task_id = fetch_schedule_task();
    //println!("switch from idle to process:{}", task_id.to_pid());
    let context_ptr = get_app_context(task_id.to_pid(), task_id.to_tid());
    unsafe {
        TASKLIST.as_mut().unwrap().cur_app = task_id;
        __switch_to(context_ptr);
    }
}