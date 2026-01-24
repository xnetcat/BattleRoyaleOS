//! Player entity

use alloc::string::String;
use glam::Vec3;
use protocol::packets::{ClientInput, PlayerState, PlayerStateFlags};
use smoltcp::wire::Ipv4Address;
use super::state::{PlayerPhase, PlayerCustomization};
use super::inventory::Inventory;

/// Maximum number of players
pub const MAX_PLAYERS: usize = 100;

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
pub const FREEFALL_SPEED_NORMAL: f32 = 50.0;
pub const FREEFALL_SPEED_DIVE: f32 = 80.0;    // Holding forward
pub const FREEFALL_SPEED_SLOW: f32 = 30.0;    // Holding back
pub const FREEFALL_HORIZONTAL: f32 = 20.0;    // Max horizontal speed in freefall

/// Glider speeds
pub const GLIDER_VERTICAL_SPEED: f32 = 10.0;
pub const GLIDER_HORIZONTAL_SPEED: f32 = 15.0;
pub const GLIDER_BOOST_SPEED: f32 = 25.0;     // Holding forward

/// Glider deploy heights
pub const AUTO_DEPLOY_HEIGHT: f32 = 100.0;
pub const MANUAL_DEPLOY_MIN_HEIGHT: f32 = 200.0;

/// Player entity
#[derive(Debug, Clone)]
pub struct Player {
    pub id: u8,
    pub name: String,
    pub address: Ipv4Address,
    pub port: u16,
    pub connected: bool,

    // Position and orientation
    pub position: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,   // Radians
    pub pitch: f32, // Radians

    // Player phase (bus, freefall, gliding, grounded, etc.)
    pub phase: PlayerPhase,

    // Health and shield
    pub health: u8,
    pub shield: u8,
    pub max_health: u8,
    pub max_shield: u8,

    // Inventory
    pub inventory: Inventory,

    // Legacy state flags for network protocol
    pub flags: u8,

    // Drop state
    pub drop_position: Vec3,
    pub dive_angle: f32,

    // Spectate
    pub spectate_target: Option<u8>,
    pub eliminator_id: Option<u8>,

    // Stats
    pub eliminations: u16,
    pub damage_dealt: u32,

    // Customization
    pub customization: PlayerCustomization,

    // Last input sequence (for lag compensation)
    pub last_input_seq: u32,
}

impl Player {
    pub fn new(id: u8, name: &str, address: Ipv4Address, port: u16) -> Self {
        Self {
            id,
            name: String::from(name),
            address,
            port,
            connected: true,
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            phase: PlayerPhase::OnBus,
            health: 100,
            shield: 0,
            max_health: 100,
            max_shield: 100,
            inventory: Inventory::new(),
            flags: PlayerStateFlags::ALIVE | PlayerStateFlags::IN_BUS,
            drop_position: Vec3::ZERO,
            dive_angle: 0.0,
            spectate_target: None,
            eliminator_id: None,
            eliminations: 0,
            damage_dealt: 0,
            customization: PlayerCustomization::default(),
            last_input_seq: 0,
        }
    }

    /// Create a player with customization
    pub fn with_customization(id: u8, name: &str, address: Ipv4Address, port: u16, customization: PlayerCustomization) -> Self {
        let mut player = Self::new(id, name, address, port);
        player.customization = customization;
        player
    }

    /// Apply client input
    pub fn apply_input(&mut self, input: &ClientInput, dt: f32) {
        if input.sequence <= self.last_input_seq {
            return; // Old input, ignore
        }
        self.last_input_seq = input.sequence;

        // Update orientation
        self.yaw = (input.yaw as f32 / 100.0).to_radians();
        self.pitch = (input.pitch as f32 / 100.0).to_radians();

        match self.phase {
            PlayerPhase::OnBus => {
                // Handle bus exit
                if input.exit_bus {
                    self.exit_bus();
                }
            }
            PlayerPhase::Freefall => {
                self.apply_freefall_input(input, dt);
            }
            PlayerPhase::Gliding => {
                self.apply_gliding_input(input, dt);
            }
            PlayerPhase::Grounded => {
                self.apply_ground_input(input, dt);
            }
            PlayerPhase::Eliminated | PlayerPhase::Spectating => {
                // No movement input when dead/spectating
            }
        }
    }

    /// Apply input during freefall
    fn apply_freefall_input(&mut self, input: &ClientInput, _dt: f32) {
        // Calculate movement direction for steering
        let forward = Vec3::new(libm::sinf(self.yaw), 0.0, libm::cosf(self.yaw));
        let right = Vec3::new(libm::cosf(self.yaw), 0.0, -libm::sinf(self.yaw));

        // Dive faster when holding forward, slower when holding back
        if input.forward > 0 {
            self.dive_angle = (self.dive_angle + 0.05).min(1.0);
        } else if input.forward < 0 {
            self.dive_angle = (self.dive_angle - 0.05).max(-0.5);
        } else {
            // Return to neutral
            self.dive_angle *= 0.95;
        }

        // Horizontal steering
        let steer = input.strafe as f32;
        self.velocity.x = forward.x * self.dive_angle * FREEFALL_HORIZONTAL + right.x * steer * FREEFALL_HORIZONTAL;
        self.velocity.z = forward.z * self.dive_angle * FREEFALL_HORIZONTAL + right.z * steer * FREEFALL_HORIZONTAL;

        // Manual glider deploy
        if input.jump && self.position.y >= MANUAL_DEPLOY_MIN_HEIGHT {
            self.deploy_glider();
        }
    }

    /// Apply input during gliding
    fn apply_gliding_input(&mut self, input: &ClientInput, _dt: f32) {
        let forward = Vec3::new(libm::sinf(self.yaw), 0.0, libm::cosf(self.yaw));
        let right = Vec3::new(libm::cosf(self.yaw), 0.0, -libm::sinf(self.yaw));

        // Forward boost or normal speed
        let speed = if input.forward > 0 {
            GLIDER_BOOST_SPEED
        } else {
            GLIDER_HORIZONTAL_SPEED
        };

        self.velocity.x = forward.x * speed;
        self.velocity.z = forward.z * speed;

        // Strafe steering
        self.velocity.x += right.x * input.strafe as f32 * 5.0;
        self.velocity.z += right.z * input.strafe as f32 * 5.0;
    }

    /// Apply input when grounded
    fn apply_ground_input(&mut self, input: &ClientInput, _dt: f32) {
        // Handle building mode
        if input.build {
            self.flags |= PlayerStateFlags::BUILDING;
        } else {
            self.flags &= !PlayerStateFlags::BUILDING;
        }

        // Calculate movement direction
        let forward = Vec3::new(libm::sinf(self.yaw), 0.0, libm::cosf(self.yaw));
        let right = Vec3::new(libm::cosf(self.yaw), 0.0, -libm::sinf(self.yaw));

        let mut move_dir = Vec3::ZERO;
        move_dir += forward * input.forward as f32;
        move_dir += right * input.strafe as f32;

        if move_dir.length_squared() > 0.001 {
            move_dir = move_dir.normalize();
        }

        // Apply movement with speed modifiers
        let mut speed = MOVE_SPEED;
        if input.crouch {
            speed *= CROUCH_MULTIPLIER;
            self.flags |= PlayerStateFlags::CROUCHING;
        } else {
            self.flags &= !PlayerStateFlags::CROUCHING;
        }

        if self.is_grounded() {
            self.velocity.x = move_dir.x * speed;
            self.velocity.z = move_dir.z * speed;

            // Jump
            if input.jump {
                self.velocity.y = JUMP_VELOCITY;
                self.flags |= PlayerStateFlags::JUMPING;
            }
        }
    }

    /// Update physics
    pub fn update(&mut self, dt: f32, buildings: &[crate::game::building::BuildPiece]) {
        // Update inventory (weapon timers)
        self.inventory.update(dt);

        match self.phase {
            PlayerPhase::OnBus => {
                // Position controlled by bus, no physics
            }
            PlayerPhase::Freefall => {
                self.update_freefall(dt, buildings);
            }
            PlayerPhase::Gliding => {
                self.update_gliding(dt, buildings);
            }
            PlayerPhase::Grounded => {
                self.update_grounded(dt, buildings);
            }
            PlayerPhase::Eliminated | PlayerPhase::Spectating => {
                // No physics when dead/spectating
            }
        }
    }

    /// Update freefall physics
    fn update_freefall(&mut self, dt: f32, buildings: &[crate::game::building::BuildPiece]) {
        // Calculate fall speed based on dive angle
        let fall_speed = if self.dive_angle > 0.3 {
            FREEFALL_SPEED_DIVE
        } else if self.dive_angle < -0.3 {
            FREEFALL_SPEED_SLOW
        } else {
            FREEFALL_SPEED_NORMAL
        };

        self.velocity.y = -fall_speed;

        // Update position with collision check
        let next_pos = self.position + self.velocity * dt;
        if !self.check_building_collision(next_pos, buildings) {
            self.position = next_pos;
        } else {
            // Simple slide? or just stop? Stop for freefall
            self.velocity = Vec3::ZERO;
        }
        
        // Auto-deploy glider at minimum height
        if self.position.y <= AUTO_DEPLOY_HEIGHT {
            self.deploy_glider();
        }
    }

    /// Update gliding physics
    fn update_gliding(&mut self, dt: f32, buildings: &[crate::game::building::BuildPiece]) {
        // Constant descent rate
        self.velocity.y = -GLIDER_VERTICAL_SPEED;

        // Update position with collision check
        let next_pos = self.position + self.velocity * dt;
        if !self.check_building_collision(next_pos, buildings) {
            self.position = next_pos;
        } else {
            // Hit something while gliding? Land/Stop
            self.velocity = Vec3::ZERO;
        }

        // Land when reaching ground
        if self.position.y <= 0.0 {
            self.land();
        }
    }

    /// Update grounded physics
    fn update_grounded(&mut self, dt: f32, buildings: &[crate::game::building::BuildPiece]) {
        // Apply gravity if not grounded
        if !self.is_grounded() {
            self.velocity.y -= GRAVITY * dt;
        }

        // Update position with collision check
        let next_pos = self.position + self.velocity * dt;
        
        // Check X/Z collision separately for sliding
        let mut final_pos = self.position;
        
        // Try moving X
        let try_x = Vec3::new(next_pos.x, self.position.y, self.position.z);
        if !self.check_building_collision(try_x, buildings) {
            final_pos.x = next_pos.x;
        } else {
            self.velocity.x = 0.0;
        }
        
        // Try moving Z
        let try_z = Vec3::new(final_pos.x, self.position.y, next_pos.z);
        if !self.check_building_collision(try_z, buildings) {
            final_pos.z = next_pos.z;
        } else {
            self.velocity.z = 0.0;
        }
        
        // Try moving Y
        let try_y = Vec3::new(final_pos.x, next_pos.y, final_pos.z);
        if !self.check_building_collision(try_y, buildings) {
            final_pos.y = next_pos.y;
        } else {
            // Hit floor or ceiling
            if self.velocity.y < 0.0 {
                // Landed on something
                // Handled implicitly by check or explicit ground check?
                // For now, simple stop
            }
            self.velocity.y = 0.0;
        }
        
        self.position = final_pos;

        // Ground collision
        if self.position.y <= 0.0 {
            self.position.y = 0.0;
            self.velocity.y = 0.0;
            self.flags &= !PlayerStateFlags::JUMPING;
        }
    }

    /// Check if player is on the ground
    pub fn is_grounded(&self) -> bool {
        self.position.y <= 0.1 && self.phase == PlayerPhase::Grounded
    }

    /// Exit the battle bus
    pub fn exit_bus(&mut self) {
        self.drop_position = self.position;
        self.phase = PlayerPhase::Freefall;
        self.flags &= !PlayerStateFlags::IN_BUS;
        self.dive_angle = 0.0;
        self.velocity.y = -FREEFALL_SPEED_NORMAL;
    }

    /// Deploy glider
    fn deploy_glider(&mut self) {
        self.phase = PlayerPhase::Gliding;
        self.flags |= PlayerStateFlags::PARACHUTE;
        self.velocity.y = -GLIDER_VERTICAL_SPEED;
    }

    /// Land on the ground
    fn land(&mut self) {
        self.phase = PlayerPhase::Grounded;
        self.position.y = 0.0;
        self.velocity = Vec3::ZERO;
        self.flags &= !PlayerStateFlags::PARACHUTE;
    }

    /// Take damage (applies to shield first, then health)
    pub fn take_damage(&mut self, amount: u8, attacker_id: Option<u8>) {
        let mut remaining = amount;

        // Shield absorbs damage first
        if self.shield > 0 {
            if self.shield >= remaining {
                self.shield -= remaining;
                remaining = 0;
            } else {
                remaining -= self.shield;
                self.shield = 0;
            }
        }

        // Apply remaining damage to health
        if remaining > 0 {
            if self.health > remaining {
                self.health -= remaining;
            } else {
                self.health = 0;
                self.eliminate(attacker_id);
            }
        }
    }

    /// Eliminate the player
    pub fn eliminate(&mut self, killer_id: Option<u8>) {
        self.health = 0;
        self.phase = PlayerPhase::Eliminated;
        self.eliminator_id = killer_id;
        self.flags &= !PlayerStateFlags::ALIVE;
    }

    /// Start spectating another player
    pub fn start_spectating(&mut self, target_id: u8) {
        self.phase = PlayerPhase::Spectating;
        self.spectate_target = Some(target_id);
    }

    /// Heal (capped at max_health or a lower cap for bandages)
    pub fn heal(&mut self, amount: u8, max_cap: u8) {
        let cap = max_cap.min(self.max_health);
        self.health = (self.health + amount).min(cap);
    }

    /// Add shield
    pub fn add_shield(&mut self, amount: u8) {
        self.shield = (self.shield + amount).min(self.max_shield);
    }

    /// Get effective health (health + shield)
    pub fn effective_health(&self) -> u16 {
        self.health as u16 + self.shield as u16
    }

    /// Check if alive
    pub fn is_alive(&self) -> bool {
        self.flags & PlayerStateFlags::ALIVE != 0
    }

    /// Check if can be damaged
    pub fn can_be_damaged(&self) -> bool {
        matches!(self.phase, PlayerPhase::Grounded | PlayerPhase::Freefall | PlayerPhase::Gliding)
            && self.is_alive()
    }

    /// Get forward direction
    pub fn forward(&self) -> Vec3 {
        Vec3::new(libm::sinf(self.yaw), 0.0, libm::cosf(self.yaw))
    }

    /// Get look direction (including pitch)
    pub fn look_direction(&self) -> Vec3 {
        Vec3::new(
            libm::sinf(self.yaw) * libm::cosf(self.pitch),
            libm::sinf(self.pitch),
            libm::cosf(self.yaw) * libm::cosf(self.pitch),
        )
    }

    /// Get eye position for shooting
    pub fn eye_position(&self) -> Vec3 {
        self.position + Vec3::new(0.0, 1.7, 0.0)
    }

    /// Record an elimination
    pub fn record_elimination(&mut self) {
        self.eliminations += 1;
    }

    /// Record damage dealt
    pub fn record_damage(&mut self, amount: u8) {
        self.damage_dealt += amount as u32;
    }

    /// Convert to network state
    pub fn to_state(&self) -> PlayerState {
        let mut state = PlayerState::new(self.id);
        state.set_position(self.position.x, self.position.y, self.position.z);
        state.yaw = (self.yaw.to_degrees() * 100.0) as i16;
        state.pitch = (self.pitch.to_degrees() * 100.0) as i16;
        state.health = self.health;
        state.weapon_id = self.inventory.selected_weapon().weapon_type as u8;
        state.state = self.flags;
        state
    }

    /// Set weapon from network sync (for remote players)
    /// This sets a weapon in the first slot based on the weapon_id received
    pub fn set_network_weapon(&mut self, weapon_id: u8) {
        use super::weapon::{Weapon, WeaponType, Rarity};

        if weapon_id == 0 {
            // Pickaxe
            self.inventory.pickaxe_selected = true;
        } else if let Some(weapon_type) = WeaponType::from_u8(weapon_id) {
            // Create a weapon of this type in first slot for rendering
            let weapon = Weapon::new(weapon_type, Rarity::Common);
            self.inventory.slots[0] = Some(weapon);
            self.inventory.selected_slot = 0;
            self.inventory.pickaxe_selected = false;
        }
    }

    /// Check collision with buildings
    fn check_building_collision(&self, pos: Vec3, buildings: &[crate::game::building::BuildPiece]) -> bool {
        let player_radius = 0.5; // Approximate radius
        let player_height = 1.8;

        for building in buildings {
            if building.is_destroyed() {
                continue;
            }

            // Simple point-in-box check adjusted for player radius
            // For walls (yaw rotation), transform point to local space
            
            // Translate relative to building center
            let dx = pos.x - building.position.x;
            let dz = pos.z - building.position.z;
            
            // Rotate into local space
            let cos_r = libm::cosf(-building.rotation);
            let sin_r = libm::sinf(-building.rotation);
            let local_x = dx * cos_r - dz * sin_r;
            let local_z = dx * sin_r + dz * cos_r;
            let local_y = pos.y - building.position.y; // Assume flat floor/wall center Y?
            // Actually building.position.y is usually the base or center?
            // BuildPiece::wall position is center-mid? let's assume centered for AABB logic
            
            let dims = building.dimensions();
            let half_w = dims.x * 0.5 + player_radius;
            // let half_h = dims.y * 0.5; // Height - check y separately
            let half_d = dims.z * 0.5 + player_radius;
            
            // Check X/Z collision in local space (assume wall is aligned X/Z locally)
            if local_x.abs() < half_w && local_z.abs() < half_d {
                // Check Y collision
                // Wall dimensions logic in building.rs: Wall = 4x4x0.2
                // If position is centered, Y extends +/- 2.0
                // Player y is feet position.
                // Building y is center.
                let half_h = dims.y * 0.5;
                let building_min_y = building.position.y - half_h;
                let building_max_y = building.position.y + half_h;
                
                if pos.y + player_height > building_min_y && pos.y < building_max_y {
                    return true;
                }
            }
        }
        false
    }
}
