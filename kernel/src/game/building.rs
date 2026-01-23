//! Building system

use glam::Vec3;

/// Building piece types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BuildType {
    Wall,
    Floor,
    Ramp,
    Roof,
}

/// A placed building piece
#[derive(Debug, Clone)]
pub struct BuildPiece {
    pub build_type: BuildType,
    pub position: Vec3,
    pub rotation: f32, // Yaw in radians
    pub health: u16,
}

impl BuildPiece {
    /// Create a wall piece
    pub fn wall(position: Vec3, rotation: f32) -> Self {
        Self {
            build_type: BuildType::Wall,
            position,
            rotation,
            health: 150,
        }
    }

    /// Create a floor piece
    pub fn floor(position: Vec3, rotation: f32) -> Self {
        Self {
            build_type: BuildType::Floor,
            position,
            rotation,
            health: 140,
        }
    }

    /// Create a ramp piece
    pub fn ramp(position: Vec3, rotation: f32) -> Self {
        Self {
            build_type: BuildType::Ramp,
            position,
            rotation,
            health: 140,
        }
    }

    /// Take damage
    pub fn damage(&mut self, amount: u16) -> bool {
        if self.health > amount {
            self.health -= amount;
            false
        } else {
            self.health = 0;
            true // Destroyed
        }
    }

    /// Check if piece is destroyed
    pub fn is_destroyed(&self) -> bool {
        self.health == 0
    }

    /// Get the dimensions of this piece
    pub fn dimensions(&self) -> Vec3 {
        match self.build_type {
            BuildType::Wall => Vec3::new(4.0, 4.0, 0.2),
            BuildType::Floor => Vec3::new(4.0, 0.2, 4.0),
            BuildType::Ramp => Vec3::new(4.0, 4.0, 4.0),
            BuildType::Roof => Vec3::new(4.0, 0.2, 4.0),
        }
    }

    /// Get material cost for this piece type
    pub fn material_cost(&self) -> u32 {
        match self.build_type {
            BuildType::Wall => 10,
            BuildType::Floor => 10,
            BuildType::Ramp => 10,
            BuildType::Roof => 10,
        }
    }
}

/// Snap position to build grid
pub fn snap_to_grid(position: Vec3) -> Vec3 {
    let grid_size = 4.0;
    Vec3::new(
        libm::roundf(position.x / grid_size) * grid_size,
        libm::roundf(position.y / grid_size) * grid_size,
        libm::roundf(position.z / grid_size) * grid_size,
    )
}

/// Get valid build positions around a player
pub fn get_build_positions(player_pos: Vec3, player_yaw: f32) -> [Vec3; 4] {
    let forward = Vec3::new(libm::sinf(player_yaw), 0.0, libm::cosf(player_yaw));
    let right = Vec3::new(libm::cosf(player_yaw), 0.0, -libm::sinf(player_yaw));

    let distance = 4.0;

    [
        snap_to_grid(player_pos + forward * distance), // Front
        snap_to_grid(player_pos - forward * distance), // Back
        snap_to_grid(player_pos + right * distance),   // Right
        snap_to_grid(player_pos - right * distance),   // Left
    ]
}
