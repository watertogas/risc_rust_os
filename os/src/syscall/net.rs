use crate::net::port_table::{accept, listen, port_acceptable, PortFd};
use crate::net::udp::UDP;
use crate::net::{net_interrupt_handler, IPv4};
use alloc::sync::Arc;
use crate::task::process::set_new_fd;
use crate::task::process::get_syscall_return_value;
use crate::task::schedule::get_current_task;

// just support udp
pub fn syscall_connect(raddr: u32, lport: u16, rport: u16) -> isize {
    let udp_node = UDP::new(IPv4::from_u32(raddr), lport, rport);
    let pid =  get_current_task().to_pid();
    let fd = set_new_fd(pid, Arc::new(udp_node));
    fd as isize
}

// listen a port
pub fn syscall_listen(port: u16) -> isize {
    match listen(port) {
        Some(port_index) => {
            let pid =  get_current_task().to_pid();
            set_new_fd(pid, Arc::new(PortFd::new(port_index)));
            // NOTICE: this return the port index, not the fd
            port_index as isize
        }
        None => -1,
    }
}

// accept a tcp connection
pub fn syscall_accept(port_index: usize) -> isize {
    println!("accepting port {}", port_index);
    let cur_task = get_current_task();
    accept(port_index, cur_task);
    // block_current_and_run_next();

    // NOTICE: There does not have interrupt handler, just call it munually.
    loop {
        net_interrupt_handler();

        if !port_acceptable(port_index) {
            break;
        }
    }
    get_syscall_return_value(cur_task)
}