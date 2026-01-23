//! Triangle rasterization
//!
//! Optimized software rasterizer using scanline algorithm with z-buffering.
//! Uses direct memory access during single-threaded rendering for performance.

use super::framebuffer::{rgb, Framebuffer, FRAMEBUFFER};
use super::zbuffer::{ZBuffer, ZBUFFER};
use renderer::vertex::Vertex;

/// Render context for fast rasterization
/// Holds raw pointers to framebuffer and z-buffer for direct access
pub struct RenderContext {
    fb_ptr: *mut u32,
    fb_width: usize,
    fb_height: usize,
    zb_ptr: *mut f32,
}

impl RenderContext {
    /// Create a new render context by acquiring framebuffer and z-buffer
    /// SAFETY: Only call from main rendering thread. Context must be dropped
    /// before any other code accesses the framebuffer or z-buffer.
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

        // Drop the guards - we're using raw pointers now
        drop(fb_guard);
        drop(zb_guard);

        Some(ctx)
    }

    /// Get framebuffer dimensions
    #[inline]
    pub fn dimensions(&self) -> (usize, usize) {
        (self.fb_width, self.fb_height)
    }

    /// Put a pixel at (x, y) with color if z-test passes
    /// Uses inverted depth: larger z = closer to camera
    #[inline]
    pub fn put_pixel_with_z(&self, x: usize, y: usize, z: f32, color: u32) {
        if x >= self.fb_width || y >= self.fb_height {
            return;
        }

        let idx = y * self.fb_width + x;
        unsafe {
            let z_ptr = self.zb_ptr.add(idx);
            let current_z = *z_ptr;
            // Larger z = closer (reversed depth buffer)
            if z > current_z {
                *z_ptr = z;
                let fb_ptr = self.fb_ptr.add(idx);
                *fb_ptr = color;
            }
        }
    }

    /// Clear the framebuffer with a color (optimized 64-bit writes)
    pub fn clear(&self, color: u32) {
        let size = self.fb_width * self.fb_height;
        // Write two pixels at a time using 64-bit stores
        let color64 = (color as u64) | ((color as u64) << 32);
        let ptr64 = self.fb_ptr as *mut u64;
        let pairs = size / 2;

        unsafe {
            for i in 0..pairs {
                *ptr64.add(i) = color64;
            }
            // Handle odd pixel if present
            if size & 1 != 0 {
                *self.fb_ptr.add(size - 1) = color;
            }
        }
    }

    /// Clear the z-buffer (optimized 64-bit writes)
    /// Uses negative infinity since larger z = closer
    pub fn clear_zbuffer(&self) {
        let size = self.fb_width * self.fb_height;
        // Write two f32 values at a time using 64-bit stores
        // NEG_INFINITY bit pattern: 0xFF800000
        let neg_inf_bits: u64 = 0xFF800000_FF800000;
        let ptr64 = self.zb_ptr as *mut u64;
        let pairs = size / 2;

        unsafe {
            for i in 0..pairs {
                *ptr64.add(i) = neg_inf_bits;
            }
            // Handle odd element if present
            if size & 1 != 0 {
                *self.zb_ptr.add(size - 1) = f32::NEG_INFINITY;
            }
        }
    }
}

// RenderContext doesn't implement Send/Sync - it's for single-threaded use only
// This is intentional for performance

/// Rasterize a triangle with per-vertex colors (Gouraud shading)
pub fn rasterize_triangle_shaded(v0: &Vertex, v1: &Vertex, v2: &Vertex) {
    let ctx = match RenderContext::acquire() {
        Some(c) => c,
        None => return,
    };

    rasterize_triangle_with_context(&ctx, v0, v1, v2);
}

/// Rasterize a triangle using a pre-acquired render context
/// This is faster when rendering multiple triangles
#[inline]
pub fn rasterize_triangle_with_context(ctx: &RenderContext, v0: &Vertex, v1: &Vertex, v2: &Vertex) {
    // Sort vertices by Y coordinate
    let (mut v0, mut v1, mut v2) = (v0.clone(), v1.clone(), v2.clone());
    if v1.position.y < v0.position.y {
        core::mem::swap(&mut v0, &mut v1);
    }
    if v2.position.y < v0.position.y {
        core::mem::swap(&mut v0, &mut v2);
    }
    if v2.position.y < v1.position.y {
        core::mem::swap(&mut v1, &mut v2);
    }

    let total_height = v2.position.y - v0.position.y;
    if total_height < 0.001 {
        return;
    }

    let (fb_width, fb_height) = ctx.dimensions();
    let y_min = (v0.position.y as i32).max(0);
    let y_max = (v2.position.y as i32).min(fb_height as i32 - 1);

    for y in y_min..=y_max {
        let y_f = y as f32;
        let second_half = y_f > v1.position.y || (v1.position.y - v0.position.y).abs() < 0.001;
        let segment_height = if second_half {
            v2.position.y - v1.position.y
        } else {
            v1.position.y - v0.position.y
        };

        if segment_height.abs() < 0.001 {
            continue;
        }

        let alpha = (y_f - v0.position.y) / total_height;
        let beta = if second_half {
            (y_f - v1.position.y) / segment_height
        } else {
            (y_f - v0.position.y) / segment_height
        };

        // Interpolate vertices
        let mut va = lerp_vertex(&v0, &v2, alpha);
        let mut vb = if second_half {
            lerp_vertex(&v1, &v2, beta)
        } else {
            lerp_vertex(&v0, &v1, beta)
        };

        if va.position.x > vb.position.x {
            core::mem::swap(&mut va, &mut vb);
        }

        let x_start = (va.position.x as i32).max(0) as usize;
        let x_end = (vb.position.x as i32).min(fb_width as i32 - 1) as usize;

        if x_start >= x_end {
            continue;
        }

        let dx = vb.position.x - va.position.x;
        if dx.abs() < 0.001 {
            continue;
        }

        // Precompute for scanline
        let inv_dx = 1.0 / dx;
        let dz = (vb.position.z - va.position.z) * inv_dx;
        let dr = (vb.color.x - va.color.x) * inv_dx;
        let dg = (vb.color.y - va.color.y) * inv_dx;
        let db = (vb.color.z - va.color.z) * inv_dx;

        let mut z = va.position.z + (x_start as f32 - va.position.x) * dz;
        let mut r = va.color.x + (x_start as f32 - va.position.x) * dr;
        let mut g = va.color.y + (x_start as f32 - va.position.x) * dg;
        let mut b = va.color.z + (x_start as f32 - va.position.x) * db;

        for x in x_start..=x_end {
            let color = rgb(
                (r * 255.0).clamp(0.0, 255.0) as u8,
                (g * 255.0).clamp(0.0, 255.0) as u8,
                (b * 255.0).clamp(0.0, 255.0) as u8,
            );

            ctx.put_pixel_with_z(x, y as usize, z, color);

            z += dz;
            r += dr;
            g += dg;
            b += db;
        }
    }
}

/// Linearly interpolate between two vertices
#[inline]
fn lerp_vertex(a: &Vertex, b: &Vertex, t: f32) -> Vertex {
    Vertex {
        position: glam::Vec3::new(
            a.position.x + (b.position.x - a.position.x) * t,
            a.position.y + (b.position.y - a.position.y) * t,
            a.position.z + (b.position.z - a.position.z) * t,
        ),
        normal: a.normal.lerp(b.normal, t),
        color: glam::Vec3::new(
            a.color.x + (b.color.x - a.color.x) * t,
            a.color.y + (b.color.y - a.color.y) * t,
            a.color.z + (b.color.z - a.color.z) * t,
        ),
        uv: a.uv.lerp(b.uv, t),
    }
}

/// Rasterize a triangle with flat color (no interpolation - faster)
pub fn rasterize_triangle(v0: &Vertex, v1: &Vertex, v2: &Vertex) {
    let ctx = match RenderContext::acquire() {
        Some(c) => c,
        None => return,
    };

    let color = rgb(
        (v0.color.x * 255.0).clamp(0.0, 255.0) as u8,
        (v0.color.y * 255.0).clamp(0.0, 255.0) as u8,
        (v0.color.z * 255.0).clamp(0.0, 255.0) as u8,
    );

    // Sort vertices by Y coordinate
    let (mut v0, mut v1, mut v2) = (v0.clone(), v1.clone(), v2.clone());
    if v1.position.y < v0.position.y {
        core::mem::swap(&mut v0, &mut v1);
    }
    if v2.position.y < v0.position.y {
        core::mem::swap(&mut v0, &mut v2);
    }
    if v2.position.y < v1.position.y {
        core::mem::swap(&mut v1, &mut v2);
    }

    let (x0, y0, z0) = (v0.position.x, v0.position.y, v0.position.z);
    let (x1, y1, z1) = (v1.position.x, v1.position.y, v1.position.z);
    let (x2, y2, z2) = (v2.position.x, v2.position.y, v2.position.z);

    let total_height = y2 - y0;
    if total_height < 0.001 {
        return;
    }

    let (fb_width, fb_height) = ctx.dimensions();
    let y_min = (y0 as i32).max(0);
    let y_max = (y2 as i32).min(fb_height as i32 - 1);

    for y in y_min..=y_max {
        let y_f = y as f32;
        let second_half = y_f > y1 || (y1 - y0).abs() < 0.001;
        let segment_height = if second_half { y2 - y1 } else { y1 - y0 };

        if segment_height.abs() < 0.001 {
            continue;
        }

        let alpha = (y_f - y0) / total_height;
        let beta = if second_half {
            (y_f - y1) / segment_height
        } else {
            (y_f - y0) / segment_height
        };

        let mut xa = x0 + (x2 - x0) * alpha;
        let mut xb = if second_half {
            x1 + (x2 - x1) * beta
        } else {
            x0 + (x1 - x0) * beta
        };

        let mut za = z0 + (z2 - z0) * alpha;
        let mut zb = if second_half {
            z1 + (z2 - z1) * beta
        } else {
            z0 + (z1 - z0) * beta
        };

        if xa > xb {
            core::mem::swap(&mut xa, &mut xb);
            core::mem::swap(&mut za, &mut zb);
        }

        let x_start = (xa as i32).max(0) as usize;
        let x_end = (xb as i32).min(fb_width as i32 - 1) as usize;

        if x_start >= x_end {
            continue;
        }

        let dx = xb - xa;
        if dx.abs() < 0.001 {
            continue;
        }

        let dz = (zb - za) / dx;
        let mut z = za + (x_start as f32 - xa) * dz;

        for x in x_start..=x_end {
            ctx.put_pixel_with_z(x, y as usize, z, color);
            z += dz;
        }
    }
}

/// Convert vertex color to packed RGB
#[inline]
pub fn vertex_color(v: &Vertex) -> u32 {
    let r = (v.color.x * 255.0).clamp(0.0, 255.0) as u8;
    let g = (v.color.y * 255.0).clamp(0.0, 255.0) as u8;
    let b = (v.color.z * 255.0).clamp(0.0, 255.0) as u8;
    rgb(r, g, b)
}

/// Draw a wireframe triangle (for debugging)
pub fn draw_triangle_wireframe(v0: &Vertex, v1: &Vertex, v2: &Vertex, color: u32) {
    draw_line(
        v0.position.x as i32,
        v0.position.y as i32,
        v1.position.x as i32,
        v1.position.y as i32,
        color,
    );
    draw_line(
        v1.position.x as i32,
        v1.position.y as i32,
        v2.position.x as i32,
        v2.position.y as i32,
        color,
    );
    draw_line(
        v2.position.x as i32,
        v2.position.y as i32,
        v0.position.x as i32,
        v0.position.y as i32,
        color,
    );
}

/// Draw a line using Bresenham's algorithm
pub fn draw_line(x0: i32, y0: i32, x1: i32, y1: i32, color: u32) {
    let fb_guard = FRAMEBUFFER.lock();
    let fb = match fb_guard.as_ref() {
        Some(f) => f,
        None => return,
    };

    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    let mut x = x0;
    let mut y = y0;

    loop {
        if x >= 0 && x < fb.width as i32 && y >= 0 && y < fb.height as i32 {
            fb.put_pixel(x as usize, y as usize, color);
        }

        if x == x1 && y == y1 {
            break;
        }

        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}
