pub mod node;
mod runtime;
pub mod spawn;
mod time;

use node::NodeHandle;

pub use spawn::spawn;
pub use time::now;
pub use time::sleep;

pub fn in_sim() -> bool {
    NodeHandle::exists()
}
