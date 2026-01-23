//! Storm/zone mechanics

use glam::Vec3;

/// Storm phase configuration
#[derive(Debug, Clone, Copy)]
pub struct StormPhase {
    pub radius: f32,
    pub shrink_time: f32, // seconds
    pub wait_time: f32,   // seconds before shrinking
    pub damage: u8,       // damage per tick
}

/// Default storm phases
const PHASES: &[StormPhase] = &[
    StormPhase {
        radius: 1000.0,
        shrink_time: 0.0,
        wait_time: 120.0,
        damage: 1,
    },
    StormPhase {
        radius: 500.0,
        shrink_time: 60.0,
        wait_time: 90.0,
        damage: 2,
    },
    StormPhase {
        radius: 250.0,
        shrink_time: 45.0,
        wait_time: 60.0,
        damage: 5,
    },
    StormPhase {
        radius: 100.0,
        shrink_time: 30.0,
        wait_time: 30.0,
        damage: 10,
    },
    StormPhase {
        radius: 25.0,
        shrink_time: 15.0,
        wait_time: 15.0,
        damage: 15,
    },
    StormPhase {
        radius: 0.0,
        shrink_time: 10.0,
        wait_time: 0.0,
        damage: 20,
    },
];

/// Storm state
#[derive(Debug, Clone)]
pub struct Storm {
    pub center: Vec3,
    pub radius: f32,
    pub target_center: Vec3,
    pub target_radius: f32,
    pub phase: usize,
    pub timer: f32,
    pub shrinking: bool,
}

impl Storm {
    pub fn new() -> Self {
        Self {
            center: Vec3::new(0.0, 0.0, 0.0),
            radius: PHASES[0].radius,
            target_center: Vec3::ZERO,
            target_radius: PHASES[0].radius,
            phase: 0,
            timer: PHASES[0].wait_time,
            shrinking: false,
        }
    }

    /// Update storm state
    pub fn update(&mut self, dt: f32) {
        self.timer -= dt;

        if self.timer <= 0.0 {
            if self.shrinking {
                // Finished shrinking, start waiting for next phase
                self.phase += 1;
                if self.phase < PHASES.len() {
                    self.timer = PHASES[self.phase].wait_time;
                    self.shrinking = false;

                    // Set new target
                    self.pick_next_target();
                }
            } else {
                // Start shrinking
                self.shrinking = true;
                if self.phase < PHASES.len() {
                    self.timer = PHASES[self.phase].shrink_time;
                }
            }
        }

        // Interpolate during shrink
        if self.shrinking && self.phase < PHASES.len() {
            let phase = &PHASES[self.phase];
            let t = 1.0 - (self.timer / phase.shrink_time).max(0.0);

            let prev_radius = if self.phase > 0 {
                PHASES[self.phase - 1].radius
            } else {
                PHASES[0].radius
            };

            self.radius = prev_radius + (phase.radius - prev_radius) * t;
            self.center = self.center.lerp(self.target_center, t * dt);
        }
    }

    /// Pick a new target center for the next phase
    fn pick_next_target(&mut self) {
        // Simple: move towards origin with some randomness
        // In a real game, this would be randomized within the current circle
        let offset_x = libm::sinf(self.phase as f32 * 17.3) * 50.0;
        let offset_z = libm::cosf(self.phase as f32 * 23.7) * 50.0;
        self.target_center = Vec3::new(offset_x, 0.0, offset_z);
    }

    /// Check if a position is inside the safe zone
    pub fn contains(&self, pos: Vec3) -> bool {
        let dx = pos.x - self.center.x;
        let dz = pos.z - self.center.z;
        let dist_sq = dx * dx + dz * dz;
        dist_sq <= self.radius * self.radius
    }

    /// Get damage per tick for current phase
    pub fn damage_per_tick(&self) -> u8 {
        if self.phase < PHASES.len() {
            PHASES[self.phase].damage
        } else {
            PHASES[PHASES.len() - 1].damage
        }
    }

    /// Get time remaining in current state
    pub fn time_remaining(&self) -> f32 {
        self.timer
    }

    /// Check if storm is currently shrinking
    pub fn is_shrinking(&self) -> bool {
        self.shrinking
    }

    /// Get current phase number
    pub fn current_phase(&self) -> usize {
        self.phase
    }
}
