//! Character customization screen

use crate::game::state::{GameState, MenuAction, PlayerCustomization, CustomizationCategory, PLAYER_CUSTOMIZATION};
use crate::graphics::font;
use crate::graphics::framebuffer::FRAMEBUFFER;
use crate::graphics::rasterizer::RenderContext;
use crate::graphics::ui::colors;
use crate::graphics::ui::panel::{draw_gradient_background_raw, draw_panel_raw, fill_rect_raw};

/// Customization screen state
pub struct CustomizationScreen {
    pub selected_category: usize,
    pub local_customization: PlayerCustomization,
    pub preview_rotation: f32,
    pub fb_width: usize,
    pub fb_height: usize,
}

impl CustomizationScreen {
    pub fn new(fb_width: usize, fb_height: usize) -> Self {
        Self {
            selected_category: 0,
            local_customization: *PLAYER_CUSTOMIZATION.lock(),
            preview_rotation: 0.0,
            fb_width,
            fb_height,
        }
    }

    /// Reload customization from global state
    pub fn reload(&mut self) {
        self.local_customization = *PLAYER_CUSTOMIZATION.lock();
    }

    /// Save customization to global state
    pub fn save(&self) {
        *PLAYER_CUSTOMIZATION.lock() = self.local_customization;
    }

    /// Handle input and return new state if transitioning
    pub fn update(&mut self, action: MenuAction) -> Option<GameState> {
        match action {
            MenuAction::Up => {
                if self.selected_category == 0 {
                    self.selected_category = CustomizationCategory::COUNT - 1;
                } else {
                    self.selected_category -= 1;
                }
            }
            MenuAction::Down => {
                self.selected_category = (self.selected_category + 1) % CustomizationCategory::COUNT;
            }
            MenuAction::Left => {
                let category = CustomizationCategory::from_index(self.selected_category);
                self.local_customization.prev(category);
            }
            MenuAction::Right => {
                let category = CustomizationCategory::from_index(self.selected_category);
                self.local_customization.next(category);
            }
            MenuAction::Back => {
                self.save();
                return Some(GameState::MainMenu);
            }
            MenuAction::Select | MenuAction::None => {}
        }

        None
    }

    /// Draw the customization screen
    pub fn draw(&self, _ctx: &RenderContext, fb_width: usize, fb_height: usize, _rotation: f32) {
        let fb_guard = FRAMEBUFFER.lock();
        let fb = match fb_guard.as_ref() {
            Some(f) => f,
            None => return,
        };

        // Draw gradient background
        draw_gradient_background_raw(fb, fb_width, fb_height);

        // Draw title
        let title = "CUSTOMIZE";
        let title_scale = 4;
        let title_y = 40;
        font::draw_string_centered_raw(fb, title_y, title, colors::TITLE, title_scale);

        // === LEFT PANEL: Options ===
        let options_panel_x = 50;
        let options_panel_y = 100;
        let options_panel_width = 350;
        let options_panel_height = 500;

        draw_panel_raw(fb, options_panel_x, options_panel_y, options_panel_width, options_panel_height, colors::PANEL_BG);

        let item_height = 55;
        let padding = 15;
        let scale = 2;

        for i in 0..CustomizationCategory::COUNT {
            let category = CustomizationCategory::from_index(i);
            let item_y = options_panel_y + padding + i * item_height;
            let selected = i == self.selected_category;

            self.draw_category_option(fb, options_panel_x + padding, item_y, options_panel_width - padding * 2, 45, category, selected, scale);
        }

        // === RIGHT PANEL: Preview ===
        let preview_panel_x = 450;
        let preview_panel_y = 100;
        let preview_panel_width = 400;
        let preview_panel_height = 500;

        draw_panel_raw(fb, preview_panel_x, preview_panel_y, preview_panel_width, preview_panel_height, colors::PANEL_BG);

        // Preview title
        font::draw_string_raw(
            fb,
            preview_panel_x + 20,
            preview_panel_y + 15,
            "PREVIEW",
            colors::SUBTITLE,
            scale,
        );

        // Draw a simple preview representation
        // (Full 3D preview would require integrating with the rasterizer here)
        self.draw_preview_placeholder(fb, preview_panel_x + 50, preview_panel_y + 60, 300, 400);

        // Draw footer
        let footer = "LEFT/RIGHT TO CHANGE. ESC TO SAVE.";
        let footer_y = fb_height - 50;
        font::draw_string_centered_raw(fb, footer_y, footer, colors::SUBTITLE, 2);
    }

    fn draw_category_option(&self, fb: &crate::graphics::framebuffer::Framebuffer, x: usize, y: usize, width: usize, height: usize, category: CustomizationCategory, selected: bool, scale: usize) {
        let bg_color = if selected {
            colors::BUTTON_SELECTED
        } else {
            colors::BUTTON_NORMAL
        };

        // Draw background
        fill_rect_raw(fb, x, y, width, height, bg_color);

        let text_height = font::char_height(scale);
        let text_y = y + (height.saturating_sub(text_height)) / 2;

        // Draw label
        let label = category.label();
        font::draw_string_raw(fb, x + 10, text_y, label, colors::BUTTON_TEXT, scale);

        // Draw value indicator
        let value = self.local_customization.get(category);
        let max = category.max_value();

        // Draw as dots/indicators
        let dot_size = 12;
        let dot_spacing = 20;
        let dots_width = (max as usize + 1) * dot_spacing;
        let dots_x = x + width - dots_width - 10;
        let dots_y = y + (height - dot_size) / 2;

        for i in 0..=max {
            let dot_x = dots_x + i as usize * dot_spacing;
            let filled = i == value;
            let color = if filled { colors::FN_YELLOW } else { colors::PANEL_BORDER };

            // Draw dot
            for dy in 0..dot_size {
                for dx in 0..dot_size {
                    let dist_sq = (dx as i32 - 6) * (dx as i32 - 6) + (dy as i32 - 6) * (dy as i32 - 6);
                    if dist_sq <= if filled { 36 } else { 25 } {
                        if dot_x + dx < fb.width && dots_y + dy < fb.height {
                            fb.put_pixel(dot_x + dx, dots_y + dy, color);
                        }
                    }
                }
            }
        }

        // Selection arrow
        if selected {
            font::draw_string_raw(fb, x - 20, text_y, ">", colors::FN_YELLOW, scale);
        }
    }

    fn draw_preview_placeholder(&self, fb: &crate::graphics::framebuffer::Framebuffer, x: usize, y: usize, width: usize, height: usize) {
        // Draw a simple colored representation of the character
        let custom = &self.local_customization;

        // Get colors based on customization
        let skin_colors = [0xFFDBB4, 0xD4A574, 0x8B5A2B];
        let shirt_colors = [0x3366CC, 0xCC3333, 0x33CC33, 0xEEEEEE];
        let pants_colors = [0x2244AA, 0x333333, 0x8B4513];
        let hair_colors = [0x2C2C2C, 0x654321, 0xFFD700, 0x8B4513];

        let skin = skin_colors[custom.skin_tone as usize % 3];
        let shirt = shirt_colors[custom.shirt_color as usize % 4];
        let pants = pants_colors[custom.pants_color as usize % 3];
        let hair = hair_colors[custom.hair_color as usize % 4];

        let cx = x + width / 2;
        let cy = y + height / 2;

        // Simple body representation (scaled blocks)
        let scale = 6;

        // Head
        fill_rect_raw(fb, cx - 3 * scale, cy - 12 * scale, 6 * scale, 6 * scale, skin);

        // Hair (on top of head)
        let hair_height = match custom.hair_style {
            0 => 1,
            1 => 2,
            2 => 3,
            _ => 0,
        };
        if hair_height > 0 {
            fill_rect_raw(fb, cx - 3 * scale, cy - 12 * scale - hair_height * scale, 6 * scale, hair_height * scale, hair);
        }

        // Torso
        fill_rect_raw(fb, cx - 4 * scale, cy - 6 * scale, 8 * scale, 8 * scale, shirt);

        // Arms
        fill_rect_raw(fb, cx - 6 * scale, cy - 5 * scale, 2 * scale, 6 * scale, shirt);
        fill_rect_raw(fb, cx + 4 * scale, cy - 5 * scale, 2 * scale, 6 * scale, shirt);

        // Legs
        fill_rect_raw(fb, cx - 3 * scale, cy + 2 * scale, 3 * scale, 8 * scale, pants);
        fill_rect_raw(fb, cx, cy + 2 * scale, 3 * scale, 8 * scale, pants);

        // Shoes
        let shoe_color = if custom.shoes_color == 0 { 0x222222 } else { 0x654321 };
        fill_rect_raw(fb, cx - 3 * scale, cy + 10 * scale, 3 * scale, 2 * scale, shoe_color);
        fill_rect_raw(fb, cx, cy + 10 * scale, 3 * scale, 2 * scale, shoe_color);

        // Backpack indicator
        if custom.backpack_style > 0 {
            let bp_size = custom.backpack_style as usize * scale;
            let bp_color = if custom.backpack_style == 1 {
                0x556B2F
            } else if custom.backpack_style == 2 {
                0xD2B48C
            } else {
                0x333333
            };
            fill_rect_raw(fb, cx + 4 * scale, cy - 4 * scale, bp_size, bp_size + scale, bp_color);
        }
    }
}
