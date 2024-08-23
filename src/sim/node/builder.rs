use std::rc::Rc;

use super::{state::NodeState, Node};

#[derive(Clone)]
pub struct NodeBuilder {}

impl NodeBuilder {
    #[allow(unused)]
    pub(crate) fn new() -> Self {
        Self {}
    }

    pub fn build(&self) -> Node {
        Node(Rc::new(NodeState::new()))
    }
}
