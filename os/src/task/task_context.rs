use crate::mm::memory_set::UserMemorySets;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;

#[derive(Copy, Clone, PartialEq)]
#[derive(Debug)]
pub enum TaskStatus {
    UNINIT,
    READY,
    RUNNING,
    EXIT,
    ZOMBIE,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct TaskContext{
    pub registers : [usize; 12],
    pub ra : usize,
    pub sp : usize,
}

pub struct UserTask{
    pub pid : usize,
    pub entry_point : usize,
    pub user_stack : usize,
    pub kernel_stack: usize,
    pub trap_context_ptr: usize,
    pub status : TaskStatus,
    pub memorys : UserMemorySets,
    pub context : TaskContext,
}

pub struct TaskSupervisor{
    pub cur_app : usize,
    pub tasks : BTreeMap<usize, UserTask>,
    pub kstacks : UserMemorySets,
    pub readylist : VecDeque<usize>,
    pub deadlist : Vec<usize>,
}