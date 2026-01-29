//! Weapon System
//!
//! Defines weapon types, rarities, and weapon instances.

/// Weapon type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeaponType {
    Pickaxe,
    Pistol,
    Shotgun,
    AssaultRifle,
    Sniper,
    Smg,
}

impl WeaponType {
    /// Convert from u8 (network protocol)
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Pickaxe),
            1 => Some(Self::Pistol),
            2 => Some(Self::Shotgun),
            3 => Some(Self::AssaultRifle),
            4 => Some(Self::Sniper),
            5 => Some(Self::Smg),
            _ => None,
        }
    }

    /// Convert to u8
    pub fn to_u8(self) -> u8 {
        match self {
            Self::Pickaxe => 0,
            Self::Pistol => 1,
            Self::Shotgun => 2,
            Self::AssaultRifle => 3,
            Self::Sniper => 4,
            Self::Smg => 5,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Pickaxe => "PICKAXE",
            Self::Pistol => "PISTOL",
            Self::Shotgun => "SHOTGUN",
            Self::AssaultRifle => "ASSAULT RIFLE",
            Self::Sniper => "SNIPER",
            Self::Smg => "SMG",
        }
    }

    /// Base damage for this weapon type
    pub fn base_damage(&self) -> u8 {
        match self {
            Self::Pickaxe => 20,
            Self::Pistol => 23,
            Self::Shotgun => 90,
            Self::AssaultRifle => 30,
            Self::Sniper => 100,
            Self::Smg => 17,
        }
    }

    /// Rounds per second (fire rate)
    pub fn fire_rate(&self) -> f32 {
        match self {
            Self::Pickaxe => 1.0,
            Self::Pistol => 6.75,
            Self::Shotgun => 0.7,
            Self::AssaultRifle => 5.5,
            Self::Sniper => 0.33,
            Self::Smg => 12.0,
        }
    }

    /// Magazine size
    pub fn magazine_size(&self) -> u16 {
        match self {
            Self::Pickaxe => 0, // Infinite
            Self::Pistol => 16,
            Self::Shotgun => 5,
            Self::AssaultRifle => 30,
            Self::Sniper => 1,
            Self::Smg => 30,
        }
    }

    /// Effective range in units
    pub fn range(&self) -> f32 {
        match self {
            Self::Pickaxe => 2.0,
            Self::Pistol => 50.0,
            Self::Shotgun => 15.0,
            Self::AssaultRifle => 100.0,
            Self::Sniper => 500.0,
            Self::Smg => 40.0,
        }
    }

    /// Reload time in seconds
    pub fn reload_time(&self) -> f32 {
        match self {
            Self::Pickaxe => 0.0,
            Self::Pistol => 1.5,
            Self::Shotgun => 4.5,
            Self::AssaultRifle => 2.3,
            Self::Sniper => 2.5,
            Self::Smg => 2.0,
        }
    }

    /// Headshot multiplier
    pub fn headshot_multiplier(&self) -> f32 {
        match self {
            Self::Pickaxe => 1.0,
            Self::Pistol => 2.0,
            Self::Shotgun => 1.5,
            Self::AssaultRifle => 2.0,
            Self::Sniper => 2.5,
            Self::Smg => 1.75,
        }
    }

    /// Is this a hitscan weapon?
    pub fn is_hitscan(&self) -> bool {
        true // All current weapons are hitscan
    }

    /// Spread angle in degrees (0 = perfectly accurate)
    pub fn spread(&self) -> f32 {
        match self {
            Self::Pickaxe => 0.0,
            Self::Pistol => 1.5,
            Self::Shotgun => 5.0,
            Self::AssaultRifle => 2.0,
            Self::Sniper => 0.0,
            Self::Smg => 3.0,
        }
    }
}

/// Weapon rarity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

impl Rarity {
    /// Color for this rarity (RGB)
    pub fn color(&self) -> u32 {
        match self {
            Self::Common => 0x888888,
            Self::Uncommon => 0x44CC44,
            Self::Rare => 0x4488FF,
            Self::Epic => 0xAA44CC,
            Self::Legendary => 0xFFAA00,
        }
    }

    /// Damage multiplier for this rarity
    pub fn damage_multiplier(&self) -> f32 {
        match self {
            Self::Common => 1.00,
            Self::Uncommon => 1.05,
            Self::Rare => 1.10,
            Self::Epic => 1.15,
            Self::Legendary => 1.21,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Common => "COMMON",
            Self::Uncommon => "UNCOMMON",
            Self::Rare => "RARE",
            Self::Epic => "EPIC",
            Self::Legendary => "LEGENDARY",
        }
    }
}

/// A weapon instance
#[derive(Debug, Clone)]
pub struct Weapon {
    pub weapon_type: WeaponType,
    pub rarity: Rarity,
    pub ammo: u16,
    pub max_ammo: u16,
    pub reload_timer: f32,
    pub fire_cooldown: f32,
}

impl Weapon {
    /// Create a new weapon
    pub fn new(weapon_type: WeaponType, rarity: Rarity) -> Self {
        let max_ammo = weapon_type.magazine_size();
        Self {
            weapon_type,
            rarity,
            ammo: max_ammo,
            max_ammo,
            reload_timer: 0.0,
            fire_cooldown: 0.0,
        }
    }

    /// Create the pickaxe (always available)
    pub fn pickaxe() -> Self {
        Self::new(WeaponType::Pickaxe, Rarity::Common)
    }

    /// Get display name
    pub fn name(&self) -> &'static str {
        self.weapon_type.name()
    }

    /// Get damage with rarity modifier
    pub fn damage(&self) -> u8 {
        let base = self.weapon_type.base_damage() as f32;
        let modified = base * self.rarity.damage_multiplier();
        modified as u8
    }

    /// Get headshot damage
    pub fn headshot_damage(&self) -> u8 {
        let base = self.damage() as f32;
        let modified = base * self.weapon_type.headshot_multiplier();
        modified as u8
    }

    /// Check if weapon can fire
    pub fn can_fire(&self) -> bool {
        self.fire_cooldown <= 0.0 && self.ammo > 0 && self.reload_timer <= 0.0
    }

    /// Check if weapon is reloading
    pub fn is_reloading(&self) -> bool {
        self.reload_timer > 0.0
    }

    /// Fire the weapon
    pub fn fire(&mut self) -> bool {
        if !self.can_fire() {
            return false;
        }

        if self.weapon_type != WeaponType::Pickaxe {
            self.ammo -= 1;
        }
        self.fire_cooldown = 1.0 / self.weapon_type.fire_rate();
        true
    }

    /// Start reloading
    pub fn start_reload(&mut self) {
        if self.ammo < self.max_ammo && self.reload_timer <= 0.0 {
            self.reload_timer = self.weapon_type.reload_time();
        }
    }

    /// Update timers
    pub fn update(&mut self, dt: f32) {
        if self.fire_cooldown > 0.0 {
            self.fire_cooldown -= dt;
        }

        if self.reload_timer > 0.0 {
            self.reload_timer -= dt;
            if self.reload_timer <= 0.0 {
                self.ammo = self.max_ammo;
            }
        }
    }

    /// Add ammo (returns amount actually added)
    pub fn add_ammo(&mut self, amount: u16) -> u16 {
        let space = self.max_ammo - self.ammo;
        let added = amount.min(space);
        self.ammo += added;
        added
    }
}

/// Ammo types (shared across weapon types)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmmoType {
    Light,
    Medium,
    Heavy,
    Shells,
}

impl AmmoType {
    /// Get ammo type for a weapon
    pub fn for_weapon(weapon_type: WeaponType) -> Option<Self> {
        match weapon_type {
            WeaponType::Pickaxe => None,
            WeaponType::Pistol | WeaponType::Smg => Some(Self::Light),
            WeaponType::AssaultRifle => Some(Self::Medium),
            WeaponType::Sniper => Some(Self::Heavy),
            WeaponType::Shotgun => Some(Self::Shells),
        }
    }
}
