//! Triangle rasterization

use super::framebuffer::{rgb, FRAMEBUFFER};
use super::zbuffer::ZBUFFER;
use renderer::vertex::Vertex;

/// Rasterize a triangle with depth testing
pub fn rasterize_triangle(v0: &Vertex, v1: &Vertex, v2: &Vertex) {
    let mut fb_guard = FRAMEBUFFER.lock();
    let mut zb_guard = ZBUFFER.lock();

    let fb = match fb_guard.as_ref() {
        Some(f) => f,
        None => return,
    };
    let zb = match zb_guard.as_mut() {
        Some(z) => z,
        None => return,
    };

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

    let color = vertex_color(&v0);

    // Check for degenerate triangle
    let total_height = y2 - y0;
    if total_height < 0.001 {
        return;
    }

    // Rasterize the triangle using scanline algorithm
    for y in (y0 as i32).max(0)..=(y2 as i32).min(fb.height as i32 - 1) {
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

        // Calculate x coordinates for this scanline
        let mut xa = x0 + (x2 - x0) * alpha;
        let mut xb = if second_half {
            x1 + (x2 - x1) * beta
        } else {
            x0 + (x1 - x0) * beta
        };

        // Calculate z coordinates for this scanline
        let mut za = z0 + (z2 - z0) * alpha;
        let mut z_b = if second_half {
            z1 + (z2 - z1) * beta
        } else {
            z0 + (z1 - z0) * beta
        };

        if xa > xb {
            core::mem::swap(&mut xa, &mut xb);
            core::mem::swap(&mut za, &mut z_b);
        }

        let x_start = (xa as i32).max(0) as usize;
        let x_end = (xb as i32).min(fb.width as i32 - 1) as usize;

        if x_start >= x_end {
            continue;
        }

        let dx = xb - xa;
        for x in x_start..=x_end {
            let t = if dx.abs() > 0.001 {
                (x as f32 - xa) / dx
            } else {
                0.0
            };
            let z = za + (z_b - za) * t;

            if zb.test_and_set(x, y as usize, z) {
                fb.put_pixel(x, y as usize, color);
            }
        }
    }
}

/// Rasterize a triangle with per-vertex colors (Gouraud shading)
pub fn rasterize_triangle_shaded(v0: &Vertex, v1: &Vertex, v2: &Vertex) {
    let mut fb_guard = FRAMEBUFFER.lock();
    let mut zb_guard = ZBUFFER.lock();

    let fb = match fb_guard.as_ref() {
        Some(f) => f,
        None => return,
    };
    let zb = match zb_guard.as_mut() {
        Some(z) => z,
        None => return,
    };

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

    for y in (v0.position.y as i32).max(0)..=(v2.position.y as i32).min(fb.height as i32 - 1) {
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
        let mut va = v0.lerp(&v2, alpha);
        let mut vb = if second_half {
            v1.lerp(&v2, beta)
        } else {
            v0.lerp(&v1, beta)
        };

        if va.position.x > vb.position.x {
            core::mem::swap(&mut va, &mut vb);
        }

        let x_start = (va.position.x as i32).max(0) as usize;
        let x_end = (vb.position.x as i32).min(fb.width as i32 - 1) as usize;

        if x_start >= x_end {
            continue;
        }

        let dx = vb.position.x - va.position.x;
        for x in x_start..=x_end {
            let t = if dx.abs() > 0.001 {
                (x as f32 - va.position.x) / dx
            } else {
                0.0
            };

            let v = va.lerp(&vb, t);
            let z = v.position.z;

            if zb.test_and_set(x, y as usize, z) {
                let color = vertex_color(&v);
                fb.put_pixel(x, y as usize, color);
            }
        }
    }
}

/// Convert vertex color to packed RGB
#[inline]
fn vertex_color(v: &Vertex) -> u32 {
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
