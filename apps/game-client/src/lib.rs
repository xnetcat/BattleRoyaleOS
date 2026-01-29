//! Game Client Application
//!
//! The main game client that runs on a BattleRoyaleOS instance.
//! This crate provides the game loop, state machine, and rendering logic
//! for the full game experience.

#![no_std]

pub mod game_loop;
pub mod state_machine;

pub use game_loop::GameClient;
pub use state_machine::ClientState;

use game_types::{GameState, PlayerCustomization, Settings};

/// Configuration for the game client
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Screen width
    pub width: u32,
    /// Screen height
    pub height: u32,
    /// Target frames per second
    pub target_fps: u32,
    /// Player customization
    pub customization: PlayerCustomization,
    /// Game settings
    pub settings: Settings,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            width: 1024,
            height: 768,
            target_fps: 30,
            customization: PlayerCustomization::default(),
            settings: Settings::default(),
        }
    }
}

/// Client initialization result
pub struct ClientInit {
    pub config: ClientConfig,
    pub initial_state: GameState,
}

impl ClientInit {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            config: ClientConfig {
                width,
                height,
                ..Default::default()
            },
            initial_state: GameState::PartyLobby,
        }
    }
}
