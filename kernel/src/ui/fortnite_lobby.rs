//! Fortnite-style 3D lobby screen
//!
//! A modern lobby with a 3D player preview on a glowing platform,
//! tropical background, and game mode selection.

use alloc::vec::Vec;
use crate::game::state::{
    GameState, LobbyPlayer, MenuAction, NetworkMode,
    PLAYER_CUSTOMIZATION, get_network_mode,
};
use crate::graphics::font;
use crate::graphics::framebuffer::FRAMEBUFFER;
use crate::graphics::rasterizer::RenderContext;
use crate::graphics::ui::colors;
use crate::graphics::ui::panel::{draw_panel_raw, fill_rect_raw};

/// Lobby tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LobbyTab {
    Play,
    Locker,
    ItemShop,
    Career,
}

impl LobbyTab {
    pub const COUNT: usize = 4;

    pub fn from_index(index: usize) -> Self {
        match index % Self::COUNT {
            0 => Self::Play,
            1 => Self::Locker,
            2 => Self::ItemShop,
            _ => Self::Career,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Play => "PLAY",
            Self::Locker => "LOCKER",
            Self::ItemShop => "ITEM SHOP",
            Self::Career => "CAREER",
        }
    }
}

/// Game mode selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    Solo,
    Duos,
    Squads,
}

impl GameMode {
    pub const COUNT: usize = 3;

    pub fn from_index(index: usize) -> Self {
        match index % Self::COUNT {
            0 => Self::Solo,
            1 => Self::Duos,
            _ => Self::Squads,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Solo => "SOLO",
            Self::Duos => "DUOS",
            Self::Squads => "SQUADS",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Solo => "100 players, no teammates",
            Self::Duos => "50 teams of 2",
            Self::Squads => "25 teams of 4",
        }
    }
}

/// Fortnite-style lobby screen
pub struct FortniteLobby {
    /// Currently selected tab
    pub selected_tab: usize,
    /// Selected game mode
    pub selected_mode: usize,
    /// Player rotation for 3D preview
    pub player_rotation: f32,
    /// Is player ready to start
    pub is_ready: bool,
    /// Players in lobby
    pub players: Vec<LobbyPlayer>,
    /// Local player ID
    pub local_player_id: Option<u8>,
    /// Countdown active
    pub countdown_active: bool,
    /// Countdown value
    pub countdown_value: u8,
    /// Framebuffer dimensions
    pub fb_width: usize,
    pub fb_height: usize,
}

impl FortniteLobby {
    pub fn new(fb_width: usize, fb_height: usize) -> Self {
        // Create local player
        let local_player = LobbyPlayer::new(0, "Player");

        Self {
            selected_tab: 0,
            selected_mode: 0,
            player_rotation: 0.0,
            is_ready: false,
            players: alloc::vec![local_player],
            local_player_id: Some(0),
            countdown_active: false,
            countdown_value: 5,
            fb_width,
            fb_height,
        }
    }

    /// Get player rotation for 3D rendering
    pub fn get_rotation(&self) -> f32 {
        self.player_rotation
    }

    /// Update rotation each frame
    pub fn tick(&mut self) {
        self.player_rotation += 0.01;
        if self.player_rotation > core::f32::consts::TAU {
            self.player_rotation -= core::f32::consts::TAU;
        }
    }

    /// Handle input and return new state if transitioning
    pub fn update(&mut self, action: MenuAction) -> Option<GameState> {
        match action {
            MenuAction::Left => {
                // Switch tabs or game mode
                if self.selected_tab == 0 {
                    // On play tab, switch game mode
                    if self.selected_mode == 0 {
                        self.selected_mode = GameMode::COUNT - 1;
                    } else {
                        self.selected_mode -= 1;
                    }
                } else {
                    // Switch tabs
                    if self.selected_tab == 0 {
                        self.selected_tab = LobbyTab::COUNT - 1;
                    } else {
                        self.selected_tab -= 1;
                    }
                }
            }
            MenuAction::Right => {
                if self.selected_tab == 0 {
                    self.selected_mode = (self.selected_mode + 1) % GameMode::COUNT;
                } else {
                    self.selected_tab = (self.selected_tab + 1) % LobbyTab::COUNT;
                }
            }
            MenuAction::Up => {
                // Switch to previous tab
                if self.selected_tab == 0 {
                    self.selected_tab = LobbyTab::COUNT - 1;
                } else {
                    self.selected_tab -= 1;
                }
            }
            MenuAction::Down => {
                // Switch to next tab
                self.selected_tab = (self.selected_tab + 1) % LobbyTab::COUNT;
            }
            MenuAction::Select => {
                match LobbyTab::from_index(self.selected_tab) {
                    LobbyTab::Play => {
                        // Toggle ready or start game
                        if let Some(id) = self.local_player_id {
                            self.is_ready = !self.is_ready;
                            if let Some(player) = self.players.get_mut(id as usize) {
                                player.ready = self.is_ready;
                            }
                        }

                        // Check if all players ready
                        if self.is_ready && self.all_ready() {
                            self.countdown_active = true;
                            // Start matchmaking (will be skipped in offline mode)
                            return Some(GameState::Matchmaking { elapsed_secs: 0 });
                        }
                    }
                    LobbyTab::Locker => {
                        return Some(GameState::Customization);
                    }
                    _ => {
                        // Other tabs not implemented yet
                    }
                }
            }
            MenuAction::Back => {
                // Back from party lobby goes to settings (no main menu anymore)
                return Some(GameState::Settings);
            }
            _ => {}
        }

        None
    }

    /// Check if 'T' key pressed to enter test map
    pub fn check_test_map_key(&self, t_pressed: bool) -> Option<GameState> {
        if t_pressed {
            return Some(GameState::TestMap);
        }
        None
    }

    /// Check if all players are ready
    fn all_ready(&self) -> bool {
        !self.players.is_empty() && self.players.iter().all(|p| p.ready)
    }

    /// Get ready count
    fn ready_count(&self) -> usize {
        self.players.iter().filter(|p| p.ready).count()
    }

    /// Draw the lobby screen (full, including background)
    pub fn draw(&self, _ctx: &RenderContext, fb_width: usize, fb_height: usize) {
        self.draw_ui_only(_ctx, fb_width, fb_height, false);
    }

    /// Draw only the UI elements (no background or 3D preview)
    /// Used when 3D player preview is rendered separately
    pub fn draw_ui_only(&self, _ctx: &RenderContext, fb_width: usize, fb_height: usize, skip_background: bool) {
        let fb_guard = FRAMEBUFFER.lock();
        let fb = match fb_guard.as_ref() {
            Some(f) => f,
            None => return,
        };

        // Only draw background if not skipped (when 3D is rendered separately)
        if !skip_background {
            self.draw_sunset_background(fb, fb_width, fb_height);
            // Draw silhouette only when no 3D rendering
            self.draw_player_preview_silhouette(fb, fb_width, fb_height);
        }

        // Draw header bar with tabs
        self.draw_header(fb, fb_width);

        // Draw player info panel (right side)
        self.draw_player_info(fb, fb_width, fb_height);

        // Draw bottom bar with play button and mode selection
        self.draw_bottom_bar(fb, fb_width, fb_height);

        // Draw network status bar
        self.draw_network_status(fb, fb_width, fb_height);
    }

    fn draw_sunset_background(&self, fb: &crate::graphics::framebuffer::Framebuffer, fb_width: usize, fb_height: usize) {
        // Sunset gradient: orange -> pink -> purple -> dark blue
        let colors_top = [0xFF, 0x8C, 0x00]; // Orange
        let colors_mid1 = [0xFF, 0x69, 0xB4]; // Pink
        let colors_mid2 = [0x94, 0x00, 0xD3]; // Purple
        let colors_bot = [0x19, 0x19, 0x70];  // Dark blue

        for y in 0..fb_height.min(fb.height) {
            let t = y as f32 / fb_height as f32;

            let (r, g, b) = if t < 0.3 {
                // Top section (orange to pink)
                let local_t = t / 0.3;
                (
                    lerp_u8(colors_top[0], colors_mid1[0], local_t),
                    lerp_u8(colors_top[1], colors_mid1[1], local_t),
                    lerp_u8(colors_top[2], colors_mid1[2], local_t),
                )
            } else if t < 0.6 {
                // Middle section (pink to purple)
                let local_t = (t - 0.3) / 0.3;
                (
                    lerp_u8(colors_mid1[0], colors_mid2[0], local_t),
                    lerp_u8(colors_mid1[1], colors_mid2[1], local_t),
                    lerp_u8(colors_mid1[2], colors_mid2[2], local_t),
                )
            } else {
                // Bottom section (purple to dark blue)
                let local_t = (t - 0.6) / 0.4;
                (
                    lerp_u8(colors_mid2[0], colors_bot[0], local_t),
                    lerp_u8(colors_mid2[1], colors_bot[1], local_t),
                    lerp_u8(colors_mid2[2], colors_bot[2], local_t),
                )
            };

            let color = ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);

            for x in 0..fb_width.min(fb.width) {
                fb.put_pixel(x, y, color);
            }
        }

        // Draw some palm tree silhouettes (simplified)
        self.draw_palm_silhouettes(fb, fb_width, fb_height);
    }

    fn draw_palm_silhouettes(&self, fb: &crate::graphics::framebuffer::Framebuffer, fb_width: usize, fb_height: usize) {
        let palm_color = 0x0A0A20; // Very dark silhouette

        // Left palm tree
        let palm1_x = fb_width / 8;
        let palm1_base = fb_height - 100;
        self.draw_palm_silhouette(fb, palm1_x, palm1_base, 80, palm_color);

        // Right palm tree
        let palm2_x = fb_width * 7 / 8;
        let palm2_base = fb_height - 120;
        self.draw_palm_silhouette(fb, palm2_x, palm2_base, 100, palm_color);
    }

    fn draw_palm_silhouette(&self, fb: &crate::graphics::framebuffer::Framebuffer, x: usize, base_y: usize, height: usize, color: u32) {
        // Trunk
        let trunk_width = 8;
        for y in 0..height {
            let actual_y = base_y - y;
            if actual_y < fb.height {
                for dx in 0..trunk_width {
                    let actual_x = x + dx;
                    if actual_x < fb.width {
                        fb.put_pixel(actual_x, actual_y, color);
                    }
                }
            }
        }

        // Fronds (simplified as triangles)
        let frond_y = base_y - height;
        let frond_width = 60;
        let frond_height = 30;

        // Draw multiple fronds at angles
        for angle_idx in 0..5 {
            let angle = (angle_idx as f32 - 2.0) * 0.5; // -1.0 to 1.0
            let frond_x = x + trunk_width / 2;

            for i in 0..frond_height {
                let width_at_point = (frond_width * (frond_height - i) / frond_height) as i32;
                let offset_x = (angle * i as f32 * 1.5) as i32;
                let actual_y = frond_y - i;

                if actual_y < fb.height {
                    for dx in -width_at_point/2..=width_at_point/2 {
                        let actual_x = (frond_x as i32 + offset_x + dx) as usize;
                        if actual_x < fb.width {
                            fb.put_pixel(actual_x, actual_y, color);
                        }
                    }
                }
            }
        }
    }

    fn draw_header(&self, fb: &crate::graphics::framebuffer::Framebuffer, fb_width: usize) {
        // Header background
        fill_rect_raw(fb, 0, 0, fb_width, 60, 0x20102030);

        // Game title
        font::draw_string_raw(fb, 20, 15, "BATTLE ROYALE", colors::TITLE, 3);

        // Tab buttons
        let tab_start_x = 300;
        let tab_width = 120;
        let tab_spacing = 10;

        for i in 0..LobbyTab::COUNT {
            let tab = LobbyTab::from_index(i);
            let tab_x = tab_start_x + i * (tab_width + tab_spacing);
            let selected = i == self.selected_tab;

            let bg_color = if selected { 0x504080C0 } else { 0x30304060 };
            fill_rect_raw(fb, tab_x, 10, tab_width, 40, bg_color);

            let text_color = if selected { colors::WHITE } else { colors::SUBTITLE };
            let label = tab.label();
            let text_x = tab_x + (tab_width - font::string_width(label, 2)) / 2;
            font::draw_string_raw(fb, text_x, 18, label, text_color, 2);
        }
    }

    fn draw_player_preview_silhouette(&self, fb: &crate::graphics::framebuffer::Framebuffer, fb_width: usize, fb_height: usize) {
        // Draw glowing platform (simplified)
        let platform_center_x = fb_width / 3;
        let platform_y = fb_height - 200;
        let platform_width = 200;
        let platform_height = 20;

        // Glow effect
        for glow in 0..10 {
            let alpha = (10 - glow) as u32 * 15;
            let color = (0x40 << 16) | ((0x80 + alpha) << 8) | 0xFF;
            fill_rect_raw(
                fb,
                platform_center_x - platform_width / 2 - glow * 2,
                platform_y - glow,
                platform_width + glow * 4,
                platform_height + glow * 2,
                color,
            );
        }

        // Platform surface
        fill_rect_raw(
            fb,
            platform_center_x - platform_width / 2,
            platform_y,
            platform_width,
            platform_height,
            0x6080C0FF,
        );

        // Player silhouette label (3D model rendered separately)
        font::draw_string_raw(
            fb,
            platform_center_x - 50,
            platform_y - 100,
            "[3D PLAYER]",
            colors::SUBTITLE,
            2,
        );
    }

    fn draw_player_info(&self, fb: &crate::graphics::framebuffer::Framebuffer, fb_width: usize, _fb_height: usize) {
        let panel_x = fb_width * 2 / 3;
        let panel_y = 100;
        let panel_width = fb_width / 3 - 20;
        let panel_height = 200;

        draw_panel_raw(fb, panel_x, panel_y, panel_width, panel_height, 0x30203040);

        // Player name
        let custom = PLAYER_CUSTOMIZATION.lock();
        font::draw_string_raw(fb, panel_x + 20, panel_y + 20, "Player", colors::WHITE, 3);

        // Level
        font::draw_string_raw(fb, panel_x + 20, panel_y + 60, "Level: 1", colors::SUBTITLE, 2);

        // Ready status
        let ready_text = if self.is_ready { "READY!" } else { "NOT READY" };
        let ready_color = if self.is_ready { colors::READY } else { colors::NOT_READY };
        font::draw_string_raw(fb, panel_x + 20, panel_y + 100, ready_text, ready_color, 2);

        drop(custom);
    }

    fn draw_bottom_bar(&self, fb: &crate::graphics::framebuffer::Framebuffer, fb_width: usize, fb_height: usize) {
        let bar_y = fb_height - 100;
        let bar_height = 80;

        // Semi-transparent background
        fill_rect_raw(fb, 0, bar_y, fb_width, bar_height, 0x40102030);

        // Game mode selector (left side)
        let mode = GameMode::from_index(self.selected_mode);
        font::draw_string_raw(fb, 30, bar_y + 10, "BATTLE ROYALE", colors::WHITE, 2);

        // Mode dropdown
        let mode_label = mode.label();
        font::draw_string_raw(fb, 30, bar_y + 40, mode_label, colors::FN_YELLOW, 3);

        // Left/right arrows for mode
        font::draw_string_raw(fb, 10, bar_y + 40, "<", colors::SUBTITLE, 3);
        font::draw_string_raw(fb, 30 + font::string_width(mode_label, 3) + 10, bar_y + 40, ">", colors::SUBTITLE, 3);

        // PLAY button (right side)
        let play_button_width = 200;
        let play_button_height = 60;
        let play_button_x = fb_width - play_button_width - 30;
        let play_button_y = bar_y + (bar_height - play_button_height) / 2;

        let button_color = if self.is_ready { colors::READY } else { colors::FN_YELLOW };
        fill_rect_raw(fb, play_button_x, play_button_y, play_button_width, play_button_height, button_color);

        let play_text = if self.is_ready { "READY!" } else { "PLAY" };
        let text_width = font::string_width(play_text, 3);
        let text_x = play_button_x + (play_button_width - text_width) / 2;
        let text_y = play_button_y + (play_button_height - 24) / 2;
        font::draw_string_raw(fb, text_x, text_y, play_text, colors::BLACK, 3);

        // Player count
        let mut count_buf = [0u8; 16];
        let count_str = format_player_count(self.players.len(), 100, &mut count_buf);
        font::draw_string_raw(fb, play_button_x, play_button_y - 25, count_str, colors::SUBTITLE, 2);
    }

    fn draw_network_status(&self, fb: &crate::graphics::framebuffer::Framebuffer, fb_width: usize, fb_height: usize) {
        let status_y = fb_height - 25;

        let network_mode = get_network_mode();
        let mut status_buf = [0u8; 64];

        let status_str = match network_mode {
            NetworkMode::Offline => "OFFLINE MODE",
            NetworkMode::Server { port } => {
                format_server_status(port, &mut status_buf)
            }
            NetworkMode::Client { server_ip, port } => {
                format_client_status(&server_ip, port, &mut status_buf)
            }
        };

        font::draw_string_raw(fb, 10, status_y, status_str, colors::SUBTITLE, 1);

        // Test map hint
        let hint = "Press T for Model Viewer";
        let hint_x = fb_width - font::string_width(hint, 1) - 10;
        font::draw_string_raw(fb, hint_x, status_y, hint, colors::SUBTITLE, 1);
    }
}

/// Linear interpolation for u8
fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    ((a as f32) + (b as f32 - a as f32) * t) as u8
}

/// Format player count string
fn format_player_count<'a>(current: usize, max: usize, buf: &'a mut [u8; 16]) -> &'a str {
    let mut pos = 0;

    pos = write_num(buf, pos, current);

    buf[pos] = b'/';
    pos += 1;

    pos = write_num(buf, pos, max);

    let suffix = b" Players";
    for &b in suffix {
        if pos < buf.len() {
            buf[pos] = b;
            pos += 1;
        }
    }

    unsafe { core::str::from_utf8_unchecked(&buf[..pos]) }
}

/// Format server status
fn format_server_status<'a>(port: u16, buf: &'a mut [u8; 64]) -> &'a str {
    let prefix = b"SERVER: 10.0.2.15:";
    let mut pos = 0;

    for &b in prefix {
        buf[pos] = b;
        pos += 1;
    }

    pos = write_num(buf, pos, port as usize);

    unsafe { core::str::from_utf8_unchecked(&buf[..pos]) }
}

/// Format client status
fn format_client_status<'a>(ip: &[u8; 4], port: u16, buf: &'a mut [u8; 64]) -> &'a str {
    let prefix = b"CONNECTED TO: ";
    let mut pos = 0;

    for &b in prefix {
        buf[pos] = b;
        pos += 1;
    }

    for i in 0..4 {
        pos = write_num(buf, pos, ip[i] as usize);
        if i < 3 {
            buf[pos] = b'.';
            pos += 1;
        }
    }

    buf[pos] = b':';
    pos += 1;

    pos = write_num(buf, pos, port as usize);

    unsafe { core::str::from_utf8_unchecked(&buf[..pos]) }
}

/// Write number to buffer
fn write_num(buf: &mut [u8], start: usize, value: usize) -> usize {
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
