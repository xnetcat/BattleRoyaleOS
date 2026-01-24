//! Game world state

use super::building::BuildPiece;
use super::bus::BattleBus;
use super::map::GameMap;
use super::player::{Player, MAX_PLAYERS};
use super::state::PlayerPhase;
use super::storm::Storm;
use alloc::vec::Vec;
use glam::Vec3;
use protocol::packets::{ClientInput, PlayerState, WorldStateDelta};
use smoltcp::wire::Ipv4Address;
use spin::Mutex;

/// Game world
pub struct GameWorld {
    pub tick: u32,
    pub players: Vec<Player>,
    pub buildings: Vec<BuildPiece>,
    pub bus: BattleBus,
    pub storm: Storm,
    pub map: GameMap,
    pub is_server: bool,

    // Delta tracking for network updates
    changed_players: Vec<u8>,
}

impl GameWorld {
    pub fn new(is_server: bool) -> Self {
        Self {
            tick: 0,
            players: Vec::with_capacity(MAX_PLAYERS),
            buildings: Vec::new(),
            bus: BattleBus::new(),
            storm: Storm::new(),
            map: GameMap::new(12345), // Fixed seed for now
            is_server,
            changed_players: Vec::new(),
        }
    }

    /// Add a new player (server only)
    pub fn add_player(&mut self, name: &str, address: Ipv4Address, port: u16) -> Option<u8> {
        if self.players.len() >= MAX_PLAYERS {
            return None;
        }

        let id = self.players.len() as u8;
        let mut player = Player::new(id, name, address, port);

        // Start on the bus
        player.position = self.bus.position;
        player.phase = PlayerPhase::OnBus;

        self.players.push(player);
        self.changed_players.push(id);

        Some(id)
    }

    /// Apply client input to a player
    pub fn apply_input(&mut self, player_id: u8, input: &ClientInput) {
        if let Some(player) = self.players.get_mut(player_id as usize) {
            player.apply_input(input, 1.0 / 20.0); // 20 Hz server tick

            // Check for building
            if input.build && player.inventory.materials.wood >= 10 {
                self.try_build(player_id);
            }

            self.changed_players.push(player_id);
        }
    }

    /// Try to place a building piece
    fn try_build(&mut self, player_id: u8) {
        let player = &mut self.players[player_id as usize];
        // Check if player has enough wood to build
        if player.inventory.materials.wood < 10 {
            return;
        }

        let forward = player.forward();
        let build_pos = player.position + forward * 4.0;

        let piece = BuildPiece::wall(build_pos, player.yaw);
        self.buildings.push(piece);
        player.inventory.materials.wood -= 10;
    }

    /// Update the world (server tick)
    pub fn update(&mut self, dt: f32) {
        self.tick += 1;

        // Update bus
        if self.bus.active {
            self.bus.update(dt);

            // Move players still on bus
            for player in &mut self.players {
                if player.phase == PlayerPhase::OnBus {
                    player.position = self.bus.position;
                }
            }
        }

        // Update players
        for player in &mut self.players {
            player.update(dt);

            // Storm damage (no attacker)
            if player.is_alive() && !self.storm.contains(player.position) {
                player.take_damage(self.storm.damage_per_tick(), None);
            }
        }

        // Update storm
        self.storm.update(dt);

        // Track all players as changed for simplicity
        // A more optimized version would only track actually changed players
        for player in &self.players {
            if !self.changed_players.contains(&player.id) {
                self.changed_players.push(player.id);
            }
        }
    }

    /// Get world state delta for network transmission
    pub fn get_delta(&self) -> WorldStateDelta {
        let player_states: Vec<PlayerState> = self
            .changed_players
            .iter()
            .filter_map(|&id| self.players.get(id as usize).map(|p| p.to_state()))
            .collect();

        WorldStateDelta {
            tick: self.tick,
            player_count: player_states.len() as u8,
            players: player_states,
            storm_x: (self.storm.center.x * 65536.0) as i32,
            storm_z: (self.storm.center.z * 65536.0) as i32,
            storm_radius: (self.storm.radius * 100.0) as u32,
        }
    }

    /// Clear the changed players list after sending delta
    pub fn clear_delta(&mut self) {
        self.changed_players.clear();
    }

    /// Apply a delta from the server (client only)
    pub fn apply_delta(&mut self, delta: &WorldStateDelta) {
        self.tick = delta.tick;

        // Update storm
        self.storm.center.x = delta.storm_x as f32 / 65536.0;
        self.storm.center.z = delta.storm_z as f32 / 65536.0;
        self.storm.radius = delta.storm_radius as f32 / 100.0;

        // Update players
        for state in &delta.players {
            let id = state.player_id as usize;

            // Ensure player exists
            while self.players.len() <= id {
                self.players.push(Player::new(
                    self.players.len() as u8,
                    "Unknown",
                    Ipv4Address::new(0, 0, 0, 0),
                    0,
                ));
            }

            let player = &mut self.players[id];
            player.position = Vec3::new(state.world_x(), state.world_y(), state.world_z());
            player.yaw = state.yaw_radians();
            player.pitch = state.pitch_radians();
            player.health = state.health;
            player.set_network_weapon(state.weapon_id);
            player.flags = state.state;
        }
    }

    /// Get number of alive players
    pub fn alive_count(&self) -> usize {
        self.players.iter().filter(|p| p.is_alive()).count()
    }

    /// Get player by ID
    pub fn get_player(&self, id: u8) -> Option<&Player> {
        self.players.get(id as usize)
    }

    /// Get player by ID (mutable)
    pub fn get_player_mut(&mut self, id: u8) -> Option<&mut Player> {
        self.players.get_mut(id as usize)
    }
}

/// Global game world
pub static GAME_WORLD: Mutex<Option<GameWorld>> = Mutex::new(None);

/// Initialize the game world
pub fn init(is_server: bool) {
    *GAME_WORLD.lock() = Some(GameWorld::new(is_server));
}
