//! Player entity

use alloc::string::String;
use glam::Vec3;
use protocol::packets::{ClientInput, PlayerState, PlayerStateFlags};
use smoltcp::wire::Ipv4Address;

/// Maximum number of players
pub const MAX_PLAYERS: usize = 100;

/// Movement speed (units per second)
pub const MOVE_SPEED: f32 = 10.0;

/// Jump velocity
pub const JUMP_VELOCITY: f32 = 15.0;

/// Gravity
pub const GRAVITY: f32 = 30.0;

/// Freefall speed
pub const FREEFALL_SPEED: f32 = 50.0;

/// Parachute speed
pub const PARACHUTE_SPEED: f32 = 10.0;

/// Parachute deploy height
pub const PARACHUTE_HEIGHT: f32 = 100.0;

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

    // State
    pub health: u8,
    pub materials: u32,
    pub weapon_id: u8,
    pub flags: u8,

    // Bus/drop state
    pub in_bus: bool,
    pub parachute: bool,

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
            health: 100,
            materials: 500,
            weapon_id: 0,
            flags: PlayerStateFlags::ALIVE | PlayerStateFlags::IN_BUS,
            in_bus: true,
            parachute: false,
            last_input_seq: 0,
        }
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

        // Handle bus exit
        if input.exit_bus && self.in_bus {
            self.exit_bus();
            return;
        }

        // Skip movement if in bus
        if self.in_bus {
            return;
        }

        // Handle building
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

        // Apply movement
        if !self.parachute && self.is_grounded() {
            self.velocity.x = move_dir.x * MOVE_SPEED;
            self.velocity.z = move_dir.z * MOVE_SPEED;

            // Jump
            if input.jump {
                self.velocity.y = JUMP_VELOCITY;
                self.flags |= PlayerStateFlags::JUMPING;
            }
        } else if self.parachute {
            // Limited air control during parachute
            self.velocity.x = move_dir.x * MOVE_SPEED * 0.5;
            self.velocity.z = move_dir.z * MOVE_SPEED * 0.5;
        }

        // Crouch
        if input.crouch {
            self.flags |= PlayerStateFlags::CROUCHING;
        } else {
            self.flags &= !PlayerStateFlags::CROUCHING;
        }
    }

    /// Update physics
    pub fn update(&mut self, dt: f32) {
        if self.in_bus {
            return;
        }

        // Apply gravity
        if !self.is_grounded() {
            if self.parachute {
                self.velocity.y = -PARACHUTE_SPEED;
            } else if self.position.y > PARACHUTE_HEIGHT {
                self.velocity.y = -FREEFALL_SPEED;
            } else {
                // Auto-deploy parachute
                if !self.parachute && self.position.y > 0.0 {
                    self.parachute = true;
                    self.flags |= PlayerStateFlags::PARACHUTE;
                }
            }

            // Regular gravity when close to ground
            if self.position.y <= PARACHUTE_HEIGHT && !self.parachute {
                self.velocity.y -= GRAVITY * dt;
            }
        }

        // Update position
        self.position += self.velocity * dt;

        // Ground collision
        if self.position.y <= 0.0 {
            self.position.y = 0.0;
            self.velocity.y = 0.0;
            self.parachute = false;
            self.flags &= !PlayerStateFlags::PARACHUTE;
            self.flags &= !PlayerStateFlags::JUMPING;
        }
    }

    /// Check if player is on the ground
    pub fn is_grounded(&self) -> bool {
        self.position.y <= 0.1
    }

    /// Exit the battle bus
    pub fn exit_bus(&mut self) {
        self.in_bus = false;
        self.flags &= !PlayerStateFlags::IN_BUS;
        // Start freefall
        self.velocity.y = -FREEFALL_SPEED;
    }

    /// Take damage
    pub fn take_damage(&mut self, amount: u8) {
        if self.health > amount {
            self.health -= amount;
        } else {
            self.health = 0;
            self.flags &= !PlayerStateFlags::ALIVE;
        }
    }

    /// Check if alive
    pub fn is_alive(&self) -> bool {
        self.flags & PlayerStateFlags::ALIVE != 0
    }

    /// Get forward direction
    pub fn forward(&self) -> Vec3 {
        Vec3::new(libm::sinf(self.yaw), 0.0, libm::cosf(self.yaw))
    }

    /// Convert to network state
    pub fn to_state(&self) -> PlayerState {
        let mut state = PlayerState::new(self.id);
        state.set_position(self.position.x, self.position.y, self.position.z);
        state.yaw = (self.yaw.to_degrees() * 100.0) as i16;
        state.pitch = (self.pitch.to_degrees() * 100.0) as i16;
        state.health = self.health;
        state.weapon_id = self.weapon_id;
        state.state = self.flags;
        state
    }
}
