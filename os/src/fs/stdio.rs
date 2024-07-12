
use crate::mm::memory_set::UserBuffer;
use crate::fs::File;
use crate::drivers::chardev::UART;
use crate::drivers::chardev::CharDevice;

///Standard input
pub struct Stdin;
///Standard output
pub struct Stdout;

impl File for Stdin {
    fn readable(&self) -> bool {
        true
    }
    fn writable(&self) -> bool {
        false
    }
    fn read(&self, user_buf: &UserBuffer) -> usize {
        assert_eq!(user_buf.len, 1);
        let phys_buf = user_buf.kernel_bufs.get(&0).unwrap();
        let buffers = unsafe {core::slice::from_raw_parts_mut(phys_buf.start as *mut u8, phys_buf.len)};
        buffers[0] = UART.read();
        1
    }
    fn write(&self, _user_buf: &UserBuffer) -> usize {
        panic!("Cannot write to stdin!");
    }
}

impl File for Stdout {
    fn readable(&self) -> bool {
        false
    }
    fn writable(&self) -> bool {
        true
    }
    fn read(&self, _user_buf: &UserBuffer) -> usize {
        panic!("Cannot read from stdout!");
    }
    fn write(&self, user_buf: &UserBuffer) -> usize {
        let nums = user_buf.kernel_bufs.len();
        for i in 0..nums {
            let phys_buf = user_buf.kernel_bufs.get(&i).unwrap();
            let cur_buf = unsafe {core::slice::from_raw_parts(phys_buf.start as *const u8, phys_buf.len)};
            let str = core::str::from_utf8(cur_buf).unwrap();
            print!("{}", str);
        }
        user_buf.len
    }
}