//! Framebuffer wrapper for Limine

use crate::boot::FRAMEBUFFER_REQUEST;
use spin::Mutex;

/// Framebuffer information
pub struct Framebuffer {
    pub address: *mut u32,
    pub width: usize,
    pub height: usize,
    pub pitch: usize, // Bytes per row
    pub bpp: u16,
}

impl Framebuffer {
    /// Create framebuffer from Limine response
    pub fn from_limine() -> Option<Self> {
        let response = FRAMEBUFFER_REQUEST.get_response()?;
        let fb = response.framebuffers().next()?;

        Some(Self {
            address: fb.addr() as *mut u32,
            width: fb.width() as usize,
            height: fb.height() as usize,
            pitch: fb.pitch() as usize,
            bpp: fb.bpp(),
        })
    }

    /// Put a pixel at (x, y) with color
    #[inline]
    pub fn put_pixel(&self, x: usize, y: usize, color: u32) {
        if x < self.width && y < self.height {
            unsafe {
                let offset = y * (self.pitch / 4) + x;
                *self.address.add(offset) = color;
            }
        }
    }

    /// Get pixel at (x, y)
    #[inline]
    pub fn get_pixel(&self, x: usize, y: usize) -> u32 {
        if x < self.width && y < self.height {
            unsafe {
                let offset = y * (self.pitch / 4) + x;
                *self.address.add(offset)
            }
        } else {
            0
        }
    }

    /// Clear the framebuffer with a color
    pub fn clear(&self, color: u32) {
        for y in 0..self.height {
            for x in 0..self.width {
                self.put_pixel(x, y, color);
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

    /// Get raw pointer to a scanline
    #[inline]
    pub unsafe fn scanline_ptr(&self, y: usize) -> *mut u32 {
        self.address.add(y * (self.pitch / 4))
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
