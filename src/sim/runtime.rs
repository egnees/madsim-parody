use std::{
    cell::{RefCell, RefMut},
    future::Future,
    pin::Pin,
    rc::Rc,
    sync::Arc,
    task::{Context, Poll},
};

use futures::FutureExt;
use task::Task;
use thiserror::Error;

use state::RuntimeState;
use waker::Waker;

////////////////////////////////////////////////////////////////////////////////

mod state;
mod task;
mod waker;

////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub(crate) struct Runtime(Rc<RefCell<RuntimeState>>);

impl Runtime {
    pub fn new() -> Self {
        Default::default()
    }

    fn state(&self) -> RefMut<'_, RuntimeState> {
        self.0.borrow_mut()
    }

    pub fn has_work(&self) -> bool {
        self.0.borrow().queue_size() > 0
    }

    pub fn next_step(&self) -> bool {
        let Some(mut task) = self.state().take_task() else {
            return false;
        };

        let waker = futures::task::waker(Arc::new(Waker {
            handle: Rc::downgrade(&self.0),
            task_id: task.id(),
        }));

        let mut context = Context::from_waker(&waker);

        if task.poll(&mut context).is_pending() {
            self.state().add_task(task);
        }

        true
    }

    #[allow(unused)]
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
            // which is okay behaviour
            let _ = sender.send(result);
        };
        self.submit(task);
        JoinHandle { receiver }
    }

    fn submit(&self, task: impl Future<Output = ()> + 'static) {
        let task: Task = task.into();
        let mut state = self.state();
        let id = task.id();
        state.add_task(task);
        state.push_task(id);
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

////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests;
