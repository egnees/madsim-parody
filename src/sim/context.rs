use super::node::NodeHandle;

////////////////////////////////////////////////////////////////////////////////

pub struct ContextGuard {}

impl ContextGuard {
    pub fn new(handle: NodeHandle) -> Self {
        NodeHandle::set(Some(handle));
        Self {}
    }
}

impl Drop for ContextGuard {
    fn drop(&mut self) {
        NodeHandle::set(None)
    }
}
