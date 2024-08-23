use std::future::Future;

use super::{node, runtime::JoinHandle};

pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + 'static,
{
    node::NodeHandle::current().spawn(future)
}

#[cfg(test)]
mod tests {
    use std::sync::{atomic::AtomicUsize, Arc};

    use crate::sim::node::NodeBuilder;

    use super::spawn;

    #[test]
    fn basic() {
        let node = NodeBuilder::new().build();
        let handle = node.handle().spawn(async { 5 });
        let (sender, receiver) = std::sync::mpsc::channel();
        node.handle().spawn(async move {
            let result = spawn(async { 10 }).await.unwrap();
            let x = result + handle.await.unwrap();
            sender.send(x).unwrap();
        });
        node.handle().make_steps(None);
        let result = receiver.recv().unwrap();
        assert_eq!(result, 15);
    }

    #[test]
    fn node_alternation() {
        let node1 = NodeBuilder::new().build();
        let node2 = NodeBuilder::new().build();

        let cnt = Arc::new(AtomicUsize::new(0));

        node1.handle().spawn({
            let cnt = cnt.clone();
            async move {
                spawn(async move {
                    cnt.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                })
            }
        });

        node2.handle().spawn({
            let cnt = cnt.clone();
            async move {
                spawn(async move {
                    cnt.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                })
            }
        });

        assert_eq!(node1.handle().make_steps(None), 2);
        assert_eq!(node2.handle().make_steps(None), 2);

        assert_eq!(cnt.load(std::sync::atomic::Ordering::SeqCst), 2);
    }
}
