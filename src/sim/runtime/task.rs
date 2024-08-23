use std::{
    future::Future,
    ops::Deref,
    pin::Pin,
    task::{Context, Poll},
};

////////////////////////////////////////////////////////////////////////////////

pub(crate) struct Task(Pin<Box<dyn Future<Output = ()>>>);

impl Task {
    pub fn id(&self) -> TaskId {
        self.0.deref() as *const dyn Future<Output = ()> as *const () as TaskId
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        self.0.as_mut().poll(cx)
    }
}

impl<T> From<T> for Task
where
    T: Future<Output = ()> + 'static,
{
    fn from(value: T) -> Self {
        Task(Box::pin(value))
    }
}

////////////////////////////////////////////////////////////////////////////////

pub(crate) type TaskId = usize;
