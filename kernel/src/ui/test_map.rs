//! Test map / Model gallery viewer
//!
//! A debug screen for viewing all voxel models in the game.

use crate::game::state::{GameState, MenuAction};
use crate::graphics::font;
use crate::graphics::framebuffer::FRAMEBUFFER;
use crate::graphics::rasterizer::RenderContext;
use crate::graphics::ui::colors;
use crate::graphics::ui::panel::{draw_gradient_background_raw, draw_panel_raw, fill_rect_raw};

/// Model names for the gallery
const MODEL_NAMES: &[&str] = &[
    "Player",
    "Shotgun",
    "Assault Rifle",
    "Pistol",
    "SMG",
    "Sniper Rifle",
    "Pickaxe",
    "Glider (Red)",
    "Glider (Blue)",
    "Glider (Green)",
    "Glider (Orange)",
    "Pine Tree",
    "Oak Tree",
    "Rock",
    "Wall (Wood)",
    "Wall (Brick)",
    "Wall (Metal)",
    "Floor (Wood)",
    "Ramp (Wood)",
    "Battle Bus",
    "Chest",
    "Backpack (Small)",
    "Backpack (Medium)",
    "Backpack (Large)",
];

/// Model triangle counts (approximate)
const MODEL_TRI_COUNTS: &[usize] = &[
    312,   // Player
    96,    // Shotgun
    144,   // AR
    72,    // Pistol
    108,   // SMG
    192,   // Sniper
    84,    // Pickaxe
    288,   // Glider Red
    288,   // Glider Blue
    288,   // Glider Green
    288,   // Glider Orange
    432,   // Pine Tree
    576,   // Oak Tree
    96,    // Rock
    384,   // Wall Wood
    384,   // Wall Brick
    384,   // Wall Metal
    192,   // Floor Wood
    480,   // Ramp Wood
    864,   // Battle Bus
    108,   // Chest
    96,    // Backpack Small
    144,   // Backpack Medium
    192,   // Backpack Large
];

/// Model sizes (width x height x depth in voxels)
const MODEL_SIZES: &[(usize, usize, usize)] = &[
    (8, 24, 4),   // Player
    (16, 4, 3),   // Shotgun
    (20, 5, 3),   // AR
    (8, 6, 2),    // Pistol
    (14, 5, 3),   // SMG
    (24, 5, 3),   // Sniper
    (12, 16, 3),  // Pickaxe
    (24, 8, 16),  // Glider Red
    (24, 8, 16),  // Glider Blue
    (24, 8, 16),  // Glider Green
    (24, 8, 16),  // Glider Orange
    (10, 20, 10), // Pine Tree
    (12, 16, 12), // Oak Tree
    (6, 4, 5),    // Rock
    (16, 16, 2),  // Wall Wood
    (16, 16, 2),  // Wall Brick
    (16, 16, 2),  // Wall Metal
    (16, 2, 16),  // Floor Wood
    (16, 16, 16), // Ramp Wood
    (20, 16, 32), // Battle Bus
    (6, 5, 4),    // Chest
    (4, 5, 2),    // Backpack Small
    (5, 7, 3),    // Backpack Medium
    (6, 8, 4),    // Backpack Large
];

/// Test map screen state
pub struct TestMapScreen {
    /// Currently selected model index
    pub current_model: usize,
    /// Model rotation angle
    pub rotation: f32,
    /// Zoom level (1.0 = default)
    pub zoom: f32,
    /// Framebuffer dimensions
    pub fb_width: usize,
    pub fb_height: usize,
}

impl TestMapScreen {
    pub fn new(fb_width: usize, fb_height: usize) -> Self {
        Self {
            current_model: 0,
            rotation: 0.0,
            zoom: 1.0,
            fb_width,
            fb_height,
        }
    }

    /// Handle input and return new state if transitioning
    pub fn update(&mut self, action: MenuAction) -> Option<GameState> {
        match action {
            MenuAction::Left => {
                if self.current_model == 0 {
                    self.current_model = MODEL_NAMES.len() - 1;
                } else {
                    self.current_model -= 1;
                }
            }
            MenuAction::Right => {
                self.current_model = (self.current_model + 1) % MODEL_NAMES.len();
            }
            MenuAction::Up => {
                // Zoom in
                self.zoom = (self.zoom + 0.1).min(3.0);
            }
            MenuAction::Down => {
                // Zoom out
                self.zoom = (self.zoom - 0.1).max(0.3);
            }
            MenuAction::Back => {
                return Some(GameState::PartyLobby);
            }
            _ => {}
        }

        None
    }

    /// Update rotation (call each frame)
    pub fn tick(&mut self) {
        self.rotation += 0.02;
        if self.rotation > core::f32::consts::TAU {
            self.rotation -= core::f32::consts::TAU;
        }
    }

    /// Get current model index
    pub fn get_model_index(&self) -> usize {
        self.current_model
    }

    /// Get current rotation
    pub fn get_rotation(&self) -> f32 {
        self.rotation
    }

    /// Get current zoom
    pub fn get_zoom(&self) -> f32 {
        self.zoom
    }

    /// Draw the test map UI (2D overlay)
    pub fn draw(&self, _ctx: &RenderContext, fb_width: usize, fb_height: usize) {
        let fb_guard = FRAMEBUFFER.lock();
        let fb = match fb_guard.as_ref() {
            Some(f) => f,
            None => return,
        };

        // Semi-transparent header bar
        fill_rect_raw(fb, 0, 0, fb_width, 80, 0x00101020);

        // Title
        font::draw_string_centered_raw(fb, 20, "MODEL VIEWER", colors::TITLE, 3);

        // Model name with navigation arrows
        let name = MODEL_NAMES.get(self.current_model).unwrap_or(&"Unknown");
        let nav_text_scale = 2;

        // Left arrow
        font::draw_string_raw(fb, 50, 50, "<", colors::FN_YELLOW, nav_text_scale);

        // Model name centered
        font::draw_string_centered_raw(fb, 50, name, colors::WHITE, nav_text_scale);

        // Right arrow
        let right_x = fb_width - 70;
        font::draw_string_raw(fb, right_x, 50, ">", colors::FN_YELLOW, nav_text_scale);

        // Bottom info panel
        let panel_y = fb_height - 120;
        draw_panel_raw(fb, 20, panel_y, fb_width - 40, 100, colors::PANEL_BG);

        // Model info
        let tri_count = MODEL_TRI_COUNTS.get(self.current_model).unwrap_or(&0);
        let size = MODEL_SIZES.get(self.current_model).unwrap_or(&(0, 0, 0));

        let info_scale = 2;
        let line_height = 30;

        // Triangle count
        let mut tri_buf = [0u8; 32];
        let tri_str = format_model_info("Triangles: ", *tri_count, &mut tri_buf);
        font::draw_string_raw(fb, 40, panel_y + 15, tri_str, colors::WHITE, info_scale);

        // Size
        let mut size_buf = [0u8; 48];
        let size_str = format_size_info(size.0, size.1, size.2, &mut size_buf);
        font::draw_string_raw(fb, 40, panel_y + 15 + line_height, size_str, colors::WHITE, info_scale);

        // Zoom level
        let mut zoom_buf = [0u8; 24];
        let zoom_str = format_zoom_info(self.zoom, &mut zoom_buf);
        font::draw_string_raw(fb, 40, panel_y + 15 + line_height * 2, zoom_str, colors::SUBTITLE, info_scale);

        // Controls
        let controls = "[LEFT/RIGHT] Navigate  [UP/DOWN] Zoom  [ESC] Back";
        font::draw_string_centered_raw(fb, fb_height - 30, controls, colors::SUBTITLE, 1);
    }
}

/// Format model info string
fn format_model_info<'a>(prefix: &str, value: usize, buf: &'a mut [u8; 32]) -> &'a str {
    let prefix_bytes = prefix.as_bytes();
    let mut pos = 0;

    for &b in prefix_bytes {
        if pos < buf.len() {
            buf[pos] = b;
            pos += 1;
        }
    }

    // Write number
    if value == 0 {
        if pos < buf.len() {
            buf[pos] = b'0';
            pos += 1;
        }
    } else {
        let mut n = value;
        let start = pos;
        while n > 0 && pos < buf.len() {
            buf[pos] = b'0' + (n % 10) as u8;
            n /= 10;
            pos += 1;
        }
        buf[start..pos].reverse();
    }

    unsafe { core::str::from_utf8_unchecked(&buf[..pos]) }
}

/// Format size info string
fn format_size_info<'a>(w: usize, h: usize, d: usize, buf: &'a mut [u8; 48]) -> &'a str {
    let prefix = b"Size: ";
    let mut pos = 0;

    for &b in prefix {
        buf[pos] = b;
        pos += 1;
    }

    // Width
    pos = write_number(buf, pos, w);
    buf[pos] = b'x';
    pos += 1;

    // Height
    pos = write_number(buf, pos, h);
    buf[pos] = b'x';
    pos += 1;

    // Depth
    pos = write_number(buf, pos, d);

    // " voxels"
    let suffix = b" voxels";
    for &b in suffix {
        if pos < buf.len() {
            buf[pos] = b;
            pos += 1;
        }
    }

    unsafe { core::str::from_utf8_unchecked(&buf[..pos]) }
}

/// Format zoom info string
fn format_zoom_info<'a>(zoom: f32, buf: &'a mut [u8; 24]) -> &'a str {
    let prefix = b"Zoom: ";
    let mut pos = 0;

    for &b in prefix {
        buf[pos] = b;
        pos += 1;
    }

    // Write zoom as percentage
    let zoom_pct = (zoom * 100.0) as usize;
    pos = write_number(buf, pos, zoom_pct);
    buf[pos] = b'%';
    pos += 1;

    unsafe { core::str::from_utf8_unchecked(&buf[..pos]) }
}

/// Write a number to buffer and return new position
fn write_number(buf: &mut [u8], start: usize, value: usize) -> usize {
    let mut pos = start;

    if value == 0 {
        buf[pos] = b'0';
        return pos + 1;
    }

    let mut n = value;
    let num_start = pos;
    while n > 0 && pos < buf.len() {
        buf[pos] = b'0' + (n % 10) as u8;
        n /= 10;
        pos += 1;
    }
    buf[num_start..pos].reverse();

    pos
}
