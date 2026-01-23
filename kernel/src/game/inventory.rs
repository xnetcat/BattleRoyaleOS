//! Player inventory system

use super::weapon::{Weapon, WeaponType, Rarity, AmmoType};

/// Number of weapon slots
pub const INVENTORY_SLOTS: usize = 5;

/// Player inventory
#[derive(Debug, Clone)]
pub struct Inventory {
    /// Weapon slots (0-4)
    pub slots: [Option<Weapon>; INVENTORY_SLOTS],
    /// Currently selected slot index
    pub selected_slot: usize,
    /// The pickaxe (always available, not in slots)
    pub pickaxe: Weapon,
    /// Whether pickaxe is selected (true) or a slot (false)
    pub pickaxe_selected: bool,
    /// Ammo reserves by type
    pub ammo: AmmoReserves,
    /// Building materials
    pub materials: Materials,
}

/// Ammo reserves
#[derive(Debug, Clone, Copy, Default)]
pub struct AmmoReserves {
    pub light: u16,
    pub medium: u16,
    pub heavy: u16,
    pub shells: u16,
}

impl AmmoReserves {
    /// Get ammo count for a type
    pub fn get(&self, ammo_type: AmmoType) -> u16 {
        match ammo_type {
            AmmoType::Light => self.light,
            AmmoType::Medium => self.medium,
            AmmoType::Heavy => self.heavy,
            AmmoType::Shells => self.shells,
        }
    }

    /// Add ammo
    pub fn add(&mut self, ammo_type: AmmoType, amount: u16) {
        match ammo_type {
            AmmoType::Light => self.light = (self.light + amount).min(999),
            AmmoType::Medium => self.medium = (self.medium + amount).min(999),
            AmmoType::Heavy => self.heavy = (self.heavy + amount).min(999),
            AmmoType::Shells => self.shells = (self.shells + amount).min(999),
        }
    }

    /// Take ammo (returns amount taken)
    pub fn take(&mut self, ammo_type: AmmoType, amount: u16) -> u16 {
        match ammo_type {
            AmmoType::Light => {
                let taken = amount.min(self.light);
                self.light -= taken;
                taken
            }
            AmmoType::Medium => {
                let taken = amount.min(self.medium);
                self.medium -= taken;
                taken
            }
            AmmoType::Heavy => {
                let taken = amount.min(self.heavy);
                self.heavy -= taken;
                taken
            }
            AmmoType::Shells => {
                let taken = amount.min(self.shells);
                self.shells -= taken;
                taken
            }
        }
    }
}

/// Building materials
#[derive(Debug, Clone, Copy)]
pub struct Materials {
    pub wood: u32,
    pub brick: u32,
    pub metal: u32,
}

impl Default for Materials {
    fn default() -> Self {
        Self {
            wood: 0,
            brick: 0,
            metal: 0,
        }
    }
}

impl Materials {
    pub fn total(&self) -> u32 {
        self.wood + self.brick + self.metal
    }

    pub fn add_wood(&mut self, amount: u32) {
        self.wood = (self.wood + amount).min(999);
    }

    pub fn add_brick(&mut self, amount: u32) {
        self.brick = (self.brick + amount).min(999);
    }

    pub fn add_metal(&mut self, amount: u32) {
        self.metal = (self.metal + amount).min(999);
    }
}

impl Default for Inventory {
    fn default() -> Self {
        Self::new()
    }
}

impl Inventory {
    /// Create a new empty inventory
    pub fn new() -> Self {
        Self {
            slots: [None, None, None, None, None],
            selected_slot: 0,
            pickaxe: Weapon::pickaxe(),
            pickaxe_selected: true,
            ammo: AmmoReserves::default(),
            materials: Materials::default(),
        }
    }

    /// Get the currently selected weapon
    pub fn selected_weapon(&self) -> &Weapon {
        if self.pickaxe_selected {
            &self.pickaxe
        } else {
            self.slots[self.selected_slot].as_ref().unwrap_or(&self.pickaxe)
        }
    }

    /// Get the currently selected weapon mutably
    pub fn selected_weapon_mut(&mut self) -> &mut Weapon {
        if self.pickaxe_selected {
            &mut self.pickaxe
        } else {
            if self.slots[self.selected_slot].is_some() {
                self.slots[self.selected_slot].as_mut().unwrap()
            } else {
                &mut self.pickaxe
            }
        }
    }

    /// Select pickaxe
    pub fn select_pickaxe(&mut self) {
        self.pickaxe_selected = true;
    }

    /// Select a slot (1-5)
    pub fn select_slot(&mut self, slot: usize) {
        if slot < INVENTORY_SLOTS {
            self.selected_slot = slot;
            self.pickaxe_selected = false;
        }
    }

    /// Cycle to next weapon
    pub fn next_weapon(&mut self) {
        if self.pickaxe_selected {
            // Go to first slot
            self.pickaxe_selected = false;
            self.selected_slot = 0;
        } else {
            self.selected_slot = (self.selected_slot + 1) % INVENTORY_SLOTS;
            if self.selected_slot == 0 && self.slots.iter().all(|s| s.is_none()) {
                // No weapons, go back to pickaxe
                self.pickaxe_selected = true;
            }
        }
    }

    /// Cycle to previous weapon
    pub fn prev_weapon(&mut self) {
        if self.pickaxe_selected {
            // Go to last slot
            self.pickaxe_selected = false;
            self.selected_slot = INVENTORY_SLOTS - 1;
        } else if self.selected_slot == 0 {
            self.pickaxe_selected = true;
        } else {
            self.selected_slot -= 1;
        }
    }

    /// Add a weapon to inventory (returns weapon that was dropped if slot full)
    pub fn add_weapon(&mut self, weapon: Weapon) -> Option<Weapon> {
        // Try to find empty slot
        for i in 0..INVENTORY_SLOTS {
            if self.slots[i].is_none() {
                self.slots[i] = Some(weapon);
                return None;
            }
        }

        // All slots full, swap with current slot
        let old = self.slots[self.selected_slot].take();
        self.slots[self.selected_slot] = Some(weapon);
        old
    }

    /// Drop the currently selected weapon
    pub fn drop_selected(&mut self) -> Option<Weapon> {
        if self.pickaxe_selected {
            None // Can't drop pickaxe
        } else {
            let weapon = self.slots[self.selected_slot].take();
            self.pickaxe_selected = true;
            weapon
        }
    }

    /// Find first empty slot index
    pub fn first_empty_slot(&self) -> Option<usize> {
        self.slots.iter().position(|s| s.is_none())
    }

    /// Check if inventory is full
    pub fn is_full(&self) -> bool {
        self.slots.iter().all(|s| s.is_some())
    }

    /// Get weapon count
    pub fn weapon_count(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }

    /// Update all weapons (timers)
    pub fn update(&mut self, dt: f32) {
        self.pickaxe.update(dt);
        for slot in &mut self.slots {
            if let Some(weapon) = slot {
                weapon.update(dt);
            }
        }
    }

    /// Reload current weapon from ammo reserves
    pub fn reload_current(&mut self) {
        if self.pickaxe_selected {
            return;
        }

        if let Some(weapon) = &mut self.slots[self.selected_slot] {
            if let Some(ammo_type) = AmmoType::for_weapon(weapon.weapon_type) {
                if weapon.ammo < weapon.max_ammo && !weapon.is_reloading() {
                    let needed = weapon.max_ammo - weapon.ammo;
                    let available = self.ammo.get(ammo_type);
                    if available > 0 {
                        weapon.start_reload();
                        // Ammo will be deducted when reload completes
                    }
                }
            }
        }
    }
}
