//! World Types
//!
//! Battle bus, storm, loot drops, and other world state types.

use glam::Vec3;

/// Battle bus state
#[derive(Debug, Clone)]
pub struct BattleBus {
    pub position: Vec3,
    pub direction: Vec3,
    pub speed: f32,
    pub active: bool,
    pub departure_time: f32,
}

impl Default for BattleBus {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            direction: Vec3::X,
            speed: 50.0,
            active: false,
            departure_time: 0.0,
        }
    }
}

impl BattleBus {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start the bus flying across the map
    pub fn start(&mut self, start: Vec3, end: Vec3, speed: f32) {
        self.position = start;
        self.direction = (end - start).normalize();
        self.speed = speed;
        self.active = true;
        self.departure_time = 0.0;
    }

    /// Update bus position
    pub fn update(&mut self, dt: f32) {
        if self.active {
            self.position += self.direction * self.speed * dt;
            self.departure_time += dt;
        }
    }

    /// Stop the bus
    pub fn stop(&mut self) {
        self.active = false;
    }
}

/// Storm phase
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StormPhase {
    /// Waiting before first shrink
    Waiting,
    /// Storm is shrinking
    Shrinking,
    /// Storm is paused between phases
    Paused,
    /// Final circle, match ending
    Final,
}

impl Default for StormPhase {
    fn default() -> Self {
        Self::Waiting
    }
}

/// Storm state
#[derive(Debug, Clone)]
pub struct Storm {
    pub center: Vec3,
    pub radius: f32,
    pub target_center: Vec3,
    pub target_radius: f32,
    pub phase: StormPhase,
    pub phase_number: u8,
    pub timer: f32,
    pub damage_per_second: f32,
}

impl Default for Storm {
    fn default() -> Self {
        Self {
            center: Vec3::new(0.0, 0.0, 0.0),
            radius: 500.0,
            target_center: Vec3::new(0.0, 0.0, 0.0),
            target_radius: 500.0,
            phase: StormPhase::Waiting,
            phase_number: 0,
            timer: 120.0, // 2 minutes before first shrink
            damage_per_second: 1.0,
        }
    }
}

impl Storm {
    pub fn new(center: Vec3, radius: f32) -> Self {
        Self {
            center,
            radius,
            target_center: center,
            target_radius: radius,
            ..Default::default()
        }
    }

    /// Check if a position is inside the safe zone
    pub fn is_safe(&self, position: Vec3) -> bool {
        let dx = position.x - self.center.x;
        let dz = position.z - self.center.z;
        let dist_sq = dx * dx + dz * dz;
        dist_sq <= self.radius * self.radius
    }

    /// Get damage at a position (0 if safe, damage_per_second if in storm)
    pub fn damage_at(&self, position: Vec3) -> f32 {
        if self.is_safe(position) {
            0.0
        } else {
            self.damage_per_second
        }
    }

    /// Update storm state
    pub fn update(&mut self, dt: f32) {
        self.timer -= dt;

        match self.phase {
            StormPhase::Waiting => {
                if self.timer <= 0.0 {
                    self.start_shrink();
                }
            }
            StormPhase::Shrinking => {
                // Move towards target
                let shrink_speed: f32 = 10.0; // Units per second
                let center_dir = self.target_center - self.center;
                let center_dist = center_dir.length();
                if center_dist > 0.1 {
                    self.center += center_dir.normalize() * shrink_speed.min(center_dist) * dt;
                }

                let radius_diff = self.radius - self.target_radius;
                if radius_diff > 0.1 {
                    self.radius -= shrink_speed.min(radius_diff) * dt;
                }

                // Check if we've reached target
                if (self.center - self.target_center).length() < 1.0
                    && (self.radius - self.target_radius).abs() < 1.0
                {
                    self.center = self.target_center;
                    self.radius = self.target_radius;
                    self.phase = StormPhase::Paused;
                    self.timer = 60.0; // 1 minute pause
                }
            }
            StormPhase::Paused => {
                if self.timer <= 0.0 {
                    self.start_shrink();
                }
            }
            StormPhase::Final => {
                // Final circle - continuous damage
            }
        }
    }

    /// Start the next shrink phase
    fn start_shrink(&mut self) {
        self.phase_number += 1;

        if self.phase_number >= 7 {
            self.phase = StormPhase::Final;
            self.target_radius = 0.0;
            self.damage_per_second = 10.0;
            return;
        }

        self.phase = StormPhase::Shrinking;

        // Calculate new target (shrink towards center with random offset)
        let shrink_factor = 0.5; // Each phase shrinks to 50% radius
        self.target_radius = self.radius * shrink_factor;

        // Random offset within current circle
        let offset_x = libm::sinf(self.phase_number as f32 * 1.7) * self.radius * 0.2;
        let offset_z = libm::cosf(self.phase_number as f32 * 2.3) * self.radius * 0.2;
        self.target_center = Vec3::new(
            self.center.x + offset_x,
            0.0,
            self.center.z + offset_z,
        );

        // Increase damage each phase
        self.damage_per_second = (self.phase_number as f32).min(5.0);
    }
}

/// Loot drop item type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LootType {
    Weapon,
    Ammo,
    Health,
    Shield,
    Materials,
}

/// Loot drop
#[derive(Debug, Clone)]
pub struct LootDrop {
    pub position: Vec3,
    pub loot_type: LootType,
    pub weapon_type: Option<u8>,
    pub rarity: u8,
    pub amount: u16,
    pub collected: bool,
}

impl LootDrop {
    pub fn weapon(position: Vec3, weapon_type: u8, rarity: u8) -> Self {
        Self {
            position,
            loot_type: LootType::Weapon,
            weapon_type: Some(weapon_type),
            rarity,
            amount: 1,
            collected: false,
        }
    }

    pub fn ammo(position: Vec3, amount: u16) -> Self {
        Self {
            position,
            loot_type: LootType::Ammo,
            weapon_type: None,
            rarity: 0,
            amount,
            collected: false,
        }
    }

    pub fn health(position: Vec3, amount: u16) -> Self {
        Self {
            position,
            loot_type: LootType::Health,
            weapon_type: None,
            rarity: 0,
            amount,
            collected: false,
        }
    }

    pub fn shield(position: Vec3, amount: u16) -> Self {
        Self {
            position,
            loot_type: LootType::Shield,
            weapon_type: None,
            rarity: 0,
            amount,
            collected: false,
        }
    }

    pub fn collect(&mut self) {
        self.collected = true;
    }
}
