//! Procedural voxel model generation
//!
//! Creates detailed voxel models for characters, weapons, buildings, etc.

use crate::voxel::{VoxelModel, VoxelColor, CharacterCustomization, palette};
use glam::Vec3;

/// Create a detailed voxel player character
/// Size: 8x24x4 voxels (width x height x depth)
pub fn create_player_model(customization: &CharacterCustomization) -> VoxelModel {
    let mut model = VoxelModel::with_origin(8, 24, 4, Vec3::new(4.0, 0.0, 2.0));

    let skin = customization.skin_color();
    let hair = customization.hair_color();
    let shirt = customization.shirt_color();
    let pants = customization.pants_color();
    let shoes = customization.shoes_color();

    // === LEGS (y: 0-7) ===
    // Left leg
    model.fill_box(1, 0, 1, 2, 1, 2, shoes);      // Left shoe
    model.fill_box(1, 2, 1, 2, 7, 2, pants);      // Left leg
    // Right leg
    model.fill_box(5, 0, 1, 6, 1, 2, shoes);      // Right shoe
    model.fill_box(5, 2, 1, 6, 7, 2, pants);      // Right leg

    // === TORSO (y: 8-15) ===
    model.fill_box(1, 8, 1, 6, 15, 2, shirt);     // Main body

    // === ARMS (y: 10-15) ===
    // Left arm
    model.fill_box(0, 10, 1, 0, 15, 2, shirt);    // Upper arm
    model.fill_box(0, 10, 1, 0, 11, 2, skin);     // Hand
    // Right arm
    model.fill_box(7, 10, 1, 7, 15, 2, shirt);    // Upper arm
    model.fill_box(7, 10, 1, 7, 11, 2, skin);     // Hand

    // === HEAD (y: 16-23) ===
    model.fill_box(2, 16, 0, 5, 23, 3, skin);     // Head base

    // Eyes (darker)
    let eye_color = VoxelColor::from_hex(0x222222);
    model.set_color(2, 20, 0, eye_color);         // Left eye
    model.set_color(5, 20, 0, eye_color);         // Right eye

    // Hair based on style
    match customization.hair_style {
        0 => {
            // Short hair
            model.fill_box(2, 22, 0, 5, 23, 3, hair);
        }
        1 => {
            // Medium hair
            model.fill_box(2, 21, 0, 5, 23, 3, hair);
            model.fill_box(1, 22, 1, 6, 23, 2, hair);
        }
        2 => {
            // Long hair
            model.fill_box(2, 20, 0, 5, 23, 3, hair);
            model.fill_box(1, 18, 1, 6, 23, 2, hair);
        }
        _ => {
            // Bald/buzzcut
            model.fill_box(2, 22, 1, 5, 23, 2, hair);
        }
    }

    model
}

/// Create first-person arms holding a weapon
/// Size: 12x8x6 voxels
pub fn create_fp_arms(skin: VoxelColor, shirt: VoxelColor) -> VoxelModel {
    let mut model = VoxelModel::with_origin(12, 8, 6, Vec3::new(6.0, 0.0, 3.0));

    // Left arm (left side of screen)
    model.fill_box(0, 0, 2, 2, 3, 3, shirt);      // Sleeve
    model.fill_box(0, 0, 1, 2, 2, 1, skin);       // Hand

    // Right arm (right side, holding weapon)
    model.fill_box(9, 0, 2, 11, 3, 3, shirt);     // Sleeve
    model.fill_box(9, 0, 1, 11, 2, 1, skin);      // Hand

    model
}

/// Create a detailed shotgun model
/// Size: 16x4x3 voxels
pub fn create_shotgun_model() -> VoxelModel {
    let mut model = VoxelModel::with_origin(16, 4, 3, Vec3::new(8.0, 2.0, 1.5));

    let metal = palette::GUN_METAL;
    let dark = palette::GUN_DARK;
    let grip = palette::GUN_GRIP;
    let accent = palette::GUN_ACCENT;

    // Main barrel
    model.fill_box(4, 2, 1, 15, 3, 1, metal);
    model.fill_box(4, 2, 0, 15, 3, 0, dark);
    model.fill_box(4, 2, 2, 15, 3, 2, dark);

    // Second barrel (double barrel shotgun)
    model.fill_box(8, 1, 1, 15, 1, 1, metal);

    // Receiver
    model.fill_box(2, 1, 0, 7, 3, 2, metal);
    model.fill_box(3, 2, 1, 6, 2, 1, dark);       // Ejection port

    // Pump grip
    model.fill_box(8, 0, 0, 11, 0, 2, grip);
    model.fill_box(8, 1, 0, 11, 1, 0, grip);
    model.fill_box(8, 1, 2, 11, 1, 2, grip);

    // Stock/pistol grip
    model.fill_box(0, 0, 0, 3, 2, 2, grip);
    model.fill_box(0, 0, 1, 1, 0, 1, dark);

    // Accents (sights)
    model.set_color(14, 3, 1, accent);            // Front sight
    model.set_color(5, 3, 1, accent);             // Rear sight

    model
}

/// Create assault rifle model
/// Size: 20x5x3 voxels
pub fn create_ar_model() -> VoxelModel {
    let mut model = VoxelModel::with_origin(20, 5, 3, Vec3::new(10.0, 2.5, 1.5));

    let metal = palette::GUN_METAL;
    let dark = palette::GUN_DARK;
    let grip = palette::GUN_GRIP;
    let accent = palette::GUN_ACCENT;

    // Barrel
    model.fill_box(12, 2, 1, 19, 3, 1, metal);
    model.fill_box(12, 2, 0, 17, 2, 0, dark);
    model.fill_box(12, 3, 0, 17, 3, 0, dark);

    // Receiver
    model.fill_box(5, 1, 0, 14, 4, 2, metal);

    // Magazine
    model.fill_box(7, 0, 0, 10, 1, 2, dark);
    model.set_color(8, 0, 1, accent);            // Ammo indicator

    // Stock
    model.fill_box(0, 2, 0, 5, 3, 2, grip);
    model.fill_box(0, 1, 1, 2, 2, 1, grip);

    // Pistol grip
    model.fill_box(5, 0, 0, 6, 1, 2, grip);

    // Foregrip
    model.fill_box(11, 1, 0, 13, 1, 2, grip);

    // Sights
    model.set_color(18, 4, 1, accent);
    model.set_color(10, 4, 1, accent);

    // Charging handle
    model.set_color(8, 4, 1, dark);

    model
}

/// Create pistol model
/// Size: 8x6x2 voxels
pub fn create_pistol_model() -> VoxelModel {
    let mut model = VoxelModel::with_origin(8, 6, 2, Vec3::new(4.0, 3.0, 1.0));

    let metal = palette::GUN_METAL;
    let dark = palette::GUN_DARK;
    let grip = palette::GUN_GRIP;

    // Slide
    model.fill_box(1, 3, 0, 7, 5, 1, metal);
    model.fill_box(2, 4, 0, 6, 4, 0, dark);      // Slide serrations

    // Frame
    model.fill_box(1, 1, 0, 5, 3, 1, dark);

    // Grip
    model.fill_box(1, 0, 0, 3, 2, 1, grip);

    // Trigger guard
    model.set_color(4, 1, 0, dark);
    model.set_color(4, 1, 1, dark);

    // Magazine
    model.set_color(2, 0, 0, metal);
    model.set_color(2, 0, 1, metal);

    model
}

/// Create SMG model
/// Size: 14x5x3 voxels
pub fn create_smg_model() -> VoxelModel {
    let mut model = VoxelModel::with_origin(14, 5, 3, Vec3::new(7.0, 2.5, 1.5));

    let metal = palette::GUN_METAL;
    let dark = palette::GUN_DARK;
    let grip = palette::GUN_GRIP;

    // Barrel
    model.fill_box(9, 2, 1, 13, 3, 1, metal);

    // Receiver
    model.fill_box(3, 1, 0, 10, 4, 2, metal);

    // Magazine (vertical)
    model.fill_box(5, 0, 0, 7, 1, 2, dark);

    // Stock (folded)
    model.fill_box(0, 2, 1, 3, 3, 1, metal);
    model.fill_box(0, 3, 1, 1, 4, 1, metal);

    // Pistol grip
    model.fill_box(3, 0, 0, 4, 1, 2, grip);

    model
}

/// Create sniper rifle model
/// Size: 24x5x3 voxels
pub fn create_sniper_model() -> VoxelModel {
    let mut model = VoxelModel::with_origin(24, 5, 3, Vec3::new(12.0, 2.5, 1.5));

    let metal = palette::GUN_METAL;
    let dark = palette::GUN_DARK;
    let grip = palette::GUN_GRIP;
    let accent = VoxelColor::from_hex(0x444444);

    // Long barrel
    model.fill_box(14, 2, 1, 23, 2, 1, metal);
    model.fill_box(14, 3, 1, 20, 3, 1, dark);

    // Receiver
    model.fill_box(8, 1, 0, 16, 4, 2, metal);

    // Scope
    model.fill_box(10, 4, 0, 15, 4, 2, dark);
    model.set_color(10, 4, 1, accent);           // Front lens
    model.set_color(15, 4, 1, accent);           // Rear lens

    // Magazine
    model.fill_box(10, 0, 0, 12, 1, 2, dark);

    // Stock
    model.fill_box(0, 1, 0, 8, 3, 2, grip);
    model.fill_box(0, 3, 1, 4, 4, 1, grip);      // Cheek rest

    // Pistol grip
    model.fill_box(6, 0, 0, 8, 1, 2, grip);

    // Bipod (folded)
    model.set_color(16, 1, 0, metal);
    model.set_color(16, 1, 2, metal);

    model
}

/// Create pickaxe model
/// Size: 12x16x3 voxels
pub fn create_pickaxe_model() -> VoxelModel {
    let mut model = VoxelModel::with_origin(12, 16, 3, Vec3::new(6.0, 0.0, 1.5));

    let wood = palette::WOOD_MEDIUM;
    let metal = palette::METAL_GRAY;
    let metal_dark = palette::METAL_DARK;

    // Handle
    model.fill_box(5, 0, 1, 6, 11, 1, wood);

    // Head - horizontal part
    model.fill_box(1, 12, 1, 10, 13, 1, metal);
    model.fill_box(0, 12, 1, 0, 12, 1, metal_dark);  // Point
    model.fill_box(11, 12, 1, 11, 12, 1, metal_dark); // Point

    // Head - top part
    model.fill_box(4, 14, 1, 7, 15, 1, metal);
    model.fill_box(5, 14, 0, 6, 14, 2, metal);

    model
}

/// Create a detailed backpack
/// Size: 6x8x4 voxels
pub fn create_backpack_model(style: u8) -> VoxelModel {
    let color = match style {
        0 => return VoxelModel::new(1, 1, 1), // No backpack
        1 => palette::BACKPACK_GREEN,
        2 => palette::BACKPACK_TAN,
        _ => palette::PANTS_BLACK,
    };

    let mut model = VoxelModel::with_origin(6, 8, 4, Vec3::new(3.0, 4.0, 0.0));

    let size = match style {
        1 => (4, 5, 2), // Small
        2 => (5, 7, 3), // Medium
        _ => (6, 8, 4), // Large
    };

    let ox = (6 - size.0) / 2;
    let oy = (8 - size.1) / 2;
    let oz = 0;

    // Main body
    model.fill_box(ox, oy, oz, ox + size.0 - 1, oy + size.1 - 1, oz + size.2 - 1, color);

    // Straps
    let strap = VoxelColor::from_hex(0x333333);
    if size.1 > 4 {
        model.set_color(ox, oy + 1, oz + size.2 - 1, strap);
        model.set_color(ox + size.0 - 1, oy + 1, oz + size.2 - 1, strap);
        model.set_color(ox, oy + size.1 - 2, oz + size.2 - 1, strap);
        model.set_color(ox + size.0 - 1, oy + size.1 - 2, oz + size.2 - 1, strap);
    }

    // Pocket
    let pocket = VoxelColor::from_hex(0x2A2A2A);
    if size.0 > 3 && size.1 > 3 {
        model.fill_box(ox + 1, oy + 1, oz, ox + size.0 - 2, oy + 2, oz, pocket);
    }

    model
}

/// Create a glider model
/// Size: 24x8x16 voxels
pub fn create_glider_model(style: u8) -> VoxelModel {
    let mut model = VoxelModel::with_origin(24, 8, 16, Vec3::new(12.0, 0.0, 8.0));

    let main_color = match style {
        0 => palette::GLIDER_RED,
        1 => palette::GLIDER_BLUE,
        2 => VoxelColor::from_hex(0x44AA44), // Green
        _ => VoxelColor::from_hex(0xFFAA00), // Orange
    };

    let accent = VoxelColor::from_hex(0x222222);
    let string = VoxelColor::from_hex(0x888888);

    // Canopy - main dome shape
    // Bottom layer (widest)
    model.fill_box(2, 6, 2, 21, 6, 13, main_color);
    // Second layer
    model.fill_box(4, 7, 4, 19, 7, 11, main_color);

    // Accent stripes
    model.fill_box(11, 6, 2, 12, 7, 13, accent);
    model.fill_box(2, 6, 7, 21, 7, 8, accent);

    // Support strings (4 corners to center)
    // Front left
    model.set_color(4, 5, 4, string);
    model.set_color(6, 4, 5, string);
    model.set_color(8, 3, 6, string);
    model.set_color(10, 2, 7, string);
    model.set_color(11, 1, 7, string);

    // Front right
    model.set_color(19, 5, 4, string);
    model.set_color(17, 4, 5, string);
    model.set_color(15, 3, 6, string);
    model.set_color(13, 2, 7, string);
    model.set_color(12, 1, 7, string);

    // Back left
    model.set_color(4, 5, 11, string);
    model.set_color(6, 4, 10, string);
    model.set_color(8, 3, 9, string);
    model.set_color(10, 2, 8, string);

    // Back right
    model.set_color(19, 5, 11, string);
    model.set_color(17, 4, 10, string);
    model.set_color(15, 3, 9, string);
    model.set_color(13, 2, 8, string);

    // Harness connection point
    model.fill_box(10, 0, 7, 13, 0, 8, accent);

    model
}

/// Create a tree (pine style)
/// Size: 10x20x10 voxels
pub fn create_pine_tree() -> VoxelModel {
    let mut model = VoxelModel::with_origin(10, 20, 10, Vec3::new(5.0, 0.0, 5.0));

    let trunk = palette::WOOD_DARK;
    let leaves = palette::LEAF_GREEN;
    let leaves_dark = palette::GRASS_DARK;

    // Trunk
    model.fill_box(4, 0, 4, 5, 8, 5, trunk);

    // Foliage layers (cone shape)
    // Bottom layer
    model.fill_box(1, 6, 1, 8, 8, 8, leaves);
    // Second layer
    model.fill_box(2, 9, 2, 7, 12, 7, leaves);
    // Third layer
    model.fill_box(3, 13, 3, 6, 16, 6, leaves);
    // Top
    model.fill_box(4, 17, 4, 5, 19, 5, leaves);

    // Add some darker patches for depth
    model.set_color(2, 7, 2, leaves_dark);
    model.set_color(7, 7, 7, leaves_dark);
    model.set_color(3, 10, 3, leaves_dark);
    model.set_color(6, 10, 6, leaves_dark);
    model.set_color(4, 14, 4, leaves_dark);

    model
}

/// Create a tree (oak style)
/// Size: 12x16x12 voxels
pub fn create_oak_tree() -> VoxelModel {
    let mut model = VoxelModel::with_origin(12, 16, 12, Vec3::new(6.0, 0.0, 6.0));

    let trunk = palette::WOOD_DARK;
    let leaves = palette::LEAF_GREEN;

    // Trunk
    model.fill_box(5, 0, 5, 6, 7, 6, trunk);

    // Branches
    model.fill_box(4, 6, 5, 4, 7, 6, trunk);
    model.fill_box(7, 6, 5, 7, 7, 6, trunk);
    model.fill_box(5, 6, 4, 6, 7, 4, trunk);
    model.fill_box(5, 6, 7, 6, 7, 7, trunk);

    // Foliage (roughly spherical)
    model.fill_box(2, 8, 2, 9, 13, 9, leaves);
    model.fill_box(1, 9, 3, 10, 12, 8, leaves);
    model.fill_box(3, 9, 1, 8, 12, 10, leaves);
    model.fill_box(3, 14, 3, 8, 15, 8, leaves);

    model
}

/// Create a rock
/// Size: 6x4x5 voxels (irregular shape)
pub fn create_rock(seed: u32) -> VoxelModel {
    let mut model = VoxelModel::with_origin(6, 4, 5, Vec3::new(3.0, 0.0, 2.5));

    let gray = palette::STONE_GRAY;
    let dark = palette::STONE_DARK;

    // Base irregular shape
    model.fill_box(1, 0, 1, 4, 2, 3, gray);
    model.fill_box(0, 0, 1, 0, 1, 2, gray);
    model.fill_box(5, 0, 2, 5, 1, 3, gray);
    model.fill_box(2, 3, 2, 3, 3, 2, gray);

    // Add some variation based on seed
    if seed % 2 == 0 {
        model.set_color(1, 1, 1, dark);
        model.set_color(4, 2, 3, dark);
    } else {
        model.set_color(3, 2, 2, dark);
        model.set_color(0, 0, 2, dark);
    }

    model
}

/// Create a wooden wall building piece
/// Size: 16x16x2 voxels
pub fn create_wall_wood() -> VoxelModel {
    let mut model = VoxelModel::with_origin(16, 16, 2, Vec3::new(8.0, 0.0, 1.0));

    let plank = palette::WOOD_PLANK;
    let dark = palette::WOOD_DARK;
    let light = palette::WOOD_LIGHT;

    // Main planks (horizontal)
    for y in 0..16 {
        let color = if y % 4 < 2 { plank } else { light };
        model.fill_box(0, y, 0, 15, y, 1, color);
    }

    // Vertical supports
    model.fill_box(0, 0, 0, 0, 15, 1, dark);
    model.fill_box(15, 0, 0, 15, 15, 1, dark);
    model.fill_box(7, 0, 0, 8, 15, 1, dark);

    // Horizontal supports
    model.fill_box(0, 0, 0, 15, 0, 1, dark);
    model.fill_box(0, 15, 0, 15, 15, 1, dark);
    model.fill_box(0, 7, 0, 15, 8, 1, dark);

    model
}

/// Create a brick wall building piece
/// Size: 16x16x2 voxels
pub fn create_wall_brick() -> VoxelModel {
    let mut model = VoxelModel::with_origin(16, 16, 2, Vec3::new(8.0, 0.0, 1.0));

    let brick = palette::BRICK_RED;
    let mortar = VoxelColor::from_hex(0xAAAAAA);

    // Fill with bricks
    for y in 0..16 {
        for x in 0..16 {
            let is_mortar = y % 3 == 2 || (x + (y / 3) * 2) % 4 == 3;
            let color = if is_mortar { mortar } else { brick };
            model.fill_box(x, y, 0, x, y, 1, color);
        }
    }

    model
}

/// Create a metal wall building piece
/// Size: 16x16x2 voxels
pub fn create_wall_metal() -> VoxelModel {
    let mut model = VoxelModel::with_origin(16, 16, 2, Vec3::new(8.0, 0.0, 1.0));

    let metal = palette::METAL_GRAY;
    let dark = palette::METAL_DARK;
    let rivet = VoxelColor::from_hex(0x505050);

    // Main panels
    model.fill_box(0, 0, 0, 15, 15, 1, metal);

    // Panel divisions
    model.fill_box(7, 0, 0, 8, 15, 0, dark);
    model.fill_box(0, 7, 0, 15, 8, 0, dark);

    // Rivets
    for x in [1, 6, 9, 14].iter() {
        for y in [1, 6, 9, 14].iter() {
            model.set_color(*x, *y, 0, rivet);
        }
    }

    model
}

/// Create a floor/platform piece
/// Size: 16x2x16 voxels
pub fn create_floor_wood() -> VoxelModel {
    let mut model = VoxelModel::with_origin(16, 2, 16, Vec3::new(8.0, 0.0, 8.0));

    let plank = palette::WOOD_PLANK;
    let dark = palette::WOOD_DARK;

    // Top surface with planks
    for z in 0..16 {
        let color = if z % 3 == 2 { dark } else { plank };
        model.fill_box(0, 1, z, 15, 1, z, color);
    }

    // Support beams underneath
    model.fill_box(0, 0, 0, 15, 0, 0, dark);
    model.fill_box(0, 0, 7, 15, 0, 8, dark);
    model.fill_box(0, 0, 15, 15, 0, 15, dark);
    model.fill_box(0, 0, 0, 0, 0, 15, dark);
    model.fill_box(7, 0, 0, 8, 0, 15, dark);
    model.fill_box(15, 0, 0, 15, 0, 15, dark);

    model
}

/// Create a ramp/stairs piece
/// Size: 16x16x16 voxels
pub fn create_ramp_wood() -> VoxelModel {
    let mut model = VoxelModel::with_origin(16, 16, 16, Vec3::new(8.0, 0.0, 8.0));

    let plank = palette::WOOD_PLANK;
    let dark = palette::WOOD_DARK;

    // Create stepped ramp
    for z in 0..16 {
        let height = z;
        let color = if z % 3 == 2 { dark } else { plank };
        model.fill_box(0, 0, z, 15, height, z, color);
    }

    // Side rails
    for z in 0..16 {
        let height = z + 2;
        model.set_color(0, height.min(15), z, dark);
        model.set_color(15, height.min(15), z, dark);
    }

    model
}

/// Create the battle bus
/// Size: 20x16x32 voxels
pub fn create_battle_bus() -> VoxelModel {
    let mut model = VoxelModel::with_origin(20, 16, 32, Vec3::new(10.0, 0.0, 16.0));

    let body_blue = VoxelColor::from_hex(0x2266AA);
    let body_light = VoxelColor::from_hex(0x3388CC);
    let window = VoxelColor::from_hex(0x88CCFF);
    let wheel = VoxelColor::from_hex(0x222222);
    let balloon_red = VoxelColor::from_hex(0xCC3333);
    let balloon_stripe = VoxelColor::from_hex(0xEEEEEE);
    let rope = VoxelColor::from_hex(0x886644);

    // === BUS BODY ===
    // Main body
    model.fill_box(2, 0, 4, 17, 8, 27, body_blue);
    // Roof
    model.fill_box(3, 9, 5, 16, 9, 26, body_light);
    // Front
    model.fill_box(4, 1, 1, 15, 7, 3, body_blue);
    // Windshield
    model.fill_box(5, 4, 1, 14, 7, 1, window);
    // Side windows
    for z in [6, 10, 14, 18, 22].iter() {
        model.fill_box(2, 4, *z, 2, 7, *z + 2, window);
        model.fill_box(17, 4, *z, 17, 7, *z + 2, window);
    }

    // Wheels
    model.fill_box(3, 0, 5, 4, 1, 7, wheel);
    model.fill_box(15, 0, 5, 16, 1, 7, wheel);
    model.fill_box(3, 0, 23, 4, 1, 25, wheel);
    model.fill_box(15, 0, 23, 16, 1, 25, wheel);

    // === BALLOON ===
    // Main balloon body (stretched sphere)
    model.fill_box(4, 12, 8, 15, 15, 23, balloon_red);
    model.fill_box(3, 13, 10, 16, 14, 21, balloon_red);
    model.fill_box(5, 11, 10, 14, 11, 21, balloon_red);

    // White stripes
    model.fill_box(9, 12, 8, 10, 15, 23, balloon_stripe);
    model.fill_box(4, 12, 15, 15, 15, 16, balloon_stripe);

    // Ropes connecting balloon to bus
    model.fill_box(9, 10, 10, 10, 11, 10, rope);
    model.fill_box(9, 10, 21, 10, 11, 21, rope);

    model
}

/// Create a loot chest
/// Size: 6x5x4 voxels
pub fn create_chest() -> VoxelModel {
    let mut model = VoxelModel::with_origin(6, 5, 4, Vec3::new(3.0, 0.0, 2.0));

    let wood = palette::WOOD_MEDIUM;
    let metal = palette::METAL_GRAY;
    let gold = VoxelColor::from_hex(0xFFD700);

    // Main body
    model.fill_box(0, 0, 0, 5, 3, 3, wood);
    // Lid
    model.fill_box(0, 4, 0, 5, 4, 3, wood);

    // Metal bands
    model.fill_box(0, 0, 0, 0, 4, 3, metal);
    model.fill_box(5, 0, 0, 5, 4, 3, metal);
    model.fill_box(0, 2, 0, 5, 2, 0, metal);

    // Lock
    model.set_color(2, 2, 0, gold);
    model.set_color(3, 2, 0, gold);

    model
}
