//! VMSVGA (VMware SVGA II) GPU driver
//!
//! This driver provides hardware-accelerated graphics for VMware-compatible
//! virtual machines, including QEMU (-vga vmware) and VirtualBox (VMSVGA adapter).
//!
//! Features:
//! - Hardware framebuffer with configurable resolution
//! - FIFO command buffer for accelerated operations
//! - Screen update commands for efficient display refresh
//! - Rectangle fill and copy acceleration (when supported)

pub mod fifo;
pub mod regs;

use crate::drivers::pci::{self, PciDevice};
use crate::memory::paging;
use crate::serial_println;
use alloc::vec::Vec;
use fifo::VmsvgaFifo;
use regs::{SvgaReg, VMSVGA_DEVICE_ID, VMWARE_VENDOR_ID};
use spin::Mutex;

/// VMSVGA device state
pub struct VmsvgaDevice {
    /// I/O port base address
    io_base: u16,
    /// Virtual address of framebuffer
    fb_virt: u64,
    /// Framebuffer size in bytes
    fb_size: usize,
    /// Display width in pixels
    width: u32,
    /// Display height in pixels
    height: u32,
    /// Bits per pixel
    bpp: u32,
    /// Bytes per line (pitch)
    pitch: u32,
    /// Device capabilities
    capabilities: u32,
    /// FIFO command buffer
    fifo: VmsvgaFifo,
    /// Back buffer for double buffering
    back_buffer: Vec<u32>,
    /// Whether the device is initialized
    initialized: bool,
}

// Safety: Device state is protected by mutex
unsafe impl Send for VmsvgaDevice {}
unsafe impl Sync for VmsvgaDevice {}

impl VmsvgaDevice {
    /// Create an uninitialized device
    pub const fn new() -> Self {
        Self {
            io_base: 0,
            fb_virt: 0,
            fb_size: 0,
            width: 0,
            height: 0,
            bpp: 0,
            pitch: 0,
            capabilities: 0,
            fifo: VmsvgaFifo::new(),
            back_buffer: Vec::new(),
            initialized: false,
        }
    }

    /// Check if device is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Get framebuffer dimensions
    pub fn dimensions(&self) -> (usize, usize) {
        (self.width as usize, self.height as usize)
    }

    /// Get framebuffer pitch (bytes per line)
    pub fn pitch(&self) -> usize {
        self.pitch as usize
    }

    /// Get bits per pixel
    pub fn bpp(&self) -> u16 {
        self.bpp as u16
    }

    /// Get device capabilities
    pub fn capabilities(&self) -> u32 {
        self.capabilities
    }

    /// Get pointer to the front buffer (hardware framebuffer)
    pub fn front_buffer(&self) -> *mut u32 {
        self.fb_virt as *mut u32
    }

    /// Get pointer to the back buffer
    pub fn back_buffer(&self) -> *mut u32 {
        self.back_buffer.as_ptr() as *mut u32
    }

    /// Get back buffer as slice
    pub fn back_buffer_slice(&self) -> &[u32] {
        &self.back_buffer
    }

    /// Put a pixel in the back buffer
    #[inline]
    pub fn put_pixel(&self, x: usize, y: usize, color: u32) {
        if x < self.width as usize && y < self.height as usize {
            let offset = y * (self.pitch as usize / 4) + x;
            unsafe {
                let ptr = self.back_buffer.as_ptr() as *mut u32;
                *ptr.add(offset) = color;
            }
        }
    }

    /// Get a pixel from the back buffer
    #[inline]
    pub fn get_pixel(&self, x: usize, y: usize) -> u32 {
        if x < self.width as usize && y < self.height as usize {
            let offset = y * (self.pitch as usize / 4) + x;
            self.back_buffer[offset]
        } else {
            0
        }
    }

    /// Clear the back buffer with a color
    pub fn clear(&self, color: u32) {
        let row_pixels = self.pitch as usize / 4;
        let total = row_pixels * self.height as usize;
        let ptr = self.back_buffer.as_ptr() as *mut u64;
        let color64 = ((color as u64) << 32) | (color as u64);

        unsafe {
            for i in 0..(total / 2) {
                *ptr.add(i) = color64;
            }
            if total % 2 == 1 {
                let ptr32 = self.back_buffer.as_ptr() as *mut u32;
                *ptr32.add(total - 1) = color;
            }
        }
    }

    /// Present: copy back buffer to front buffer and trigger screen update
    pub fn present(&self) {
        let row_pixels = self.pitch as usize / 4;
        let total = row_pixels * self.height as usize;

        // Copy back buffer to front buffer
        unsafe {
            let src = self.back_buffer.as_ptr() as *const u64;
            let dst = self.fb_virt as *mut u64;

            for i in 0..(total / 2) {
                *dst.add(i) = *src.add(i);
            }
            if total % 2 == 1 {
                let src32 = self.back_buffer.as_ptr() as *const u32;
                let dst32 = self.fb_virt as *mut u32;
                *dst32.add(total - 1) = *src32.add(total - 1);
            }
        }

        // Trigger screen update via FIFO
        self.fifo.cmd_update_full(self.width, self.height);
    }

    /// Trigger a screen update (call after writing to front buffer directly)
    pub fn update_screen(&self) {
        self.fifo.cmd_update_full(self.width, self.height);
    }

    /// Fill a rectangle in the back buffer
    pub fn fill_rect(&self, x: usize, y: usize, w: usize, h: usize, color: u32) {
        for dy in 0..h {
            for dx in 0..w {
                self.put_pixel(x + dx, y + dy, color);
            }
        }
    }

    /// Get scanline pointer in the back buffer
    #[inline]
    pub unsafe fn scanline_ptr(&self, y: usize) -> *mut u32 {
        (self.back_buffer.as_ptr() as *mut u32).add(y * (self.pitch as usize / 4))
    }

    /// Get pixel at linear index from back buffer
    #[inline]
    pub fn pixel_at(&self, idx: usize) -> u32 {
        if idx < self.back_buffer.len() {
            self.back_buffer[idx]
        } else {
            0
        }
    }

    /// Set pixel at linear index in back buffer
    #[inline]
    pub fn set_pixel_at(&self, idx: usize, color: u32) {
        if idx < self.back_buffer.len() {
            unsafe {
                let ptr = self.back_buffer.as_ptr() as *mut u32;
                *ptr.add(idx) = color;
            }
        }
    }

    /// Draw horizontal line in back buffer
    #[inline]
    pub fn hline(&self, x1: usize, x2: usize, y: usize, color: u32) {
        if y >= self.height as usize {
            return;
        }
        let start = x1.min(x2).min(self.width as usize);
        let end = x1.max(x2).min(self.width as usize);
        for x in start..end {
            self.put_pixel(x, y, color);
        }
    }
}

/// Global VMSVGA device instance
pub static VMSVGA_DEVICE: Mutex<VmsvgaDevice> = Mutex::new(VmsvgaDevice::new());

/// Check if VMSVGA device is available without initializing
pub fn is_available() -> bool {
    pci::find_device(VMWARE_VENDOR_ID, VMSVGA_DEVICE_ID).is_some()
}

/// Find the VMSVGA PCI device
fn find_device() -> Option<PciDevice> {
    pci::find_device(VMWARE_VENDOR_ID, VMSVGA_DEVICE_ID)
}

/// Initialize the VMSVGA driver
/// Returns (width, height) on success
pub fn init() -> Option<(usize, usize)> {
    // Find PCI device
    let pci_dev = match find_device() {
        Some(dev) => dev,
        None => {
            serial_println!("VMSVGA: Device not found");
            return None;
        }
    };


    // Enable PCI bus mastering and memory space
    pci_dev.enable_bus_master();
    pci_dev.enable_memory_space();

    // BAR0 is I/O space (bit 0 set indicates I/O)
    // The actual I/O base is BAR0 with bit 0 cleared
    let io_base = (pci_dev.bar0 & 0xFFFFFFFE) as u16;

    // Negotiate SVGA version
    let version = match regs::negotiate_version(io_base) {
        Some(v) => v,
        None => {
            serial_println!("VMSVGA: Version negotiation failed");
            return None;
        }
    };

    // Read device capabilities
    let capabilities = regs::read_reg(io_base, SvgaReg::Capabilities);

    // Get framebuffer info
    let fb_phys = regs::read_reg(io_base, SvgaReg::FbStart) as u64;
    let fb_size = regs::read_reg(io_base, SvgaReg::FbSize) as usize;
    let vram_size = regs::read_reg(io_base, SvgaReg::VramSize) as usize;

    // Get FIFO info
    let fifo_phys = regs::read_reg(io_base, SvgaReg::MemStart) as u64;
    let fifo_size = regs::read_reg(io_base, SvgaReg::MemSize) as usize;

    // Get maximum resolution
    let max_width = regs::read_reg(io_base, SvgaReg::MaxWidth);
    let max_height = regs::read_reg(io_base, SvgaReg::MaxHeight);

    // Choose a target resolution (prefer 1024x768, fall back to lower)
    let (target_width, target_height) = if max_width >= 1024 && max_height >= 768 {
        (1024, 768)
    } else if max_width >= 800 && max_height >= 600 {
        (800, 600)
    } else {
        (640, 480)
    };

    // Map framebuffer into kernel address space
    let fb_virt = match paging::map_mmio(fb_phys, fb_size) {
        Some(virt) => virt,
        None => {
            serial_println!("VMSVGA: Failed to map framebuffer");
            return None;
        }
    };
    // Map FIFO into kernel address space
    let fifo_virt = match paging::map_mmio(fifo_phys, fifo_size) {
        Some(virt) => virt,
        None => {
            serial_println!("VMSVGA: Failed to map FIFO");
            return None;
        }
    };

    // Set display mode
    regs::write_reg(io_base, SvgaReg::Width, target_width);
    regs::write_reg(io_base, SvgaReg::Height, target_height);
    regs::write_reg(io_base, SvgaReg::BitsPerPixel, 32);

    // Enable SVGA mode
    regs::write_reg(io_base, SvgaReg::Enable, 1);

    // Read back actual settings
    let width = regs::read_reg(io_base, SvgaReg::Width);
    let height = regs::read_reg(io_base, SvgaReg::Height);
    let bpp = regs::read_reg(io_base, SvgaReg::BitsPerPixel);
    let pitch = regs::read_reg(io_base, SvgaReg::BytesPerLine);


    // Allocate back buffer
    let row_pixels = pitch as usize / 4;
    let back_buffer = alloc::vec![0u32; row_pixels * height as usize];

    // Initialize device state
    let mut device = VMSVGA_DEVICE.lock();
    device.io_base = io_base;
    device.fb_virt = fb_virt;
    device.fb_size = fb_size;
    device.width = width;
    device.height = height;
    device.bpp = bpp;
    device.pitch = pitch;
    device.capabilities = capabilities;
    device.back_buffer = back_buffer;

    // Initialize FIFO
    device.fifo.init(fifo_virt, fifo_size, io_base, capabilities);

    // Signal configuration done
    regs::write_reg(io_base, SvgaReg::ConfigDone, 1);

    device.initialized = true;

    serial_println!(
        "VMSVGA: Initialized {}x{}x{}",
        width,
        height,
        bpp
    );

    Some((width as usize, height as usize))
}
