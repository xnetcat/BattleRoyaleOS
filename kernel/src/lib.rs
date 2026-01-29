//! BattleRoyaleOS Kernel Library
//!
//! This crate exposes kernel modules for use by applications.
//! The kernel initializes hardware and then delegates to the appropriate app.

#![no_std]
#![feature(abi_x86_interrupt)]

extern crate alloc;

pub mod api;
pub mod app;
pub mod boot;
pub mod drivers;
pub mod game;
pub mod gfx;
pub mod graphics;
pub mod memory;
pub mod net;
pub mod smp;
pub mod ui;

// Re-export commonly used items
pub use graphics::framebuffer;
pub use graphics::gpu;
pub use graphics::rasterizer;
pub use graphics::pipeline;
pub use graphics::tiles;
pub use graphics::zbuffer;
pub use graphics::font;
pub use graphics::cursor;
pub use graphics::vsync;
pub use graphics::culling;
pub use graphics::gpu_batch;
pub use graphics::gpu_render;

// Re-export serial macro
pub use drivers::serial;

/// Read the CPU timestamp counter
#[inline]
pub fn read_tsc() -> u64 {
    unsafe { core::arch::x86_64::_rdtsc() }
}

/// Halt the CPU in a loop
pub fn halt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
