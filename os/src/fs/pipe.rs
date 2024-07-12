use crate::mm::memory_set::UserBuffer;
use alloc::sync::Arc;
use spin::Mutex;
use crate::task::suspend_task_and_run_next;
use crate::fs::File;
use crate::task::TaskhandleStatus;

//just use a 4k Page for pipe buffer
const PIPE_RINGBUFFER_SIZE : usize = 4096;

pub struct Pipe {
    readable: bool,
    writable: bool,
    buffer: Arc<Mutex<PipeRingBuffer>>,
}


impl Pipe {
    pub fn get_read_pipe(buffer: Arc<Mutex<PipeRingBuffer>>) -> Self {
        Self {
            readable: true,
            writable: false,
            buffer,
        }
    }
    pub fn get_write_pipe(buffer: Arc<Mutex<PipeRingBuffer>>) -> Self {
        Self {
            readable: false,
            writable: true,
            buffer,
        }
    }
}

impl Drop for Pipe {
    fn drop(&mut self) {
        if self.writable {
            self.buffer.lock().writer_dead()
        }
        if self.readable {
            self.buffer.lock().reader_dead()
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum RingBufferStatus {
    EMPTY,
    NORMAL,
    FULL,
}

pub struct PipeRingBuffer {
    arr: [u8; PIPE_RINGBUFFER_SIZE],
    head: usize,
    tail: usize,
    reader_alive : bool,
    writer_alive : bool,
    status: RingBufferStatus,
}

impl PipeRingBuffer {
    pub fn new() -> Self {
        Self {
            arr: [0; PIPE_RINGBUFFER_SIZE],
            head: 0,
            tail: 0,
            reader_alive : true,
            writer_alive : true,
            status: RingBufferStatus::EMPTY,
        }
    }
    pub fn writer_dead(&mut self) {
        self.writer_alive = false;
    }
    pub fn reader_dead(&mut self) {
        self.reader_alive = false;
    }
    pub fn avaliable_read(&self) -> usize {
        if self.status == RingBufferStatus::FULL {
            PIPE_RINGBUFFER_SIZE
        } else {
            if self.head <= self.tail {
                self.tail - self.head
            } else {
                self.tail - self.head + PIPE_RINGBUFFER_SIZE
            }
        }
    }
    pub fn avaliable_write(&self) -> usize {
        PIPE_RINGBUFFER_SIZE - self.avaliable_read()
    }
    pub fn read_data(&mut self, out_buf : &mut [u8])-> (TaskhandleStatus, usize) {
        //check if we can read
        if self.avaliable_read() == 0 {
            //no read data
            if self.writer_alive {
                return (TaskhandleStatus::DOWAIT, 0);
            } else {
                return (TaskhandleStatus::DOSTOP, 0);
            }
        } else {
            let mut read_num = out_buf.len();
            let mut status = TaskhandleStatus::OK;
            if self.avaliable_read() < read_num {
                //no avalibale read data, just stop
                read_num = self.avaliable_read();
                status = TaskhandleStatus::DOSTOP;
            }
            let mut ring_index = self.head;
            for index in 0..read_num {
                if ring_index == PIPE_RINGBUFFER_SIZE {
                    ring_index = 0;
                }
                out_buf[index] = self.arr[ring_index];
                ring_index += 1;
            }
            self.head = ring_index % PIPE_RINGBUFFER_SIZE;
            if self.head == self.tail {
                self.status = RingBufferStatus::EMPTY;
            }
            return (status, read_num);
        }
    }
    pub fn write_data(&mut self, in_buf : &[u8])-> (TaskhandleStatus, usize) {
        if self.avaliable_write() == 0 {
            //can not write, then check read status
            if self.reader_alive {
                return (TaskhandleStatus::DOWAIT, 0);
            } else {
                return (TaskhandleStatus::DOSTOP, 0);
            }
        } else {
            let mut write_num = in_buf.len();
            let mut status = TaskhandleStatus::OK;
            if self.avaliable_write() < write_num {
                //no avalibale write data, just stop
                write_num = self.avaliable_read();
                status = TaskhandleStatus::DOSTOP;
            }
            let mut ring_index = self.tail;
            for index in 0..write_num {
                if ring_index == PIPE_RINGBUFFER_SIZE {
                    ring_index = 0;
                }
                self.arr[ring_index] = in_buf[index];
                ring_index += 1;
            }
            self.tail = ring_index % PIPE_RINGBUFFER_SIZE;
            if self.head == self.tail {
                self.status = RingBufferStatus::FULL;
            }
            return (status, write_num);
        }
    }
}

impl File for Pipe {
    fn readable(&self) -> bool {
        self.readable
    }
    fn writable(&self) -> bool {
        self.writable
    }
    fn read(&self, user_buf: &UserBuffer) -> usize {
        let nums = user_buf.kernel_bufs.len();
        let mut len : usize = 0;
        for i in 0..nums {
            loop {
                let phys_buf = user_buf.kernel_bufs.get(&i).unwrap();
                let cur_buf = unsafe {core::slice::from_raw_parts_mut(phys_buf.start as *mut u8, phys_buf.len)};
                let (status, read_len) : (TaskhandleStatus, usize);
                {
                    (status, read_len) = self.buffer.lock().read_data(cur_buf);
                }
                match status {
                    TaskhandleStatus::DOWAIT => {
                        suspend_task_and_run_next();
                    },
                    TaskhandleStatus::DOSTOP => {
                        return read_len + len;
                    },
                    TaskhandleStatus::OK => {
                        len += read_len;
                        break;
                    },
                    _ => {
                        panic!("TaskhandleStatus unexpected!");
                    },
                }
            }
        }
        len
    }
    fn write(&self, user_buf: &UserBuffer) -> usize {
        let nums = user_buf.kernel_bufs.len();
        let mut len : usize = 0;
        for i in 0..nums {
            loop {
                let phys_buf = user_buf.kernel_bufs.get(&i).unwrap();
                let cur_buf = unsafe {core::slice::from_raw_parts(phys_buf.start as *const u8, phys_buf.len)};
                let (status, write_len) : (TaskhandleStatus, usize);
                {
                    (status, write_len) = self.buffer.lock().write_data(cur_buf);
                }
                match status {
                    TaskhandleStatus::DOWAIT => {
                        suspend_task_and_run_next();
                    },
                    TaskhandleStatus::DOSTOP => {
                        return write_len + len;
                    },
                    TaskhandleStatus::OK => {
                        len += write_len;
                        break;
                    },
                    _ => {
                        panic!("TaskhandleStatus unexpected!");
                    },
                }
            }
        }
        len
    }
}

pub fn get_dual_pipe_file() -> (Arc<Pipe>, Arc<Pipe>) {
    let buffer = Arc::new(Mutex::new(PipeRingBuffer::new()));
    let read_pipe = Arc::new(Pipe::get_read_pipe(buffer.clone()));
    let write_pipe = Arc::new(Pipe::get_write_pipe(buffer.clone()));
    (read_pipe, write_pipe)
}