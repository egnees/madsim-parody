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
}

impl UdpSocket {
    pub fn bind(addr: impl ToSocketAddrs) -> io::Result<Self> {
        let node = NodeHandle::current();
        let info = node.info();
        let net = node.network_handle();

        for addr in addr.to_socket_addrs()? {
            if addr.ip() != node.ip() {
                continue;
            }
            let socket = Rc::new(RefCell::new(UpdSocketData {
                recv_buf: Buffer::with_capacity(info.udp_recv_buffer_size),
                recv_waiters: Vec::new(),
                local_addr: addr,
            }));
            if let Ok(()) = net.register_upd_socket(socket.clone()) {
                return Ok(Self { data: socket });
            }
        }

        Err(io::Error::new(
            io::ErrorKind::AddrInUse,
            "address already is use or no address provided",
        ))
    }

    pub fn send_to(&self, buf: &[u8], target: impl ToSocketAddrs) -> io::Result<usize> {
        let mut target = target.to_socket_addrs()?;
        let Some(target) = target.next() else {
            return Err(io::Error::new(
                io::ErrorKind::AddrNotAvailable,
                "address is not available",
            ));
        };
        let node = NodeHandle::current();
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
        NodeHandle::current()
            .network_handle()
            .deregister_socket(self.local_addr());
    }
}

unsafe impl Send for UdpSocket {}
unsafe impl Sync for UdpSocket {}

////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use std::{net::SocketAddr, rc::Rc, sync::atomic::AtomicBool};

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
        let node1 = NodeBuilder::from_ip_addr(ip1)
            .unwrap()
            .build(&mut sim)
            .unwrap();
        let node2 = NodeBuilder::from_ip_addr(ip2)
            .unwrap()
            .build(&mut sim)
            .unwrap();
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

    #[test]
    fn looped() {
        let mut sim = Sim::new(123);
        let node = NodeBuilder::from_ip_addr("10.12.1.1")
            .unwrap()
            .build(&mut sim)
            .unwrap();
        let flag = Rc::new(AtomicBool::new(false));
        node.spawn({
            let flag = flag.clone();
            async move {
                let socket = UdpSocket::bind("10.12.1.1:80").unwrap();
                socket.send_to(b"hello", "10.12.1.1:80").unwrap();
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
        let node = NodeBuilder::from_ip_addr("10.12.1.1")
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
        let node = NodeBuilder::from_ip_addr("10.12.1.2")
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
}
