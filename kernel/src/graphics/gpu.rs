//! GPU backend abstraction layer
//!
//! This module provides a unified interface for graphics output, supporting
//! multiple backends:
//! - VMSVGA (VMware SVGA II) for hardware-accelerated rendering
//! - Software framebuffer (Limine) as a fallback
//!
//! The init() function automatically selects the best available backend.

use crate::drivers::vmsvga;
use crate::graphics::framebuffer::{self, Framebuffer, FRAMEBUFFER};
use crate::serial_println;
use spin::Mutex;

/// GPU backend type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GpuBackend {
    /// Software rendering via Limine framebuffer
    Software,
    /// Hardware-accelerated VMSVGA
    Vmsvga,
}

/// Currently active GPU backend
static ACTIVE_BACKEND: Mutex<GpuBackend> = Mutex::new(GpuBackend::Software);

/// Get the currently active backend
pub fn active_backend() -> GpuBackend {
    *ACTIVE_BACKEND.lock()
}

/// Initialize the GPU subsystem
///
/// Attempts to initialize VMSVGA first, falls back to Limine framebuffer.
/// Returns (width, height) on success.
pub fn init() -> (usize, usize) {
    // Try VMSVGA first
    if vmsvga::is_available() {
        serial_println!("GPU: VMSVGA device detected, attempting initialization...");
        if let Some((w, h)) = vmsvga::init() {
            *ACTIVE_BACKEND.lock() = GpuBackend::Vmsvga;
            serial_println!("GPU: Using VMSVGA backend {}x{}", w, h);

            // Also initialize the software framebuffer as it's used by the existing codebase
            // We'll sync the dimensions
            init_software_framebuffer_compat(w, h);

            return (w, h);
        }
        serial_println!("GPU: VMSVGA initialization failed, falling back to software");
    } else {
        serial_println!("GPU: VMSVGA device not available");
    }

    // Fall back to Limine framebuffer
    serial_println!("GPU: Using software rendering (Limine framebuffer)");
    *ACTIVE_BACKEND.lock() = GpuBackend::Software;

    if let Some((w, h)) = framebuffer::init() {
        serial_println!("GPU: Software framebuffer {}x{}", w, h);
        (w, h)
    } else {
        serial_println!("GPU: ERROR - No framebuffer available!");
        // Return a minimal resolution as fallback
        (640, 480)
    }
}

/// Initialize a compatibility software framebuffer for VMSVGA mode
///
/// The existing codebase uses FRAMEBUFFER directly. When VMSVGA is active,
/// Limine's framebuffer and VMSVGA share the same physical memory, so we just
/// need to send VMSVGA UPDATE commands after Limine's present().
fn init_software_framebuffer_compat(_width: usize, _height: usize) {
    // Initialize the Limine framebuffer - it shares physical memory with VMSVGA
    let _ = framebuffer::init();
}

/// Get framebuffer dimensions from the active backend
pub fn dimensions() -> (usize, usize) {
    match *ACTIVE_BACKEND.lock() {
        GpuBackend::Vmsvga => {
            let device = vmsvga::VMSVGA_DEVICE.lock();
            device.dimensions()
        }
        GpuBackend::Software => {
            let fb = FRAMEBUFFER.lock();
            if let Some(ref f) = *fb {
                (f.width, f.height)
            } else {
                (0, 0)
            }
        }
    }
}

/// Get framebuffer pitch (bytes per line)
pub fn pitch() -> usize {
    match *ACTIVE_BACKEND.lock() {
        GpuBackend::Vmsvga => {
            let device = vmsvga::VMSVGA_DEVICE.lock();
            device.pitch()
        }
        GpuBackend::Software => {
            let fb = FRAMEBUFFER.lock();
            if let Some(ref f) = *fb {
                f.pitch
            } else {
                0
            }
        }
    }
}

/// Present the back buffer to the display
///
/// This copies the back buffer to the front buffer and triggers
/// a screen update (for VMSVGA).
pub fn present() {
    // Always use Limine's present() to copy back buffer to front buffer.
    // Limine's front buffer is mapped with proper caching by the bootloader.
    {
        let fb = FRAMEBUFFER.lock();
        if let Some(ref f) = *fb {
            f.present();
        }
    }

    // If VMSVGA is active, send UPDATE command to refresh the display.
    // This tells VMSVGA that the framebuffer contents have changed.
    // Limine's framebuffer should be the same as VMSVGA's when -vga vmware is used.
    if *ACTIVE_BACKEND.lock() == GpuBackend::Vmsvga {
        let device = vmsvga::VMSVGA_DEVICE.lock();
        if device.is_initialized() {
            device.update_screen();
        }
    }
}

/// Clear the back buffer with a color
pub fn clear(color: u32) {
    match *ACTIVE_BACKEND.lock() {
        GpuBackend::Vmsvga => {
            let device = vmsvga::VMSVGA_DEVICE.lock();
            if device.is_initialized() {
                device.clear(color);
            }
        }
        GpuBackend::Software => {
            let fb = FRAMEBUFFER.lock();
            if let Some(ref f) = *fb {
                f.clear(color);
            }
        }
    }
}

/// Put a pixel at (x, y) with color - writes to back buffer
pub fn put_pixel(x: usize, y: usize, color: u32) {
    match *ACTIVE_BACKEND.lock() {
        GpuBackend::Vmsvga => {
            let device = vmsvga::VMSVGA_DEVICE.lock();
            if device.is_initialized() {
                device.put_pixel(x, y, color);
            }
        }
        GpuBackend::Software => {
            let fb = FRAMEBUFFER.lock();
            if let Some(ref f) = *fb {
                f.put_pixel(x, y, color);
            }
        }
    }
}

/// Get a pixel at (x, y) from the back buffer
pub fn get_pixel(x: usize, y: usize) -> u32 {
    match *ACTIVE_BACKEND.lock() {
        GpuBackend::Vmsvga => {
            let device = vmsvga::VMSVGA_DEVICE.lock();
            if device.is_initialized() {
                device.get_pixel(x, y)
            } else {
                0
            }
        }
        GpuBackend::Software => {
            let fb = FRAMEBUFFER.lock();
            if let Some(ref f) = *fb {
                f.get_pixel(x, y)
            } else {
                0
            }
        }
    }
}

/// Fill a rectangle
pub fn fill_rect(x: usize, y: usize, w: usize, h: usize, color: u32) {
    match *ACTIVE_BACKEND.lock() {
        GpuBackend::Vmsvga => {
            let device = vmsvga::VMSVGA_DEVICE.lock();
            if device.is_initialized() {
                device.fill_rect(x, y, w, h, color);
            }
        }
        GpuBackend::Software => {
            let fb = FRAMEBUFFER.lock();
            if let Some(ref f) = *fb {
                f.fill_rect(x, y, w, h, color);
            }
        }
    }
}

/// Check if the GPU subsystem is initialized
pub fn is_initialized() -> bool {
    match *ACTIVE_BACKEND.lock() {
        GpuBackend::Vmsvga => {
            let device = vmsvga::VMSVGA_DEVICE.lock();
            device.is_initialized()
        }
        GpuBackend::Software => {
            let fb = FRAMEBUFFER.lock();
            fb.is_some()
        }
    }
}

/// Get a string describing the current backend
pub fn backend_name() -> &'static str {
    match *ACTIVE_BACKEND.lock() {
        GpuBackend::Vmsvga => "VMSVGA (hardware)",
        GpuBackend::Software => "Software (Limine)",
    }
}
