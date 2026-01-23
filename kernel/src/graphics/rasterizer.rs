//! Software Triangle Rasterizer
//!
//! Based on the standard barycentric coordinate method from Scratchapixel.
//! Uses edge functions for coverage testing and proper perspective-correct interpolation.
//!
//! Reference: https://www.scratchapixel.com/lessons/3d-basic-rendering/rasterization-practical-implementation

use super::framebuffer::{rgb, FRAMEBUFFER};
use super::zbuffer::ZBUFFER;
use renderer::vertex::Vertex;

/// Edge function: determines which side of an edge a point lies on.
/// Returns positive if point c is to the left of edge a->b (counter-clockwise)
/// Returns negative if point c is to the right of edge a->b (clockwise)
/// Returns zero if point c is on the edge
#[inline]
fn edge_function(ax: f32, ay: f32, bx: f32, by: f32, cx: f32, cy: f32) -> f32 {
    (cx - ax) * (by - ay) - (cy - ay) * (bx - ax)
}

/// Render context for fast rasterization
pub struct RenderContext {
    fb_ptr: *mut u32,
    fb_width: usize,
    fb_height: usize,
    zb_ptr: *mut f32,
}

impl RenderContext {
    /// Acquire render context with direct buffer access
    pub fn acquire() -> Option<Self> {
        let fb_guard = FRAMEBUFFER.lock();
        let zb_guard = ZBUFFER.lock();

        let fb = fb_guard.as_ref()?;
        let zb = zb_guard.as_ref()?;

        let ctx = Self {
            fb_ptr: fb.address,
            fb_width: fb.width,
            fb_height: fb.height,
            zb_ptr: zb.data.as_ptr() as *mut f32,
        };

        drop(fb_guard);
        drop(zb_guard);

        Some(ctx)
    }

    pub fn dimensions(&self) -> (usize, usize) {
        (self.fb_width, self.fb_height)
    }

    /// Fast clear using 64-bit writes
    pub fn clear(&self, color: u32) {
        let size = self.fb_width * self.fb_height;
        let color64 = (color as u64) | ((color as u64) << 32);
        let ptr64 = self.fb_ptr as *mut u64;
        let pairs = size / 2;

        unsafe {
            for i in 0..pairs {
                *ptr64.add(i) = color64;
            }
            if size & 1 != 0 {
                *self.fb_ptr.add(size - 1) = color;
            }
        }
    }

    /// Clear z-buffer to minimum depth (NEG_INFINITY = farthest when using z > test)
    pub fn clear_zbuffer(&self) {
        let size = self.fb_width * self.fb_height;
        // f32::NEG_INFINITY bit pattern: 0xFF800000
        let neg_inf_bits: u64 = 0xFF800000_FF800000;
        let ptr64 = self.zb_ptr as *mut u64;
        let pairs = size / 2;

        unsafe {
            for i in 0..pairs {
                *ptr64.add(i) = neg_inf_bits;
            }
            if size & 1 != 0 {
                *self.zb_ptr.add(size - 1) = f32::NEG_INFINITY;
            }
        }
    }
}

/// Rasterize a triangle using barycentric coordinates with proper z-buffering
pub fn rasterize_triangle_with_context(ctx: &RenderContext, v0: &Vertex, v1: &Vertex, v2: &Vertex) {
    let (fb_width, fb_height) = ctx.dimensions();
    let fb_width_i = fb_width as i32;
    let fb_height_i = fb_height as i32;

    // Get screen coordinates
    let (x0, y0, z0) = (v0.position.x, v0.position.y, v0.position.z);
    let (x1, y1, z1) = (v1.position.x, v1.position.y, v1.position.z);
    let (x2, y2, z2) = (v2.position.x, v2.position.y, v2.position.z);

    // Compute triangle bounding box
    let min_x = libm::floorf(x0.min(x1).min(x2)) as i32;
    let max_x = libm::ceilf(x0.max(x1).max(x2)) as i32;
    let min_y = libm::floorf(y0.min(y1).min(y2)) as i32;
    let max_y = libm::ceilf(y0.max(y1).max(y2)) as i32;

    // Clip to screen
    let min_x = min_x.max(0);
    let max_x = max_x.min(fb_width_i - 1);
    let min_y = min_y.max(0);
    let max_y = max_y.min(fb_height_i - 1);

    if min_x > max_x || min_y > max_y {
        return;
    }

    // Compute triangle area (2x area via edge function)
    let area = edge_function(x0, y0, x1, y1, x2, y2);

    // Skip degenerate triangles
    if area.abs() < 0.0001 {
        return;
    }

    let inv_area = 1.0 / area;

    // Get vertex colors
    let (cr0, cg0, cb0) = (v0.color.x, v0.color.y, v0.color.z);
    let (cr1, cg1, cb1) = (v1.color.x, v1.color.y, v1.color.z);
    let (cr2, cg2, cb2) = (v2.color.x, v2.color.y, v2.color.z);

    // Rasterize: iterate over all pixels in bounding box
    for py in min_y..=max_y {
        for px in min_x..=max_x {
            // Sample at pixel center
            let px_f = px as f32 + 0.5;
            let py_f = py as f32 + 0.5;

            // Compute barycentric coordinates using edge functions
            // w0 = edge opposite to v0 (edge v1->v2)
            // w1 = edge opposite to v1 (edge v2->v0)
            // w2 = edge opposite to v2 (edge v0->v1)
            let w0 = edge_function(x1, y1, x2, y2, px_f, py_f);
            let w1 = edge_function(x2, y2, x0, y0, px_f, py_f);
            let w2 = edge_function(x0, y0, x1, y1, px_f, py_f);

            // Check if pixel is inside triangle
            // For CCW triangles (positive area), all weights should be >= 0
            // For CW triangles (negative area), all weights should be <= 0
            let inside = if area > 0.0 {
                w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0
            } else {
                w0 <= 0.0 && w1 <= 0.0 && w2 <= 0.0
            };

            if inside {
                // Normalize barycentric coordinates
                let b0 = w0 * inv_area;
                let b1 = w1 * inv_area;
                let b2 = w2 * inv_area;

                // Interpolate depth (z is 1/w from perspective divide, larger = closer)
                // For depth test, we want to keep the closest pixel (largest z)
                let z = b0 * z0 + b1 * z1 + b2 * z2;

                let idx = (py as usize) * fb_width + (px as usize);

                unsafe {
                    let current_z = *ctx.zb_ptr.add(idx);
                    // Larger z = closer (since z = 1/w)
                    if z > current_z {
                        *ctx.zb_ptr.add(idx) = z;

                        // Interpolate color
                        let r = (b0 * cr0 + b1 * cr1 + b2 * cr2).clamp(0.0, 1.0);
                        let g = (b0 * cg0 + b1 * cg1 + b2 * cg2).clamp(0.0, 1.0);
                        let blue = (b0 * cb0 + b1 * cb1 + b2 * cb2).clamp(0.0, 1.0);

                        let color = rgb(
                            (r * 255.0) as u8,
                            (g * 255.0) as u8,
                            (blue * 255.0) as u8,
                        );

                        *ctx.fb_ptr.add(idx) = color;
                    }
                }
            }
        }
    }
}

/// Rasterize with automatic context acquisition
pub fn rasterize_triangle_shaded(v0: &Vertex, v1: &Vertex, v2: &Vertex) {
    if let Some(ctx) = RenderContext::acquire() {
        rasterize_triangle_with_context(&ctx, v0, v1, v2);
    }
}

/// Rasterize with flat shading (use v0's color for entire triangle)
pub fn rasterize_triangle(v0: &Vertex, v1: &Vertex, v2: &Vertex) {
    rasterize_triangle_shaded(v0, v1, v2);
}
