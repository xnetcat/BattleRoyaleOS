//! Game Server Application
//!
//! Headless dedicated server for hosting Battle Royale matches.
//! Runs without graphics, handling game logic and network synchronization.

#![no_std]

pub mod server_loop;

pub use server_loop::GameServer;

use game_types::GameState;

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// UDP port to listen on
    pub port: u16,
    /// Maximum players
    pub max_players: u8,
    /// Tick rate (updates per second)
    pub tick_rate: u32,
    /// Match timeout in seconds
    pub match_timeout: u32,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 5000,
            max_players: 100,
            tick_rate: 30,
            match_timeout: 1800, // 30 minutes
        }
    }
}

/// Server state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerState {
    /// Waiting for players to join
    Lobby,
    /// Countdown to match start
    Countdown { remaining: u8 },
    /// Match in progress
    InProgress,
    /// Match ended
    Ended { winner_id: Option<u8> },
}

impl Default for ServerState {
    fn default() -> Self {
        Self::Lobby
    }
}
