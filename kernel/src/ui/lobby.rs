//! Lobby screen

use alloc::vec::Vec;
use crate::game::state::{GameState, LobbyPlayer, MenuAction, PLAYER_CUSTOMIZATION};
use crate::graphics::font;
use crate::graphics::framebuffer::FRAMEBUFFER;
use crate::graphics::rasterizer::RenderContext;
use crate::graphics::ui::colors;
use crate::graphics::ui::list::PlayerList;
use crate::graphics::ui::panel::{draw_gradient_background_raw, draw_panel_raw, fill_rect_raw};

/// Lobby screen state
pub struct LobbyScreen {
    pub players: Vec<LobbyPlayer>,
    pub local_player_id: Option<u8>,
    pub local_ready: bool,
    pub auto_start_timer: u8,
    pub countdown_active: bool,
    pub countdown_value: u8,
    pub player_list: PlayerList,
    pub fb_width: usize,
    pub fb_height: usize,
}

impl LobbyScreen {
    pub fn new(fb_width: usize, fb_height: usize) -> Self {
        // Create a local player for testing
        let local_player = LobbyPlayer::new(0, "Player");

        Self {
            players: alloc::vec![local_player],
            local_player_id: Some(0),
            local_ready: false,
            auto_start_timer: 60,
            countdown_active: false,
            countdown_value: 5,
            player_list: PlayerList::new(50, 150, 400, 450),
            fb_width,
            fb_height,
        }
    }

    /// Add a player to the lobby
    pub fn add_player(&mut self, name: &str) -> u8 {
        let id = self.players.len() as u8;
        let mut player = LobbyPlayer::new(id, name);
        player.customization = *PLAYER_CUSTOMIZATION.lock();
        self.players.push(player);
        id
    }

    /// Remove a player from the lobby
    pub fn remove_player(&mut self, id: u8) {
        self.players.retain(|p| p.id != id);
    }

    /// Set player ready status
    pub fn set_ready(&mut self, id: u8, ready: bool) {
        if let Some(player) = self.players.iter_mut().find(|p| p.id == id) {
            player.ready = ready;
        }
    }

    /// Count ready players
    pub fn ready_count(&self) -> usize {
        self.players.iter().filter(|p| p.ready).count()
    }

    /// Check if all players are ready
    pub fn all_ready(&self) -> bool {
        !self.players.is_empty() && self.players.iter().all(|p| p.ready)
    }

    /// Handle input and return new state if transitioning
    pub fn update(&mut self, action: MenuAction) -> Option<GameState> {
        match action {
            MenuAction::Select => {
                // Toggle ready with Enter/Space
                if let Some(id) = self.local_player_id {
                    self.local_ready = !self.local_ready;
                    self.set_ready(id, self.local_ready);
                }
            }
            MenuAction::Back => {
                // Leave lobby with Escape
                return Some(GameState::MainMenu);
            }
            MenuAction::Up => {
                self.player_list.scroll_up();
            }
            MenuAction::Down => {
                self.player_list.scroll_down(self.players.len());
            }
            _ => {}
        }

        // Update auto-start timer
        if !self.countdown_active {
            // Check if all ready to start countdown
            if self.all_ready() && self.players.len() >= 1 {
                self.countdown_active = true;
                self.countdown_value = 5;
                return Some(GameState::Countdown { remaining_secs: 5 });
            }
        } else {
            // Countdown is active
            if self.countdown_value == 0 {
                return Some(GameState::BusPhase);
            }
        }

        None
    }

    /// Decrement countdown (call from main loop with timer)
    pub fn tick_countdown(&mut self) {
        if self.countdown_active && self.countdown_value > 0 {
            self.countdown_value -= 1;
        }
    }

    /// Draw the lobby screen
    pub fn draw(&self, _ctx: &RenderContext, fb_width: usize, fb_height: usize) {
        let fb_guard = FRAMEBUFFER.lock();
        let fb = match fb_guard.as_ref() {
            Some(f) => f,
            None => return,
        };

        // Draw gradient background
        draw_gradient_background_raw(fb, fb_width, fb_height);

        // Draw title
        let title = "LOBBY";
        let title_scale = 4;
        font::draw_string_raw(fb, 50, 40, title, colors::TITLE, title_scale);

        // Draw player count
        let mut count_buf = [0u8; 16];
        let count_str = format_player_count(self.players.len(), &mut count_buf);
        font::draw_string_raw(fb, 50, 100, count_str, colors::WHITE, 3);

        // Draw player list
        self.player_list.draw(fb, &self.players);

        // === RIGHT PANEL: Status ===
        let status_x = 500;
        let status_y = 150;
        let status_width = 350;
        let status_height = 300;

        draw_panel_raw(fb, status_x, status_y, status_width, status_height, colors::PANEL_BG);

        let scale = 2;
        let line_height = 40;
        let mut y = status_y + 20;

        // Ready count
        let ready = self.ready_count();
        let total = self.players.len();
        let mut ready_buf = [0u8; 32];
        let ready_str = format_ready_count(ready, total, &mut ready_buf);
        font::draw_string_raw(fb, status_x + 20, y, ready_str, colors::WHITE, scale);

        y += line_height;

        // Auto-start timer or countdown
        if self.countdown_active {
            font::draw_string_raw(fb, status_x + 20, y, "STARTING IN:", colors::FN_YELLOW, scale);
            y += line_height;

            // Draw countdown number (large)
            let mut num_buf = [0u8; 4];
            let num_str = font::format_number(self.countdown_value as u32, &mut num_buf);
            font::draw_string_raw(fb, status_x + 100, y, num_str, colors::TITLE, 6);
        } else {
            font::draw_string_raw(fb, status_x + 20, y, "WAITING FOR PLAYERS", colors::SUBTITLE, scale);
            y += line_height;

            // Auto-start timer
            let mut timer_buf = [0u8; 32];
            let timer_str = format_timer(self.auto_start_timer, &mut timer_buf);
            font::draw_string_raw(fb, status_x + 20, y, timer_str, colors::SUBTITLE, scale);
        }

        // === READY BUTTON ===
        let button_x = status_x + 25;
        let button_y = status_y + status_height - 80;
        let button_width = status_width - 50;
        let button_height = 60;

        let button_color = if self.local_ready {
            colors::READY
        } else {
            colors::BUTTON_NORMAL
        };

        fill_rect_raw(fb, button_x, button_y, button_width, button_height, button_color);

        // Button border
        for x in button_x..(button_x + button_width).min(fb.width) {
            if button_y < fb.height {
                fb.put_pixel(x, button_y, colors::PANEL_BORDER);
            }
            if button_y + button_height - 1 < fb.height {
                fb.put_pixel(x, button_y + button_height - 1, colors::PANEL_BORDER);
            }
        }
        for y in button_y..(button_y + button_height).min(fb.height) {
            if button_x < fb.width {
                fb.put_pixel(button_x, y, colors::PANEL_BORDER);
            }
            if button_x + button_width - 1 < fb.width {
                fb.put_pixel(button_x + button_width - 1, y, colors::PANEL_BORDER);
            }
        }

        let button_text = if self.local_ready { "READY!" } else { "PRESS ENTER" };
        let text_width = font::string_width(button_text, scale);
        let text_x = button_x + (button_width - text_width) / 2;
        let text_y = button_y + (button_height - font::char_height(scale)) / 2;
        font::draw_string_raw(fb, text_x, text_y, button_text, colors::WHITE, scale);

        // Footer
        let footer = "ENTER: READY/UNREADY   ESC: LEAVE";
        font::draw_string_centered_raw(fb, fb_height - 40, footer, colors::SUBTITLE, 2);
    }
}

/// Format player count "PLAYERS: X/100"
fn format_player_count<'a>(count: usize, buf: &'a mut [u8; 16]) -> &'a str {
    let prefix = b"PLAYERS: ";
    let mut pos = 0;

    for &b in prefix {
        buf[pos] = b;
        pos += 1;
    }

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

    unsafe { core::str::from_utf8_unchecked(&buf[..pos]) }
}

/// Format ready count "READY: X/Y"
fn format_ready_count<'a>(ready: usize, total: usize, buf: &'a mut [u8; 32]) -> &'a str {
    let prefix = b"READY: ";
    let mut pos = 0;

    for &b in prefix {
        buf[pos] = b;
        pos += 1;
    }

    // Ready count
    if ready == 0 {
        buf[pos] = b'0';
        pos += 1;
    } else {
        let mut n = ready;
        let start = pos;
        while n > 0 && pos < 32 {
            buf[pos] = b'0' + (n % 10) as u8;
            n /= 10;
            pos += 1;
        }
        buf[start..pos].reverse();
    }

    buf[pos] = b'/';
    pos += 1;

    // Total count
    if total == 0 {
        buf[pos] = b'0';
        pos += 1;
    } else {
        let mut n = total;
        let start = pos;
        while n > 0 && pos < 32 {
            buf[pos] = b'0' + (n % 10) as u8;
            n /= 10;
            pos += 1;
        }
        buf[start..pos].reverse();
    }

    unsafe { core::str::from_utf8_unchecked(&buf[..pos]) }
}

/// Format timer "AUTO START: XXs"
fn format_timer<'a>(seconds: u8, buf: &'a mut [u8; 32]) -> &'a str {
    let prefix = b"AUTO START: ";
    let mut pos = 0;

    for &b in prefix {
        buf[pos] = b;
        pos += 1;
    }

    // Seconds
    if seconds == 0 {
        buf[pos] = b'0';
        pos += 1;
    } else {
        let mut n = seconds as usize;
        let start = pos;
        while n > 0 && pos < 32 {
            buf[pos] = b'0' + (n % 10) as u8;
            n /= 10;
            pos += 1;
        }
        buf[start..pos].reverse();
    }

    buf[pos] = b's';
    pos += 1;

    unsafe { core::str::from_utf8_unchecked(&buf[..pos]) }
}
