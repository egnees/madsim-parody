////////////////////////////////////////////////////////////////////////////////

use std::{
    io::{self},
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
};

////////////////////////////////////////////////////////////////////////////////

pub trait ToIpAddr {
    fn to_ip_addr(&self) -> io::Result<IpAddr>;
}

////////////////////////////////////////////////////////////////////////////////

impl ToIpAddr for IpAddr {
    fn to_ip_addr(&self) -> io::Result<IpAddr> {
        Ok(*self)
    }
}

impl ToIpAddr for Ipv4Addr {
    fn to_ip_addr(&self) -> io::Result<IpAddr> {
        IpAddr::V4(*self).to_ip_addr()
    }
}

impl ToIpAddr for Ipv6Addr {
    fn to_ip_addr(&self) -> io::Result<IpAddr> {
        IpAddr::V6(*self).to_ip_addr()
    }
}

////////////////////////////////////////////////////////////////////////////////

impl ToIpAddr for &str {
    fn to_ip_addr(&self) -> io::Result<IpAddr> {
        self.parse::<IpAddr>()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "DNS is not supported"))
    }
}

impl ToIpAddr for String {
    fn to_ip_addr(&self) -> io::Result<IpAddr> {
        self.as_str().to_ip_addr()
    }
}

////////////////////////////////////////////////////////////////////////////////

impl<T: ToIpAddr> ToIpAddr for &T {
    fn to_ip_addr(&self) -> io::Result<IpAddr> {
        (**self).to_ip_addr()
    }
}

////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    use crate::net::ip_addr::ToIpAddr;

    ////////////////////////////////////////////////////////////////////////////////

    #[test]
    fn from_ip_addr() {
        let ipv4 = Ipv4Addr::new(127, 0, 0, 1);
        let ip_ref = IpAddr::V4(ipv4);
        assert_eq!(ip_ref.to_ip_addr().unwrap(), ip_ref);
        assert_eq!(ipv4.to_ip_addr().unwrap(), ip_ref);
        let ipv6 = "::1".parse::<Ipv6Addr>().unwrap();
        let ipv6_ref = IpAddr::V6(ipv6);
        assert_eq!(ipv6.to_ip_addr().unwrap(), ipv6_ref);
    }

    ////////////////////////////////////////////////////////////////////////////////

    #[test]
    fn from_str() {
        let ipv4 = "127.0.0.1".parse::<Ipv4Addr>().unwrap();
        let ip_ref = IpAddr::V4(ipv4);
        assert_eq!("127.0.0.1".to_ip_addr().unwrap(), ip_ref);
        assert_eq!("127.0.0.1".to_string().to_ip_addr().unwrap(), ip_ref);

        let ipv6 = "::1".parse::<Ipv6Addr>().unwrap();
        let ipv6_ref = IpAddr::V6(ipv6);
        assert_eq!("::1".to_ip_addr().unwrap(), ipv6_ref);
        assert_eq!("::1".to_string().to_ip_addr().unwrap(), ipv6_ref);
    }

    ////////////////////////////////////////////////////////////////////////////////

    #[test]
    fn from_ref() {
        assert_eq!(
            (&&"198.162.0.2").to_ip_addr().unwrap(),
            "198.162.0.2".parse::<Ipv4Addr>().unwrap()
        );
    }
}
