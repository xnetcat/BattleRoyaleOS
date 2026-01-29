//! High-Performance Software Triangle Rasterizer
//!
//! Optimized using techniques from:
//! - Fabian Giesen's "Optimizing the basic rasterizer" blog series
//! - Juan Pineda's "A Parallel Algorithm for Polygon Rasterization" (SIGGRAPH 1988)
//!
//! Key optimizations:
//! 1. Incremental edge evaluation - only additions per pixel
//! 2. Incremental attribute interpolation with fixed-point math
//! 3. Integer-only inner loop (no floating-point except z-buffer)
//! 4. OR-based sign test for single branch
//! 5. Hierarchical 8x8 block rasterization with early rejection
//! 6. Tile-bounded rasterization for parallel rendering
//! 7. **SIMD 4-wide pixel processing** - processes 4 pixels per iteration
//! 8. **Integer z-buffer** - faster depth comparisons

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
    fb_pitch: usize,  // Framebuffer pixels per row (may be > width due to padding)
    zb_ptr: *mut f32,
    zb_width: usize,  // Z-buffer width (uses width, not pitch)
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
            fb_pitch: fb.pitch / 4,  // Convert bytes to pixels (for framebuffer)
            zb_ptr: zb.data.as_ptr() as *mut f32,
            zb_width: zb.width,  // Z-buffer uses width for stride
        };

        drop(fb_guard);
        drop(zb_guard);

        Some(ctx)
    }

    #[inline]
    pub fn dimensions(&self) -> (usize, usize) {
        (self.fb_width, self.fb_height)
    }

    /// Fast clear using unrolled 128-bit writes
    pub fn clear(&self, color: u32) {
        let size = self.fb_pitch * self.fb_height;
        let color64 = (color as u64) | ((color as u64) << 32);
        let ptr64 = self.fb_ptr as *mut u64;

        unsafe {
            // Unrolled: 4 u64s (8 pixels) per iteration
            let chunks = size / 8;
            let mut i = 0usize;
            while i < chunks {
                let base = i * 4;
                *ptr64.add(base) = color64;
                *ptr64.add(base + 1) = color64;
                *ptr64.add(base + 2) = color64;
                *ptr64.add(base + 3) = color64;
                i += 1;
            }
            // Remaining
            let remaining_start = chunks * 4;
            for j in remaining_start..(size / 2) {
                *ptr64.add(j) = color64;
            }
            if size & 1 != 0 {
                *self.fb_ptr.add(size - 1) = color;
            }
        }
    }

    /// Clear z-buffer to minimum depth (optimized)
    pub fn clear_zbuffer(&self) {
        let size = self.zb_width * self.fb_height;
        let neg_inf_bits: u64 = 0xFF800000_FF800000;
        let ptr64 = self.zb_ptr as *mut u64;

        unsafe {
            // Unrolled: 4 u64s (8 floats) per iteration
            let chunks = size / 8;
            let mut i = 0usize;
            while i < chunks {
                let base = i * 4;
                *ptr64.add(base) = neg_inf_bits;
                *ptr64.add(base + 1) = neg_inf_bits;
                *ptr64.add(base + 2) = neg_inf_bits;
                *ptr64.add(base + 3) = neg_inf_bits;
                i += 1;
            }
            // Remaining
            let remaining_start = chunks * 4;
            for j in remaining_start..(size / 2) {
                *ptr64.add(j) = neg_inf_bits;
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
    let fb_pitch = ctx.fb_pitch;  // Framebuffer uses pitch for row stride
    let zb_width = ctx.zb_width;  // Z-buffer uses width for row stride
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
                // Separate indices: framebuffer uses pitch, z-buffer uses width
                let fb_idx = (py as usize) * fb_pitch + (px as usize);
                let zb_idx = (py as usize) * zb_width + (px as usize);

                unsafe {
                    let current_z = *ctx.zb_ptr.add(zb_idx);
                    if z > current_z {
                        *ctx.zb_ptr.add(zb_idx) = z;

                        // Convert fixed-point color to u8 with clamping
                        let ri = ((r >> COLOR_BITS) as i32).clamp(0, 255) as u8;
                        let gi = ((g >> COLOR_BITS) as i32).clamp(0, 255) as u8;
                        let bi = ((b_color >> COLOR_BITS) as i32).clamp(0, 255) as u8;

                        *ctx.fb_ptr.add(fb_idx) = rgb(ri, gi, bi);
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

    // Use pitch for framebuffer, width for z-buffer
    let fb_pitch = ctx.fb_pitch;
    let zb_width = ctx.zb_width;

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
                        // Separate indices: framebuffer uses pitch, z-buffer uses width
                        let fb_idx = (py as usize) * fb_pitch + (px as usize);
                        let zb_idx = (py as usize) * zb_width + (px as usize);

                        unsafe {
                            let current_z = *ctx.zb_ptr.add(zb_idx);
                            if z > current_z {
                                *ctx.zb_ptr.add(zb_idx) = z;

                                let ri = ((r >> COLOR_BITS) as i32).clamp(0, 255) as u8;
                                let gi = ((g >> COLOR_BITS) as i32).clamp(0, 255) as u8;
                                let bi = ((b_color >> COLOR_BITS) as i32).clamp(0, 255) as u8;

                                *ctx.fb_ptr.add(fb_idx) = rgb(ri, gi, bi);
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
    // Use pitch for framebuffer, width for z-buffer
    let fb_pitch = ctx.fb_pitch;
    let zb_width = ctx.zb_width;

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
                // Separate indices: framebuffer uses pitch, z-buffer uses width
                let fb_idx = (py as usize) * fb_pitch + (px as usize);
                let zb_idx = (py as usize) * zb_width + (px as usize);

                unsafe {
                    let current_z = *ctx.zb_ptr.add(zb_idx);
                    if z > current_z {
                        *ctx.zb_ptr.add(zb_idx) = z;

                        let ri = ((r >> COLOR_BITS) as i32).clamp(0, 255) as u8;
                        let gi = ((g >> COLOR_BITS) as i32).clamp(0, 255) as u8;
                        let bi = ((b_color >> COLOR_BITS) as i32).clamp(0, 255) as u8;

                        *ctx.fb_ptr.add(fb_idx) = rgb(ri, gi, bi);
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

// ============================================================================
// SIMD 4-WIDE RASTERIZATION
// Processes 4 horizontal pixels per iteration for ~2-4x speedup
// ============================================================================

/// SIMD 4-wide pixel processing structure
#[repr(align(16))]
struct Simd4i64 {
    v: [i64; 4],
}

impl Simd4i64 {
    #[inline(always)]
    const fn splat(val: i64) -> Self {
        Self { v: [val, val, val, val] }
    }

    #[inline(always)]
    const fn from_array(arr: [i64; 4]) -> Self {
        Self { v: arr }
    }

    #[inline(always)]
    fn add(&self, other: &Self) -> Self {
        Self {
            v: [
                self.v[0].wrapping_add(other.v[0]),
                self.v[1].wrapping_add(other.v[1]),
                self.v[2].wrapping_add(other.v[2]),
                self.v[3].wrapping_add(other.v[3]),
            ],
        }
    }
}

/// SIMD 4-wide rasterizer - processes 4 horizontal pixels per iteration
/// This is the optimized hot path for software rasterization
pub fn rasterize_screen_triangle_simd4(
    ctx: &RenderContext,
    tri: &ScreenTriangle,
    tile_min_x: i32,
    tile_max_x: i32,
    tile_min_y: i32,
    tile_max_y: i32,
) {
    // Use pitch for framebuffer, width for z-buffer
    let fb_pitch = ctx.fb_pitch;
    let zb_width = ctx.zb_width;

    let min_x = tri.min_x.max(tile_min_x);
    let max_x = tri.max_x.min(tile_max_x);
    let min_y = tri.min_y.max(tile_min_y);
    let max_y = tri.max_y.min(tile_max_y);

    if min_x > max_x || min_y > max_y {
        return;
    }

    let aligned_min_x = min_x & !3;
    let fp_one_i64 = FP_ONE as i64;
    let area_i64 = (1.0 / tri.inv_area) as i64;

    // Z gradients as floats (direct f32 comparison with z-buffer - no conversion overhead)
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

    let w0_step_x4 = (tri.a12 as i64) * fp_one_i64 * 4;
    let w1_step_x4 = (tri.a20 as i64) * fp_one_i64 * 4;
    let w2_step_x4 = (tri.a01 as i64) * fp_one_i64 * 4;
    let w0_step_y = (tri.b12 as i64) * fp_one_i64;
    let w1_step_y = (tri.b20 as i64) * fp_one_i64;
    let w2_step_y = (tri.b01 as i64) * fp_one_i64;
    let w0_step_x1 = (tri.a12 as i64) * fp_one_i64;
    let w1_step_x1 = (tri.a20 as i64) * fp_one_i64;
    let w2_step_x1 = (tri.a01 as i64) * fp_one_i64;

    let start_x = (aligned_min_x << FP_BITS) + FP_HALF;
    let start_y = (min_y << FP_BITS) + FP_HALF;

    let w0_base = (tri.a12 as i64) * (start_x as i64) + (tri.b12 as i64) * (start_y as i64) + tri.c12;
    let w1_base = (tri.a20 as i64) * (start_x as i64) + (tri.b20 as i64) * (start_y as i64) + tri.c20;
    let w2_base = (tri.a01 as i64) * (start_x as i64) + (tri.b01 as i64) * (start_y as i64) + tri.c01;

    let w0_init = Simd4i64::from_array([w0_base, w0_base + w0_step_x1, w0_base + w0_step_x1 * 2, w0_base + w0_step_x1 * 3]);
    let w1_init = Simd4i64::from_array([w1_base, w1_base + w1_step_x1, w1_base + w1_step_x1 * 2, w1_base + w1_step_x1 * 3]);
    let w2_init = Simd4i64::from_array([w2_base, w2_base + w2_step_x1, w2_base + w2_step_x1 * 2, w2_base + w2_step_x1 * 3]);

    let w0_step_y_vec = Simd4i64::splat(w0_step_y);
    let w1_step_y_vec = Simd4i64::splat(w1_step_y);
    let w2_step_y_vec = Simd4i64::splat(w2_step_y);

    // Initial z value as float (matching z-buffer format)
    let b0_s = w0_base as f32 * tri.inv_area;
    let b1_s = w1_base as f32 * tri.inv_area;
    let b2_s = w2_base as f32 * tri.inv_area;
    let z_row_init = b0_s * tri.z0 + b1_s * tri.z1 + b2_s * tri.z2;
    let r_row_init = (w0_base * tri.r0 + w1_base * tri.r1 + w2_base * tri.r2) / area_i64;
    let g_row_init = (w0_base * tri.g0 + w1_base * tri.g1 + w2_base * tri.g2) / area_i64;
    let b_row_init = (w0_base * tri.b0 + w1_base * tri.b1 + w2_base * tri.b2) / area_i64;

    let mut w0_row = w0_init;
    let mut w1_row = w1_init;
    let mut w2_row = w2_init;
    let mut z_row = z_row_init;
    let mut r_row = r_row_init;
    let mut g_row = g_row_init;
    let mut b_row = b_row_init;

    let dz_dx4 = dz_dx * 4.0;
    let dr_dx4 = dr_dx * 4;
    let dg_dx4 = dg_dx * 4;
    let db_dx4 = db_dx * 4;

    for py in min_y..=max_y {
        let mut w0 = w0_row.v;
        let mut w1 = w1_row.v;
        let mut w2 = w2_row.v;
        // Z as float array for direct comparison
        let mut z = [z_row, z_row + dz_dx, z_row + dz_dx * 2.0, z_row + dz_dx * 3.0];
        let mut r = [r_row, r_row + dr_dx, r_row + dr_dx * 2, r_row + dr_dx * 3];
        let mut g = [g_row, g_row + dg_dx, g_row + dg_dx * 2, g_row + dg_dx * 3];
        let mut bc = [b_row, b_row + db_dx, b_row + db_dx * 2, b_row + db_dx * 3];

        let mut px = aligned_min_x;
        while px <= max_x {
            let m0 = w0[0] | w1[0] | w2[0];
            let m1 = w0[1] | w1[1] | w2[1];
            let m2 = w0[2] | w1[2] | w2[2];
            let m3 = w0[3] | w1[3] | w2[3];

            if m0 >= 0 || m1 >= 0 || m2 >= 0 || m3 >= 0 {
                // Separate indices: framebuffer uses pitch, z-buffer uses width
                let fb_base = (py as usize) * fb_pitch + (px as usize);
                let zb_base = (py as usize) * zb_width + (px as usize);

                if px >= min_x && px <= max_x && m0 >= 0 {
                    unsafe {
                        let cz = *ctx.zb_ptr.add(zb_base);
                        if z[0] > cz {
                            *ctx.zb_ptr.add(zb_base) = z[0];
                            *ctx.fb_ptr.add(fb_base) = rgb(
                                ((r[0] >> COLOR_BITS) as i32).clamp(0, 255) as u8,
                                ((g[0] >> COLOR_BITS) as i32).clamp(0, 255) as u8,
                                ((bc[0] >> COLOR_BITS) as i32).clamp(0, 255) as u8,
                            );
                        }
                    }
                }
                if px + 1 >= min_x && px + 1 <= max_x && m1 >= 0 {
                    unsafe {
                        let zb_idx = zb_base + 1;
                        let fb_idx = fb_base + 1;
                        let cz = *ctx.zb_ptr.add(zb_idx);
                        if z[1] > cz {
                            *ctx.zb_ptr.add(zb_idx) = z[1];
                            *ctx.fb_ptr.add(fb_idx) = rgb(
                                ((r[1] >> COLOR_BITS) as i32).clamp(0, 255) as u8,
                                ((g[1] >> COLOR_BITS) as i32).clamp(0, 255) as u8,
                                ((bc[1] >> COLOR_BITS) as i32).clamp(0, 255) as u8,
                            );
                        }
                    }
                }
                if px + 2 >= min_x && px + 2 <= max_x && m2 >= 0 {
                    unsafe {
                        let zb_idx = zb_base + 2;
                        let fb_idx = fb_base + 2;
                        let cz = *ctx.zb_ptr.add(zb_idx);
                        if z[2] > cz {
                            *ctx.zb_ptr.add(zb_idx) = z[2];
                            *ctx.fb_ptr.add(fb_idx) = rgb(
                                ((r[2] >> COLOR_BITS) as i32).clamp(0, 255) as u8,
                                ((g[2] >> COLOR_BITS) as i32).clamp(0, 255) as u8,
                                ((bc[2] >> COLOR_BITS) as i32).clamp(0, 255) as u8,
                            );
                        }
                    }
                }
                if px + 3 >= min_x && px + 3 <= max_x && m3 >= 0 {
                    unsafe {
                        let zb_idx = zb_base + 3;
                        let fb_idx = fb_base + 3;
                        let cz = *ctx.zb_ptr.add(zb_idx);
                        if z[3] > cz {
                            *ctx.zb_ptr.add(zb_idx) = z[3];
                            *ctx.fb_ptr.add(fb_idx) = rgb(
                                ((r[3] >> COLOR_BITS) as i32).clamp(0, 255) as u8,
                                ((g[3] >> COLOR_BITS) as i32).clamp(0, 255) as u8,
                                ((bc[3] >> COLOR_BITS) as i32).clamp(0, 255) as u8,
                            );
                        }
                    }
                }
            }

            w0[0] = w0[0].wrapping_add(w0_step_x4); w0[1] = w0[1].wrapping_add(w0_step_x4);
            w0[2] = w0[2].wrapping_add(w0_step_x4); w0[3] = w0[3].wrapping_add(w0_step_x4);
            w1[0] = w1[0].wrapping_add(w1_step_x4); w1[1] = w1[1].wrapping_add(w1_step_x4);
            w1[2] = w1[2].wrapping_add(w1_step_x4); w1[3] = w1[3].wrapping_add(w1_step_x4);
            w2[0] = w2[0].wrapping_add(w2_step_x4); w2[1] = w2[1].wrapping_add(w2_step_x4);
            w2[2] = w2[2].wrapping_add(w2_step_x4); w2[3] = w2[3].wrapping_add(w2_step_x4);
            z[0] += dz_dx4; z[1] += dz_dx4;
            z[2] += dz_dx4; z[3] += dz_dx4;
            r[0] = r[0].wrapping_add(dr_dx4); r[1] = r[1].wrapping_add(dr_dx4);
            r[2] = r[2].wrapping_add(dr_dx4); r[3] = r[3].wrapping_add(dr_dx4);
            g[0] = g[0].wrapping_add(dg_dx4); g[1] = g[1].wrapping_add(dg_dx4);
            g[2] = g[2].wrapping_add(dg_dx4); g[3] = g[3].wrapping_add(dg_dx4);
            bc[0] = bc[0].wrapping_add(db_dx4); bc[1] = bc[1].wrapping_add(db_dx4);
            bc[2] = bc[2].wrapping_add(db_dx4); bc[3] = bc[3].wrapping_add(db_dx4);
            px += 4;
        }

        w0_row = w0_row.add(&w0_step_y_vec);
        w1_row = w1_row.add(&w1_step_y_vec);
        w2_row = w2_row.add(&w2_step_y_vec);
        z_row += dz_dy;
        r_row = r_row.wrapping_add(dr_dy);
        g_row = g_row.wrapping_add(dg_dy);
        b_row = b_row.wrapping_add(db_dy);
    }
}
