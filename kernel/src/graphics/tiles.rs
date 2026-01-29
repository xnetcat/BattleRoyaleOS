//! Tile-based parallel rendering

use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicU16, AtomicUsize, Ordering};
use renderer::vertex::Vertex;
use spin::Mutex;

/// Tile size in pixels (64x64 fits in L1 cache)
pub const TILE_SIZE: usize = 64;

/// Maximum triangles per frame
pub const MAX_TRIANGLES_PER_FRAME: usize = 32768;

/// Maximum triangles per tile bin
pub const MAX_TRIANGLES_PER_TILE: usize = 512;

/// Fixed-point precision for edge functions
const FP_BITS: i32 = 4;
const FP_ONE: i32 = 1 << FP_BITS;
const FP_HALF: i32 = FP_ONE >> 1;

/// Color fixed-point precision
const COLOR_BITS: i32 = 16;
const COLOR_ONE: i32 = 1 << COLOR_BITS;

/// Pre-computed screen-space triangle with edge coefficients (cache-line aligned)
#[repr(C, align(64))]
#[derive(Clone, Copy)]
pub struct ScreenTriangle {
    // Fixed-point screen positions
    pub x0: i32,
    pub y0: i32,
    pub z0: f32,
    pub x1: i32,
    pub y1: i32,
    pub z1: f32,
    pub x2: i32,
    pub y2: i32,
    pub z2: f32,
    // Pre-computed edge coefficients (A*x + B*y + C >= 0)
    pub a12: i32,
    pub b12: i32,
    pub c12: i64,
    pub a20: i32,
    pub b20: i32,
    pub c20: i64,
    pub a01: i32,
    pub b01: i32,
    pub c01: i64,
    // Bounding box (pixel coordinates)
    pub min_x: i32,
    pub max_x: i32,
    pub min_y: i32,
    pub max_y: i32,
    // 1/area for barycentric
    pub inv_area: f32,
    // Winding order
    pub is_cw: bool,
    // Fixed-point colors (pre-scaled by COLOR_ONE * 255)
    pub r0: i64,
    pub g0: i64,
    pub b0: i64,
    pub r1: i64,
    pub g1: i64,
    pub b1: i64,
    pub r2: i64,
    pub g2: i64,
    pub b2: i64,
}

impl ScreenTriangle {
    /// Create a ScreenTriangle from transformed vertices
    /// Returns None if triangle is degenerate or fully clipped
    pub fn from_vertices(v0: &Vertex, v1: &Vertex, v2: &Vertex, fb_width: i32, fb_height: i32) -> Option<Self> {
        // Convert to fixed-point
        let x0 = (v0.position.x * FP_ONE as f32) as i32;
        let y0 = (v0.position.y * FP_ONE as f32) as i32;
        let x1 = (v1.position.x * FP_ONE as f32) as i32;
        let y1 = (v1.position.y * FP_ONE as f32) as i32;
        let x2 = (v2.position.x * FP_ONE as f32) as i32;
        let y2 = (v2.position.y * FP_ONE as f32) as i32;

        // Compute bounding box (pixel coordinates)
        let min_x = ((x0.min(x1).min(x2)) >> FP_BITS).max(0);
        let max_x = ((x0.max(x1).max(x2) + FP_ONE - 1) >> FP_BITS).min(fb_width - 1);
        let min_y = ((y0.min(y1).min(y2)) >> FP_BITS).max(0);
        let max_y = ((y0.max(y1).max(y2) + FP_ONE - 1) >> FP_BITS).min(fb_height - 1);

        // Skip if bounding box is empty
        if min_x > max_x || min_y > max_y {
            return None;
        }

        // Edge coefficients
        let mut a12 = y1 - y2;
        let mut b12 = x2 - x1;
        let mut a20 = y2 - y0;
        let mut b20 = x0 - x2;
        let mut a01 = y0 - y1;
        let mut b01 = x1 - x0;

        // Compute signed area
        let area = (a01 as i64) * (x2 as i64) + (b01 as i64) * (y2 as i64)
            + (x0 as i64) * (y1 as i64) - (y0 as i64) * (x1 as i64);

        if area == 0 {
            return None;
        }

        let is_cw = area < 0;
        let area = if is_cw {
            a12 = -a12;
            b12 = -b12;
            a20 = -a20;
            b20 = -b20;
            a01 = -a01;
            b01 = -b01;
            -area
        } else {
            area
        };

        // Edge constants (C in Ax + By + C)
        let mut c12 = (x1 as i64) * (y2 as i64) - (y1 as i64) * (x2 as i64);
        let mut c20 = (x2 as i64) * (y0 as i64) - (y2 as i64) * (x0 as i64);
        let mut c01 = (x0 as i64) * (y1 as i64) - (y0 as i64) * (x1 as i64);
        if is_cw {
            c12 = -c12;
            c20 = -c20;
            c01 = -c01;
        }

        // Pre-scale colors for interpolation
        let r0 = (v0.color.x * 255.0 * COLOR_ONE as f32) as i64;
        let g0 = (v0.color.y * 255.0 * COLOR_ONE as f32) as i64;
        let b0 = (v0.color.z * 255.0 * COLOR_ONE as f32) as i64;
        let r1 = (v1.color.x * 255.0 * COLOR_ONE as f32) as i64;
        let g1 = (v1.color.y * 255.0 * COLOR_ONE as f32) as i64;
        let b1 = (v1.color.z * 255.0 * COLOR_ONE as f32) as i64;
        let r2 = (v2.color.x * 255.0 * COLOR_ONE as f32) as i64;
        let g2 = (v2.color.y * 255.0 * COLOR_ONE as f32) as i64;
        let b2 = (v2.color.z * 255.0 * COLOR_ONE as f32) as i64;

        Some(Self {
            x0,
            y0,
            z0: v0.position.z,
            x1,
            y1,
            z1: v1.position.z,
            x2,
            y2,
            z2: v2.position.z,
            a12,
            b12,
            c12,
            a20,
            b20,
            c20,
            a01,
            b01,
            c01,
            min_x,
            max_x,
            min_y,
            max_y,
            inv_area: 1.0 / (area as f32),
            is_cw,
            r0,
            g0,
            b0,
            r1,
            g1,
            b1,
            r2,
            g2,
            b2,
        })
    }

    /// Check if this triangle overlaps a tile
    #[inline]
    pub fn overlaps_tile(&self, tile_x: i32, tile_y: i32, tile_w: i32, tile_h: i32) -> bool {
        let tile_right = tile_x + tile_w;
        let tile_bottom = tile_y + tile_h;
        !(self.max_x < tile_x || self.min_x >= tile_right || self.max_y < tile_y || self.min_y >= tile_bottom)
    }
}

/// Lock-free per-tile bin using atomic counter
pub struct TileBinLockFree {
    indices: UnsafeCell<[u16; MAX_TRIANGLES_PER_TILE]>,
    count: AtomicU16,
}

// Safety: We ensure exclusive access via atomic operations
unsafe impl Sync for TileBinLockFree {}
unsafe impl Send for TileBinLockFree {}

impl TileBinLockFree {
    pub const fn new() -> Self {
        Self {
            indices: UnsafeCell::new([0u16; MAX_TRIANGLES_PER_TILE]),
            count: AtomicU16::new(0),
        }
    }

    /// Add a triangle index to this bin (lock-free)
    /// Returns true if added, false if bin is full
    #[inline]
    pub fn add(&self, triangle_idx: u16) -> bool {
        let slot = self.count.fetch_add(1, Ordering::AcqRel) as usize;
        if slot < MAX_TRIANGLES_PER_TILE {
            // Safety: slot is unique due to atomic increment
            unsafe {
                (*self.indices.get())[slot] = triangle_idx;
            }
            true
        } else {
            // Bin overflow - don't decrement, just drop the triangle
            false
        }
    }

    /// Get triangle count
    #[inline]
    pub fn len(&self) -> usize {
        (self.count.load(Ordering::Acquire) as usize).min(MAX_TRIANGLES_PER_TILE)
    }

    /// Get triangle index at position
    #[inline]
    pub fn get(&self, idx: usize) -> Option<u16> {
        if idx < self.len() {
            // Safety: idx is within bounds
            Some(unsafe { (*self.indices.get())[idx] })
        } else {
            None
        }
    }

    /// Clear the bin for next frame
    #[inline]
    pub fn clear(&self) {
        self.count.store(0, Ordering::Release);
    }
}

/// Lock-free triangle storage for the frame
/// Uses UnsafeCell for lock-free writes (single producer) and reads (multiple consumers)
pub struct TriangleStorage {
    triangles: UnsafeCell<[ScreenTriangle; MAX_TRIANGLES_PER_FRAME]>,
    count: AtomicUsize,
}

// Safety: TriangleStorage is safe to share across threads because:
// - Writes only happen from the main thread (single producer)
// - Each slot is written exactly once per frame before any reads
// - Reads happen after the write barrier (fetch_add with AcqRel)
unsafe impl Sync for TriangleStorage {}

impl TriangleStorage {
    const fn new() -> Self {
        // Create zeroed ScreenTriangle for initialization
        const EMPTY: ScreenTriangle = ScreenTriangle {
            x0: 0, y0: 0, z0: 0.0,
            x1: 0, y1: 0, z1: 0.0,
            x2: 0, y2: 0, z2: 0.0,
            a12: 0, b12: 0, c12: 0,
            a20: 0, b20: 0, c20: 0,
            a01: 0, b01: 0, c01: 0,
            min_x: 0, max_x: 0, min_y: 0, max_y: 0,
            inv_area: 0.0, is_cw: false,
            r0: 0, g0: 0, b0: 0,
            r1: 0, g1: 0, b1: 0,
            r2: 0, g2: 0, b2: 0,
        };
        Self {
            triangles: UnsafeCell::new([EMPTY; MAX_TRIANGLES_PER_FRAME]),
            count: AtomicUsize::new(0),
        }
    }

    /// Add a triangle (lock-free, single producer)
    #[inline]
    pub fn add(&self, tri: ScreenTriangle) -> Option<u16> {
        let idx = self.count.fetch_add(1, Ordering::AcqRel);
        if idx >= MAX_TRIANGLES_PER_FRAME {
            return None;
        }
        // Safety: idx is unique due to atomic increment, single producer
        unsafe {
            (*self.triangles.get())[idx] = tri;
        }
        Some(idx as u16)
    }

    /// Get a triangle by index (lock-free read)
    #[inline]
    pub fn get(&self, idx: u16) -> Option<ScreenTriangle> {
        let idx = idx as usize;
        if idx < self.count.load(Ordering::Acquire) {
            // Safety: idx is within bounds and data was written before count update
            Some(unsafe { (*self.triangles.get())[idx] })
        } else {
            None
        }
    }

    /// Get current triangle count
    #[inline]
    pub fn len(&self) -> usize {
        self.count.load(Ordering::Acquire)
    }

    /// Reset for new frame
    #[inline]
    pub fn reset(&self) {
        self.count.store(0, Ordering::Release);
    }
}

/// Global lock-free triangle storage
static TRIANGLE_STORAGE: TriangleStorage = TriangleStorage::new();

/// Atomic count of triangles (for backward compatibility)
pub static TRIANGLE_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Maximum number of tiles (for static allocation)
/// 512 tiles supports up to ~1600x1200 with 64x64 tiles (25*20=500)
const MAX_TILES: usize = 512;

/// Lock-free triangle bins (one per tile)
pub static TILE_BINS_LOCKFREE: [TileBinLockFree; MAX_TILES] = {
    const INIT: TileBinLockFree = TileBinLockFree::new();
    [INIT; MAX_TILES]
};

/// Initialize the frame triangle buffer (no-op for lock-free storage)
pub fn init_triangle_buffer() {
    TRIANGLE_STORAGE.reset();
    TRIANGLE_COUNT.store(0, Ordering::Release);
}

/// Reset triangle buffer for new frame (LOCK-FREE)
#[inline]
pub fn reset_triangle_buffer() {
    TRIANGLE_STORAGE.reset();
    TRIANGLE_COUNT.store(0, Ordering::Release);
}

/// Add a screen triangle to the frame buffer (LOCK-FREE)
/// Returns the triangle index, or None if buffer is full
#[inline]
pub fn add_triangle(tri: ScreenTriangle) -> Option<u16> {
    TRIANGLE_STORAGE.add(tri)
}

/// Get a triangle from the frame buffer (LOCK-FREE)
#[inline]
pub fn get_triangle(idx: u16) -> Option<ScreenTriangle> {
    TRIANGLE_STORAGE.get(idx)
}

/// Get the number of triangles in the current frame
#[inline]
pub fn triangle_count() -> usize {
    TRIANGLE_STORAGE.len()
}

/// Clear all lock-free bins
pub fn clear_lockfree_bins() {
    for bin in TILE_BINS_LOCKFREE.iter() {
        bin.clear();
    }
}

/// Cached tile grid dimensions (set once during init, read without locking)
static TILE_GRID_WIDTH: AtomicUsize = AtomicUsize::new(0);
static TILE_GRID_HEIGHT: AtomicUsize = AtomicUsize::new(0);

/// Set tile grid dimensions (call once during init)
pub fn set_tile_grid_dimensions(screen_width: usize, screen_height: usize) {
    let tiles_x = (screen_width + TILE_SIZE - 1) / TILE_SIZE;
    let tiles_y = (screen_height + TILE_SIZE - 1) / TILE_SIZE;
    TILE_GRID_WIDTH.store(tiles_x, Ordering::Release);
    TILE_GRID_HEIGHT.store(tiles_y, Ordering::Release);
}

/// Bin a triangle to appropriate tiles (TRULY lock-free version)
/// Computes tile indices directly from triangle bounds - no mutex needed
#[inline]
pub fn bin_triangle_lockfree(triangle_idx: u16, tri: &ScreenTriangle) {
    let tiles_x = TILE_GRID_WIDTH.load(Ordering::Acquire);
    if tiles_x == 0 {
        return; // Not initialized
    }

    // Compute which tiles this triangle overlaps
    let tile_min_x = (tri.min_x as usize) / TILE_SIZE;
    let tile_max_x = (tri.max_x as usize) / TILE_SIZE;
    let tile_min_y = (tri.min_y as usize) / TILE_SIZE;
    let tile_max_y = (tri.max_y as usize) / TILE_SIZE;

    // Clamp to valid tile range
    let tile_max_x = tile_max_x.min(tiles_x - 1);
    let tile_max_y = tile_max_y.min(TILE_GRID_HEIGHT.load(Ordering::Acquire) - 1);

    // Add to each overlapping tile's bin (no locking required)
    for ty in tile_min_y..=tile_max_y {
        let row_start = ty * tiles_x;
        for tx in tile_min_x..=tile_max_x {
            let tile_idx = row_start + tx;
            if tile_idx < MAX_TILES {
                TILE_BINS_LOCKFREE[tile_idx].add(triangle_idx);
            }
        }
    }
}

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

    /// Get the next tile index (for parallel work-stealing)
    pub fn get_next_tile_idx(&self) -> Option<usize> {
        let idx = self.next_tile.fetch_add(1, Ordering::Relaxed);
        if idx < self.tiles.len() {
            Some(idx)
        } else {
            None
        }
    }

    /// Get tile by index
    pub fn get_tile(&self, idx: usize) -> Option<&Tile> {
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
    // Set tile grid dimensions for lock-free binning
    set_tile_grid_dimensions(width, height);
    // Also initialize the triangle buffer for parallel rendering
    init_triangle_buffer();
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
