//! Framebuffer wrapper for Limine with double buffering

use crate::boot::FRAMEBUFFER_REQUEST;
use alloc::vec::Vec;
use spin::Mutex;

/// Framebuffer information with double buffering support
pub struct Framebuffer {
    pub address: *mut u32,      // Front buffer (display)
    pub back_buffer: Vec<u32>,  // Back buffer (render target)
    pub width: usize,
    pub height: usize,
    pub pitch: usize, // Bytes per row
    pub bpp: u16,
}

impl Framebuffer {
    /// Create framebuffer from Limine response with back buffer
    pub fn from_limine() -> Option<Self> {
        let response = FRAMEBUFFER_REQUEST.get_response()?;
        let fb = response.framebuffers().next()?;

        let width = fb.width() as usize;
        let height = fb.height() as usize;
        let pitch = fb.pitch() as usize;

        // Allocate back buffer (same size as front buffer row stride)
        let row_pixels = pitch / 4;
        let back_buffer = alloc::vec![0u32; row_pixels * height];

        Some(Self {
            address: fb.addr() as *mut u32,
            back_buffer,
            width,
            height,
            pitch,
            bpp: fb.bpp(),
        })
    }

    /// Put a pixel at (x, y) with color - writes to BACK buffer
    #[inline]
    pub fn put_pixel(&self, x: usize, y: usize, color: u32) {
        if x < self.width && y < self.height {
            let offset = y * (self.pitch / 4) + x;
            // Safety: we're writing to our own back buffer within bounds
            unsafe {
                let ptr = self.back_buffer.as_ptr() as *mut u32;
                *ptr.add(offset) = color;
            }
        }
    }

    /// Get pixel at (x, y) from back buffer
    #[inline]
    pub fn get_pixel(&self, x: usize, y: usize) -> u32 {
        if x < self.width && y < self.height {
            let offset = y * (self.pitch / 4) + x;
            self.back_buffer[offset]
        } else {
            0
        }
    }

    /// Alias for put_pixel
    #[inline]
    pub fn set_pixel(&self, x: usize, y: usize, color: u32) {
        self.put_pixel(x, y, color);
    }

    /// Get pixel at linear index
    #[inline]
    pub fn pixel_at(&self, idx: usize) -> u32 {
        if idx < self.back_buffer.len() {
            self.back_buffer[idx]
        } else {
            0
        }
    }

    /// Set pixel at linear index
    #[inline]
    pub fn set_pixel_at(&self, idx: usize, color: u32) {
        if idx < self.back_buffer.len() {
            unsafe {
                let ptr = self.back_buffer.as_ptr() as *mut u32;
                *ptr.add(idx) = color;
            }
        }
    }

    /// Clear the back buffer with a color (optimized 64-bit writes)
    pub fn clear(&self, color: u32) {
        let row_pixels = self.pitch / 4;
        let total = row_pixels * self.height;
        let ptr = self.back_buffer.as_ptr() as *mut u64;
        let color64 = ((color as u64) << 32) | (color as u64);

        unsafe {
            for i in 0..(total / 2) {
                *ptr.add(i) = color64;
            }
            // Handle odd pixel if any
            if total % 2 == 1 {
                let ptr32 = self.back_buffer.as_ptr() as *mut u32;
                *ptr32.add(total - 1) = color;
            }
        }
    }

    /// Present: copy back buffer to front buffer (display)
    pub fn present(&self) {
        let row_pixels = self.pitch / 4;
        let total = row_pixels * self.height;

        unsafe {
            // Fast copy using 64-bit writes
            let src = self.back_buffer.as_ptr() as *const u64;
            let dst = self.address as *mut u64;

            for i in 0..(total / 2) {
                *dst.add(i) = *src.add(i);
            }
            // Handle odd pixel if any
            if total % 2 == 1 {
                let src32 = self.back_buffer.as_ptr() as *const u32;
                let dst32 = self.address;
                *dst32.add(total - 1) = *src32.add(total - 1);
            }
        }
    }

    /// Fill a rectangle
    pub fn fill_rect(&self, x: usize, y: usize, w: usize, h: usize, color: u32) {
        for dy in 0..h {
            for dx in 0..w {
                self.put_pixel(x + dx, y + dy, color);
            }
        }
    }

    /// Draw a horizontal line
    #[inline]
    pub fn hline(&self, x1: usize, x2: usize, y: usize, color: u32) {
        if y >= self.height {
            return;
        }
        let start = x1.min(x2).min(self.width);
        let end = x1.max(x2).min(self.width);
        for x in start..end {
            self.put_pixel(x, y, color);
        }
    }

    /// Get raw pointer to a scanline in the BACK buffer
    #[inline]
    pub unsafe fn scanline_ptr(&self, y: usize) -> *mut u32 {
        unsafe { (self.back_buffer.as_ptr() as *mut u32).add(y * (self.pitch / 4)) }
    }

    /// Get total pixel count
    pub fn pixel_count(&self) -> usize {
        self.width * self.height
    }
}

// Safety: The framebuffer is memory-mapped and access is coordinated through tiles
unsafe impl Send for Framebuffer {}
unsafe impl Sync for Framebuffer {}

/// Global framebuffer instance
pub static FRAMEBUFFER: Mutex<Option<Framebuffer>> = Mutex::new(None);

/// Initialize the framebuffer
pub fn init() -> Option<(usize, usize)> {
    let fb = Framebuffer::from_limine()?;
    let (w, h) = (fb.width, fb.height);
    *FRAMEBUFFER.lock() = Some(fb);
    Some((w, h))
}

/// Pack RGB values into a 32-bit color
#[inline]
pub const fn rgb(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

/// Interpolate between two colors
#[inline]
pub fn lerp_color(c1: u32, c2: u32, t: f32) -> u32 {
    let r1 = ((c1 >> 16) & 0xFF) as f32;
    let g1 = ((c1 >> 8) & 0xFF) as f32;
    let b1 = (c1 & 0xFF) as f32;

    let r2 = ((c2 >> 16) & 0xFF) as f32;
    let g2 = ((c2 >> 8) & 0xFF) as f32;
    let b2 = (c2 & 0xFF) as f32;

    let r = (r1 + (r2 - r1) * t) as u32;
    let g = (g1 + (g2 - g1) * t) as u32;
    let b = (b1 + (b2 - b1) * t) as u32;

    (r << 16) | (g << 8) | b
}
