use std::{collections::HashSet, net::IpAddr};

use crate::net::ip_addr::ToIpAddr;

#[derive(Default)]
pub(crate) struct NetworkTopology {
    links: HashSet<(IpAddr, IpAddr)>,
    nodes: HashSet<IpAddr>,
}

impl NetworkTopology {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn register_node(&mut self, addr: impl ToIpAddr) {
        let addr = addr.to_ip_addr().unwrap();
        self.nodes.insert(addr);
        for other in self.nodes.iter() {
            self.links.insert((addr, *other));
            self.links.insert((*other, addr));
        }
    }

    pub fn separate<A: ToIpAddr>(&mut self, group: &[A]) {
        let mut sep_nodes = group
            .iter()
            .map(|a| a.to_ip_addr().unwrap())
            .collect::<Vec<_>>();
        sep_nodes.sort();
        for sep_node in sep_nodes.iter() {
            if !self.nodes.contains(sep_node) {
                panic!("node '{}' is not registered", sep_node);
            }
            for other in self.nodes.iter() {
                if sep_nodes.binary_search(other).is_err() {
                    self.links.remove(&(*sep_node, *other));
                    self.links.remove(&(*other, *sep_node));
                }
            }
        }
    }

    pub fn repair<A: ToIpAddr>(&mut self, group: &[A]) {
        for a in group.iter().map(|a| a.to_ip_addr().unwrap()) {
            for b in group.iter().map(|a| a.to_ip_addr().unwrap()) {
                self.links.insert((a, b));
            }
        }
    }

    pub fn repair_all(&mut self) {
        for a in self.nodes.iter() {
            for b in self.nodes.iter() {
                self.links.insert((*a, *b));
            }
        }
    }

    pub fn node_registered(&self, addr: impl ToIpAddr) -> bool {
        self.nodes.contains(&addr.to_ip_addr().unwrap())
    }

    pub fn hops(&self, from: impl ToIpAddr, to: impl ToIpAddr) -> Option<usize> {
        let from = from.to_ip_addr().unwrap();
        let to = to.to_ip_addr().unwrap();
        if !self.node_registered(from) || !self.node_registered(to) {
            None
        } else if from == to {
            Some(0)
        } else if self.links.contains(&(from, to)) {
            Some(1)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::NetworkTopology;

    #[test]
    fn works() {
        let mut topology = NetworkTopology::new();

        let first = "192.168.1.2";
        let second = "192.168.1.3";
        let third = "10.133.14.2";

        topology.register_node(first);
        assert!(topology.node_registered(first));
        assert_eq!(topology.hops(first, first), Some(0));

        topology.register_node(second);
        assert!(topology.node_registered(second));
        assert!(topology.node_registered(first));

        assert_eq!(topology.hops(first, second), Some(1));
        assert!(topology.hops(first, third).is_none());
        assert!(topology.hops(second, third).is_none());

        topology.register_node(third);
        assert_eq!(topology.hops(first, third), Some(1));
        assert_eq!(topology.hops(second, third), Some(1));
        assert_eq!(topology.hops(third, first), Some(1));
        assert_eq!(topology.hops(third, second), Some(1));

        topology.separate(&[first, second]);
        assert_eq!(topology.hops(first, second), Some(1));
        assert_eq!(topology.hops(second, first), Some(1));
        assert_eq!(topology.hops(first, first), Some(0));
        assert_eq!(topology.hops(second, second), Some(0));
        assert_eq!(topology.hops(first, third), None);
        assert_eq!(topology.hops(second, third), None);
        assert_eq!(topology.hops(third, first), None);
        assert_eq!(topology.hops(third, second), None);

        topology.separate(&[first]);
        assert_eq!(topology.hops(first, first), Some(0));
        assert_eq!(topology.hops(first, second), None);
        assert_eq!(topology.hops(first, third), None);
        assert_eq!(topology.hops(second, third), None);
        assert_eq!(topology.hops(second, first), None);
        assert_eq!(topology.hops(third, first), None);
        assert_eq!(topology.hops(third, second), None);

        topology.repair(&[first, third]);
        assert_eq!(topology.hops(first, first), Some(0));
        assert_eq!(topology.hops(first, third), Some(1));
        assert_eq!(topology.hops(third, first), Some(1));
        assert_eq!(topology.hops(third, third), Some(0));
        assert_eq!(topology.hops(second, second), Some(0));
        assert_eq!(topology.hops(first, second), None);
        assert_eq!(topology.hops(second, first), None);
        assert_eq!(topology.hops(second, third), None);
        assert_eq!(topology.hops(third, second), None);

        topology.repair_all();
        for a in &[first, second, third] {
            for b in &[first, second, third] {
                let hops = if a == b { 0 } else { 1 };
                assert_eq!(topology.hops(a, b), Some(hops));
            }
        }
    }
}
