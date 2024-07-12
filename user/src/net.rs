use super::*;

pub fn connect(ip: u32, sport: u16, dport: u16) -> isize {
    syscall_connect(ip, sport, dport)
}

pub fn listen(sport: u16) -> isize {
    syscall_listen(sport)
}

pub fn accept(socket_fd: usize) -> isize {
    syscall_accept(socket_fd)
}