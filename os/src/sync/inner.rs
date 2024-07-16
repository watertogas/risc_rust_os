//! Uniprocessor interior mutability primitives
use core::cell::{RefCell, RefMut};
use riscv::register::sstatus;
use core::sync::atomic::compiler_fence;
use core::sync::atomic::Ordering;

/// Wrap a static data structure inside it so that we are
/// able to access it without any `unsafe`.
///
/// We should only use it in single processor
///
/// In order to get mutable reference of inner data, call
/// `exclusive_access`.
pub struct OneCoreCell<T> {
    /// inner data
    inner: RefCell<T>,
}

unsafe impl<T> Sync for OneCoreCell<T> {}

impl<T> OneCoreCell<T> {
    /// User is responsible to guarantee that inner struct is only used in
    /// uniprocessor.
    pub unsafe fn new(value: T) -> Self {
        Self {
            inner: RefCell::new(value),
        }
    }
    /// Exclusive access inner data in UPSafeCell. Panic if the data has been borrowed.
    pub fn exclusive_access(&self) -> RefMut<'_, T> {
        self.inner.borrow_mut()
    }
}

//dymainc mask inter
pub struct InterruptMask{
    pub int_on : bool,
}
impl InterruptMask {
    pub fn new() -> Self {
        Self {
            int_on: false,
        }
    }
    pub fn mask_interrupt(&mut self){
        let sie = sstatus::read().sie();
        if sie {
            unsafe {
                sstatus::clear_sie();
            }
            compiler_fence(Ordering::SeqCst);
            self.int_on = true;
        } else {
            self.int_on = false;
        }
    }
    pub fn unmask_interrupt(&mut self){
        if self.int_on {
            unsafe {
                sstatus::set_sie();
            }
        }
    }
}
