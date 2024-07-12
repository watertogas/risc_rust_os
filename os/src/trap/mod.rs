pub mod context;

use core::arch::global_asm;
use core::arch::asm;
use context::TrapContext;
use crate::syscall::syscall_fn;
use crate::timer::set_timer_trigger;
use crate::timer::check_timer;
use crate::task::suspend_task_and_run_next;
use crate::mm::memory_set::RISV_TRAP_TEXT_STRAT;
use crate::task::process::get_current_context_uaddr;
use crate::task::process::get_current_context_kaddr;
use crate::task::schedule::get_current_task;
use crate::task::process::set_signal;
use crate::task::handle_task_signals;
use crate::task::signal::SIGSEGV;
use crate::task::signal::SIGILL;

use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Interrupt, Trap},
    sie, stval, stvec, sscratch, sstatus,
};

global_asm!(include_str!("trap.S"));


pub fn set_kernel_trap_entry() {
    extern "C" {
        fn _user_trap_entry();
        fn _kernel_trap_entry();
    }
    let kern_trap_entry_va = _kernel_trap_entry as usize - _user_trap_entry as usize + RISV_TRAP_TEXT_STRAT;
    unsafe {
        stvec::write(kern_trap_entry_va, TrapMode::Direct);
        //user sscatch register to store trap handler addr
        sscratch::write(kernel_trap_handler as usize);
    }
}


pub fn set_user_trap_entry()
{
    unsafe {
        stvec::write(RISV_TRAP_TEXT_STRAT as usize, TrapMode::Direct);
    }
}

/// timer interrupt enabled
pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}

#[no_mangle]
pub fn enable_supervisor_interrupt() {
    unsafe {
        sstatus::set_sie();
    }
}

#[no_mangle]
pub fn disable_supervisor_interrupt() {
    unsafe {
        sstatus::clear_sie();
    }
}

#[no_mangle]
/// handle an interrupt, exception, or system call from user space
pub fn user_trap_handler(cx: &mut TrapContext) {
    set_kernel_trap_entry();
    let scause = scause::read(); // get trap cause
    let stval = stval::read(); // get extra value
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            //println!("[kernel] user syscall");
            cx.sepc += 4;
            //allow user syscall to be interrupted
            enable_supervisor_interrupt();
            cx.cr[10] = syscall_fn(cx.cr[17], [cx.cr[10], cx.cr[11], cx.cr[12]]) as usize;
        }
        Trap::Exception(Exception::StoreFault)
        | Trap::Exception(Exception::StorePageFault)
        | Trap::Exception(Exception::InstructionFault)
        | Trap::Exception(Exception::InstructionPageFault)
        | Trap::Exception(Exception::LoadFault)
        | Trap::Exception(Exception::LoadPageFault) => {
            println!(
                "[kernel] {:?} in application:{}, bad addr = {:#x}, bad instruction = {:#x}, kernel killed it.",
                scause.cause(),
                get_current_task().to_pid(),
                stval,
                cx.sepc,
            );
            set_signal(0, SIGSEGV);
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            set_signal(0, SIGILL);
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            set_timer_trigger();
            check_timer();
            suspend_task_and_run_next();
        }
        Trap::Interrupt(Interrupt::SupervisorExternal) => {
            crate::board::irq_handler();
        }
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }
    //println!("[kernel] syscall done");
    cx.context_addr = get_current_context_uaddr();

    //handle current signals
    handle_task_signals();

    user_trap_return()
}

#[no_mangle]
/// return to user space
pub fn user_trap_return() 
{
    //we must disable kernel interrupt, then we can do syscall context restore in kernel mode
    disable_supervisor_interrupt();
    extern "C" {
        fn _user_trap_return();
        fn _user_trap_entry();
    }
    set_user_trap_entry();
    let restore_va  = (_user_trap_return as usize - _user_trap_entry as usize) + RISV_TRAP_TEXT_STRAT;
    let trap_cx_ptr =  get_current_context_kaddr();
    unsafe {
        asm!(
            "jr {restore_va}",             // jump to new addr of __restore asm function
            restore_va = in(reg) restore_va,
            in("a0") trap_cx_ptr,      // a0 = physical_addr of trap context at kernel mode
            options(noreturn)
        );
    }
}

#[no_mangle]
/// handle an interrupt, exception, or system call from kernel space
pub fn kernel_trap_handler() {
    let scause = scause::read(); // get trap cause
    let stval = stval::read(); // get extra value
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            panic!("[kernel] syscall.");
        }
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::StorePageFault) => {
            println!(
                "[kernel] {:?} in application:{}, bad addr = {:#x}, kernel killed it.",
                scause.cause(),
                get_current_task().to_pid(),
                stval
            );
            panic!("[kernel] Pagefault.");
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            panic!("[kernel] IllegalInstruction.");
        }
        Trap::Interrupt(Interrupt::SupervisorExternal) => {
            crate::board::irq_handler();
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            set_timer_trigger();
            check_timer();
            //it can be done to switch task in kernel interrupt, but we ignore it now
        }
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }
}