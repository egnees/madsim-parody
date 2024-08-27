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
    pub fn with_ip(ip_addr: impl ToIpAddr) -> io::Result<Self> {
        ip_addr.to_ip_addr().and_then(|ip| {
            if ip.is_loopback() || ip.is_multicast() || ip.is_unspecified() {
                Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "loopback, multicast and unspecified IP not supported",
                ))
            } else {
                Ok(Self {
                    ip,
                    udp_send_buffer_size: Node::UDP_SEND_BUF_SIZE,
                    udp_recv_buffer_size: Node::UDP_RECV_BUF_SIZE,
                })
            }
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

#[cfg(test)]
mod tests {
    use crate::sim::Sim;

    use super::NodeBuilder;

    #[test]
    fn bad_ip() {
        assert!(NodeBuilder::with_ip("aba").is_err());
        assert!(NodeBuilder::with_ip("127.0.0.1").is_err());
        assert!(NodeBuilder::with_ip("224.255.0.1").is_err());
        assert!(NodeBuilder::with_ip("0.0.0.0").is_err());
    }

    #[test]
    fn nodes_with_equal_ips() {
        let mut sim = Sim::new(123);
        let ip = "10.12.1.1";
        NodeBuilder::with_ip(ip).unwrap().build(&mut sim).unwrap();
        assert!(NodeBuilder::with_ip(ip).unwrap().build(&mut sim).is_none());
    }
}
