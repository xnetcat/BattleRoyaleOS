//! Voxel rendering system
//!
//! Provides voxel-based model creation and rendering for blocky Minecraft-style graphics.
//! Voxels are converted to triangle meshes for rendering with the existing rasterizer.

use alloc::vec::Vec;
use alloc::vec;
use glam::{Vec2, Vec3};
use crate::vertex::Vertex;
use crate::mesh::Mesh;

/// A color in the voxel palette (RGB)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoxelColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl VoxelColor {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub const fn from_hex(hex: u32) -> Self {
        Self {
            r: ((hex >> 16) & 0xFF) as u8,
            g: ((hex >> 8) & 0xFF) as u8,
            b: (hex & 0xFF) as u8,
        }
    }

    pub fn to_vec3(&self) -> Vec3 {
        Vec3::new(
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
        )
    }

    /// Apply simple lighting (darker for certain faces)
    pub fn shade(&self, factor: f32) -> Vec3 {
        Vec3::new(
            (self.r as f32 / 255.0) * factor,
            (self.g as f32 / 255.0) * factor,
            (self.b as f32 / 255.0) * factor,
        )
    }
}

/// Standard color palette for voxel models
pub mod palette {
    use super::VoxelColor;

    // Skin tones
    pub const SKIN_LIGHT: VoxelColor = VoxelColor::from_hex(0xFFDBB4);
    pub const SKIN_MEDIUM: VoxelColor = VoxelColor::from_hex(0xD4A574);
    pub const SKIN_DARK: VoxelColor = VoxelColor::from_hex(0x8B5A2B);

    // Hair colors
    pub const HAIR_BLACK: VoxelColor = VoxelColor::from_hex(0x2C2C2C);
    pub const HAIR_BROWN: VoxelColor = VoxelColor::from_hex(0x654321);
    pub const HAIR_BLONDE: VoxelColor = VoxelColor::from_hex(0xFFD700);
    pub const HAIR_RED: VoxelColor = VoxelColor::from_hex(0x8B4513);

    // Clothing
    pub const SHIRT_BLUE: VoxelColor = VoxelColor::from_hex(0x3366CC);
    pub const SHIRT_RED: VoxelColor = VoxelColor::from_hex(0xCC3333);
    pub const SHIRT_GREEN: VoxelColor = VoxelColor::from_hex(0x33CC33);
    pub const SHIRT_WHITE: VoxelColor = VoxelColor::from_hex(0xEEEEEE);
    pub const PANTS_BLUE: VoxelColor = VoxelColor::from_hex(0x2244AA);
    pub const PANTS_BLACK: VoxelColor = VoxelColor::from_hex(0x333333);
    pub const PANTS_BROWN: VoxelColor = VoxelColor::from_hex(0x8B4513);
    pub const SHOES_BLACK: VoxelColor = VoxelColor::from_hex(0x222222);
    pub const SHOES_BROWN: VoxelColor = VoxelColor::from_hex(0x654321);

    // Materials
    pub const WOOD_LIGHT: VoxelColor = VoxelColor::from_hex(0xDEB887);
    pub const WOOD_MEDIUM: VoxelColor = VoxelColor::from_hex(0xCD853F);
    pub const WOOD_DARK: VoxelColor = VoxelColor::from_hex(0x8B4513);
    pub const WOOD_PLANK: VoxelColor = VoxelColor::from_hex(0xC4A76E);
    pub const BRICK_RED: VoxelColor = VoxelColor::from_hex(0xB44332);
    pub const BRICK_BROWN: VoxelColor = VoxelColor::from_hex(0x8B5A2B);
    pub const STONE_GRAY: VoxelColor = VoxelColor::from_hex(0x808080);
    pub const STONE_DARK: VoxelColor = VoxelColor::from_hex(0x505050);
    pub const METAL_GRAY: VoxelColor = VoxelColor::from_hex(0xA0A0A0);
    pub const METAL_DARK: VoxelColor = VoxelColor::from_hex(0x606060);

    // Nature
    pub const GRASS_GREEN: VoxelColor = VoxelColor::from_hex(0x4CAF50);
    pub const GRASS_DARK: VoxelColor = VoxelColor::from_hex(0x388E3C);
    pub const LEAF_GREEN: VoxelColor = VoxelColor::from_hex(0x66BB6A);
    pub const DIRT_BROWN: VoxelColor = VoxelColor::from_hex(0x795548);
    pub const SAND_TAN: VoxelColor = VoxelColor::from_hex(0xD7CCC8);
    pub const WATER_BLUE: VoxelColor = VoxelColor::from_hex(0x42A5F5);

    // Sky
    pub const SKY_BLUE: VoxelColor = VoxelColor::from_hex(0x87CEEB);
    pub const CLOUD_WHITE: VoxelColor = VoxelColor::from_hex(0xFFFFFF);

    // Weapons
    pub const GUN_METAL: VoxelColor = VoxelColor::from_hex(0x4A4A4A);
    pub const GUN_DARK: VoxelColor = VoxelColor::from_hex(0x2A2A2A);
    pub const GUN_GRIP: VoxelColor = VoxelColor::from_hex(0x3D2B1F);
    pub const GUN_ACCENT: VoxelColor = VoxelColor::from_hex(0xCC3333);

    // Equipment
    pub const BACKPACK_GREEN: VoxelColor = VoxelColor::from_hex(0x556B2F);
    pub const BACKPACK_TAN: VoxelColor = VoxelColor::from_hex(0xD2B48C);
    pub const GLIDER_RED: VoxelColor = VoxelColor::from_hex(0xE53935);
    pub const GLIDER_BLUE: VoxelColor = VoxelColor::from_hex(0x1E88E5);

    // Chrome/metallic
    pub const CHROME: VoxelColor = VoxelColor::from_hex(0xCCCCCC);
    pub const CHROME_DARK: VoxelColor = VoxelColor::from_hex(0x999999);

    // Lights
    pub const HEADLIGHT: VoxelColor = VoxelColor::from_hex(0xFFFF99);
    pub const TAILLIGHT: VoxelColor = VoxelColor::from_hex(0xFF3333);

    // Fabric/materials
    pub const CANVAS_TAN: VoxelColor = VoxelColor::from_hex(0xD4C4A8);
    pub const ROPE_BROWN: VoxelColor = VoxelColor::from_hex(0x8B7355);
    pub const RUBBER: VoxelColor = VoxelColor::from_hex(0x1A1A1A);
    pub const GLASS: VoxelColor = VoxelColor::from_hex(0xAADDFF);

    // Bus colors
    pub const BUS_BLUE: VoxelColor = VoxelColor::from_hex(0x2266AA);
    pub const BUS_LIGHT_BLUE: VoxelColor = VoxelColor::from_hex(0x3388CC);
    pub const BUS_YELLOW: VoxelColor = VoxelColor::from_hex(0xFFCC00);

    // Balloon colors
    pub const BALLOON_RED: VoxelColor = VoxelColor::from_hex(0xCC3333);
    pub const BALLOON_WHITE: VoxelColor = VoxelColor::from_hex(0xEEEEEE);
}

/// A single voxel (empty or filled with color)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Voxel {
    Empty,
    Filled(VoxelColor),
}

impl Default for Voxel {
    fn default() -> Self {
        Voxel::Empty
    }
}

/// Face direction for a voxel
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    Top,    // +Y
    Bottom, // -Y
    Front,  // +Z
    Back,   // -Z
    Right,  // +X
    Left,   // -X
}

impl Face {
    pub fn normal(&self) -> Vec3 {
        match self {
            Face::Top => Vec3::Y,
            Face::Bottom => Vec3::NEG_Y,
            Face::Front => Vec3::Z,
            Face::Back => Vec3::NEG_Z,
            Face::Right => Vec3::X,
            Face::Left => Vec3::NEG_X,
        }
    }

    /// Shading factor for each face (simple directional lighting)
    pub fn shade_factor(&self) -> f32 {
        match self {
            Face::Top => 1.0,
            Face::Bottom => 0.5,
            Face::Front => 0.85,
            Face::Back => 0.65,
            Face::Right => 0.75,
            Face::Left => 0.75,
        }
    }
}

/// A 3D voxel model with fixed dimensions
#[derive(Clone)]
pub struct VoxelModel {
    pub width: usize,   // X dimension
    pub height: usize,  // Y dimension
    pub depth: usize,   // Z dimension
    pub voxels: Vec<Voxel>,
    pub origin: Vec3,   // Offset for positioning
}

impl VoxelModel {
    /// Create an empty voxel model
    pub fn new(width: usize, height: usize, depth: usize) -> Self {
        let size = width * height * depth;
        Self {
            width,
            height,
            depth,
            voxels: vec![Voxel::Empty; size],
            origin: Vec3::ZERO,
        }
    }

    /// Create with custom origin
    pub fn with_origin(width: usize, height: usize, depth: usize, origin: Vec3) -> Self {
        let mut model = Self::new(width, height, depth);
        model.origin = origin;
        model
    }

    /// Get voxel index from coordinates
    #[inline]
    fn index(&self, x: usize, y: usize, z: usize) -> usize {
        x + y * self.width + z * self.width * self.height
    }

    /// Get voxel at position
    pub fn get(&self, x: usize, y: usize, z: usize) -> Voxel {
        if x >= self.width || y >= self.height || z >= self.depth {
            return Voxel::Empty;
        }
        self.voxels[self.index(x, y, z)]
    }

    /// Set voxel at position
    pub fn set(&mut self, x: usize, y: usize, z: usize, voxel: Voxel) {
        if x < self.width && y < self.height && z < self.depth {
            let idx = self.index(x, y, z);
            self.voxels[idx] = voxel;
        }
    }

    /// Set a voxel with color
    pub fn set_color(&mut self, x: usize, y: usize, z: usize, color: VoxelColor) {
        self.set(x, y, z, Voxel::Filled(color));
    }

    /// Fill a box region with a color
    pub fn fill_box(&mut self, x1: usize, y1: usize, z1: usize, x2: usize, y2: usize, z2: usize, color: VoxelColor) {
        for z in z1..=z2.min(self.depth - 1) {
            for y in y1..=y2.min(self.height - 1) {
                for x in x1..=x2.min(self.width - 1) {
                    self.set_color(x, y, z, color);
                }
            }
        }
    }

    /// Check if a face should be visible (not occluded by adjacent voxel)
    fn face_visible(&self, x: usize, y: usize, z: usize, face: Face) -> bool {
        let (nx, ny, nz) = match face {
            Face::Top => (x as i32, y as i32 + 1, z as i32),
            Face::Bottom => (x as i32, y as i32 - 1, z as i32),
            Face::Front => (x as i32, y as i32, z as i32 + 1),
            Face::Back => (x as i32, y as i32, z as i32 - 1),
            Face::Right => (x as i32 + 1, y as i32, z as i32),
            Face::Left => (x as i32 - 1, y as i32, z as i32),
        };

        // Face is visible if neighbor is outside bounds or empty
        if nx < 0 || ny < 0 || nz < 0 {
            return true;
        }
        if nx >= self.width as i32 || ny >= self.height as i32 || nz >= self.depth as i32 {
            return true;
        }

        matches!(self.get(nx as usize, ny as usize, nz as usize), Voxel::Empty)
    }

    /// Convert voxel model to triangle mesh
    pub fn to_mesh(&self, scale: f32) -> Mesh {
        let mut mesh = Mesh::new();

        for z in 0..self.depth {
            for y in 0..self.height {
                for x in 0..self.width {
                    if let Voxel::Filled(color) = self.get(x, y, z) {
                        // Check each face
                        for face in [Face::Top, Face::Bottom, Face::Front, Face::Back, Face::Right, Face::Left] {
                            if self.face_visible(x, y, z, face) {
                                self.add_face(&mut mesh, x, y, z, face, color, scale);
                            }
                        }
                    }
                }
            }
        }

        mesh
    }

    /// Add a single face to the mesh
    fn add_face(&self, mesh: &mut Mesh, x: usize, y: usize, z: usize, face: Face, color: VoxelColor, scale: f32) {
        let base_idx = mesh.vertices.len() as u32;
        let normal = face.normal();
        let shaded_color = color.shade(face.shade_factor());

        // Calculate world position with origin offset
        let wx = (x as f32 - self.origin.x) * scale;
        let wy = (y as f32 - self.origin.y) * scale;
        let wz = (z as f32 - self.origin.z) * scale;

        // Define face vertices (4 corners) - CCW winding when viewed from outside
        let positions: [Vec3; 4] = match face {
            Face::Top => [
                // CCW when viewed from +Y (above): back-left, front-left, front-right, back-right
                Vec3::new(wx, wy + scale, wz),
                Vec3::new(wx, wy + scale, wz + scale),
                Vec3::new(wx + scale, wy + scale, wz + scale),
                Vec3::new(wx + scale, wy + scale, wz),
            ],
            Face::Bottom => [
                // CCW when viewed from -Y (below): front-left, back-left, back-right, front-right
                Vec3::new(wx, wy, wz + scale),
                Vec3::new(wx, wy, wz),
                Vec3::new(wx + scale, wy, wz),
                Vec3::new(wx + scale, wy, wz + scale),
            ],
            Face::Front => [
                Vec3::new(wx, wy, wz + scale),
                Vec3::new(wx, wy + scale, wz + scale),
                Vec3::new(wx + scale, wy + scale, wz + scale),
                Vec3::new(wx + scale, wy, wz + scale),
            ],
            Face::Back => [
                Vec3::new(wx + scale, wy, wz),
                Vec3::new(wx + scale, wy + scale, wz),
                Vec3::new(wx, wy + scale, wz),
                Vec3::new(wx, wy, wz),
            ],
            Face::Right => [
                Vec3::new(wx + scale, wy, wz + scale),
                Vec3::new(wx + scale, wy + scale, wz + scale),
                Vec3::new(wx + scale, wy + scale, wz),
                Vec3::new(wx + scale, wy, wz),
            ],
            Face::Left => [
                Vec3::new(wx, wy, wz),
                Vec3::new(wx, wy + scale, wz),
                Vec3::new(wx, wy + scale, wz + scale),
                Vec3::new(wx, wy, wz + scale),
            ],
        };

        // Add 4 vertices
        for pos in &positions {
            mesh.vertices.push(Vertex {
                position: *pos,
                normal,
                color: shaded_color,
                uv: Vec2::ZERO,
            });
        }

        // Add 2 triangles (6 indices)
        mesh.indices.push(base_idx);
        mesh.indices.push(base_idx + 1);
        mesh.indices.push(base_idx + 2);
        mesh.indices.push(base_idx);
        mesh.indices.push(base_idx + 2);
        mesh.indices.push(base_idx + 3);
    }

    /// Count filled voxels
    pub fn voxel_count(&self) -> usize {
        self.voxels.iter().filter(|v| matches!(v, Voxel::Filled(_))).count()
    }

    /// Merge another model into this one at an offset
    pub fn merge(&mut self, other: &VoxelModel, offset_x: i32, offset_y: i32, offset_z: i32) {
        for z in 0..other.depth {
            for y in 0..other.height {
                for x in 0..other.width {
                    let voxel = other.get(x, y, z);
                    if let Voxel::Filled(_) = voxel {
                        let nx = x as i32 + offset_x;
                        let ny = y as i32 + offset_y;
                        let nz = z as i32 + offset_z;
                        if nx >= 0 && ny >= 0 && nz >= 0 {
                            self.set(nx as usize, ny as usize, nz as usize, voxel);
                        }
                    }
                }
            }
        }
    }
}

/// Character customization slots
#[derive(Debug, Clone, Copy)]
pub struct CharacterCustomization {
    pub skin_tone: u8,      // 0-2 (light, medium, dark)
    pub hair_style: u8,     // 0-3
    pub hair_color: u8,     // 0-3 (black, brown, blonde, red)
    pub shirt_color: u8,    // 0-3
    pub pants_color: u8,    // 0-2
    pub shoes_color: u8,    // 0-1
    pub backpack_style: u8, // 0-3 (none, small, medium, large)
    pub glider_style: u8,   // 0-3
}

impl Default for CharacterCustomization {
    fn default() -> Self {
        Self {
            skin_tone: 0,
            hair_style: 0,
            hair_color: 0,
            shirt_color: 0,
            pants_color: 0,
            shoes_color: 0,
            backpack_style: 1,
            glider_style: 0,
        }
    }
}

impl CharacterCustomization {
    pub fn skin_color(&self) -> VoxelColor {
        match self.skin_tone {
            0 => palette::SKIN_LIGHT,
            1 => palette::SKIN_MEDIUM,
            _ => palette::SKIN_DARK,
        }
    }

    pub fn hair_color(&self) -> VoxelColor {
        match self.hair_color {
            0 => palette::HAIR_BLACK,
            1 => palette::HAIR_BROWN,
            2 => palette::HAIR_BLONDE,
            _ => palette::HAIR_RED,
        }
    }

    pub fn shirt_color(&self) -> VoxelColor {
        match self.shirt_color {
            0 => palette::SHIRT_BLUE,
            1 => palette::SHIRT_RED,
            2 => palette::SHIRT_GREEN,
            _ => palette::SHIRT_WHITE,
        }
    }

    pub fn pants_color(&self) -> VoxelColor {
        match self.pants_color {
            0 => palette::PANTS_BLUE,
            1 => palette::PANTS_BLACK,
            _ => palette::PANTS_BROWN,
        }
    }

    pub fn shoes_color(&self) -> VoxelColor {
        match self.shoes_color {
            0 => palette::SHOES_BLACK,
            _ => palette::SHOES_BROWN,
        }
    }
}
