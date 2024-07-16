pub mod block;
pub mod bus;
pub mod plic;
pub mod chardev;
pub mod gpu;
pub mod input;
pub mod net;

pub use block::BLOCK_DEVICE;
pub use gpu::GPU_DEVICE;
pub use input::KEYBOARD_DEVICE;
pub use input::MOUSE_DEVICE;
pub use net::NET_DEVICE;
pub use bus::virtio::VirtioHal;
pub use bus::virtio::init_dma_allocator;

//this is for QEMU devices
pub type BlockDeviceImpl = crate::drivers::block::VirtIOBlock;