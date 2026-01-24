//! Mouse cursor rendering
//!
//! Provides a simple arrow cursor for UI interaction.

use crate::graphics::framebuffer::Framebuffer;

/// Simple arrow cursor bitmap (12x16 pixels)
/// 0 = transparent, 1 = black outline, 2 = white fill
const CURSOR_DATA: [[u8; 12]; 16] = [
    [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [1, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [1, 2, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0],
    [1, 2, 2, 2, 1, 0, 0, 0, 0, 0, 0, 0],
    [1, 2, 2, 2, 2, 1, 0, 0, 0, 0, 0, 0],
    [1, 2, 2, 2, 2, 2, 1, 0, 0, 0, 0, 0],
    [1, 2, 2, 2, 2, 2, 2, 1, 0, 0, 0, 0],
    [1, 2, 2, 2, 2, 2, 2, 2, 1, 0, 0, 0],
    [1, 2, 2, 2, 2, 2, 2, 2, 2, 1, 0, 0],
    [1, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 0],
    [1, 2, 2, 1, 2, 2, 1, 0, 0, 0, 0, 0],
    [1, 2, 1, 0, 1, 2, 2, 1, 0, 0, 0, 0],
    [1, 1, 0, 0, 1, 2, 2, 1, 0, 0, 0, 0],
    [1, 0, 0, 0, 0, 1, 2, 2, 1, 0, 0, 0],
    [0, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0],
];

/// Cursor width in pixels
pub const CURSOR_WIDTH: usize = 12;

/// Cursor height in pixels
pub const CURSOR_HEIGHT: usize = 16;

/// Draw the mouse cursor at the given position
///
/// The cursor hotspot is at (0, 0) - the top-left corner.
pub fn draw_cursor(fb: &Framebuffer, x: i32, y: i32) {
    for (dy, row) in CURSOR_DATA.iter().enumerate() {
        for (dx, &pixel) in row.iter().enumerate() {
            if pixel == 0 {
                continue; // Transparent
            }

            let px = x + dx as i32;
            let py = y + dy as i32;

            // Bounds check
            if px < 0 || py < 0 || px >= fb.width as i32 || py >= fb.height as i32 {
                continue;
            }

            let color = if pixel == 1 {
                0x000000 // Black outline
            } else {
                0xFFFFFF // White fill
            };

            fb.put_pixel(px as usize, py as usize, color);
        }
    }
}

/// Draw the mouse cursor with a custom outline and fill color
pub fn draw_cursor_colored(fb: &Framebuffer, x: i32, y: i32, outline: u32, fill: u32) {
    for (dy, row) in CURSOR_DATA.iter().enumerate() {
        for (dx, &pixel) in row.iter().enumerate() {
            if pixel == 0 {
                continue;
            }

            let px = x + dx as i32;
            let py = y + dy as i32;

            if px < 0 || py < 0 || px >= fb.width as i32 || py >= fb.height as i32 {
                continue;
            }

            let color = if pixel == 1 { outline } else { fill };
            fb.put_pixel(px as usize, py as usize, color);
        }
    }
}

/// Check if a point is within a rectangular area
pub fn point_in_rect(x: i32, y: i32, rect_x: usize, rect_y: usize, width: usize, height: usize) -> bool {
    x >= rect_x as i32
        && x < (rect_x + width) as i32
        && y >= rect_y as i32
        && y < (rect_y + height) as i32
}
