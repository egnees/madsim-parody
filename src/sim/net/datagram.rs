use std::{collections::VecDeque, net::SocketAddr};

////////////////////////////////////////////////////////////////////////////////

#[derive(PartialEq, Eq, Debug)]
pub struct Datagram {
    pub from: SocketAddr,
    pub to: SocketAddr,
    pub data: Vec<u8>,
}

////////////////////////////////////////////////////////////////////////////////

pub struct Buffer {
    capacity: usize,
    len: usize,
    buf: VecDeque<Datagram>,
}

impl Buffer {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            capacity,
            len: 0,
            buf: Default::default(),
        }
    }

    pub fn add_datagram(&mut self, datagram: Datagram) -> bool {
        if datagram.data.len() + self.len <= self.capacity {
            self.len += datagram.data.len();
            self.buf.push_back(datagram);
            true
        } else {
            false
        }
    }

    pub fn take_datagram(&mut self) -> Option<Datagram> {
        let datagram = self.buf.pop_front()?;
        self.len -= datagram.data.len();
        Some(datagram)
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use super::{Buffer, Datagram};

    fn dgram(size: usize) -> Datagram {
        let addr = "1.1.1.1:8080".parse::<SocketAddr>().unwrap();
        Datagram {
            from: addr,
            to: addr,
            data: vec![0u8; size],
        }
    }

    #[test]
    fn basic() {
        let mut buffer = Buffer::with_capacity(10);

        assert!(buffer.add_datagram(dgram(1)));
        assert!(!buffer.add_datagram(dgram(10)));

        assert_eq!(buffer.take_datagram(), Some(dgram(1)));
        assert_eq!(buffer.take_datagram(), None);

        assert!(buffer.add_datagram(dgram(9)));
        assert!(!buffer.add_datagram(dgram(2)));
        assert!(buffer.add_datagram(dgram(1)));

        assert_eq!(buffer.take_datagram(), Some(dgram(9)));
        assert_eq!(buffer.take_datagram(), Some(dgram(1)));
        assert_eq!(buffer.take_datagram(), None);
    }
}
