//! GPU backend abstraction layer
//!
//! This module provides a unified interface for graphics output, supporting
//! multiple backends:
//! - SVGA3D for true GPU 3D hardware acceleration
//! - VMSVGA (VMware SVGA II) for 2D hardware-accelerated display
//! - Software framebuffer (Limine) as a fallback
//!
//! The init() function automatically selects the best available backend.

use crate::drivers::vmsvga;
use crate::graphics::framebuffer::{self, Framebuffer, FRAMEBUFFER};
use crate::graphics::gpu3d;
use crate::serial_println;
use spin::Mutex;

/// GPU backend type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GpuBackend {
    /// Software rendering via Limine framebuffer
    Software,
    /// Hardware-accelerated VMSVGA (2D acceleration only)
    Vmsvga,
    /// SVGA3D (true GPU 3D rasterization)
    Svga3D,
}

/// Currently active GPU backend
static ACTIVE_BACKEND: Mutex<GpuBackend> = Mutex::new(GpuBackend::Software);

/// Get the currently active backend
pub fn active_backend() -> GpuBackend {
    *ACTIVE_BACKEND.lock()
}

/// Initialize the GPU subsystem
///
/// Attempts to initialize SVGA3D first, then VMSVGA, falls back to Limine framebuffer.
/// Returns (width, height) on success.
pub fn init() -> (usize, usize) {
    // Try VMSVGA first (required for SVGA3D)
    if vmsvga::is_available() {
        serial_println!("GPU: VMSVGA device detected, attempting initialization...");
        if let Some((w, h)) = vmsvga::init() {
            // VMSVGA 2D is now active, try to enable SVGA3D
            if vmsvga::is_3d_available() {
                // Try to initialize GPU3D rendering
                if gpu3d::init(w as u32, h as u32) {
                    *ACTIVE_BACKEND.lock() = GpuBackend::Svga3D;
                    serial_println!("GPU: Using SVGA3D backend (true GPU 3D rasterization) {}x{}", w, h);
                } else {
                    *ACTIVE_BACKEND.lock() = GpuBackend::Vmsvga;
                    serial_println!("GPU: SVGA3D init failed, using VMSVGA 2D backend {}x{}", w, h);
                }
            } else {
                *ACTIVE_BACKEND.lock() = GpuBackend::Vmsvga;
                serial_println!("GPU: SVGA3D not available, using VMSVGA 2D backend {}x{}", w, h);
            }

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
        GpuBackend::Svga3D | GpuBackend::Vmsvga => {
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
        GpuBackend::Svga3D | GpuBackend::Vmsvga => {
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
    let backend = *ACTIVE_BACKEND.lock();

    // For SVGA3D, use the GPU 3D end_frame which presents the render target
    if backend == GpuBackend::Svga3D && gpu3d::is_ready() {
        gpu3d::end_frame();
        return;
    }

    // For VMSVGA and Software, use Limine's present() to copy back buffer to front buffer.
    // Limine's front buffer is mapped with proper caching by the bootloader.
    {
        let fb = FRAMEBUFFER.lock();
        if let Some(ref f) = *fb {
            f.present();
        }
    }

    // If VMSVGA is active (but not SVGA3D), send UPDATE command to refresh the display.
    // This tells VMSVGA that the framebuffer contents have changed.
    // Limine's framebuffer should be the same as VMSVGA's when -vga vmware is used.
    if backend == GpuBackend::Vmsvga {
        let device = vmsvga::VMSVGA_DEVICE.lock();
        if device.is_initialized() {
            device.update_screen();
        }
    }
}

/// Clear the back buffer with a color
pub fn clear(color: u32) {
    match *ACTIVE_BACKEND.lock() {
        GpuBackend::Svga3D => {
            // For SVGA3D, clear the GPU render target
            if gpu3d::is_ready() {
                gpu3d::clear(color, 1.0);
            }
        }
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
/// Note: For SVGA3D, 2D operations still use the software path for UI compatibility
pub fn put_pixel(x: usize, y: usize, color: u32) {
    match *ACTIVE_BACKEND.lock() {
        GpuBackend::Svga3D | GpuBackend::Vmsvga => {
            // For SVGA3D and VMSVGA, use the software framebuffer for 2D ops
            let fb = FRAMEBUFFER.lock();
            if let Some(ref f) = *fb {
                f.put_pixel(x, y, color);
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
        GpuBackend::Svga3D | GpuBackend::Vmsvga => {
            // For SVGA3D and VMSVGA, use the software framebuffer for 2D ops
            let fb = FRAMEBUFFER.lock();
            if let Some(ref f) = *fb {
                f.get_pixel(x, y)
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
/// Note: For SVGA3D, 2D operations still use the software path for UI compatibility
pub fn fill_rect(x: usize, y: usize, w: usize, h: usize, color: u32) {
    match *ACTIVE_BACKEND.lock() {
        GpuBackend::Svga3D | GpuBackend::Vmsvga => {
            // For SVGA3D and VMSVGA, use the software framebuffer for 2D ops
            let fb = FRAMEBUFFER.lock();
            if let Some(ref f) = *fb {
                f.fill_rect(x, y, w, h, color);
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
        GpuBackend::Svga3D => {
            gpu3d::is_ready()
        }
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
        GpuBackend::Svga3D => "SVGA3D (GPU 3D)",
        GpuBackend::Vmsvga => "VMSVGA (2D accel)",
        GpuBackend::Software => "Software (Limine)",
    }
}

/// Check if GPU 3D rendering is available
pub fn has_3d() -> bool {
    *ACTIVE_BACKEND.lock() == GpuBackend::Svga3D
}

/// Check if any hardware acceleration is available
pub fn has_hw_accel() -> bool {
    let backend = *ACTIVE_BACKEND.lock();
    backend == GpuBackend::Svga3D || backend == GpuBackend::Vmsvga
}
