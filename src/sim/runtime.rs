use std::{
    cell::RefCell,
    future::Future,
    pin::Pin,
    rc::Rc,
    sync::Arc,
    task::{Context, Poll},
};

use futures::FutureExt;
use thiserror::Error;

use state::State;
use waker::Waker;

////////////////////////////////////////////////////////////////////////////////

mod state;
mod task;
mod waker;

#[cfg(test)]
mod tests;

////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub(crate) struct Runtime {
    state: Rc<RefCell<State>>,
}

impl Runtime {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn next_step(&self) -> bool {
        let Some(mut task) = self.state.borrow_mut().take_task() else {
            return false;
        };

        let waker = futures::task::waker(Arc::new(Waker {
            state: Rc::downgrade(&self.state),
            task_id: task.id(),
        }));

        let mut context = Context::from_waker(&waker);

        if task.poll(&mut context).is_pending() {
            self.state.borrow_mut().add_task(task);
        }

        true
    }

    pub fn make_steps(&self, steps: Option<usize>) -> usize {
        let mut cnt = 0;
        if let Some(steps) = steps {
            for _ in 0..steps {
                if !self.next_step() {
                    break;
                }
                cnt += 1
            }
        } else {
            while self.next_step() {
                cnt += 1
            }
        };
        cnt
    }

    pub fn spawn<F>(&self, task: F) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
    {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let task = async move {
            let result = task.await;
            // if send failed, then join handle has been dropped,
            // which is normal behaviour
            let _ = sender.send(result);
        };
        self.submit(task);
        JoinHandle { receiver }
    }

    fn submit(&self, task: impl Future<Output = ()> + 'static) {
        self.state.borrow_mut().add_task(task.into())
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Error, PartialEq)]
#[error("the task has been dropped")]
pub struct JoinError {}

pub struct JoinHandle<T> {
    receiver: tokio::sync::oneshot::Receiver<T>,
}

impl<T> Future for JoinHandle<T> {
    type Output = Result<T, JoinError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.get_mut()
            .receiver
            .poll_unpin(cx)
            .map_err(|_| JoinError {})
    }
}
