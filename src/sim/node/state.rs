use crate::sim::{
    runtime::Runtime,
    time::{TimeDriver, TimeHandle},
};

////////////////////////////////////////////////////////////////////////////////

pub(crate) struct NodeState {
    pub runtime: Runtime,
    pub time_handle: TimeHandle,
}

impl NodeState {
    pub fn new() -> Self {
        Self {
            runtime: Runtime::new(),
            time_handle: TimeDriver::start(),
        }
    }
}
