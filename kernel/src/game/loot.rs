//! Loot system - drops, spawns, and pickups

use glam::Vec3;
use super::weapon::{Weapon, WeaponType, Rarity, AmmoType};

/// Maximum loot drops in world
pub const MAX_LOOT_DROPS: usize = 256;

/// Pickup range
pub const PICKUP_RANGE: f32 = 2.5;

/// Loot drop glow pulse speed
pub const GLOW_PULSE_SPEED: f32 = 3.0;

/// Loot item types
#[derive(Debug, Clone)]
pub enum LootItem {
    /// A weapon
    Weapon(Weapon),
    /// Ammo box
    Ammo {
        ammo_type: AmmoType,
        amount: u16,
    },
    /// Building materials
    Materials {
        wood: u32,
        brick: u32,
        metal: u32,
    },
    /// Health item (bandages, medkit)
    Health {
        amount: u8,
        use_time: f32,
        max_health: u8, // Cap for healing (bandages cap at 75)
    },
    /// Shield item (small shield, big shield, slurp)
    Shield {
        amount: u8,
        use_time: f32,
    },
}

impl LootItem {
    /// Get the rarity color for this item
    pub fn rarity_color(&self) -> u32 {
        match self {
            LootItem::Weapon(w) => w.rarity.color(),
            LootItem::Ammo { .. } => 0x888888,    // Gray
            LootItem::Materials { .. } => 0x888888, // Gray
            LootItem::Health { amount, .. } => {
                if *amount >= 100 { 0x44AA44 } // Green for medkit
                else { 0x888888 }               // Gray for bandage
            }
            LootItem::Shield { amount, .. } => {
                if *amount >= 50 { 0xAA44CC }   // Purple for big shield
                else { 0x4488FF }               // Blue for small shield
            }
        }
    }

    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            LootItem::Weapon(w) => w.name(),
            LootItem::Ammo { ammo_type, .. } => match ammo_type {
                AmmoType::Light => "LIGHT AMMO",
                AmmoType::Medium => "MEDIUM AMMO",
                AmmoType::Heavy => "HEAVY AMMO",
                AmmoType::Shells => "SHELLS",
            },
            LootItem::Materials { .. } => "MATERIALS",
            LootItem::Health { amount, .. } => {
                if *amount >= 100 { "MEDKIT" }
                else { "BANDAGES" }
            }
            LootItem::Shield { amount, .. } => {
                if *amount >= 50 { "SHIELD POTION" }
                else { "SMALL SHIELD" }
            }
        }
    }
}

/// A loot drop in the world
#[derive(Debug, Clone)]
pub struct LootDrop {
    /// Unique ID
    pub id: u16,
    /// World position
    pub position: Vec3,
    /// The item
    pub item: LootItem,
    /// Rotation (for visual spin)
    pub rotation: f32,
    /// Glow pulse timer
    pub glow_timer: f32,
    /// Whether this drop is from a player (vs chest/spawn)
    pub from_player: bool,
    /// Time until despawn (drops from kills persist longer)
    pub despawn_timer: f32,
}

impl LootDrop {
    pub fn new(id: u16, position: Vec3, item: LootItem, from_player: bool) -> Self {
        Self {
            id,
            position,
            item,
            rotation: 0.0,
            glow_timer: 0.0,
            from_player,
            despawn_timer: if from_player { 120.0 } else { 300.0 },
        }
    }

    /// Update the drop (rotation, glow, despawn)
    pub fn update(&mut self, dt: f32) {
        self.rotation += dt * 1.5;
        if self.rotation > core::f32::consts::TAU {
            self.rotation -= core::f32::consts::TAU;
        }

        self.glow_timer += dt * GLOW_PULSE_SPEED;
        if self.glow_timer > core::f32::consts::TAU {
            self.glow_timer -= core::f32::consts::TAU;
        }

        self.despawn_timer -= dt;
    }

    /// Check if should despawn
    pub fn should_despawn(&self) -> bool {
        self.despawn_timer <= 0.0
    }

    /// Get glow intensity (0.0 to 1.0)
    pub fn glow_intensity(&self) -> f32 {
        0.5 + 0.5 * libm::sinf(self.glow_timer)
    }
}

/// Chest loot tiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChestTier {
    /// Normal chest - common to rare weapons
    Normal,
    /// Rare chest - rare to epic weapons
    Rare,
    /// Supply drop - epic to legendary weapons
    SupplyDrop,
}

/// Loot manager
#[derive(Debug)]
pub struct LootManager {
    /// All active loot drops
    pub drops: [Option<LootDrop>; MAX_LOOT_DROPS],
    /// Next drop ID
    next_id: u16,
    /// RNG seed for loot generation
    seed: u32,
}

impl Default for LootManager {
    fn default() -> Self {
        Self::new(12345)
    }
}

impl LootManager {
    pub fn new(seed: u32) -> Self {
        Self {
            drops: [const { None }; MAX_LOOT_DROPS],
            next_id: 0,
            seed,
        }
    }

    /// Update all drops
    pub fn update(&mut self, dt: f32) {
        for drop in &mut self.drops {
            if let Some(d) = drop {
                d.update(dt);
                if d.should_despawn() {
                    *drop = None;
                }
            }
        }
    }

    /// Spawn a specific loot drop
    pub fn spawn_drop(&mut self, position: Vec3, item: LootItem, from_player: bool) -> Option<u16> {
        // Find empty slot
        for slot in &mut self.drops {
            if slot.is_none() {
                let id = self.next_id;
                self.next_id = self.next_id.wrapping_add(1);
                *slot = Some(LootDrop::new(id, position, item, from_player));
                return Some(id);
            }
        }
        None
    }

    /// Spawn loot from a chest
    pub fn spawn_chest_loot(&mut self, position: Vec3, tier: ChestTier) {
        let weapon = self.generate_weapon(tier);
        let offset1 = Vec3::new(-0.5, 0.0, 0.0);
        let offset2 = Vec3::new(0.5, 0.0, 0.0);
        let offset3 = Vec3::new(0.0, 0.0, 0.5);

        // Spawn weapon
        self.spawn_drop(position + offset1, LootItem::Weapon(weapon), false);

        // Spawn ammo or materials (generate first to avoid borrow issues)
        self.seed = self.next_random();
        let secondary_item = if self.seed % 2 == 0 {
            self.generate_ammo()
        } else {
            self.generate_materials()
        };
        self.spawn_drop(position + offset2, secondary_item, false);

        // Chance for healing item (generate first to avoid borrow issues)
        self.seed = self.next_random();
        if self.seed % 3 == 0 {
            let healing_item = self.generate_healing();
            self.spawn_drop(position + offset3, healing_item, false);
        }
    }

    /// Spawn floor loot at a position
    pub fn spawn_floor_loot(&mut self, position: Vec3) {
        self.seed = self.next_random();
        let item = match self.seed % 10 {
            0..=4 => LootItem::Weapon(self.generate_weapon(ChestTier::Normal)),
            5..=7 => self.generate_ammo(),
            8 => self.generate_materials(),
            _ => self.generate_healing(),
        };
        self.spawn_drop(position, item, false);
    }

    /// Spawn loot from eliminated player
    pub fn spawn_death_loot(&mut self, position: Vec3, weapons: &[Option<Weapon>; 5], materials: (u32, u32, u32)) {
        let mut offset_angle = 0.0f32;
        let drop_radius = 1.5;

        // Drop all weapons
        for weapon in weapons.iter().flatten() {
            let offset = Vec3::new(
                libm::cosf(offset_angle) * drop_radius,
                0.0,
                libm::sinf(offset_angle) * drop_radius,
            );
            self.spawn_drop(position + offset, LootItem::Weapon(weapon.clone()), true);
            offset_angle += core::f32::consts::TAU / 6.0;
        }

        // Drop materials if any
        if materials.0 > 0 || materials.1 > 0 || materials.2 > 0 {
            let offset = Vec3::new(
                libm::cosf(offset_angle) * drop_radius,
                0.0,
                libm::sinf(offset_angle) * drop_radius,
            );
            self.spawn_drop(
                position + offset,
                LootItem::Materials {
                    wood: materials.0,
                    brick: materials.1,
                    metal: materials.2,
                },
                true,
            );
        }
    }

    /// Get nearest loot drop within pickup range
    pub fn get_nearest_pickup(&self, position: Vec3) -> Option<&LootDrop> {
        let mut nearest: Option<&LootDrop> = None;
        let mut nearest_dist_sq = PICKUP_RANGE * PICKUP_RANGE;

        for drop in &self.drops {
            if let Some(d) = drop {
                let dist_sq = (d.position - position).length_squared();
                if dist_sq < nearest_dist_sq {
                    nearest_dist_sq = dist_sq;
                    nearest = Some(d);
                }
            }
        }

        nearest
    }

    /// Pick up a loot drop by ID, returns the item
    pub fn pickup(&mut self, id: u16) -> Option<LootItem> {
        for drop in &mut self.drops {
            if let Some(d) = drop {
                if d.id == id {
                    let item = d.item.clone();
                    *drop = None;
                    return Some(item);
                }
            }
        }
        None
    }

    /// Get drops near a position for rendering
    pub fn get_drops_near(&self, position: Vec3, range: f32) -> impl Iterator<Item = &LootDrop> {
        let range_sq = range * range;
        self.drops.iter().filter_map(move |d| {
            d.as_ref().filter(|drop| (drop.position - position).length_squared() <= range_sq)
        })
    }

    /// Generate a random weapon based on chest tier
    fn generate_weapon(&mut self, tier: ChestTier) -> Weapon {
        self.seed = self.next_random();
        let weapon_type = match self.seed % 5 {
            0 => WeaponType::Pistol,
            1 => WeaponType::Shotgun,
            2 => WeaponType::AssaultRifle,
            3 => WeaponType::Sniper,
            _ => WeaponType::Smg,
        };

        self.seed = self.next_random();
        let rarity = match tier {
            ChestTier::Normal => match self.seed % 100 {
                0..=50 => Rarity::Common,
                51..=85 => Rarity::Uncommon,
                _ => Rarity::Rare,
            },
            ChestTier::Rare => match self.seed % 100 {
                0..=30 => Rarity::Uncommon,
                31..=70 => Rarity::Rare,
                _ => Rarity::Epic,
            },
            ChestTier::SupplyDrop => match self.seed % 100 {
                0..=20 => Rarity::Rare,
                21..=60 => Rarity::Epic,
                _ => Rarity::Legendary,
            },
        };

        Weapon::new(weapon_type, rarity)
    }

    /// Generate random ammo
    fn generate_ammo(&mut self) -> LootItem {
        self.seed = self.next_random();
        let ammo_type = match self.seed % 4 {
            0 => AmmoType::Light,
            1 => AmmoType::Medium,
            2 => AmmoType::Heavy,
            _ => AmmoType::Shells,
        };

        self.seed = self.next_random();
        let amount = match ammo_type {
            AmmoType::Light => 30 + (self.seed % 30) as u16,
            AmmoType::Medium => 20 + (self.seed % 20) as u16,
            AmmoType::Heavy => 6 + (self.seed % 6) as u16,
            AmmoType::Shells => 5 + (self.seed % 5) as u16,
        };

        LootItem::Ammo { ammo_type, amount }
    }

    /// Generate random materials
    fn generate_materials(&mut self) -> LootItem {
        self.seed = self.next_random();
        LootItem::Materials {
            wood: 20 + (self.seed % 30),
            brick: 10 + (self.seed % 20),
            metal: 5 + (self.seed % 15),
        }
    }

    /// Generate random healing item
    fn generate_healing(&mut self) -> LootItem {
        self.seed = self.next_random();
        match self.seed % 4 {
            0 => LootItem::Health {
                amount: 15,
                use_time: 4.0,
                max_health: 75,
            }, // Bandages
            1 => LootItem::Health {
                amount: 100,
                use_time: 10.0,
                max_health: 100,
            }, // Medkit
            2 => LootItem::Shield {
                amount: 25,
                use_time: 2.0,
            }, // Small shield
            _ => LootItem::Shield {
                amount: 50,
                use_time: 5.0,
            }, // Big shield
        }
    }

    /// Simple LCG random
    fn next_random(&mut self) -> u32 {
        self.seed = self.seed.wrapping_mul(1103515245).wrapping_add(12345);
        self.seed
    }
}

/// Loot spawn point in the world
#[derive(Debug, Clone, Copy)]
pub struct LootSpawn {
    pub position: Vec3,
    pub spawn_type: LootSpawnType,
    pub spawned: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum LootSpawnType {
    /// Floor loot
    Floor,
    /// Chest
    Chest(ChestTier),
    /// Ammo box
    AmmoBox,
}
