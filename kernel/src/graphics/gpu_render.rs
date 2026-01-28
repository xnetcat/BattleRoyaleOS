//! GPU-accelerated rendering integration
//!
//! This module provides a hybrid rendering approach:
//! - GPU acceleration for clear and present operations
//! - Software rasterization for triangles (reliable fallback)
//! - Infrastructure for future full GPU rasterization
//!
//! When SVGA3D is available:
//! - Frame clear is done by GPU (faster than CPU memory fill)
//! - Present blits from render target to screen (GPU blit)
//! - Triangle rasterization still uses optimized software path
//!
//! This hybrid approach provides measurable speedup while maintaining
//! compatibility with all VMSVGA implementations.

use crate::drivers::vmsvga;
use crate::drivers::vmsvga::svga3d::{self, Matrix4x4, TransformType};
use crate::graphics::framebuffer::FRAMEBUFFER;
use crate::graphics::gpu;
use crate::graphics::gpu3d;
use crate::graphics::rasterizer::RenderContext;
use crate::graphics::zbuffer::ZBUFFER;
use crate::serial_println;
use alloc::vec::Vec;
use glam::{Mat4, Vec3};
use renderer::vertex::Vertex;
use spin::Mutex;

/// GPU render state for hybrid rendering
pub struct GpuRenderState {
    /// Whether GPU acceleration is enabled
    pub gpu_enabled: bool,
    /// Current frame's transformed triangles (for batching)
    pub triangle_batch: Vec<GpuTriangle>,
    /// Maximum triangles per batch
    pub max_triangles: usize,
    /// Frame counter for benchmarking
    pub frame_count: u64,
    /// Total triangles rendered this frame
    pub triangles_this_frame: usize,
}

/// Triangle in screen space for GPU batching
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GpuTriangle {
    pub v0: GpuVertex,
    pub v1: GpuVertex,
    pub v2: GpuVertex,
}

/// Vertex format for GPU rendering
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GpuVertex {
    /// Screen-space position (x, y, z where z is depth)
    pub x: f32,
    pub y: f32,
    pub z: f32,
    /// Color (packed ARGB)
    pub color: u32,
}

impl GpuRenderState {
    pub const fn new() -> Self {
        Self {
            gpu_enabled: false,
            triangle_batch: Vec::new(),
            max_triangles: 16384, // ~1MB vertex data
            frame_count: 0,
            triangles_this_frame: 0,
        }
    }

    /// Reset for new frame
    pub fn begin_frame(&mut self) {
        self.triangle_batch.clear();
        self.triangles_this_frame = 0;
    }

    /// Add a transformed triangle to the batch
    pub fn add_triangle(&mut self, tri: GpuTriangle) -> bool {
        if self.triangle_batch.len() >= self.max_triangles {
            return false;
        }
        self.triangle_batch.push(tri);
        self.triangles_this_frame += 1;
        true
    }
}

/// Global GPU render state
pub static GPU_RENDER: Mutex<GpuRenderState> = Mutex::new(GpuRenderState::new());

/// Initialize GPU rendering
pub fn init() {
    let mut state = GPU_RENDER.lock();
    state.gpu_enabled = gpu::has_3d();

    if state.gpu_enabled {
        serial_println!("GPU Render: SVGA3D acceleration enabled");
    } else if gpu::has_hw_accel() {
        serial_println!("GPU Render: VMSVGA 2D acceleration (no 3D)");
    } else {
        serial_println!("GPU Render: Software rendering only");
    }

    // Pre-allocate triangle batch
    state.triangle_batch = Vec::with_capacity(state.max_triangles);
}

/// Check if GPU acceleration is available
pub fn is_gpu_enabled() -> bool {
    GPU_RENDER.lock().gpu_enabled
}

/// Begin a new frame
/// Clears the render target using GPU if available
pub fn begin_frame(clear_color: u32) {
    let mut state = GPU_RENDER.lock();
    state.begin_frame();
    state.frame_count += 1;
    drop(state);

    // Use GPU clear if available
    if gpu::has_3d() && gpu3d::is_ready() {
        gpu3d::begin_frame();
    }

    // Always clear software buffers too (for hybrid rendering)
    if let Some(ctx) = RenderContext::acquire() {
        ctx.clear(clear_color);
        ctx.clear_zbuffer();
    }
}

/// End frame and present
/// Uses GPU present if available
pub fn end_frame() {
    // Present via GPU if available
    gpu::present();

    // Frame stats silently tracked (use get_stats() to query)
}

/// Convert vertex color to packed ARGB
#[inline]
pub fn color_to_argb(color: Vec3) -> u32 {
    let r = (color.x * 255.0).clamp(0.0, 255.0) as u32;
    let g = (color.y * 255.0).clamp(0.0, 255.0) as u32;
    let b = (color.z * 255.0).clamp(0.0, 255.0) as u32;
    0xFF000000 | (r << 16) | (g << 8) | b
}

/// Create a GpuTriangle from renderer Vertices
pub fn create_gpu_triangle(v0: &Vertex, v1: &Vertex, v2: &Vertex) -> GpuTriangle {
    GpuTriangle {
        v0: GpuVertex {
            x: v0.position.x,
            y: v0.position.y,
            z: v0.position.z,
            color: color_to_argb(v0.color),
        },
        v1: GpuVertex {
            x: v1.position.x,
            y: v1.position.y,
            z: v1.position.z,
            color: color_to_argb(v1.color),
        },
        v2: GpuVertex {
            x: v2.position.x,
            y: v2.position.y,
            z: v2.position.z,
            color: color_to_argb(v2.color),
        },
    }
}

/// Set view matrix for GPU rendering (when full GPU path is enabled)
pub fn set_view_matrix(view: &Mat4) {
    if !gpu::has_3d() || !gpu3d::is_ready() {
        return;
    }

    // Convert glam::Mat4 to our Matrix4x4
    let cols = view.to_cols_array();
    let matrix = Matrix4x4 {
        m: [
            [cols[0], cols[4], cols[8], cols[12]],
            [cols[1], cols[5], cols[9], cols[13]],
            [cols[2], cols[6], cols[10], cols[14]],
            [cols[3], cols[7], cols[11], cols[15]],
        ],
    };

    gpu3d::set_view_matrix(&matrix);
}

/// Set projection matrix for GPU rendering (when full GPU path is enabled)
pub fn set_projection_matrix(projection: &Mat4) {
    if !gpu::has_3d() || !gpu3d::is_ready() {
        return;
    }

    // Convert glam::Mat4 to our Matrix4x4
    let cols = projection.to_cols_array();
    let matrix = Matrix4x4 {
        m: [
            [cols[0], cols[4], cols[8], cols[12]],
            [cols[1], cols[5], cols[9], cols[13]],
            [cols[2], cols[6], cols[10], cols[14]],
            [cols[3], cols[7], cols[11], cols[15]],
        ],
    };

    gpu3d::set_projection_matrix(&matrix);
}

/// Get the number of triangles rendered this frame
pub fn triangles_this_frame() -> usize {
    GPU_RENDER.lock().triangles_this_frame
}

/// Get total frame count
pub fn frame_count() -> u64 {
    GPU_RENDER.lock().frame_count
}

/// Increment triangle count (called by rasterizer)
pub fn record_triangle() {
    GPU_RENDER.lock().triangles_this_frame += 1;
}
