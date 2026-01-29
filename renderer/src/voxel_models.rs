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

/// Create a detailed pump-action shotgun model
/// Size: 32x8x6 voxels (double resolution for detail)
pub fn create_shotgun_model() -> VoxelModel {
    let mut model = VoxelModel::with_origin(32, 8, 6, Vec3::new(16.0, 4.0, 3.0));

    let metal = palette::GUN_METAL;
    let dark = palette::GUN_DARK;
    let grip = palette::GUN_GRIP;
    let accent = palette::GUN_ACCENT;
    let chrome = palette::CHROME_DARK;

    // === BARREL (with ventilated rib) ===
    // Main barrel
    model.fill_box(10, 4, 2, 31, 5, 3, metal);
    model.fill_box(10, 4, 1, 31, 4, 1, dark);
    model.fill_box(10, 5, 1, 31, 5, 1, dark);
    model.fill_box(10, 4, 4, 31, 4, 4, dark);
    model.fill_box(10, 5, 4, 31, 5, 4, dark);

    // Ventilated rib (top of barrel with gaps)
    for x in (12..30).step_by(2) {
        model.set_color(x, 6, 2, metal);
        model.set_color(x, 6, 3, metal);
    }

    // Muzzle
    model.fill_box(30, 4, 2, 31, 5, 3, dark);

    // Bead sight (front)
    model.set_color(30, 6, 2, accent);
    model.set_color(30, 6, 3, accent);

    // === MAGAZINE TUBE (under barrel) ===
    model.fill_box(10, 2, 2, 26, 3, 3, metal);
    model.fill_box(26, 2, 2, 27, 3, 3, dark); // Magazine cap

    // === RECEIVER ===
    model.fill_box(4, 2, 1, 14, 5, 4, metal);
    model.fill_box(5, 3, 2, 13, 4, 3, dark); // Receiver top

    // Ejection port
    model.fill_box(8, 4, 4, 12, 5, 4, dark);

    // Shell port (loading gate)
    model.fill_box(6, 2, 1, 10, 3, 1, dark);

    // Trigger guard
    model.fill_box(6, 1, 2, 10, 1, 3, metal);
    model.set_color(8, 1, 2, dark); // Trigger
    model.set_color(8, 1, 3, dark);

    // === PUMP/FORE-END (grooved grip) ===
    model.fill_box(14, 1, 1, 22, 3, 4, grip);
    // Grooves
    for x in [15, 17, 19, 21].iter() {
        model.fill_box(*x, 1, 1, *x, 3, 1, dark);
        model.fill_box(*x, 1, 4, *x, 3, 4, dark);
    }

    // === STOCK ===
    model.fill_box(0, 1, 1, 5, 4, 4, grip);
    model.fill_box(0, 0, 2, 3, 0, 3, grip); // Stock toe

    // Buttpad
    model.fill_box(0, 1, 1, 0, 4, 4, dark);

    // Pistol grip area
    model.fill_box(4, 0, 1, 6, 1, 4, grip);

    // === SIGHTS ===
    // Rear sight (ghost ring)
    model.set_color(8, 6, 2, chrome);
    model.set_color(8, 6, 3, chrome);

    model
}

/// Create detailed assault rifle model (M4/AR-15 style)
/// Size: 40x10x6 voxels (double resolution for detail)
pub fn create_ar_model() -> VoxelModel {
    let mut model = VoxelModel::with_origin(40, 10, 6, Vec3::new(20.0, 5.0, 3.0));

    let metal = palette::GUN_METAL;
    let dark = palette::GUN_DARK;
    let grip = palette::GUN_GRIP;
    let accent = palette::GUN_ACCENT;
    let chrome = palette::CHROME_DARK;
    let rail = VoxelColor::from_hex(0x3A3A3A);

    // === BARREL ===
    model.fill_box(26, 4, 2, 39, 5, 3, metal);
    model.fill_box(26, 4, 1, 35, 4, 1, dark);
    model.fill_box(26, 5, 1, 35, 5, 1, dark);
    model.fill_box(26, 4, 4, 35, 4, 4, dark);
    model.fill_box(26, 5, 4, 35, 5, 4, dark);

    // Muzzle brake
    model.fill_box(37, 4, 2, 39, 5, 3, dark);
    model.fill_box(38, 4, 1, 39, 5, 1, metal);
    model.fill_box(38, 4, 4, 39, 5, 4, metal);

    // Gas block
    model.fill_box(30, 6, 2, 32, 6, 3, metal);

    // === HANDGUARD (quad rail) ===
    model.fill_box(20, 3, 1, 29, 6, 4, dark);

    // Picatinny rails (top, bottom, sides)
    // Top rail
    for x in (20..29).step_by(2) {
        model.set_color(x, 7, 2, rail);
        model.set_color(x, 7, 3, rail);
    }
    // Side rails
    for x in (21..28).step_by(2) {
        model.set_color(x, 4, 0, rail);
        model.set_color(x, 5, 0, rail);
        model.set_color(x, 4, 5, rail);
        model.set_color(x, 5, 5, rail);
    }
    // Bottom rail
    for x in (21..28).step_by(2) {
        model.set_color(x, 2, 2, rail);
        model.set_color(x, 2, 3, rail);
    }

    // === UPPER RECEIVER ===
    model.fill_box(10, 4, 1, 22, 7, 4, metal);

    // Carry handle / optic rail
    model.fill_box(12, 8, 2, 20, 8, 3, metal);
    // Rail slots
    for x in (13..19).step_by(2) {
        model.set_color(x, 9, 2, rail);
        model.set_color(x, 9, 3, rail);
    }

    // Forward assist
    model.set_color(17, 6, 4, chrome);
    model.set_color(18, 6, 4, chrome);

    // Ejection port cover
    model.fill_box(14, 5, 4, 18, 6, 4, dark);

    // Charging handle
    model.fill_box(10, 7, 2, 12, 7, 3, dark);
    model.set_color(10, 8, 2, dark);
    model.set_color(10, 8, 3, dark);

    // === LOWER RECEIVER ===
    model.fill_box(10, 2, 1, 18, 4, 4, metal);

    // Trigger guard
    model.fill_box(12, 1, 1, 16, 1, 4, metal);
    model.set_color(14, 1, 2, dark); // Trigger
    model.set_color(14, 1, 3, dark);

    // Magazine well
    model.fill_box(13, 1, 1, 17, 2, 4, dark);

    // === MAGAZINE (curved) ===
    model.fill_box(13, 0, 1, 17, 1, 4, dark);
    // Ammo indicator window
    model.set_color(14, 0, 1, accent);
    model.set_color(15, 0, 1, accent);

    // === PISTOL GRIP ===
    model.fill_box(10, 0, 1, 12, 2, 4, grip);

    // === BUFFER TUBE ===
    model.fill_box(2, 4, 2, 10, 5, 3, dark);

    // === STOCK (collapsible) ===
    model.fill_box(0, 3, 1, 6, 6, 4, grip);
    model.fill_box(0, 2, 2, 2, 2, 3, grip); // Stock toe

    // Buttpad
    model.fill_box(0, 3, 1, 0, 6, 4, dark);

    // Adjustment lever
    model.set_color(4, 4, 0, chrome);
    model.set_color(4, 5, 0, chrome);

    // === SIGHTS ===
    // Front sight post
    model.set_color(34, 7, 2, accent);
    model.set_color(34, 7, 3, accent);

    // Rear sight (flip-up)
    model.set_color(15, 9, 2, chrome);
    model.set_color(15, 9, 3, chrome);

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

/// Create detailed bolt-action sniper rifle model
/// Size: 48x10x6 voxels (double resolution for detail)
pub fn create_sniper_model() -> VoxelModel {
    let mut model = VoxelModel::with_origin(48, 10, 6, Vec3::new(24.0, 5.0, 3.0));

    let metal = palette::GUN_METAL;
    let dark = palette::GUN_DARK;
    let grip = palette::GUN_GRIP;
    let chrome = palette::CHROME_DARK;
    let lens = VoxelColor::from_hex(0x4488CC); // Blue-tinted lens
    let lens_rim = VoxelColor::from_hex(0x222222);

    // === LONG BARREL (heavy profile) ===
    model.fill_box(30, 4, 2, 47, 5, 3, metal);
    model.fill_box(30, 4, 1, 42, 4, 1, dark);
    model.fill_box(30, 5, 1, 42, 5, 1, dark);
    model.fill_box(30, 4, 4, 42, 4, 4, dark);
    model.fill_box(30, 5, 4, 42, 5, 4, dark);

    // Muzzle brake (threaded)
    model.fill_box(44, 4, 2, 47, 5, 3, dark);
    model.fill_box(45, 3, 2, 46, 3, 3, metal);
    model.fill_box(45, 6, 2, 46, 6, 3, metal);

    // Barrel fluting (weight reduction grooves)
    for x in (32..42).step_by(3) {
        model.set_color(x, 4, 1, metal);
        model.set_color(x, 5, 1, metal);
        model.set_color(x, 4, 4, metal);
        model.set_color(x, 5, 4, metal);
    }

    // === RECEIVER (long action) ===
    model.fill_box(16, 3, 1, 32, 6, 4, metal);

    // === LARGE SCOPE (with detailed lenses) ===
    // Scope body (tube)
    model.fill_box(18, 7, 1, 30, 9, 4, dark);
    model.fill_box(17, 7, 2, 17, 9, 3, dark); // Eyepiece bell
    model.fill_box(31, 7, 2, 32, 9, 3, dark); // Objective bell

    // Scope rings (mounts)
    model.fill_box(20, 6, 1, 22, 7, 4, chrome);
    model.fill_box(27, 6, 1, 29, 7, 4, chrome);

    // Front lens (objective)
    model.fill_box(32, 7, 2, 32, 9, 3, lens_rim);
    model.set_color(32, 8, 2, lens);
    model.set_color(32, 8, 3, lens);

    // Rear lens (eyepiece)
    model.fill_box(17, 7, 2, 17, 9, 3, lens_rim);
    model.set_color(17, 8, 2, lens);
    model.set_color(17, 8, 3, lens);

    // Turrets (windage/elevation)
    model.fill_box(24, 9, 2, 25, 10, 3, chrome);
    model.fill_box(24, 8, 4, 25, 9, 5, chrome);

    // === BOLT HANDLE ===
    model.fill_box(22, 5, 4, 24, 5, 5, chrome);
    model.fill_box(24, 5, 5, 25, 6, 6, chrome); // Bolt knob

    // Ejection port
    model.fill_box(20, 5, 4, 24, 6, 4, dark);

    // === MAGAZINE (detachable box) ===
    model.fill_box(20, 1, 1, 26, 3, 4, dark);
    model.fill_box(21, 0, 2, 25, 0, 3, dark);

    // Magazine release
    model.set_color(19, 2, 2, chrome);

    // === TRIGGER GUARD & TRIGGER ===
    model.fill_box(16, 1, 1, 20, 1, 4, metal);
    model.set_color(18, 1, 2, chrome); // Trigger
    model.set_color(18, 1, 3, chrome);

    // === PISTOL GRIP (ergonomic) ===
    model.fill_box(14, 0, 1, 18, 3, 4, grip);

    // Grip texture
    for y in 0..3 {
        model.set_color(14, y, 1, dark);
        model.set_color(14, y, 4, dark);
    }

    // === STOCK (adjustable thumbhole) ===
    model.fill_box(0, 2, 1, 14, 5, 4, grip);
    model.fill_box(0, 1, 2, 8, 1, 3, grip); // Stock toe

    // Cheek rest (adjustable)
    model.fill_box(4, 6, 1, 10, 7, 4, grip);
    model.fill_box(6, 7, 2, 8, 7, 3, dark); // Adjustment mechanism

    // Thumbhole
    model.fill_box(10, 2, 2, 12, 4, 3, dark);

    // Buttpad (rubber)
    model.fill_box(0, 2, 1, 0, 5, 4, dark);

    // === BIPOD (deployed) ===
    // Left leg
    model.fill_box(32, 0, 0, 33, 3, 0, metal);
    model.fill_box(33, 0, 0, 34, 0, 0, dark); // Foot
    // Right leg
    model.fill_box(32, 0, 5, 33, 3, 5, metal);
    model.fill_box(33, 0, 5, 34, 0, 5, dark); // Foot
    // Bipod mount
    model.fill_box(32, 3, 1, 34, 4, 4, chrome);

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

/// Create the battle bus with detailed features
/// Size: 40x32x64 voxels (double resolution for detail)
pub fn create_battle_bus() -> VoxelModel {
    let mut model = VoxelModel::with_origin(40, 32, 64, Vec3::new(20.0, 0.0, 32.0));

    // Colors
    let body_blue = palette::BUS_BLUE;
    let body_light = palette::BUS_LIGHT_BLUE;
    let body_dark = VoxelColor::from_hex(0x1A4488);
    let window = palette::GLASS;
    let window_frame = palette::CHROME_DARK;
    let wheel = palette::RUBBER;
    let wheel_hub = palette::CHROME;
    let bumper = palette::CHROME;
    let headlight = palette::HEADLIGHT;
    let taillight = palette::TAILLIGHT;
    let grille = VoxelColor::from_hex(0x333333);
    let door_handle = palette::CHROME;
    let exhaust = palette::METAL_DARK;
    let mirror = palette::CHROME_DARK;

    // === BUS BODY (main structure) ===
    // Main body (rounded corners implied by layered fill)
    model.fill_box(4, 2, 8, 35, 16, 55, body_blue);

    // Roof (slightly lighter)
    model.fill_box(5, 17, 9, 34, 18, 54, body_light);
    model.fill_box(6, 19, 10, 33, 19, 53, body_light);

    // Lower body trim (darker accent)
    model.fill_box(4, 2, 8, 35, 3, 55, body_dark);

    // === FRONT SECTION ===
    // Front panel
    model.fill_box(8, 3, 2, 31, 14, 7, body_blue);
    model.fill_box(8, 3, 1, 31, 12, 1, body_blue);

    // Grille (4-slot design)
    model.fill_box(12, 4, 1, 27, 8, 1, grille);
    model.fill_box(14, 5, 0, 16, 7, 0, grille);
    model.fill_box(18, 5, 0, 20, 7, 0, grille);
    model.fill_box(21, 5, 0, 23, 7, 0, grille);
    model.fill_box(25, 5, 0, 27, 7, 0, grille);

    // Headlights (dual on each side)
    model.fill_box(9, 6, 0, 11, 8, 1, headlight);
    model.fill_box(9, 4, 0, 11, 5, 1, headlight);
    model.fill_box(28, 6, 0, 30, 8, 1, headlight);
    model.fill_box(28, 4, 0, 30, 5, 1, headlight);

    // Front bumper
    model.fill_box(6, 1, 0, 33, 2, 2, bumper);
    model.fill_box(8, 0, 1, 31, 0, 1, bumper);

    // Windshield with frame
    model.fill_box(10, 10, 2, 29, 16, 2, window);
    model.fill_box(9, 10, 2, 9, 16, 2, window_frame);
    model.fill_box(30, 10, 2, 30, 16, 2, window_frame);
    model.fill_box(10, 9, 2, 29, 9, 2, window_frame);
    model.fill_box(10, 17, 2, 29, 17, 2, window_frame);

    // === SIDE WINDOWS (8 per side with frames) ===
    for i in 0..8 {
        let z = 12 + i * 5;
        // Left side windows
        model.fill_box(4, 8, z, 4, 14, z + 3, window);
        model.fill_box(4, 7, z, 4, 7, z + 3, window_frame);
        model.fill_box(4, 15, z, 4, 15, z + 3, window_frame);
        model.fill_box(4, 8, z - 1, 4, 14, z - 1, window_frame);
        model.fill_box(4, 8, z + 4, 4, 14, z + 4, window_frame);

        // Right side windows
        model.fill_box(35, 8, z, 35, 14, z + 3, window);
        model.fill_box(35, 7, z, 35, 7, z + 3, window_frame);
        model.fill_box(35, 15, z, 35, 15, z + 3, window_frame);
        model.fill_box(35, 8, z - 1, 35, 14, z - 1, window_frame);
        model.fill_box(35, 8, z + 4, 35, 14, z + 4, window_frame);
    }

    // === DOOR OUTLINES AND HANDLES ===
    // Front door (left side)
    model.fill_box(3, 3, 14, 3, 15, 14, body_dark);
    model.fill_box(3, 3, 22, 3, 15, 22, body_dark);
    model.set_color(3, 9, 20, door_handle);
    model.set_color(3, 9, 21, door_handle);

    // Rear door (left side)
    model.fill_box(3, 3, 36, 3, 15, 36, body_dark);
    model.fill_box(3, 3, 44, 3, 15, 44, body_dark);
    model.set_color(3, 9, 42, door_handle);
    model.set_color(3, 9, 43, door_handle);

    // === SIDE MIRRORS ===
    model.fill_box(2, 12, 6, 2, 14, 8, mirror);
    model.fill_box(37, 12, 6, 37, 14, 8, mirror);

    // === WHEELS (with hub caps and tread detail) ===
    // Front wheels
    for wx in [6, 7, 32, 33].iter() {
        for wz in [10, 11, 12].iter() {
            model.fill_box(*wx, 0, *wz, *wx, 3, *wz, wheel);
        }
    }
    model.set_color(6, 1, 11, wheel_hub);
    model.set_color(33, 1, 11, wheel_hub);

    // Rear wheels
    for wx in [6, 7, 32, 33].iter() {
        for wz in [46, 47, 48].iter() {
            model.fill_box(*wx, 0, *wz, *wx, 3, *wz, wheel);
        }
    }
    model.set_color(6, 1, 47, wheel_hub);
    model.set_color(33, 1, 47, wheel_hub);

    // Wheel wells (darker recesses)
    model.fill_box(5, 2, 9, 8, 4, 13, body_dark);
    model.fill_box(31, 2, 9, 34, 4, 13, body_dark);
    model.fill_box(5, 2, 45, 8, 4, 49, body_dark);
    model.fill_box(31, 2, 45, 34, 4, 49, body_dark);

    // === REAR SECTION ===
    // Rear panel
    model.fill_box(8, 3, 56, 31, 14, 57, body_blue);

    // Rear window
    model.fill_box(12, 8, 57, 27, 14, 57, window);
    model.fill_box(11, 8, 57, 11, 14, 57, window_frame);
    model.fill_box(28, 8, 57, 28, 14, 57, window_frame);

    // Taillights
    model.fill_box(8, 5, 57, 10, 8, 58, taillight);
    model.fill_box(29, 5, 57, 31, 8, 58, taillight);

    // Rear bumper
    model.fill_box(6, 1, 56, 33, 2, 58, bumper);

    // Exhaust pipe
    model.fill_box(28, 1, 58, 30, 2, 60, exhaust);

    // === BALLOON (integrated, more detailed) ===
    let balloon = create_balloon();
    model.merge(&balloon, 4, 20, 16);

    model
}

/// Create a detailed hot air balloon for the battle bus
/// Size: 32x24x32 voxels (separate model for clarity)
pub fn create_balloon() -> VoxelModel {
    let mut model = VoxelModel::with_origin(32, 24, 32, Vec3::new(16.0, 0.0, 16.0));

    let red = palette::BALLOON_RED;
    let white = palette::BALLOON_WHITE;
    let rope = palette::ROPE_BROWN;
    let vent = VoxelColor::from_hex(0x444444);

    // === MAIN BALLOON ENVELOPE (elongated oval shape) ===
    // Bottom layer (attachment point)
    model.fill_box(12, 0, 12, 19, 1, 19, red);

    // Lower section (widening)
    model.fill_box(10, 2, 10, 21, 4, 21, red);
    model.fill_box(8, 5, 8, 23, 7, 23, red);

    // Middle section (widest)
    model.fill_box(6, 8, 6, 25, 12, 25, red);
    model.fill_box(5, 10, 7, 26, 11, 24, red);
    model.fill_box(7, 10, 5, 24, 11, 26, red);

    // Upper section (narrowing)
    model.fill_box(8, 13, 8, 23, 16, 23, red);
    model.fill_box(10, 17, 10, 21, 20, 21, red);

    // Top section (dome)
    model.fill_box(12, 21, 12, 19, 22, 19, red);
    model.fill_box(14, 23, 14, 17, 23, 17, red);

    // === 8 VERTICAL STRIPES (alternating red/white) ===
    // Stripe 1 (front)
    for y in 2..23 {
        model.fill_box(15, y, 5, 16, y, 7, white);
    }
    // Stripe 2 (back)
    for y in 2..23 {
        model.fill_box(15, y, 24, 16, y, 26, white);
    }
    // Stripe 3 (left)
    for y in 2..23 {
        model.fill_box(5, y, 15, 7, y, 16, white);
    }
    // Stripe 4 (right)
    for y in 2..23 {
        model.fill_box(24, y, 15, 26, y, 16, white);
    }
    // Diagonal stripes (front-left, front-right, back-left, back-right)
    for y in 5..20 {
        let offset = (y - 5) / 3;
        if 8 + offset < 24 && 8 + offset < 24 {
            model.set_color(8 + offset, y, 8 + offset, white);
            model.set_color(23 - offset, y, 8 + offset, white);
            model.set_color(8 + offset, y, 23 - offset, white);
            model.set_color(23 - offset, y, 23 - offset, white);
        }
    }

    // === VENT AT TOP ===
    model.fill_box(14, 23, 14, 17, 23, 17, vent);

    // === SUSPENSION ROPES (12 ropes to corners) ===
    // Front-left ropes
    for i in 0..6 {
        model.set_color(10 - i, i, 10 - i, rope);
    }
    for i in 0..6 {
        model.set_color(12 - i, i, 10 - i, rope);
    }
    for i in 0..6 {
        model.set_color(10 - i, i, 12 - i, rope);
    }

    // Front-right ropes
    for i in 0..6 {
        model.set_color(21 + i, i, 10 - i, rope);
    }
    for i in 0..6 {
        model.set_color(19 + i, i, 10 - i, rope);
    }
    for i in 0..6 {
        model.set_color(21 + i, i, 12 - i, rope);
    }

    // Back-left ropes
    for i in 0..6 {
        model.set_color(10 - i, i, 21 + i, rope);
    }
    for i in 0..6 {
        model.set_color(12 - i, i, 21 + i, rope);
    }
    for i in 0..6 {
        model.set_color(10 - i, i, 19 + i, rope);
    }

    // Back-right ropes
    for i in 0..6 {
        model.set_color(21 + i, i, 21 + i, rope);
    }
    for i in 0..6 {
        model.set_color(19 + i, i, 21 + i, rope);
    }
    for i in 0..6 {
        model.set_color(21 + i, i, 19 + i, rope);
    }

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
