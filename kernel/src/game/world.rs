//! Game world state

use super::bot::{BotController, BotInput, create_bot_player};
use super::building::BuildPiece;
use super::bus::BattleBus;
use super::combat::{self, CombatManager, HitResult};
use super::loot::{LootManager, LootItem, ChestTier};
use super::map::{GameMap, VegetationType};
use super::player::{Player, MAX_PLAYERS};
use super::state::PlayerPhase;
use super::storm::Storm;
use super::weapon::{AmmoType, WeaponType};
use alloc::vec::Vec;
use glam::Vec3;
use protocol::packets::{ClientInput, PlayerState, WorldStateDelta};
use smoltcp::wire::Ipv4Address;
use spin::Mutex;
use alloc::string::String;
use alloc::format;

/// Kill feed entry
#[derive(Clone)]
pub struct KillFeedEntry {
    pub message: String,
    pub timer: f32,
}

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

    // Local player ID (for client)
    pub local_player_id: Option<u8>,

    // Kill feed
    pub kill_feed: Vec<KillFeedEntry>,

    // Combat manager for hit markers, damage numbers
    pub combat: CombatManager,

    // Loot manager
    pub loot: LootManager,

    // Whether world loot has been spawned
    loot_spawned: bool,

    // Bot AI controllers (indexed by player ID)
    bot_controllers: Vec<Option<BotController>>,

    // Whether bots have been spawned
    bots_spawned: bool,
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
            local_player_id: None,
            kill_feed: Vec::new(),
            combat: CombatManager::new(),
            loot: LootManager::new(12345),
            loot_spawned: false,
            bot_controllers: Vec::new(),
            bots_spawned: false,
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
        // First apply movement and orientation
        if let Some(player) = self.players.get_mut(player_id as usize) {
            player.apply_input(input, 1.0 / 20.0); // 20 Hz server tick
            self.changed_players.push(player_id);
        }

        // Handle fire input separately (needs immutable borrow of players for hitscan)
        if input.fire {
            self.process_fire(player_id);
        }

        // Check for building
        if let Some(player) = self.players.get(player_id as usize) {
            if input.build && player.inventory.materials.wood >= 10 {
                self.try_build(player_id);
            }
        }
    }

    /// Process fire input and perform hitscan
    fn process_fire(&mut self, player_id: u8) {
        // Get shooter info
        let (origin, direction, weapon_clone, can_fire, is_pickaxe) = {
            let player = match self.players.get(player_id as usize) {
                Some(p) => p,
                None => return,
            };

            // Only grounded players can fire
            if player.phase != PlayerPhase::Grounded {
                return;
            }

            let weapon = player.inventory.selected_weapon();
            let can_fire = weapon.can_fire();
            let weapon_clone = weapon.clone();
            let is_pickaxe = weapon.weapon_type == WeaponType::Pickaxe;

            (player.eye_position(), player.look_direction(), weapon_clone, can_fire, is_pickaxe)
        };

        if !can_fire {
            return;
        }

        // Fire the weapon (consume ammo, set cooldown)
        if let Some(player) = self.players.get_mut(player_id as usize) {
            let weapon = player.inventory.selected_weapon_mut();
            if !weapon.fire() {
                return;
            }
        }

        // Handle pickaxe harvesting separately
        if is_pickaxe {
            self.process_harvest(player_id, origin, direction);
            return;
        }

        // Perform hitscan
        let hit_result = combat::hitscan(origin, direction, &weapon_clone, player_id, &self.players);

        // Process hit result
        match hit_result {
            HitResult::PlayerHit { player_id: victim_id, damage, headshot, distance: _ } => {
                // Apply damage to victim
                if let Some(victim) = self.players.get_mut(victim_id as usize) {
                    victim.take_damage(damage, Some(player_id));

                    // Add hit marker
                    self.combat.add_hit_marker(headshot);

                    // Add damage number at victim position
                    let victim_pos = victim.position + Vec3::new(0.0, 1.5, 0.0);
                    self.combat.add_damage_number(victim_pos, damage, headshot);

                    // Check for elimination
                    if victim.health == 0 {
                        // Record elimination for killer
                        if let Some(killer) = self.players.get_mut(player_id as usize) {
                            killer.record_elimination();
                        }

                        // Get names for kill feed
                        let killer_name = self.players.get(player_id as usize)
                            .map(|p| p.name.clone())
                            .unwrap_or_else(|| String::from("???"));
                        let victim_name = self.players.get(victim_id as usize)
                            .map(|p| p.name.clone())
                            .unwrap_or_else(|| String::from("???"));

                        // Add to world kill feed
                        self.kill_feed.push(KillFeedEntry {
                            message: format!("{} eliminated {}", killer_name, victim_name),
                            timer: 5.0,
                        });

                        // Add to combat manager kill feed
                        self.combat.add_kill(player_id, victim_id, weapon_clone.weapon_type, headshot);
                    }
                }

                // Record damage dealt by shooter
                if let Some(shooter) = self.players.get_mut(player_id as usize) {
                    shooter.record_damage(damage);
                }
            }
            HitResult::WorldHit { position: _, distance: _ } => {
                // Hit world geometry - could add bullet hole effect later
            }
            HitResult::Miss => {
                // Missed everything
            }
        }
    }

    /// Process pickaxe harvesting
    fn process_harvest(&mut self, player_id: u8, origin: Vec3, direction: Vec3) {
        let harvest_range = 3.0; // Pickaxe range
        let player_pos = match self.players.get(player_id as usize) {
            Some(p) => p.position,
            None => return,
        };

        // Check for harvestable vegetation (trees, rocks)
        let mut best_hit: Option<(usize, f32, VegetationType)> = None;

        for i in 0..self.map.vegetation_count {
            if let Some(veg) = &self.map.vegetation[i] {
                let to_veg = veg.position - player_pos;
                let dist = to_veg.length();

                if dist > harvest_range + 2.0 {
                    continue; // Too far
                }

                // Simple raycast check - does ray pass near vegetation?
                let t = direction.dot(to_veg);
                if t < 0.0 || t > harvest_range {
                    continue; // Behind player or too far
                }

                let closest_point = origin + direction * t;
                let dist_to_veg = (closest_point - veg.position).length();

                // Vegetation hitbox size depends on type
                let hitbox_radius = match veg.veg_type {
                    VegetationType::TreePine | VegetationType::TreeOak | VegetationType::TreeBirch => 1.5,
                    VegetationType::Bush => 1.0,
                    VegetationType::Rock => 1.2,
                };

                if dist_to_veg < hitbox_radius {
                    // Hit! Check if closest
                    match &best_hit {
                        Some((_, best_dist, _)) if *best_dist <= t => {}
                        _ => best_hit = Some((i, t, veg.veg_type)),
                    }
                }
            }
        }

        // Apply harvest reward
        if let Some((veg_idx, _, veg_type)) = best_hit {
            // Give materials based on vegetation type
            let (wood, brick, metal) = match veg_type {
                VegetationType::TreePine | VegetationType::TreeOak | VegetationType::TreeBirch => (15, 0, 0),
                VegetationType::Bush => (5, 0, 0),
                VegetationType::Rock => (0, 10, 5),
            };

            if let Some(player) = self.players.get_mut(player_id as usize) {
                player.inventory.materials.add_wood(wood);
                player.inventory.materials.add_brick(brick);
                player.inventory.materials.add_metal(metal);
            }

            // Add visual feedback (damage number showing materials gained)
            if let Some(veg) = &self.map.vegetation[veg_idx] {
                let hit_pos = veg.position + Vec3::new(0.0, 1.5, 0.0);
                self.combat.add_damage_number(hit_pos, (wood + brick + metal) as u8, false);
            }

            // Remove vegetation after enough hits (simple: remove immediately for now)
            // In a full implementation, vegetation would have health
            self.map.vegetation[veg_idx] = None;
            self.map.vegetation_count = self.map.vegetation_count.saturating_sub(1);
        }

        // Also check for hitting player buildings (for material recovery)
        let mut building_hit_idx: Option<usize> = None;
        for (i, building) in self.buildings.iter().enumerate() {
            if building.is_destroyed() {
                continue;
            }

            let to_build = building.position - player_pos;
            let dist = to_build.length();

            if dist > harvest_range + 3.0 {
                continue;
            }

            let t = direction.dot(to_build);
            if t < 0.0 || t > harvest_range {
                continue;
            }

            let closest_point = origin + direction * t;
            let dist_to_build = (closest_point - building.position).length();

            if dist_to_build < 2.5 { // Building hitbox
                building_hit_idx = Some(i);
                break;
            }
        }

        if let Some(idx) = building_hit_idx {
            // Damage building and give materials
            let building = &mut self.buildings[idx];
            building.damage(50); // 50 pickaxe damage

            // Give back some materials
            if let Some(player) = self.players.get_mut(player_id as usize) {
                player.inventory.materials.add_wood(5); // Small refund
            }

            // Visual feedback
            let hit_pos = building.position + Vec3::new(0.0, 1.0, 0.0);
            self.combat.add_damage_number(hit_pos, 50, false);
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

        // Update players with terrain height
        for player in &mut self.players {
            let terrain_height = self.map.get_height_at(player.position.x, player.position.z);
            player.update(dt, &self.buildings, terrain_height);

            // Storm damage (no attacker)
            if player.is_alive() && !self.storm.contains(player.position) {
                player.take_damage(self.storm.damage_per_tick(), None);
            }
        }

        // Update kill feed timers
        self.kill_feed.retain_mut(|entry| {
            entry.timer -= dt;
            entry.timer > 0.0
        });

        // Update combat effects (hit markers, damage numbers)
        self.combat.update(dt);

        // Update loot drops
        self.loot.update(dt);

        // Spawn world loot when bus finishes (or immediately for single player)
        if !self.loot_spawned && (!self.bus.active || self.players.iter().all(|p| p.phase != PlayerPhase::OnBus)) {
            self.spawn_world_loot();
        }

        // Update storm
        self.storm.update(dt);

        // Update bot AI and apply their inputs
        self.update_bots(dt);

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

    /// Spawn all world loot from map spawn points
    pub fn spawn_world_loot(&mut self) {
        if self.loot_spawned {
            return;
        }
        self.loot_spawned = true;

        // Spawn loot at each map spawn point
        for i in 0..self.map.loot_spawn_count {
            if let Some(spawn) = &mut self.map.loot_spawns[i] {
                if spawn.spawned {
                    continue;
                }
                spawn.spawned = true;

                match spawn.spawn_type {
                    super::loot::LootSpawnType::Chest(tier) => {
                        self.loot.spawn_chest_loot(spawn.position, tier);
                    }
                    super::loot::LootSpawnType::Floor => {
                        self.loot.spawn_floor_loot(spawn.position);
                    }
                    super::loot::LootSpawnType::AmmoBox => {
                        // Ammo boxes spawn random ammo
                        let ammo_type = match (self.tick as usize + i) % 4 {
                            0 => AmmoType::Light,
                            1 => AmmoType::Medium,
                            2 => AmmoType::Heavy,
                            _ => AmmoType::Shells,
                        };
                        self.loot.spawn_drop(
                            spawn.position,
                            LootItem::Ammo { ammo_type, amount: 30 },
                            false,
                        );
                    }
                }
            }
        }
    }

    /// Try to pick up loot for a player
    pub fn try_pickup(&mut self, player_id: u8) -> bool {
        let player_pos = match self.players.get(player_id as usize) {
            Some(p) => p.position,
            None => return false,
        };

        // Find nearest loot
        let pickup = self.loot.get_nearest_pickup(player_pos);
        let pickup_id = match pickup {
            Some(drop) => drop.id,
            None => return false,
        };

        // Pick up the item
        let item = match self.loot.pickup(pickup_id) {
            Some(item) => item,
            None => return false,
        };

        // Add to player inventory
        if let Some(player) = self.players.get_mut(player_id as usize) {
            match item {
                LootItem::Weapon(weapon) => {
                    // If inventory full, drop current weapon
                    if let Some(dropped) = player.inventory.add_weapon(weapon) {
                        self.loot.spawn_drop(player.position, LootItem::Weapon(dropped), true);
                    }
                }
                LootItem::Ammo { ammo_type, amount } => {
                    player.inventory.ammo.add(ammo_type, amount);
                }
                LootItem::Materials { wood, brick, metal } => {
                    player.inventory.materials.add_wood(wood);
                    player.inventory.materials.add_brick(brick);
                    player.inventory.materials.add_metal(metal);
                }
                LootItem::Health { amount, max_health, .. } => {
                    player.heal(amount, max_health);
                }
                LootItem::Shield { amount, .. } => {
                    player.add_shield(amount);
                }
            }
            return true;
        }

        false
    }

    /// Check for victory condition (last player standing)
    pub fn check_victory(&self) -> Option<u8> {
        let alive: Vec<u8> = self.players.iter()
            .filter(|p| p.is_alive())
            .map(|p| p.id)
            .collect();

        if alive.len() == 1 {
            Some(alive[0])
        } else {
            None
        }
    }

    /// Get winner's name
    pub fn get_winner_name(&self, winner_id: u8) -> String {
        self.players.get(winner_id as usize)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| String::from("Unknown"))
    }

    /// Spawn bots for single-player mode
    pub fn spawn_bots(&mut self, count: usize) {
        if self.bots_spawned {
            return;
        }
        self.bots_spawned = true;

        let start_id = self.players.len() as u8;

        for i in 0..count {
            let id = start_id + i as u8;
            if id >= MAX_PLAYERS as u8 {
                break;
            }

            let seed = 12345u32.wrapping_add(i as u32 * 7919);
            let mut bot = create_bot_player(id, seed);

            // Start bot at a random position on the map
            let angle = (i as f32 / count as f32) * core::f32::consts::TAU;
            let dist = 200.0 + (i as f32 % 5.0) * 100.0;
            let bot_x = libm::cosf(angle) * dist;
            let bot_z = libm::sinf(angle) * dist;
            let terrain_height = self.map.get_height_at(bot_x, bot_z);
            bot.position = Vec3::new(bot_x, terrain_height, bot_z);
            bot.phase = PlayerPhase::Grounded;

            self.players.push(bot);

            // Ensure bot_controllers vec is large enough
            while self.bot_controllers.len() <= id as usize {
                self.bot_controllers.push(None);
            }
            self.bot_controllers[id as usize] = Some(BotController::new(seed));
        }
    }

    /// Update all bot AI
    fn update_bots(&mut self, dt: f32) {
        // Collect bot inputs first (to avoid borrow issues)
        let mut bot_inputs: Vec<(u8, BotInput)> = Vec::new();

        for i in 0..self.bot_controllers.len() {
            if let Some(controller) = &mut self.bot_controllers[i] {
                if let Some(bot) = self.players.get(i) {
                    if bot.is_alive() && bot.phase == PlayerPhase::Grounded {
                        let input = controller.update(
                            bot,
                            &self.players,
                            self.storm.center,
                            self.storm.radius,
                            dt,
                        );
                        bot_inputs.push((i as u8, input));
                    }
                }
            }
        }

        // Apply bot inputs
        for (bot_id, input) in bot_inputs {
            self.apply_bot_input(bot_id, input, dt);
        }
    }

    /// Apply bot AI input to a bot player
    fn apply_bot_input(&mut self, bot_id: u8, input: BotInput, dt: f32) {
        // Apply movement
        if let Some(bot) = self.players.get_mut(bot_id as usize) {
            // Update orientation to face target
            let yaw_diff = input.target_yaw - bot.yaw;
            let turn_speed = 5.0 * dt;
            if yaw_diff.abs() < turn_speed {
                bot.yaw = input.target_yaw;
            } else if yaw_diff > 0.0 {
                bot.yaw += turn_speed;
            } else {
                bot.yaw -= turn_speed;
            }

            // Normalize yaw
            while bot.yaw > core::f32::consts::PI {
                bot.yaw -= core::f32::consts::TAU;
            }
            while bot.yaw < -core::f32::consts::PI {
                bot.yaw += core::f32::consts::TAU;
            }

            // Apply movement
            let forward = Vec3::new(libm::sinf(bot.yaw), 0.0, libm::cosf(bot.yaw));
            let speed = 8.0; // Slightly slower than player
            bot.velocity.x = forward.x * input.forward as f32 * speed;
            bot.velocity.z = forward.z * input.forward as f32 * speed;
        }

        // Handle firing
        if input.fire {
            self.process_fire(bot_id);
        }
    }
}

/// Global game world
pub static GAME_WORLD: Mutex<Option<GameWorld>> = Mutex::new(None);

/// Initialize the game world
pub fn init(is_server: bool) {
    *GAME_WORLD.lock() = Some(GameWorld::new(is_server));
}
