use crate::mm::address::PhysPageNum;
use crate::mm::address::PhysAddr;
use alloc::vec::Vec;
use crate::board::AVALIABLE_MEMORY_END;
use crate::mm::address::round_down_in_4k;
use crate::common::memset_usize;
use alloc::boxed::Box;

pub struct FrameWrapper {
    pub ppn: PhysPageNum,
}

impl FrameWrapper {
    pub fn new(page : PhysPageNum) ->Self {
        Self {
            ppn : page,
        }
    }
    pub fn clear_frame(&self) {
        //clear physical page, beacuse it might be used as page table
        let page_addr : usize = PhysAddr::from(self.ppn).into();
        //here physical address is equal to kernel virtual address
        memset_usize(page_addr, 0, 512);
    }
}
///dealloc frame if FrameWrapper is not used
impl Drop for FrameWrapper {
    fn drop(&mut self) {
        frame_dealloc(self.ppn);
    }
}

trait FrameAllocator {
    fn alloc(&mut self) -> Option<PhysPageNum>;
    fn dealloc(&mut self, ppn : PhysPageNum);
}

pub struct StackFrameAllocator {
    cur_ppn : usize,
    last_ppn : usize,
    unused_ppns : Vec<usize>,
}

impl StackFrameAllocator {
    pub fn new()->Self {
        Self {
            cur_ppn : 0,
            last_ppn : 0,
            unused_ppns : Vec::new(),
        }
    }
    pub fn init(&mut self, cur : PhysPageNum,  last : PhysPageNum) {
        self.cur_ppn = cur.0;
        self.last_ppn = last.0;
    }
}

static mut FRAME_ALLOCATOR: Option<&mut StackFrameAllocator> = None;
impl FrameAllocator for StackFrameAllocator {
    fn alloc(&mut self) -> Option<PhysPageNum> {
        if let Some(ppn) = self.unused_ppns.pop() {
            Some(ppn.into())
        } else if self.cur_ppn == self.last_ppn{
            None
        } else {
            self.cur_ppn += 1;
            Some((self.cur_ppn - 1).into())
        }
    }
    fn dealloc(&mut self, ppn : PhysPageNum) {
        if ppn.0 > self.cur_ppn || self.unused_ppns.iter().any(|&v| v == ppn.0) {
            panic!("Frame ppn={:#x} has not been allocated!", ppn.0);
        }
        self.unused_ppns.push(ppn.0);
    }
}

pub fn init_frame_allocator() {
    extern "C" {
        fn ekernel();
    }
    unsafe {
        let allocator = Box::new(StackFrameAllocator::new());
        FRAME_ALLOCATOR = Some(Box::leak(allocator));
        FRAME_ALLOCATOR.as_mut().unwrap().init(
            PhysAddr::from(ekernel as usize).into(), 
            PhysAddr::from(round_down_in_4k(AVALIABLE_MEMORY_END)).into());
    }   
}

pub fn frame_alloc()->Option<FrameWrapper> {
    unsafe {
        FRAME_ALLOCATOR.as_mut().unwrap().alloc().map(FrameWrapper::new)
    }
}

pub fn frame_dealloc(ppn : PhysPageNum) {
    unsafe {
        FRAME_ALLOCATOR.as_mut().unwrap().dealloc(ppn)
    }
}