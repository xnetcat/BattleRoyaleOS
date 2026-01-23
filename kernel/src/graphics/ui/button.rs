//! Button UI primitive

use crate::graphics::font;
use crate::graphics::framebuffer::Framebuffer;
use super::colors;

/// A clickable button with label
#[derive(Debug, Clone)]
pub struct Button {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub label: &'static str,
    pub selected: bool,
}

impl Button {
    /// Create a new button
    pub fn new(x: usize, y: usize, width: usize, height: usize, label: &'static str) -> Self {
        Self {
            x,
            y,
            width,
            height,
            label,
            selected: false,
        }
    }

    /// Create a centered button at given y position
    pub fn centered(y: usize, width: usize, height: usize, label: &'static str, fb_width: usize) -> Self {
        let x = (fb_width.saturating_sub(width)) / 2;
        Self::new(x, y, width, height, label)
    }

    /// Draw the button
    pub fn draw(&self, fb: &Framebuffer) {
        let bg_color = if self.selected {
            colors::BUTTON_SELECTED
        } else {
            colors::BUTTON_NORMAL
        };

        let border_color = if self.selected {
            colors::FN_YELLOW
        } else {
            colors::PANEL_BORDER
        };

        // Draw border (2px)
        for y in self.y..(self.y + self.height).min(fb.height) {
            for x in self.x..(self.x + self.width).min(fb.width) {
                let is_border = x < self.x + 2
                    || x >= self.x + self.width - 2
                    || y < self.y + 2
                    || y >= self.y + self.height - 2;

                let color = if is_border { border_color } else { bg_color };
                fb.put_pixel(x, y, color);
            }
        }

        // Draw centered label
        let scale = 2;
        let text_width = font::string_width(self.label, scale);
        let text_height = font::char_height(scale);

        let text_x = self.x + (self.width.saturating_sub(text_width)) / 2;
        let text_y = self.y + (self.height.saturating_sub(text_height)) / 2;

        font::draw_string_raw(fb, text_x, text_y, self.label, colors::BUTTON_TEXT, scale);

        // Draw selection indicator (arrow) if selected
        if self.selected {
            let arrow_x = self.x.saturating_sub(30);
            let arrow_y = self.y + (self.height.saturating_sub(text_height)) / 2;
            font::draw_string_raw(fb, arrow_x, arrow_y, ">", colors::FN_YELLOW, scale);
        }
    }

    /// Check if point is inside button
    pub fn contains(&self, px: usize, py: usize) -> bool {
        px >= self.x && px < self.x + self.width && py >= self.y && py < self.y + self.height
    }
}

/// A list of menu buttons
pub struct ButtonList {
    pub buttons: [Button; 4],
    pub selected_index: usize,
    pub count: usize,
}

impl ButtonList {
    /// Create a new button list (up to 4 buttons)
    pub fn new(buttons: [Button; 4], count: usize) -> Self {
        let mut list = Self {
            buttons,
            selected_index: 0,
            count,
        };
        if count > 0 {
            list.buttons[0].selected = true;
        }
        list
    }

    /// Move selection up
    pub fn select_prev(&mut self) {
        if self.count == 0 {
            return;
        }
        self.buttons[self.selected_index].selected = false;
        if self.selected_index == 0 {
            self.selected_index = self.count - 1;
        } else {
            self.selected_index -= 1;
        }
        self.buttons[self.selected_index].selected = true;
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        if self.count == 0 {
            return;
        }
        self.buttons[self.selected_index].selected = false;
        self.selected_index = (self.selected_index + 1) % self.count;
        self.buttons[self.selected_index].selected = true;
    }

    /// Set selection by index
    pub fn select(&mut self, index: usize) {
        if index >= self.count {
            return;
        }
        self.buttons[self.selected_index].selected = false;
        self.selected_index = index;
        self.buttons[self.selected_index].selected = true;
    }

    /// Draw all buttons
    pub fn draw(&self, fb: &Framebuffer) {
        for i in 0..self.count {
            self.buttons[i].draw(fb);
        }
    }

    /// Get the selected button's label
    pub fn selected_label(&self) -> &'static str {
        self.buttons[self.selected_index].label
    }
}

/// A toggle switch (ON/OFF)
pub struct Toggle {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub label: &'static str,
    pub value: bool,
    pub selected: bool,
}

impl Toggle {
    pub fn new(x: usize, y: usize, width: usize, height: usize, label: &'static str, value: bool) -> Self {
        Self {
            x,
            y,
            width,
            height,
            label,
            value,
            selected: false,
        }
    }

    pub fn draw(&self, fb: &Framebuffer) {
        let bg_color = if self.selected {
            colors::BUTTON_SELECTED
        } else {
            colors::BUTTON_NORMAL
        };

        // Draw background
        for y in self.y..(self.y + self.height).min(fb.height) {
            for x in self.x..(self.x + self.width).min(fb.width) {
                fb.put_pixel(x, y, bg_color);
            }
        }

        let scale = 2;
        let text_height = font::char_height(scale);
        let text_y = self.y + (self.height.saturating_sub(text_height)) / 2;

        // Draw label on left
        font::draw_string_raw(fb, self.x + 10, text_y, self.label, colors::BUTTON_TEXT, scale);

        // Draw value on right
        let value_str = if self.value { "ON" } else { "OFF" };
        let value_color = if self.value { colors::READY } else { colors::NOT_READY };
        let value_width = font::string_width(value_str, scale);
        let value_x = self.x + self.width - value_width - 10;
        font::draw_string_raw(fb, value_x, text_y, value_str, value_color, scale);
    }

    pub fn toggle(&mut self) {
        self.value = !self.value;
    }
}

/// A slider (range value)
pub struct Slider {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub label: &'static str,
    pub value: u8,
    pub min: u8,
    pub max: u8,
    pub selected: bool,
}

impl Slider {
    pub fn new(x: usize, y: usize, width: usize, height: usize, label: &'static str, value: u8, min: u8, max: u8) -> Self {
        Self {
            x,
            y,
            width,
            height,
            label,
            value,
            min,
            max,
            selected: false,
        }
    }

    pub fn draw(&self, fb: &Framebuffer) {
        let bg_color = if self.selected {
            colors::BUTTON_SELECTED
        } else {
            colors::BUTTON_NORMAL
        };

        // Draw background
        for y in self.y..(self.y + self.height).min(fb.height) {
            for x in self.x..(self.x + self.width).min(fb.width) {
                fb.put_pixel(x, y, bg_color);
            }
        }

        let scale = 2;
        let text_height = font::char_height(scale);
        let text_y = self.y + (self.height.saturating_sub(text_height)) / 2;

        // Draw label on left
        font::draw_string_raw(fb, self.x + 10, text_y, self.label, colors::BUTTON_TEXT, scale);

        // Draw slider bar
        let bar_x = self.x + self.width / 2;
        let bar_width = self.width / 2 - 40;
        let bar_y = self.y + self.height / 2 - 4;
        let bar_height = 8;

        // Draw bar background
        for y in bar_y..(bar_y + bar_height).min(fb.height) {
            for x in bar_x..(bar_x + bar_width).min(fb.width) {
                fb.put_pixel(x, y, colors::PANEL_BG);
            }
        }

        // Draw filled portion
        let fill_ratio = (self.value - self.min) as f32 / (self.max - self.min) as f32;
        let fill_width = (bar_width as f32 * fill_ratio) as usize;
        for y in bar_y..(bar_y + bar_height).min(fb.height) {
            for x in bar_x..(bar_x + fill_width).min(fb.width) {
                fb.put_pixel(x, y, colors::FN_BLUE);
            }
        }

        // Draw value on right
        let mut buf = [0u8; 8];
        let value_str = font::format_number(self.value as u32, &mut buf);
        let value_width = font::string_width(value_str, scale);
        let value_x = self.x + self.width - value_width - 10;
        font::draw_string_raw(fb, value_x, text_y, value_str, colors::BUTTON_TEXT, scale);
    }

    pub fn increase(&mut self) {
        if self.value < self.max {
            self.value += 1;
        }
    }

    pub fn decrease(&mut self) {
        if self.value > self.min {
            self.value -= 1;
        }
    }
}
