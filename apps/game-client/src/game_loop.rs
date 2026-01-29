//! Game Loop
//!
//! Main game loop for the client application.
//! This runs as part of the kernel's main loop.

use crate::{ClientConfig, ClientState};
use game_types::GameState;

/// Game client instance
pub struct GameClient {
    config: ClientConfig,
    state: ClientState,
    running: bool,
}

impl GameClient {
    /// Create a new game client
    pub fn new(config: ClientConfig) -> Self {
        Self {
            config,
            state: ClientState::new(),
            running: false,
        }
    }

    /// Get current configuration
    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    /// Get current state
    pub fn state(&self) -> &ClientState {
        &self.state
    }

    /// Get mutable state
    pub fn state_mut(&mut self) -> &mut ClientState {
        &mut self.state
    }

    /// Check if client is running
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Start the client
    pub fn start(&mut self) {
        self.running = true;
        self.state = ClientState::new();
    }

    /// Stop the client
    pub fn stop(&mut self) {
        self.running = false;
    }

    /// Update the game state
    pub fn update(&mut self, dt: f32) {
        if !self.running {
            return;
        }
        self.state.update(dt);
    }

    /// Get current game state
    pub fn game_state(&self) -> GameState {
        self.state.game_state
    }

    /// Check if in menu
    pub fn is_in_menu(&self) -> bool {
        self.state.game_state.is_menu()
    }

    /// Check if in gameplay
    pub fn is_in_gameplay(&self) -> bool {
        self.state.game_state.is_gameplay()
    }

    /// Get frame count
    pub fn frame_count(&self) -> u64 {
        self.state.frame_count
    }

    /// Get screen dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }
}
