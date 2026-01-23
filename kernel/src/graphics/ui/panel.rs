//! Panel UI primitives - backgrounds and containers

use crate::graphics::framebuffer::{Framebuffer, FRAMEBUFFER};
use super::colors;

/// Draw a vertical gradient background
pub fn draw_gradient_background(fb_width: usize, fb_height: usize) {
    let fb_guard = FRAMEBUFFER.lock();
    let fb = match fb_guard.as_ref() {
        Some(f) => f,
        None => return,
    };

    draw_gradient_background_raw(fb, fb_width, fb_height);
}

/// Draw a vertical gradient background without holding the lock
pub fn draw_gradient_background_raw(fb: &Framebuffer, fb_width: usize, fb_height: usize) {
    let top_r = ((colors::BG_TOP >> 16) & 0xFF) as f32;
    let top_g = ((colors::BG_TOP >> 8) & 0xFF) as f32;
    let top_b = (colors::BG_TOP & 0xFF) as f32;

    let bot_r = ((colors::BG_BOTTOM >> 16) & 0xFF) as f32;
    let bot_g = ((colors::BG_BOTTOM >> 8) & 0xFF) as f32;
    let bot_b = (colors::BG_BOTTOM & 0xFF) as f32;

    for y in 0..fb_height.min(fb.height) {
        let t = y as f32 / fb_height as f32;

        let r = (top_r + (bot_r - top_r) * t) as u32;
        let g = (top_g + (bot_g - top_g) * t) as u32;
        let b = (top_b + (bot_b - top_b) * t) as u32;
        let color = (r << 16) | (g << 8) | b;

        for x in 0..fb_width.min(fb.width) {
            fb.put_pixel(x, y, color);
        }
    }
}

/// Draw a panel with border
pub fn draw_panel(x: usize, y: usize, width: usize, height: usize, bg_color: u32) {
    let fb_guard = FRAMEBUFFER.lock();
    let fb = match fb_guard.as_ref() {
        Some(f) => f,
        None => return,
    };

    draw_panel_raw(fb, x, y, width, height, bg_color);
}

/// Draw a panel without holding the lock
pub fn draw_panel_raw(fb: &Framebuffer, x: usize, y: usize, width: usize, height: usize, bg_color: u32) {
    let border_color = colors::PANEL_BORDER;
    let border_width = 2;

    for py in y..(y + height).min(fb.height) {
        for px in x..(x + width).min(fb.width) {
            let is_border = px < x + border_width
                || px >= x + width - border_width
                || py < y + border_width
                || py >= y + height - border_width;

            let color = if is_border { border_color } else { bg_color };
            fb.put_pixel(px, py, color);
        }
    }
}

/// Draw a rounded panel (approximated with corner pixels)
pub fn draw_rounded_panel_raw(fb: &Framebuffer, x: usize, y: usize, width: usize, height: usize, bg_color: u32, radius: usize) {
    let border_color = colors::PANEL_BORDER;

    for py in y..(y + height).min(fb.height) {
        for px in x..(x + width).min(fb.width) {
            // Check if in corner regions
            let in_tl = px < x + radius && py < y + radius;
            let in_tr = px >= x + width - radius && py < y + radius;
            let in_bl = px < x + radius && py >= y + height - radius;
            let in_br = px >= x + width - radius && py >= y + height - radius;

            let mut skip = false;

            if in_tl {
                let dx = x + radius - px;
                let dy = y + radius - py;
                if dx * dx + dy * dy > radius * radius {
                    skip = true;
                }
            }
            if in_tr {
                let dx = px - (x + width - radius - 1);
                let dy = y + radius - py;
                if dx * dx + dy * dy > radius * radius {
                    skip = true;
                }
            }
            if in_bl {
                let dx = x + radius - px;
                let dy = py - (y + height - radius - 1);
                if dx * dx + dy * dy > radius * radius {
                    skip = true;
                }
            }
            if in_br {
                let dx = px - (x + width - radius - 1);
                let dy = py - (y + height - radius - 1);
                if dx * dx + dy * dy > radius * radius {
                    skip = true;
                }
            }

            if skip {
                continue;
            }

            // Check for border
            let is_border = px < x + 2
                || px >= x + width - 2
                || py < y + 2
                || py >= y + height - 2;

            let color = if is_border { border_color } else { bg_color };
            fb.put_pixel(px, py, color);
        }
    }
}

/// Draw a horizontal divider line
pub fn draw_divider_raw(fb: &Framebuffer, x: usize, y: usize, width: usize, color: u32) {
    for px in x..(x + width).min(fb.width) {
        if y < fb.height {
            fb.put_pixel(px, y, color);
        }
        if y + 1 < fb.height {
            fb.put_pixel(px, y + 1, color);
        }
    }
}

/// Draw a progress bar
pub fn draw_progress_bar_raw(
    fb: &Framebuffer,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    progress: f32,  // 0.0 to 1.0
    fill_color: u32,
    bg_color: u32,
) {
    let border_color = colors::PANEL_BORDER;

    for py in y..(y + height).min(fb.height) {
        for px in x..(x + width).min(fb.width) {
            let is_border = px < x + 1
                || px >= x + width - 1
                || py < y + 1
                || py >= y + height - 1;

            let progress_x = x + 1 + ((width - 2) as f32 * progress.clamp(0.0, 1.0)) as usize;
            let is_filled = px >= x + 1 && px < progress_x && py >= y + 1 && py < y + height - 1;

            let color = if is_border {
                border_color
            } else if is_filled {
                fill_color
            } else {
                bg_color
            };

            fb.put_pixel(px, py, color);
        }
    }
}

/// Draw a color swatch
pub fn draw_swatch_raw(fb: &Framebuffer, x: usize, y: usize, size: usize, color: u32, selected: bool) {
    let border_color = if selected { colors::FN_YELLOW } else { colors::PANEL_BORDER };
    let border_width = if selected { 3 } else { 2 };

    for py in y..(y + size).min(fb.height) {
        for px in x..(x + size).min(fb.width) {
            let is_border = px < x + border_width
                || px >= x + size - border_width
                || py < y + border_width
                || py >= y + size - border_width;

            let c = if is_border { border_color } else { color };
            fb.put_pixel(px, py, c);
        }
    }
}

/// Draw a simple filled rectangle
pub fn fill_rect_raw(fb: &Framebuffer, x: usize, y: usize, width: usize, height: usize, color: u32) {
    for py in y..(y + height).min(fb.height) {
        for px in x..(x + width).min(fb.width) {
            fb.put_pixel(px, py, color);
        }
    }
}

/// Draw a crosshair at center of screen
pub fn draw_crosshair_raw(fb: &Framebuffer, fb_width: usize, fb_height: usize, color: u32) {
    let cx = fb_width / 2;
    let cy = fb_height / 2;
    let size = 10;
    let gap = 3;

    // Horizontal lines
    for x in (cx - size - gap)..(cx - gap) {
        if x < fb.width && cy < fb.height {
            fb.put_pixel(x, cy, color);
        }
    }
    for x in (cx + gap + 1)..(cx + size + gap + 1) {
        if x < fb.width && cy < fb.height {
            fb.put_pixel(x, cy, color);
        }
    }

    // Vertical lines
    for y in (cy - size - gap)..(cy - gap) {
        if cx < fb.width && y < fb.height {
            fb.put_pixel(cx, y, color);
        }
    }
    for y in (cy + gap + 1)..(cy + size + gap + 1) {
        if cx < fb.width && y < fb.height {
            fb.put_pixel(cx, y, color);
        }
    }
}
