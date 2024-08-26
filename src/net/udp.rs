use crate::sim;

pub enum UpdSocket {
    Virtual(sim::UdpSocket),
}
