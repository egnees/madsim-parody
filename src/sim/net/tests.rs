use std::net::SocketAddr;

use crate::sim::{node::NodeBuilder, Sim};

use super::UdpSocket;

#[test]
fn network_split_udp() {
    let mut sim = Sim::new(321);
    let node1 = NodeBuilder::with_ip("10.12.1.1")
        .unwrap()
        .build(&mut sim)
        .unwrap();
    let node2 = NodeBuilder::with_ip("10.13.1.1")
        .unwrap()
        .build(&mut sim)
        .unwrap();
    sim.network().separate(&[node1.ip()]);
    node1.spawn(async {
        let socket = UdpSocket::bind("0.0.0.0:123").unwrap();
        let mut buf = [0u8; 10];
        socket.recv_from(&mut buf).await;
        unreachable!("received message from node2")
    });
    node2.spawn({
        let node1 = node1.clone();
        async move {
            let socket = UdpSocket::bind("0.0.0.0:123").unwrap();
            for _ in 0..1000 {
                socket
                    .send_to(b"hello", SocketAddr::new(node1.ip(), 123))
                    .unwrap();
            }
        }
    });
    node1.make_steps(None);
    node2.make_steps(None);
    node1.make_steps(None);
}
