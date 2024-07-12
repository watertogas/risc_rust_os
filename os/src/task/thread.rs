use riscv::register::sstatus::{self, SPP};
use crate::mm::kernel_set::alloc_kernel_stack;
use alloc::vec::Vec;
use crate::mm::memory_set::UserMemorySets;
use crate::trap::context::TrapContext;
use crate::mm::memory_set::UserBuffer;
use crate::config::KERNEL_PAGE_SIZE;
use crate::mm::kernel_set::get_kernel_stack_top;
use crate::mm::memory_set::get_user_stack_top;
use crate::mm::memory_set::get_user_trap_context_start;
use crate::mm::kernel_set::KernelStack;
use crate::task::TaskContext;
use crate::task::task_context::TaskStatus;
use crate::mm::kernel_set::get_kernel_stap;

pub struct Thread
{
    pub tid : usize,
    pub exit_code : usize,
    //private kernel stack
    pub kern_stack : KernelStack,
    //private memory: usr_stack, trap_context
    pub private_mem : Vec<UserMemorySets>,
    //task context
    pub context : TaskContext,
    pub status : TaskStatus,
}

impl Thread {
    pub fn new(id : usize) ->Self {
        Self { 
            tid: id,
            exit_code: 0,
            kern_stack : alloc_kernel_stack(),
            private_mem: Vec::with_capacity(1),
            context : TaskContext {
                registers : [0; 12],
                ra : 0,
                sp : 0,
            },
            //new thread will alreays be running
            status : TaskStatus::RUNNING,
        }
    }
    pub fn dump_thread(&self) {
        println! ("tid[{}], kern_stack_id:{}", self.tid, self.kern_stack.stack_id);
        self.private_mem[0].print_maps();
        println! ("ra:0x{:0x}, sp:0x{:0x}", self.context.ra, self.context.sp);
        println! ("context_addr:0x{:0x}", &self.context as *const TaskContext as usize);
    }
    pub fn init_private_memorys(&mut self, root_ppn : usize) {
        self.private_mem.pop();
        self.private_mem.push(UserMemorySets::new());
        self.private_mem[0].table.set_root_ppn(root_ppn);
        self.private_mem[0].add_user_stack(self.tid);
        self.private_mem[0].add_trap_context(self.tid);
    }
    pub fn init_task_data(&mut self) {
        extern "C" {
            fn user_trap_return();
        }
        self.context.sp = get_kernel_stack_top(self.kern_stack.stack_id);
        self.context.ra = user_trap_return as usize;
    }
    pub fn set_user_trap_context(&self, entry_point : usize, args : &[usize]) {
        extern "C" {
            fn user_trap_handler();
        }
        let tid = self.tid;
        let trap_paddr = self.private_mem[0].get_trap_context_paddr(tid);
        let trap_vaddr = get_user_trap_context_start(tid);
        let mut user_stack = get_user_stack_top(tid);
        //add start-up arguments
        let args_num = (args.len() - 2)/2;
        if args_num > 0 {
            let mut data_len : usize = args_num * core::mem::size_of::<usize>();
            for i in 0..args_num {
                let string_len = args[i*2 + 2 + 1];
                data_len += string_len;
            }
            //println! ("all start-up data_len:{}", data_len);
            if data_len > KERNEL_PAGE_SIZE {
                panic! ("too many start-up args: data_len:{}", data_len);
            }
            let mut start_addr = self.private_mem[0].get_user_start_args_paddr(tid);
            for j in 0..args_num {
                let string_buf = args[j*2 + 2];
                let string_len = args[j*2 + 2 + 1];
                let len_buf = unsafe {core::slice::from_raw_parts_mut(start_addr as *mut usize, 1)};
                len_buf[0] = string_len;
                start_addr += core::mem::size_of::<usize>();
                let user_buf = UserBuffer::new(string_buf, string_len);
                user_buf.read_buff_to_kernel_slice(start_addr, string_len);
                start_addr = start_addr + string_len + 1;
            }
            user_stack -= KERNEL_PAGE_SIZE;
        }
        //set trap context
        unsafe{
            let cx = trap_paddr as *mut TrapContext;
            let mut cur_sstatus = sstatus::read();
            cur_sstatus.set_spp(SPP::User);
            let mut temp = TrapContext {
                cr : [0; 32],
                sepc : entry_point,
                sstatus : cur_sstatus,
                kernel_stack : get_kernel_stack_top(self.kern_stack.stack_id),
                kernel_stap : get_kernel_stap(),
                user_stap : self.private_mem[0].table.get_root_stap(),
                trap_handler : user_trap_handler as usize,
                context_addr : trap_vaddr,
            };
            temp.cr[2] = user_stack;
            //set R0 = args, R1 = args_addr
            temp.cr[10] = args_num;
            temp.cr[11] = user_stack;
            *cx = temp;
        }
    }
    pub fn fork_thread(&self, thread : &mut Thread) {
        //fork user memory
        self.private_mem[0].fork_user_memory(&mut thread.private_mem[0]);
        //set forked thread data and return value should be 0
        thread.set_fork_data(0);
        //set task data
        thread.init_task_data();
    }
    //child process should return 0 in sys_fork
    pub fn set_fork_data(&self, ret : usize) {
        let tid = self.tid;
        let trap_paddr = self.private_mem[0].get_trap_context_paddr(tid);
        let src_data = unsafe {core::slice::from_raw_parts_mut(trap_paddr as *mut usize , 512)};
        //set retrun value
        src_data[10] = ret;
        //set context_addr
        src_data[38] = get_user_trap_context_start(tid);
        //set kernel_stack
        src_data[34] = get_kernel_stack_top(self.kern_stack.stack_id);
        //set user satp:
        src_data[36] = self.private_mem[0].table.get_root_stap();
    }
    pub fn set_extra_thread_args(&self, start_func: usize, arg_addr: usize) {
        let tid = self.tid;
        let trap_paddr = self.private_mem[0].get_trap_context_paddr(tid);
        let src_data = unsafe {core::slice::from_raw_parts_mut(trap_paddr as *mut usize , 512)};
        //r0 : start_func
        src_data[10] = start_func;
        //r1 : arg_addr
        src_data[11] = arg_addr;
    }
    pub fn write_syscall_return_value(&self, value: usize) {
        let tid = self.tid;
        let trap_paddr = self.private_mem[0].get_trap_context_paddr(tid);
        let src_data = unsafe {core::slice::from_raw_parts_mut(trap_paddr as *mut usize , 512)};
        //r0 : start_func
        src_data[10] = value;
    }
    pub fn read_syscall_return_value(&self) -> usize {
        let tid = self.tid;
        let trap_paddr = self.private_mem[0].get_trap_context_paddr(tid);
        let src_data = unsafe {core::slice::from_raw_parts_mut(trap_paddr as *mut usize , 512)};
        //r0 
        src_data[10]
    }
}