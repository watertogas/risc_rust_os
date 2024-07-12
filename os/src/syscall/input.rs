use crate::drivers::{KEYBOARD_DEVICE, MOUSE_DEVICE};
use crate::drivers::chardev::UART;

pub fn syscall_event_get() -> isize {
    let kb = KEYBOARD_DEVICE.clone();
    let mouse = MOUSE_DEVICE.clone();
    if !kb.is_empty() {
        kb.read_event() as isize
    } else if !mouse.is_empty() {
        mouse.read_event() as isize
    } else {
        0
    }
}

//if key is pressed, uart will recived a signle char
pub fn syscall_key_pressed() -> isize {
    let res = !UART.read_buffer_is_empty();
    if res {
        1
    } else {
        0
    }
}