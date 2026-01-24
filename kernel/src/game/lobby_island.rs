//! Lobby Island - Pre-game warmup area
//!
//! A smaller map where all 100 players spawn before the match.
//! Features:
//! - Smaller map (200x200 units)
//! - All weapons available
//! - Respawn on death (3 second timer)
//! - Countdown to game start when enough players

use alloc::vec::Vec;
use glam::Vec3;
use spin::Mutex;
use super::player::Player;

/// Lobby island map size (smaller than main map)
pub const LOBBY_MAP_SIZE: f32 = 200.0;

/// Respawn time in seconds
pub const RESPAWN_TIME: f32 = 3.0;

/// Minimum players to start countdown
pub const MIN_PLAYERS_TO_START: usize = 2;

/// Countdown duration in seconds
pub const COUNTDOWN_DURATION: f32 = 30.0;

/// Event from lobby island update
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LobbyIslandEvent {
    /// No event
    None,
    /// Player respawned
    PlayerRespawned { player_id: u8 },
    /// Countdown started
    CountdownStarted,
    /// Countdown tick (new second)
    CountdownTick { remaining_secs: u8 },
    /// Ready to start game
    StartGame,
}

/// Lobby island state
#[derive(Debug, Clone)]
pub struct LobbyIsland {
    /// Players in the lobby island
    pub players: Vec<Player>,
    /// Respawn timers for each player (indexed by player ID)
    pub respawn_timers: [f32; 100],
    /// Countdown timer (None = waiting for players)
    pub countdown: Option<f32>,
    /// Required players to start
    pub required_players: usize,
    /// Has the game been started
    pub game_started: bool,
}

impl LobbyIsland {
    /// Create a new lobby island
    pub fn new() -> Self {
        Self {
            players: Vec::new(),
            respawn_timers: [0.0; 100],
            countdown: None,
            required_players: MIN_PLAYERS_TO_START,
            game_started: false,
        }
    }

    /// Create a new lobby island with custom player requirement
    pub fn with_required_players(required: usize) -> Self {
        Self {
            required_players: required,
            ..Self::new()
        }
    }

    /// Add a player to the lobby island
    pub fn add_player(&mut self, mut player: Player) -> u8 {
        let id = self.players.len() as u8;
        player.id = id;

        // Spawn at random position on lobby island
        let spawn_pos = self.get_spawn_position(id);
        player.position = spawn_pos;
        player.health = 100;
        player.shield = 100;

        self.players.push(player);
        id
    }

    /// Get spawn position for a player
    fn get_spawn_position(&self, player_id: u8) -> Vec3 {
        // Distribute players around the island
        let angle = (player_id as f32 / 100.0) * core::f32::consts::TAU;
        let radius = LOBBY_MAP_SIZE * 0.3;
        Vec3::new(
            libm::cosf(angle) * radius,
            0.0,
            libm::sinf(angle) * radius,
        )
    }

    /// Update the lobby island
    pub fn update(&mut self, dt: f32) -> LobbyIslandEvent {
        if self.game_started {
            return LobbyIslandEvent::None;
        }

        let mut event = LobbyIslandEvent::None;

        // Update respawn timers
        let mut respawned_id: Option<u8> = None;
        for (id, timer) in self.respawn_timers.iter_mut().enumerate() {
            if *timer > 0.0 {
                *timer -= dt;
                if *timer <= 0.0 {
                    *timer = 0.0;
                    respawned_id = Some(id as u8);
                }
            }
        }

        // Handle respawn separately to avoid borrow conflict
        if let Some(id) = respawned_id {
            let spawn_pos = self.get_spawn_position(id);
            if let Some(player) = self.players.get_mut(id as usize) {
                player.health = 100;
                player.shield = 100;
                player.position = spawn_pos;
                event = LobbyIslandEvent::PlayerRespawned { player_id: id };
            }
        }

        // Check if we should start countdown
        let alive_count = self.players.iter().filter(|p| p.health > 0).count();
        if alive_count >= self.required_players && self.countdown.is_none() {
            self.countdown = Some(COUNTDOWN_DURATION);
            return LobbyIslandEvent::CountdownStarted;
        }

        // Update countdown
        if let Some(ref mut countdown) = self.countdown {
            let prev_secs = libm::ceilf(*countdown) as u8;
            *countdown -= dt;
            let new_secs = libm::ceilf(*countdown) as u8;

            if *countdown <= 0.0 {
                self.game_started = true;
                return LobbyIslandEvent::StartGame;
            }

            if new_secs != prev_secs {
                return LobbyIslandEvent::CountdownTick { remaining_secs: new_secs };
            }
        }

        event
    }

    /// Handle player death (start respawn timer)
    pub fn player_died(&mut self, player_id: u8) {
        if (player_id as usize) < self.respawn_timers.len() {
            self.respawn_timers[player_id as usize] = RESPAWN_TIME;
        }
    }

    /// Get respawn timer for a player
    pub fn get_respawn_timer(&self, player_id: u8) -> f32 {
        self.respawn_timers.get(player_id as usize).copied().unwrap_or(0.0)
    }

    /// Check if player is waiting to respawn
    pub fn is_respawning(&self, player_id: u8) -> bool {
        self.get_respawn_timer(player_id) > 0.0
    }

    /// Get countdown remaining seconds (None if not counting down)
    pub fn get_countdown_secs(&self) -> Option<u8> {
        self.countdown.map(|c| libm::ceilf(c) as u8)
    }

    /// Get player count
    pub fn player_count(&self) -> usize {
        self.players.len()
    }

    /// Get alive player count
    pub fn alive_count(&self) -> usize {
        self.players.iter().filter(|p| p.health > 0).count()
    }

    /// Reset lobby island for a new session
    pub fn reset(&mut self) {
        self.players.clear();
        self.respawn_timers = [0.0; 100];
        self.countdown = None;
        self.game_started = false;
    }

    /// Get player by ID
    pub fn get_player(&self, id: u8) -> Option<&Player> {
        self.players.get(id as usize)
    }

    /// Get mutable player by ID
    pub fn get_player_mut(&mut self, id: u8) -> Option<&mut Player> {
        self.players.get_mut(id as usize)
    }
}

impl Default for LobbyIsland {
    fn default() -> Self {
        Self::new()
    }
}

/// Global lobby island state
pub static LOBBY_ISLAND: Mutex<Option<LobbyIsland>> = Mutex::new(None);

/// Initialize lobby island
pub fn init() {
    *LOBBY_ISLAND.lock() = Some(LobbyIsland::new());
}

/// Initialize lobby island with custom player requirement (for testing)
pub fn init_with_required_players(required: usize) {
    *LOBBY_ISLAND.lock() = Some(LobbyIsland::with_required_players(required));
}

/// Get lobby island state
pub fn get_lobby_island() -> Option<LobbyIsland> {
    LOBBY_ISLAND.lock().clone()
}
