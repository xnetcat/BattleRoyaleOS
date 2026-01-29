//! Game Application Module
//!
//! Contains the game client code that runs on top of the kernel.
//! This module is separate from kernel hardware init and provides
//! the actual game loop, rendering, and UI.

pub mod hud;
pub mod input;
pub mod render;
pub mod run;
pub mod terrain;

pub use input::get_menu_action;
pub use render::{render_worker, set_gpu_batch_available, GPU_BATCH_AVAILABLE};
pub use run::{run, set_benchmark_mode, set_test_mode, network_worker};
