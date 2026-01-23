//! Voxel world generation
//!
//! Creates terrain, buildings, and environmental features using voxels.

use alloc::vec::Vec;
use crate::voxel::{VoxelModel, VoxelColor, palette};
use crate::mesh::Mesh;
use glam::Vec3;

/// A chunk of terrain (16x16 area)
pub struct TerrainChunk {
    pub x: i32,
    pub z: i32,
    pub heightmap: [[u8; 16]; 16],
    pub surface_type: [[SurfaceType; 16]; 16],
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SurfaceType {
    Grass,
    Dirt,
    Sand,
    Stone,
    Water,
    Road,
}

impl SurfaceType {
    pub fn color(&self) -> VoxelColor {
        match self {
            SurfaceType::Grass => palette::GRASS_GREEN,
            SurfaceType::Dirt => palette::DIRT_BROWN,
            SurfaceType::Sand => palette::SAND_TAN,
            SurfaceType::Stone => palette::STONE_GRAY,
            SurfaceType::Water => palette::WATER_BLUE,
            SurfaceType::Road => palette::STONE_DARK,
        }
    }
}

impl TerrainChunk {
    /// Generate a terrain chunk with height variation
    pub fn generate(chunk_x: i32, chunk_z: i32, seed: u32) -> Self {
        let mut heightmap = [[0u8; 16]; 16];
        let mut surface_type = [[SurfaceType::Grass; 16]; 16];

        for lz in 0..16 {
            for lx in 0..16 {
                let wx = chunk_x * 16 + lx as i32;
                let wz = chunk_z * 16 + lz as i32;

                // Simple noise-like height generation
                let h1 = simple_noise(wx as f32 * 0.05, wz as f32 * 0.05, seed);
                let h2 = simple_noise(wx as f32 * 0.1, wz as f32 * 0.1, seed + 1000);
                let height = (h1 * 8.0 + h2 * 4.0 + 4.0) as u8;

                heightmap[lz][lx] = height.clamp(1, 32);

                // Determine surface type based on height and position
                let surface = if height < 3 {
                    SurfaceType::Water
                } else if height < 5 {
                    SurfaceType::Sand
                } else if height > 20 {
                    SurfaceType::Stone
                } else {
                    SurfaceType::Grass
                };

                surface_type[lz][lx] = surface;
            }
        }

        Self {
            x: chunk_x,
            z: chunk_z,
            heightmap,
            surface_type,
        }
    }

    /// Convert chunk to mesh
    pub fn to_mesh(&self, scale: f32) -> Mesh {
        let mut model = VoxelModel::new(16, 33, 16);
        model.origin = Vec3::new(0.0, 0.0, 0.0);

        for lz in 0..16 {
            for lx in 0..16 {
                let height = self.heightmap[lz][lx] as usize;
                let surface = self.surface_type[lz][lx];
                let top_color = surface.color();

                // Top surface
                model.set_color(lx, height, lz, top_color);

                // Fill below with dirt/stone
                for y in 0..height {
                    let color = if y < height.saturating_sub(3) {
                        palette::STONE_GRAY
                    } else {
                        palette::DIRT_BROWN
                    };
                    model.set_color(lx, y, lz, color);
                }
            }
        }

        let mut mesh = model.to_mesh(scale);

        // Offset mesh to world position
        let offset = Vec3::new(
            self.x as f32 * 16.0 * scale,
            0.0,
            self.z as f32 * 16.0 * scale,
        );

        for vertex in &mut mesh.vertices {
            vertex.position += offset;
        }

        mesh
    }
}

/// Simple pseudo-random noise function
fn simple_noise(x: f32, z: f32, seed: u32) -> f32 {
    let ix = libm::floorf(x) as i32;
    let iz = libm::floorf(z) as i32;
    let fx = x - libm::floorf(x);
    let fz = z - libm::floorf(z);

    let v00 = hash_float(ix, iz, seed);
    let v10 = hash_float(ix + 1, iz, seed);
    let v01 = hash_float(ix, iz + 1, seed);
    let v11 = hash_float(ix + 1, iz + 1, seed);

    let u = fx * fx * (3.0 - 2.0 * fx);
    let v = fz * fz * (3.0 - 2.0 * fz);

    let a = v00 + (v10 - v00) * u;
    let b = v01 + (v11 - v01) * u;

    a + (b - a) * v
}

fn hash_float(x: i32, z: i32, seed: u32) -> f32 {
    let n = x.wrapping_mul(374761393) + z.wrapping_mul(668265263) + seed as i32;
    let n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    let n = n ^ (n >> 16);
    (n & 0x7FFFFFFF) as f32 / 0x7FFFFFFF as f32
}

/// A placed building/structure in the world
#[derive(Clone)]
pub struct PlacedStructure {
    pub position: Vec3,
    pub rotation: f32, // Y rotation in radians
    pub structure_type: StructureType,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StructureType {
    SmallHouse,
    MediumHouse,
    LargeHouse,
    Tower,
    Warehouse,
    Fence,
    Crate,
}

/// Create a small house
/// Size: 20x16x24 voxels
pub fn create_small_house() -> VoxelModel {
    let mut model = VoxelModel::with_origin(20, 16, 24, Vec3::new(10.0, 0.0, 12.0));

    let wall = palette::WOOD_PLANK;
    let roof = palette::BRICK_RED;
    let window = VoxelColor::from_hex(0x88CCFF);
    let door = palette::WOOD_DARK;
    let floor = palette::WOOD_MEDIUM;

    // Foundation/floor
    model.fill_box(0, 0, 0, 19, 0, 23, floor);

    // Walls
    // Front wall (with door and windows)
    model.fill_box(0, 1, 0, 19, 10, 0, wall);
    // Door opening
    model.fill_box(8, 1, 0, 11, 7, 0, door);
    // Windows
    model.fill_box(2, 4, 0, 5, 7, 0, window);
    model.fill_box(14, 4, 0, 17, 7, 0, window);

    // Back wall
    model.fill_box(0, 1, 23, 19, 10, 23, wall);
    model.fill_box(4, 4, 23, 7, 7, 23, window);
    model.fill_box(12, 4, 23, 15, 7, 23, window);

    // Side walls
    model.fill_box(0, 1, 0, 0, 10, 23, wall);
    model.fill_box(3, 4, 0, 3, 7, 0, window);
    model.fill_box(19, 1, 0, 19, 10, 23, wall);

    // Roof (peaked)
    for y in 0..6 {
        let inset = y;
        model.fill_box(inset, 11 + y, 0, 19 - inset, 11 + y, 23, roof);
    }

    model
}

/// Create a warehouse/depot building
/// Size: 32x12x24 voxels
pub fn create_warehouse() -> VoxelModel {
    let mut model = VoxelModel::with_origin(32, 12, 24, Vec3::new(16.0, 0.0, 12.0));

    let wall = palette::METAL_GRAY;
    let wall_dark = palette::METAL_DARK;
    let roof = palette::METAL_DARK;
    let door = VoxelColor::from_hex(0x666666);
    let floor = VoxelColor::from_hex(0x444444);

    // Floor
    model.fill_box(0, 0, 0, 31, 0, 23, floor);

    // Walls (corrugated metal look)
    for y in 1..10 {
        let color = if y % 2 == 0 { wall } else { wall_dark };
        model.fill_box(0, y, 0, 31, y, 0, color);
        model.fill_box(0, y, 23, 31, y, 23, color);
        model.fill_box(0, y, 0, 0, y, 23, color);
        model.fill_box(31, y, 0, 31, y, 23, color);
    }

    // Large doors (front)
    model.fill_box(4, 1, 0, 12, 8, 0, door);
    model.fill_box(19, 1, 0, 27, 8, 0, door);

    // Roof (flat with slight curve)
    model.fill_box(0, 10, 0, 31, 10, 23, roof);
    model.fill_box(1, 11, 4, 30, 11, 19, roof);

    model
}

/// Create a watch tower
/// Size: 12x32x12 voxels
pub fn create_tower() -> VoxelModel {
    let mut model = VoxelModel::with_origin(12, 32, 12, Vec3::new(6.0, 0.0, 6.0));

    let wood = palette::WOOD_MEDIUM;
    let wood_dark = palette::WOOD_DARK;
    let floor = palette::WOOD_PLANK;

    // Support legs (4 corners)
    for &(x, z) in &[(0, 0), (10, 0), (0, 10), (10, 10)] {
        model.fill_box(x, 0, z, x + 1, 20, z + 1, wood_dark);
    }

    // Cross bracing
    for y in [4, 10, 16].iter() {
        model.fill_box(0, *y, 0, 11, *y, 0, wood);
        model.fill_box(0, *y, 11, 11, *y, 11, wood);
        model.fill_box(0, *y, 0, 0, *y, 11, wood);
        model.fill_box(11, *y, 0, 11, *y, 11, wood);
    }

    // Platform floor
    model.fill_box(0, 21, 0, 11, 21, 11, floor);

    // Platform walls (railing)
    model.fill_box(0, 22, 0, 11, 24, 0, wood);
    model.fill_box(0, 22, 11, 11, 24, 11, wood);
    model.fill_box(0, 22, 0, 0, 24, 11, wood);
    model.fill_box(11, 22, 0, 11, 24, 11, wood);

    // Opening in one side
    model.fill_box(4, 22, 0, 7, 24, 0, VoxelColor::from_hex(0x000000)); // Clear

    // Roof
    model.fill_box(0, 28, 0, 11, 28, 11, wood_dark);
    model.fill_box(2, 29, 2, 9, 29, 9, wood_dark);
    model.fill_box(4, 30, 4, 7, 30, 7, wood_dark);
    model.fill_box(5, 31, 5, 6, 31, 6, wood_dark);

    model
}

/// Create a wooden fence section
/// Size: 16x6x2 voxels
pub fn create_fence() -> VoxelModel {
    let mut model = VoxelModel::with_origin(16, 6, 2, Vec3::new(8.0, 0.0, 1.0));

    let wood = palette::WOOD_MEDIUM;
    let wood_dark = palette::WOOD_DARK;

    // Vertical posts
    model.fill_box(0, 0, 0, 1, 5, 1, wood_dark);
    model.fill_box(7, 0, 0, 8, 5, 1, wood_dark);
    model.fill_box(14, 0, 0, 15, 5, 1, wood_dark);

    // Horizontal rails
    model.fill_box(0, 4, 0, 15, 4, 0, wood);
    model.fill_box(0, 2, 0, 15, 2, 0, wood);

    model
}

/// Create a supply crate
/// Size: 4x4x4 voxels
pub fn create_supply_crate() -> VoxelModel {
    let mut model = VoxelModel::with_origin(4, 4, 4, Vec3::new(2.0, 0.0, 2.0));

    let wood = palette::WOOD_PLANK;
    let wood_dark = palette::WOOD_DARK;
    let metal = palette::METAL_GRAY;

    // Main body
    model.fill_box(0, 0, 0, 3, 3, 3, wood);

    // Dark corners/edges
    model.fill_box(0, 0, 0, 0, 3, 0, wood_dark);
    model.fill_box(3, 0, 0, 3, 3, 0, wood_dark);
    model.fill_box(0, 0, 3, 0, 3, 3, wood_dark);
    model.fill_box(3, 0, 3, 3, 3, 3, wood_dark);

    // Metal reinforcements
    model.fill_box(0, 0, 0, 3, 0, 0, metal);
    model.fill_box(0, 3, 0, 3, 3, 0, metal);

    model
}

/// Create a cloud
/// Size: 12x4x8 voxels
pub fn create_cloud(seed: u32) -> VoxelModel {
    let mut model = VoxelModel::with_origin(12, 4, 8, Vec3::new(6.0, 2.0, 4.0));

    let white = palette::CLOUD_WHITE;

    // Base layer
    model.fill_box(2, 0, 2, 9, 1, 5, white);

    // Middle layer
    model.fill_box(1, 1, 1, 10, 2, 6, white);

    // Top bumps based on seed
    if seed % 3 == 0 {
        model.fill_box(3, 3, 3, 5, 3, 4, white);
        model.fill_box(7, 3, 2, 8, 3, 5, white);
    } else if seed % 3 == 1 {
        model.fill_box(2, 3, 2, 4, 3, 5, white);
        model.fill_box(6, 3, 3, 9, 3, 4, white);
    } else {
        model.fill_box(4, 3, 2, 7, 3, 5, white);
    }

    model
}
