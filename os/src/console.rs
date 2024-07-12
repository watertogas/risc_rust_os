use crate::sbi::console_putchar;
use core::fmt::{self, Write};
use crate::drivers::chardev::{CharDevice, UART};
use crate::config::is_mmio_uart_ready;

struct Stdout;

impl fmt::Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if is_mmio_uart_ready() {
            for c in s.chars() {
                UART.write(c as u8);
            }
        } else {
            for c in s.chars() {
                console_putchar(c as usize);
            }
        }
        Ok(())
    }
}

pub fn print(args: fmt::Arguments) {
    Stdout.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!($fmt $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! println {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
    }
}