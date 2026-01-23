//! Vertex format definition

use glam::{Vec2, Vec3};

/// A vertex with position, normal, color, and UV coordinates
#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub color: Vec3,
    pub uv: Vec2,
}

impl Vertex {
    /// Create a new vertex
    pub const fn new(position: Vec3, normal: Vec3, color: Vec3, uv: Vec2) -> Self {
        Self {
            position,
            normal,
            color,
            uv,
        }
    }

    /// Create a vertex with just position and color
    pub const fn pos_color(position: Vec3, color: Vec3) -> Self {
        Self {
            position,
            normal: Vec3::new(0.0, 1.0, 0.0),
            color,
            uv: Vec2::new(0.0, 0.0),
        }
    }

    /// Linearly interpolate between two vertices
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        Self {
            position: self.position.lerp(other.position, t),
            normal: self.normal.lerp(other.normal, t).normalize(),
            color: self.color.lerp(other.color, t),
            uv: self.uv.lerp(other.uv, t),
        }
    }
}

impl Default for Vertex {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            normal: Vec3::Y,
            color: Vec3::ONE,
            uv: Vec2::ZERO,
        }
    }
}
