//! Game state machine
//!
//! Manages the overall game state transitions from menu to gameplay to victory.

use spin::Mutex;

/// Main game state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameState {
    /// Main menu - play, settings, quit buttons
    MainMenu,
    /// Settings screen - graphics, audio, controls
    Settings,
    /// Player customization screen
    Customization,
    /// Lobby - waiting for players, ready up
    Lobby,
    /// Countdown before bus phase
    Countdown { remaining_secs: u8 },
    /// Bus flying across the map
    BusPhase,
    /// Active gameplay
    InGame,
    /// Victory/defeat screen
    Victory { winner_id: Option<u8> },
}

impl Default for GameState {
    fn default() -> Self {
        Self::MainMenu
    }
}

/// Player's current phase within the game
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlayerPhase {
    /// On the battle bus, hasn't dropped yet
    #[default]
    OnBus,
    /// Freefalling after exiting bus
    Freefall,
    /// Glider/parachute deployed
    Gliding,
    /// On the ground, normal gameplay
    Grounded,
    /// Dead, eliminated from the match
    Eliminated,
    /// Spectating another player
    Spectating,
}

/// Menu selection action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    /// No action
    None,
    /// Move selection up
    Up,
    /// Move selection down
    Down,
    /// Move selection left (for settings sliders)
    Left,
    /// Move selection right (for settings sliders)
    Right,
    /// Confirm/select current option
    Select,
    /// Go back / cancel
    Back,
}

/// Main menu options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainMenuOption {
    Play,
    Settings,
    Customization,
    Quit,
}

impl MainMenuOption {
    pub const COUNT: usize = 4;

    pub fn from_index(index: usize) -> Self {
        match index % Self::COUNT {
            0 => Self::Play,
            1 => Self::Settings,
            2 => Self::Customization,
            _ => Self::Quit,
        }
    }

    pub fn to_index(self) -> usize {
        match self {
            Self::Play => 0,
            Self::Settings => 1,
            Self::Customization => 2,
            Self::Quit => 3,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Play => "PLAY",
            Self::Settings => "SETTINGS",
            Self::Customization => "CUSTOMIZE",
            Self::Quit => "QUIT",
        }
    }
}

/// Settings options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsOption {
    ShowFps,
    InvertY,
    Sensitivity,
    RenderDistance,
    Volume,
    Back,
}

impl SettingsOption {
    pub const COUNT: usize = 6;

    pub fn from_index(index: usize) -> Self {
        match index % Self::COUNT {
            0 => Self::ShowFps,
            1 => Self::InvertY,
            2 => Self::Sensitivity,
            3 => Self::RenderDistance,
            4 => Self::Volume,
            _ => Self::Back,
        }
    }

    pub fn to_index(self) -> usize {
        match self {
            Self::ShowFps => 0,
            Self::InvertY => 1,
            Self::Sensitivity => 2,
            Self::RenderDistance => 3,
            Self::Volume => 4,
            Self::Back => 5,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::ShowFps => "SHOW FPS",
            Self::InvertY => "INVERT Y",
            Self::Sensitivity => "SENSITIVITY",
            Self::RenderDistance => "RENDER DIST",
            Self::Volume => "VOLUME",
            Self::Back => "BACK",
        }
    }

    pub fn is_toggle(self) -> bool {
        matches!(self, Self::ShowFps | Self::InvertY)
    }

    pub fn is_range(self) -> bool {
        matches!(self, Self::Sensitivity | Self::RenderDistance | Self::Volume)
    }
}

/// Game settings
#[derive(Debug, Clone, Copy)]
pub struct Settings {
    pub show_fps: bool,
    pub invert_y: bool,
    pub sensitivity: u8,      // 1-10
    pub render_distance: u8,  // 1-3
    pub volume: u8,           // 0-100
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

impl Settings {
    /// Get value for a settings option
    pub fn get_value(&self, option: SettingsOption) -> i32 {
        match option {
            SettingsOption::ShowFps => self.show_fps as i32,
            SettingsOption::InvertY => self.invert_y as i32,
            SettingsOption::Sensitivity => self.sensitivity as i32,
            SettingsOption::RenderDistance => self.render_distance as i32,
            SettingsOption::Volume => self.volume as i32,
            SettingsOption::Back => 0,
        }
    }

    /// Get display string for a settings option value
    pub fn get_value_str(&self, option: SettingsOption) -> &'static str {
        match option {
            SettingsOption::ShowFps => if self.show_fps { "ON" } else { "OFF" },
            SettingsOption::InvertY => if self.invert_y { "ON" } else { "OFF" },
            _ => "", // Numeric values handled differently
        }
    }

    /// Adjust a toggle setting
    pub fn toggle(&mut self, option: SettingsOption) {
        match option {
            SettingsOption::ShowFps => self.show_fps = !self.show_fps,
            SettingsOption::InvertY => self.invert_y = !self.invert_y,
            _ => {}
        }
    }

    /// Adjust a range setting
    pub fn adjust(&mut self, option: SettingsOption, delta: i8) {
        match option {
            SettingsOption::Sensitivity => {
                let new_val = (self.sensitivity as i16 + delta as i16).clamp(1, 10);
                self.sensitivity = new_val as u8;
            }
            SettingsOption::RenderDistance => {
                let new_val = (self.render_distance as i16 + delta as i16).clamp(1, 3);
                self.render_distance = new_val as u8;
            }
            SettingsOption::Volume => {
                let new_val = (self.volume as i16 + delta as i16 * 10).clamp(0, 100);
                self.volume = new_val as u8;
            }
            _ => {}
        }
    }
}

/// Character customization options (voxel-based)
#[derive(Debug, Clone, Copy)]
pub struct PlayerCustomization {
    pub skin_tone: u8,       // 0-2 (light, medium, dark)
    pub hair_style: u8,      // 0-3
    pub hair_color: u8,      // 0-3 (black, brown, blonde, red)
    pub shirt_color: u8,     // 0-3
    pub pants_color: u8,     // 0-2
    pub shoes_color: u8,     // 0-1
    pub backpack_style: u8,  // 0-3 (none, small, medium, large)
    pub glider_style: u8,    // 0-3
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

    /// Convert to renderer's CharacterCustomization
    pub fn to_renderer(&self) -> renderer::voxel::CharacterCustomization {
        renderer::voxel::CharacterCustomization {
            skin_tone: self.skin_tone,
            hair_style: self.hair_style,
            hair_color: self.hair_color,
            shirt_color: self.shirt_color,
            pants_color: self.pants_color,
            shoes_color: self.shoes_color,
            backpack_style: self.backpack_style,
            glider_style: self.glider_style,
        }
    }
}

/// Lobby player info
#[derive(Debug, Clone)]
pub struct LobbyPlayer {
    pub id: u8,
    pub name: [u8; 16],
    pub ready: bool,
    pub customization: PlayerCustomization,
}

impl LobbyPlayer {
    pub fn new(id: u8, name: &str) -> Self {
        let mut name_buf = [0u8; 16];
        let bytes = name.as_bytes();
        let len = bytes.len().min(16);
        name_buf[..len].copy_from_slice(&bytes[..len]);

        Self {
            id,
            name: name_buf,
            ready: false,
            customization: PlayerCustomization::default(),
        }
    }

    pub fn name_str(&self) -> &str {
        let end = self.name.iter().position(|&b| b == 0).unwrap_or(16);
        core::str::from_utf8(&self.name[..end]).unwrap_or("???")
    }
}

/// Global game state
pub static GAME_STATE: Mutex<GameState> = Mutex::new(GameState::MainMenu);

/// Global settings
pub static SETTINGS: Mutex<Settings> = Mutex::new(Settings {
    show_fps: true,
    invert_y: false,
    sensitivity: 5,
    render_distance: 3,
    volume: 80,
});

/// Local player customization
pub static PLAYER_CUSTOMIZATION: Mutex<PlayerCustomization> = Mutex::new(PlayerCustomization {
    skin_tone: 0,
    hair_style: 0,
    hair_color: 0,
    shirt_color: 0,
    pants_color: 0,
    shoes_color: 0,
    backpack_style: 1,
    glider_style: 0,
});

/// Transition to a new game state
pub fn set_state(new_state: GameState) {
    *GAME_STATE.lock() = new_state;
}

/// Get current game state
pub fn get_state() -> GameState {
    *GAME_STATE.lock()
}

/// Check if we're in a menu state
pub fn is_menu_state() -> bool {
    matches!(
        get_state(),
        GameState::MainMenu | GameState::Settings | GameState::Customization | GameState::Lobby
    )
}

/// Check if we're in active gameplay
pub fn is_gameplay_state() -> bool {
    matches!(
        get_state(),
        GameState::BusPhase | GameState::InGame
    )
}
