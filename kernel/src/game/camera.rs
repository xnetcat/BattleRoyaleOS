//! Camera system for different game phases

use glam::Vec3;

/// Camera mode for different game phases
#[derive(Debug, Clone, Copy)]
pub enum CameraMode {
    /// Overhead view following the bus
    BusOverhead {
        height: f32,
        look_ahead: f32,
    },
    /// Behind player during freefall
    Freefall {
        distance: f32,
        height_offset: f32,
    },
    /// Wide view during gliding
    Gliding {
        distance: f32,
        height_offset: f32,
        fov_bonus: f32,
    },
    /// Standard third-person follow
    ThirdPerson {
        distance: f32,
        height_offset: f32,
    },
    /// First-person view
    FirstPerson {
        eye_height: f32,
    },
    /// Spectating another player
    Spectate {
        target_id: u8,
    },
    /// Victory camera (orbiting winner)
    Victory {
        orbit_angle: f32,
        orbit_radius: f32,
    },
}

impl Default for CameraMode {
    fn default() -> Self {
        Self::ThirdPerson {
            distance: 5.0,
            height_offset: 2.0,
        }
    }
}

/// Game camera
#[derive(Debug, Clone)]
pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    pub mode: CameraMode,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 5.0, -10.0),
            target: Vec3::ZERO,
            mode: CameraMode::default(),
            fov: core::f32::consts::PI / 3.0, // 60 degrees
            near: 0.1,
            far: 1000.0,
        }
    }
}

impl Camera {
    /// Update camera based on player position, yaw, and current mode
    pub fn update(&mut self, player_pos: Vec3, player_yaw: f32, player_pitch: f32, dt: f32) {
        match self.mode {
            CameraMode::BusOverhead { height, look_ahead } => {
                self.position = Vec3::new(player_pos.x, player_pos.y + height, player_pos.z - look_ahead);
                self.target = Vec3::new(player_pos.x + look_ahead * 2.0, player_pos.y, player_pos.z + look_ahead);
            }

            CameraMode::Freefall { distance, height_offset } => {
                let offset = Vec3::new(
                    -libm::sinf(player_yaw) * distance,
                    height_offset,
                    -libm::cosf(player_yaw) * distance,
                );
                self.position = player_pos + offset;
                self.target = player_pos + Vec3::new(0.0, 1.0, 0.0);
            }

            CameraMode::Gliding { distance, height_offset, fov_bonus: _ } => {
                let offset = Vec3::new(
                    -libm::sinf(player_yaw) * distance,
                    height_offset,
                    -libm::cosf(player_yaw) * distance,
                );
                self.position = player_pos + offset;
                self.target = player_pos + Vec3::new(0.0, 0.5, 0.0);
            }

            CameraMode::ThirdPerson { distance, height_offset } => {
                let offset = Vec3::new(
                    -libm::sinf(player_yaw) * distance,
                    height_offset,
                    -libm::cosf(player_yaw) * distance,
                );
                self.position = player_pos + offset;
                self.target = player_pos + Vec3::new(0.0, 1.5, 0.0);
            }

            CameraMode::FirstPerson { eye_height } => {
                self.position = player_pos + Vec3::new(0.0, eye_height, 0.0);
                // Look direction based on yaw and pitch
                let look_dir = Vec3::new(
                    libm::sinf(player_yaw) * libm::cosf(player_pitch),
                    libm::sinf(player_pitch),
                    libm::cosf(player_yaw) * libm::cosf(player_pitch),
                );
                self.target = self.position + look_dir;
            }

            CameraMode::Spectate { target_id: _ } => {
                // Would need to look up target player's position
                // For now, just orbit around a point
            }

            CameraMode::Victory { ref mut orbit_angle, orbit_radius } => {
                *orbit_angle += dt * 0.5;
                if *orbit_angle > core::f32::consts::TAU {
                    *orbit_angle -= core::f32::consts::TAU;
                }
                self.position = Vec3::new(
                    player_pos.x + libm::sinf(*orbit_angle) * orbit_radius,
                    player_pos.y + 3.0,
                    player_pos.z + libm::cosf(*orbit_angle) * orbit_radius,
                );
                self.target = player_pos + Vec3::new(0.0, 1.0, 0.0);
            }
        }
    }

    /// Set camera mode for bus phase
    pub fn set_bus_mode(&mut self) {
        self.mode = CameraMode::BusOverhead {
            height: 50.0,
            look_ahead: 30.0,
        };
    }

    /// Set camera mode for freefall
    pub fn set_freefall_mode(&mut self) {
        self.mode = CameraMode::Freefall {
            distance: 8.0,
            height_offset: 2.0,
        };
    }

    /// Set camera mode for gliding
    pub fn set_gliding_mode(&mut self) {
        self.mode = CameraMode::Gliding {
            distance: 10.0,
            height_offset: 3.0,
            fov_bonus: 10.0,
        };
    }

    /// Set camera mode for ground gameplay
    pub fn set_ground_mode(&mut self, first_person: bool) {
        if first_person {
            self.mode = CameraMode::FirstPerson { eye_height: 1.7 };
        } else {
            self.mode = CameraMode::ThirdPerson {
                distance: 5.0,
                height_offset: 2.0,
            };
        }
    }

    /// Set spectate mode
    pub fn set_spectate_mode(&mut self, target_id: u8) {
        self.mode = CameraMode::Spectate { target_id };
    }

    /// Set victory mode
    pub fn set_victory_mode(&mut self) {
        self.mode = CameraMode::Victory {
            orbit_angle: 0.0,
            orbit_radius: 10.0,
        };
    }

    /// Get current FOV (may be modified by mode)
    pub fn get_fov(&self) -> f32 {
        match self.mode {
            CameraMode::Gliding { fov_bonus, .. } => self.fov + fov_bonus.to_radians(),
            _ => self.fov,
        }
    }
}
