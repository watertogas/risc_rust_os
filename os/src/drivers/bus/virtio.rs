
use crate::mm::frame_allocator::frame_alloc;
use crate::mm::frame_allocator::frame_dealloc;
use crate::mm::page_table::kern_vaddr_to_phy_addr;
use crate::mm::address::StepByOne;
use crate::mm::address::PhysAddr;
use crate::mm::address::PhysPageNum;
use crate::mm::frame_allocator::FrameWrapper;
use lazy_static::*;
use virtio_drivers::Hal;
use spin::Mutex;
use alloc::vec::Vec;

pub struct VirtioHal;

lazy_static! {
    static ref QUEUE_FRAMES: Mutex<Vec<FrameWrapper>> = Mutex::new(Vec::new());
}

impl Hal for VirtioHal {
    fn dma_alloc(pages: usize) -> usize {
        let mut ppn_base = PhysPageNum(0);
        for i in 0..pages {
            let frame = frame_alloc().unwrap();
            if i == 0 {
                ppn_base = frame.ppn;
            }
            assert_eq!(frame.ppn.0, ppn_base.0 + i);
            //println!("allocated frame ppn=0x{:x}", frame.ppn.0);
            QUEUE_FRAMES.lock().push(frame);
        }
        let pa: PhysAddr = ppn_base.into();
        pa.0
    }

    fn dma_dealloc(pa: usize, pages: usize) -> i32 {
        let pa = PhysAddr::from(pa);
        let mut ppn_base: PhysPageNum = pa.into();
        for _ in 0..pages {
            frame_dealloc(ppn_base);
            ppn_base.step();
        }
        0
    }
    //now vaddr == paddr in kernel
    fn phys_to_virt(addr: usize) -> usize {
        addr
    }
    fn virt_to_phys(vaddr: usize) -> usize {        
        kern_vaddr_to_phy_addr(vaddr)
    }
}