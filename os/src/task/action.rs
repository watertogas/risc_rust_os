use crate::task::signal::SignalFlags;
use crate::task::signal::MAX_SIGNAL_NUM;


#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SignalAction {
    pub handler: usize,
    pub mask: SignalFlags,
}

impl Default for SignalAction {
    fn default() -> Self {
        Self {
            handler: 0,
            mask: SignalFlags::empty(),
        }
    }
}

#[derive(Clone)]
pub struct SignalHandler {
    pub existed_signals : SignalFlags,
    pub global_mask : SignalFlags,
    pub action_table: [SignalAction; MAX_SIGNAL_NUM],
    pub cur_signum : isize,
}

impl SignalHandler {
    pub fn new() -> Self {
        Self {
            existed_signals : SignalFlags::empty(),
            global_mask : SignalFlags::empty(),
            action_table : [SignalAction::default(); MAX_SIGNAL_NUM],
            cur_signum : -1,
        }
    }
}