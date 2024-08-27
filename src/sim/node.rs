pub mod builder;
mod info;

////////////////////////////////////////////////////////////////////////////////

use core::cell::RefCell;
use std::{
    collections::BTreeSet,
    future::Future,
    net::IpAddr,
    rc::{Rc, Weak},
    time::Duration,
    u16,
};

use crate::{sim::runtime::JoinHandle, time::Timestamp};

use super::{
    context::ContextGuard,
    net::NetworkHandle,
    runtime::Runtime,
    time::{TimeDriver, TimerEntry},
};

////////////////////////////////////////////////////////////////////////////////

pub use builder::NodeBuilder;
use info::NodeInfo;

////////////////////////////////////////////////////////////////////////////////

struct NodeState {
    runtime: Runtime,
    time_driver: TimeDriver,
    network_handle: NetworkHandle,
    info: NodeInfo,
    free_ports: RefCell<BTreeSet<u16>>,
}

impl NodeState {
    fn new(info: NodeInfo, network_handle: NetworkHandle) -> Self {
        Self {
            runtime: Runtime::new(),
            time_driver: TimeDriver::new(),
            network_handle,
            info,
            free_ports: RefCell::new(BTreeSet::from_iter(1..=u16::MAX)),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct Node(Rc<NodeState>);

impl Node {
    const UPD_RECV_BUF_SIZE: usize = 4096;
    const UPD_SEND_BUF_SIZE: usize = 4096;

    pub fn handle(&self) -> NodeHandle {
        NodeHandle(Rc::downgrade(&self.0))
    }
}

////////////////////////////////////////////////////////////////////////////////

thread_local! {
    static NODE_HANDLE: RefCell<Option<NodeHandle>> = const { RefCell::new(None) };
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone)]
pub struct NodeHandle(Weak<NodeState>);

impl NodeHandle {
    pub fn current() -> Self {
        NODE_HANDLE.with(|h| {
            let h = h.borrow().as_ref().cloned();
            if let Some(h) = h {
                h.clone()
            } else {
                eprintln!("here!");
                panic!("node handle can be obtained only within a simulation")
            }
        })
    }

    pub(crate) fn exists() -> bool {
        NODE_HANDLE.with(|h| h.borrow().is_some())
    }

    pub(crate) fn set(handle: Option<NodeHandle>) {
        NODE_HANDLE.with(|h| *h.borrow_mut() = handle);
    }

    pub(crate) fn network_handle(&self) -> NetworkHandle {
        self.state().network_handle.clone()
    }

    ////////////////////////////////////////////////////////////////////////////////

    pub fn spawn<F>(&self, task: F) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
    {
        self.state().runtime.spawn(task)
    }

    ////////////////////////////////////////////////////////////////////////////////

    pub fn next_step(&self) -> bool {
        let _guard = ContextGuard::new(self.clone());
        let state = self.state();
        let runtime_made_step = state.runtime.next_step();
        if runtime_made_step {
            true
        } else if let Some(time) = self.next_event_timestamp() {
            state.network_handle.advance_to_time(time);
            state.time_driver.advance_to_time(time);
            true
        } else {
            false
        }
    }

    pub fn make_steps(&self, steps: Option<usize>) -> usize {
        let mut made_steps = 0;
        if let Some(steps) = steps {
            for _ in 0..steps {
                if !self.next_step() {
                    break;
                }
                made_steps += 1;
            }
        } else {
            while self.next_step() {
                made_steps += 1;
            }
        }
        made_steps
    }

    pub fn step_duration(&self, duration: Duration) -> usize {
        let until = self.time() + duration;
        let mut steps = 0;
        while let Some(next) = self.next_event_timestamp() {
            if next <= until {
                self.next_step();
                steps += 1
            } else {
                break;
            }
        }
        assert!(self.time() <= until);
        self.state().time_driver.advance_to_time(until);
        steps
    }

    ////////////////////////////////////////////////////////////////////////////////

    pub fn ip(&self) -> IpAddr {
        self.state().info.ip
    }

    pub fn time(&self) -> Timestamp {
        self.state().time_driver.time()
    }

    ////////////////////////////////////////////////////////////////////////////////

    pub(crate) fn info(&self) -> NodeInfo {
        self.state().info.clone()
    }

    pub(crate) fn take_port(&self, port: Option<u16>) -> Option<u16> {
        if let Some(port) = port {
            self.state().free_ports.borrow_mut().take(&port)
        } else {
            self.state().free_ports.borrow_mut().pop_first()
        }
    }

    pub(crate) fn return_port(&self, port: u16) {
        let not_existed = self.state().free_ports.borrow_mut().insert(port);
        assert!(not_existed);
    }

    ////////////////////////////////////////////////////////////////////////////////

    pub(crate) fn add_timer(&self, duration: Duration) -> Rc<TimerEntry> {
        let state = self.state();
        let time = state.time_driver.time();
        state.time_driver.add_timer(time + duration)
    }

    fn next_event_timestamp(&self) -> Option<Timestamp> {
        let state = self.state();
        if state.runtime.has_work() {
            Some(self.time())
        } else {
            let next_time_driver = state.time_driver.next_timer().map(|entry| entry.timestamp);
            let next_network = state.network_handle.next_event_timestamp();
            if next_time_driver.is_none() {
                next_network
            } else if next_network.is_none() {
                next_time_driver
            } else {
                let time_driver = next_time_driver.unwrap();
                let network = next_network.unwrap();
                Some(time_driver.min(network))
            }
        }
    }

    ////////////////////////////////////////////////////////////////////////////////

    fn state(&self) -> Rc<NodeState> {
        self.0.upgrade().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, net::IpAddr};

    use crate::sim::Sim;

    use super::{info::NodeInfo, NodeState};

    #[test]
    fn free_ports() {
        let sim = Sim::new(123);
        let node_state = NodeState::new(
            NodeInfo {
                ip: "1.1.1.1".parse::<IpAddr>().unwrap(),
                udp_send_buffer_size: 0,
                udp_recv_buffer_size: 0,
            },
            sim.network(),
        );
        assert_eq!(node_state.free_ports.borrow().len(), u16::MAX.into());
        assert_eq!(
            *node_state.free_ports.borrow(),
            BTreeSet::from_iter(1..=u16::MAX)
        );
    }
}
