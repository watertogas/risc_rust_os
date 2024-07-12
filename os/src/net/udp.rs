use super::net_interrupt_handler;
use super::socket::{add_socket, pop_data, remove_socket};
use super::LOSE_NET_STACK;
use super::NET_DEVICE;
use crate::fs::File;
use alloc::vec;
use lose_net_stack::packets::udp::UDPPacket;
use lose_net_stack::IPv4;
use lose_net_stack::MacAddress;
use crate::mm::memory_set::UserBuffer;

pub struct UDP {
    pub target: IPv4,
    pub sport: u16,
    pub dport: u16,
    pub socket_index: usize,
}

impl UDP {
    pub fn new(target: IPv4, sport: u16, dport: u16) -> Self {
        let index = add_socket(target, sport, dport).expect("can't add socket");

        Self {
            target,
            sport,
            dport,
            socket_index: index,
        }
    }
}

impl File for UDP {
    fn readable(&self) -> bool {
        true
    }

    fn writable(&self) -> bool {
        true
    }

    fn read(&self, buf: &UserBuffer) -> usize {
        loop {
            if let Some(data) = pop_data(self.socket_index) {
                let data_len = data.len();
                buf.write_kernel_slice_to_user(data.as_ptr() as usize, data_len);
                return data_len;
            } else {
                net_interrupt_handler();
            }
        }
    }

    fn write(&self, buf: &UserBuffer) -> usize {
        let lose_net_stack = LOSE_NET_STACK.0.exclusive_access();

        let len = buf.len;
        let mut data = vec![0u8; len];
        buf.read_buff_to_kernel_slice(data.as_mut_ptr() as usize, len);

        let udp_packet = UDPPacket::new(
            lose_net_stack.ip,
            lose_net_stack.mac,
            self.sport,
            self.target,
            MacAddress::new([0xff, 0xff, 0xff, 0xff, 0xff, 0xff]),
            self.dport,
            len,
            data.as_ref(),
        );
        NET_DEVICE.transmit(&udp_packet.build_data());
        len
    }
}

impl Drop for UDP {
    fn drop(&mut self) {
        remove_socket(self.socket_index)
    }
}
