//! Server/Client selection screen
//!
//! Allows players to choose between hosting a server, joining a server, or playing offline.

use crate::game::state::{GameState, MenuAction, NetworkMode, set_network_mode};
use crate::graphics::font;
use crate::graphics::framebuffer::FRAMEBUFFER;
use crate::graphics::rasterizer::RenderContext;
use crate::graphics::ui::colors;
use crate::graphics::ui::panel::{draw_gradient_background_raw, draw_panel_raw, fill_rect_raw};

/// Server mode options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerMode {
    /// Host a game server
    Host,
    /// Join an existing server
    Join,
    /// Play offline (single player)
    Offline,
}

impl ServerMode {
    pub const COUNT: usize = 3;

    pub fn from_index(index: usize) -> Self {
        match index % Self::COUNT {
            0 => Self::Host,
            1 => Self::Join,
            _ => Self::Offline,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Host => "HOST GAME",
            Self::Join => "JOIN GAME",
            Self::Offline => "OFFLINE",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Host => "Start a server and wait for players",
            Self::Join => "Connect to an existing server",
            Self::Offline => "Play single player with bots",
        }
    }
}

/// Input mode for IP entry
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputMode {
    /// Selecting mode (Host/Join/Offline)
    ModeSelect,
    /// Entering IP address
    IpEntry,
}

/// Server selection screen state
pub struct ServerSelectScreen {
    /// Selected server mode
    pub selected_mode: usize,
    /// IP address octets (for Join mode)
    pub ip_octets: [u8; 4],
    /// Currently editing IP octet (0-3)
    pub ip_cursor: usize,
    /// Port number
    pub port: u16,
    /// Current input mode
    input_mode: InputMode,
    /// Local IP address for display
    pub local_ip: [u8; 4],
    /// Framebuffer dimensions
    pub fb_width: usize,
    pub fb_height: usize,
}

impl ServerSelectScreen {
    pub fn new(fb_width: usize, fb_height: usize) -> Self {
        Self {
            selected_mode: 2, // Default to Offline
            ip_octets: [192, 168, 1, 1],
            ip_cursor: 0,
            port: 5000,
            input_mode: InputMode::ModeSelect,
            local_ip: [10, 0, 2, 15], // QEMU default
            fb_width,
            fb_height,
        }
    }

    /// Handle input and return new state if transitioning
    pub fn update(&mut self, action: MenuAction) -> Option<GameState> {
        match self.input_mode {
            InputMode::ModeSelect => self.handle_mode_select(action),
            InputMode::IpEntry => self.handle_ip_entry(action),
        }
    }

    fn handle_mode_select(&mut self, action: MenuAction) -> Option<GameState> {
        match action {
            MenuAction::Up => {
                if self.selected_mode == 0 {
                    self.selected_mode = ServerMode::COUNT - 1;
                } else {
                    self.selected_mode -= 1;
                }
            }
            MenuAction::Down => {
                self.selected_mode = (self.selected_mode + 1) % ServerMode::COUNT;
            }
            MenuAction::Select => {
                let mode = ServerMode::from_index(self.selected_mode);
                match mode {
                    ServerMode::Host => {
                        set_network_mode(NetworkMode::Server { port: self.port });
                        return Some(GameState::PartyLobby);
                    }
                    ServerMode::Join => {
                        // Enter IP entry mode
                        self.input_mode = InputMode::IpEntry;
                        self.ip_cursor = 0;
                    }
                    ServerMode::Offline => {
                        set_network_mode(NetworkMode::Offline);
                        return Some(GameState::PartyLobby);
                    }
                }
            }
            MenuAction::Back => {
                return Some(GameState::PartyLobby);
            }
            _ => {}
        }

        None
    }

    fn handle_ip_entry(&mut self, action: MenuAction) -> Option<GameState> {
        match action {
            MenuAction::Left => {
                if self.ip_cursor > 0 {
                    self.ip_cursor -= 1;
                }
            }
            MenuAction::Right => {
                if self.ip_cursor < 3 {
                    self.ip_cursor += 1;
                }
            }
            MenuAction::Up => {
                // Increment current octet
                let octet = &mut self.ip_octets[self.ip_cursor];
                *octet = octet.wrapping_add(1);
            }
            MenuAction::Down => {
                // Decrement current octet
                let octet = &mut self.ip_octets[self.ip_cursor];
                *octet = octet.wrapping_sub(1);
            }
            MenuAction::Select => {
                // Connect with entered IP
                set_network_mode(NetworkMode::Client {
                    server_ip: self.ip_octets,
                    port: self.port,
                });
                return Some(GameState::PartyLobby);
            }
            MenuAction::Back => {
                // Return to mode selection
                self.input_mode = InputMode::ModeSelect;
            }
            _ => {}
        }

        None
    }

    /// Handle numeric key input for IP entry
    pub fn handle_number_key(&mut self, digit: u8) {
        if self.input_mode != InputMode::IpEntry {
            return;
        }

        let octet = &mut self.ip_octets[self.ip_cursor];
        let new_val = (*octet as u16 * 10 + digit as u16).min(255);
        *octet = new_val as u8;
    }

    /// Clear current IP octet
    pub fn clear_octet(&mut self) {
        if self.input_mode == InputMode::IpEntry {
            self.ip_octets[self.ip_cursor] = 0;
        }
    }

    /// Draw the server selection screen
    pub fn draw(&self, _ctx: &RenderContext, fb_width: usize, fb_height: usize) {
        let fb_guard = FRAMEBUFFER.lock();
        let fb = match fb_guard.as_ref() {
            Some(f) => f,
            None => return,
        };

        // Draw gradient background
        draw_gradient_background_raw(fb, fb_width, fb_height);

        // Draw title
        font::draw_string_centered_raw(fb, 40, "NETWORK MODE", colors::TITLE, 4);

        // Draw subtitle
        font::draw_string_centered_raw(fb, 100, "Choose how to play", colors::SUBTITLE, 2);

        // Draw mode options
        let panel_width = 400;
        let panel_height = 100;
        let panel_spacing = 20;
        let start_y = 160;
        let panel_x = (fb_width - panel_width) / 2;

        for i in 0..ServerMode::COUNT {
            let mode = ServerMode::from_index(i);
            let panel_y = start_y + i * (panel_height + panel_spacing);
            let selected = i == self.selected_mode && self.input_mode == InputMode::ModeSelect;

            let bg_color = if selected {
                colors::BUTTON_SELECTED
            } else {
                colors::PANEL_BG
            };

            draw_panel_raw(fb, panel_x, panel_y, panel_width, panel_height, bg_color);

            // Mode label
            let label_y = panel_y + 20;
            font::draw_string_raw(
                fb,
                panel_x + 20,
                label_y,
                mode.label(),
                if selected { colors::FN_YELLOW } else { colors::WHITE },
                3,
            );

            // Mode description
            let desc_y = panel_y + 60;
            font::draw_string_raw(
                fb,
                panel_x + 20,
                desc_y,
                mode.description(),
                colors::SUBTITLE,
                1,
            );

            // Selection indicator
            if selected {
                font::draw_string_raw(fb, panel_x - 30, label_y, ">", colors::FN_YELLOW, 3);
            }
        }

        // Draw IP entry panel if in IP entry mode
        if self.input_mode == InputMode::IpEntry {
            self.draw_ip_entry(fb, fb_width, fb_height);
        }

        // Draw local IP info when in Host mode
        if self.selected_mode == 0 && self.input_mode == InputMode::ModeSelect {
            let info_y = start_y + ServerMode::COUNT * (panel_height + panel_spacing) + 20;
            let mut ip_buf = [0u8; 32];
            let ip_str = format_ip_display("Your IP: ", &self.local_ip, &mut ip_buf);
            font::draw_string_centered_raw(fb, info_y, ip_str, colors::SUBTITLE, 2);
        }

        // Draw controls footer
        let footer = if self.input_mode == InputMode::IpEntry {
            "[UP/DOWN] Adjust  [LEFT/RIGHT] Move  [ENTER] Connect  [ESC] Back"
        } else {
            "[UP/DOWN] Select  [ENTER] Confirm  [ESC] Back"
        };
        font::draw_string_centered_raw(fb, fb_height - 40, footer, colors::SUBTITLE, 2);
    }

    fn draw_ip_entry(&self, fb: &crate::graphics::framebuffer::Framebuffer, fb_width: usize, _fb_height: usize) {
        // Overlay panel for IP entry
        let panel_width = 500;
        let panel_height = 200;
        let panel_x = (fb_width - panel_width) / 2;
        let panel_y = 250;

        // Dark overlay
        for y in 0..fb.height {
            for x in 0..fb.width {
                let existing = fb.get_pixel(x, y);
                let r = ((existing >> 16) & 0xFF) / 2;
                let g = ((existing >> 8) & 0xFF) / 2;
                let b = (existing & 0xFF) / 2;
                fb.put_pixel(x, y, (r << 16) | (g << 8) | b);
            }
        }

        draw_panel_raw(fb, panel_x, panel_y, panel_width, panel_height, colors::PANEL_BG);

        // Title
        font::draw_string_raw(fb, panel_x + 20, panel_y + 20, "ENTER SERVER IP", colors::TITLE, 3);

        // IP address display with editable octets
        let ip_y = panel_y + 80;
        let octet_width = 60;
        let dot_width = 20;
        let total_ip_width = octet_width * 4 + dot_width * 3;
        let ip_start_x = panel_x + (panel_width - total_ip_width) / 2;

        for i in 0..4 {
            let octet_x = ip_start_x + i * (octet_width + dot_width);
            let is_selected = i == self.ip_cursor;

            // Octet background
            let octet_bg = if is_selected {
                colors::FN_YELLOW
            } else {
                colors::BUTTON_NORMAL
            };
            fill_rect_raw(fb, octet_x, ip_y, octet_width, 40, octet_bg);

            // Octet value
            let mut octet_buf = [0u8; 4];
            let octet_str = font::format_number(self.ip_octets[i] as u32, &mut octet_buf);
            let text_color = if is_selected { colors::BLACK } else { colors::WHITE };
            let text_x = octet_x + (octet_width - font::string_width(octet_str, 3)) / 2;
            font::draw_string_raw(fb, text_x, ip_y + 8, octet_str, text_color, 3);

            // Dot separator
            if i < 3 {
                let dot_x = octet_x + octet_width + 5;
                font::draw_string_raw(fb, dot_x, ip_y + 8, ".", colors::WHITE, 3);
            }
        }

        // Port display
        let port_y = panel_y + 140;
        let mut port_buf = [0u8; 16];
        let port_str = format_port_display(&mut port_buf, self.port);
        font::draw_string_raw(fb, panel_x + 20, port_y, port_str, colors::SUBTITLE, 2);
    }
}

/// Format IP address display
fn format_ip_display<'a>(prefix: &str, ip: &[u8; 4], buf: &'a mut [u8; 32]) -> &'a str {
    let mut pos = 0;

    for &b in prefix.as_bytes() {
        buf[pos] = b;
        pos += 1;
    }

    for i in 0..4 {
        pos = write_number_to_buf(buf, pos, ip[i] as usize);
        if i < 3 {
            buf[pos] = b'.';
            pos += 1;
        }
    }

    unsafe { core::str::from_utf8_unchecked(&buf[..pos]) }
}

/// Format port display
fn format_port_display<'a>(buf: &'a mut [u8; 16], port: u16) -> &'a str {
    let prefix = b"Port: ";
    let mut pos = 0;

    for &b in prefix {
        buf[pos] = b;
        pos += 1;
    }

    pos = write_number_to_buf(buf, pos, port as usize);

    unsafe { core::str::from_utf8_unchecked(&buf[..pos]) }
}

/// Write a number to buffer
fn write_number_to_buf(buf: &mut [u8], start: usize, value: usize) -> usize {
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
