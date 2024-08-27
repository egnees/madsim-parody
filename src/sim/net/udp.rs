use std::{
    cell::RefCell,
    future::poll_fn,
    io,
    net::SocketAddr,
    rc::Rc,
    task::{Poll, Waker},
};

use crate::{net::socket_addr::ToSocketAddrs, sim::node::NodeHandle};

use super::datagram::Buffer;

////////////////////////////////////////////////////////////////////////////////

pub struct UpdSocketData {
    pub recv_buf: Buffer,
    pub recv_waiters: Vec<Waker>,
    pub local_addr: SocketAddr,
}

////////////////////////////////////////////////////////////////////////////////

pub struct UdpSocket {
    data: Rc<RefCell<UpdSocketData>>,
    owner_node: NodeHandle,
}

impl UdpSocket {
    pub fn bind(addr: impl ToSocketAddrs) -> io::Result<Self> {
        let node = NodeHandle::current();
        let info = node.info();
        let net = node.network_handle();

        for mut addr in addr.to_socket_addrs()? {
            if addr.ip().is_multicast() {
                continue;
            }
            if addr.ip().is_unspecified() || addr.ip().is_loopback() {
                addr.set_ip(node.ip());
            }
            let port = if addr.port() == 0 {
                None
            } else {
                Some(addr.port())
            };
            if let Some(port) = node.take_port(port) {
                let addr = SocketAddr::new(addr.ip(), port);
                let socket = Rc::new(RefCell::new(UpdSocketData {
                    recv_buf: Buffer::with_capacity(info.udp_recv_buffer_size),
                    recv_waiters: Vec::new(),
                    local_addr: addr,
                }));
                if let Ok(()) = net.register_upd_socket(socket.clone()) {
                    return Ok(Self {
                        data: socket,
                        owner_node: node,
                    });
                }
            }
        }

        Err(io::Error::new(
            io::ErrorKind::AddrInUse,
            "address already is use or no address provided",
        ))
    }

    pub fn send_to(&self, buf: &[u8], target: impl ToSocketAddrs) -> io::Result<usize> {
        let mut target = target.to_socket_addrs()?;
        let Some(mut target) = target.next() else {
            return Err(io::Error::new(
                io::ErrorKind::AddrNotAvailable,
                "address is not available",
            ));
        };
        if target.ip().is_multicast() || target.ip().is_unspecified() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "multicast and unspecified IP not supported",
            ));
        }
        if target.ip().is_loopback() {
            target.set_ip(self.owner_node.ip());
        }
        let node = self.owner_node.clone();
        let info = node.info();
        let buf = &buf[..info.udp_send_buffer_size.min(buf.len())];
        node.network_handle()
            .send_upd_packet(self.local_addr(), target, buf);
        Ok(buf.len())
    }

    pub async fn recv_from(&self, buf: &mut [u8]) -> (usize, SocketAddr) {
        let data = Rc::downgrade(&self.data);
        poll_fn(move |cx| {
            let data = data.upgrade().unwrap();
            let mut state = data.borrow_mut();
            if let Some(dgram) = state.recv_buf.take_datagram() {
                let len = dgram.data.len().min(buf.len());
                buf[..len].copy_from_slice(&dgram.data[..len]);
                Poll::Ready((len, dgram.from))
            } else {
                state.recv_waiters.push(cx.waker().clone());
                Poll::Pending
            }
        })
        .await
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.data.borrow().local_addr
    }
}

impl Drop for UdpSocket {
    fn drop(&mut self) {
        // udp socket can be dropped outside of sim
        if self.owner_node.alive() {
            self.owner_node.return_port(self.local_addr().port());
            if self.owner_node.network_handle().alive() {
                self.owner_node
                    .network_handle()
                    .deregister_socket(self.local_addr());
            }
        }
    }
}

// UdpSocket must be used only within the simulation
unsafe impl Send for UdpSocket {}
unsafe impl Sync for UdpSocket {}

////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use std::{cell::RefCell, net::SocketAddr, rc::Rc, sync::atomic::AtomicBool};

    use crate::{
        net::socket_addr::ToSocketAddrs,
        sim::{node::NodeBuilder, Sim},
        time::Timestamp,
    };

    use super::UdpSocket;

    #[test]
    fn communication() {
        let mut sim = Sim::new(123);
        let ip1 = "10.12.1.1";
        let ip2 = "10.12.1.2";
        let socket1 = "10.12.1.1:123";
        let socket2 = "10.12.1.2:345";
        let node1 = NodeBuilder::with_ip(ip1).unwrap().build(&mut sim).unwrap();
        let node2 = NodeBuilder::with_ip(ip2).unwrap().build(&mut sim).unwrap();
        let flag = Rc::new(AtomicBool::new(false));
        node1.spawn({
            let flag = flag.clone();
            async move {
                let socket = UdpSocket::bind(socket1).unwrap();
                let mut buf = [0u8; 10];
                let (bytes, sender) = socket.recv_from(&mut buf).await;
                assert_eq!(bytes, 5);
                assert_eq!(sender, socket2.to_socket_addrs().unwrap().next().unwrap());
                assert_eq!(&buf[..bytes], b"hello");
                flag.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        });
        node2.spawn(async move {
            let socket = UdpSocket::bind(socket2).unwrap();
            let bytes = socket.send_to(b"hello", socket1).unwrap();
            assert_eq!(bytes, 5);
        });
        node1.make_steps(None);
        node2.make_steps(None);
        node1.make_steps(None);
        assert_eq!(flag.load(std::sync::atomic::Ordering::SeqCst), true);
    }

    #[test_case("10.12.1.1:80", "10.12.1.1:80")]
    #[test_case("127.0.0.1:80", "10.12.1.1:80")]
    #[test_case("127.0.0.1:80", "127.0.0.1:80")]
    #[test_case("10.12.1.1:80", "127.0.0.1:80")]
    fn looped(bind: &'static str, send_to: &'static str) {
        let mut sim = Sim::new(123);
        let node = NodeBuilder::with_ip("10.12.1.1")
            .unwrap()
            .build(&mut sim)
            .unwrap();
        let flag = Rc::new(AtomicBool::new(false));
        node.spawn({
            let flag = flag.clone();
            async move {
                let socket = UdpSocket::bind(bind).unwrap();
                socket.send_to(b"hello", send_to).unwrap();
                let mut buf = [0u8; 5];
                let (len, from) = socket.recv_from(&mut buf).await;
                assert_eq!(len, 5);
                assert_eq!(&buf, b"hello");
                assert_eq!(from, "10.12.1.1:80".parse::<SocketAddr>().unwrap());
                flag.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        });
        sim.make_steps();
        assert_eq!(flag.load(std::sync::atomic::Ordering::SeqCst), true);
        assert_eq!(node.time(), Timestamp::from_secs(0));
    }

    #[test]
    fn bind_on_same_addr() {
        let mut sim = Sim::new(123);
        let node = NodeBuilder::with_ip("10.12.1.1")
            .unwrap()
            .build(&mut sim)
            .unwrap();
        node.spawn(async {
            let _socket1 = UdpSocket::bind("10.12.1.1:80").unwrap();
            let try_bind2 = UdpSocket::bind("10.12.1.1:80");
            assert!(try_bind2.is_err());
        });
        let steps = sim.make_steps();
        assert!(steps >= 1);
    }

    #[test]
    fn bind_on_other_ip() {
        let mut sim = Sim::new(123);
        let node = NodeBuilder::with_ip("10.12.1.2")
            .unwrap()
            .build(&mut sim)
            .unwrap();
        node.spawn(async {
            let try_bind = UdpSocket::bind("10.12.1.1");
            assert!(try_bind.is_err());
        });
        let steps = sim.make_steps();
        assert!(steps >= 1);
    }

    #[test]
    fn auto_port_selection() {
        let mut sim = Sim::new(123);
        let node = NodeBuilder::with_ip("10.12.1.1")
            .unwrap()
            .build(&mut sim)
            .unwrap();
        node.spawn(async {
            let socket = UdpSocket::bind("10.12.1.1:0").unwrap();
            assert!(socket.local_addr().port() != 0);
        });
        let steps = sim.make_steps();
        assert!(steps >= 1);
        let ports = Rc::new(RefCell::new(Vec::<u16>::new()));
        let (sender, _recv) = tokio::sync::broadcast::channel::<bool>(1);
        for _ in 1..=(u16::MAX as usize) + 100 {
            node.spawn({
                let mut recv = sender.subscribe();
                let ports = ports.clone();
                async move {
                    let socket = UdpSocket::bind("10.12.1.1:0");
                    if let Ok(socket) = &socket {
                        ports.borrow_mut().push(socket.local_addr().port());
                    }
                    recv.recv().await.unwrap();
                }
            });
        }
        sim.make_steps();
        sender.send(true).unwrap();
        {
            let mut ports = ports.borrow_mut();
            ports.sort();
            assert_eq!(ports.len(), u16::MAX.into());
            for i in 0..ports.len() {
                assert_eq!(ports[i], (i + 1) as u16);
            }
        }
        sim.make_steps();
    }

    #[test]
    fn bind_on_zero() {
        let mut sim = Sim::new(123);
        let node = NodeBuilder::with_ip("10.12.1.1")
            .unwrap()
            .build(&mut sim)
            .unwrap();
        node.spawn({
            let node = node.clone();
            async move {
                let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
                assert!(socket.local_addr().port() != 0);
                assert_eq!(socket.local_addr().ip(), node.ip());

                let socket = UdpSocket::bind("0.0.0.0:123").unwrap();
                assert_eq!(socket.local_addr().port(), 123);
                assert_eq!(socket.local_addr().ip(), node.ip());
            }
        });
        sim.make_steps();
    }

    #[test]
    fn can_be_dropped_outside_sim() {
        let mut sim = Sim::new(123);
        let node = NodeBuilder::with_ip("10.12.1.1")
            .unwrap()
            .build(&mut sim)
            .unwrap();
        let (send, recv) = std::sync::mpsc::channel();
        node.spawn(async move {
            let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
            send.send(socket).unwrap();
        });
        sim.make_steps();
        let socket = recv.recv().unwrap();
        drop(socket);
    }

    #[test]
    fn multicast() {
        let mut sim = Sim::new(123);
        let node = NodeBuilder::with_ip("10.12.1.1")
            .unwrap()
            .build(&mut sim)
            .unwrap();
        node.spawn(async {
            let multicast = UdpSocket::bind("224.255.0.1");
            assert!(multicast.is_err());
        });
        sim.make_steps();
    }

    #[test]
    fn bad_send_to() {
        let mut sim = Sim::new(123);
        let node = NodeBuilder::with_ip("10.12.1.1")
            .unwrap()
            .build(&mut sim)
            .unwrap();
        node.spawn(async {
            let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
            assert!(socket.send_to(b"some message", "224.13.3.1:80").is_err());
            assert!(socket.send_to(b"some message", "0.0.0.0:80").is_err());
        });
        sim.make_steps();
    }
}
