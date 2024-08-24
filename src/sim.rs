pub mod node;
mod runtime;
pub mod spawn;
mod time;

use node::NodeHandle;
pub use spawn::spawn;

pub(crate) fn in_sim() -> bool {
    NodeHandle::exists()
}
