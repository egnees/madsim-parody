mod context;
mod net;
mod runtime;
mod time;

pub mod node;
pub mod spawn;

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::hash::RandomState;
use std::net::IpAddr;

use net::Network;
use node::Node;
use node::NodeHandle;

pub use net::NetworkHandle;
pub use net::UdpSocket;
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
            nodes: HashMap::with_hasher(RandomState::new()),
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

    pub fn make_steps(&self) -> usize {
        let mut was_step = true;
        let mut steps = 0;
        let mut nodes = self.nodes.keys().cloned().collect::<Vec<_>>();
        nodes.sort();
        while was_step {
            was_step = false;
            for node in nodes.iter() {
                let node = self.nodes.get(node).unwrap().handle();
                let node_steps = node.make_steps(None);
                if node_steps > 0 {
                    steps += node_steps;
                    was_step = true;
                }
            }
        }
        steps
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
