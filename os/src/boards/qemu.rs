use crate::drivers::plic::PLIC;
use crate::drivers::plic::IntrTargetPriority;
use crate::drivers::*;
use crate::drivers::chardev::{CharDevice, UART};
use crate::config::set_mmio_uart_ready;
use crate::println;

//qemu clock frequency; which means ticks in a second
pub const CLOCK_FREQ : usize = 12500000;
//avaliable heap(64MB):end of kernel ~ 0x83600000
pub const AVALIABLE_FRAMES_END : usize = 0x83600000;
//DMA area: 10MB: 0x83600000 ~ 0x84000000
//avaliable memrory(64MB):0x80000000 ~ 0x84000000
pub const AVALIABLE_MEMORY_END : usize = 0x84000000;
pub const VIRT_PLIC: usize = 0xC00_0000;
pub const VIRT_UART: usize = 0x1000_0000;
//memory-mapped input/output devices
pub const MMIO: &[(usize, usize)] = &[
    (0x0010_0000, 0x00_2000), // VIRT_TEST/RTC  in virt machine
    (0x2000000, 0x10000),   //did not know???
    (0xc000000, 0x210000), // VIRT_PLIC in virt machine
    (0x10000000, 0x9000),  // VIRT_UART0 with GPU  in virt machine
];

pub type CharDeviceImpl = crate::drivers::chardev::NS16550a<VIRT_UART>;

pub fn init_plic()
{
    use riscv::register::sie;
    let mut plic = unsafe { PLIC::new(VIRT_PLIC) };
    let hart_id: usize = 0;
    let supervisor = IntrTargetPriority::Supervisor;
    let machine = IntrTargetPriority::Machine;
    plic.set_threshold(hart_id, supervisor, 0);
    plic.set_threshold(hart_id, machine, 1);
    //irq nums: 5 keyboard, 6 mouse, 8 block, 10 uart
    //for intr_src_id in [5usize, 6, 8, 10] {
    for intr_src_id in [8, 10] {
        plic.enable(hart_id, supervisor, intr_src_id);
        plic.set_priority(intr_src_id, 1);
    }
    unsafe {
        sie::set_sext();
    }
}

pub fn init_qemu_devices()
{
    //init dma area
    init_dma_allocator();
    println!("init dma memory done");
    //init uart
    UART.init();
    set_mmio_uart_ready();
    println!("Testing we can print with ns16550a");
    //init gpu
    let _gpu = GPU_DEVICE.clone();
    println!("GPU init done");
    //init keyboard
    let _keybord = KEYBOARD_DEVICE.clone();
    println!("keybord init done");
    //init mouse
    let _mouse = MOUSE_DEVICE.clone();
    println!("mouse init done");
    //init plic and enable all interrupts
    init_plic();
}

pub fn irq_handler() {
    let mut plic = unsafe { PLIC::new(VIRT_PLIC) };
    let intr_src_id = plic.claim(0, IntrTargetPriority::Supervisor);
    match intr_src_id {
        5 => KEYBOARD_DEVICE.handle_irq(),
        6 => MOUSE_DEVICE.handle_irq(),
        8 => BLOCK_DEVICE.handle_irq(),
        10 => UART.handle_irq(),
        _ => panic!("unsupported IRQ {}", intr_src_id),
    }
    plic.complete(0, IntrTargetPriority::Supervisor, intr_src_id);
}