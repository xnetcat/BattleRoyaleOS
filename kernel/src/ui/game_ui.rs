//! In-game UI elements (HUD, crosshair, weapon display, etc.)

use crate::game::state::PlayerPhase;
use crate::graphics::font;
use crate::graphics::framebuffer::{Framebuffer, FRAMEBUFFER};
use crate::graphics::rasterizer::RenderContext;
use crate::graphics::ui::colors as ui_colors;
use crate::graphics::ui::panel::{draw_crosshair_raw, draw_gradient_background_raw, draw_panel_raw, draw_progress_bar_raw, fill_rect_raw};

/// Draw countdown screen
pub fn draw_countdown(_ctx: &RenderContext, fb_width: usize, fb_height: usize, seconds: u8) {
    let fb_guard = FRAMEBUFFER.lock();
    let fb = match fb_guard.as_ref() {
        Some(f) => f,
        None => return,
    };

    // Draw gradient background
    draw_gradient_background_raw(fb, fb_width, fb_height);

    // Draw "GAME STARTING" text
    let title = "GAME STARTING";
    font::draw_string_centered_raw(fb, fb_height / 2 - 100, title, colors::TITLE, 4);

    // Draw large countdown number
    let mut buf = [0u8; 4];
    let num_str = font::format_number(seconds as u32, &mut buf);
    font::draw_string_centered_raw(fb, fb_height / 2, num_str, colors::FN_YELLOW, 10);

    // Draw "GET READY!" text
    font::draw_string_centered_raw(fb, fb_height / 2 + 120, "GET READY!", colors::WHITE, 3);
}

/// Draw victory/defeat screen
pub fn draw_victory(_ctx: &RenderContext, fb_width: usize, fb_height: usize, winner_id: Option<u8>) {
    let fb_guard = FRAMEBUFFER.lock();
    let fb = match fb_guard.as_ref() {
        Some(f) => f,
        None => return,
    };

    // Draw gradient background
    draw_gradient_background_raw(fb, fb_width, fb_height);

    // Check if local player won (simplified - assumes local is player 0)
    let is_winner = winner_id == Some(0);

    if is_winner {
        // Victory screen
        let title = "VICTORY ROYALE!";
        font::draw_string_centered_raw(fb, fb_height / 2 - 80, title, colors::FN_YELLOW, 5);

        // Confetti-like decorations (simple colored dots)
        for i in 0..50 {
            let x = (i * 37 + 100) % fb_width;
            let y = (i * 23 + 50) % (fb_height / 2);
            let color = match i % 4 {
                0 => colors::FN_YELLOW,
                1 => colors::FN_BLUE,
                2 => 0xCC3366, // Pink
                _ => colors::READY,
            };
            fill_rect_raw(fb, x, y, 6, 6, color);
        }
    } else {
        // Defeat screen
        let title = "BETTER LUCK NEXT TIME";
        font::draw_string_centered_raw(fb, fb_height / 2 - 80, title, colors::HEALTH_LOW, 4);

        let placement = match winner_id {
            Some(_) => "YOU PLACED: #2",
            None => "MATCH ENDED",
        };
        font::draw_string_centered_raw(fb, fb_height / 2, placement, colors::WHITE, 3);
    }

    // Stats panel
    let panel_width = 400;
    let panel_height = 150;
    let panel_x = (fb_width - panel_width) / 2;
    let panel_y = fb_height / 2 + 60;
    draw_panel_raw(fb, panel_x, panel_y, panel_width, panel_height, colors::PANEL_BG);

    // Draw stats (placeholder values)
    font::draw_string_raw(fb, panel_x + 20, panel_y + 20, "ELIMINATIONS:", colors::SUBTITLE, 2);
    font::draw_string_raw(fb, panel_x + 250, panel_y + 20, "0", colors::WHITE, 2);

    font::draw_string_raw(fb, panel_x + 20, panel_y + 60, "DAMAGE DEALT:", colors::SUBTITLE, 2);
    font::draw_string_raw(fb, panel_x + 250, panel_y + 60, "0", colors::WHITE, 2);

    font::draw_string_raw(fb, panel_x + 20, panel_y + 100, "TIME SURVIVED:", colors::SUBTITLE, 2);
    font::draw_string_raw(fb, panel_x + 250, panel_y + 100, "0:00", colors::WHITE, 2);

    // Return to menu prompt
    font::draw_string_centered_raw(fb, fb_height - 60, "PRESS ENTER TO CONTINUE", colors::SUBTITLE, 2);
}

/// In-game UI manager
pub struct GameUI {
    pub fb_width: usize,
    pub fb_height: usize,
}

impl GameUI {
    pub fn new(fb_width: usize, fb_height: usize) -> Self {
        Self { fb_width, fb_height }
    }

    /// Draw the full game HUD
    pub fn draw(&self, fb: &Framebuffer, health: u8, shield: u8, ammo: u16, max_ammo: u16, materials: u32, alive_count: usize, eliminations: u16, phase: PlayerPhase, weapon_name: &str) {
        match phase {
            PlayerPhase::OnBus => self.draw_bus_ui(fb),
            PlayerPhase::Freefall => self.draw_freefall_ui(fb, 500.0), // Placeholder altitude
            PlayerPhase::Gliding => self.draw_gliding_ui(fb, 200.0),
            PlayerPhase::Grounded => {
                self.draw_ground_ui(fb, health, shield, ammo, max_ammo, materials, alive_count, eliminations, weapon_name);
            }
            PlayerPhase::Eliminated => self.draw_eliminated_ui(fb),
            PlayerPhase::Spectating => self.draw_spectating_ui(fb, "PlayerName"),
        }
    }

    /// Draw UI when on the battle bus
    fn draw_bus_ui(&self, fb: &Framebuffer) {
        // Large prompt in center
        let prompt = "PRESS SPACE TO DROP!";
        font::draw_string_centered_raw(fb, self.fb_height / 2 - 50, prompt, colors::FN_YELLOW, 4);

        // Minimap hint
        font::draw_string_centered_raw(fb, self.fb_height / 2 + 20, "LOOK AT MINIMAP FOR DROP LOCATION", colors::WHITE, 2);
    }

    /// Draw UI during freefall
    fn draw_freefall_ui(&self, fb: &Framebuffer, altitude: f32) {
        // Crosshair
        draw_crosshair_raw(fb, self.fb_width, self.fb_height, colors::WHITE);

        // Altitude indicator on right side
        self.draw_altitude_indicator(fb, altitude);

        // Deploy prompt when low enough
        if altitude < 200.0 && altitude > 100.0 {
            let prompt = "PRESS SPACE TO DEPLOY GLIDER";
            font::draw_string_centered_raw(fb, self.fb_height - 150, prompt, colors::FN_YELLOW, 2);
        }
    }

    /// Draw UI while gliding
    fn draw_gliding_ui(&self, fb: &Framebuffer, altitude: f32) {
        // Crosshair
        draw_crosshair_raw(fb, self.fb_width, self.fb_height, colors::WHITE);

        // Altitude indicator
        self.draw_altitude_indicator(fb, altitude);

        // Gliding indicator
        font::draw_string_centered_raw(fb, 50, "GLIDING", colors::FN_BLUE, 2);
    }

    /// Draw full ground gameplay UI
    fn draw_ground_ui(&self, fb: &Framebuffer, health: u8, shield: u8, ammo: u16, max_ammo: u16, materials: u32, alive_count: usize, eliminations: u16, weapon_name: &str) {
        // Crosshair
        draw_crosshair_raw(fb, self.fb_width, self.fb_height, colors::WHITE);

        // === BOTTOM LEFT: Health and Shield ===
        let bottom_left_y = self.fb_height - 120;

        // Health bar
        self.draw_health_bar(fb, 20, bottom_left_y, 200, 25, health);
        // Shield bar
        self.draw_shield_bar(fb, 20, bottom_left_y + 30, 200, 20, shield);

        // === BOTTOM CENTER: Weapon Hotbar ===
        self.draw_weapon_hotbar(fb, 0, weapon_name, ammo, max_ammo);

        // === BOTTOM RIGHT: Materials ===
        let materials_x = self.fb_width - 150;
        self.draw_materials(fb, materials_x, bottom_left_y, materials);

        // === TOP RIGHT: Player count and eliminations ===
        let top_right_x = self.fb_width - 120;
        self.draw_player_count(fb, top_right_x, 20, alive_count);
        self.draw_eliminations(fb, top_right_x, 60, eliminations);

        // === MINIMAP (top left) ===
        self.draw_minimap_placeholder(fb, 20, 60, 150);
    }

    /// Draw eliminated/death screen
    fn draw_eliminated_ui(&self, fb: &Framebuffer) {
        // Darken screen
        // (Would ideally use alpha blending)

        let text = "ELIMINATED";
        font::draw_string_centered_raw(fb, self.fb_height / 2 - 50, text, colors::HEALTH_LOW, 5);

        let spectate = "PRESS SPACE TO SPECTATE";
        font::draw_string_centered_raw(fb, self.fb_height / 2 + 30, spectate, colors::WHITE, 2);
    }

    /// Draw spectating UI
    fn draw_spectating_ui(&self, fb: &Framebuffer, target_name: &str) {
        // Spectating banner at top
        let banner_y = 20;
        draw_panel_raw(fb, self.fb_width / 2 - 150, banner_y, 300, 50, colors::PANEL_BG);

        font::draw_string_raw(fb, self.fb_width / 2 - 130, banner_y + 10, "SPECTATING:", colors::SUBTITLE, 2);
        font::draw_string_raw(fb, self.fb_width / 2 - 20, banner_y + 10, target_name, colors::WHITE, 2);

        // Navigation hint
        let hint = "A/D: SWITCH PLAYER";
        font::draw_string_centered_raw(fb, self.fb_height - 40, hint, colors::SUBTITLE, 2);
    }

    /// Draw health bar
    fn draw_health_bar(&self, fb: &Framebuffer, x: usize, y: usize, width: usize, height: usize, health: u8) {
        let fill_color = if health > 50 {
            colors::HEALTH_HIGH
        } else if health > 25 {
            colors::HEALTH_MED
        } else {
            colors::HEALTH_LOW
        };

        let progress = health as f32 / 100.0;
        draw_progress_bar_raw(fb, x, y, width, height, progress, fill_color, colors::PANEL_BG);

        // Health icon/text
        let mut buf = [0u8; 8];
        let health_str = font::format_number(health as u32, &mut buf);
        font::draw_string_raw(fb, x + width + 10, y + 4, health_str, colors::WHITE, 2);
    }

    /// Draw shield bar
    fn draw_shield_bar(&self, fb: &Framebuffer, x: usize, y: usize, width: usize, height: usize, shield: u8) {
        if shield == 0 {
            return;
        }

        let fill_color = colors::FN_BLUE;
        let progress = shield as f32 / 100.0;
        draw_progress_bar_raw(fb, x, y, width, height, progress, fill_color, colors::PANEL_BG);

        // Shield text
        let mut buf = [0u8; 8];
        let shield_str = font::format_number(shield as u32, &mut buf);
        font::draw_string_raw(fb, x + width + 10, y + 2, shield_str, colors::FN_BLUE, 2);
    }

    /// Draw altitude indicator (vertical bar on right side)
    fn draw_altitude_indicator(&self, fb: &Framebuffer, altitude: f32) {
        let bar_x = self.fb_width - 60;
        let bar_y = 100;
        let bar_width = 20;
        let bar_height = 400;

        // Background
        fill_rect_raw(fb, bar_x, bar_y, bar_width, bar_height, colors::PANEL_BG);

        // Fill based on altitude (max 500m)
        let max_alt = 500.0;
        let fill_ratio = (altitude / max_alt).clamp(0.0, 1.0);
        let fill_height = (bar_height as f32 * fill_ratio) as usize;
        let fill_y = bar_y + bar_height - fill_height;

        let fill_color = if altitude < 100.0 {
            colors::HEALTH_LOW
        } else if altitude < 200.0 {
            colors::HEALTH_MED
        } else {
            colors::HEALTH_HIGH
        };

        fill_rect_raw(fb, bar_x, fill_y, bar_width, fill_height, fill_color);

        // Altitude number
        let mut buf = [0u8; 8];
        let alt_int = altitude as u32;
        let alt_str = font::format_number(alt_int, &mut buf);
        font::draw_string_raw(fb, bar_x - 50, bar_y + bar_height + 10, alt_str, colors::WHITE, 2);
        font::draw_string_raw(fb, bar_x - 10, bar_y + bar_height + 10, "M", colors::SUBTITLE, 2);

        // Deploy line at 100m
        let deploy_y = bar_y + bar_height - (bar_height as f32 * (100.0 / max_alt)) as usize;
        for x in bar_x..(bar_x + bar_width) {
            if deploy_y < fb.height {
                fb.put_pixel(x, deploy_y, colors::FN_YELLOW);
            }
        }
    }

    /// Draw weapon hotbar
    fn draw_weapon_hotbar(&self, fb: &Framebuffer, selected_slot: usize, weapon_name: &str, ammo: u16, max_ammo: u16) {
        let slot_size = 60;
        let slot_spacing = 10;
        let total_slots = 5;
        let total_width = total_slots * slot_size + (total_slots - 1) * slot_spacing;
        let start_x = (self.fb_width - total_width) / 2;
        let y = self.fb_height - 80;

        // Draw 5 weapon slots
        for i in 0..total_slots {
            let slot_x = start_x + i * (slot_size + slot_spacing);
            let is_selected = i == selected_slot;

            let bg_color = if is_selected {
                colors::BUTTON_SELECTED
            } else {
                colors::PANEL_BG
            };

            fill_rect_raw(fb, slot_x, y, slot_size, slot_size, bg_color);

            // Border
            let border_color = if is_selected {
                colors::FN_YELLOW
            } else {
                colors::PANEL_BORDER
            };

            for x in slot_x..(slot_x + slot_size).min(fb.width) {
                if y < fb.height {
                    fb.put_pixel(x, y, border_color);
                }
                if y + slot_size - 1 < fb.height {
                    fb.put_pixel(x, y + slot_size - 1, border_color);
                }
            }
            for sy in y..(y + slot_size).min(fb.height) {
                if slot_x < fb.width {
                    fb.put_pixel(slot_x, sy, border_color);
                }
                if slot_x + slot_size - 1 < fb.width {
                    fb.put_pixel(slot_x + slot_size - 1, sy, border_color);
                }
            }

            // Slot number
            let mut num_buf = [0u8; 2];
            num_buf[0] = b'0' + (i + 1) as u8;
            let num_str = unsafe { core::str::from_utf8_unchecked(&num_buf[..1]) };
            font::draw_string_raw(fb, slot_x + 5, y + 5, num_str, colors::SUBTITLE, 1);
        }

        // Weapon name below hotbar
        let name_width = font::string_width(weapon_name, 2);
        let name_x = (self.fb_width - name_width) / 2;
        font::draw_string_raw(fb, name_x, y + slot_size + 5, weapon_name, colors::WHITE, 2);

        // Ammo display
        let mut ammo_buf = [0u8; 16];
        let ammo_str = format_ammo(ammo, max_ammo, &mut ammo_buf);
        let ammo_width = font::string_width(ammo_str, 2);
        let ammo_x = (self.fb_width - ammo_width) / 2;
        font::draw_string_raw(fb, ammo_x, y + slot_size + 30, ammo_str, colors::FN_YELLOW, 2);
    }

    /// Draw materials count
    fn draw_materials(&self, fb: &Framebuffer, x: usize, y: usize, materials: u32) {
        // Background panel
        draw_panel_raw(fb, x, y, 130, 60, colors::PANEL_BG);

        // Material icon (simple square)
        fill_rect_raw(fb, x + 10, y + 15, 30, 30, WOOD_PLANK);

        // Count
        let mut buf = [0u8; 12];
        let mat_str = font::format_number(materials, &mut buf);
        font::draw_string_raw(fb, x + 50, y + 20, mat_str, colors::WHITE, 2);
    }

    /// Draw player count (top right)
    fn draw_player_count(&self, fb: &Framebuffer, x: usize, y: usize, count: usize) {
        draw_panel_raw(fb, x, y, 100, 35, colors::PANEL_BG);

        // Player icon (simple figure)
        fill_rect_raw(fb, x + 8, y + 8, 6, 8, colors::WHITE);
        fill_rect_raw(fb, x + 6, y + 16, 10, 12, colors::WHITE);

        // Count
        let mut buf = [0u8; 8];
        let count_str = font::format_number(count as u32, &mut buf);
        font::draw_string_raw(fb, x + 30, y + 10, count_str, colors::WHITE, 2);
    }

    /// Draw eliminations (top right, below player count)
    fn draw_eliminations(&self, fb: &Framebuffer, x: usize, y: usize, elims: u16) {
        draw_panel_raw(fb, x, y, 100, 35, colors::PANEL_BG);

        // Skull icon (simple)
        fill_rect_raw(fb, x + 8, y + 8, 10, 10, colors::WHITE);
        fb.put_pixel(x + 10, y + 12, colors::PANEL_BG);
        fb.put_pixel(x + 14, y + 12, colors::PANEL_BG);
        fill_rect_raw(fb, x + 10, y + 18, 6, 4, colors::WHITE);

        // Count
        let mut buf = [0u8; 8];
        let elim_str = font::format_number(elims as u32, &mut buf);
        font::draw_string_raw(fb, x + 30, y + 10, elim_str, colors::WHITE, 2);
    }

    /// Draw minimap placeholder
    fn draw_minimap_placeholder(&self, fb: &Framebuffer, x: usize, y: usize, size: usize) {
        // Background
        fill_rect_raw(fb, x, y, size, size, colors::PANEL_BG);

        // Border
        for i in x..(x + size).min(fb.width) {
            if y < fb.height {
                fb.put_pixel(i, y, colors::PANEL_BORDER);
            }
            if y + size - 1 < fb.height {
                fb.put_pixel(i, y + size - 1, colors::PANEL_BORDER);
            }
        }
        for j in y..(y + size).min(fb.height) {
            if x < fb.width {
                fb.put_pixel(x, j, colors::PANEL_BORDER);
            }
            if x + size - 1 < fb.width {
                fb.put_pixel(x + size - 1, j, colors::PANEL_BORDER);
            }
        }

        // Simple terrain representation
        fill_rect_raw(fb, x + 10, y + 10, size - 20, size - 20, GRASS_GREEN);

        // Player position (center dot)
        let cx = x + size / 2;
        let cy = y + size / 2;
        for dy in 0..6 {
            for dx in 0..6 {
                let dist_sq = (dx as i32 - 3) * (dx as i32 - 3) + (dy as i32 - 3) * (dy as i32 - 3);
                if dist_sq <= 9 && cx + dx - 3 < fb.width && cy + dy - 3 < fb.height {
                    fb.put_pixel(cx + dx - 3, cy + dy - 3, colors::WHITE);
                }
            }
        }

        // Direction indicator
        fb.put_pixel(cx, cy - 5, colors::FN_YELLOW);
        fb.put_pixel(cx, cy - 6, colors::FN_YELLOW);
    }
}

/// Additional color constants for game UI
mod colors {
    pub use crate::graphics::ui::colors::*;
}

// Additional game-specific colors
const WOOD_PLANK: u32 = 0xC4A76E;
const GRASS_GREEN: u32 = 0x4CAF50;

/// Format ammo display "XX/YY"
fn format_ammo<'a>(current: u16, max: u16, buf: &'a mut [u8; 16]) -> &'a str {
    let mut pos = 0;

    // Current ammo
    if current == 0 {
        buf[pos] = b'0';
        pos += 1;
    } else {
        let mut n = current as usize;
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

    // Max ammo
    if max == 0 {
        buf[pos] = b'0';
        pos += 1;
    } else {
        let mut n = max as usize;
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
