//! Game Types
//!
//! Pure data types and game logic that can be shared between kernel and applications.
//! No OS dependencies - uses only `no_std` compatible crates.

#![no_std]

pub mod inventory;
pub mod phase;
pub mod state;
pub mod weapon;
pub mod world;

pub use inventory::{AmmoReserves, Inventory, Materials, INVENTORY_SLOTS};
pub use phase::PlayerPhase;
pub use state::{CustomizationCategory, GameState, MenuAction, NetworkMode, PlayerCustomization, Settings};
pub use weapon::{AmmoType, Rarity, Weapon, WeaponType};
pub use world::{BattleBus, LootDrop, Storm, StormPhase};

/// Maximum number of players in a match
pub const MAX_PLAYERS: usize = 100;

/// Movement constants
pub mod movement {
    /// Movement speed (units per second)
    pub const MOVE_SPEED: f32 = 10.0;
    /// Sprint speed multiplier
    pub const SPRINT_MULTIPLIER: f32 = 1.5;
    /// Crouch speed multiplier
    pub const CROUCH_MULTIPLIER: f32 = 0.5;
    /// Jump velocity
    pub const JUMP_VELOCITY: f32 = 15.0;
    /// Gravity
    pub const GRAVITY: f32 = 30.0;

    /// Freefall speeds
    pub const FREEFALL_SPEED_NORMAL: f32 = 70.0;
    pub const FREEFALL_SPEED_DIVE: f32 = 120.0;
    pub const FREEFALL_SPEED_SLOW: f32 = 40.0;
    pub const FREEFALL_HORIZONTAL: f32 = 30.0;

    /// Glider speeds
    pub const GLIDER_VERTICAL_SPEED: f32 = 25.0;
    pub const GLIDER_DIVE_SPEED: f32 = 45.0;
    pub const GLIDER_HORIZONTAL_SPEED: f32 = 20.0;
    pub const GLIDER_BOOST_SPEED: f32 = 35.0;

    /// Glider deploy heights
    pub const AUTO_DEPLOY_HEIGHT: f32 = 50.0;
    pub const MANUAL_DEPLOY_MIN_HEIGHT: f32 = 100.0;
}
