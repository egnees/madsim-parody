mod context;
mod net;
mod runtime;
mod time;

pub mod node;
pub mod spawn;

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::net::IpAddr;

use net::Network;
pub use net::NetworkHandle;
pub use net::UdpSocket;
use node::Node;
use node::NodeHandle;

pub use spawn::spawn;
pub use time::now;
pub use time::sleep;

use crate::net::ip_addr::ToIpAddr;

////////////////////////////////////////////////////////////////////////////////

pub fn in_sim() -> bool {
    NodeHandle::exists()
}

////////////////////////////////////////////////////////////////////////////////

pub struct Sim {
    nodes: HashMap<IpAddr, Node>,
    network: Network,
}

impl Sim {
    pub fn new(seed: u64) -> Self {
        Self {
            nodes: Default::default(),
            network: Network::new(seed),
        }
    }

    pub fn node(&self, addr: impl ToIpAddr) -> Option<NodeHandle> {
        self.nodes
            .get(&addr.to_ip_addr().unwrap())
            .map(|node| node.handle())
    }

    pub fn network(&self) -> NetworkHandle {
        self.network.handle()
    }

    ////////////////////////////////////////////////////////////////////////////////

    fn add_node(&mut self, node: Node) -> Option<NodeHandle> {
        let ip = node.handle().ip();
        if let Entry::Vacant(e) = self.nodes.entry(ip) {
            let handle = node.handle();
            e.insert(node);
            self.network().register_node(ip);
            Some(handle)
        } else {
            None
        }
    }
}
