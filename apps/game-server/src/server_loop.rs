//! Server Loop
//!
//! Main loop for the dedicated server.

use crate::{ServerConfig, ServerState};

/// Game server instance
pub struct GameServer {
    config: ServerConfig,
    state: ServerState,
    tick_count: u64,
    running: bool,
    player_count: u8,
    match_time: f32,
}

impl GameServer {
    /// Create a new game server
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config,
            state: ServerState::Lobby,
            tick_count: 0,
            running: false,
            player_count: 0,
            match_time: 0.0,
        }
    }

    /// Get current configuration
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    /// Get current state
    pub fn state(&self) -> ServerState {
        self.state
    }

    /// Check if server is running
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Start the server
    pub fn start(&mut self) {
        self.running = true;
        self.state = ServerState::Lobby;
        self.tick_count = 0;
        self.player_count = 0;
        self.match_time = 0.0;
    }

    /// Stop the server
    pub fn stop(&mut self) {
        self.running = false;
    }

    /// Update the server
    pub fn tick(&mut self, dt: f32) {
        if !self.running {
            return;
        }

        self.tick_count += 1;

        match &mut self.state {
            ServerState::Lobby => {
                // Wait for enough players
                if self.player_count >= 2 {
                    self.state = ServerState::Countdown { remaining: 10 };
                }
            }
            ServerState::Countdown { remaining } => {
                // Count down to match start
                if *remaining > 0 {
                    // Tick down every second
                    if self.tick_count % self.config.tick_rate as u64 == 0 {
                        *remaining -= 1;
                    }
                } else {
                    self.state = ServerState::InProgress;
                    self.match_time = 0.0;
                }
            }
            ServerState::InProgress => {
                self.match_time += dt;

                // Check for match end conditions
                // (actual game logic would go here)

                // Timeout check
                if self.match_time >= self.config.match_timeout as f32 {
                    self.state = ServerState::Ended { winner_id: None };
                }
            }
            ServerState::Ended { .. } => {
                // Match ended, wait for reset
            }
        }
    }

    /// Get tick count
    pub fn tick_count(&self) -> u64 {
        self.tick_count
    }

    /// Get player count
    pub fn player_count(&self) -> u8 {
        self.player_count
    }

    /// Set player count (for testing)
    pub fn set_player_count(&mut self, count: u8) {
        self.player_count = count;
    }

    /// Get match time
    pub fn match_time(&self) -> f32 {
        self.match_time
    }

    /// End the match with a winner
    pub fn end_match(&mut self, winner_id: Option<u8>) {
        self.state = ServerState::Ended { winner_id };
    }

    /// Reset for a new match
    pub fn reset(&mut self) {
        self.state = ServerState::Lobby;
        self.tick_count = 0;
        self.player_count = 0;
        self.match_time = 0.0;
    }
}
