//! Procedural mesh generation

use crate::vertex::Vertex;
use alloc::vec;
use alloc::vec::Vec;
use glam::{Vec2, Vec3};

/// A triangle mesh
#[derive(Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl Mesh {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    /// Get number of triangles
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Get a triangle by index
    pub fn get_triangle(&self, index: usize) -> Option<(&Vertex, &Vertex, &Vertex)> {
        let base = index * 3;
        if base + 2 >= self.indices.len() {
            return None;
        }
        let i0 = self.indices[base] as usize;
        let i1 = self.indices[base + 1] as usize;
        let i2 = self.indices[base + 2] as usize;
        Some((
            self.vertices.get(i0)?,
            self.vertices.get(i1)?,
            self.vertices.get(i2)?,
        ))
    }
}

/// Create a unit cube centered at origin with per-face shading
pub fn create_cube(base_color: Vec3) -> Mesh {
    let mut mesh = Mesh::new();

    // Define vertices for each face with normals
    let positions = [
        // Front face (+Z)
        Vec3::new(-0.5, -0.5, 0.5),
        Vec3::new(0.5, -0.5, 0.5),
        Vec3::new(0.5, 0.5, 0.5),
        Vec3::new(-0.5, 0.5, 0.5),
        // Back face (-Z)
        Vec3::new(0.5, -0.5, -0.5),
        Vec3::new(-0.5, -0.5, -0.5),
        Vec3::new(-0.5, 0.5, -0.5),
        Vec3::new(0.5, 0.5, -0.5),
        // Top face (+Y)
        Vec3::new(-0.5, 0.5, 0.5),
        Vec3::new(0.5, 0.5, 0.5),
        Vec3::new(0.5, 0.5, -0.5),
        Vec3::new(-0.5, 0.5, -0.5),
        // Bottom face (-Y)
        Vec3::new(-0.5, -0.5, -0.5),
        Vec3::new(0.5, -0.5, -0.5),
        Vec3::new(0.5, -0.5, 0.5),
        Vec3::new(-0.5, -0.5, 0.5),
        // Right face (+X)
        Vec3::new(0.5, -0.5, 0.5),
        Vec3::new(0.5, -0.5, -0.5),
        Vec3::new(0.5, 0.5, -0.5),
        Vec3::new(0.5, 0.5, 0.5),
        // Left face (-X)
        Vec3::new(-0.5, -0.5, -0.5),
        Vec3::new(-0.5, -0.5, 0.5),
        Vec3::new(-0.5, 0.5, 0.5),
        Vec3::new(-0.5, 0.5, -0.5),
    ];

    let normals = [
        Vec3::new(0.0, 0.0, 1.0),  // Front
        Vec3::new(0.0, 0.0, -1.0), // Back
        Vec3::new(0.0, 1.0, 0.0),  // Top
        Vec3::new(0.0, -1.0, 0.0), // Bottom
        Vec3::new(1.0, 0.0, 0.0),  // Right
        Vec3::new(-1.0, 0.0, 0.0), // Left
    ];

    // Simple directional light from upper-right-front
    let light_dir = Vec3::new(0.5, 0.7, 0.5).normalize();
    let ambient = 0.3;

    // Create vertices with per-face lighting
    for face in 0..6 {
        // Calculate lighting for this face
        let normal = normals[face];
        let dot = normal.dot(light_dir).max(0.0);
        let brightness = ambient + dot * (1.0 - ambient);
        let face_color = base_color * brightness;

        for v in 0..4 {
            let idx = face * 4 + v;
            mesh.vertices.push(Vertex {
                position: positions[idx],
                normal: normals[face],
                color: face_color,
                uv: Vec2::ZERO,
            });
        }
    }

    // Create indices (two triangles per face)
    for face in 0..6u32 {
        let base = face * 4;
        mesh.indices.push(base);
        mesh.indices.push(base + 1);
        mesh.indices.push(base + 2);
        mesh.indices.push(base);
        mesh.indices.push(base + 2);
        mesh.indices.push(base + 3);
    }

    mesh
}

/// Create a player mesh (simplified capsule - box body + head)
pub fn create_player_mesh(body_color: Vec3, head_color: Vec3) -> Mesh {
    let mut mesh = Mesh::new();

    // Body (tall box)
    let body = create_box(Vec3::new(0.4, 1.2, 0.3), Vec3::new(0.0, 0.6, 0.0), body_color);

    // Head (cube)
    let head = create_box(Vec3::new(0.3, 0.3, 0.3), Vec3::new(0.0, 1.35, 0.0), head_color);

    // Merge meshes
    let body_offset = mesh.vertices.len() as u32;
    mesh.vertices.extend(body.vertices);
    mesh.indices
        .extend(body.indices.iter().map(|i| i + body_offset));

    let head_offset = mesh.vertices.len() as u32;
    mesh.vertices.extend(head.vertices);
    mesh.indices
        .extend(head.indices.iter().map(|i| i + head_offset));

    mesh
}

/// Create a wall mesh (4x4x0.2)
pub fn create_wall_mesh(color: Vec3) -> Mesh {
    create_box(Vec3::new(4.0, 4.0, 0.2), Vec3::ZERO, color)
}

/// Create a ramp mesh
pub fn create_ramp_mesh(color: Vec3) -> Mesh {
    let mut mesh = Mesh::new();

    // Triangular prism ramp
    let vertices = [
        // Bottom triangle
        Vertex::new(
            Vec3::new(-2.0, 0.0, 2.0),
            Vec3::new(0.0, -1.0, 0.0),
            color,
            Vec2::ZERO,
        ),
        Vertex::new(
            Vec3::new(2.0, 0.0, 2.0),
            Vec3::new(0.0, -1.0, 0.0),
            color,
            Vec2::ZERO,
        ),
        Vertex::new(
            Vec3::new(-2.0, 0.0, -2.0),
            Vec3::new(0.0, -1.0, 0.0),
            color,
            Vec2::ZERO,
        ),
        Vertex::new(
            Vec3::new(2.0, 0.0, -2.0),
            Vec3::new(0.0, -1.0, 0.0),
            color,
            Vec2::ZERO,
        ),
        // Top edge
        Vertex::new(
            Vec3::new(-2.0, 4.0, -2.0),
            Vec3::new(0.0, 0.707, 0.707),
            color,
            Vec2::ZERO,
        ),
        Vertex::new(
            Vec3::new(2.0, 4.0, -2.0),
            Vec3::new(0.0, 0.707, 0.707),
            color,
            Vec2::ZERO,
        ),
    ];

    mesh.vertices.extend(vertices);

    // Bottom face
    mesh.indices.extend([0, 1, 3, 0, 3, 2]);
    // Ramp surface
    mesh.indices.extend([0, 2, 4, 0, 4, 5, 0, 5, 1]);
    // Left side
    mesh.indices.extend([0, 4, 2]);
    // Right side
    mesh.indices.extend([1, 3, 5]);
    // Back
    mesh.indices.extend([2, 4, 5, 2, 5, 3]);

    mesh
}

/// Create a battle bus mesh
pub fn create_battle_bus_mesh() -> Mesh {
    let mut mesh = Mesh::new();

    // Bus body
    let body = create_box(
        Vec3::new(3.0, 2.0, 6.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.2, 0.3, 0.8),
    );

    // Balloon (simplified as stretched cube)
    let balloon = create_box(
        Vec3::new(4.0, 3.0, 4.0),
        Vec3::new(0.0, 4.0, 0.0),
        Vec3::new(0.8, 0.2, 0.2),
    );

    // Merge
    let body_offset = mesh.vertices.len() as u32;
    mesh.vertices.extend(body.vertices);
    mesh.indices
        .extend(body.indices.iter().map(|i| i + body_offset));

    let balloon_offset = mesh.vertices.len() as u32;
    mesh.vertices.extend(balloon.vertices);
    mesh.indices
        .extend(balloon.indices.iter().map(|i| i + balloon_offset));

    mesh
}

/// Create a ground plane
pub fn create_ground_mesh(size: f32, color: Vec3) -> Mesh {
    let mut mesh = Mesh::new();

    let half = size / 2.0;
    mesh.vertices.push(Vertex::new(
        Vec3::new(-half, 0.0, -half),
        Vec3::Y,
        color,
        Vec2::new(0.0, 0.0),
    ));
    mesh.vertices.push(Vertex::new(
        Vec3::new(half, 0.0, -half),
        Vec3::Y,
        color,
        Vec2::new(1.0, 0.0),
    ));
    mesh.vertices.push(Vertex::new(
        Vec3::new(half, 0.0, half),
        Vec3::Y,
        color,
        Vec2::new(1.0, 1.0),
    ));
    mesh.vertices.push(Vertex::new(
        Vec3::new(-half, 0.0, half),
        Vec3::Y,
        color,
        Vec2::new(0.0, 1.0),
    ));

    mesh.indices.extend([0, 1, 2, 0, 2, 3]);

    mesh
}

/// Create a terrain grid mesh with height variation
/// subdivisions: number of quads per side (total triangles = 2 * subdivisions^2)
pub fn create_terrain_grid(size: f32, subdivisions: usize, base_color: Vec3) -> Mesh {
    let mut mesh = Mesh::new();

    let half = size / 2.0;
    let step = size / subdivisions as f32;

    // Create vertices
    for z in 0..=subdivisions {
        for x in 0..=subdivisions {
            let fx = x as f32 * step - half;
            let fz = z as f32 * step - half;

            // Simple height variation using a combination of sine waves
            let h1 = libm::sinf(fx * 0.5) * 0.3;
            let h2 = libm::sinf(fz * 0.5) * 0.3;
            let h3 = libm::sinf((fx + fz) * 0.3) * 0.2;
            let height = h1 + h2 + h3;

            // Color variation based on height
            let color_factor = (height + 1.0) * 0.5; // 0-1 range
            let color = Vec3::new(
                base_color.x * (0.7 + color_factor * 0.3),
                base_color.y * (0.5 + color_factor * 0.5),
                base_color.z * (0.7 + color_factor * 0.3),
            );

            mesh.vertices.push(Vertex::new(
                Vec3::new(fx, height, fz),
                Vec3::Y, // Will be recalculated for proper normals
                color,
                Vec2::new(x as f32 / subdivisions as f32, z as f32 / subdivisions as f32),
            ));
        }
    }

    // Create indices
    let row_size = subdivisions + 1;
    for z in 0..subdivisions {
        for x in 0..subdivisions {
            let tl = (z * row_size + x) as u32;
            let tr = tl + 1;
            let bl = tl + row_size as u32;
            let br = bl + 1;

            // Two triangles per quad
            mesh.indices.extend([tl, bl, tr]); // First triangle
            mesh.indices.extend([tr, bl, br]); // Second triangle
        }
    }

    // Calculate normals (average of adjacent face normals)
    let mut normals = vec![Vec3::ZERO; mesh.vertices.len()];

    for i in (0..mesh.indices.len()).step_by(3) {
        let i0 = mesh.indices[i] as usize;
        let i1 = mesh.indices[i + 1] as usize;
        let i2 = mesh.indices[i + 2] as usize;

        let v0 = mesh.vertices[i0].position;
        let v1 = mesh.vertices[i1].position;
        let v2 = mesh.vertices[i2].position;

        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let face_normal = edge1.cross(edge2);

        normals[i0] += face_normal;
        normals[i1] += face_normal;
        normals[i2] += face_normal;
    }

    // Normalize and apply
    for (i, normal) in normals.iter().enumerate() {
        let length = libm::sqrtf(normal.x * normal.x + normal.y * normal.y + normal.z * normal.z);
        let n = if length > 0.0001 {
            Vec3::new(normal.x / length, normal.y / length, normal.z / length)
        } else {
            Vec3::Y
        };
        mesh.vertices[i].normal = n;
    }

    mesh
}

/// Create a hexagonal lobby platform with glow effect
pub fn create_lobby_platform(scale: f32) -> Mesh {
    let mut mesh = Mesh::new();

    let radius = 3.0 * scale;
    let height = 0.3 * scale;
    let segments = 6;

    // Platform colors (glowing cyan/blue)
    let top_color = Vec3::new(0.4, 0.8, 1.0);
    let side_color = Vec3::new(0.2, 0.5, 0.8);

    // Top center vertex
    let center_idx = mesh.vertices.len() as u32;
    mesh.vertices.push(Vertex::new(Vec3::new(0.0, height, 0.0), Vec3::Y, top_color, Vec2::ZERO));

    // Top edge vertices
    for i in 0..segments {
        let angle = (i as f32 / segments as f32) * core::f32::consts::TAU;
        let x = libm::cosf(angle) * radius;
        let z = libm::sinf(angle) * radius;
        mesh.vertices.push(Vertex::new(Vec3::new(x, height, z), Vec3::Y, top_color * 0.8, Vec2::ZERO));
    }

    // Top face triangles
    for i in 0..segments {
        let next = (i + 1) % segments;
        mesh.indices.extend([center_idx, center_idx + 1 + i as u32, center_idx + 1 + next as u32]);
    }

    // Side faces
    for i in 0..segments {
        let angle1 = (i as f32 / segments as f32) * core::f32::consts::TAU;
        let angle2 = ((i + 1) as f32 / segments as f32) * core::f32::consts::TAU;

        let x1 = libm::cosf(angle1) * radius;
        let z1 = libm::sinf(angle1) * radius;
        let x2 = libm::cosf(angle2) * radius;
        let z2 = libm::sinf(angle2) * radius;

        let normal = Vec3::new((x1 + x2) * 0.5, 0.0, (z1 + z2) * 0.5).normalize();

        let base = mesh.vertices.len() as u32;
        mesh.vertices.push(Vertex::new(Vec3::new(x1, 0.0, z1), normal, side_color * 0.7, Vec2::ZERO));
        mesh.vertices.push(Vertex::new(Vec3::new(x2, 0.0, z2), normal, side_color * 0.7, Vec2::ZERO));
        mesh.vertices.push(Vertex::new(Vec3::new(x2, height, z2), normal, side_color, Vec2::ZERO));
        mesh.vertices.push(Vertex::new(Vec3::new(x1, height, z1), normal, side_color, Vec2::ZERO));

        mesh.indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    mesh
}

/// Create a palm tree mesh for lobby decoration
pub fn create_palm_tree(height: f32, frond_count: usize) -> Mesh {
    let mut mesh = Mesh::new();

    let trunk_color = Vec3::new(0.5, 0.35, 0.2);
    let frond_color = Vec3::new(0.2, 0.6, 0.3);
    let trunk_radius = height * 0.05;
    let segments = 6;

    // Trunk (tapered cylinder)
    for i in 0..segments {
        let angle1 = (i as f32 / segments as f32) * core::f32::consts::TAU;
        let angle2 = ((i + 1) as f32 / segments as f32) * core::f32::consts::TAU;

        let x1_bot = libm::cosf(angle1) * trunk_radius;
        let z1_bot = libm::sinf(angle1) * trunk_radius;
        let x2_bot = libm::cosf(angle2) * trunk_radius;
        let z2_bot = libm::sinf(angle2) * trunk_radius;

        let x1_top = libm::cosf(angle1) * trunk_radius * 0.7;
        let z1_top = libm::sinf(angle1) * trunk_radius * 0.7;
        let x2_top = libm::cosf(angle2) * trunk_radius * 0.7;
        let z2_top = libm::sinf(angle2) * trunk_radius * 0.7;

        let normal = Vec3::new((x1_bot + x2_bot) * 0.5, 0.0, (z1_bot + z2_bot) * 0.5).normalize();

        let base = mesh.vertices.len() as u32;
        mesh.vertices.push(Vertex::new(Vec3::new(x1_bot, 0.0, z1_bot), normal, trunk_color * 0.8, Vec2::ZERO));
        mesh.vertices.push(Vertex::new(Vec3::new(x2_bot, 0.0, z2_bot), normal, trunk_color * 0.8, Vec2::ZERO));
        mesh.vertices.push(Vertex::new(Vec3::new(x2_top, height, z2_top), normal, trunk_color, Vec2::ZERO));
        mesh.vertices.push(Vertex::new(Vec3::new(x1_top, height, z1_top), normal, trunk_color, Vec2::ZERO));

        mesh.indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    // Fronds (simplified as triangular shapes)
    let frond_length = height * 0.6;
    let frond_width = height * 0.15;

    for i in 0..frond_count {
        let angle = (i as f32 / frond_count as f32) * core::f32::consts::TAU;
        let droop = 0.3; // How much the frond droops

        let start_x = 0.0;
        let start_z = 0.0;
        let end_x = libm::cosf(angle) * frond_length;
        let end_z = libm::sinf(angle) * frond_length;
        let end_y = height - frond_length * droop;

        let perp_x = -libm::sinf(angle) * frond_width * 0.5;
        let perp_z = libm::cosf(angle) * frond_width * 0.5;

        let frond_normal = Vec3::Y;

        let base = mesh.vertices.len() as u32;
        mesh.vertices.push(Vertex::new(Vec3::new(start_x, height, start_z), frond_normal, frond_color, Vec2::ZERO));
        mesh.vertices.push(Vertex::new(Vec3::new(end_x + perp_x, end_y, end_z + perp_z), frond_normal, frond_color * 0.9, Vec2::ZERO));
        mesh.vertices.push(Vertex::new(Vec3::new(end_x - perp_x, end_y, end_z - perp_z), frond_normal, frond_color * 0.9, Vec2::ZERO));

        mesh.indices.extend([base, base + 1, base + 2]);
    }

    mesh
}

/// Create a cylindrical storm wall mesh
/// segments: number of vertical strips around the cylinder
/// height: how tall the wall is
pub fn create_storm_wall(segments: usize, height: f32) -> Mesh {
    let mut mesh = Mesh::new();

    // Storm wall is semi-transparent purple
    let storm_color = Vec3::new(0.5, 0.1, 0.6);
    let storm_color_light = Vec3::new(0.7, 0.2, 0.8);

    // Create vertical strips - only the outside faces (viewed from inside the circle)
    for i in 0..segments {
        let angle1 = (i as f32 / segments as f32) * core::f32::consts::TAU;
        let angle2 = ((i + 1) as f32 / segments as f32) * core::f32::consts::TAU;

        // Unit circle positions (will be scaled by radius at render time)
        let x1 = libm::cosf(angle1);
        let z1 = libm::sinf(angle1);
        let x2 = libm::cosf(angle2);
        let z2 = libm::sinf(angle2);

        // Normal pointing inward (toward center)
        let normal = Vec3::new(-(x1 + x2) * 0.5, 0.0, -(z1 + z2) * 0.5).normalize();

        // Alternate colors for visual effect
        let color = if i % 2 == 0 { storm_color } else { storm_color_light };

        let base = mesh.vertices.len() as u32;
        // Bottom vertices
        mesh.vertices.push(Vertex::new(Vec3::new(x1, 0.0, z1), normal, color * 0.6, Vec2::ZERO));
        mesh.vertices.push(Vertex::new(Vec3::new(x2, 0.0, z2), normal, color * 0.6, Vec2::ZERO));
        // Top vertices
        mesh.vertices.push(Vertex::new(Vec3::new(x2, height, z2), normal, color, Vec2::ZERO));
        mesh.vertices.push(Vertex::new(Vec3::new(x1, height, z1), normal, color, Vec2::ZERO));

        // Two triangles for this strip
        mesh.indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    mesh
}

/// Helper: Create a box with given dimensions and offset
fn create_box(size: Vec3, offset: Vec3, color: Vec3) -> Mesh {
    let mut mesh = Mesh::new();
    let half = size * 0.5;

    let positions = [
        // Front
        offset + Vec3::new(-half.x, -half.y, half.z),
        offset + Vec3::new(half.x, -half.y, half.z),
        offset + Vec3::new(half.x, half.y, half.z),
        offset + Vec3::new(-half.x, half.y, half.z),
        // Back
        offset + Vec3::new(half.x, -half.y, -half.z),
        offset + Vec3::new(-half.x, -half.y, -half.z),
        offset + Vec3::new(-half.x, half.y, -half.z),
        offset + Vec3::new(half.x, half.y, -half.z),
        // Top
        offset + Vec3::new(-half.x, half.y, half.z),
        offset + Vec3::new(half.x, half.y, half.z),
        offset + Vec3::new(half.x, half.y, -half.z),
        offset + Vec3::new(-half.x, half.y, -half.z),
        // Bottom
        offset + Vec3::new(-half.x, -half.y, -half.z),
        offset + Vec3::new(half.x, -half.y, -half.z),
        offset + Vec3::new(half.x, -half.y, half.z),
        offset + Vec3::new(-half.x, -half.y, half.z),
        // Right
        offset + Vec3::new(half.x, -half.y, half.z),
        offset + Vec3::new(half.x, -half.y, -half.z),
        offset + Vec3::new(half.x, half.y, -half.z),
        offset + Vec3::new(half.x, half.y, half.z),
        // Left
        offset + Vec3::new(-half.x, -half.y, -half.z),
        offset + Vec3::new(-half.x, -half.y, half.z),
        offset + Vec3::new(-half.x, half.y, half.z),
        offset + Vec3::new(-half.x, half.y, -half.z),
    ];

    let normals = [
        Vec3::Z,
        Vec3::NEG_Z,
        Vec3::Y,
        Vec3::NEG_Y,
        Vec3::X,
        Vec3::NEG_X,
    ];

    for face in 0..6 {
        for v in 0..4 {
            mesh.vertices.push(Vertex {
                position: positions[face * 4 + v],
                normal: normals[face],
                color,
                uv: Vec2::ZERO,
            });
        }
    }

    for face in 0..6u32 {
        let base = face * 4;
        mesh.indices.push(base);
        mesh.indices.push(base + 1);
        mesh.indices.push(base + 2);
        mesh.indices.push(base);
        mesh.indices.push(base + 2);
        mesh.indices.push(base + 3);
    }

    mesh
}
