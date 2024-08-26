use std::net::IpAddr;

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone)]
pub struct NodeInfo {
    pub ip: IpAddr,
    pub udp_send_buffer_size: usize,
    pub udp_recv_buffer_size: usize,
}
