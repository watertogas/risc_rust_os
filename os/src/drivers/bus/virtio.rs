
use crate::mm::page_table::kern_vaddr_to_phy_addr;
use crate::mm::address::PhysAddr;
use lazy_static::*;
use virtio_drivers::Hal;
use crate::config::AVALIABLE_FRAMES_END;
use crate::config::AVALIABLE_MEMORY_END;
use crate::config::KERNEL_PAGE_SIZE;
use buddy_system_allocator::LockedFrameAllocator;

pub struct VirtioHal;

lazy_static! {
    static ref DMA_ALLOCATOR: LockedFrameAllocator = LockedFrameAllocator::new();
}

pub fn init_dma_allocator(){
    let dma_start : PhysAddr = PhysAddr::from(AVALIABLE_FRAMES_END);
    let dma_end : PhysAddr = PhysAddr::from(AVALIABLE_MEMORY_END);
    DMA_ALLOCATOR.lock().add_frame(dma_start.0, dma_end.0);
}

impl Hal for VirtioHal {
    fn dma_alloc(pages: usize) -> usize {
        let total_size = pages * KERNEL_PAGE_SIZE;
        if let Some(phys_addr) = DMA_ALLOCATOR.lock().alloc(total_size) {
            phys_addr
        } else {
            panic!("Can not alloc memory for pages: {}", pages);
        }
    }

    fn dma_dealloc(pa: usize, pages: usize) -> i32 {
        let total_size = pages * KERNEL_PAGE_SIZE;
        DMA_ALLOCATOR.lock().dealloc(pa, total_size);
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