//! Software Rendering Backend
//!
//! Uses the existing tile-based parallel software rasterizer.

use crate::api::types::Color;
use crate::gfx::device::{GpuTriangle, GpuVertex};
use crate::graphics::rasterizer::RenderContext;
use renderer::vertex::Vertex;
use glam::Vec3;

/// Software renderer backend
pub struct SoftwareBackend {
    width: u32,
    height: u32,
}

impl SoftwareBackend {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Clear the framebuffer
    pub fn clear(&self, color: Color) {
        crate::graphics::gpu::clear(color.to_u32());
    }

    /// Clear the depth buffer
    pub fn clear_depth(&self) {
        crate::graphics::zbuffer::clear();
    }

    /// Draw triangles
    pub fn draw_triangles(&self, triangles: &[GpuTriangle]) {
        let ctx = match RenderContext::acquire() {
            Some(ctx) => ctx,
            None => return,
        };

        for tri in triangles {
            let v0 = gpu_vertex_to_renderer(&tri.v0);
            let v1 = gpu_vertex_to_renderer(&tri.v1);
            let v2 = gpu_vertex_to_renderer(&tri.v2);

            crate::graphics::rasterizer::rasterize_triangle_with_context(&ctx, &v0, &v1, &v2);
        }
    }

    /// Draw a single triangle
    pub fn draw_triangle(&self, v0: &GpuVertex, v1: &GpuVertex, v2: &GpuVertex) {
        let ctx = match RenderContext::acquire() {
            Some(ctx) => ctx,
            None => return,
        };

        let rv0 = gpu_vertex_to_renderer(v0);
        let rv1 = gpu_vertex_to_renderer(v1);
        let rv2 = gpu_vertex_to_renderer(v2);

        crate::graphics::rasterizer::rasterize_triangle_with_context(&ctx, &rv0, &rv1, &rv2);
    }

    /// Fill a rectangle
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

/// Convert a GpuVertex to a renderer Vertex
fn gpu_vertex_to_renderer(v: &GpuVertex) -> Vertex {
    let color = Color::from_u32(v.color);
    Vertex::pos_color(
        Vec3::new(v.x, v.y, v.z),
        Vec3::new(
            color.r as f32 / 255.0,
            color.g as f32 / 255.0,
            color.b as f32 / 255.0,
        ),
    )
}
