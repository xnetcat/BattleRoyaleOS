//! Vertex transformation pipeline

use glam::{Mat4, Vec3, Vec4};
use renderer::vertex::Vertex;

/// Transform a vertex from world space to screen space
pub fn transform_vertex(
    vertex: &Vertex,
    model: &Mat4,
    view: &Mat4,
    projection: &Mat4,
    viewport_width: f32,
    viewport_height: f32,
) -> Vertex {
    // Model-View-Projection transformation
    let world_pos = *model * Vec4::new(vertex.position.x, vertex.position.y, vertex.position.z, 1.0);
    let view_pos = *view * world_pos;
    let clip_pos = *projection * view_pos;

    // Perspective division
    let w = clip_pos.w;
    if w.abs() < 0.0001 {
        return vertex.clone();
    }

    let ndc = Vec3::new(clip_pos.x / w, clip_pos.y / w, clip_pos.z / w);

    // Viewport transformation (NDC to screen coordinates)
    let screen_x = (ndc.x + 1.0) * 0.5 * viewport_width;
    let screen_y = (1.0 - ndc.y) * 0.5 * viewport_height; // Flip Y
    // Use 1/w for depth (linear depth, better precision)
    // Closer objects have larger 1/w values
    let screen_z = 1.0 / w;

    Vertex {
        position: Vec3::new(screen_x, screen_y, screen_z),
        normal: vertex.normal,
        color: vertex.color,
        uv: vertex.uv,
    }
}

/// Transform a triangle and perform backface culling
/// Returns None if the triangle should be culled
pub fn transform_triangle(
    v0: &Vertex,
    v1: &Vertex,
    v2: &Vertex,
    model: &Mat4,
    view: &Mat4,
    projection: &Mat4,
    viewport_width: f32,
    viewport_height: f32,
) -> Option<(Vertex, Vertex, Vertex)> {
    let tv0 = transform_vertex(v0, model, view, projection, viewport_width, viewport_height);
    let tv1 = transform_vertex(v1, model, view, projection, viewport_width, viewport_height);
    let tv2 = transform_vertex(v2, model, view, projection, viewport_width, viewport_height);

    // Near plane clipping (simple rejection)
    if tv0.position.z < 0.0 || tv1.position.z < 0.0 || tv2.position.z < 0.0 {
        return None;
    }

    // Far plane clipping
    if tv0.position.z > 1.0 && tv1.position.z > 1.0 && tv2.position.z > 1.0 {
        return None;
    }

    // Backface culling using screen-space winding order
    let edge1 = tv1.position - tv0.position;
    let edge2 = tv2.position - tv0.position;
    let cross_z = edge1.x * edge2.y - edge1.y * edge2.x;

    // In screen space with Y pointing down, front-facing triangles
    // (CCW in world space) become CW, giving negative cross_z
    if cross_z > 0.0 {
        return None;
    }

    Some((tv0, tv1, tv2))
}

/// Create a perspective projection matrix
pub fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    Mat4::perspective_rh(fov_y, aspect, near, far)
}

/// Create a look-at view matrix
pub fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Mat4 {
    Mat4::look_at_rh(eye, target, up)
}

/// Create a translation matrix
pub fn translate(offset: Vec3) -> Mat4 {
    Mat4::from_translation(offset)
}

/// Create a rotation matrix from Euler angles (in radians)
pub fn rotate(pitch: f32, yaw: f32, roll: f32) -> Mat4 {
    Mat4::from_euler(glam::EulerRot::YXZ, yaw, pitch, roll)
}

/// Create a scale matrix
pub fn scale(s: Vec3) -> Mat4 {
    Mat4::from_scale(s)
}

/// Simple directional lighting
pub fn apply_lighting(vertex: &mut Vertex, light_dir: Vec3, ambient: f32) {
    let normal = vertex.normal.normalize();
    let intensity = normal.dot(-light_dir).max(0.0);
    let total_light = (ambient + intensity * (1.0 - ambient)).clamp(0.0, 1.0);

    vertex.color *= total_light;
}
