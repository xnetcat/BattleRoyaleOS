//! Depth buffer for 3D rendering

use alloc::vec;
use alloc::vec::Vec;
use spin::Mutex;

/// Z-buffer for depth testing
pub struct ZBuffer {
    pub data: Vec<f32>,
    pub width: usize,
    pub height: usize,
}

impl ZBuffer {
    /// Create a new z-buffer
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            data: vec![f32::NEG_INFINITY; width * height],
            width,
            height,
        }
    }

    /// Clear the z-buffer
    pub fn clear(&mut self) {
        for z in &mut self.data {
            *z = f32::NEG_INFINITY;
        }
    }

    /// Test and set depth at (x, y)
    /// Returns true if the new depth is closer (should draw)
    /// Uses reversed depth: larger z = closer
    #[inline]
    pub fn test_and_set(&mut self, x: usize, y: usize, depth: f32) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        let idx = y * self.width + x;
        if depth > self.data[idx] {
            self.data[idx] = depth;
            true
        } else {
            false
        }
    }

    /// Get depth at (x, y)
    #[inline]
    pub fn get(&self, x: usize, y: usize) -> f32 {
        if x >= self.width || y >= self.height {
            return f32::INFINITY;
        }
        self.data[y * self.width + x]
    }

    /// Set depth at (x, y) without testing
    #[inline]
    pub fn set(&mut self, x: usize, y: usize, depth: f32) {
        if x < self.width && y < self.height {
            self.data[y * self.width + x] = depth;
        }
    }
}

/// Global z-buffer
pub static ZBUFFER: Mutex<Option<ZBuffer>> = Mutex::new(None);

/// Initialize the z-buffer
pub fn init(width: usize, height: usize) {
    *ZBUFFER.lock() = Some(ZBuffer::new(width, height));
}

/// Clear the z-buffer
pub fn clear() {
    if let Some(zb) = ZBUFFER.lock().as_mut() {
        zb.clear();
    }
}
