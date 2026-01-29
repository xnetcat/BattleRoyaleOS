//! Game State Types
//!
//! Defines game state, settings, and customization options.

/// Main game state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameState {
    /// Party Lobby - social hub, party up to 4, customize, queue
    PartyLobby,
    /// Settings screen - graphics, audio, controls
    Settings,
    /// Player customization screen
    Customization,
    /// Matchmaking queue - searching for players
    Matchmaking { elapsed_secs: u16 },
    /// Lobby Island - warmup area, respawn on death
    LobbyIsland,
    /// Final countdown before bus (10 seconds)
    LobbyCountdown { remaining_secs: u8 },
    /// Bus flying across the map
    BusPhase,
    /// Active gameplay
    InGame,
    /// Victory/defeat screen
    Victory { winner_id: Option<u8> },
    /// Test map - model gallery viewer
    TestMap,
    /// Server selection screen
    ServerSelect,
}

impl Default for GameState {
    fn default() -> Self {
        Self::PartyLobby
    }
}

impl GameState {
    /// Check if we're in a menu state
    pub fn is_menu(&self) -> bool {
        matches!(
            self,
            GameState::PartyLobby
                | GameState::Settings
                | GameState::Customization
                | GameState::Matchmaking { .. }
                | GameState::TestMap
                | GameState::ServerSelect
        )
    }

    /// Check if we're in active gameplay
    pub fn is_gameplay(&self) -> bool {
        matches!(self, GameState::BusPhase | GameState::InGame)
    }

    /// Check if we're in lobby island (warmup)
    pub fn is_warmup(&self) -> bool {
        matches!(self, GameState::LobbyIsland | GameState::LobbyCountdown { .. })
    }
}

/// Network connection mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkMode {
    /// Offline single-player mode
    Offline,
    /// Server mode - host a game
    Server { port: u16 },
    /// Client mode - connect to a server
    Client { server_ip: [u8; 4], port: u16 },
}

impl Default for NetworkMode {
    fn default() -> Self {
        Self::Offline
    }
}

/// Menu navigation action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MenuAction {
    #[default]
    None,
    Up,
    Down,
    Left,
    Right,
    Select,
    Back,
}

impl MenuAction {
    pub fn is_directional(&self) -> bool {
        matches!(self, Self::Up | Self::Down | Self::Left | Self::Right)
    }
}

/// Game settings
#[derive(Debug, Clone, Copy)]
pub struct Settings {
    pub show_fps: bool,
    pub invert_y: bool,
    pub sensitivity: u8,
    pub render_distance: u8,
    pub volume: u8,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            show_fps: true,
            invert_y: false,
            sensitivity: 5,
            render_distance: 3,
            volume: 80,
        }
    }
}

/// Character customization options (voxel-based)
#[derive(Debug, Clone, Copy)]
pub struct PlayerCustomization {
    pub skin_tone: u8,
    pub hair_style: u8,
    pub hair_color: u8,
    pub shirt_color: u8,
    pub pants_color: u8,
    pub shoes_color: u8,
    pub backpack_style: u8,
    pub glider_style: u8,
}

impl Default for PlayerCustomization {
    fn default() -> Self {
        Self {
            skin_tone: 0,
            hair_style: 0,
            hair_color: 0,
            shirt_color: 0,
            pants_color: 0,
            shoes_color: 0,
            backpack_style: 1,
            glider_style: 0,
        }
    }
}

/// Customization category for UI navigation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CustomizationCategory {
    SkinTone,
    HairStyle,
    HairColor,
    ShirtColor,
    PantsColor,
    ShoesColor,
    Backpack,
    Glider,
}

impl CustomizationCategory {
    pub const COUNT: usize = 8;

    pub fn from_index(index: usize) -> Self {
        match index % Self::COUNT {
            0 => Self::SkinTone,
            1 => Self::HairStyle,
            2 => Self::HairColor,
            3 => Self::ShirtColor,
            4 => Self::PantsColor,
            5 => Self::ShoesColor,
            6 => Self::Backpack,
            _ => Self::Glider,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::SkinTone => "SKIN TONE",
            Self::HairStyle => "HAIR STYLE",
            Self::HairColor => "HAIR COLOR",
            Self::ShirtColor => "SHIRT",
            Self::PantsColor => "PANTS",
            Self::ShoesColor => "SHOES",
            Self::Backpack => "BACKPACK",
            Self::Glider => "GLIDER",
        }
    }

    pub fn max_value(self) -> u8 {
        match self {
            Self::SkinTone => 2,
            Self::HairStyle => 3,
            Self::HairColor => 3,
            Self::ShirtColor => 3,
            Self::PantsColor => 2,
            Self::ShoesColor => 1,
            Self::Backpack => 3,
            Self::Glider => 3,
        }
    }
}

impl PlayerCustomization {
    /// Get value for a category
    pub fn get(&self, category: CustomizationCategory) -> u8 {
        match category {
            CustomizationCategory::SkinTone => self.skin_tone,
            CustomizationCategory::HairStyle => self.hair_style,
            CustomizationCategory::HairColor => self.hair_color,
            CustomizationCategory::ShirtColor => self.shirt_color,
            CustomizationCategory::PantsColor => self.pants_color,
            CustomizationCategory::ShoesColor => self.shoes_color,
            CustomizationCategory::Backpack => self.backpack_style,
            CustomizationCategory::Glider => self.glider_style,
        }
    }

    /// Set value for a category
    pub fn set(&mut self, category: CustomizationCategory, value: u8) {
        let max = category.max_value();
        let clamped = value.min(max);
        match category {
            CustomizationCategory::SkinTone => self.skin_tone = clamped,
            CustomizationCategory::HairStyle => self.hair_style = clamped,
            CustomizationCategory::HairColor => self.hair_color = clamped,
            CustomizationCategory::ShirtColor => self.shirt_color = clamped,
            CustomizationCategory::PantsColor => self.pants_color = clamped,
            CustomizationCategory::ShoesColor => self.shoes_color = clamped,
            CustomizationCategory::Backpack => self.backpack_style = clamped,
            CustomizationCategory::Glider => self.glider_style = clamped,
        }
    }

    /// Increment value for a category (wrapping)
    pub fn next(&mut self, category: CustomizationCategory) {
        let current = self.get(category);
        let max = category.max_value();
        let new_value = if current >= max { 0 } else { current + 1 };
        self.set(category, new_value);
    }

    /// Decrement value for a category (wrapping)
    pub fn prev(&mut self, category: CustomizationCategory) {
        let current = self.get(category);
        let max = category.max_value();
        let new_value = if current == 0 { max } else { current - 1 };
        self.set(category, new_value);
    }
}
