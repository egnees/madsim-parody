mod runtime;
mod time;

pub mod node;
pub mod spawn;

use node::NodeHandle;

pub use spawn::spawn;
pub use time::now;
pub use time::sleep;

pub fn in_sim() -> bool {
    NodeHandle::exists()
}
