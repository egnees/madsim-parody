use crate::sim::runtime::Runtime;

////////////////////////////////////////////////////////////////////////////////

pub(crate) struct NodeState {
    pub runtime: Runtime,
}

impl NodeState {
    pub fn new() -> Self {
        Self {
            runtime: Runtime::new(),
        }
    }
}
