use hashbrown::HashMap;
use crate::task::TaskContext;
use crate::trap::context::TrapContext;
use crate::task::id::IdWrapper;
use crate::mm::address::PhysPageNum;
use crate::mm::address::PhysAddr;
use crate::task::task_context::TaskStatus;
use core::arch::asm;
use crate::task::schedule::add_schedule_task;
use alloc::vec::Vec;
use alloc::boxed::Box;
use crate::mm::memory_set::UserMemorySets;
use crate::task::id::alloc_pid;
use crate::task::thread::Thread;
use crate::task::schedule::get_current_task;
use crate::fs::open_file;
use crate::fs::OpenFlags;
use crate::mm::memory_set::UserBuffer;
use alloc::sync::Arc;
use crate::fs::{File, Stdin, Stdout};
use crate::task::action::SignalHandler;
use crate::task::action::SignalAction;
use crate::task::signal::SignalFlags;
use crate::task::signal::check_signal_action;
use crate::task::signal::MAX_SIGNAL_NUM;
use crate::config::KERNEL_PAGE_SIZE;
use crate::task::TaskhandleStatus;
use crate::task::schedule::remove_schedule_task;
use crate::task::schedule::TaskID;
use crate::mm::memory_set::get_user_trap_context_start;
use crate::sync::SpinLock;
use crate::sync::MutexLock;
use crate::sync::Semaphore;
use crate::sync::Condvar;
use crate::sync::Mutex;

pub const FD_STDIN: usize = 0;
pub const FD_STDOUT: usize = 1;
pub const FD_STDERR: usize = 2;

//now only support 256 opened files for each process
const OS_MAX_FILE_DESCRIPTOR_NUM : usize = 256;
//1024 threads for each process should be enough
const OS_MAX_THREAD_NUM : usize = 1024;
//64 locks for a process should be enough
const OS_MAX_LOCK_NUM : usize = 64;

pub struct Process{
    pub pid : IdWrapper,
    pub ppid : usize,
    pub childpid : HashMap<usize, usize>,
    pub status : TaskStatus,
    pub exit_code : isize,
    pub fd_table: HashMap<usize, Arc<dyn File + Send + Sync>>,
    pub sig_handler : SignalHandler,
    pub threads : HashMap<usize, Thread>,
    //base memorys: text&rodata&bss&data......
    pub user_memorys : Vec<UserMemorySets>,
    pub spinlocks : Vec<Option<SpinLock>>,
    pub mutexlocks : Vec<Option<MutexLock>>,
    pub semaphores : Vec<Option<Semaphore>>,
    pub condvars : Vec<Option<Condvar>>,
}

impl Process {
    pub fn new(id : IdWrapper) ->Self {
        Self { 
            pid : id,
            ppid: 0,
            childpid : HashMap::new(),
            status : TaskStatus::READY,
            exit_code : 0,
            fd_table: HashMap::new(),
            sig_handler : SignalHandler::new(),
            threads : HashMap::new(),
            user_memorys : Vec::with_capacity(1),
            spinlocks: Vec::new(),
            mutexlocks : Vec::new(),
            semaphores : Vec::new(),
            condvars : Vec::new(),
        }
    }
    pub fn dump_process(&self) {
        println! ("pid:{}, ppid:{}", self.pid.id, self.ppid);
        for (pid, _) in &self.childpid {
            print!("child pid:{}", pid);
        }
        //println! ("process status:{}", self.status);
        println! ("process exit_code:{}", self.exit_code);
        for (fd, _) in &self.childpid {
            print!("file fd:{}", fd);
        }
        for (_, thread) in &self.threads {
            thread.dump_thread();
        }
    }
    fn alloc_an_new_fd(&self)->usize {
        let mut new_fd : usize = OS_MAX_FILE_DESCRIPTOR_NUM;
        for fd in 0..OS_MAX_FILE_DESCRIPTOR_NUM {
            if !self.fd_table.contains_key(&fd) {
                new_fd = fd;
                break;
            }
        }
        if new_fd == OS_MAX_FILE_DESCRIPTOR_NUM {
            panic!("cannot found any avaliable fd");
        }
        new_fd
    }
    fn alloc_an_thread_id(&self)->usize {
        let mut thread_id : usize = OS_MAX_THREAD_NUM;
        for id in 0..OS_MAX_THREAD_NUM {
            if !self.threads.contains_key(&id) {
                thread_id = id;
                break;
            }
        }
        if thread_id == OS_MAX_THREAD_NUM {
            panic!("cannot found any avaliable thread");
        }
        thread_id
    }
    pub fn set_new_file_descriptor(&mut self, file : Arc<dyn File + Send + Sync>) ->usize {
        let new_fd = self.alloc_an_new_fd();
        self.fd_table.insert(new_fd, file);
        new_fd
    }
    pub fn remove_file_descriptor(&mut self, fd : usize) ->isize {
        if self.fd_table.contains_key(&fd) {
            self.fd_table.remove(&fd);
            0
        } else {
            -1
        }
    }
    pub fn dup_file_descriptor(&mut self, fd : usize) ->isize {
        if !self.fd_table.contains_key(&fd) {
            -1
        } else {
            let file = self.fd_table.get(&fd);
            let new_fd = self.alloc_an_new_fd();
            self.fd_table.insert(new_fd, file.unwrap().clone());
            new_fd as isize
        }
    }
    pub fn mask_signal(&mut self, mask : i32) ->isize {
        let old_mask = self.sig_handler.global_mask.bits();
        if let Some(flags) = SignalFlags::from_bits(mask) {
            self.sig_handler.global_mask = flags;
            old_mask as isize
        } else {
            -1
        }
    }
    pub fn set_signal(&mut self, signum : i32) ->isize {
        if let Some(flags) = SignalFlags::from_bits(1 << signum) {
            if self.sig_handler.existed_signals.contains(flags) {
                -1
            } else {
                self.sig_handler.existed_signals.insert(flags);
                0
            }
        } else {
            -1
        }
    }
    pub fn active_signal(&mut self, signum : usize, action : usize,  old_action : usize) ->isize {
        if !check_signal_action(signum, action, old_action) {
            println! ("unexpected action: signum={}, action=0x{:0x}, old_action=0x{:0x}", signum, action, old_action);
            return -1;
        }
        let action_len = core::mem::size_of::<SignalAction>();
        let action_buf : UserBuffer = UserBuffer::new(action, action_len);
        let old_action_buf : UserBuffer = UserBuffer::new(old_action, action_len);
        let kern_action : usize = &self.sig_handler.action_table[signum] as *const SignalAction as usize;
        old_action_buf.write_kernel_slice_to_user(kern_action, action_len);
        action_buf.read_buff_to_kernel_slice(kern_action, action_len);
        return 0;
    }
    pub fn handle_user_signal(&mut self){
        let cur_signal = self.sig_handler.cur_signum;
        let hander_func = self.sig_handler.action_table[cur_signal as usize].handler;
        //now assume that tid is zero
        let cur_context = self.threads.get(&0).unwrap().private_mem[0].get_trap_context_paddr(0);
        let trap_context_size = core::mem::size_of::<TrapContext>()/core::mem::size_of::<usize>();
        let src_data = unsafe {core::slice::from_raw_parts_mut(cur_context as *mut usize , trap_context_size)};
        let dst_data = unsafe {core::slice::from_raw_parts_mut((cur_context + KERNEL_PAGE_SIZE/2) as *mut usize , trap_context_size)};
        dst_data.copy_from_slice(src_data);
        //sepc
        src_data[32] = hander_func;
        //the a0 should be sig_num??
        src_data[10] = cur_signal as usize;
    }
    pub fn signal_recover(&mut self)->isize{
        self.sig_handler.cur_signum = -1;
        let cur_context = self.threads.get(&0).unwrap().private_mem[0].get_trap_context_paddr(0);
        let trap_context_size = core::mem::size_of::<TrapContext>()/core::mem::size_of::<usize>();
        let dst_data = unsafe {core::slice::from_raw_parts_mut(cur_context as *mut usize , trap_context_size)};
        let src_data = unsafe {core::slice::from_raw_parts_mut((cur_context + KERNEL_PAGE_SIZE/2) as *mut usize , trap_context_size)};
        dst_data.copy_from_slice(src_data);
        //return the right
        dst_data[10] as isize
    }
    pub fn handle_signal(&mut self) -> TaskhandleStatus{
        let handler = &mut self.sig_handler;
        let mut should_handle : bool = false;
        for i in 0..MAX_SIGNAL_NUM {
            let signal = SignalFlags::from_bits(1 << i).unwrap();
            if handler.existed_signals.contains(signal) && !(handler.global_mask.contains(signal)) {
                //no signal are being handled
                if handler.cur_signum == -1 {
                    should_handle = true;
                } else {
                    if !handler.action_table[i].mask.contains(signal) {
                        should_handle = true;
                    }
                }
            }
            if should_handle {
                match signal {
                    //kernel flags may kill application
                    SignalFlags::SIGDEF |
                    SignalFlags::SIGINT | 
                    SignalFlags::SIGILL | 
                    SignalFlags::SIGABRT |
                    SignalFlags::SIGFPE |
                    SignalFlags::SIGKILL |
                    SignalFlags::SIGSEGV => {
                        if let Some((errno, msg)) = signal.check_error() {
                            println!("***KILL APPLICATION*** err:{}.", msg);
                            self.exit_code = errno as isize;
                        }
                        return TaskhandleStatus::DOSTOP;
                    },
                    //continue & stop signals
                    SignalFlags::SIGCONT => {
                        //clear stop&cont signal
                        handler.existed_signals.remove(SignalFlags::SIGSTOP);
                        handler.existed_signals.remove(signal);
                        return TaskhandleStatus::OK;
                    }
                    SignalFlags::SIGSTOP => {
                        //wait for sigcount to come
                        return TaskhandleStatus::DOWAIT;
                    },
                    //other user flags
                    _ => {
                        let action = &handler.action_table[i];
                        if action.handler != 0 {
                            handler.cur_signum = i as isize;
                            //clear current signal
                            handler.existed_signals.remove(signal);
                            return TaskhandleStatus::DOSIGNAL;
                        }
                    },
                }
            }
        }
        return TaskhandleStatus::OK;
    }
    pub fn remove_other_threads(&mut self, thread_id : usize){
        let pid = self.pid.id;
        let mut tids : Vec<usize> = Vec::with_capacity(self.threads.len());
        for (tid, _) in &self.threads {
            if *tid != thread_id {
                tids.push(*tid);
            }
        }
        for thread in &tids {
            remove_schedule_task(pid, *thread);
            self.threads.remove(thread);
        }
    }
    pub fn replace_process(&mut self, elf_data: &[u8], tid : usize, args : &[usize]) {
        //load new elf
        let mut new_user_mem = UserMemorySets::new();
        let entry_point = new_user_mem.load_with_elf(elf_data);
        //process table must be same with thread table
        let main_thread =  self.threads.get_mut(&tid).unwrap();
        let root_ppn = new_user_mem.table.root_ppn;
        main_thread.init_private_memorys(PhysAddr::from(root_ppn).into());
        main_thread.set_user_trap_context(entry_point, args);
        main_thread.init_task_data();
        //remove other thread
        self.remove_other_threads(tid);
        //insert current memory data
        self.user_memorys.pop();
        self.user_memorys.push(new_user_mem);
    }
    pub fn exit_process(&mut self, tid : usize, exit_code : isize) {
        //set current APP to zombie
        self.status = TaskStatus::ZOMBIE;
        if exit_code != 0 {
            self.exit_code = exit_code;
        }
        self.fd_table.clear();
        //remove other thread
        self.remove_other_threads(tid);
        //remove userpace memorys
        self.user_memorys.pop();
        self.threads.get_mut(&tid).unwrap().private_mem.pop();
        //remove other resouces
        self.mutexlocks.clear();
    }
    pub fn fork_process(&self, old_tid: usize, new_process : &mut Process){
        //set process info
        new_process.ppid = self.pid.id;
        //copy fd table
        let old_table = &self.fd_table;
        for (fd, file) in old_table {
            new_process.fd_table.insert(*fd, file.clone());
        }
        //fork user memory
        new_process.user_memorys.push(UserMemorySets::new());
        self.user_memorys[0].fork_user_memory(&mut new_process.user_memorys[0]);
        //fork thread
        let mut new_thread = Thread::new(old_tid);
        // the process table must be same with thread table
        let root_ppn = new_process.user_memorys[0].table.root_ppn;
        let mut new_thread_mem = UserMemorySets::new();
        new_thread_mem.table.set_root_ppn(PhysAddr::from(root_ppn).into());
        new_thread.private_mem.push(new_thread_mem);
        self.threads.get(&old_tid).unwrap().fork_thread(&mut new_thread);
        new_process.threads.insert(old_tid, new_thread);
    }
    pub fn add_thread(&mut self, thread_func: usize, start_func: usize, arg_addr: usize)-> isize {
        let tid = self.alloc_an_thread_id();
        let mut new_thread = Thread::new(tid);
        //process table must be same with thread table
        let root_ppn = self.user_memorys[0].table.root_ppn;
        new_thread.init_private_memorys(PhysAddr::from(root_ppn).into());
        //no start-up args
        let args : [usize; 2] = [0 ; 2];
        new_thread.set_user_trap_context(thread_func, &args);
        new_thread.init_task_data();
        new_thread.set_extra_thread_args(start_func, arg_addr);
        self.threads.insert(tid, new_thread);
        add_schedule_task(self.pid.id, tid);
        tid as isize
    }
    pub fn exit_thread(&mut self, tid: usize, exit_code: i32)-> isize {
        //main thread can also be exited, So if we are the last
        //thread running, we should exit the process
        let mut alive_alone = true;
        for (other_tid, thread) in &self.threads {
            if *other_tid == tid {
                continue;
            }
            if thread.status != TaskStatus::ZOMBIE {
                alive_alone =  false;
                break;
            }
        }
        if alive_alone {
            println!("thread live alone:{}, exit process now", tid);
            self.exit_process(tid, exit_code as isize);
        } else {
            //make the thread to be zombie and wait others to release
            let cur_thread = self.threads.get_mut(&tid).unwrap();
            //remove current user memorys & reserve kernel stack memory
            cur_thread.private_mem[0].remove_all_map();
            cur_thread.private_mem.pop();
            //-2 is reserved for running status
            if exit_code != -2 {
                cur_thread.exit_code = exit_code as usize;
            }
            cur_thread.status = TaskStatus::ZOMBIE;
        }
        0
    }
    pub fn wait_thread(&mut self, tid: usize)-> isize {
        if !self.threads.contains_key(&tid) {
            println!("Thread:{} is not existed", tid);
            return -3;
        }
        let status = self.threads.get(&tid).unwrap().status;
        let exit_code = self.threads.get(&tid).unwrap().exit_code;
        //remove zombie thread if needed
        if status == TaskStatus::ZOMBIE {
            //remove thread
            self.threads.remove(&tid);
            return exit_code as isize;
        } else {
            //thread is still running
            return -2;
        }
    }
    fn add_new_mutex_lock(&mut self)->isize {
        let mut id : usize = OS_MAX_LOCK_NUM;
        if self.mutexlocks.len() >= OS_MAX_LOCK_NUM {
            return -1;
        }
        for i in 0..self.mutexlocks.len() {
            if self.mutexlocks[i].is_none() {
                id = i;
                break;
            }
        }
        if id == OS_MAX_LOCK_NUM {
            self.mutexlocks.push(Some(MutexLock::new()));
            id = self.mutexlocks.len() - 1;
        } else {
            self.mutexlocks[id] = Some(MutexLock::new());
        }
        id as isize
    }
    fn add_new_spin_lock(&mut self)->isize {
        let mut id : usize = OS_MAX_LOCK_NUM;
        if self.spinlocks.len() >= OS_MAX_LOCK_NUM {
            return -1;
        }
        for i in 0..self.spinlocks.len() {
            if self.spinlocks[i].is_none() {
                id = i;
                break;
            }
        }
        if id == OS_MAX_LOCK_NUM {
            self.spinlocks.push(Some(SpinLock::new()));
            id = self.spinlocks.len() - 1;
        } else {
            self.spinlocks[id] = Some(SpinLock::new());
        }
        (id + OS_MAX_LOCK_NUM) as isize
    }
    fn add_new_lock(&mut self, blocking : bool)->isize {
        if blocking {
            self.add_new_mutex_lock()
        } else {
            self.add_new_spin_lock()
        }
    }
    fn process_lock_operation(&mut self, id : usize, lock : bool)->isize {
        if id < OS_MAX_LOCK_NUM {
            if id < self.mutexlocks.len() {
                if self.mutexlocks[id].is_some() {
                    if lock {
                        return self.mutexlocks[id].as_mut().unwrap().lock() as isize;
                    } else {
                        self.mutexlocks[id].as_mut().unwrap().unlock();
                    }
                    return 0;
                }
            }
        } else if (id - OS_MAX_LOCK_NUM) < OS_MAX_LOCK_NUM {
            let spin_id = id - OS_MAX_LOCK_NUM;
            if spin_id < self.spinlocks.len() {
                if self.spinlocks[spin_id].is_some() {
                    if lock {
                        return self.spinlocks[spin_id].as_mut().unwrap().lock() as isize;
                    } else {
                        self.spinlocks[spin_id].as_mut().unwrap().unlock();
                    }
                    return 0;
                }
            }
        }
        return -1;
    }
    fn add_new_semaphore(&mut self, res_count : usize)->isize {
        let mut id : usize = 0xFFFFFFFF;
        for i in 0..self.semaphores.len() {
            if self.semaphores[i].is_none() {
                id = i;
                break;
            }
        }
        if id == 0xFFFFFFFF {
            self.semaphores.push(Some(Semaphore::new(res_count)));
            id = self.semaphores.len() -1;
        } else {
            self.semaphores[id] = Some(Semaphore::new(res_count));
        }
        id as isize
    }
    fn process_sem_operation(&mut self, sem_id : usize, up : bool)->isize {
        if sem_id >= self.semaphores.len() {
            return -1;
        }
        if self.semaphores[sem_id].is_none() {
            return -2;
        }
        if up {
            self.semaphores[sem_id].as_mut().unwrap().up();
        }  else {
            self.semaphores[sem_id].as_mut().unwrap().down();
        }
        return 0;
    }
    fn add_new_condvar(&mut self)->isize {
        let mut id : usize = 0xFFFFFFFF;
        for i in 0..self.condvars.len() {
            if self.condvars[i].is_none() {
                id = i;
                break;
            }
        }
        if id == 0xFFFFFFFF {
            self.condvars.push(Some(Condvar::new()));
            id = self.condvars.len() -1;
        } else {
            self.condvars[id] = Some(Condvar::new());
        }
        id as isize
    }
    fn operate_condvar(&mut self, condvar_id : usize, mutext_id : usize, signal : bool)->isize {
        if condvar_id >= self.condvars.len() {
            return -1;
        }
        if self.condvars[condvar_id].is_none() {
            return -2;
        }
        if signal {
            //currently wake up all threads
            self.condvars[condvar_id].as_mut().unwrap().signal_all();
            0
        } else {
            //wait, this requires unlock & lock
            self.process_lock_operation(mutext_id, false);
            self.condvars[condvar_id].as_mut().unwrap().wait();
            self.process_lock_operation(mutext_id, true);
            0
        }
    }
    fn add_framebuffer(&mut self, phys_framebuffer : usize, buf_len : usize) ->isize {
        self.user_memorys[0].add_framebuffer_addr(phys_framebuffer, buf_len)
    }
}

pub struct ProcessPool {
    pub processes : HashMap<usize, Process>,
    //no other purpose, just skip the ilde pid
    pub idle_pid : IdWrapper,
}

impl ProcessPool {
    pub fn new() ->Self {
        Self {
            processes : HashMap::new(),
            idle_pid : alloc_pid(),
        }
    }
    #[allow(unused)]
    pub fn dump_all_apps(&self) {
        println! ("dump all apps");
        for (_, process) in &self.processes{
            process.dump_process();
        }
    }
    pub fn exec_app(&mut self, elf_data: &[u8], args : &[usize]) {
        //remove old app
        let (pid, tid) = get_current_task().to_pid_tid();
        //just replace current forked process
        let process = self.processes.get_mut(&pid).unwrap();
        process.replace_process(elf_data, tid, args);
        //update instruction cache
        unsafe {
            asm!("fence.i");
        }
    }
    pub fn load_init_porcess(&mut self, elf_data: &[u8]) {
        //set a empty process with main thread
        let id = alloc_pid();
        let pid = id.id;
        self.processes.insert(pid, Process::new(id));
        let process = self.processes.get_mut(&pid).unwrap();
        process.threads.insert(0, Thread::new(0));
        //no start-up args
        let args : [usize; 2] = [0 ; 2];
        process.replace_process(elf_data,0, &args);
        //add stdin & stdout & stderr
        process.fd_table.insert(FD_STDIN, Arc::new(Stdin));
        process.fd_table.insert(FD_STDOUT, Arc::new(Stdout));
        process.fd_table.insert(FD_STDERR, Arc::new(Stdout));
        //add task schedule
        add_schedule_task(pid, 0);
        //update instruction cache
        unsafe {
            asm!("fence.i");
        }
    }
    pub fn fork_app(&mut self) -> usize{
        //get old task first
        let (old_pid, old_tid) = get_current_task().to_pid_tid();
        //get new pid
        let pid_wrapper = alloc_pid();
        let new_pid = pid_wrapper.id;
        let mut new_process: Process = Process::new(pid_wrapper);
        //fork data & push into child
        self.processes.get_mut(&old_pid).unwrap().fork_process(old_tid, &mut new_process);
        self.processes.get_mut(&old_pid).unwrap().childpid.insert(new_pid, new_pid);
        //insert to POOL
        self.processes.insert(new_pid, new_process);
        //add task schedule
        add_schedule_task(new_pid, old_tid);
        //println! ("forked app: {}", new_pid);
        //update instruction cache
        unsafe {
            asm!("fence.i");
        }
        new_pid
    }
    pub fn exit_cur_app(&mut self, exit_code : isize) {
        let (pid, tid) = get_current_task().to_pid_tid();
        self.processes.get_mut(&pid).unwrap().exit_process(tid, exit_code);
        //all child process will be taken care of by init process
        let cur_task = self.processes.get(&pid).unwrap();
        //this is strange, but we have no method
        let mut child_process : Vec<usize> = Vec::new();
        for (child, _) in &cur_task.childpid {
            child_process.push(*child);
        }
        for process in &child_process {
            self.processes.get_mut(&1).unwrap().childpid.insert(*process, *process);
            self.processes.get_mut(process).unwrap().ppid = 1;
        }
    }
    pub fn get_task_context(&self, pid : usize, tid : usize) -> *const TaskContext {
        let process = self.processes.get(&pid).unwrap();
        let thread = process.threads.get(&tid).unwrap();
        &thread.context as *const TaskContext
    }

}


static mut PROCESSES: Option<&mut ProcessPool> = None;

pub fn init_kernel_task_manager()
{
    let pool = Box::new(ProcessPool::new());
    unsafe {
        PROCESSES = Some(Box::leak(pool));
    }
}

pub fn get_app_context(pid : usize, tid : usize)-> *const TaskContext {
    unsafe {
        PROCESSES.as_mut().unwrap().get_task_context(pid, tid)
    }
}

pub fn exit_current_app(exit_code : isize) {
    unsafe {
        PROCESSES.as_mut().unwrap().exit_cur_app(exit_code)
    }
}

pub fn fork_new_app()->usize {
    unsafe {
        PROCESSES.as_mut().unwrap().fork_app()
    }
}

pub fn get_current_root_ppn()->PhysPageNum{
    let pid = get_current_task().to_pid();
    unsafe {
        PROCESSES.as_mut().unwrap().processes.get(&pid).unwrap().user_memorys[0].table.root_ppn
    }
}

//context addr in user mode
pub fn get_current_context_uaddr()->usize {
    let tid = get_current_task().to_tid();
    get_user_trap_context_start(tid)
}
//context addr in kernel mode
pub fn get_current_context_kaddr()->usize {
    let (pid, tid) = get_current_task().to_pid_tid();
    unsafe {
        let process = PROCESSES.as_mut().unwrap().processes.get(&pid).unwrap();
        process.user_memorys[0].get_trap_context_paddr(tid)
    }
}

pub fn check_is_empty()->bool {
    let pid = get_current_task().to_pid();
    unsafe {
        let process = PROCESSES.as_mut().unwrap().processes.get_mut(&pid).unwrap();
        process.childpid.is_empty()
    }
}

pub fn recycle_an_child(pid: usize)
{
    let cur_pid = get_current_task().to_pid();
    unsafe {
        PROCESSES.as_mut().unwrap().processes.get_mut(&cur_pid).unwrap().childpid.remove(&pid);
        PROCESSES.as_mut().unwrap().processes.remove(&pid);
    }
}

fn write_exit_code(exit_code_ptr: *mut i32, exit_code : i32)
{
    let result_len = core::mem::size_of::<i32>();
    let user_buf = UserBuffer::new(exit_code_ptr as usize, result_len);
    let code : [i32; 1] = [exit_code; 1];
    user_buf.write_kernel_slice_to_user(code.as_ptr() as usize, result_len);
}

pub fn set_new_fd(pid : usize, file : Arc<dyn File + Send + Sync>) -> usize{
    unsafe {
        let process = PROCESSES.as_mut().unwrap().processes.get_mut(&pid);
        process.unwrap().set_new_file_descriptor(file)
    }
}

pub fn remove_fd(pid : usize, fd : usize) -> isize{
    unsafe {
        let process = PROCESSES.as_mut().unwrap().processes.get_mut(&pid);
        process.unwrap().remove_file_descriptor(fd)
    }
}

pub fn dup_fd(pid : usize, fd : usize) -> isize{
    unsafe {
        let process = PROCESSES.as_mut().unwrap().processes.get_mut(&pid);
        process.unwrap().dup_file_descriptor(fd)
    }
}

pub fn set_signal_mask(mask : i32) -> isize{
    let cur_pid = get_current_task().to_pid();
    unsafe {
        let process = PROCESSES.as_mut().unwrap().processes.get_mut(&cur_pid);
        process.unwrap().mask_signal(mask)
    }
}

pub fn set_signal(pid : usize, signum : i32) -> isize{
    let cur_pid : usize;
    //check signum first
    if signum >= MAX_SIGNAL_NUM as i32 {
        return -1;
    }
    if pid == 0 {
        cur_pid = get_current_task().to_pid();
    } else {
        cur_pid = pid;
    }
    unsafe {
        let process = PROCESSES.as_mut().unwrap().processes.get_mut(&cur_pid);
        process.unwrap().set_signal(signum)
    }
}

pub fn set_signal_action(signum : i32, action : usize,  old_action : usize) -> isize{
    let cur_pid = get_current_task().to_pid();
    unsafe {
        let process = PROCESSES.as_mut().unwrap().processes.get_mut(&cur_pid);
        process.unwrap().active_signal(signum as usize, action, old_action)
    }
}

pub fn handle_current_signals()->TaskhandleStatus{
    let cur_pid = get_current_task().to_pid();
    unsafe {
        let process: Option<&mut Process> = PROCESSES.as_mut().unwrap().processes.get_mut(&cur_pid);
        process.unwrap().handle_signal()
    }
}

pub fn handle_an_user_signal(){
    let cur_pid = get_current_task().to_pid();
    unsafe {
        let process: Option<&mut Process> = PROCESSES.as_mut().unwrap().processes.get_mut(&cur_pid);
        process.unwrap().handle_user_signal()
    }
}

pub fn return_from_signal()->isize{
    let cur_pid = get_current_task().to_pid();
    unsafe {
        let process = PROCESSES.as_mut().unwrap().processes.get_mut(&cur_pid);
        process.unwrap().signal_recover()
    }
}

pub fn find_file_by_fd(pid : usize, fd : usize)-> Option<Arc<dyn File + Send + Sync>> {
    unsafe {
        let process = PROCESSES.as_mut().unwrap().processes.get(&pid);
        match process.unwrap().fd_table.get(&fd) {
            Some(file) => {
                Some(file.clone())
            },
            None => {
                None
            }
        }
    }
}

pub fn wait_any_child(exit_code_ptr: *mut i32)-> isize {
    unsafe {
        if check_is_empty() {
            //println! ("child is empty...");
            return -1;
        }
        let cur_pid = get_current_task().to_pid();
        let thread = PROCESSES.as_mut().unwrap().processes.get_mut(&cur_pid).unwrap();
        for (id, _) in &thread.childpid {
            let child = PROCESSES.as_mut().unwrap().processes.get_mut(id).unwrap();
            if child.status == TaskStatus::ZOMBIE {
                write_exit_code(exit_code_ptr, child.exit_code as i32);
                recycle_an_child(*id);
                return *id as isize
            } else {
                continue;
            }
        }
        return -2;
    }
}

pub fn wait_single_child(pid: isize, exit_code_ptr: *mut i32)-> isize {
    unsafe {
        if check_is_empty() {
            return -1;
        }
        let child_pid : usize = pid as usize;
        let cur_pid = get_current_task().to_pid();
        if !PROCESSES.as_mut().unwrap().processes.get(&cur_pid).unwrap().childpid.contains_key(&child_pid){
            println! ("pid {} is not {}'s parrent", cur_pid, pid);
            return -1;
        }
        let child = PROCESSES.as_mut().unwrap().processes.get(&child_pid).unwrap();
        if child.status == TaskStatus::ZOMBIE {
            write_exit_code(exit_code_ptr, child.exit_code as i32);
            recycle_an_child(child_pid);
            return pid;
        } else {
            //println! ("child {} is still running, parent:{}", child_pid, cur_pid);
            return -2;
        }
    }
}

pub fn exec_an_app(elf_data: &[u8], args : &[usize]) {
    unsafe {
        PROCESSES.as_mut().unwrap().exec_app(elf_data, args)
    }
}

pub fn run_init_process() 
{
    let inode = open_file("initproc", OpenFlags::RDONLY).unwrap();
    let v = inode.read_all();
    unsafe {
        PROCESSES.as_mut().unwrap().load_init_porcess(v.as_slice());
    }
}


pub fn create_new_thread(thread_func: usize, start_func: usize, arg_addr: usize)->isize {
    let cur_pid = get_current_task().to_pid();
    unsafe {
        let process: Option<&mut Process> = PROCESSES.as_mut().unwrap().processes.get_mut(&cur_pid);
        process.unwrap().add_thread(thread_func, start_func, arg_addr)
    }
}

pub fn exit_current_thread(exit_code: i32)->isize {
    let (pid, tid) = get_current_task().to_pid_tid();
    unsafe {
        let process: Option<&mut Process> = PROCESSES.as_mut().unwrap().processes.get_mut(&pid);
        process.unwrap().exit_thread(tid, exit_code)
    }
}

pub fn wait_thread(tid : usize)->isize {
    let (pid, cur_tid) = get_current_task().to_pid_tid();
    if cur_tid == tid {
        println!("A thread:{} should never wait itself..", cur_tid);
        return -1;
    }
    unsafe {
        let process: Option<&mut Process> = PROCESSES.as_mut().unwrap().processes.get_mut(&pid);
        process.unwrap().wait_thread(tid)
    }
}

pub fn try_wakeup_task(task_id : TaskID) {
    let (pid, tid) = task_id.to_pid_tid();
    unsafe {
        //if process is not exsited, just return
        if !PROCESSES.as_mut().unwrap().processes.contains_key(&pid) {
            return;
        }
        let process = PROCESSES.as_mut().unwrap().processes.get_mut(&pid).unwrap();
        //process should be running
        if process.status != TaskStatus::READY {
            return;
        }
        if !process.threads.contains_key(&tid){
            return;
        }
        let thread = process.threads.get_mut(&tid).unwrap();
        //when thread in ready status, it has been stopped
        if thread.status != TaskStatus::READY {
            panic!("wake up non-ready thread: pid={}, tid={}.", task_id.to_pid(), task_id.to_tid());
            //return;
        } else {
            thread.status = TaskStatus::RUNNING;
            add_schedule_task(pid, tid)
        }
    }
}

pub fn block_current_task(){
    let (pid, tid) = get_current_task().to_pid_tid();
    unsafe {
        let process: Option<&mut Process> = PROCESSES.as_mut().unwrap().processes.get_mut(&pid);
        process.unwrap().threads.get_mut(&tid).unwrap().status = TaskStatus::READY;
    }
}

pub fn add_new_mutext(blocking : bool) -> isize {
    let pid = get_current_task().to_pid();
    unsafe {
        PROCESSES.as_mut().unwrap().processes.get_mut(&pid).unwrap().add_new_lock(blocking)
    }
}

pub fn operation_lock(lock_id : usize, lock : bool) -> isize
{
    let pid = get_current_task().to_pid();
    unsafe {
        PROCESSES.as_mut().unwrap().processes.get_mut(&pid).unwrap().process_lock_operation(lock_id, lock)
    }
}

pub fn add_semaphore(res_count: usize) -> isize {
    let pid = get_current_task().to_pid();
    unsafe {
        PROCESSES.as_mut().unwrap().processes.get_mut(&pid).unwrap().add_new_semaphore(res_count)
    }
}

pub fn operation_semaphore(sem_id : usize, up : bool) -> isize
{
    let pid = get_current_task().to_pid();
    unsafe {
        PROCESSES.as_mut().unwrap().processes.get_mut(&pid).unwrap().process_sem_operation(sem_id, up)
    }
}

pub fn add_condvar() -> isize {
    let pid = get_current_task().to_pid();
    unsafe {
        PROCESSES.as_mut().unwrap().processes.get_mut(&pid).unwrap().add_new_condvar()
    }
}

pub fn operation_condvar(condvar_id : usize, mutext_id : usize, signal : bool) -> isize
{
    let pid = get_current_task().to_pid();
    unsafe {
        PROCESSES.as_mut().unwrap().processes.get_mut(&pid).unwrap().operate_condvar(condvar_id, mutext_id, signal)
    }
}

pub fn map_framebuffer(phys_framebuffer : usize, buf_len : usize) -> isize {
    let pid = get_current_task().to_pid();
    unsafe {
        PROCESSES.as_mut().unwrap().processes.get_mut(&pid).unwrap().add_framebuffer(phys_framebuffer, buf_len)
    }
}

pub fn set_syscall_return_value(task_id : TaskID, value : usize) {
    let (pid, tid) = task_id.to_pid_tid();
    unsafe {
        //if process is not exsited, just return
        if !PROCESSES.as_mut().unwrap().processes.contains_key(&pid) {
            return;
        }
        let process = PROCESSES.as_mut().unwrap().processes.get_mut(&pid).unwrap();
        if !process.threads.contains_key(&tid){
            return;
        }
        let thread = process.threads.get_mut(&tid).unwrap();
        if !thread.private_mem.is_empty() {
            thread.write_syscall_return_value(value);
        }
    }
}

pub fn get_syscall_return_value(task_id : TaskID) -> isize{
    let (pid, tid) = task_id.to_pid_tid();
    unsafe {
        //if process is not exsited, just return
        if !PROCESSES.as_mut().unwrap().processes.contains_key(&pid) {
            panic!("process is not existed??");
        }
        let process = PROCESSES.as_mut().unwrap().processes.get_mut(&pid).unwrap();
        if !process.threads.contains_key(&tid){
            panic!("thread is not existed??");
        }
        let thread = process.threads.get_mut(&tid).unwrap();
        if !thread.private_mem.is_empty() {
            thread.read_syscall_return_value() as isize
        } else {
            panic!("thread is dead??");
        }
    }
}