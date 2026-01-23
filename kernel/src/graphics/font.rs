//! Extended 8x8 bitmap font for text rendering
//!
//! Supports full alphabet (A-Z), digits (0-9), and common punctuation.

use super::framebuffer::FRAMEBUFFER;

/// 8x8 bitmap font data for digits, letters, and punctuation
/// Each character is 8 bytes, one byte per row
/// Total: 48 glyphs
static FONT_DATA: [[u8; 8]; 48] = [
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
    // A (10)
    [0x18, 0x3C, 0x66, 0x66, 0x7E, 0x66, 0x66, 0x00],
    // B (11)
    [0x7C, 0x66, 0x66, 0x7C, 0x66, 0x66, 0x7C, 0x00],
    // C (12)
    [0x3C, 0x66, 0x60, 0x60, 0x60, 0x66, 0x3C, 0x00],
    // D (13)
    [0x78, 0x6C, 0x66, 0x66, 0x66, 0x6C, 0x78, 0x00],
    // E (14)
    [0x7E, 0x60, 0x60, 0x7C, 0x60, 0x60, 0x7E, 0x00],
    // F (15)
    [0x7E, 0x60, 0x60, 0x7C, 0x60, 0x60, 0x60, 0x00],
    // G (16)
    [0x3C, 0x66, 0x60, 0x6E, 0x66, 0x66, 0x3E, 0x00],
    // H (17)
    [0x66, 0x66, 0x66, 0x7E, 0x66, 0x66, 0x66, 0x00],
    // I (18)
    [0x3C, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00],
    // J (19)
    [0x1E, 0x0C, 0x0C, 0x0C, 0x0C, 0x6C, 0x38, 0x00],
    // K (20)
    [0x66, 0x6C, 0x78, 0x70, 0x78, 0x6C, 0x66, 0x00],
    // L (21)
    [0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x7E, 0x00],
    // M (22)
    [0x63, 0x77, 0x7F, 0x6B, 0x63, 0x63, 0x63, 0x00],
    // N (23)
    [0x66, 0x76, 0x7E, 0x7E, 0x6E, 0x66, 0x66, 0x00],
    // O (24)
    [0x3C, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00],
    // P (25)
    [0x7C, 0x66, 0x66, 0x7C, 0x60, 0x60, 0x60, 0x00],
    // Q (26)
    [0x3C, 0x66, 0x66, 0x66, 0x6A, 0x6C, 0x36, 0x00],
    // R (27)
    [0x7C, 0x66, 0x66, 0x7C, 0x6C, 0x66, 0x66, 0x00],
    // S (28)
    [0x3C, 0x66, 0x60, 0x3C, 0x06, 0x66, 0x3C, 0x00],
    // T (29)
    [0x7E, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00],
    // U (30)
    [0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00],
    // V (31)
    [0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x18, 0x00],
    // W (32)
    [0x63, 0x63, 0x63, 0x6B, 0x7F, 0x77, 0x63, 0x00],
    // X (33)
    [0x66, 0x66, 0x3C, 0x18, 0x3C, 0x66, 0x66, 0x00],
    // Y (34)
    [0x66, 0x66, 0x66, 0x3C, 0x18, 0x18, 0x18, 0x00],
    // Z (35)
    [0x7E, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x7E, 0x00],
    // : (36)
    [0x00, 0x18, 0x18, 0x00, 0x18, 0x18, 0x00, 0x00],
    // space (37)
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    // . (38)
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00],
    // / (39)
    [0x02, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x40, 0x00],
    // - (40)
    [0x00, 0x00, 0x00, 0x7E, 0x00, 0x00, 0x00, 0x00],
    // _ (41)
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7E, 0x00],
    // ! (42)
    [0x18, 0x18, 0x18, 0x18, 0x18, 0x00, 0x18, 0x00],
    // ? (43)
    [0x3C, 0x66, 0x06, 0x0C, 0x18, 0x00, 0x18, 0x00],
    // ( (44)
    [0x0C, 0x18, 0x30, 0x30, 0x30, 0x18, 0x0C, 0x00],
    // ) (45)
    [0x30, 0x18, 0x0C, 0x0C, 0x0C, 0x18, 0x30, 0x00],
    // # (46)
    [0x24, 0x24, 0x7E, 0x24, 0x7E, 0x24, 0x24, 0x00],
    // > (47)
    [0x30, 0x18, 0x0C, 0x06, 0x0C, 0x18, 0x30, 0x00],
];

/// Get glyph index for a character
fn char_to_glyph(c: char) -> usize {
    match c {
        '0'..='9' => (c as usize) - ('0' as usize),
        'A' | 'a' => 10,
        'B' | 'b' => 11,
        'C' | 'c' => 12,
        'D' | 'd' => 13,
        'E' | 'e' => 14,
        'F' | 'f' => 15,
        'G' | 'g' => 16,
        'H' | 'h' => 17,
        'I' | 'i' => 18,
        'J' | 'j' => 19,
        'K' | 'k' => 20,
        'L' | 'l' => 21,
        'M' | 'm' => 22,
        'N' | 'n' => 23,
        'O' | 'o' => 24,
        'P' | 'p' => 25,
        'Q' | 'q' => 26,
        'R' | 'r' => 27,
        'S' | 's' => 28,
        'T' | 't' => 29,
        'U' | 'u' => 30,
        'V' | 'v' => 31,
        'W' | 'w' => 32,
        'X' | 'x' => 33,
        'Y' | 'y' => 34,
        'Z' | 'z' => 35,
        ':' => 36,
        ' ' => 37,
        '.' => 38,
        '/' => 39,
        '-' => 40,
        '_' => 41,
        '!' => 42,
        '?' => 43,
        '(' => 44,
        ')' => 45,
        '#' => 46,
        '>' => 47,
        _ => 37, // Default to space
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

/// Draw a character without holding the framebuffer lock (for batch drawing)
/// Caller must ensure fb is valid
pub fn draw_char_raw(fb: &super::framebuffer::Framebuffer, x: usize, y: usize, c: char, color: u32, scale: usize) {
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

/// Draw a string without holding the framebuffer lock (for batch drawing)
pub fn draw_string_raw(fb: &super::framebuffer::Framebuffer, x: usize, y: usize, s: &str, color: u32, scale: usize) {
    let mut cx = x;
    for c in s.chars() {
        draw_char_raw(fb, cx, y, c, color, scale);
        cx += 8 * scale + scale; // Character width + spacing
    }
}

/// Get the pixel width of a string at a given scale
pub fn string_width(s: &str, scale: usize) -> usize {
    if s.is_empty() {
        return 0;
    }
    let char_count = s.chars().count();
    char_count * (8 * scale) + (char_count - 1) * scale
}

/// Get the pixel height at a given scale
pub fn char_height(scale: usize) -> usize {
    8 * scale
}

/// Draw a centered string
pub fn draw_string_centered(y: usize, s: &str, color: u32, scale: usize, fb_width: usize) {
    let text_width = string_width(s, scale);
    let x = if text_width >= fb_width {
        0
    } else {
        (fb_width - text_width) / 2
    };
    draw_string(x, y, s, color, scale);
}

/// Draw a centered string without holding the framebuffer lock
pub fn draw_string_centered_raw(fb: &super::framebuffer::Framebuffer, y: usize, s: &str, color: u32, scale: usize) {
    let text_width = string_width(s, scale);
    let x = if text_width >= fb.width {
        0
    } else {
        (fb.width - text_width) / 2
    };
    draw_string_raw(fb, x, y, s, color, scale);
}

/// Draw FPS counter in top-left corner with solid background
/// Uses a larger, more visible format
pub fn draw_fps(fps: u32, _fb_width: usize) {
    // Format: "FPS: XXX"
    let mut buf = [0u8; 12];
    let s = format_fps(fps, &mut buf);

    let scale = 3; // Larger scale for visibility
    let char_width = 8 * scale + scale;
    let text_width = s.len() * char_width;
    let x = 10; // Top-left corner for visibility
    let y = 10;

    // Draw solid background rectangle first
    let bg_color = 0x00202040u32; // Dark blue-gray, matches clear color
    let fb_guard = FRAMEBUFFER.lock();
    if let Some(fb) = fb_guard.as_ref() {
        let padding = 6;
        let y_start = if y >= padding { y - padding } else { 0 };
        let y_end = (y + 8 * scale + padding).min(fb.height);
        let x_start = if x >= padding { x - padding } else { 0 };
        let x_end = (x + text_width + padding).min(fb.width);
        for py in y_start..y_end {
            for px in x_start..x_end {
                fb.put_pixel(px, py, bg_color);
            }
        }
    }
    drop(fb_guard);

    let color = 0x00FFFF00; // Yellow for maximum visibility
    draw_string(x, y, s, color, scale);
}

/// Draw game HUD (health, materials, alive count)
pub fn draw_hud(health: u8, materials: u32, alive: usize, total: usize, _fb_width: usize, fb_height: usize) {
    let scale = 2;
    let char_width = 8 * scale + scale;
    let line_height = 8 * scale + 8;
    let padding = 10;

    // Bottom-left corner for HUD
    let base_y = fb_height - padding - line_height * 3;

    // Draw background
    let bg_color = 0x00202040u32;
    let fb_guard = FRAMEBUFFER.lock();
    if let Some(fb) = fb_guard.as_ref() {
        let bg_width = char_width * 12;
        let bg_height = line_height * 3 + padding;
        for py in base_y.saturating_sub(padding)..(base_y + bg_height).min(fb.height) {
            for px in 0..(bg_width + padding * 2).min(fb.width) {
                fb.put_pixel(px, py, bg_color);
            }
        }
    }
    drop(fb_guard);

    // Health (red/green based on value)
    let health_color = if health > 50 {
        0x0000FF00 // Green
    } else if health > 25 {
        0x00FFFF00 // Yellow
    } else {
        0x00FF0000 // Red
    };
    let mut buf = [0u8; 16];
    let health_str = format_stat("HP", health as u32, &mut buf);
    draw_string(padding, base_y, health_str, health_color, scale);

    // Materials (orange)
    let mut buf2 = [0u8; 16];
    let mat_str = format_stat("MAT", materials, &mut buf2);
    draw_string(padding, base_y + line_height, mat_str, 0x00FFA500, scale);

    // Alive count (white)
    let mut buf3 = [0u8; 16];
    let alive_str = format_alive(alive, total, &mut buf3);
    draw_string(padding, base_y + line_height * 2, alive_str, 0x00FFFFFF, scale);
}

/// Format a stat line like "HP: 100"
fn format_stat<'a>(label: &str, value: u32, buf: &'a mut [u8; 16]) -> &'a str {
    let mut pos = 0;
    for c in label.bytes() {
        if pos < 16 {
            buf[pos] = c;
            pos += 1;
        }
    }
    if pos < 16 {
        buf[pos] = b':';
        pos += 1;
    }
    if pos < 16 {
        buf[pos] = b' ';
        pos += 1;
    }

    // Write number
    if value == 0 {
        if pos < 16 {
            buf[pos] = b'0';
            pos += 1;
        }
    } else {
        let mut n = value;
        let start = pos;
        while n > 0 && pos < 16 {
            buf[pos] = b'0' + (n % 10) as u8;
            n /= 10;
            pos += 1;
        }
        // Reverse the digits
        buf[start..pos].reverse();
    }

    unsafe { core::str::from_utf8_unchecked(&buf[..pos]) }
}

/// Format alive count like "50/100"
fn format_alive<'a>(alive: usize, total: usize, buf: &'a mut [u8; 16]) -> &'a str {
    let mut pos = 0;

    // Write number (alive)
    if alive == 0 {
        buf[pos] = b'0';
        pos += 1;
    } else {
        let mut n = alive;
        let start = pos;
        while n > 0 && pos < 16 {
            buf[pos] = b'0' + (n % 10) as u8;
            n /= 10;
            pos += 1;
        }
        buf[start..pos].reverse();
    }

    buf[pos] = b'/';
    pos += 1;

    // Write total
    if total == 0 {
        buf[pos] = b'0';
        pos += 1;
    } else {
        let mut n = total;
        let start = pos;
        while n > 0 && pos < 16 {
            buf[pos] = b'0' + (n % 10) as u8;
            n /= 10;
            pos += 1;
        }
        buf[start..pos].reverse();
    }

    unsafe { core::str::from_utf8_unchecked(&buf[..pos]) }
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

/// Format a number into a buffer, returns the slice used
pub fn format_number(value: u32, buf: &mut [u8]) -> &str {
    if buf.is_empty() {
        return "";
    }

    if value == 0 {
        buf[0] = b'0';
        return unsafe { core::str::from_utf8_unchecked(&buf[..1]) };
    }

    let mut n = value;
    let mut pos = 0;
    while n > 0 && pos < buf.len() {
        buf[pos] = b'0' + (n % 10) as u8;
        n /= 10;
        pos += 1;
    }
    buf[..pos].reverse();

    unsafe { core::str::from_utf8_unchecked(&buf[..pos]) }
}
