//! Combat system with hitscan and damage calculation

use glam::Vec3;
use super::weapon::{Weapon, WeaponType};
use super::player::Player;

/// Result of a hitscan check
#[derive(Debug, Clone, Copy)]
pub enum HitResult {
    /// No hit
    Miss,
    /// Hit a player
    PlayerHit {
        player_id: u8,
        damage: u8,
        headshot: bool,
        distance: f32,
    },
    /// Hit world geometry
    WorldHit {
        position: Vec3,
        distance: f32,
    },
}

/// Player hitbox dimensions (in world units)
pub const PLAYER_HEIGHT: f32 = 1.8;
pub const PLAYER_WIDTH: f32 = 0.6;
pub const PLAYER_DEPTH: f32 = 0.4;
pub const HEAD_RADIUS: f32 = 0.2;
pub const HEAD_HEIGHT: f32 = 1.65; // Center of head relative to feet

/// Damage falloff ranges
pub const FALLOFF_START: f32 = 50.0;
pub const FALLOFF_END: f32 = 100.0;
pub const FALLOFF_MIN_MULT: f32 = 0.7;

/// Combat manager
#[derive(Debug, Clone)]
pub struct CombatManager {
    /// Recent hits for visual feedback
    pub hit_markers: [Option<HitMarker>; 8],
    /// Recent damage numbers
    pub damage_numbers: [Option<DamageNumber>; 16],
    /// Kill feed entries
    pub kill_feed: [Option<KillFeedEntry>; 6],
}

/// Hit marker for visual feedback
#[derive(Debug, Clone, Copy)]
pub struct HitMarker {
    pub timer: f32,
    pub headshot: bool,
}

/// Floating damage number
#[derive(Debug, Clone, Copy)]
pub struct DamageNumber {
    pub position: Vec3,
    pub damage: u8,
    pub headshot: bool,
    pub timer: f32,
    pub velocity_y: f32,
}

/// Kill feed entry
#[derive(Debug, Clone, Copy)]
pub struct KillFeedEntry {
    pub killer_id: u8,
    pub victim_id: u8,
    pub weapon_type: WeaponType,
    pub headshot: bool,
    pub timer: f32,
}

impl Default for CombatManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CombatManager {
    pub fn new() -> Self {
        Self {
            hit_markers: [None; 8],
            damage_numbers: [None; 16],
            kill_feed: [None; 6],
        }
    }

    /// Update combat effects
    pub fn update(&mut self, dt: f32) {
        // Update hit markers
        for marker in &mut self.hit_markers {
            if let Some(m) = marker {
                m.timer -= dt;
                if m.timer <= 0.0 {
                    *marker = None;
                }
            }
        }

        // Update damage numbers
        for number in &mut self.damage_numbers {
            if let Some(n) = number {
                n.timer -= dt;
                n.position.y += n.velocity_y * dt;
                n.velocity_y -= 9.8 * dt; // Gravity
                if n.timer <= 0.0 {
                    *number = None;
                }
            }
        }

        // Update kill feed
        for entry in &mut self.kill_feed {
            if let Some(e) = entry {
                e.timer -= dt;
                if e.timer <= 0.0 {
                    *entry = None;
                }
            }
        }
    }

    /// Add a hit marker
    pub fn add_hit_marker(&mut self, headshot: bool) {
        for marker in &mut self.hit_markers {
            if marker.is_none() {
                *marker = Some(HitMarker {
                    timer: 0.5,
                    headshot,
                });
                return;
            }
        }
        // Replace oldest if full
        self.hit_markers[0] = Some(HitMarker {
            timer: 0.5,
            headshot,
        });
    }

    /// Add a damage number
    pub fn add_damage_number(&mut self, position: Vec3, damage: u8, headshot: bool) {
        for number in &mut self.damage_numbers {
            if number.is_none() {
                *number = Some(DamageNumber {
                    position,
                    damage,
                    headshot,
                    timer: 1.5,
                    velocity_y: 3.0,
                });
                return;
            }
        }
        // Replace oldest if full
        self.damage_numbers[0] = Some(DamageNumber {
            position,
            damage,
            headshot,
            timer: 1.5,
            velocity_y: 3.0,
        });
    }

    /// Add a kill feed entry
    pub fn add_kill(&mut self, killer_id: u8, victim_id: u8, weapon_type: WeaponType, headshot: bool) {
        // Shift entries down
        for i in (1..self.kill_feed.len()).rev() {
            self.kill_feed[i] = self.kill_feed[i - 1];
        }
        self.kill_feed[0] = Some(KillFeedEntry {
            killer_id,
            victim_id,
            weapon_type,
            headshot,
            timer: 5.0,
        });
    }
}

/// Perform a hitscan shot from shooter
pub fn hitscan(
    origin: Vec3,
    direction: Vec3,
    weapon: &Weapon,
    shooter_id: u8,
    players: &[Player],
) -> HitResult {
    let max_range = weapon.weapon_type.range();
    let mut closest_hit: Option<(f32, u8, bool)> = None;

    // Check against all players
    for player in players {
        // Skip self, dead players, or players on bus
        if player.id == shooter_id || player.health == 0 {
            continue;
        }

        // Ray-capsule intersection for body
        if let Some((dist, is_head)) = ray_player_intersection(origin, direction, player) {
            if dist <= max_range {
                match closest_hit {
                    Some((closest_dist, _, _)) if dist >= closest_dist => {}
                    _ => {
                        closest_hit = Some((dist, player.id, is_head));
                    }
                }
            }
        }
    }

    if let Some((distance, player_id, headshot)) = closest_hit {
        // Calculate damage with falloff and headshot
        let mut damage = weapon.damage() as f32;

        // Apply headshot multiplier
        if headshot {
            damage *= weapon.weapon_type.headshot_multiplier();
        }

        // Apply distance falloff
        if distance > FALLOFF_START {
            let falloff_progress = ((distance - FALLOFF_START) / (FALLOFF_END - FALLOFF_START)).min(1.0);
            let falloff_mult = 1.0 - (1.0 - FALLOFF_MIN_MULT) * falloff_progress;
            damage *= falloff_mult;
        }

        HitResult::PlayerHit {
            player_id,
            damage: damage as u8,
            headshot,
            distance,
        }
    } else {
        HitResult::Miss
    }
}

/// Ray-player intersection test
/// Returns distance and whether it was a headshot
fn ray_player_intersection(origin: Vec3, direction: Vec3, player: &Player) -> Option<(f32, bool)> {
    let player_pos = player.position;

    // Check head first (sphere test)
    let head_center = player_pos + Vec3::new(0.0, HEAD_HEIGHT, 0.0);
    if let Some(dist) = ray_sphere_intersection(origin, direction, head_center, HEAD_RADIUS) {
        return Some((dist, true));
    }

    // Check body (capsule approximated as box)
    let body_min = player_pos + Vec3::new(-PLAYER_WIDTH / 2.0, 0.0, -PLAYER_DEPTH / 2.0);
    let body_max = player_pos + Vec3::new(PLAYER_WIDTH / 2.0, PLAYER_HEIGHT - 0.3, PLAYER_DEPTH / 2.0);

    if let Some(dist) = ray_aabb_intersection(origin, direction, body_min, body_max) {
        return Some((dist, false));
    }

    None
}

/// Ray-sphere intersection
fn ray_sphere_intersection(origin: Vec3, direction: Vec3, center: Vec3, radius: f32) -> Option<f32> {
    let oc = origin - center;
    let a = direction.dot(direction);
    let b = 2.0 * oc.dot(direction);
    let c = oc.dot(oc) - radius * radius;
    let discriminant = b * b - 4.0 * a * c;

    if discriminant < 0.0 {
        return None;
    }

    let t = (-b - libm::sqrtf(discriminant)) / (2.0 * a);
    if t > 0.0 {
        Some(t)
    } else {
        None
    }
}

/// Ray-AABB intersection
fn ray_aabb_intersection(origin: Vec3, direction: Vec3, min: Vec3, max: Vec3) -> Option<f32> {
    let inv_dir = Vec3::new(
        if direction.x.abs() < 0.0001 { f32::MAX } else { 1.0 / direction.x },
        if direction.y.abs() < 0.0001 { f32::MAX } else { 1.0 / direction.y },
        if direction.z.abs() < 0.0001 { f32::MAX } else { 1.0 / direction.z },
    );

    let t1 = (min.x - origin.x) * inv_dir.x;
    let t2 = (max.x - origin.x) * inv_dir.x;
    let t3 = (min.y - origin.y) * inv_dir.y;
    let t4 = (max.y - origin.y) * inv_dir.y;
    let t5 = (min.z - origin.z) * inv_dir.z;
    let t6 = (max.z - origin.z) * inv_dir.z;

    let tmin = t1.min(t2).max(t3.min(t4)).max(t5.min(t6));
    let tmax = t1.max(t2).min(t3.max(t4)).min(t5.max(t6));

    if tmax < 0.0 || tmin > tmax {
        return None;
    }

    Some(if tmin < 0.0 { tmax } else { tmin })
}

/// Apply spread to a direction vector
pub fn apply_spread(direction: Vec3, spread_degrees: f32, seed: u32) -> Vec3 {
    if spread_degrees <= 0.0 {
        return direction;
    }

    // Simple pseudo-random based on seed
    let random1 = ((seed * 1103515245 + 12345) % 0x7FFFFFFF) as f32 / 0x7FFFFFFF as f32;
    let random2 = ((seed * 1103515245 + 12345).wrapping_mul(7) % 0x7FFFFFFF) as f32 / 0x7FFFFFFF as f32;

    let spread_rad = spread_degrees.to_radians();
    let angle = random1 * core::f32::consts::TAU;
    let offset = random2 * spread_rad;

    // Create perpendicular vectors
    let up = if direction.y.abs() < 0.9 {
        Vec3::Y
    } else {
        Vec3::X
    };
    let right = direction.cross(up).normalize();
    let up = right.cross(direction).normalize();

    // Apply offset
    let offset_vec = right * libm::cosf(angle) * libm::sinf(offset)
                   + up * libm::sinf(angle) * libm::sinf(offset);

    (direction + offset_vec).normalize()
}

/// Shotgun pellet spread pattern (returns multiple directions)
pub fn shotgun_pellet_directions(base_direction: Vec3, pellet_count: u8, spread: f32, seed: u32) -> [Vec3; 10] {
    let mut directions = [base_direction; 10];

    for i in 0..pellet_count.min(10) as usize {
        let pellet_seed = seed.wrapping_add(i as u32 * 12345);
        directions[i] = apply_spread(base_direction, spread, pellet_seed);
    }

    directions
}

/// Calculate damage to structures
pub fn structure_damage(weapon: &Weapon) -> u16 {
    match weapon.weapon_type {
        WeaponType::Pickaxe => 50,
        WeaponType::Pistol => 15,
        WeaponType::Shotgun => 40,
        WeaponType::AssaultRifle => 25,
        WeaponType::Sniper => 100,
        WeaponType::Smg => 15,
    }
}
