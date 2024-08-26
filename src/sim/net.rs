use std::{
    cell::RefCell,
    collections::{hash_map::Entry, BinaryHeap},
    io,
    net::SocketAddr,
    rc::{Rc, Weak},
    time::Duration,
};

use datagram::Datagram;
use event::NetworkEvent;
use registry::{SocketData, SocketRegistry};

mod datagram;
mod event;
mod registry;
mod topology;
mod udp;

use rand::{
    distributions::uniform::{UniformDuration, UniformSampler},
    rngs::StdRng,
    Rng, SeedableRng,
};
use topology::NetworkTopology;
use udp::UpdSocketData;

use crate::{net::ip_addr::ToIpAddr, time::Timestamp};

use super::now;

pub use udp::UdpSocket;

////////////////////////////////////////////////////////////////////////////////

struct NetworkState {
    registry: SocketRegistry,
    rng: StdRng,
    min_delay: Duration,
    max_delay: Duration,
    drop_rate: f64,
    events: BinaryHeap<NetworkEvent>,
    topology: NetworkTopology,
}

impl NetworkState {
    pub fn new(seed: u64) -> Self {
        Self {
            registry: Default::default(),
            rng: StdRng::seed_from_u64(seed),
            min_delay: Network::DEFAULT_MIN_DELAY,
            max_delay: Network::DEFAULT_MAX_DELAY,
            drop_rate: Network::DEFAULT_DROP_RATE,
            events: Default::default(),
            topology: NetworkTopology::new(),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

pub(crate) struct Network(Rc<RefCell<NetworkState>>);

impl Network {
    const DEFAULT_MIN_DELAY: Duration = Duration::from_millis(100);
    const DEFAULT_MAX_DELAY: Duration = Duration::from_millis(500);
    const DEFAULT_DROP_RATE: f64 = 0.05;

    pub(crate) fn new(seed: u64) -> Self {
        Self(Rc::new(RefCell::new(NetworkState::new(seed))))
    }

    pub fn handle(&self) -> NetworkHandle {
        NetworkHandle(Rc::downgrade(&self.0))
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone)]
pub struct NetworkHandle(Weak<RefCell<NetworkState>>);

impl NetworkHandle {
    fn register_upd_socket(&self, socket: Rc<RefCell<UpdSocketData>>) -> io::Result<()> {
        let state = self.state();
        let mut state = state.borrow_mut();
        let addr = socket.borrow().local_addr;
        if let Entry::Vacant(e) = state.registry.0.entry(addr) {
            e.insert(SocketData::Udp(Rc::downgrade(&socket)));
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::AddrInUse,
                "address already in use",
            ))
        }
    }

    fn deregister_socket(&self, addr: SocketAddr) {
        self.state().borrow_mut().registry.0.remove(&addr).unwrap();
    }

    fn send_upd_packet(&self, from: SocketAddr, to: SocketAddr, packet: &[u8]) -> bool {
        let state = self.state();
        let mut state = state.borrow_mut();
        // 'from' socket must be registered
        let SocketData::Udp(from_socket) =
            state.registry.0.get(&from).expect("'from' not registered")
        else {
            panic!("socket has inconsistent type")
        };
        // 'from' socket is not alive
        let Some(from_socket) = from_socket.upgrade() else {
            return true;
        };
        let Some(SocketData::Udp(to_socket)) = state.registry.0.get(&to) else {
            return true;
        };
        // 'to' socket is not alive
        let Some(to_socket) = to_socket.upgrade() else {
            return true;
        };
        // package dropped
        if to_socket.borrow().local_addr != from_socket.borrow().local_addr
            && state.rng.gen_range(0.0..1.0) < state.drop_rate
        {
            return true;
        }
        // drop if not connected
        let Some(hops) = state.topology.hops(
            from_socket.borrow().local_addr.ip(),
            to_socket.borrow().local_addr.ip(),
        ) else {
            return true;
        };
        // package not dropped
        let delay = UniformDuration::new(state.min_delay, state.max_delay)
            .sample(&mut state.rng)
            .checked_mul(hops as u32).unwrap();
        let timestamp = now() + delay;
        let event = NetworkEvent {
            timestamp,
            sender: from_socket.borrow().local_addr,
            receiver: to_socket.borrow().local_addr,
            data: Vec::from_iter(packet.iter().cloned()),
        };
        state.events.push(event);
        false
    }

    ////////////////////////////////////////////////////////////////////////////////

    pub(crate) fn register_node(&self, addr: impl ToIpAddr) {
        self.state().borrow_mut().topology.register_node(addr);
    }

    pub fn separate<A: ToIpAddr>(&self, group: &[A]) {
        self.state().borrow_mut().topology.separate(group);
    }

    pub fn repair<A: ToIpAddr>(&self, group: &[A]) {
        self.state().borrow_mut().topology.repair(group);
    }

    pub fn repair_all(&mut self) {
        self.state().borrow_mut().topology.repair_all()
    }

    ////////////////////////////////////////////////////////////////////////////////

    pub(crate) fn next_event_timestamp(&self) -> Option<Timestamp> {
        self.state().borrow().events.peek().map(|e| e.timestamp)
    }

    pub(crate) fn advance_to_time(&self, timestamp: Timestamp) {
        while let Some(time) = self.next_event_timestamp() {
            if time > timestamp {
                break;
            }
            let next_event = self.state().borrow_mut().events.pop().unwrap();
            self.handle_event(next_event);
        }
    }

    fn handle_event(&self, event: NetworkEvent) {
        let receiver = event.receiver;
        if let Some(SocketData::Udp(receiver_data)) =
            self.state().borrow_mut().registry.0.get(&receiver)
        {
            if let Some(receiver_data) = receiver_data.upgrade() {
                receiver_data.borrow_mut().recv_buf.add_datagram(Datagram {
                    from: event.sender,
                    to: receiver,
                    data: event.data,
                });
                receiver_data
                    .borrow_mut()
                    .recv_waiters
                    .drain(..)
                    .for_each(|waiter| waiter.wake());
            }
        }
    }

    ////////////////////////////////////////////////////////////////////////////////

    fn state(&self) -> Rc<RefCell<NetworkState>> {
        self.0.upgrade().unwrap()
    }
}
