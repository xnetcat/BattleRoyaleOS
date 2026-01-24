//! Bot AI system for single-player battles

use glam::Vec3;
use super::player::Player;
use super::state::PlayerPhase;
use super::weapon::{Weapon, WeaponType, Rarity};

/// Bot AI state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BotState {
    /// Wandering around looking for loot/targets
    Wander,
    /// Chasing a target player
    Chase,
    /// Attacking a visible target
    Attack,
    /// Fleeing from danger (low health, storm)
    Flee,
}

/// Bot AI controller
#[derive(Debug, Clone)]
pub struct BotController {
    /// Current AI state
    pub state: BotState,
    /// Current waypoint target
    pub waypoint: Vec3,
    /// Time until next state check
    pub state_timer: f32,
    /// Target player ID (for chase/attack)
    pub target_id: Option<u8>,
    /// Fire cooldown (reaction time)
    pub fire_timer: f32,
    /// Wander direction change timer
    pub wander_timer: f32,
    /// Random seed for this bot
    seed: u32,
}

impl Default for BotController {
    fn default() -> Self {
        Self::new(0)
    }
}

impl BotController {
    pub fn new(seed: u32) -> Self {
        Self {
            state: BotState::Wander,
            waypoint: Vec3::ZERO,
            state_timer: 0.0,
            target_id: None,
            fire_timer: 0.0,
            wander_timer: 0.0,
            seed,
        }
    }

    /// Update the bot AI and return input decisions
    pub fn update(
        &mut self,
        bot: &Player,
        players: &[Player],
        storm_center: Vec3,
        storm_radius: f32,
        dt: f32,
    ) -> BotInput {
        // Update timers
        self.state_timer -= dt;
        self.fire_timer -= dt;
        self.wander_timer -= dt;

        // State machine
        if self.state_timer <= 0.0 {
            self.evaluate_state(bot, players, storm_center, storm_radius);
            self.state_timer = 0.5; // Re-evaluate every 0.5 seconds
        }

        // Generate input based on state
        match self.state {
            BotState::Wander => self.wander_behavior(bot, storm_center, storm_radius, dt),
            BotState::Chase => self.chase_behavior(bot, players),
            BotState::Attack => self.attack_behavior(bot, players, dt),
            BotState::Flee => self.flee_behavior(bot, storm_center, storm_radius),
        }
    }

    /// Evaluate and potentially change state
    fn evaluate_state(
        &mut self,
        bot: &Player,
        players: &[Player],
        storm_center: Vec3,
        storm_radius: f32,
    ) {
        // Check if we're in the storm
        let dist_to_center = (bot.position - storm_center).length();
        let in_storm = dist_to_center > storm_radius;

        // Check for nearby visible players
        let (nearest_enemy, nearest_dist) = self.find_nearest_enemy(bot, players);

        // State transitions
        if bot.health < 30 || (in_storm && dist_to_center > storm_radius + 50.0) {
            // Flee if low health or deep in storm
            self.state = BotState::Flee;
            self.target_id = None;
        } else if let Some(enemy_id) = nearest_enemy {
            if nearest_dist < 30.0 {
                // Close enough to attack
                self.state = BotState::Attack;
                self.target_id = Some(enemy_id);
            } else if nearest_dist < 100.0 {
                // Chase them
                self.state = BotState::Chase;
                self.target_id = Some(enemy_id);
            } else {
                // Too far, wander
                self.state = BotState::Wander;
                self.target_id = None;
            }
        } else {
            // No enemies nearby
            self.state = BotState::Wander;
            self.target_id = None;
        }
    }

    /// Find nearest visible enemy player
    fn find_nearest_enemy(&self, bot: &Player, players: &[Player]) -> (Option<u8>, f32) {
        let mut nearest: Option<u8> = None;
        let mut nearest_dist = f32::MAX;

        for player in players {
            if player.id == bot.id || !player.is_alive() {
                continue;
            }

            let dist = (player.position - bot.position).length();
            if dist < nearest_dist {
                // Simple visibility check (no obstacles for now)
                nearest_dist = dist;
                nearest = Some(player.id);
            }
        }

        (nearest, nearest_dist)
    }

    /// Wander behavior - move around randomly, pick up loot
    fn wander_behavior(
        &mut self,
        bot: &Player,
        storm_center: Vec3,
        storm_radius: f32,
        dt: f32,
    ) -> BotInput {
        // Change direction periodically
        if self.wander_timer <= 0.0 {
            self.wander_timer = 2.0 + self.next_random_f32() * 3.0;

            // Pick a random direction, but prefer toward safe zone
            let random_angle = self.next_random_f32() * core::f32::consts::TAU;
            let random_dist = 20.0 + self.next_random_f32() * 30.0;

            let mut target = bot.position + Vec3::new(
                libm::cosf(random_angle) * random_dist,
                0.0,
                libm::sinf(random_angle) * random_dist,
            );

            // If outside storm, move toward center
            let dist_to_center = (bot.position - storm_center).length();
            if dist_to_center > storm_radius * 0.8 {
                let to_center = (storm_center - bot.position).normalize();
                target = bot.position + to_center * random_dist;
            }

            self.waypoint = target;
        }

        self.move_toward(bot, self.waypoint)
    }

    /// Chase behavior - pursue target
    fn chase_behavior(&self, bot: &Player, players: &[Player]) -> BotInput {
        let target_pos = self.target_id
            .and_then(|id| players.get(id as usize))
            .map(|p| p.position)
            .unwrap_or(bot.position);

        self.move_toward(bot, target_pos)
    }

    /// Attack behavior - shoot at target
    fn attack_behavior(&mut self, bot: &Player, players: &[Player], dt: f32) -> BotInput {
        let target = self.target_id
            .and_then(|id| players.get(id as usize));

        let target_pos = match target {
            Some(p) if p.is_alive() => p.position,
            _ => {
                // Target gone, go back to wandering
                self.state = BotState::Wander;
                return BotInput::default();
            }
        };

        // Calculate direction to target
        let to_target = target_pos - bot.position;
        let dist = to_target.length();
        let direction = if dist > 0.01 { to_target / dist } else { Vec3::Z };

        // Calculate yaw to face target
        let target_yaw = libm::atan2f(direction.x, direction.z);

        // Move closer if too far, back up if too close
        let forward = if dist > 20.0 {
            1
        } else if dist < 8.0 {
            -1
        } else {
            0
        };

        // Fire if facing target and cooldown ready
        let yaw_diff = (target_yaw - bot.yaw).abs();
        let facing_target = yaw_diff < 0.3 || yaw_diff > core::f32::consts::TAU - 0.3;

        let fire = if facing_target && self.fire_timer <= 0.0 {
            self.fire_timer = 0.2 + self.next_random_f32() * 0.3; // Reaction time
            true
        } else {
            false
        };

        BotInput {
            forward,
            strafe: 0,
            jump: false,
            fire,
            target_yaw,
            target_pitch: 0.0, // Aim at body level
        }
    }

    /// Flee behavior - run toward safe zone
    fn flee_behavior(&self, bot: &Player, storm_center: Vec3, storm_radius: f32) -> BotInput {
        // Move toward safe zone center
        self.move_toward(bot, storm_center)
    }

    /// Generate movement input toward a target position
    fn move_toward(&self, bot: &Player, target: Vec3) -> BotInput {
        let to_target = target - bot.position;
        let dist = to_target.length();

        if dist < 2.0 {
            return BotInput::default();
        }

        let direction = to_target / dist;
        let target_yaw = libm::atan2f(direction.x, direction.z);

        BotInput {
            forward: 1,
            strafe: 0,
            jump: false,
            fire: false,
            target_yaw,
            target_pitch: 0.0,
        }
    }

    /// Get next random number
    fn next_random(&mut self) -> u32 {
        self.seed = self.seed.wrapping_mul(1103515245).wrapping_add(12345);
        self.seed
    }

    /// Get next random float 0-1
    fn next_random_f32(&mut self) -> f32 {
        (self.next_random() & 0x7FFFFFFF) as f32 / 0x7FFFFFFF as f32
    }
}

/// Input generated by bot AI
#[derive(Debug, Clone, Copy, Default)]
pub struct BotInput {
    pub forward: i8,
    pub strafe: i8,
    pub jump: bool,
    pub fire: bool,
    pub target_yaw: f32,
    pub target_pitch: f32,
}

/// Bot names for variety
pub const BOT_NAMES: [&str; 20] = [
    "Rustacean",
    "BareMetal",
    "KernelPanic",
    "NoStd",
    "BitShift",
    "StackSmash",
    "HeapSpray",
    "SegFault",
    "PageFault",
    "NullPtr",
    "DanglingRef",
    "RaceCondition",
    "DeadLock",
    "MemLeak",
    "BufferBot",
    "CacheHit",
    "BranchMiss",
    "IRQHandler",
    "DMAMaster",
    "BusArbiter",
];

/// Get a bot name by index
pub fn get_bot_name(index: usize) -> &'static str {
    BOT_NAMES[index % BOT_NAMES.len()]
}

/// Spawn a bot player
pub fn create_bot_player(id: u8, seed: u32) -> Player {
    use smoltcp::wire::Ipv4Address;

    let name = get_bot_name(id as usize);
    let mut player = Player::new(id, name, Ipv4Address::new(0, 0, 0, 0), 0);

    // Give bot a random weapon to start
    let weapon_type = match seed % 5 {
        0 => WeaponType::Pistol,
        1 => WeaponType::Shotgun,
        2 => WeaponType::AssaultRifle,
        3 => WeaponType::Smg,
        _ => WeaponType::Pistol,
    };

    let rarity = match (seed >> 4) % 5 {
        0 => Rarity::Common,
        1 => Rarity::Uncommon,
        2 => Rarity::Rare,
        3 => Rarity::Epic,
        _ => Rarity::Common,
    };

    player.inventory.add_weapon(Weapon::new(weapon_type, rarity));
    player.inventory.select_slot(0); // Select the weapon

    player
}
