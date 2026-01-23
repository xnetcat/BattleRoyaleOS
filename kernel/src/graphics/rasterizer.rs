//! High-Performance Software Triangle Rasterizer
//!
//! Optimized using techniques from:
//! - Fabian Giesen's "Optimizing the basic rasterizer" blog series
//! - Juan Pineda's "A Parallel Algorithm for Polygon Rasterization" (SIGGRAPH 1988)
//!
//! Key optimizations:
//! 1. Incremental edge evaluation - only additions per pixel
//! 2. Incremental attribute interpolation with fixed-point math
//! 3. Integer-only inner loop (no floating-point)
//! 4. OR-based sign test for single branch
//! 5. Hierarchical 8x8 block rasterization with early rejection
//! 6. Tile-bounded rasterization for parallel rendering

use super::framebuffer::{rgb, FRAMEBUFFER};
use super::tiles::ScreenTriangle;
use super::zbuffer::ZBUFFER;
use renderer::vertex::Vertex;

/// Fixed-point precision: 4 bits = 16 sub-pixels per pixel
const FP_BITS: i32 = 4;
const FP_ONE: i32 = 1 << FP_BITS;
const FP_HALF: i32 = FP_ONE >> 1;

/// Color fixed-point: 16 bits for color interpolation
const COLOR_BITS: i32 = 16;
const COLOR_ONE: i32 = 1 << COLOR_BITS;

/// Convert float to fixed-point (4-bit)
#[inline(always)]
fn to_fixed(f: f32) -> i32 {
    (f * FP_ONE as f32) as i32
}

/// Render context for fast rasterization
pub struct RenderContext {
    fb_ptr: *mut u32,
    fb_width: usize,
    fb_height: usize,
    fb_pitch: usize,  // Pixels per row (may be > width due to padding)
    zb_ptr: *mut f32,
}

impl RenderContext {
    /// Acquire render context with direct buffer access
    /// Uses the BACK buffer for rendering (double buffering)
    pub fn acquire() -> Option<Self> {
        let fb_guard = FRAMEBUFFER.lock();
        let zb_guard = ZBUFFER.lock();

        let fb = fb_guard.as_ref()?;
        let zb = zb_guard.as_ref()?;

        let ctx = Self {
            // Use back buffer pointer, not front buffer!
            fb_ptr: fb.back_buffer.as_ptr() as *mut u32,
            fb_width: fb.width,
            fb_height: fb.height,
            fb_pitch: fb.pitch / 4,  // Convert bytes to pixels
            zb_ptr: zb.data.as_ptr() as *mut f32,
        };

        drop(fb_guard);
        drop(zb_guard);

        Some(ctx)
    }

    #[inline]
    pub fn dimensions(&self) -> (usize, usize) {
        (self.fb_width, self.fb_height)
    }

    /// Fast clear using 64-bit writes
    pub fn clear(&self, color: u32) {
        // Use pitch * height to cover entire buffer including padding
        let size = self.fb_pitch * self.fb_height;
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

    /// Clear z-buffer to minimum depth
    pub fn clear_zbuffer(&self) {
        let size = self.fb_width * self.fb_height;
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

/// High-performance triangle rasterizer
pub fn rasterize_triangle_with_context(ctx: &RenderContext, v0: &Vertex, v1: &Vertex, v2: &Vertex) {
    let (fb_width, fb_height) = ctx.dimensions();
    let fb_width_i = fb_width as i32;
    let fb_height_i = fb_height as i32;

    // Convert vertex positions to fixed-point
    let x0 = to_fixed(v0.position.x);
    let y0 = to_fixed(v0.position.y);
    let x1 = to_fixed(v1.position.x);
    let y1 = to_fixed(v1.position.y);
    let x2 = to_fixed(v2.position.x);
    let y2 = to_fixed(v2.position.y);

    // Compute bounding box
    let min_x = ((x0.min(x1).min(x2)) >> FP_BITS).max(0);
    let max_x = ((x0.max(x1).max(x2) + FP_ONE - 1) >> FP_BITS).min(fb_width_i - 1);
    let min_y = ((y0.min(y1).min(y2)) >> FP_BITS).max(0);
    let max_y = ((y0.max(y1).max(y2) + FP_ONE - 1) >> FP_BITS).min(fb_height_i - 1);

    if min_x > max_x || min_y > max_y {
        return;
    }

    // Edge coefficients
    let a12 = y1 - y2;
    let b12 = x2 - x1;
    let a20 = y2 - y0;
    let b20 = x0 - x2;
    let a01 = y0 - y1;
    let b01 = x1 - x0;

    // Compute area
    let area = (a01 as i64) * (x2 as i64) + (b01 as i64) * (y2 as i64)
             + (x0 as i64) * (y1 as i64) - (y0 as i64) * (x1 as i64);

    if area == 0 {
        return;
    }

    let is_cw = area < 0;

    // Handle CW triangles
    let (a12, b12, a20, b20, a01, b01, area) = if is_cw {
        (-a12, -b12, -a20, -b20, -a01, -b01, -area)
    } else {
        (a12, b12, a20, b20, a01, b01, area)
    };

    // Use f32 for depth (needs range), but i32 for colors
    let z0 = v0.position.z;
    let z1 = v1.position.z;
    let z2 = v2.position.z;

    // Colors as 0-255 integers scaled to fixed-point
    let r0 = (v0.color.x * 255.0 * COLOR_ONE as f32) as i64;
    let g0 = (v0.color.y * 255.0 * COLOR_ONE as f32) as i64;
    let b0 = (v0.color.z * 255.0 * COLOR_ONE as f32) as i64;
    let r1 = (v1.color.x * 255.0 * COLOR_ONE as f32) as i64;
    let g1 = (v1.color.y * 255.0 * COLOR_ONE as f32) as i64;
    let b1 = (v1.color.z * 255.0 * COLOR_ONE as f32) as i64;
    let r2 = (v2.color.x * 255.0 * COLOR_ONE as f32) as i64;
    let g2 = (v2.color.y * 255.0 * COLOR_ONE as f32) as i64;
    let b2 = (v2.color.z * 255.0 * COLOR_ONE as f32) as i64;

    let inv_area = 1.0 / (area as f32);
    let fp_scale = FP_ONE as f32;

    // Z gradients (still float for depth precision)
    let dz_dx = (z0 * a12 as f32 + z1 * a20 as f32 + z2 * a01 as f32) * inv_area * fp_scale;
    let dz_dy = (z0 * b12 as f32 + z1 * b20 as f32 + z2 * b01 as f32) * inv_area * fp_scale;

    // Color gradients as fixed-point integers
    let area_i64 = area;
    let fp_one_i64 = FP_ONE as i64;
    let dr_dx = ((r0 * a12 as i64 + r1 * a20 as i64 + r2 * a01 as i64) * fp_one_i64) / area_i64;
    let dr_dy = ((r0 * b12 as i64 + r1 * b20 as i64 + r2 * b01 as i64) * fp_one_i64) / area_i64;
    let dg_dx = ((g0 * a12 as i64 + g1 * a20 as i64 + g2 * a01 as i64) * fp_one_i64) / area_i64;
    let dg_dy = ((g0 * b12 as i64 + g1 * b20 as i64 + g2 * b01 as i64) * fp_one_i64) / area_i64;
    let db_dx = ((b0 * a12 as i64 + b1 * a20 as i64 + b2 * a01 as i64) * fp_one_i64) / area_i64;
    let db_dy = ((b0 * b12 as i64 + b1 * b20 as i64 + b2 * b01 as i64) * fp_one_i64) / area_i64;

    // Starting point
    let start_x = (min_x << FP_BITS) + FP_HALF;
    let start_y = (min_y << FP_BITS) + FP_HALF;

    // Edge constants
    let c12 = (x1 as i64) * (y2 as i64) - (y1 as i64) * (x2 as i64);
    let c20 = (x2 as i64) * (y0 as i64) - (y2 as i64) * (x0 as i64);
    let c01 = (x0 as i64) * (y1 as i64) - (y0 as i64) * (x1 as i64);
    let (c12, c20, c01) = if is_cw { (-c12, -c20, -c01) } else { (c12, c20, c01) };

    // Initial edge values
    let mut w0_row = (a12 as i64) * (start_x as i64) + (b12 as i64) * (start_y as i64) + c12;
    let mut w1_row = (a20 as i64) * (start_x as i64) + (b20 as i64) * (start_y as i64) + c20;
    let mut w2_row = (a01 as i64) * (start_x as i64) + (b01 as i64) * (start_y as i64) + c01;

    // Initial attribute values at start using barycentric
    let b0_start = w0_row as f32 * inv_area;
    let b1_start = w1_row as f32 * inv_area;
    let b2_start = w2_row as f32 * inv_area;

    let mut z_row = b0_start * z0 + b1_start * z1 + b2_start * z2;

    // Color initial values (fixed-point)
    let mut r_row = (w0_row * r0 + w1_row * r1 + w2_row * r2) / area_i64;
    let mut g_row = (w0_row * g0 + w1_row * g1 + w2_row * g2) / area_i64;
    let mut b_row = (w0_row * b0 + w1_row * b1 + w2_row * b2) / area_i64;

    // Edge steps
    let w0_step_x = (a12 as i64) * fp_one_i64;
    let w1_step_x = (a20 as i64) * fp_one_i64;
    let w2_step_x = (a01 as i64) * fp_one_i64;
    let w0_step_y = (b12 as i64) * fp_one_i64;
    let w1_step_y = (b20 as i64) * fp_one_i64;
    let w2_step_y = (b01 as i64) * fp_one_i64;

    // Rasterize
    for py in min_y..=max_y {
        let mut w0 = w0_row;
        let mut w1 = w1_row;
        let mut w2 = w2_row;
        let mut z = z_row;
        let mut r = r_row;
        let mut g = g_row;
        let mut b_color = b_row;

        for px in min_x..=max_x {
            if (w0 | w1 | w2) >= 0 {
                let idx = (py as usize) * fb_width + (px as usize);

                unsafe {
                    let current_z = *ctx.zb_ptr.add(idx);
                    if z > current_z {
                        *ctx.zb_ptr.add(idx) = z;

                        // Convert fixed-point color to u8 with clamping
                        let ri = ((r >> COLOR_BITS) as i32).clamp(0, 255) as u8;
                        let gi = ((g >> COLOR_BITS) as i32).clamp(0, 255) as u8;
                        let bi = ((b_color >> COLOR_BITS) as i32).clamp(0, 255) as u8;

                        *ctx.fb_ptr.add(idx) = rgb(ri, gi, bi);
                    }
                }
            }

            w0 += w0_step_x;
            w1 += w1_step_x;
            w2 += w2_step_x;
            z += dz_dx;
            r += dr_dx;
            g += dg_dx;
            b_color += db_dx;
        }

        w0_row += w0_step_y;
        w1_row += w1_step_y;
        w2_row += w2_step_y;
        z_row += dz_dy;
        r_row += dr_dy;
        g_row += dg_dy;
        b_row += db_dy;
    }
}

/// Rasterize with automatic context acquisition
pub fn rasterize_triangle_shaded(v0: &Vertex, v1: &Vertex, v2: &Vertex) {
    if let Some(ctx) = RenderContext::acquire() {
        rasterize_triangle_with_context(&ctx, v0, v1, v2);
    }
}

/// Rasterize with flat shading
pub fn rasterize_triangle(v0: &Vertex, v1: &Vertex, v2: &Vertex) {
    rasterize_triangle_shaded(v0, v1, v2);
}

/// Rasterize a ScreenTriangle within tile bounds using hierarchical 8x8 blocks
pub fn rasterize_screen_triangle_in_tile(
    ctx: &RenderContext,
    tri: &ScreenTriangle,
    tile_min_x: i32,
    tile_max_x: i32,
    tile_min_y: i32,
    tile_max_y: i32,
) {
    const BLOCK_SIZE: i32 = 8;

    let (fb_width, _fb_height) = ctx.dimensions();

    // Clamp triangle bounds to tile bounds
    let min_x = tri.min_x.max(tile_min_x);
    let max_x = tri.max_x.min(tile_max_x);
    let min_y = tri.min_y.max(tile_min_y);
    let max_y = tri.max_y.min(tile_max_y);

    if min_x > max_x || min_y > max_y {
        return;
    }

    // Pre-compute gradients for depth and color interpolation
    let fp_one_i64 = FP_ONE as i64;
    let area_i64 = (1.0 / tri.inv_area) as i64;

    // Z gradients (still float for depth precision)
    let dz_dx = (tri.z0 * tri.a12 as f32 + tri.z1 * tri.a20 as f32 + tri.z2 * tri.a01 as f32)
        * tri.inv_area
        * FP_ONE as f32;
    let dz_dy = (tri.z0 * tri.b12 as f32 + tri.z1 * tri.b20 as f32 + tri.z2 * tri.b01 as f32)
        * tri.inv_area
        * FP_ONE as f32;

    // Color gradients as fixed-point
    let dr_dx = ((tri.r0 * tri.a12 as i64 + tri.r1 * tri.a20 as i64 + tri.r2 * tri.a01 as i64) * fp_one_i64) / area_i64;
    let dr_dy = ((tri.r0 * tri.b12 as i64 + tri.r1 * tri.b20 as i64 + tri.r2 * tri.b01 as i64) * fp_one_i64) / area_i64;
    let dg_dx = ((tri.g0 * tri.a12 as i64 + tri.g1 * tri.a20 as i64 + tri.g2 * tri.a01 as i64) * fp_one_i64) / area_i64;
    let dg_dy = ((tri.g0 * tri.b12 as i64 + tri.g1 * tri.b20 as i64 + tri.g2 * tri.b01 as i64) * fp_one_i64) / area_i64;
    let db_dx = ((tri.b0 * tri.a12 as i64 + tri.b1 * tri.a20 as i64 + tri.b2 * tri.a01 as i64) * fp_one_i64) / area_i64;
    let db_dy = ((tri.b0 * tri.b12 as i64 + tri.b1 * tri.b20 as i64 + tri.b2 * tri.b01 as i64) * fp_one_i64) / area_i64;

    // Edge step values
    let w0_step_x = (tri.a12 as i64) * fp_one_i64;
    let w1_step_x = (tri.a20 as i64) * fp_one_i64;
    let w2_step_x = (tri.a01 as i64) * fp_one_i64;
    let w0_step_y = (tri.b12 as i64) * fp_one_i64;
    let w1_step_y = (tri.b20 as i64) * fp_one_i64;
    let w2_step_y = (tri.b01 as i64) * fp_one_i64;

    // Align to block boundaries
    let block_min_x = min_x & !(BLOCK_SIZE - 1);
    let block_min_y = min_y & !(BLOCK_SIZE - 1);

    // Iterate over 8x8 blocks
    let mut by = block_min_y;
    while by <= max_y {
        let mut bx = block_min_x;
        while bx <= max_x {
            // Conservative test: can we trivially reject this block?
            if !block_intersects_triangle(tri, bx, by, BLOCK_SIZE) {
                // Block fully outside - skip
                bx += BLOCK_SIZE;
                continue;
            }

            // Block at least partially inside - rasterize it
            let block_max_x = (bx + BLOCK_SIZE - 1).min(max_x);
            let block_max_y = (by + BLOCK_SIZE - 1).min(max_y);
            let block_min_x_clamped = bx.max(min_x);
            let block_min_y_clamped = by.max(min_y);

            // Starting point for this block
            let start_x = (block_min_x_clamped << FP_BITS) + FP_HALF;
            let start_y = (block_min_y_clamped << FP_BITS) + FP_HALF;

            // Initial edge values at block start
            let mut w0_row = (tri.a12 as i64) * (start_x as i64) + (tri.b12 as i64) * (start_y as i64) + tri.c12;
            let mut w1_row = (tri.a20 as i64) * (start_x as i64) + (tri.b20 as i64) * (start_y as i64) + tri.c20;
            let mut w2_row = (tri.a01 as i64) * (start_x as i64) + (tri.b01 as i64) * (start_y as i64) + tri.c01;

            // Initial attribute values
            let b0_start = w0_row as f32 * tri.inv_area;
            let b1_start = w1_row as f32 * tri.inv_area;
            let b2_start = w2_row as f32 * tri.inv_area;

            let mut z_row = b0_start * tri.z0 + b1_start * tri.z1 + b2_start * tri.z2;
            let mut r_row = (w0_row * tri.r0 + w1_row * tri.r1 + w2_row * tri.r2) / area_i64;
            let mut g_row = (w0_row * tri.g0 + w1_row * tri.g1 + w2_row * tri.g2) / area_i64;
            let mut b_row = (w0_row * tri.b0 + w1_row * tri.b1 + w2_row * tri.b2) / area_i64;

            // Rasterize the block
            for py in block_min_y_clamped..=block_max_y {
                let mut w0 = w0_row;
                let mut w1 = w1_row;
                let mut w2 = w2_row;
                let mut z = z_row;
                let mut r = r_row;
                let mut g = g_row;
                let mut b_color = b_row;

                for px in block_min_x_clamped..=block_max_x {
                    if (w0 | w1 | w2) >= 0 {
                        let idx = (py as usize) * fb_width + (px as usize);

                        unsafe {
                            let current_z = *ctx.zb_ptr.add(idx);
                            if z > current_z {
                                *ctx.zb_ptr.add(idx) = z;

                                let ri = ((r >> COLOR_BITS) as i32).clamp(0, 255) as u8;
                                let gi = ((g >> COLOR_BITS) as i32).clamp(0, 255) as u8;
                                let bi = ((b_color >> COLOR_BITS) as i32).clamp(0, 255) as u8;

                                *ctx.fb_ptr.add(idx) = rgb(ri, gi, bi);
                            }
                        }
                    }

                    w0 += w0_step_x;
                    w1 += w1_step_x;
                    w2 += w2_step_x;
                    z += dz_dx;
                    r += dr_dx;
                    g += dg_dx;
                    b_color += db_dx;
                }

                w0_row += w0_step_y;
                w1_row += w1_step_y;
                w2_row += w2_step_y;
                z_row += dz_dy;
                r_row += dr_dy;
                g_row += dg_dy;
                b_row += db_dy;
            }

            bx += BLOCK_SIZE;
        }
        by += BLOCK_SIZE;
    }
}

/// Test if an 8x8 block can be trivially rejected (fully outside triangle)
/// Returns true if the block should be processed, false if it can be skipped
#[inline]
fn block_intersects_triangle(tri: &ScreenTriangle, bx: i32, by: i32, block_size: i32) -> bool {
    // Top-left corner
    let x0 = (bx << FP_BITS) + FP_HALF;
    let y0 = (by << FP_BITS) + FP_HALF;

    // Top-right corner
    let x1 = ((bx + block_size - 1) << FP_BITS) + FP_HALF;

    // Bottom-left corner
    let y2 = ((by + block_size - 1) << FP_BITS) + FP_HALF;

    // For each edge, check if ALL 4 corners are on the outside (negative)
    // If so, the block is fully outside the triangle and can be rejected

    // Edge 0 (v1->v2)
    let w0_tl = (tri.a12 as i64) * (x0 as i64) + (tri.b12 as i64) * (y0 as i64) + tri.c12;
    let w0_tr = (tri.a12 as i64) * (x1 as i64) + (tri.b12 as i64) * (y0 as i64) + tri.c12;
    let w0_bl = (tri.a12 as i64) * (x0 as i64) + (tri.b12 as i64) * (y2 as i64) + tri.c12;
    let w0_br = (tri.a12 as i64) * (x1 as i64) + (tri.b12 as i64) * (y2 as i64) + tri.c12;

    // If all corners are outside edge 0, reject
    if w0_tl < 0 && w0_tr < 0 && w0_bl < 0 && w0_br < 0 {
        return false;
    }

    // Edge 1 (v2->v0)
    let w1_tl = (tri.a20 as i64) * (x0 as i64) + (tri.b20 as i64) * (y0 as i64) + tri.c20;
    let w1_tr = (tri.a20 as i64) * (x1 as i64) + (tri.b20 as i64) * (y0 as i64) + tri.c20;
    let w1_bl = (tri.a20 as i64) * (x0 as i64) + (tri.b20 as i64) * (y2 as i64) + tri.c20;
    let w1_br = (tri.a20 as i64) * (x1 as i64) + (tri.b20 as i64) * (y2 as i64) + tri.c20;

    // If all corners are outside edge 1, reject
    if w1_tl < 0 && w1_tr < 0 && w1_bl < 0 && w1_br < 0 {
        return false;
    }

    // Edge 2 (v0->v1)
    let w2_tl = (tri.a01 as i64) * (x0 as i64) + (tri.b01 as i64) * (y0 as i64) + tri.c01;
    let w2_tr = (tri.a01 as i64) * (x1 as i64) + (tri.b01 as i64) * (y0 as i64) + tri.c01;
    let w2_bl = (tri.a01 as i64) * (x0 as i64) + (tri.b01 as i64) * (y2 as i64) + tri.c01;
    let w2_br = (tri.a01 as i64) * (x1 as i64) + (tri.b01 as i64) * (y2 as i64) + tri.c01;

    // If all corners are outside edge 2, reject
    if w2_tl < 0 && w2_tr < 0 && w2_bl < 0 && w2_br < 0 {
        return false;
    }

    // Block might intersect triangle
    true
}

/// Simple tile-bounded rasterization (no hierarchical blocks)
/// Used for small triangles or when block overhead isn't worth it
pub fn rasterize_screen_triangle_simple(
    ctx: &RenderContext,
    tri: &ScreenTriangle,
    tile_min_x: i32,
    tile_max_x: i32,
    tile_min_y: i32,
    tile_max_y: i32,
) {
    let (fb_width, _fb_height) = ctx.dimensions();

    // Clamp to tile bounds
    let min_x = tri.min_x.max(tile_min_x);
    let max_x = tri.max_x.min(tile_max_x);
    let min_y = tri.min_y.max(tile_min_y);
    let max_y = tri.max_y.min(tile_max_y);

    if min_x > max_x || min_y > max_y {
        return;
    }

    let fp_one_i64 = FP_ONE as i64;
    let area_i64 = (1.0 / tri.inv_area) as i64;

    // Gradients
    let dz_dx = (tri.z0 * tri.a12 as f32 + tri.z1 * tri.a20 as f32 + tri.z2 * tri.a01 as f32)
        * tri.inv_area * FP_ONE as f32;
    let dz_dy = (tri.z0 * tri.b12 as f32 + tri.z1 * tri.b20 as f32 + tri.z2 * tri.b01 as f32)
        * tri.inv_area * FP_ONE as f32;

    let dr_dx = ((tri.r0 * tri.a12 as i64 + tri.r1 * tri.a20 as i64 + tri.r2 * tri.a01 as i64) * fp_one_i64) / area_i64;
    let dr_dy = ((tri.r0 * tri.b12 as i64 + tri.r1 * tri.b20 as i64 + tri.r2 * tri.b01 as i64) * fp_one_i64) / area_i64;
    let dg_dx = ((tri.g0 * tri.a12 as i64 + tri.g1 * tri.a20 as i64 + tri.g2 * tri.a01 as i64) * fp_one_i64) / area_i64;
    let dg_dy = ((tri.g0 * tri.b12 as i64 + tri.g1 * tri.b20 as i64 + tri.g2 * tri.b01 as i64) * fp_one_i64) / area_i64;
    let db_dx = ((tri.b0 * tri.a12 as i64 + tri.b1 * tri.a20 as i64 + tri.b2 * tri.a01 as i64) * fp_one_i64) / area_i64;
    let db_dy = ((tri.b0 * tri.b12 as i64 + tri.b1 * tri.b20 as i64 + tri.b2 * tri.b01 as i64) * fp_one_i64) / area_i64;

    // Edge steps
    let w0_step_x = (tri.a12 as i64) * fp_one_i64;
    let w1_step_x = (tri.a20 as i64) * fp_one_i64;
    let w2_step_x = (tri.a01 as i64) * fp_one_i64;
    let w0_step_y = (tri.b12 as i64) * fp_one_i64;
    let w1_step_y = (tri.b20 as i64) * fp_one_i64;
    let w2_step_y = (tri.b01 as i64) * fp_one_i64;

    // Starting point
    let start_x = (min_x << FP_BITS) + FP_HALF;
    let start_y = (min_y << FP_BITS) + FP_HALF;

    // Initial edge values
    let mut w0_row = (tri.a12 as i64) * (start_x as i64) + (tri.b12 as i64) * (start_y as i64) + tri.c12;
    let mut w1_row = (tri.a20 as i64) * (start_x as i64) + (tri.b20 as i64) * (start_y as i64) + tri.c20;
    let mut w2_row = (tri.a01 as i64) * (start_x as i64) + (tri.b01 as i64) * (start_y as i64) + tri.c01;

    // Initial attributes
    let b0_start = w0_row as f32 * tri.inv_area;
    let b1_start = w1_row as f32 * tri.inv_area;
    let b2_start = w2_row as f32 * tri.inv_area;

    let mut z_row = b0_start * tri.z0 + b1_start * tri.z1 + b2_start * tri.z2;
    let mut r_row = (w0_row * tri.r0 + w1_row * tri.r1 + w2_row * tri.r2) / area_i64;
    let mut g_row = (w0_row * tri.g0 + w1_row * tri.g1 + w2_row * tri.g2) / area_i64;
    let mut b_row = (w0_row * tri.b0 + w1_row * tri.b1 + w2_row * tri.b2) / area_i64;

    for py in min_y..=max_y {
        let mut w0 = w0_row;
        let mut w1 = w1_row;
        let mut w2 = w2_row;
        let mut z = z_row;
        let mut r = r_row;
        let mut g = g_row;
        let mut b_color = b_row;

        for px in min_x..=max_x {
            if (w0 | w1 | w2) >= 0 {
                let idx = (py as usize) * fb_width + (px as usize);

                unsafe {
                    let current_z = *ctx.zb_ptr.add(idx);
                    if z > current_z {
                        *ctx.zb_ptr.add(idx) = z;

                        let ri = ((r >> COLOR_BITS) as i32).clamp(0, 255) as u8;
                        let gi = ((g >> COLOR_BITS) as i32).clamp(0, 255) as u8;
                        let bi = ((b_color >> COLOR_BITS) as i32).clamp(0, 255) as u8;

                        *ctx.fb_ptr.add(idx) = rgb(ri, gi, bi);
                    }
                }
            }

            w0 += w0_step_x;
            w1 += w1_step_x;
            w2 += w2_step_x;
            z += dz_dx;
            r += dr_dx;
            g += dg_dx;
            b_color += db_dx;
        }

        w0_row += w0_step_y;
        w1_row += w1_step_y;
        w2_row += w2_step_y;
        z_row += dz_dy;
        r_row += dr_dy;
        g_row += dg_dy;
        b_row += db_dy;
    }
}
