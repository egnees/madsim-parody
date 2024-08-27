use std::future::Future;

use super::{node::NodeHandle, runtime::JoinHandle};

////////////////////////////////////////////////////////////////////////////////

pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + 'static,
{
    NodeHandle::current().spawn(future)
}

////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use std::sync::{atomic::AtomicUsize, Arc};

    use crate::sim::{node::builder::NodeBuilder, Sim};

    use super::spawn;

    #[test]
    fn basic() {
        let mut sim = Sim::new(123);
        let node = NodeBuilder::with_ip("1.1.1.1")
            .unwrap()
            .build(&mut sim)
            .unwrap();
        let (sender, receiver) = std::sync::mpsc::channel();
        node.spawn(async move {
            let x = spawn(async { 10 }).await.unwrap();
            sender.send(x).unwrap();
        });
        node.make_steps(None);
        let result = receiver.recv().unwrap();
        assert_eq!(result, 10);
    }

    #[test]
    fn node_alternation() {
        let mut sim = Sim::new(123);
        let node1 = NodeBuilder::with_ip("1.1.1.1")
            .unwrap()
            .build(&mut sim)
            .unwrap();
        let node2 = NodeBuilder::with_ip("1.1.1.2")
            .unwrap()
            .build(&mut sim)
            .unwrap();

        let cnt = Arc::new(AtomicUsize::new(0));

        node1.spawn({
            let cnt = cnt.clone();
            async move {
                spawn(async move {
                    cnt.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                })
            }
        });

        node2.spawn({
            let cnt = cnt.clone();
            async move {
                spawn(async move {
                    cnt.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                })
            }
        });

        node1.make_steps(None);
        assert_eq!(cnt.load(std::sync::atomic::Ordering::SeqCst), 1);

        node2.make_steps(None);
        assert_eq!(cnt.load(std::sync::atomic::Ordering::SeqCst), 2);
    }
}
