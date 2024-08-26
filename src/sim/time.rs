use std::{
    cell::{RefCell, RefMut},
    cmp::Ordering,
    collections::BinaryHeap,
    future::poll_fn,
    rc::Rc,
    task::Poll,
    time::Duration,
};

use futures::task::AtomicWaker;

use super::node::NodeHandle;
use crate::time::Timestamp;

////////////////////////////////////////////////////////////////////////////////

pub(crate) struct TimerEntry {
    pub timestamp: Timestamp,
    pub waker: AtomicWaker,
}

impl PartialEq for TimerEntry {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp == other.timestamp
    }
}

impl Eq for TimerEntry {}

impl Ord for TimerEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other.timestamp.cmp(&self.timestamp)
    }
}

impl PartialOrd for TimerEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct TimeState {
    heap: BinaryHeap<Rc<TimerEntry>>,
    time: Timestamp,
}

////////////////////////////////////////////////////////////////////////////////

pub struct TimeDriver(RefCell<TimeState>);

impl TimeDriver {
    pub fn new() -> Self {
        Self(RefCell::new(Default::default()))
    }

    pub fn add_timer(&self, timestamp: Timestamp) -> Rc<TimerEntry> {
        let waker = AtomicWaker::new();
        let entry = Rc::new(TimerEntry { timestamp, waker });
        self.state().heap.push(entry.clone());
        entry
    }

    pub fn next_timer(&self) -> Option<Rc<TimerEntry>> {
        self.state().heap.peek().cloned()
    }

    pub fn advance_to_next_timer(&self) -> bool {
        let next = self.state().heap.pop();
        if let Some(next) = next {
            self.state().time = next.timestamp;
            next.waker.wake();
            true
        } else {
            false
        }
    }

    pub fn advance_to_time(&self, to: Timestamp) {
        assert!(self.state().time <= to);
        while let Some(entry) = self.peek() {
            if entry.timestamp <= to {
                self.advance_to_next_timer();
            } else {
                break;
            }
        }
        self.state().time = to;
    }

    pub fn time(&self) -> Timestamp {
        self.state().time
    }

    ////////////////////////////////////////////////////////////////////////////////

    fn state(&self) -> RefMut<'_, TimeState> {
        self.0.borrow_mut()
    }

    fn peek(&self) -> Option<Rc<TimerEntry>> {
        self.state().heap.peek().cloned()
    }
}

////////////////////////////////////////////////////////////////////////////////

pub async fn sleep(duration: Duration) {
    let node_handle = NodeHandle::current();
    let timer_entry = node_handle.add_timer(duration);
    poll_fn(move |cx| {
        timer_entry.waker.register(cx.waker());
        if timer_entry.timestamp <= node_handle.time() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    })
    .await;
}

////////////////////////////////////////////////////////////////////////////////

pub fn now() -> Duration {
    NodeHandle::current().time()
}

////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize},
        Arc,
    };

    use futures::task::{waker, ArcWake};

    use crate::sim::{node::NodeBuilder, spawn, Sim};

    use super::*;

    ////////////////////////////////////////////////////////////////////////////////

    fn make_node() -> (Sim, NodeHandle) {
        let mut sim = Sim::new(123);
        let node = NodeBuilder::from_ip_addr("1.1.1.1")
            .unwrap()
            .build(&mut sim)
            .unwrap();
        (sim, node)
    }

    ////////////////////////////////////////////////////////////////////////////////

    #[test]
    fn driver_works() {
        let driver = TimeDriver::new();
        assert_eq!(driver.time(), Duration::from_secs(0));

        driver.add_timer(Duration::from_secs(1));
        driver.add_timer(Duration::from_secs(2));
        driver.add_timer(Duration::from_millis(500));

        assert_eq!(
            driver.next_timer().unwrap().timestamp,
            Duration::from_millis(500)
        );

        assert!(driver.advance_to_next_timer());
        assert_eq!(
            driver.next_timer().unwrap().timestamp,
            Duration::from_secs(1)
        );

        driver.advance_to_time(Duration::from_millis(2100));
        assert!(driver.next_timer().is_none());
    }

    #[test]
    fn driver_wakes_up() {
        let driver = TimeDriver::new();
        let entry = driver.add_timer(Duration::from_secs(1));
        struct Waker {
            wakeups: AtomicUsize,
        }

        impl ArcWake for Waker {
            fn wake_by_ref(arc_self: &Arc<Self>) {
                arc_self
                    .wakeups
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
        }

        let arc_waker = Arc::new(Waker {
            wakeups: AtomicUsize::new(0),
        });
        let waker = waker(arc_waker.clone());

        entry.waker.register(&waker);
        assert!(driver.advance_to_next_timer());
        assert_eq!(
            arc_waker.wakeups.load(std::sync::atomic::Ordering::SeqCst),
            1
        );
    }

    ////////////////////////////////////////////////////////////////////////////////

    #[test]
    fn time_works() {
        let (_sim, node) = make_node();
        assert_eq!(node.time(), Duration::from_secs(0));
        node.step_duration(Duration::from_secs(2));
        assert_eq!(node.time(), Duration::from_secs(2));
        node.spawn(async {
            assert_eq!(now(), Duration::from_secs(2));
        });
        assert_eq!(node.make_steps(None), 1);
    }

    #[test]
    fn sleep_works() {
        let (_sim, node) = make_node();
        let flag = Rc::new(RefCell::new(false));
        node.spawn({
            let flag = flag.clone();
            async move {
                sleep(Duration::from_secs(1)).await;
                *flag.borrow_mut() = true;
            }
        });
        let steps = node.make_steps(None);
        assert!(steps > 1);
        assert_eq!(node.time(), Duration::from_secs(1));
        assert!(*flag.borrow());

        *flag.borrow_mut() = false;
        node.spawn({
            let flag = flag.clone();
            async move {
                sleep(Duration::from_secs(1)).await;
                *flag.borrow_mut() = true;
            }
        });
        let steps = node.step_duration(Duration::from_secs(1));
        assert!(steps > 0);
        assert_eq!(node.time(), Duration::from_secs(2));
        assert!(*flag.borrow());

        let cnt = Rc::new(RefCell::new(0usize));
        node.spawn({
            let cnt = cnt.clone();
            async move {
                spawn({
                    let cnt = cnt.clone();
                    async move {
                        sleep(Duration::from_secs(1)).await;
                        *cnt.borrow_mut() += 1;
                    }
                });
                sleep(Duration::from_secs(2)).await;
                *cnt.borrow_mut() += 1;
            }
        });
        let steps = node.step_duration(Duration::from_millis(1500)); // 1.5s
        assert!(steps > 0);
        assert_eq!(node.time(), Duration::from_millis(3500));
        assert_eq!(*cnt.borrow(), 1);

        let steps = node.step_duration(Duration::from_millis(500));
        assert!(steps > 0);
        assert_eq!(node.time(), Duration::from_secs(4));
        assert_eq!(*cnt.borrow(), 2);
    }

    #[test]
    fn now_works() {
        let (_sim, node) = make_node();
        node.spawn(async {
            assert_eq!(now(), Duration::from_secs(0));
            sleep(Duration::from_secs(2)).await;
            spawn(async {
                assert_eq!(now(), Duration::from_secs(2));
                spawn(async {
                    assert_eq!(now(), Duration::from_secs(2));
                    sleep(Duration::from_secs(1)).await;
                    assert_eq!(now(), Duration::from_secs(3));
                });
            });
            assert_eq!(now(), Duration::from_secs(2));
        });
        node.make_steps(None);
    }

    ////////////////////////////////////////////////////////////////////////////////

    #[test]
    fn sleep_zero() {
        let (_sim, node) = make_node();
        let flag = Rc::new(AtomicBool::new(false));
        node.spawn({
            let flag = flag.clone();
            async move {
                sleep(Duration::from_secs(0)).await;
                flag.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        });
        node.step_duration(Duration::from_secs(0));
        assert_eq!(flag.load(std::sync::atomic::Ordering::SeqCst), true);
    }
}
