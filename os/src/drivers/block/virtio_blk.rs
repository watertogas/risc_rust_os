
use alloc::vec::Vec;
use virtio_drivers::{VirtIOBlk, VirtIOHeader, BlkResp, RespStatus};
use crate::drivers::bus::virtio::VirtioHal;
use crate::drivers::block::BlockDevice;
use crate::sync::Condvar;
use crate::config::get_file_block_mode;
use crate::task::block_task_and_run_next;
const VIRTIO0: usize = 0x10008000;
use crate::sync::OneCoreCell;
use crate::sync::InterruptMask;


pub struct MTVirtBlk {
    pub virt_hal : VirtIOBlk<'static, VirtioHal>,
    //there will be 16 queues at most
    pub condvars : Vec<Condvar>,
}

impl MTVirtBlk {
    #[allow(unused)]
    pub fn new() -> Self {
        let virt_hal = unsafe {VirtIOBlk::<VirtioHal>::new(&mut *(VIRTIO0 as *mut VirtIOHeader)).unwrap()};
        let mut condvars = Vec::with_capacity(16);
        for _ in 0..16{
            condvars.push(Condvar::new());
        }
        Self {
            virt_hal,
            condvars,
        }
    }
    pub fn nb_read(&mut self, block_id: usize, buf: &mut [u8], resp : &mut BlkResp) {
        let queue_id = unsafe {self.virt_hal.read_block_nb(block_id, buf, resp).unwrap()};
        self.condvars[queue_id as usize].wait_no_schedule();
    }
    pub fn nb_write(&mut self, block_id: usize, buf: &[u8], resp : &mut BlkResp) {
        let queue_id = unsafe { self.virt_hal.write_block_nb(block_id, buf, resp).unwrap() };
        self.condvars[queue_id as usize].wait_no_schedule();
    }
    pub fn work_done(&mut self) {
        while let Ok(queue_id) =  self.virt_hal.pop_used(){
            self.condvars[queue_id as usize].signal_one();
        }
    }
}

pub struct VirtIOBlock {
    virtio_blk: OneCoreCell<MTVirtBlk>,
}

impl BlockDevice for VirtIOBlock {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        if get_file_block_mode() {
            let mut int_ctrl = InterruptMask::new();
            int_ctrl.mask_interrupt();
            let mut block_device = self.virtio_blk.exclusive_access();
            let mut resp = BlkResp::default();
            block_device.nb_read(block_id, buf, &mut resp);
            drop(block_device);
            //TODO: we must disable INTERRUPT until sechule, if open it at this pointer,
            //we migth recive irq frist before we went to sleep, which will cause wakeup-lost
            //problem, then thread went to sleep amd never wake up
            block_task_and_run_next();
            int_ctrl.unmask_interrupt();
            assert_eq!(
                resp.status(),
                RespStatus::Ok,
                "Error when reading VirtIOBlk"
            );
        }else{
            self.virtio_blk.exclusive_access().virt_hal.read_block(block_id, buf).expect("Error when reading VirtIOBlk");
        }
    }
    fn write_block(&self, block_id: usize, buf: &[u8]) {
        if get_file_block_mode() {
            let mut int_ctrl = InterruptMask::new();
            int_ctrl.mask_interrupt();
            let mut block_device = self.virtio_blk.exclusive_access();
            let mut resp = BlkResp::default();
            block_device.nb_write(block_id, buf, &mut resp);
            drop(block_device);
            block_task_and_run_next();
            int_ctrl.unmask_interrupt();
            assert_eq!(
                resp.status(),
                RespStatus::Ok,
                "Error when writing VirtIOBlk"
            );
        } else {
            self.virtio_blk.exclusive_access().virt_hal.write_block(block_id, buf).expect("Error when writing VirtIOBlk");
        }
    }
    fn handle_irq(&self) {
        if get_file_block_mode() {
            self.virtio_blk.exclusive_access().work_done();
        }
    }
}

impl VirtIOBlock {
    #[allow(unused)]
    pub fn new() -> Self {
        Self { 
            virtio_blk: unsafe {OneCoreCell::new(MTVirtBlk::new())},
        }
    }
}