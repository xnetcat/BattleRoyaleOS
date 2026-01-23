//! Tile-based parallel rendering

use super::framebuffer::FRAMEBUFFER;
use super::zbuffer::ZBuffer;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};
use renderer::vertex::Vertex;
use spin::Mutex;

/// Tile size in pixels (64x64 fits in L1 cache)
pub const TILE_SIZE: usize = 64;

/// A rendering tile
#[derive(Debug, Clone)]
pub struct Tile {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

impl Tile {
    /// Check if a triangle potentially overlaps this tile
    pub fn intersects_triangle(&self, v0: &Vertex, v1: &Vertex, v2: &Vertex) -> bool {
        let min_x = libm::floorf(
            v0.position
                .x
                .min(v1.position.x)
                .min(v2.position.x),
        ) as i32;
        let max_x = libm::ceilf(
            v0.position
                .x
                .max(v1.position.x)
                .max(v2.position.x),
        ) as i32;
        let min_y = libm::floorf(
            v0.position
                .y
                .min(v1.position.y)
                .min(v2.position.y),
        ) as i32;
        let max_y = libm::ceilf(
            v0.position
                .y
                .max(v1.position.y)
                .max(v2.position.y),
        ) as i32;

        let tile_x = self.x as i32;
        let tile_y = self.y as i32;
        let tile_right = (self.x + self.width) as i32;
        let tile_bottom = (self.y + self.height) as i32;

        !(max_x < tile_x || min_x > tile_right || max_y < tile_y || min_y > tile_bottom)
    }
}

/// Work queue for distributing tiles to cores
pub struct TileWorkQueue {
    tiles: Vec<Tile>,
    next_tile: AtomicUsize,
}

impl TileWorkQueue {
    /// Create a new work queue from screen dimensions
    pub fn new(screen_width: usize, screen_height: usize) -> Self {
        let mut tiles = Vec::new();

        let tiles_x = (screen_width + TILE_SIZE - 1) / TILE_SIZE;
        let tiles_y = (screen_height + TILE_SIZE - 1) / TILE_SIZE;

        for ty in 0..tiles_y {
            for tx in 0..tiles_x {
                let x = tx * TILE_SIZE;
                let y = ty * TILE_SIZE;
                let width = TILE_SIZE.min(screen_width - x);
                let height = TILE_SIZE.min(screen_height - y);

                tiles.push(Tile {
                    x,
                    y,
                    width,
                    height,
                });
            }
        }

        Self {
            tiles,
            next_tile: AtomicUsize::new(0),
        }
    }

    /// Get the next tile to process (returns None when all tiles are done)
    pub fn get_next_tile(&self) -> Option<&Tile> {
        let idx = self.next_tile.fetch_add(1, Ordering::Relaxed);
        self.tiles.get(idx)
    }

    /// Reset the queue for a new frame
    pub fn reset(&self) {
        self.next_tile.store(0, Ordering::Relaxed);
    }

    /// Get total number of tiles
    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }
}

/// Global tile work queue
pub static TILE_QUEUE: Mutex<Option<TileWorkQueue>> = Mutex::new(None);

/// Initialize the tile system
pub fn init(width: usize, height: usize) {
    *TILE_QUEUE.lock() = Some(TileWorkQueue::new(width, height));
}

/// Reset tiles for new frame
pub fn reset() {
    if let Some(queue) = TILE_QUEUE.lock().as_ref() {
        queue.reset();
    }
}

/// Triangle binned to tiles
#[derive(Clone)]
pub struct BinnedTriangle {
    pub v0: Vertex,
    pub v1: Vertex,
    pub v2: Vertex,
}

/// Tile bin for triangles
pub struct TileBin {
    pub triangles: Vec<BinnedTriangle>,
}

impl TileBin {
    pub fn new() -> Self {
        Self {
            triangles: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.triangles.clear();
    }

    pub fn add(&mut self, v0: Vertex, v1: Vertex, v2: Vertex) {
        self.triangles.push(BinnedTriangle { v0, v1, v2 });
    }
}

/// Global triangle bins (one per tile)
pub static TRIANGLE_BINS: Mutex<Vec<TileBin>> = Mutex::new(Vec::new());

/// Initialize triangle bins
pub fn init_bins(tile_count: usize) {
    let mut bins = TRIANGLE_BINS.lock();
    bins.clear();
    for _ in 0..tile_count {
        bins.push(TileBin::new());
    }
}

/// Clear all bins for new frame
pub fn clear_bins() {
    let mut bins = TRIANGLE_BINS.lock();
    for bin in bins.iter_mut() {
        bin.clear();
    }
}

/// Bin a triangle to appropriate tiles
pub fn bin_triangle(v0: &Vertex, v1: &Vertex, v2: &Vertex) {
    let queue_guard = TILE_QUEUE.lock();
    let queue = match queue_guard.as_ref() {
        Some(q) => q,
        None => return,
    };

    let mut bins = TRIANGLE_BINS.lock();

    for (idx, tile) in queue.tiles.iter().enumerate() {
        if tile.intersects_triangle(v0, v1, v2) {
            if idx < bins.len() {
                bins[idx].add(v0.clone(), v1.clone(), v2.clone());
            }
        }
    }
}
