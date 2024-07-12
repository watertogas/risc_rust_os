use crate::drivers::GPU_DEVICE;
use crate::task::process::map_framebuffer;

pub fn syscall_map_framebuffer() -> isize {
    let fb = GPU_DEVICE.get_framebuffer();
    let len = fb.len();
    map_framebuffer(fb.as_ptr() as usize, len)
}

pub fn syscall_framebuffer_flush() -> isize {
    GPU_DEVICE.flush();
    0
}