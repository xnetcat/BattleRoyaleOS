//! Math utilities using glam

use glam::{Mat4, Vec3};

/// Compute forward direction from yaw and pitch (in radians)
pub fn direction_from_angles(yaw: f32, pitch: f32) -> Vec3 {
    let cy = libm::cosf(yaw);
    let sy = libm::sinf(yaw);
    let cp = libm::cosf(pitch);
    let sp = libm::sinf(pitch);

    Vec3::new(sy * cp, sp, cy * cp).normalize()
}

/// Create a rotation matrix from yaw (around Y axis)
pub fn rotate_y(angle: f32) -> Mat4 {
    Mat4::from_rotation_y(angle)
}

/// Create a rotation matrix from pitch (around X axis)
pub fn rotate_x(angle: f32) -> Mat4 {
    Mat4::from_rotation_x(angle)
}

/// Linear interpolation
#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Clamp a value between min and max
#[inline]
pub fn clamp(value: f32, min: f32, max: f32) -> f32 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

/// Convert degrees to radians
#[inline]
pub fn deg_to_rad(deg: f32) -> f32 {
    deg * core::f32::consts::PI / 180.0
}

/// Convert radians to degrees
#[inline]
pub fn rad_to_deg(rad: f32) -> f32 {
    rad * 180.0 / core::f32::consts::PI
}
