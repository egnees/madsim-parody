use std::{
    io::{self, ErrorKind},
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    option, vec,
};

////////////////////////////////////////////////////////////////////////////////

pub trait ToSocketAddrs {
    type Iter: Iterator<Item = SocketAddr>;

    fn to_socket_addrs(&self) -> io::Result<Self::Iter>;
}

////////////////////////////////////////////////////////////////////////////////

impl ToSocketAddrs for std::net::SocketAddr {
    type Iter = option::IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> io::Result<Self::Iter> {
        Ok(Some(*self).into_iter())
    }
}

impl ToSocketAddrs for std::net::SocketAddrV4 {
    type Iter = option::IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> io::Result<Self::Iter> {
        std::net::SocketAddr::V4(*self).to_socket_addrs()
    }
}

impl ToSocketAddrs for std::net::SocketAddrV6 {
    type Iter = option::IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> io::Result<Self::Iter> {
        std::net::SocketAddr::V6(*self).to_socket_addrs()
    }
}

////////////////////////////////////////////////////////////////////////////////

impl ToSocketAddrs for (IpAddr, u16) {
    type Iter = option::IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> io::Result<Self::Iter> {
        std::net::SocketAddr::new(self.0, self.1).to_socket_addrs()
    }
}

impl ToSocketAddrs for (Ipv4Addr, u16) {
    type Iter = option::IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> io::Result<Self::Iter> {
        std::net::SocketAddr::new(IpAddr::V4(self.0), self.1).to_socket_addrs()
    }
}

impl ToSocketAddrs for (Ipv6Addr, u16) {
    type Iter = option::IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> io::Result<Self::Iter> {
        std::net::SocketAddr::new(IpAddr::V6(self.0), self.1).to_socket_addrs()
    }
}

////////////////////////////////////////////////////////////////////////////////

impl ToSocketAddrs for (&str, u16) {
    type Iter = vec::IntoIter<std::net::SocketAddr>;
    fn to_socket_addrs(&self) -> io::Result<Self::Iter> {
        let (host, port) = *self;

        // try to parse the host as a regular IP address first
        if let Ok(addr) = host.parse::<Ipv4Addr>() {
            let addr = SocketAddrV4::new(addr, port);
            return Ok(vec![SocketAddr::V4(addr)].into_iter());
        }
        if let Ok(addr) = host.parse::<Ipv6Addr>() {
            let addr = SocketAddrV6::new(addr, port, 0, 0);
            return Ok(vec![SocketAddr::V6(addr)].into_iter());
        }

        // should try DNS here,
        // but it is not supported for now
        Err(io::Error::new(
            ErrorKind::InvalidInput,
            "DNS is not supported",
        ))
    }
}

impl ToSocketAddrs for (String, u16) {
    type Iter = vec::IntoIter<std::net::SocketAddr>;
    fn to_socket_addrs(&self) -> io::Result<Self::Iter> {
        (&*self.0, self.1).to_socket_addrs()
    }
}

// accepts strings like "localhost:123"
impl ToSocketAddrs for str {
    type Iter = vec::IntoIter<SocketAddr>;
    fn to_socket_addrs(&self) -> io::Result<vec::IntoIter<SocketAddr>> {
        // try to parse as a regular SocketAddr first
        match self.parse() {
            Ok(addr) => Ok(vec![addr].into_iter()),
            Err(..) => {
                // should try DNS here,
                // but it is not supported for now
                Err(io::Error::new(
                    ErrorKind::InvalidInput,
                    "DNS is not supported",
                ))
            }
        }
    }
}

impl ToSocketAddrs for String {
    type Iter = vec::IntoIter<SocketAddr>;
    fn to_socket_addrs(&self) -> io::Result<vec::IntoIter<SocketAddr>> {
        self.as_str().to_socket_addrs()
    }
}

////////////////////////////////////////////////////////////////////////////////

impl<'a> ToSocketAddrs for &'a [SocketAddr] {
    type Iter = vec::IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> io::Result<Self::Iter> {
        Ok(vec::Vec::from_iter(self.iter().cloned()).into_iter())
    }
}

////////////////////////////////////////////////////////////////////////////////

impl<T: ToSocketAddrs + ?Sized> ToSocketAddrs for &T {
    type Iter = T::Iter;
    fn to_socket_addrs(&self) -> io::Result<T::Iter> {
        (**self).to_socket_addrs()
    }
}

////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use std::{
        io,
        net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
        str::FromStr,
    };

    use super::ToSocketAddrs;

    #[test]
    fn from_socket_addr_and_v4() {
        let ref_addr = SocketAddr::new([127, 0, 0, 1].into(), 8080);

        {
            let got_addr = SocketAddr::from_str("127.0.0.1:8080")
                .unwrap()
                .to_socket_addrs()
                .unwrap()
                .next()
                .unwrap();
            assert_eq!(got_addr, ref_addr);
        }

        {
            let got_addr = SocketAddrV4::from_str("127.0.0.1:8080")
                .unwrap()
                .to_socket_addrs()
                .unwrap()
                .next()
                .unwrap();
            assert_eq!(got_addr, ref_addr);
        }
    }

    #[test]
    fn from_socket_addr_v6() {
        let ref_addr: SocketAddrV6 = "[::1]:8080".parse().unwrap();
        let got_addr = ref_addr.to_socket_addrs().unwrap().next().unwrap();
        assert_eq!(SocketAddr::V6(ref_addr), got_addr);
    }

    #[test]
    fn from_ip_addr() {
        let addr_ref: IpAddr = "10.13.1.1".parse().unwrap();
        let port_ref: u16 = 12345;
        let got_addr = (addr_ref, port_ref)
            .to_socket_addrs()
            .unwrap()
            .next()
            .unwrap();
        assert_eq!(addr_ref, got_addr.ip());
        assert_eq!(port_ref, got_addr.port());
    }

    #[test]
    fn from_ip_addr_v4() {
        let addr_ref: Ipv4Addr = "192.168.2.2".parse().unwrap();
        let port_ref: u16 = 54321;
        let got_addr = (addr_ref, port_ref)
            .to_socket_addrs()
            .unwrap()
            .next()
            .unwrap();
        assert_eq!(addr_ref, got_addr.ip());
        assert_eq!(port_ref, got_addr.port());
    }

    #[test]
    fn from_ip_addr_v6() {
        let addr_ref: Ipv6Addr = "2001:0db8:85a3:0000:0000:8a2e:0370:7334".parse().unwrap();
        let port_ref: u16 = 54321;
        let got_addr = (addr_ref, port_ref)
            .to_socket_addrs()
            .unwrap()
            .next()
            .unwrap();
        assert_eq!(addr_ref, got_addr.ip());
        assert_eq!(port_ref, got_addr.port());
    }

    #[test]
    fn from_str() {
        let port_ref = 123;
        let addr_ref = SocketAddr::new("1.1.1.1".parse().unwrap(), port_ref);

        assert_eq!(
            "1.1.1.1:123".to_socket_addrs().unwrap().next().unwrap(),
            addr_ref
        );

        assert_eq!(
            "1.1.1.1:123"
                .to_string()
                .to_socket_addrs()
                .unwrap()
                .next()
                .unwrap(),
            addr_ref
        );

        assert_eq!(
            ("1.1.1.1", 123).to_socket_addrs().unwrap().next().unwrap(),
            addr_ref
        );

        assert_eq!(
            ("1.1.1.1".to_string(), 123)
                .to_socket_addrs()
                .unwrap()
                .next()
                .unwrap(),
            addr_ref
        );
    }

    #[test]
    fn from_slice() {
        let ref_addrs = [
            SocketAddr::from_str("1.1.1.1:1024").unwrap(),
            SocketAddr::from_str("1.1.1.2:1025").unwrap(),
        ];

        let mut got_addrs = ref_addrs.as_slice().to_socket_addrs().unwrap();
        let first = got_addrs.next().unwrap();
        let second = got_addrs.next().unwrap();

        assert_eq!(first, ref_addrs[0]);
        assert_eq!(second, ref_addrs[1]);
    }

    #[test]
    fn from_refs() {
        let ref_addr = SocketAddr::from_str("1.1.1.1:123").unwrap();
        assert_eq!(
            (&(&ref_addr)).to_socket_addrs().unwrap().next().unwrap(),
            ref_addr
        );
    }

    #[test]
    fn dns_not_supported_handled() {
        let result = "localhost:123".to_socket_addrs();
        let err = result.err().unwrap();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }
}
