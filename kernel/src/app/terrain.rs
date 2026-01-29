//! Terrain Generation
//!
//! Creates 3D terrain meshes with procedural heightmaps.

use glam::{Vec2, Vec3};
use renderer::mesh::Mesh;
use renderer::vertex::Vertex;

/// Create a 3D terrain mesh with proper hills and valleys
/// Uses Perlin-like noise for natural-looking terrain
pub fn create_3d_terrain(size: f32, subdivisions: usize) -> Mesh {
    let mut terrain_mesh = Mesh::new();

    let half = size / 2.0;
    let step = size / subdivisions as f32;

    // Create vertices with height variation
    for z in 0..=subdivisions {
        for x in 0..=subdivisions {
            let fx = x as f32 * step - half;
            let fz = z as f32 * step - half;

            // Multi-octave noise for more natural terrain
            // Large hills
            let h1 = libm::sinf(fx * 0.01) * libm::cosf(fz * 0.01) * 15.0;
            // Medium bumps
            let h2 = libm::sinf(fx * 0.05) * libm::sinf(fz * 0.05) * 5.0;
            // Small details
            let h3 = libm::sinf(fx * 0.15 + fz * 0.1) * 2.0;
            // Add some valleys
            let h4 = libm::cosf((fx + fz) * 0.02) * 8.0;

            let height = h1 + h2 + h3 + h4;

            // Color variation based on height (grass -> dirt -> rock)
            let color = if height > 10.0 {
                // Rocky peaks - gray
                Vec3::new(0.5, 0.5, 0.45)
            } else if height > 5.0 {
                // High grass - darker green
                Vec3::new(0.2, 0.5, 0.2)
            } else if height > -5.0 {
                // Normal grass - bright green
                Vec3::new(0.3, 0.65, 0.25)
            } else {
                // Low areas - brownish
                Vec3::new(0.4, 0.35, 0.2)
            };

            terrain_mesh.vertices.push(Vertex::new(
                Vec3::new(fx, height, fz),
                Vec3::Y, // Will be recalculated
                color,
                Vec2::new(x as f32 / subdivisions as f32, z as f32 / subdivisions as f32),
            ));
        }
    }

    // Create indices for triangles
    let row_size = subdivisions + 1;
    for z in 0..subdivisions {
        for x in 0..subdivisions {
            let tl = (z * row_size + x) as u32;
            let tr = tl + 1;
            let bl = tl + row_size as u32;
            let br = bl + 1;

            // Two triangles per quad
            terrain_mesh.indices.extend([tl, bl, tr]);
            terrain_mesh.indices.extend([tr, bl, br]);
        }
    }

    // Recalculate normals for proper lighting
    recalculate_normals(&mut terrain_mesh);

    terrain_mesh
}

/// Recalculate vertex normals from face normals
fn recalculate_normals(mesh: &mut Mesh) {
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
}

extern crate alloc;
