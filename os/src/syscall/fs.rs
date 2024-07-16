use crate::fs::pipe::get_dual_pipe_file;
use crate::task::process::set_new_fd;
use crate::task::process::find_file_by_fd;
use crate::task::process::remove_fd;
use crate::task::process::dup_fd;
use crate::task::schedule::get_current_task;
use crate::mm::memory_set::UserBuffer;
use crate::fs::open_file;
use crate::fs::OpenFlags;
use alloc::string::String;

pub fn syscall_open(path: *const u8, len : usize, flags: u32) -> isize {
    let mut string = String::new();
    let user_buf = UserBuffer::new(path as usize, len);
    user_buf.read_buff_to_kernel_string(&mut string);
    if let Some(inode) = open_file(&string[0..string.len()-1], OpenFlags::from_bits(flags).unwrap()) {
        let pid =  get_current_task().to_pid();
        let fd = set_new_fd(pid, inode);
        fd as isize
    } else {
        -1
    }
}

pub fn syscall_close(fd : usize) -> isize {
    let pid = get_current_task().to_pid();
    remove_fd(pid, fd)
}

pub fn syscall_pipe(fd_buf: *mut u8) -> isize {
    let (pipe_read, pipe_write) = get_dual_pipe_file();
    let pid = get_current_task().to_pid();
    let read_fd = set_new_fd(pid, pipe_read);
    let write_fd = set_new_fd(pid, pipe_write);
    let user_buf = UserBuffer::new(fd_buf as usize, 2*core::mem::size_of::<usize>());
    if user_buf.kernel_bufs.len() == 1 {
        let phys_buf = user_buf.kernel_bufs.get(&0).unwrap();
        let cur_buf = unsafe {core::slice::from_raw_parts_mut(phys_buf.start as *mut usize, 2)};
        cur_buf[0] = read_fd;
        cur_buf[1] = write_fd;
    } else {
        //two pages
        let pipe_fds : [usize; 2] = [read_fd, write_fd];
        let phys_buf0 = user_buf.kernel_bufs.get(&0).unwrap();
        let phys_buf1 = user_buf.kernel_bufs.get(&1).unwrap();
        let cur_buf0 = unsafe {core::slice::from_raw_parts_mut(phys_buf0.start as *mut u8, phys_buf0.len)};
        let cur_buf1 = unsafe {core::slice::from_raw_parts_mut(phys_buf1.start as *mut u8, phys_buf1.len)};
        let raw_buf =  unsafe {core::slice::from_raw_parts(pipe_fds.as_ptr() as *const u8, 8)};
        let mut index = 0;
        for i in 0..phys_buf0.len {
            cur_buf0[i] = raw_buf[index];
            index += 1;
        }
        for i in 0..phys_buf1.len {
            cur_buf1[i] = raw_buf[index];
            index += 1;
        }
    }
    0
}

pub fn syscall_dup(fd : usize) -> isize {
    let pid = get_current_task().to_pid();
    dup_fd(pid, fd)
}

pub fn syscall_read(fd: usize, buf: *const u8, len: usize) -> isize {
    if len == 0 {
        return -2;
    }
    let pid = get_current_task().to_pid();
    match find_file_by_fd(pid, fd) {
        Some(file) => {
            if !file.readable() {
                return -1;
            }
            let user_buf = UserBuffer::new(buf as usize, len);
            file.read(&user_buf) as isize
        },
        None =>{
            -1
        }
    }
}

pub fn syscall_write(fd : usize, buf : *const u8, len : usize) ->isize {
    if len == 0 {
        return -2;
    }
    let pid = get_current_task().to_pid();
    match find_file_by_fd(pid, fd) {
        Some(file) => {
            if !file.writable() {
                return -1;
            }
            let user_buf = UserBuffer::new(buf as usize, len);
            file.write(&user_buf) as isize
        },
        None =>{
            -1
        }
    }
}