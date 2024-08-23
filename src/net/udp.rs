////////////////////////////////////////////////////

pub(crate) mod sealed {
    use std::io;

    use crate::net::socket_addr::ToSocketAddrs;

    #[allow(unused)]
    pub(crate) trait UdpSocket: Sized {
        async fn bind(&self, addr: impl ToSocketAddrs) -> io::Result<Self>;

        fn local_addr(&self) -> io::Result<std::net::SocketAddr>;

        async fn send_to(&self, buf: &[u8], addr: impl ToSocketAddrs) -> io::Result<usize>;

        async fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, std::net::SocketAddr)>;
    }
}

////////////////////////////////////////////////////

pub enum UpdSocket {}
