//! Settings screen

use crate::game::state::{GameState, MenuAction, Settings, SettingsOption, SETTINGS};
use crate::graphics::font;
use crate::graphics::framebuffer::{Framebuffer, FRAMEBUFFER};
use crate::graphics::rasterizer::RenderContext;
use crate::graphics::ui::colors;
use crate::graphics::ui::panel::{draw_gradient_background_raw, draw_panel_raw};

/// Settings screen state
pub struct SettingsScreen {
    pub selected_index: usize,
    pub fb_width: usize,
    pub fb_height: usize,
    pub local_settings: Settings,
}

impl SettingsScreen {
    pub fn new(fb_width: usize, fb_height: usize) -> Self {
        Self {
            selected_index: 0,
            fb_width,
            fb_height,
            local_settings: *SETTINGS.lock(),
        }
    }

    /// Reload settings from global state
    pub fn reload(&mut self) {
        self.local_settings = *SETTINGS.lock();
    }

    /// Save settings to global state
    pub fn save(&self) {
        *SETTINGS.lock() = self.local_settings;
    }

    /// Handle input and return new state if transitioning
    pub fn update(&mut self, action: MenuAction) -> Option<GameState> {
        match action {
            MenuAction::Up => {
                if self.selected_index == 0 {
                    self.selected_index = SettingsOption::COUNT - 1;
                } else {
                    self.selected_index -= 1;
                }
            }
            MenuAction::Down => {
                self.selected_index = (self.selected_index + 1) % SettingsOption::COUNT;
            }
            MenuAction::Left => {
                let option = SettingsOption::from_index(self.selected_index);
                if option.is_toggle() {
                    self.local_settings.toggle(option);
                } else if option.is_range() {
                    self.local_settings.adjust(option, -1);
                }
            }
            MenuAction::Right => {
                let option = SettingsOption::from_index(self.selected_index);
                if option.is_toggle() {
                    self.local_settings.toggle(option);
                } else if option.is_range() {
                    self.local_settings.adjust(option, 1);
                }
            }
            MenuAction::Select => {
                let option = SettingsOption::from_index(self.selected_index);
                if option.is_toggle() {
                    self.local_settings.toggle(option);
                } else if option == SettingsOption::Back {
                    self.save();
                    return Some(GameState::PartyLobby);
                }
            }
            MenuAction::Back => {
                self.save();
                return Some(GameState::PartyLobby);
            }
            MenuAction::None => {}
        }

        None
    }

    /// Draw the settings screen
    pub fn draw(&self, _ctx: &RenderContext, fb_width: usize, fb_height: usize) {
        let fb_guard = FRAMEBUFFER.lock();
        let fb = match fb_guard.as_ref() {
            Some(f) => f,
            None => return,
        };

        // Draw gradient background
        draw_gradient_background_raw(fb, fb_width, fb_height);

        // Draw title
        let title = "SETTINGS";
        let title_scale = 4;
        let title_y = 60;
        font::draw_string_centered_raw(fb, title_y, title, colors::TITLE, title_scale);

        // Draw settings panel
        let panel_width = 600;
        let panel_height = 450;
        let panel_x = (fb_width - panel_width) / 2;
        let panel_y = 140;
        draw_panel_raw(fb, panel_x, panel_y, panel_width, panel_height, colors::PANEL_BG);

        // Draw settings options
        let item_height = 60;
        let padding = 20;
        let item_width = panel_width - padding * 2;
        let scale = 2;

        for i in 0..SettingsOption::COUNT {
            let option = SettingsOption::from_index(i);
            let item_y = panel_y + padding + i * item_height;
            let selected = i == self.selected_index;

            self.draw_option(fb, panel_x + padding, item_y, item_width, item_height - 10, option, selected, scale);
        }

        // Draw footer
        let footer = "LEFT/RIGHT TO ADJUST. ESC TO SAVE AND EXIT.";
        let footer_y = fb_height - 50;
        font::draw_string_centered_raw(fb, footer_y, footer, colors::SUBTITLE, 2);
    }

    fn draw_option(&self, fb: &Framebuffer, x: usize, y: usize, width: usize, height: usize, option: SettingsOption, selected: bool, scale: usize) {
        let bg_color = if selected {
            colors::BUTTON_SELECTED
        } else {
            colors::BUTTON_NORMAL
        };

        // Draw background
        for py in y..(y + height).min(fb.height) {
            for px in x..(x + width).min(fb.width) {
                fb.put_pixel(px, py, bg_color);
            }
        }

        let text_height = font::char_height(scale);
        let text_y = y + (height.saturating_sub(text_height)) / 2;

        // Draw label
        let label = option.label();
        font::draw_string_raw(fb, x + 15, text_y, label, colors::BUTTON_TEXT, scale);

        // Draw value based on option type
        if option.is_toggle() {
            let value_str = self.local_settings.get_value_str(option);
            let value_color = if self.local_settings.get_value(option) == 1 {
                colors::READY
            } else {
                colors::NOT_READY
            };
            let value_width = font::string_width(value_str, scale);
            let value_x = x + width - value_width - 15;
            font::draw_string_raw(fb, value_x, text_y, value_str, value_color, scale);
        } else if option.is_range() {
            // Draw slider
            let bar_x = x + width / 2;
            let bar_width = width / 2 - 60;
            let bar_y = y + height / 2 - 4;
            let bar_height = 8;

            // Draw bar background
            for py in bar_y..(bar_y + bar_height).min(fb.height) {
                for px in bar_x..(bar_x + bar_width).min(fb.width) {
                    fb.put_pixel(px, py, colors::PANEL_BG);
                }
            }

            // Calculate fill based on option
            let (value, min, max) = match option {
                SettingsOption::Sensitivity => (self.local_settings.sensitivity, 1, 10),
                SettingsOption::RenderDistance => (self.local_settings.render_distance, 1, 3),
                SettingsOption::Volume => (self.local_settings.volume, 0, 100),
                _ => (0, 0, 1),
            };

            let fill_ratio = (value - min) as f32 / (max - min) as f32;
            let fill_width = (bar_width as f32 * fill_ratio) as usize;

            // Draw filled portion
            for py in bar_y..(bar_y + bar_height).min(fb.height) {
                for px in bar_x..(bar_x + fill_width).min(fb.width) {
                    fb.put_pixel(px, py, colors::FN_BLUE);
                }
            }

            // Draw value
            let mut buf = [0u8; 8];
            let value_str = font::format_number(value as u32, &mut buf);
            let value_width = font::string_width(value_str, scale);
            let value_x = x + width - value_width - 15;
            font::draw_string_raw(fb, value_x, text_y, value_str, colors::BUTTON_TEXT, scale);
        } else if option == SettingsOption::Back {
            // Draw back button indicator
            if selected {
                font::draw_string_raw(fb, x - 25, text_y, ">", colors::FN_YELLOW, scale);
            }
        }
    }
}
