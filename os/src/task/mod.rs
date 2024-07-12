
pub mod task_context;
pub mod id;
pub mod schedule;
pub mod signal;
pub mod action;
pub mod process;
pub mod thread;

use crate::task::task_context::TaskContext;
use crate::task::process::exit_current_thread;
use crate::task::process::init_kernel_task_manager;
use crate::task::schedule::init_tasklist;
use crate::task::process::exit_current_app;
use crate::task::process::handle_current_signals;
use crate::task::process::handle_an_user_signal;
use crate::task::process::block_current_task;
use crate::task::schedule::enter_schedule;
use crate::task::schedule::add_current_task;
use crate::task::id::init_id_sets;

#[derive(Copy, Clone, PartialEq)]
pub enum TaskhandleStatus {
    OK,
    DOWAIT, //read & write & signal wait
    DOSTOP, //read & write & signal task dead
    DOSIGNAL, //user should go to handle signal
}

pub fn init_for_task()
{
    init_id_sets();
    init_kernel_task_manager();
    init_tasklist();
}

pub fn exit_process_and_run_next(exit_code : isize) {
    exit_current_app(exit_code);
    enter_schedule();
}

pub fn exit_task_and_run_next(exit_code: i32)
{
    exit_current_thread(exit_code);
    enter_schedule();
}

pub fn suspend_task_and_run_next()
{
    add_current_task();
    enter_schedule();
}

//thread will give up CPU and in ready status,
//so user should ensure that all resources(mutex?)
//have been released in Rust env
pub fn block_task_and_run_next()
{
    block_current_task();
    enter_schedule();
}

pub fn handle_task_signals() {
    loop {
        let resut = handle_current_signals();
        match resut {
            TaskhandleStatus::OK => {
                break;
            },
            TaskhandleStatus::DOWAIT => {
                suspend_task_and_run_next();
            },
            TaskhandleStatus::DOSTOP => {
                exit_process_and_run_next(0);
            },
            TaskhandleStatus::DOSIGNAL => {
                handle_an_user_signal();
                break;
            },
        }
    }
}