//! SVGA3D Rendering Backend
//!
//! Uses the GPU batch renderer for hardware-accelerated 3D rendering.

use crate::api::types::Color;
use crate::gfx::device::{GpuTriangle, GpuVertex};
use crate::graphics::gpu_batch;

/// SVGA3D renderer backend
pub struct Svga3DBackend {
    width: u32,
    height: u32,
    initialized: bool,
}

impl Svga3DBackend {
    pub fn new(width: u32, height: u32) -> Self {
        let initialized = crate::graphics::gpu::has_3d();
        Self {
            width,
            height,
            initialized,
        }
    }

    /// Check if the backend is available
    pub fn is_available(&self) -> bool {
        self.initialized
    }

    /// Begin a new batch
    pub fn begin_batch(&self) {
        if self.initialized {
            gpu_batch::begin_batch();
        }
    }

    /// End the current batch
    pub fn end_batch(&self) {
        if self.initialized {
            gpu_batch::end_batch();
        }
    }

    /// Clear the framebuffer
    pub fn clear(&self, color: Color) {
        crate::graphics::gpu::clear(color.to_u32());
    }

    /// Clear the depth buffer
    pub fn clear_depth(&self) {
        crate::graphics::zbuffer::clear();
    }

    /// Add a triangle to the batch
    pub fn add_triangle(&self, v0: GpuVertex, v1: GpuVertex, v2: GpuVertex) {
        if self.initialized {
            gpu_batch::add_triangle_verts(
                v0.x, v0.y, v0.z, v0.color,
                v1.x, v1.y, v1.z, v1.color,
                v2.x, v2.y, v2.z, v2.color,
            );
        }
    }

    /// Draw triangles
    pub fn draw_triangles(&self, triangles: &[GpuTriangle]) {
        if !self.initialized {
            return;
        }

        for tri in triangles {
            self.add_triangle(tri.v0, tri.v1, tri.v2);
        }
    }

    /// Fill a rectangle (uses software path for 2D)
    pub fn fill_rect(&self, x: i32, y: i32, width: u32, height: u32, color: Color) {
        crate::graphics::gpu::fill_rect(
            x as usize,
            y as usize,
            width as usize,
            height as usize,
            color.to_u32(),
        );
    }

    /// Present the frame
    pub fn present(&self) {
        crate::graphics::gpu::present();
    }
}
