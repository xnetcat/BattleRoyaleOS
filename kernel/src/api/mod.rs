//! Kernel API
//!
//! Public API for applications to interact with kernel services.
//! This provides a stable interface that isolates applications from
//! kernel internals.

pub mod graphics;
pub mod input;
pub mod network;
pub mod time;
pub mod types;

pub use graphics::GraphicsDevice;
pub use input::{InputService, KeyState, MouseState};
pub use network::NetworkService;
pub use time::TimeService;
pub use types::*;
