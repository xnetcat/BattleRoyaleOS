//! Simple 8x8 bitmap font for text rendering

use super::framebuffer::FRAMEBUFFER;

/// 8x8 bitmap font data for digits 0-9 and a few characters
/// Each character is 8 bytes, one byte per row
static FONT_DATA: [[u8; 8]; 16] = [
    // 0
    [0x3C, 0x66, 0x6E, 0x76, 0x66, 0x66, 0x3C, 0x00],
    // 1
    [0x18, 0x38, 0x18, 0x18, 0x18, 0x18, 0x7E, 0x00],
    // 2
    [0x3C, 0x66, 0x06, 0x1C, 0x30, 0x60, 0x7E, 0x00],
    // 3
    [0x3C, 0x66, 0x06, 0x1C, 0x06, 0x66, 0x3C, 0x00],
    // 4
    [0x0E, 0x1E, 0x36, 0x66, 0x7F, 0x06, 0x06, 0x00],
    // 5
    [0x7E, 0x60, 0x7C, 0x06, 0x06, 0x66, 0x3C, 0x00],
    // 6
    [0x1C, 0x30, 0x60, 0x7C, 0x66, 0x66, 0x3C, 0x00],
    // 7
    [0x7E, 0x06, 0x0C, 0x18, 0x30, 0x30, 0x30, 0x00],
    // 8
    [0x3C, 0x66, 0x66, 0x3C, 0x66, 0x66, 0x3C, 0x00],
    // 9
    [0x3C, 0x66, 0x66, 0x3E, 0x06, 0x0C, 0x38, 0x00],
    // F
    [0x7E, 0x60, 0x60, 0x7C, 0x60, 0x60, 0x60, 0x00],
    // P
    [0x7C, 0x66, 0x66, 0x7C, 0x60, 0x60, 0x60, 0x00],
    // S
    [0x3C, 0x66, 0x60, 0x3C, 0x06, 0x66, 0x3C, 0x00],
    // : (colon)
    [0x00, 0x18, 0x18, 0x00, 0x18, 0x18, 0x00, 0x00],
    // space
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    // . (period)
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00],
];

/// Get glyph index for a character
fn char_to_glyph(c: char) -> usize {
    match c {
        '0'..='9' => (c as usize) - ('0' as usize),
        'F' | 'f' => 10,
        'P' | 'p' => 11,
        'S' | 's' => 12,
        ':' => 13,
        ' ' => 14,
        '.' => 15,
        _ => 14, // Default to space
    }
}

/// Draw a character at position (x, y) with given color
/// Scale multiplies the character size
pub fn draw_char(x: usize, y: usize, c: char, color: u32, scale: usize) {
    let fb_guard = FRAMEBUFFER.lock();
    let fb = match fb_guard.as_ref() {
        Some(f) => f,
        None => return,
    };

    let glyph = char_to_glyph(c);
    let data = &FONT_DATA[glyph];

    for row in 0..8 {
        let bits = data[row];
        for col in 0..8 {
            if bits & (0x80 >> col) != 0 {
                // Draw scaled pixel
                for sy in 0..scale {
                    for sx in 0..scale {
                        let px = x + col * scale + sx;
                        let py = y + row * scale + sy;
                        if px < fb.width && py < fb.height {
                            fb.put_pixel(px, py, color);
                        }
                    }
                }
            }
        }
    }
}

/// Draw a string at position (x, y)
pub fn draw_string(x: usize, y: usize, s: &str, color: u32, scale: usize) {
    let mut cx = x;
    for c in s.chars() {
        draw_char(cx, y, c, color, scale);
        cx += 8 * scale + scale; // Character width + spacing
    }
}

/// Draw FPS counter in top-right corner with background
pub fn draw_fps(fps: u32, fb_width: usize) {
    // Format: "FPS: XXX"
    let mut buf = [0u8; 12];
    let s = format_fps(fps, &mut buf);

    let scale = 2;
    let char_width = 8 * scale + scale;
    let text_width = s.len() * char_width;
    let x = fb_width.saturating_sub(text_width + 20);
    let y = 10;

    // Draw background rectangle
    let bg_color = 0x00000000u32; // Black
    let fb_guard = FRAMEBUFFER.lock();
    if let Some(fb) = fb_guard.as_ref() {
        let y_start = if y >= 2 { y - 2 } else { 0 };
        let y_end = y + 8 * scale + 4;
        let x_start = if x >= 4 { x - 4 } else { 0 };
        let x_end = x + text_width + 4;
        for py in y_start..y_end {
            for px in x_start..x_end {
                if px < fb.width && py < fb.height {
                    fb.put_pixel(px, py, bg_color);
                }
            }
        }
    }
    drop(fb_guard);

    let color = 0x0000FF00; // Green for visibility
    draw_string(x, y, s, color, scale);
}

/// Format FPS into a static buffer
fn format_fps(fps: u32, buf: &mut [u8; 12]) -> &str {
    buf[0] = b'F';
    buf[1] = b'P';
    buf[2] = b'S';
    buf[3] = b':';
    buf[4] = b' ';

    let mut n = fps;
    let mut len = 5;

    if n == 0 {
        buf[5] = b'0';
        len = 6;
    } else {
        // Find number of digits
        let mut temp = n;
        let mut digits = 0;
        while temp > 0 {
            digits += 1;
            temp /= 10;
        }

        // Write digits in reverse
        let mut pos = 5 + digits;
        len = pos;
        while n > 0 && pos > 5 {
            pos -= 1;
            buf[pos] = b'0' + (n % 10) as u8;
            n /= 10;
        }
    }

    // Safety: we only write ASCII characters
    unsafe { core::str::from_utf8_unchecked(&buf[..len]) }
}
