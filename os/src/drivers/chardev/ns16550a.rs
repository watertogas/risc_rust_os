///! Ref: https://www.lammertbies.nl/comm/info/serial-uart
///! Ref: ns16550a datasheet: https://datasheetspdf.com/pdf-file/605590/NationalSemiconductor/NS16550A/1
///! Ref: ns16450 datasheet: https://datasheetspdf.com/pdf-file/1311818/NationalSemiconductor/NS16450/1
use super::CharDevice;
use crate::sync::Condvar;
use alloc::collections::VecDeque;
use bitflags::*;
use volatile::{ReadOnly, Volatile, WriteOnly};
use crate::task::block_task_and_run_next;
use crate::sync::OneCoreCell;
use crate::sync::InterruptMask;

bitflags! {
    /// InterruptEnableRegister
    pub struct IER: u8 {
        const RX_AVAILABLE = 1 << 0;
        const TX_EMPTY = 1 << 1;
    }

    /// LineStatusRegister
    pub struct LSR: u8 {
        const DATA_AVAILABLE = 1 << 0;
        const THR_EMPTY = 1 << 5;
    }

    /// Model Control Register
    pub struct MCR: u8 {
        const DATA_TERMINAL_READY = 1 << 0;
        const REQUEST_TO_SEND = 1 << 1;
        const AUX_OUTPUT1 = 1 << 2;
        const AUX_OUTPUT2 = 1 << 3;
    }
}

#[repr(C)]
#[allow(dead_code)]
struct ReadWithoutDLAB {
    /// receiver buffer register
    pub rbr: ReadOnly<u8>,
    /// interrupt enable register
    pub ier: Volatile<IER>,
    /// interrupt identification register
    pub iir: ReadOnly<u8>,
    /// line control register
    pub lcr: Volatile<u8>,
    /// model control register
    pub mcr: Volatile<MCR>,
    /// line status register
    pub lsr: ReadOnly<LSR>,
    /// ignore MSR
    _padding1: ReadOnly<u8>,
    /// ignore SCR
    _padding2: ReadOnly<u8>,
}

#[repr(C)]
#[allow(dead_code)]
struct WriteWithoutDLAB {
    /// transmitter holding register
    pub thr: WriteOnly<u8>,
    /// interrupt enable register
    pub ier: Volatile<IER>,
    /// ignore FCR
    _padding0: ReadOnly<u8>,
    /// line control register
    pub lcr: Volatile<u8>,
    /// modem control register
    pub mcr: Volatile<MCR>,
    /// line status register
    pub lsr: ReadOnly<LSR>,
    /// ignore other registers
    _padding1: ReadOnly<u16>,
}

pub struct NS16550aRaw {
    base_addr: usize,
}

impl NS16550aRaw {
    fn read_end(&mut self) -> &mut ReadWithoutDLAB {
        unsafe { &mut *(self.base_addr as *mut ReadWithoutDLAB) }
    }

    fn write_end(&mut self) -> &mut WriteWithoutDLAB {
        unsafe { &mut *(self.base_addr as *mut WriteWithoutDLAB) }
    }

    pub fn new(base_addr: usize) -> Self {
        Self { base_addr }
    }

    pub fn init(&mut self) {
        let read_end = self.read_end();
        let mut mcr = MCR::empty();
        mcr |= MCR::DATA_TERMINAL_READY;
        mcr |= MCR::REQUEST_TO_SEND;
        mcr |= MCR::AUX_OUTPUT2;
        read_end.mcr.write(mcr);
        let ier = IER::RX_AVAILABLE;
        read_end.ier.write(ier);
    }

    pub fn read(&mut self) -> Option<u8> {
        let read_end = self.read_end();
        let lsr = read_end.lsr.read();
        if lsr.contains(LSR::DATA_AVAILABLE) {
            Some(read_end.rbr.read())
        } else {
            None
        }
    }

    pub fn write(&mut self, ch: u8) {
        let write_end = self.write_end();
        loop {
            if write_end.lsr.read().contains(LSR::THR_EMPTY) {
                write_end.thr.write(ch);
                break;
            }
        }
    }
}

struct NS16550aInner {
    ns16550a: NS16550aRaw,
    read_buffer: VecDeque<u8>,
    condvar : Condvar,
}

pub struct NS16550a<const BASE_ADDR: usize> {
    inner: OneCoreCell<NS16550aInner>,
}

impl<const BASE_ADDR: usize> NS16550a<BASE_ADDR> {
    pub fn new() -> Self {
        Self {
            inner: unsafe {
                OneCoreCell::new(
                    NS16550aInner {
                        ns16550a: NS16550aRaw::new(BASE_ADDR),
                        read_buffer: VecDeque::new(),
                        condvar : Condvar::new(),
                    }
                )
            },
        }
        //inner.ns16550a.init();
    }

    pub fn read_buffer_is_empty(&self) -> bool {
        let uart = self.inner.exclusive_access();
        uart.read_buffer.is_empty()
    }
}

impl<const BASE_ADDR: usize> CharDevice for NS16550a<BASE_ADDR> {
    fn init(&self) {
        let mut uart = self.inner.exclusive_access();
        uart.ns16550a.init();
    }

    fn read(&self) -> u8 {
        let mut int_ctrl = InterruptMask::new();
        loop {
            int_ctrl.mask_interrupt();
            let mut uart = self.inner.exclusive_access();
            if let Some(ch) = uart.read_buffer.pop_front() {
                drop(uart);
                int_ctrl.unmask_interrupt();
                return ch;
            } else {
                uart.condvar.wait_no_schedule();
                drop(uart);
                block_task_and_run_next();
                int_ctrl.unmask_interrupt();
            }
        }
    }
    fn write(&self, ch: u8) {
        let mut int_ctrl = InterruptMask::new();
        int_ctrl.mask_interrupt();
        self.inner.exclusive_access().ns16550a.write(ch);
        int_ctrl.unmask_interrupt();
    }
    fn handle_irq(&self) {
        let mut uart = self.inner.exclusive_access();
        let mut count = 0;
        loop {
            if let Some(ch) = uart.ns16550a.read() {
                count += 1;
                uart.read_buffer.push_back(ch);
            } else {
                break;
            }
        }
        if count > 0 {
            uart.condvar.signal_one();
        }
    }
}
