////////////////////////////////////////////////////////////////////////////////

use std::{io, net::IpAddr, rc::Rc};

use crate::{net::ip_addr::ToIpAddr, sim::Sim};

use super::{info::NodeInfo, Node, NodeHandle, NodeState};

pub struct NodeBuilder {
    ip: IpAddr,
    udp_send_buffer_size: usize,
    udp_recv_buffer_size: usize,
}

impl NodeBuilder {
    pub fn from_ip_addr(ip_addr: impl ToIpAddr) -> io::Result<Self> {
        ip_addr.to_ip_addr().map(|ip| Self {
            ip,
            udp_send_buffer_size: Node::UPD_SEND_BUF_SIZE,
            udp_recv_buffer_size: Node::UPD_RECV_BUF_SIZE,
        })
    }

    pub fn build(self, sim: &mut Sim) -> Option<NodeHandle> {
        let node = Node(Rc::new(NodeState::new(
            NodeInfo {
                ip: self.ip,
                udp_send_buffer_size: self.udp_send_buffer_size,
                udp_recv_buffer_size: self.udp_recv_buffer_size,
            },
            sim.network(),
        )));

        sim.add_node(node)
    }

    pub fn udp_send_buffer_size(mut self, size: usize) -> Self {
        self.udp_send_buffer_size = size;
        self
    }

    pub fn udp_recv_buffer_size(mut self, size: usize) -> Self {
        self.udp_recv_buffer_size = size;
        self
    }
}
