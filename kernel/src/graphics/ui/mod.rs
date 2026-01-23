//! UI primitives for menu rendering

pub mod button;
pub mod list;
pub mod panel;

pub use button::Button;
pub use list::PlayerList;
pub use panel::{draw_gradient_background, draw_panel, draw_panel_raw};

/// Common UI colors
pub mod colors {
    /// Background gradient top color (dark blue)
    pub const BG_TOP: u32 = 0x001A1A2E;
    /// Background gradient bottom color (purple-ish)
    pub const BG_BOTTOM: u32 = 0x0016213E;

    /// Panel background (semi-transparent dark)
    pub const PANEL_BG: u32 = 0x002A2A4A;
    /// Panel border
    pub const PANEL_BORDER: u32 = 0x005A5A8A;

    /// Button normal
    pub const BUTTON_NORMAL: u32 = 0x003A3A6A;
    /// Button hover/selected
    pub const BUTTON_SELECTED: u32 = 0x006A6ABA;
    /// Button text
    pub const BUTTON_TEXT: u32 = 0x00FFFFFF;

    /// Title color (golden yellow)
    pub const TITLE: u32 = 0x00FFD700;
    /// Subtitle color
    pub const SUBTITLE: u32 = 0x00AAAAAA;

    /// Ready indicator (green)
    pub const READY: u32 = 0x0044FF44;
    /// Not ready indicator (gray)
    pub const NOT_READY: u32 = 0x00888888;

    /// Health bar colors
    pub const HEALTH_HIGH: u32 = 0x0044FF44;
    pub const HEALTH_MED: u32 = 0x00FFFF44;
    pub const HEALTH_LOW: u32 = 0x00FF4444;

    /// Common white
    pub const WHITE: u32 = 0x00FFFFFF;
    /// Common black
    pub const BLACK: u32 = 0x00000000;

    /// Fortnite-style blue
    pub const FN_BLUE: u32 = 0x003D87FF;
    /// Fortnite-style purple
    pub const FN_PURPLE: u32 = 0x009D4EDD;
    /// Fortnite-style yellow
    pub const FN_YELLOW: u32 = 0x00FFFF00;
}
