use std::{cmp::Ordering, net::SocketAddr};

use crate::time::Timestamp;

////////////////////////////////////////////////////////////////////////////////

pub struct NetworkEvent {
    pub timestamp: Timestamp,
    pub sender: SocketAddr,
    pub receiver: SocketAddr,
    pub data: Vec<u8>,
}

impl PartialEq for NetworkEvent {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp == other.timestamp
    }
}

impl Eq for NetworkEvent {}

impl Ord for NetworkEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        other.timestamp.cmp(&self.timestamp)
    }
}

impl PartialOrd for NetworkEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
