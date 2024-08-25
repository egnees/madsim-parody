use std::{
    cell::RefCell, cmp::Ordering, collections::BinaryHeap, future::poll_fn, rc::Rc, task::Poll,
    time::Duration,
};

use futures::task::AtomicWaker;

use crate::time::Timestamp;

use super::node::NodeHandle;

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
pub(crate) struct TimeDriver {
    heap: BinaryHeap<Rc<TimerEntry>>,
    time: Timestamp,
}

impl TimeDriver {
    pub fn start() -> TimeHandle {
        TimeHandle(Rc::new(RefCell::new(Default::default())))
    }

    pub fn add_timer(&mut self, timestamp: Timestamp) -> Rc<TimerEntry> {
        let waker = AtomicWaker::new();
        let entry = Rc::new(TimerEntry { timestamp, waker });
        self.heap.push(entry.clone());
        entry
    }

    pub fn next_timer(&self) -> Option<Rc<TimerEntry>> {
        self.heap.peek().cloned()
    }

    pub fn advance_to_next_timer(&mut self) -> bool {
        if let Some(next) = self.heap.pop() {
            self.time = next.timestamp;
            next.waker.wake();
            true
        } else {
            false
        }
    }

    pub fn advance_time(&mut self, to: Timestamp) {
        assert!(self.time <= to);
        while let Some(entry) = self.heap.peek() {
            if entry.timestamp <= to {
                self.advance_to_next_timer();
            } else {
                break;
            }
        }
        self.time = to;
    }

    pub fn time(&self) -> Timestamp {
        self.time
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone)]
pub(crate) struct TimeHandle(pub Rc<RefCell<TimeDriver>>);

////////////////////////////////////////////////////////////////////////////////

pub async fn sleep(duration: Duration) {
    let time_handle = &NodeHandle::current().node().0.time_handle;
    let timer_entry = {
        let mut handle = time_handle.0.borrow_mut();
        let timestamp = handle.time() + duration;
        handle.add_timer(timestamp)
    };
    poll_fn(move |cx| {
        timer_entry.waker.register(cx.waker());
        if timer_entry.timestamp <= time_handle.0.borrow().time() {
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
    use std::{
        cell::RefCell,
        rc::Rc,
        sync::{
            atomic::{AtomicBool, AtomicUsize},
            Arc,
        },
        time::Duration,
    };

    use futures::task::{waker, ArcWake};

    use crate::sim::{
        node::NodeBuilder,
        spawn,
        time::{now, sleep},
    };

    use super::TimeDriver;

    ////////////////////////////////////////////////////////////////////////////////

    #[test]
    fn driver_works() {
        let handle = TimeDriver::start();
        let mut borrow = handle.0.borrow_mut();
        assert_eq!(borrow.time(), Duration::from_secs(0));

        borrow.add_timer(Duration::from_secs(1));
        borrow.add_timer(Duration::from_secs(2));
        borrow.add_timer(Duration::from_millis(500));

        assert_eq!(
            borrow.next_timer().unwrap().timestamp,
            Duration::from_millis(500)
        );

        assert!(borrow.advance_to_next_timer());
        assert_eq!(
            borrow.next_timer().unwrap().timestamp,
            Duration::from_secs(1)
        );

        borrow.advance_time(Duration::from_millis(2100));
        assert!(borrow.next_timer().is_none());
    }

    #[test]
    fn driver_wakes_up() {
        let handle = TimeDriver::start();
        let mut borrow = handle.0.borrow_mut();

        let entry = borrow.add_timer(Duration::from_secs(1));
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
        assert!(borrow.advance_to_next_timer());
        assert_eq!(
            arc_waker.wakeups.load(std::sync::atomic::Ordering::SeqCst),
            1
        );
    }

    ////////////////////////////////////////////////////////////////////////////////

    #[test]
    fn time_works() {
        let node = NodeBuilder::new().build();
        let handle = node.handle();
        assert_eq!(handle.time(), Duration::from_secs(0));
        handle.step_duration(Duration::from_secs(2));
        assert_eq!(handle.time(), Duration::from_secs(2));
        handle.spawn(async {
            assert_eq!(now(), Duration::from_secs(2));
        });
        assert_eq!(handle.make_steps(None), 1);
    }

    #[test]
    fn sleep_works() {
        let node = NodeBuilder::new().build();
        let handle = node.handle();
        let flag = Rc::new(RefCell::new(false));
        handle.spawn({
            let flag = flag.clone();
            async move {
                sleep(Duration::from_secs(1)).await;
                *flag.borrow_mut() = true;
            }
        });
        let steps = handle.make_steps(None);
        assert!(steps > 1);
        assert_eq!(handle.time(), Duration::from_secs(1));
        assert!(*flag.borrow());

        *flag.borrow_mut() = false;
        handle.spawn({
            let flag = flag.clone();
            async move {
                sleep(Duration::from_secs(1)).await;
                *flag.borrow_mut() = true;
            }
        });
        let steps = handle.step_duration(Duration::from_secs(1));
        assert!(steps > 0);
        assert_eq!(handle.time(), Duration::from_secs(2));
        assert!(*flag.borrow());

        let cnt = Rc::new(RefCell::new(0usize));
        handle.spawn({
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
        let steps = handle.step_duration(Duration::from_millis(1500)); // 1.5s
        assert!(steps > 0);
        assert_eq!(handle.time(), Duration::from_millis(3500));
        assert_eq!(*cnt.borrow(), 1);

        let steps = handle.step_duration(Duration::from_millis(500));
        assert!(steps > 0);
        assert_eq!(handle.time(), Duration::from_secs(4));
        assert_eq!(*cnt.borrow(), 2);
    }

    #[test]
    fn now_works() {
        let node = NodeBuilder::new().build();
        let handle = node.handle();
        handle.spawn(async {
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
        handle.make_steps(None);
    }

    ////////////////////////////////////////////////////////////////////////////////

    #[test]
    fn sleep_zero() {
        let node = NodeBuilder::new().build();
        let handle = node.handle();
        let flag = Rc::new(AtomicBool::new(false));
        handle.spawn({
            let flag = flag.clone();
            async move {
                sleep(Duration::from_secs(0)).await;
                flag.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        });
        handle.step_duration(Duration::from_secs(0));
        assert_eq!(flag.load(std::sync::atomic::Ordering::SeqCst), true);
    }
}
