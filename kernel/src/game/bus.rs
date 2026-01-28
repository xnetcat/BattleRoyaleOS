//! Battle Bus entity

use glam::Vec3;

/// Battle bus starting height (lowered for faster landing)
pub const BUS_HEIGHT: f32 = 150.0;

/// Battle bus speed
pub const BUS_SPEED: f32 = 100.0;

/// Map size
pub const MAP_SIZE: f32 = 2000.0;

/// Battle bus state
#[derive(Debug, Clone)]
pub struct BattleBus {
    pub position: Vec3,
    pub direction: Vec3,
    pub active: bool,
    pub progress: f32, // 0.0 to 1.0 across the map
}

impl BattleBus {
    pub fn new() -> Self {
        // Start at one edge of the map, moving across
        let start_x = -MAP_SIZE / 2.0;
        let start_z = 0.0;

        Self {
            position: Vec3::new(start_x, BUS_HEIGHT, start_z),
            direction: Vec3::new(1.0, 0.0, 0.0), // Moving along X axis
            active: true,
            progress: 0.0,
        }
    }

    /// Update bus position
    pub fn update(&mut self, dt: f32) {
        if !self.active {
            return;
        }

        // Move bus
        self.position += self.direction * BUS_SPEED * dt;

        // Update progress
        self.progress = (self.position.x + MAP_SIZE / 2.0) / MAP_SIZE;

        // Deactivate when bus has crossed the map
        if self.progress >= 1.0 {
            self.active = false;
        }
    }

    /// Get drop position for a player exiting the bus
    pub fn get_drop_position(&self) -> Vec3 {
        self.position
    }

    /// Check if bus is still active
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Get bus progress across map (0.0 to 1.0)
    pub fn get_progress(&self) -> f32 {
        self.progress
    }

    /// Randomize bus path for a new game
    pub fn randomize_path(&mut self, seed: u32) {
        // Simple deterministic "random" based on seed
        let angle = (seed as f32 * 0.1) % core::f32::consts::TAU;

        // Start position on edge of map
        let start_x = (MAP_SIZE / 2.0) * libm::cosf(angle);
        let start_z = (MAP_SIZE / 2.0) * libm::sinf(angle);

        // Direction towards opposite side
        self.position = Vec3::new(-start_x, BUS_HEIGHT, -start_z);
        self.direction = Vec3::new(start_x, 0.0, start_z).normalize();
        self.progress = 0.0;
        self.active = true;
    }
}
