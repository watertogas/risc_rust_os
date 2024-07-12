use crate::drivers::bus::virtio::VirtioHal;
use crate::sync::{Condvar, OneCoreCell};
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use core::any::Any;
use virtio_drivers::{VirtIOHeader, VirtIOInput};
use crate::task::block_task_and_run_next;
use crate::sync::InterruptMask;

const VIRTIO5: usize = 0x10005000;
const VIRTIO6: usize = 0x10006000;

struct VirtIOInputInner {
    virtio_input: VirtIOInput<'static, VirtioHal>,
    events: VecDeque<u64>,
    condvar: Condvar,
}

struct VirtIOInputWrapper {
    inner: OneCoreCell<VirtIOInputInner>,
}

pub trait InputDevice: Send + Sync + Any {
    fn read_event(&self) -> u64;
    fn handle_irq(&self);
    fn is_empty(&self) -> bool;
}

lazy_static::lazy_static!(
    pub static ref KEYBOARD_DEVICE: Arc<dyn InputDevice> = Arc::new(VirtIOInputWrapper::new(VIRTIO5));
    pub static ref MOUSE_DEVICE: Arc<dyn InputDevice> = Arc::new(VirtIOInputWrapper::new(VIRTIO6));
);

impl VirtIOInputWrapper {
    pub fn new(addr: usize) -> Self {
        let inner = VirtIOInputInner {
            virtio_input: unsafe {
                VirtIOInput::<VirtioHal>::new(&mut *(addr as *mut VirtIOHeader)).unwrap()
            },
            events: VecDeque::new(),
            condvar: Condvar::new(),
        };
        Self {
            inner: unsafe { OneCoreCell::new(inner) },
        }
    }
}

impl InputDevice for VirtIOInputWrapper {
    fn is_empty(&self) -> bool {
        self.inner.exclusive_access().events.is_empty()
    }

    fn read_event(&self) -> u64 {
        let mut int_ctrl = InterruptMask::new();
        loop {
            int_ctrl.mask_interrupt();
            let mut input = self.inner.exclusive_access();
            if let Some(event) = input.events.pop_front() {
                drop(input);
                int_ctrl.unmask_interrupt();
                return event;
            } else {
                input.condvar.wait_no_schedule();
                drop(input);
                block_task_and_run_next();
                int_ctrl.unmask_interrupt();
            }
        }
    }

    fn handle_irq(&self) {
        let mut count = 0;
        let mut result;
        let mut input = self.inner.exclusive_access();
        input.virtio_input.ack_interrupt();
        while let Some(event) = input.virtio_input.pop_pending_event() {
            count += 1;
            result = (event.event_type as u64) << 48
                    | (event.code as u64) << 32
                    | (event.value) as u64;
            input.events.push_back(result);
        }
        if count > 0 {
            input.condvar.signal_one();
        };
    }
}
