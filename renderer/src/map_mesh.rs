//! Map mesh generation
//!
//! Generates terrain, building, and vegetation meshes from GameMap data.

use alloc::vec::Vec;
use glam::{Vec2, Vec3};
use crate::mesh::Mesh;
use crate::vertex::Vertex;
use crate::voxel::{VoxelColor, palette};
use crate::voxel_models;

/// Chunk dimensions for terrain generation
pub const CHUNK_SIZE: f32 = 50.0;
pub const CHUNK_SUBDIVISIONS: usize = 10;

/// Generate a terrain chunk mesh
///
/// # Arguments
/// * `chunk_x` - Chunk X coordinate (in chunk units)
/// * `chunk_z` - Chunk Z coordinate (in chunk units)
/// * `height_fn` - Function that returns terrain height at (world_x, world_z)
/// * `scale` - World scale multiplier
pub fn generate_terrain_chunk<F>(
    chunk_x: i32,
    chunk_z: i32,
    height_fn: F,
    scale: f32,
) -> Mesh
where
    F: Fn(f32, f32) -> f32,
{
    let mut mesh = Mesh::new();

    let chunk_world_x = chunk_x as f32 * CHUNK_SIZE * scale;
    let chunk_world_z = chunk_z as f32 * CHUNK_SIZE * scale;
    let step = CHUNK_SIZE * scale / CHUNK_SUBDIVISIONS as f32;

    // Base terrain colors
    let grass_low = Vec3::new(0.3, 0.5, 0.2);
    let grass_high = Vec3::new(0.4, 0.7, 0.3);

    // Create vertices
    for z in 0..=CHUNK_SUBDIVISIONS {
        for x in 0..=CHUNK_SUBDIVISIONS {
            let world_x = chunk_world_x + x as f32 * step;
            let world_z = chunk_world_z + z as f32 * step;
            let height = height_fn(world_x / scale, world_z / scale) * scale;

            // Color based on height
            let height_factor = ((height / scale + 10.0) / 50.0).clamp(0.0, 1.0);
            let color = Vec3::new(
                grass_low.x + (grass_high.x - grass_low.x) * height_factor,
                grass_low.y + (grass_high.y - grass_low.y) * height_factor,
                grass_low.z + (grass_high.z - grass_low.z) * height_factor,
            );

            mesh.vertices.push(Vertex::new(
                Vec3::new(world_x, height, world_z),
                Vec3::Y,
                color,
                Vec2::new(x as f32 / CHUNK_SUBDIVISIONS as f32, z as f32 / CHUNK_SUBDIVISIONS as f32),
            ));
        }
    }

    // Create indices
    let row_size = CHUNK_SUBDIVISIONS + 1;
    for z in 0..CHUNK_SUBDIVISIONS {
        for x in 0..CHUNK_SUBDIVISIONS {
            let tl = (z * row_size + x) as u32;
            let tr = tl + 1;
            let bl = tl + row_size as u32;
            let br = bl + 1;

            mesh.indices.extend([tl, bl, tr]);
            mesh.indices.extend([tr, bl, br]);
        }
    }

    // Calculate proper normals
    calculate_normals(&mut mesh);

    mesh
}

/// Generate a simple terrain chunk without height function (flat with slight variation)
pub fn generate_simple_terrain_chunk(chunk_x: i32, chunk_z: i32, scale: f32) -> Mesh {
    generate_terrain_chunk(chunk_x, chunk_z, |x, z| {
        // Simple height variation using sine waves
        let h1 = libm::sinf(x * 0.05) * 2.0;
        let h2 = libm::sinf(z * 0.05) * 2.0;
        let h3 = libm::sinf((x + z) * 0.03) * 1.5;
        h1 + h2 + h3
    }, scale)
}

/// Building types for mesh generation
#[derive(Debug, Clone, Copy)]
pub enum BuildingMeshType {
    HouseSmall,
    HouseMedium,
    HouseLarge,
    Warehouse,
    Tower,
    Barn,
}

/// Generate a building mesh
pub fn generate_building_mesh(
    building_type: BuildingMeshType,
    position: Vec3,
    rotation: f32,
    scale: f32,
) -> Mesh {
    let base_mesh = match building_type {
        BuildingMeshType::HouseSmall => create_house_mesh(8.0, 6.0, 8.0, scale),
        BuildingMeshType::HouseMedium => create_house_mesh(12.0, 8.0, 10.0, scale),
        BuildingMeshType::HouseLarge => create_house_mesh(16.0, 12.0, 14.0, scale),
        BuildingMeshType::Warehouse => create_warehouse_mesh(20.0, 10.0, 30.0, scale),
        BuildingMeshType::Tower => create_tower_mesh(10.0, 40.0, 10.0, scale),
        BuildingMeshType::Barn => create_barn_mesh(15.0, 12.0, 20.0, scale),
    };

    // Transform vertices to world position with rotation
    transform_mesh(base_mesh, position, rotation)
}

/// Generate a vegetation mesh (tree, rock, bush)
#[derive(Debug, Clone, Copy)]
pub enum VegetationMeshType {
    PineTree,
    OakTree,
    Rock,
    Bush,
}

/// Generate vegetation mesh
pub fn generate_vegetation_mesh(
    veg_type: VegetationMeshType,
    position: Vec3,
    veg_scale: f32,
    world_scale: f32,
) -> Mesh {
    let model = match veg_type {
        VegetationMeshType::PineTree => voxel_models::create_pine_tree(),
        VegetationMeshType::OakTree => voxel_models::create_oak_tree(),
        VegetationMeshType::Rock => voxel_models::create_rock(0),
        VegetationMeshType::Bush => {
            // Create a simple bush (small oak tree)
            let mut bush = voxel_models::create_oak_tree();
            bush.origin.y = -2.0; // Lower origin
            bush
        }
    };

    let base_mesh = model.to_mesh(veg_scale * world_scale * 0.2);
    transform_mesh(base_mesh, position, 0.0)
}

/// Create a simple house mesh
fn create_house_mesh(width: f32, height: f32, depth: f32, scale: f32) -> Mesh {
    let mut mesh = Mesh::new();

    let w = width * scale * 0.5;
    let h = height * scale;
    let d = depth * scale * 0.5;

    // Wall color (beige/tan)
    let wall_color = Vec3::new(0.8, 0.7, 0.5);
    // Roof color (brown)
    let roof_color = Vec3::new(0.5, 0.3, 0.2);
    // Door/window color
    let detail_color = Vec3::new(0.3, 0.2, 0.15);

    // Create walls (box without top)
    add_box_faces(&mut mesh, Vec3::ZERO, Vec3::new(w * 2.0, h * 0.7, d * 2.0), wall_color, false);

    // Create roof (triangular prism)
    let roof_base_y = h * 0.7;
    let roof_peak_y = h;
    let roof_vertices = [
        // Front face
        Vertex::new(Vec3::new(-w, roof_base_y, d), Vec3::new(0.0, 0.0, 1.0), roof_color, Vec2::ZERO),
        Vertex::new(Vec3::new(w, roof_base_y, d), Vec3::new(0.0, 0.0, 1.0), roof_color, Vec2::ZERO),
        Vertex::new(Vec3::new(0.0, roof_peak_y, d), Vec3::new(0.0, 0.0, 1.0), roof_color, Vec2::ZERO),
        // Back face
        Vertex::new(Vec3::new(w, roof_base_y, -d), Vec3::new(0.0, 0.0, -1.0), roof_color, Vec2::ZERO),
        Vertex::new(Vec3::new(-w, roof_base_y, -d), Vec3::new(0.0, 0.0, -1.0), roof_color, Vec2::ZERO),
        Vertex::new(Vec3::new(0.0, roof_peak_y, -d), Vec3::new(0.0, 0.0, -1.0), roof_color, Vec2::ZERO),
    ];

    let base = mesh.vertices.len() as u32;
    mesh.vertices.extend(roof_vertices);
    mesh.indices.extend([base, base + 1, base + 2]);
    mesh.indices.extend([base + 3, base + 4, base + 5]);

    // Roof slopes
    let slope_normal = Vec3::new(0.707, 0.707, 0.0);
    let slope_vertices = [
        // Left slope
        Vertex::new(Vec3::new(-w, roof_base_y, d), slope_normal, roof_color * 0.9, Vec2::ZERO),
        Vertex::new(Vec3::new(-w, roof_base_y, -d), slope_normal, roof_color * 0.9, Vec2::ZERO),
        Vertex::new(Vec3::new(0.0, roof_peak_y, -d), slope_normal, roof_color * 0.9, Vec2::ZERO),
        Vertex::new(Vec3::new(0.0, roof_peak_y, d), slope_normal, roof_color * 0.9, Vec2::ZERO),
        // Right slope
        Vertex::new(Vec3::new(w, roof_base_y, -d), -slope_normal, roof_color * 0.8, Vec2::ZERO),
        Vertex::new(Vec3::new(w, roof_base_y, d), -slope_normal, roof_color * 0.8, Vec2::ZERO),
        Vertex::new(Vec3::new(0.0, roof_peak_y, d), -slope_normal, roof_color * 0.8, Vec2::ZERO),
        Vertex::new(Vec3::new(0.0, roof_peak_y, -d), -slope_normal, roof_color * 0.8, Vec2::ZERO),
    ];

    let base = mesh.vertices.len() as u32;
    mesh.vertices.extend(slope_vertices);
    mesh.indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    mesh.indices.extend([base + 4, base + 5, base + 6, base + 4, base + 6, base + 7]);

    mesh
}

/// Create a warehouse mesh
fn create_warehouse_mesh(width: f32, height: f32, depth: f32, scale: f32) -> Mesh {
    let mut mesh = Mesh::new();

    let w = width * scale * 0.5;
    let h = height * scale;
    let d = depth * scale * 0.5;

    // Industrial gray color
    let wall_color = Vec3::new(0.5, 0.5, 0.55);

    // Simple box shape
    add_box_faces(&mut mesh, Vec3::ZERO, Vec3::new(w * 2.0, h, d * 2.0), wall_color, true);

    mesh
}

/// Create a tower mesh
fn create_tower_mesh(width: f32, height: f32, depth: f32, scale: f32) -> Mesh {
    let mut mesh = Mesh::new();

    let w = width * scale * 0.5;
    let h = height * scale;
    let d = depth * scale * 0.5;

    // Concrete color
    let wall_color = Vec3::new(0.6, 0.6, 0.65);

    // Main tower body
    add_box_faces(&mut mesh, Vec3::ZERO, Vec3::new(w * 2.0, h, d * 2.0), wall_color, true);

    mesh
}

/// Create a barn mesh
fn create_barn_mesh(width: f32, height: f32, depth: f32, scale: f32) -> Mesh {
    let mut mesh = Mesh::new();

    let w = width * scale * 0.5;
    let h = height * scale;
    let d = depth * scale * 0.5;

    // Red barn color
    let wall_color = Vec3::new(0.6, 0.2, 0.15);

    // Main body
    add_box_faces(&mut mesh, Vec3::ZERO, Vec3::new(w * 2.0, h * 0.7, d * 2.0), wall_color, false);

    // Gambrel roof (simplified as triangular)
    let roof_color = Vec3::new(0.3, 0.15, 0.1);
    let roof_base_y = h * 0.7;
    let roof_peak_y = h;

    let slope_vertices = [
        Vertex::new(Vec3::new(-w, roof_base_y, d), Vec3::Y, roof_color, Vec2::ZERO),
        Vertex::new(Vec3::new(-w, roof_base_y, -d), Vec3::Y, roof_color, Vec2::ZERO),
        Vertex::new(Vec3::new(0.0, roof_peak_y, -d), Vec3::Y, roof_color, Vec2::ZERO),
        Vertex::new(Vec3::new(0.0, roof_peak_y, d), Vec3::Y, roof_color, Vec2::ZERO),
        Vertex::new(Vec3::new(w, roof_base_y, -d), Vec3::Y, roof_color * 0.9, Vec2::ZERO),
        Vertex::new(Vec3::new(w, roof_base_y, d), Vec3::Y, roof_color * 0.9, Vec2::ZERO),
        Vertex::new(Vec3::new(0.0, roof_peak_y, d), Vec3::Y, roof_color * 0.9, Vec2::ZERO),
        Vertex::new(Vec3::new(0.0, roof_peak_y, -d), Vec3::Y, roof_color * 0.9, Vec2::ZERO),
    ];

    let base = mesh.vertices.len() as u32;
    mesh.vertices.extend(slope_vertices);
    mesh.indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    mesh.indices.extend([base + 4, base + 5, base + 6, base + 4, base + 6, base + 7]);

    mesh
}

/// Add box faces to a mesh
fn add_box_faces(mesh: &mut Mesh, center: Vec3, size: Vec3, color: Vec3, include_top: bool) {
    let half = size * 0.5;

    // Face definitions: (normal, color_factor, vertices)
    let faces: &[(Vec3, f32, [[f32; 3]; 4])] = &[
        // Front (+Z)
        (Vec3::Z, 0.9, [
            [-half.x, 0.0, half.z],
            [half.x, 0.0, half.z],
            [half.x, size.y, half.z],
            [-half.x, size.y, half.z],
        ]),
        // Back (-Z)
        (Vec3::NEG_Z, 0.7, [
            [half.x, 0.0, -half.z],
            [-half.x, 0.0, -half.z],
            [-half.x, size.y, -half.z],
            [half.x, size.y, -half.z],
        ]),
        // Right (+X)
        (Vec3::X, 0.8, [
            [half.x, 0.0, half.z],
            [half.x, 0.0, -half.z],
            [half.x, size.y, -half.z],
            [half.x, size.y, half.z],
        ]),
        // Left (-X)
        (Vec3::NEG_X, 0.8, [
            [-half.x, 0.0, -half.z],
            [-half.x, 0.0, half.z],
            [-half.x, size.y, half.z],
            [-half.x, size.y, -half.z],
        ]),
    ];

    for (normal, brightness, positions) in faces {
        let face_color = color * brightness;
        let base = mesh.vertices.len() as u32;

        for pos in positions {
            mesh.vertices.push(Vertex::new(
                center + Vec3::new(pos[0], pos[1], pos[2]),
                *normal,
                face_color,
                Vec2::ZERO,
            ));
        }

        mesh.indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    // Top face
    if include_top {
        let base = mesh.vertices.len() as u32;
        let top_positions = [
            [-half.x, size.y, half.z],
            [half.x, size.y, half.z],
            [half.x, size.y, -half.z],
            [-half.x, size.y, -half.z],
        ];

        for pos in &top_positions {
            mesh.vertices.push(Vertex::new(
                center + Vec3::new(pos[0], pos[1], pos[2]),
                Vec3::Y,
                color,
                Vec2::ZERO,
            ));
        }

        mesh.indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}

/// Transform mesh vertices to world position with rotation
fn transform_mesh(mut mesh: Mesh, position: Vec3, rotation: f32) -> Mesh {
    let cos_r = libm::cosf(rotation);
    let sin_r = libm::sinf(rotation);

    for vertex in &mut mesh.vertices {
        // Rotate around Y axis
        let x = vertex.position.x;
        let z = vertex.position.z;
        vertex.position.x = x * cos_r - z * sin_r;
        vertex.position.z = x * sin_r + z * cos_r;

        // Translate
        vertex.position += position;

        // Rotate normal
        let nx = vertex.normal.x;
        let nz = vertex.normal.z;
        vertex.normal.x = nx * cos_r - nz * sin_r;
        vertex.normal.z = nx * sin_r + nz * cos_r;
    }

    mesh
}

/// Calculate proper normals for a mesh
fn calculate_normals(mesh: &mut Mesh) {
    let mut normals = alloc::vec![Vec3::ZERO; mesh.vertices.len()];

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

    for (i, normal) in normals.iter().enumerate() {
        let length = libm::sqrtf(normal.x * normal.x + normal.y * normal.y + normal.z * normal.z);
        let n = if length > 0.0001 {
            Vec3::new(normal.x / length, normal.y / length, normal.z / length)
        } else {
            Vec3::Y
        };
        mesh.vertices[i].normal = n;
    }
}

/// Generate lobby platform mesh (glowing hexagonal platform)
pub fn create_lobby_platform(scale: f32) -> Mesh {
    let mut mesh = Mesh::new();

    // Hexagonal platform
    let radius = 3.0 * scale;
    let height = 0.3 * scale;
    let segments = 6;

    // Platform color (glowing cyan/blue)
    let top_color = Vec3::new(0.4, 0.8, 1.0);
    let side_color = Vec3::new(0.2, 0.5, 0.8);

    // Top face vertices
    let center_idx = mesh.vertices.len() as u32;
    mesh.vertices.push(Vertex::new(Vec3::new(0.0, height, 0.0), Vec3::Y, top_color, Vec2::ZERO));

    for i in 0..segments {
        let angle = (i as f32 / segments as f32) * core::f32::consts::TAU;
        let x = libm::cosf(angle) * radius;
        let z = libm::sinf(angle) * radius;
        mesh.vertices.push(Vertex::new(Vec3::new(x, height, z), Vec3::Y, top_color * 0.8, Vec2::ZERO));
    }

    // Top face indices
    for i in 0..segments {
        let next = (i + 1) % segments;
        mesh.indices.extend([center_idx, center_idx + 1 + i as u32, center_idx + 1 + next as u32]);
    }

    // Side faces
    for i in 0..segments {
        let next = (i + 1) % segments;
        let angle1 = (i as f32 / segments as f32) * core::f32::consts::TAU;
        let angle2 = ((i + 1) as f32 / segments as f32) * core::f32::consts::TAU;

        let x1 = libm::cosf(angle1) * radius;
        let z1 = libm::sinf(angle1) * radius;
        let x2 = libm::cosf(angle2) * radius;
        let z2 = libm::sinf(angle2) * radius;

        let normal = Vec3::new((x1 + x2) * 0.5, 0.0, (z1 + z2) * 0.5).normalize();

        let base = mesh.vertices.len() as u32;
        mesh.vertices.push(Vertex::new(Vec3::new(x1, 0.0, z1), normal, side_color, Vec2::ZERO));
        mesh.vertices.push(Vertex::new(Vec3::new(x2, 0.0, z2), normal, side_color, Vec2::ZERO));
        mesh.vertices.push(Vertex::new(Vec3::new(x2, height, z2), normal, side_color * 1.2, Vec2::ZERO));
        mesh.vertices.push(Vertex::new(Vec3::new(x1, height, z1), normal, side_color * 1.2, Vec2::ZERO));

        mesh.indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    mesh
}

/// Generate palm tree mesh for lobby background
pub fn create_palm_tree_mesh(scale: f32) -> Mesh {
    let model = voxel_models::create_pine_tree(); // Use pine tree as base
    let mut mesh = model.to_mesh(scale * 0.3);

    // Recolor to palm tree colors
    let trunk_color = Vec3::new(0.5, 0.35, 0.2);
    let frond_color = Vec3::new(0.2, 0.6, 0.3);

    for vertex in &mut mesh.vertices {
        // Trunk is lower, fronds are higher
        if vertex.position.y < 2.0 * scale {
            vertex.color = trunk_color;
        } else {
            vertex.color = frond_color;
        }
    }

    mesh
}

/// Create a simple house mesh (for POI buildings)
pub fn create_house_mesh_simple(color: Vec3) -> Mesh {
    let mut mesh = Mesh::new();

    let wall_color = color;
    let roof_color = Vec3::new(0.5, 0.3, 0.2);

    // Scale
    let w = 5.0;
    let h = 4.0;
    let d = 5.0;

    // Walls
    add_box_faces(&mut mesh, Vec3::ZERO, Vec3::new(w * 2.0, h, d * 2.0), wall_color, false);

    // Roof (simple pyramid)
    let roof_base_y = h;
    let roof_peak_y = h + 2.5;

    let base = mesh.vertices.len() as u32;
    mesh.vertices.push(crate::vertex::Vertex::new(Vec3::new(-w, roof_base_y, -d), Vec3::Y, roof_color, Vec2::ZERO));
    mesh.vertices.push(crate::vertex::Vertex::new(Vec3::new(w, roof_base_y, -d), Vec3::Y, roof_color, Vec2::ZERO));
    mesh.vertices.push(crate::vertex::Vertex::new(Vec3::new(w, roof_base_y, d), Vec3::Y, roof_color, Vec2::ZERO));
    mesh.vertices.push(crate::vertex::Vertex::new(Vec3::new(-w, roof_base_y, d), Vec3::Y, roof_color, Vec2::ZERO));
    mesh.vertices.push(crate::vertex::Vertex::new(Vec3::new(0.0, roof_peak_y, 0.0), Vec3::Y, roof_color * 0.9, Vec2::ZERO));

    // Roof triangles
    mesh.indices.extend([base, base + 1, base + 4]); // Front slope
    mesh.indices.extend([base + 1, base + 2, base + 4]); // Right slope
    mesh.indices.extend([base + 2, base + 3, base + 4]); // Back slope
    mesh.indices.extend([base + 3, base, base + 4]); // Left slope

    mesh
}
