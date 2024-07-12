use super::*;
use bitflags::bitflags;
//This is for I/O devices
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::{RgbColor, Size};
use embedded_graphics::{draw_target::DrawTarget, prelude::OriginDimensions};
use virtio_input_decoder::Decoder;
pub use virtio_input_decoder::{DecodeType, Key, KeyType, Mouse};


bitflags! {
    pub struct OpenFlags: u32 {
        const RDONLY = 0;
        const WRONLY = 1 << 0;
        const RDWR = 1 << 1;
        const CREATE = 1 << 9;
        const TRUNC = 1 << 10;
    }
}

pub fn open(path: &str, flags: OpenFlags) -> isize {
    syscall_open(path, flags.bits)
}
pub fn close(fd: usize) -> isize {
    syscall_close(fd)
}

pub fn read(fd: usize, buf: &mut [u8]) -> isize {
    syscall_read(fd, buf)
}
pub fn write(fd: usize, buf: &[u8]) -> isize {
    syscall_write(fd, buf)
}

pub fn pipe(pipe_fd: &mut [usize]) -> isize {
    syscall_pipe(pipe_fd)
}

pub fn dup(fd: usize) -> isize {
    syscall_dup(fd)
}

//This is for I/O devices
pub const VIRTGPU_XRES: u32 = 1280;
pub const VIRTGPU_YRES: u32 = 800;
pub const VIRTGPU_LEN: usize = (VIRTGPU_XRES * VIRTGPU_YRES * 4) as usize;

pub fn framebuffer() -> isize {
    syscall_get_fb_addr()
}
pub fn framebuffer_flush() -> isize {
    syscall_framebuffer_flush()
}

pub struct Display {
    pub size: Size,
    pub fb: &'static mut [u8],
}

impl Display {
    pub fn new(size: Size) -> Self {
        let fb_ptr = framebuffer() as *mut u8;
        let fb = unsafe { core::slice::from_raw_parts_mut(fb_ptr, VIRTGPU_LEN as usize) };
        Self { size, fb }
    }
    pub fn framebuffer(&mut self) -> &mut [u8] {
        self.fb
    }
    pub fn paint_on_framebuffer(&mut self, p: impl FnOnce(&mut [u8]) -> ()) {
        p(self.framebuffer());
        framebuffer_flush();
    }
}

impl OriginDimensions for Display {
    fn size(&self) -> Size {
        self.size
    }
}

impl DrawTarget for Display {
    type Color = Rgb888;

    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        pixels.into_iter().for_each(|px| {
            let idx = (px.0.y * VIRTGPU_XRES as i32 + px.0.x) as usize * 4;
            if idx + 2 >= self.fb.len() {
                return;
            }
            self.fb[idx] = px.1.b();
            self.fb[idx + 1] = px.1.g();
            self.fb[idx + 2] = px.1.r();
        });
        framebuffer_flush();
        Ok(())
    }
}

pub fn event_get() -> Option<InputEvent> {
    let raw_value = syscall_event_get();
    if raw_value == 0 {
        None
    } else {
        Some((raw_value as u64).into())
    }
}

pub fn key_pressed() -> bool {
    if syscall_key_pressed() == 1 {
        true
    } else {
        false
    }
}

#[repr(C)]
pub struct InputEvent {
    pub event_type: u16,
    pub code: u16,
    pub value: u32,
}

impl From<u64> for InputEvent {
    fn from(mut v: u64) -> Self {
        let value = v as u32;
        v >>= 32;
        let code = v as u16;
        v >>= 16;
        let event_type = v as u16;
        Self {
            event_type,
            code,
            value,
        }
    }
}

impl InputEvent {
    pub fn decode(&self) -> Option<DecodeType> {
        Decoder::decode(
            self.event_type as usize,
            self.code as usize,
            self.value as usize,
        )
        .ok()
    }
}
