//! Game map with POIs, terrain, and structure placement

use glam::Vec3;
use super::loot::{LootSpawn, LootSpawnType, ChestTier};

/// Map dimensions
pub const MAP_SIZE: f32 = 2000.0;
pub const MAP_HALF: f32 = MAP_SIZE / 2.0;

/// Chunk size for terrain
pub const CHUNK_SIZE: f32 = 200.0;
pub const CHUNKS_PER_SIDE: usize = 10;

/// Terrain height parameters
pub const BASE_HEIGHT: f32 = 0.0;
pub const MAX_HILL_HEIGHT: f32 = 60.0;
pub const WATER_LEVEL: f32 = -5.0;

/// Point of Interest definition
#[derive(Debug, Clone)]
pub struct POI {
    /// Name of the location
    pub name: &'static str,
    /// Center position
    pub center: Vec3,
    /// Radius of the POI area
    pub radius: f32,
    /// Loot tier for this location
    pub loot_tier: ChestTier,
    /// Number of buildings
    pub building_count: u8,
    /// POI type for building style
    pub poi_type: POIType,
}

/// Types of POIs (affects building style)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum POIType {
    Town,
    Industrial,
    Rural,
    Military,
    Natural,
}

/// Building types that can spawn in POIs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildingType {
    HouseSmall,
    HouseMedium,
    HouseLarge,
    Warehouse,
    Tower,
    Barn,
    Shed,
    GasStation,
}

impl BuildingType {
    /// Get building dimensions (width, height, depth)
    pub fn dimensions(&self) -> (f32, f32, f32) {
        match self {
            Self::HouseSmall => (8.0, 6.0, 8.0),
            Self::HouseMedium => (12.0, 8.0, 10.0),
            Self::HouseLarge => (16.0, 12.0, 14.0),
            Self::Warehouse => (20.0, 10.0, 30.0),
            Self::Tower => (10.0, 40.0, 10.0),
            Self::Barn => (15.0, 12.0, 20.0),
            Self::Shed => (4.0, 3.0, 4.0),
            Self::GasStation => (12.0, 5.0, 8.0),
        }
    }

    /// Get loot spawn count for this building
    pub fn loot_spawns(&self) -> u8 {
        match self {
            Self::HouseSmall => 2,
            Self::HouseMedium => 4,
            Self::HouseLarge => 6,
            Self::Warehouse => 5,
            Self::Tower => 8,
            Self::Barn => 3,
            Self::Shed => 1,
            Self::GasStation => 3,
        }
    }
}

/// A building instance in the world
#[derive(Debug, Clone)]
pub struct Building {
    pub building_type: BuildingType,
    pub position: Vec3,
    pub rotation: f32, // Y-axis rotation
    pub variant: u8,   // Visual variant
}

/// Vegetation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VegetationType {
    TreePine,
    TreeOak,
    TreeBirch,
    Bush,
    Rock,
}

/// A vegetation instance
#[derive(Debug, Clone, Copy)]
pub struct Vegetation {
    pub veg_type: VegetationType,
    pub position: Vec3,
    pub scale: f32,
    pub variant: u8,
}

/// Game map containing all world data
pub struct GameMap {
    /// All POIs
    pub pois: [POI; 10],
    /// All buildings
    pub buildings: [Option<Building>; 128],
    /// Building count
    pub building_count: usize,
    /// Vegetation instances
    pub vegetation: [Option<Vegetation>; 512],
    /// Vegetation count
    pub vegetation_count: usize,
    /// Loot spawns
    pub loot_spawns: [Option<LootSpawn>; 256],
    /// Loot spawn count
    pub loot_spawn_count: usize,
    /// RNG seed
    seed: u32,
}

impl Default for GameMap {
    fn default() -> Self {
        Self::new(12345)
    }
}

impl GameMap {
    /// Create a new map with the given seed
    pub fn new(seed: u32) -> Self {
        let pois = [
            POI {
                name: "PLEASANT PARK",
                center: Vec3::new(-400.0, 0.0, -400.0),
                radius: 150.0,
                loot_tier: ChestTier::Normal,
                building_count: 8,
                poi_type: POIType::Town,
            },
            POI {
                name: "TILTED TOWERS",
                center: Vec3::new(0.0, 0.0, 0.0),
                radius: 200.0,
                loot_tier: ChestTier::Rare,
                building_count: 12,
                poi_type: POIType::Town,
            },
            POI {
                name: "RETAIL ROW",
                center: Vec3::new(500.0, 0.0, -300.0),
                radius: 120.0,
                loot_tier: ChestTier::Normal,
                building_count: 7,
                poi_type: POIType::Town,
            },
            POI {
                name: "SALTY SPRINGS",
                center: Vec3::new(-200.0, 0.0, 300.0),
                radius: 100.0,
                loot_tier: ChestTier::Normal,
                building_count: 5,
                poi_type: POIType::Town,
            },
            POI {
                name: "LONELY LODGE",
                center: Vec3::new(600.0, 0.0, 500.0),
                radius: 80.0,
                loot_tier: ChestTier::Normal,
                building_count: 3,
                poi_type: POIType::Rural,
            },
            POI {
                name: "LOOT LAKE",
                center: Vec3::new(-100.0, -5.0, -600.0),
                radius: 180.0,
                loot_tier: ChestTier::Normal,
                building_count: 4,
                poi_type: POIType::Natural,
            },
            POI {
                name: "FATAL FIELDS",
                center: Vec3::new(400.0, 0.0, 400.0),
                radius: 140.0,
                loot_tier: ChestTier::Normal,
                building_count: 4,
                poi_type: POIType::Rural,
            },
            POI {
                name: "WAILING WOODS",
                center: Vec3::new(-600.0, 0.0, 200.0),
                radius: 160.0,
                loot_tier: ChestTier::Normal,
                building_count: 2,
                poi_type: POIType::Natural,
            },
            POI {
                name: "DUSTY DEPOT",
                center: Vec3::new(200.0, 0.0, -100.0),
                radius: 100.0,
                loot_tier: ChestTier::Normal,
                building_count: 3,
                poi_type: POIType::Industrial,
            },
            POI {
                name: "TOMATO TOWN",
                center: Vec3::new(-300.0, 0.0, -700.0),
                radius: 90.0,
                loot_tier: ChestTier::Normal,
                building_count: 4,
                poi_type: POIType::Town,
            },
        ];

        let mut map = Self {
            pois,
            buildings: [const { None }; 128],
            building_count: 0,
            vegetation: [const { None }; 512],
            vegetation_count: 0,
            loot_spawns: [const { None }; 256],
            loot_spawn_count: 0,
            seed,
        };

        map.generate_buildings();
        map.generate_vegetation();
        map.generate_loot_spawns();

        map
    }

    /// Get terrain height at a world position
    pub fn get_height_at(&self, x: f32, z: f32) -> f32 {
        // Large scale hills
        let large_scale = self.noise_2d(x * 0.002, z * 0.002) * 40.0;

        // Medium details
        let medium_scale = self.noise_2d(x * 0.01, z * 0.01) * 10.0;

        // Small bumps
        let small_scale = self.noise_2d(x * 0.05, z * 0.05) * 2.0;

        // River valley through center
        let river_dist = (z * 0.5 + libm::sinf(x * 0.01) * 50.0).abs();
        let river_depth = if river_dist < 30.0 {
            -(1.0 - river_dist / 30.0) * 15.0
        } else {
            0.0
        };

        // Flatten POI areas
        let mut poi_flatten: f32 = 1.0;
        for poi in &self.pois {
            let dx = x - poi.center.x;
            let dz = z - poi.center.z;
            let dist = libm::sqrtf(dx * dx + dz * dz);
            if dist < poi.radius {
                let flatten_strength = 1.0 - (dist / poi.radius);
                poi_flatten = poi_flatten.min(1.0 - flatten_strength * 0.9);
            }
        }

        let height = (large_scale + medium_scale + small_scale) * poi_flatten + river_depth;
        height.max(WATER_LEVEL)
    }

    /// Check if a position is in water
    pub fn is_water(&self, x: f32, z: f32) -> bool {
        self.get_height_at(x, z) <= WATER_LEVEL
    }

    /// Get the POI at a position (if any)
    pub fn get_poi_at(&self, x: f32, z: f32) -> Option<&POI> {
        for poi in &self.pois {
            let dx = x - poi.center.x;
            let dz = z - poi.center.z;
            let dist = libm::sqrtf(dx * dx + dz * dz);
            if dist < poi.radius {
                return Some(poi);
            }
        }
        None
    }

    /// Get buildings near a position
    pub fn get_buildings_near(&self, position: Vec3, range: f32) -> impl Iterator<Item = &Building> {
        let range_sq = range * range;
        self.buildings[..self.building_count].iter().filter_map(move |b| {
            b.as_ref().filter(|building| {
                let dx = building.position.x - position.x;
                let dz = building.position.z - position.z;
                let dist_sq = dx * dx + dz * dz;
                dist_sq <= range_sq
            })
        })
    }

    /// Get vegetation near a position
    pub fn get_vegetation_near(&self, position: Vec3, range: f32) -> impl Iterator<Item = &Vegetation> {
        let range_sq = range * range;
        self.vegetation[..self.vegetation_count].iter().filter_map(move |v| {
            v.as_ref().filter(|veg| {
                let dx = veg.position.x - position.x;
                let dz = veg.position.z - position.z;
                let dist_sq = dx * dx + dz * dz;
                dist_sq <= range_sq
            })
        })
    }

    /// Generate buildings for all POIs
    fn generate_buildings(&mut self) {
        for poi in &self.pois.clone() {
            self.generate_poi_buildings(poi);
        }
    }

    /// Generate buildings for a single POI
    fn generate_poi_buildings(&mut self, poi: &POI) {
        let building_types: &[BuildingType] = match poi.poi_type {
            POIType::Town => &[
                BuildingType::HouseSmall,
                BuildingType::HouseMedium,
                BuildingType::HouseLarge,
                BuildingType::GasStation,
            ],
            POIType::Industrial => &[
                BuildingType::Warehouse,
                BuildingType::Shed,
            ],
            POIType::Rural => &[
                BuildingType::Barn,
                BuildingType::HouseSmall,
                BuildingType::Shed,
            ],
            POIType::Military => &[
                BuildingType::Tower,
                BuildingType::Warehouse,
            ],
            POIType::Natural => &[
                BuildingType::Shed,
            ],
        };

        for i in 0..poi.building_count {
            if self.building_count >= self.buildings.len() {
                break;
            }

            // Generate position within POI
            let angle = (i as f32 / poi.building_count as f32) * core::f32::consts::TAU;
            let dist = poi.radius * 0.3 + self.next_random_f32() * poi.radius * 0.5;
            let x = poi.center.x + libm::cosf(angle) * dist;
            let z = poi.center.z + libm::sinf(angle) * dist;
            let y = self.get_height_at(x, z);

            if y <= WATER_LEVEL {
                continue; // Don't place in water
            }

            let building_type = building_types[self.next_random() as usize % building_types.len()];

            self.buildings[self.building_count] = Some(Building {
                building_type,
                position: Vec3::new(x, y, z),
                rotation: self.next_random_f32() * core::f32::consts::TAU,
                variant: (self.next_random() % 4) as u8,
            });
            self.building_count += 1;
        }
    }

    /// Generate vegetation across the map
    fn generate_vegetation(&mut self) {
        // Dense trees in natural POIs
        for poi in &self.pois.clone() {
            if poi.poi_type == POIType::Natural {
                self.generate_forest(poi.center, poi.radius, 0.7);
            }
        }

        // Scattered trees elsewhere
        let step = 80.0;
        let mut x = -MAP_HALF;
        while x < MAP_HALF {
            let mut z = -MAP_HALF;
            while z < MAP_HALF {
                if self.vegetation_count >= self.vegetation.len() {
                    return;
                }

                // Random offset
                let px = x + (self.next_random_f32() - 0.5) * step;
                let pz = z + (self.next_random_f32() - 0.5) * step;
                let py = self.get_height_at(px, pz);

                // Skip if in water, POI, or random cull
                if py <= WATER_LEVEL || self.get_poi_at(px, pz).is_some() {
                    z += step;
                    continue;
                }

                if self.next_random_f32() > 0.3 {
                    z += step;
                    continue;
                }

                let veg_type = match self.next_random() % 10 {
                    0..=3 => VegetationType::TreePine,
                    4..=6 => VegetationType::TreeOak,
                    7 => VegetationType::TreeBirch,
                    8 => VegetationType::Bush,
                    _ => VegetationType::Rock,
                };

                self.vegetation[self.vegetation_count] = Some(Vegetation {
                    veg_type,
                    position: Vec3::new(px, py, pz),
                    scale: 0.8 + self.next_random_f32() * 0.4,
                    variant: (self.next_random() % 4) as u8,
                });
                self.vegetation_count += 1;

                z += step;
            }
            x += step;
        }
    }

    /// Generate a dense forest area
    fn generate_forest(&mut self, center: Vec3, radius: f32, density: f32) {
        let tree_count = (radius * radius * density / 100.0) as usize;

        for _ in 0..tree_count {
            if self.vegetation_count >= self.vegetation.len() {
                return;
            }

            let angle = self.next_random_f32() * core::f32::consts::TAU;
            let dist = libm::sqrtf(self.next_random_f32()) * radius;
            let x = center.x + libm::cosf(angle) * dist;
            let z = center.z + libm::sinf(angle) * dist;
            let y = self.get_height_at(x, z);

            if y <= WATER_LEVEL {
                continue;
            }

            let veg_type = match self.next_random() % 10 {
                0..=5 => VegetationType::TreePine,
                6..=8 => VegetationType::TreeOak,
                _ => VegetationType::TreeBirch,
            };

            self.vegetation[self.vegetation_count] = Some(Vegetation {
                veg_type,
                position: Vec3::new(x, y, z),
                scale: 0.8 + self.next_random_f32() * 0.6,
                variant: (self.next_random() % 4) as u8,
            });
            self.vegetation_count += 1;
        }
    }

    /// Generate loot spawns
    fn generate_loot_spawns(&mut self) {
        // Generate spawns inside buildings
        for building in &self.buildings.clone() {
            if let Some(b) = building {
                let spawn_count = b.building_type.loot_spawns();
                let (width, _height, depth) = b.building_type.dimensions();

                for i in 0..spawn_count {
                    if self.loot_spawn_count >= self.loot_spawns.len() {
                        return;
                    }

                    // Position inside building
                    let local_x = (self.next_random_f32() - 0.5) * width * 0.8;
                    let local_z = (self.next_random_f32() - 0.5) * depth * 0.8;

                    let cos_r = libm::cosf(b.rotation);
                    let sin_r = libm::sinf(b.rotation);
                    let world_x = b.position.x + local_x * cos_r - local_z * sin_r;
                    let world_z = b.position.z + local_x * sin_r + local_z * cos_r;

                    let spawn_type = if i == 0 {
                        // First spawn is always a chest
                        let tier = self.get_poi_at(b.position.x, b.position.z)
                            .map(|p| p.loot_tier)
                            .unwrap_or(ChestTier::Normal);
                        LootSpawnType::Chest(tier)
                    } else if self.next_random() % 3 == 0 {
                        LootSpawnType::AmmoBox
                    } else {
                        LootSpawnType::Floor
                    };

                    self.loot_spawns[self.loot_spawn_count] = Some(LootSpawn {
                        position: Vec3::new(world_x, b.position.y + 0.5, world_z),
                        spawn_type,
                        spawned: false,
                    });
                    self.loot_spawn_count += 1;
                }
            }
        }
    }

    /// Simple 2D noise function
    fn noise_2d(&self, x: f32, y: f32) -> f32 {
        let ix = libm::floorf(x) as i32;
        let iy = libm::floorf(y) as i32;
        let fx = x - libm::floorf(x);
        let fy = y - libm::floorf(y);

        let v00 = self.hash_2d(ix, iy);
        let v10 = self.hash_2d(ix + 1, iy);
        let v01 = self.hash_2d(ix, iy + 1);
        let v11 = self.hash_2d(ix + 1, iy + 1);

        let sx = fx * fx * (3.0 - 2.0 * fx);
        let sy = fy * fy * (3.0 - 2.0 * fy);

        let n0 = v00 + sx * (v10 - v00);
        let n1 = v01 + sx * (v11 - v01);

        n0 + sy * (n1 - n0)
    }

    /// Hash function for noise
    fn hash_2d(&self, x: i32, y: i32) -> f32 {
        let n = x.wrapping_add(y.wrapping_mul(57)).wrapping_add(self.seed as i32);
        let n = (n << 13) ^ n;
        let n = n.wrapping_mul(n.wrapping_mul(n).wrapping_mul(15731).wrapping_add(789221)).wrapping_add(1376312589);
        ((n & 0x7fffffff) as f32) / 0x7fffffff as f32 * 2.0 - 1.0
    }

    /// Get next random number
    fn next_random(&mut self) -> u32 {
        self.seed = self.seed.wrapping_mul(1103515245).wrapping_add(12345);
        self.seed
    }

    /// Get next random float 0-1
    fn next_random_f32(&mut self) -> f32 {
        (self.next_random() & 0x7FFFFFFF) as f32 / 0x7FFFFFFF as f32
    }
}

/// Check if a ray from origin in direction hits any building
pub fn ray_building_collision(
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
    buildings: &[Option<Building>],
) -> Option<(Vec3, f32)> {
    let mut closest: Option<(Vec3, f32)> = None;

    for building in buildings.iter().flatten() {
        let (width, height, depth) = building.building_type.dimensions();
        let half_w = width / 2.0;
        let half_d = depth / 2.0;

        // Simple AABB test (ignoring rotation for now)
        let min = building.position + Vec3::new(-half_w, 0.0, -half_d);
        let max = building.position + Vec3::new(half_w, height, half_d);

        if let Some((hit_point, dist)) = ray_aabb(origin, direction, min, max) {
            if dist <= max_distance {
                match closest {
                    Some((_, closest_dist)) if dist >= closest_dist => {}
                    _ => closest = Some((hit_point, dist)),
                }
            }
        }
    }

    closest
}

/// Ray-AABB intersection returning hit point
fn ray_aabb(origin: Vec3, direction: Vec3, min: Vec3, max: Vec3) -> Option<(Vec3, f32)> {
    let inv_dir = Vec3::new(
        if direction.x.abs() < 0.0001 { f32::MAX } else { 1.0 / direction.x },
        if direction.y.abs() < 0.0001 { f32::MAX } else { 1.0 / direction.y },
        if direction.z.abs() < 0.0001 { f32::MAX } else { 1.0 / direction.z },
    );

    let t1 = (min.x - origin.x) * inv_dir.x;
    let t2 = (max.x - origin.x) * inv_dir.x;
    let t3 = (min.y - origin.y) * inv_dir.y;
    let t4 = (max.y - origin.y) * inv_dir.y;
    let t5 = (min.z - origin.z) * inv_dir.z;
    let t6 = (max.z - origin.z) * inv_dir.z;

    let tmin = t1.min(t2).max(t3.min(t4)).max(t5.min(t6));
    let tmax = t1.max(t2).min(t3.max(t4)).min(t5.max(t6));

    if tmax < 0.0 || tmin > tmax {
        return None;
    }

    let t = if tmin < 0.0 { tmax } else { tmin };
    Some((origin + direction * t, t))
}
