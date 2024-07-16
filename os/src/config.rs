pub const MAX_APP_NUM : usize = 4;
pub const USER_STACK_SIZE : usize = 2*4096;
pub const KERNEL_STACK_SIZE : usize = 2*4096;
//time interval(unit: ms) to trigger process schedule
pub const SCHEDUL_INTERVAL : usize = 10;
//system heap size, 10M should be enough?
pub const KERNEL_HEAP_SIZE : usize = 0xA00000;
//system page size, fixed to 4096(4K)
pub const KERNEL_PAGE_SIZE : usize = 4096;
pub const KERNEL_PAGE_WIDTH_BITS : usize = 12;

pub use crate::board::{CLOCK_FREQ, AVALIABLE_FRAMES_END, AVALIABLE_MEMORY_END, MMIO};

//Dynamic configs for ALL OS
pub struct DynamicConfigs{
    //non blocking file operations
    pub nb_file_ops : bool,
    //if MMIO UART is ready,
    pub uart_ready : bool,
}

static mut DYNAMIC_CFG : DynamicConfigs = DynamicConfigs {
    nb_file_ops : false,
    uart_ready : false,
};

pub fn get_file_block_mode()-> bool {
    unsafe {
        DYNAMIC_CFG.nb_file_ops
    }
}

pub fn set_file_non_blocking() {
    unsafe {
        DYNAMIC_CFG.nb_file_ops =  true;
    }
}

pub fn is_mmio_uart_ready()-> bool{
    unsafe {
        DYNAMIC_CFG.uart_ready
    }
}

pub fn set_mmio_uart_ready(){
    unsafe {
        DYNAMIC_CFG.uart_ready =  true;
    }
}