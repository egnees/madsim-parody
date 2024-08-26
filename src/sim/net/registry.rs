use std::{cell::RefCell, collections::HashMap, net::SocketAddr, rc::Weak};

use super::udp::UpdSocketData;

////////////////////////////////////////////////////////////////////////////////

pub enum SocketData {
    Udp(Weak<RefCell<UpdSocketData>>),
    #[allow(unused)]
    Tcp(),
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct SocketRegistry(pub HashMap<SocketAddr, SocketData>);
