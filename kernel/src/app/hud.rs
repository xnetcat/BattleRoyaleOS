//! HUD (Heads-Up Display) Rendering
//!
//! Draws game UI elements like health bars, inventory, minimap, etc.

extern crate alloc;

use alloc::format;
use crate::game::inventory::{Inventory, Materials};
use crate::game::storm::Storm;
use crate::game::weapon;
use crate::game::world::GameWorld;
use crate::graphics::font;
use crate::graphics::framebuffer::{rgb, Framebuffer, FRAMEBUFFER};

/// Draw storm overlay effect when player is in storm
pub fn draw_storm_overlay(fb_width: usize, fb_height: usize) {
    if let Some(fb_guard) = FRAMEBUFFER.try_lock() {
        if let Some(fb) = fb_guard.as_ref() {
            // Draw purple tint on edges of screen
            let purple = rgb(128, 0, 128);
            let edge_width = 30;
            // Use pitch for correct row stride
            let pitch = fb.pitch / 4;

            // Top edge
            for y in 0..edge_width {
                let alpha = (edge_width - y) as f32 / edge_width as f32;
                for x in 0..fb_width {
                    let idx = y * pitch + x;
                    let existing = fb.pixel_at(idx);
                    let blended = blend_color(existing, purple, alpha * 0.5);
                    fb.set_pixel_at(idx, blended);
                }
            }

            // Bottom edge
            for y in (fb_height - edge_width)..fb_height {
                let alpha = (y - (fb_height - edge_width)) as f32 / edge_width as f32;
                for x in 0..fb_width {
                    let idx = y * pitch + x;
                    let existing = fb.pixel_at(idx);
                    let blended = blend_color(existing, purple, alpha * 0.5);
                    fb.set_pixel_at(idx, blended);
                }
            }
        }
    }
}

/// Blend two colors
pub fn blend_color(base: u32, overlay: u32, alpha: f32) -> u32 {
    let br = ((base >> 16) & 0xFF) as f32;
    let bg = ((base >> 8) & 0xFF) as f32;
    let bb = (base & 0xFF) as f32;

    let or = ((overlay >> 16) & 0xFF) as f32;
    let og = ((overlay >> 8) & 0xFF) as f32;
    let ob = (overlay & 0xFF) as f32;

    let r = (br * (1.0 - alpha) + or * alpha) as u32;
    let g = (bg * (1.0 - alpha) + og * alpha) as u32;
    let b = (bb * (1.0 - alpha) + ob * alpha) as u32;

    (r << 16) | (g << 8) | b
}

/// Draw inventory hotbar
pub fn draw_inventory_hotbar(inv: &Inventory, fb_width: usize, fb_height: usize) {
    if let Some(fb_guard) = FRAMEBUFFER.try_lock() {
        if let Some(fb) = fb_guard.as_ref() {
            let slot_size = 50;
            let slot_spacing = 5;
            let total_width = 6 * slot_size + 5 * slot_spacing; // 6 slots (pickaxe + 5 weapons)
            let start_x = (fb_width - total_width) / 2;
            let start_y = fb_height - slot_size - 80; // Above health bar

            // Draw pickaxe slot
            let is_selected = inv.pickaxe_selected;
            let border_color = if is_selected { rgb(255, 200, 0) } else { rgb(100, 100, 100) };
            let bg_color = rgb(50, 50, 50);

            draw_slot(fb, start_x, start_y, slot_size, bg_color, border_color);
            font::draw_string_raw(fb, start_x + 15, start_y + 20, "P", rgb(200, 200, 200), 1);

            // Draw weapon slots
            for i in 0..5 {
                let x = start_x + (i + 1) * (slot_size + slot_spacing);
                let is_selected = !inv.pickaxe_selected && inv.selected_slot == i;
                let border_color = if is_selected { rgb(255, 200, 0) } else { rgb(100, 100, 100) };

                draw_slot(fb, x, start_y, slot_size, bg_color, border_color);

                // Draw weapon info if slot is filled
                if let Some(weapon) = &inv.slots[i] {
                    let rarity_color = match weapon.rarity {
                        weapon::Rarity::Common => rgb(150, 150, 150),
                        weapon::Rarity::Uncommon => rgb(50, 200, 50),
                        weapon::Rarity::Rare => rgb(50, 100, 255),
                        weapon::Rarity::Epic => rgb(200, 50, 200),
                        weapon::Rarity::Legendary => rgb(255, 180, 0),
                    };

                    // Draw rarity indicator bar at bottom of slot
                    for dy in (slot_size - 5)..slot_size {
                        for dx in 2..(slot_size - 2) {
                            fb.set_pixel(x + dx, start_y + dy, rarity_color);
                        }
                    }

                    // Draw weapon type letter
                    let letter = match weapon.weapon_type {
                        weapon::WeaponType::Pistol => "Pi",
                        weapon::WeaponType::Shotgun => "SG",
                        weapon::WeaponType::AssaultRifle => "AR",
                        weapon::WeaponType::Smg => "SM",
                        weapon::WeaponType::Sniper => "SR",
                        weapon::WeaponType::Pickaxe => "PX",
                    };
                    font::draw_string_raw(fb, x + 10, start_y + 15, letter, rgb(255, 255, 255), 1);

                    // Draw ammo count
                    let ammo_str = format!("{}", weapon.ammo);
                    font::draw_string_raw(fb, x + 15, start_y + 32, &ammo_str, rgb(200, 200, 200), 1);
                }

                // Draw slot number
                let num_str = format!("{}", i + 2);
                font::draw_string_raw(fb, x + 3, start_y + 3, &num_str, rgb(150, 150, 150), 1);
            }
        }
    }
}

/// Draw a UI slot/box
pub fn draw_slot(fb: &Framebuffer, x: usize, y: usize, size: usize, bg: u32, border: u32) {
    // Background
    for dy in 0..size {
        for dx in 0..size {
            fb.set_pixel(x + dx, y + dy, bg);
        }
    }
    // Border
    for dx in 0..size {
        fb.set_pixel(x + dx, y, border);
        fb.set_pixel(x + dx, y + size - 1, border);
    }
    for dy in 0..size {
        fb.set_pixel(x, y + dy, border);
        fb.set_pixel(x + size - 1, y + dy, border);
    }
}

/// Draw materials HUD
pub fn draw_materials_hud(materials: &Materials, fb_width: usize, fb_height: usize) {
    if let Some(fb_guard) = FRAMEBUFFER.try_lock() {
        if let Some(fb) = fb_guard.as_ref() {
            let x = fb_width - 150;
            let y = fb_height - 100;

            // Wood
            let wood_str = format!("W: {}", materials.wood);
            font::draw_string_raw(fb, x, y, &wood_str, rgb(180, 120, 60), 1);

            // Brick
            let brick_str = format!("B: {}", materials.brick);
            font::draw_string_raw(fb, x, y + 20, &brick_str, rgb(180, 80, 80), 1);

            // Metal
            let metal_str = format!("M: {}", materials.metal);
            font::draw_string_raw(fb, x, y + 40, &metal_str, rgb(150, 150, 170), 1);
        }
    }
}

/// Draw storm timer
pub fn draw_storm_timer(storm: &Storm, fb_width: usize, _fb_height: usize) {
    if let Some(fb_guard) = FRAMEBUFFER.try_lock() {
        if let Some(fb) = fb_guard.as_ref() {
            let phase_str = if storm.shrinking {
                format!("STORM CLOSING: {:.0}s", storm.timer)
            } else {
                format!("SAFE ZONE: {:.0}s", storm.timer)
            };

            let x = (fb_width - phase_str.len() * 8) / 2;
            let color = if storm.shrinking { rgb(200, 50, 200) } else { rgb(255, 255, 255) };
            font::draw_string_raw(fb, x, 50, &phase_str, color, 1);
        }
    }
}

/// Draw minimap
pub fn draw_minimap(local_player_id: Option<u8>, world: &GameWorld, fb_width: usize, _fb_height: usize) {
    if let Some(fb_guard) = FRAMEBUFFER.try_lock() {
        if let Some(fb) = fb_guard.as_ref() {
            let map_size = 150;
            let map_x = fb_width - map_size - 20;
            let map_y = 20;

            // Draw map background
            for dy in 0..map_size {
                for dx in 0..map_size {
                    fb.set_pixel(map_x + dx, map_y + dy, rgb(20, 40, 20));
                }
            }

            // Draw map border
            for dx in 0..map_size {
                fb.set_pixel(map_x + dx, map_y, rgb(100, 100, 100));
                fb.set_pixel(map_x + dx, map_y + map_size - 1, rgb(100, 100, 100));
            }
            for dy in 0..map_size {
                fb.set_pixel(map_x, map_y + dy, rgb(100, 100, 100));
                fb.set_pixel(map_x + map_size - 1, map_y + dy, rgb(100, 100, 100));
            }

            // Scale: map is 2000 units, minimap is 150 pixels
            let scale = map_size as f32 / 2000.0;
            let offset = 1000.0; // Center offset

            // Draw storm circle
            let storm_cx = ((world.storm.center.x + offset) * scale) as i32;
            let storm_cz = ((world.storm.center.z + offset) * scale) as i32;
            let storm_r = (world.storm.radius * scale) as i32;

            // Draw circle outline (simplified)
            for angle in 0..64 {
                let a = (angle as f32 / 64.0) * core::f32::consts::TAU;
                let px = storm_cx + (libm::cosf(a) * storm_r as f32) as i32;
                let py = storm_cz + (libm::sinf(a) * storm_r as f32) as i32;
                if px >= 0 && px < map_size as i32 && py >= 0 && py < map_size as i32 {
                    fb.set_pixel(map_x + px as usize, map_y + py as usize, rgb(255, 255, 255));
                }
            }

            // Draw player positions
            for player in &world.players {
                if !player.is_alive() {
                    continue;
                }
                let px = ((player.position.x + offset) * scale) as usize;
                let py = ((player.position.z + offset) * scale) as usize;

                if px < map_size && py < map_size {
                    let color = if Some(player.id) == local_player_id {
                        rgb(0, 255, 0) // Green for local player
                    } else {
                        rgb(255, 0, 0) // Red for others
                    };

                    // Draw 3x3 dot
                    for dx in 0..3 {
                        for dy in 0..3 {
                            if px + dx < map_size && py + dy < map_size {
                                fb.set_pixel(map_x + px + dx, map_y + py + dy, color);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Linear interpolation for u8
pub fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    ((a as f32) + (b as f32 - a as f32) * t) as u8
}
