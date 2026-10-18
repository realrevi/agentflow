//! Small shared helpers used by several backend modules.

mod atomic_write;
pub mod base64;
mod blocking;
mod hex_color;
mod sync;
#[cfg(test)]
pub mod test_dir;
mod workspace_key;

pub use atomic_write::{write_atomic, write_atomic_with};
pub use blocking::run_blocking;
pub use hex_color::is_hex_color;
pub use sync::{lock, Signal};
pub use workspace_key::{now_ms, workspace_key};
