//! List UI primitives - player lists and scrollable content

use crate::game::state::LobbyPlayer;
use crate::graphics::font;
use crate::graphics::framebuffer::Framebuffer;
use super::colors;
use super::panel::draw_panel_raw;

/// A scrollable player list for lobby
pub struct PlayerList {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub scroll_offset: usize,
    pub items_per_page: usize,
}

impl PlayerList {
    pub fn new(x: usize, y: usize, width: usize, height: usize) -> Self {
        let item_height = 40;
        let items_per_page = (height - 20) / item_height;

        Self {
            x,
            y,
            width,
            height,
            scroll_offset: 0,
            items_per_page,
        }
    }

    /// Draw the player list
    pub fn draw(&self, fb: &Framebuffer, players: &[LobbyPlayer]) {
        // Draw panel background
        draw_panel_raw(fb, self.x, self.y, self.width, self.height, colors::PANEL_BG);

        let item_height = 40;
        let padding = 10;
        let scale = 2;

        // Draw header
        font::draw_string_raw(
            fb,
            self.x + padding,
            self.y + 8,
            "PLAYERS",
            colors::SUBTITLE,
            scale,
        );

        // Draw player count
        let mut count_buf = [0u8; 16];
        let count_str = format_player_count(players.len(), &mut count_buf);
        let count_width = font::string_width(count_str, scale);
        font::draw_string_raw(
            fb,
            self.x + self.width - count_width - padding,
            self.y + 8,
            count_str,
            colors::WHITE,
            scale,
        );

        // Draw divider
        let divider_y = self.y + 32;
        for x in (self.x + 4)..(self.x + self.width - 4) {
            if divider_y < fb.height {
                fb.put_pixel(x, divider_y, colors::PANEL_BORDER);
            }
        }

        // Draw players
        let start_y = divider_y + 8;
        let visible_end = (self.scroll_offset + self.items_per_page).min(players.len());

        for (i, player) in players.iter().enumerate().skip(self.scroll_offset).take(self.items_per_page) {
            let item_y = start_y + (i - self.scroll_offset) * item_height;

            if item_y + item_height > self.y + self.height {
                break;
            }

            // Draw player row
            self.draw_player_row(fb, player, item_y, padding, scale);
        }

        // Draw scroll indicators if needed
        if self.scroll_offset > 0 {
            // Up arrow
            font::draw_string_raw(
                fb,
                self.x + self.width - 20,
                start_y,
                "^",
                colors::WHITE,
                1,
            );
        }

        if visible_end < players.len() {
            // Down arrow (using V as approximation)
            font::draw_string_raw(
                fb,
                self.x + self.width - 20,
                self.y + self.height - 20,
                "V",
                colors::WHITE,
                1,
            );
        }
    }

    fn draw_player_row(&self, fb: &Framebuffer, player: &LobbyPlayer, y: usize, padding: usize, scale: usize) {
        // Ready indicator
        let indicator_color = if player.ready {
            colors::READY
        } else {
            colors::NOT_READY
        };

        // Draw ready dot
        let dot_x = self.x + padding;
        let dot_y = y + 12;
        let dot_size = 8;
        for dy in 0..dot_size {
            for dx in 0..dot_size {
                let dist_sq = (dx as i32 - 4) * (dx as i32 - 4) + (dy as i32 - 4) * (dy as i32 - 4);
                if dist_sq <= 16 && dot_x + dx < fb.width && dot_y + dy < fb.height {
                    fb.put_pixel(dot_x + dx, dot_y + dy, indicator_color);
                }
            }
        }

        // Draw player name
        let name_x = dot_x + dot_size + 10;
        font::draw_string_raw(fb, name_x, y + 8, player.name_str(), colors::WHITE, scale);

        // Draw player ID
        let mut id_buf = [0u8; 4];
        let id_str = font::format_number(player.id as u32 + 1, &mut id_buf);
        font::draw_string_raw(
            fb,
            self.x + self.width - padding - 30,
            y + 8,
            "#",
            colors::SUBTITLE,
            scale,
        );
        font::draw_string_raw(
            fb,
            self.x + self.width - padding - 10,
            y + 8,
            id_str,
            colors::SUBTITLE,
            scale,
        );
    }

    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    pub fn scroll_down(&mut self, total_items: usize) {
        if self.scroll_offset + self.items_per_page < total_items {
            self.scroll_offset += 1;
        }
    }
}

/// Format player count as "X/100"
fn format_player_count<'a>(count: usize, buf: &'a mut [u8; 16]) -> &'a str {
    let mut pos = 0;

    // Write count
    if count == 0 {
        buf[pos] = b'0';
        pos += 1;
    } else {
        let mut n = count;
        let start = pos;
        while n > 0 && pos < 16 {
            buf[pos] = b'0' + (n % 10) as u8;
            n /= 10;
            pos += 1;
        }
        buf[start..pos].reverse();
    }

    // Separator
    buf[pos] = b'/';
    pos += 1;

    // Max (100)
    buf[pos] = b'1';
    pos += 1;
    buf[pos] = b'0';
    pos += 1;
    buf[pos] = b'0';
    pos += 1;

    unsafe { core::str::from_utf8_unchecked(&buf[..pos]) }
}

/// A kill feed entry
pub struct KillFeedEntry {
    pub killer: [u8; 16],
    pub victim: [u8; 16],
    pub headshot: bool,
    pub timestamp: u32,
}

/// Kill feed display
pub struct KillFeed {
    pub entries: [Option<KillFeedEntry>; 5],
    pub count: usize,
}

impl KillFeed {
    pub fn new() -> Self {
        Self {
            entries: [None, None, None, None, None],
            count: 0,
        }
    }

    pub fn add(&mut self, killer: &str, victim: &str, headshot: bool, timestamp: u32) {
        // Shift entries down
        for i in (1..5).rev() {
            self.entries[i] = self.entries[i - 1].take();
        }

        // Add new entry
        let mut killer_buf = [0u8; 16];
        let mut victim_buf = [0u8; 16];
        let k_len = killer.len().min(16);
        let v_len = victim.len().min(16);
        killer_buf[..k_len].copy_from_slice(&killer.as_bytes()[..k_len]);
        victim_buf[..v_len].copy_from_slice(&victim.as_bytes()[..v_len]);

        self.entries[0] = Some(KillFeedEntry {
            killer: killer_buf,
            victim: victim_buf,
            headshot,
            timestamp,
        });

        self.count = (self.count + 1).min(5);
    }

    pub fn draw(&self, fb: &Framebuffer, fb_width: usize, current_time: u32) {
        let scale = 1;
        let line_height = 20;
        let x = fb_width - 300;
        let mut y = 50;

        for entry in &self.entries {
            if let Some(e) = entry {
                // Fade out after 5 seconds
                if current_time > e.timestamp + 5000 {
                    continue;
                }

                let killer_str = entry_name_str(&e.killer);
                let victim_str = entry_name_str(&e.victim);

                // Draw: "Killer [weapon] Victim"
                font::draw_string_raw(fb, x, y, killer_str, colors::WHITE, scale);

                let arrow_x = x + font::string_width(killer_str, scale) + 5;
                let arrow_color = if e.headshot { colors::FN_YELLOW } else { colors::SUBTITLE };
                font::draw_string_raw(fb, arrow_x, y, ">", arrow_color, scale);

                let victim_x = arrow_x + 15;
                font::draw_string_raw(fb, victim_x, y, victim_str, colors::HEALTH_LOW, scale);

                y += line_height;
            }
        }
    }
}

fn entry_name_str(name: &[u8; 16]) -> &str {
    let end = name.iter().position(|&b| b == 0).unwrap_or(16);
    core::str::from_utf8(&name[..end]).unwrap_or("???")
}
