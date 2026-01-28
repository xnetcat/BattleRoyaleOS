//! Guest Memory Region (GMR) management for VMSVGA
//!
//! GMRs allow the guest to register regions of physical memory that
//! can be used for DMA transfers to/from GPU surfaces.

use crate::memory::dma::{alloc_dma, DmaBuffer, PAGE_SIZE};
use crate::serial_println;
use alloc::vec::Vec;
use spin::Mutex;

use super::regs::{self, SvgaReg};

/// Maximum number of GMRs we'll use
pub const MAX_GMRS: usize = 16;

/// GMR descriptor for a contiguous physical memory region
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GmrDescriptor {
    /// Physical page number (PPN)
    pub ppn: u32,
    /// Number of pages
    pub num_pages: u32,
}

/// A registered GMR
pub struct Gmr {
    /// GMR ID
    pub id: u32,
    /// Physical address
    pub phys_addr: u64,
    /// Virtual address
    pub virt_addr: u64,
    /// Size in bytes
    pub size: usize,
    /// Whether this GMR is in use
    pub in_use: bool,
}

/// GMR manager
pub struct GmrManager {
    /// Allocated GMRs
    gmrs: Vec<Option<Gmr>>,
    /// Next GMR ID to allocate
    next_id: u32,
    /// I/O base for register access
    io_base: u16,
    /// Whether GMR is supported
    pub supported: bool,
    /// Whether GMR2 is supported (preferred)
    pub gmr2_supported: bool,
}

impl GmrManager {
    pub const fn new() -> Self {
        Self {
            gmrs: Vec::new(),
            next_id: 1, // GMR ID 0 is often special
            io_base: 0,
            supported: false,
            gmr2_supported: false,
        }
    }

    /// Initialize the GMR manager
    pub fn init(&mut self, io_base: u16, caps: u32) {
        self.io_base = io_base;
        self.supported = regs::has_capability(caps, regs::cap::GMR);
        self.gmr2_supported = regs::has_capability(caps, regs::cap::GMR2);

        // Pre-allocate GMR slots
        self.gmrs = Vec::with_capacity(MAX_GMRS);
        for _ in 0..MAX_GMRS {
            self.gmrs.push(None);
        }

        if self.gmr2_supported {
            serial_println!("GMR: GMR2 supported");
        } else if self.supported {
            serial_println!("GMR: Legacy GMR supported");
        } else {
            serial_println!("GMR: Not supported");
        }
    }

    /// Allocate a new GMR with the given size
    /// Returns the GMR ID on success
    pub fn alloc(&mut self, size: usize) -> Option<u32> {
        if !self.supported && !self.gmr2_supported {
            return None;
        }

        // Find a free slot
        let slot = self.gmrs.iter().position(|g| g.is_none())?;

        // Allocate DMA buffer
        let buffer = alloc_dma(size)?;

        let id = self.next_id;
        self.next_id += 1;

        let phys = buffer.phys;
        let virt = buffer.virt.as_ptr() as u64;

        // Register the GMR with the device
        if self.gmr2_supported {
            self.register_gmr2(id, phys, size);
        } else {
            self.register_gmr(id, phys, size);
        }

        let gmr = Gmr {
            id,
            phys_addr: phys,
            virt_addr: virt,
            size,
            in_use: true,
        };

        self.gmrs[slot] = Some(gmr);
        Some(id)
    }

    /// Register a GMR using legacy method
    fn register_gmr(&self, id: u32, phys_addr: u64, size: usize) {
        let num_pages = (size + PAGE_SIZE - 1) / PAGE_SIZE;

        // Create descriptor
        let desc = GmrDescriptor {
            ppn: (phys_addr / PAGE_SIZE as u64) as u32,
            num_pages: num_pages as u32,
        };

        // Write GMR ID
        regs::write_reg(self.io_base, SvgaReg::GmrId, id);

        // Write descriptor (physical address of descriptor)
        // For single-region GMR, we write the descriptor inline
        // Format: [ppn, num_pages] terminated by [0, 0]
        let desc_ptr = &desc as *const GmrDescriptor as u64;
        regs::write_reg(self.io_base, SvgaReg::GmrDescriptor, desc_ptr as u32);
    }

    /// Register a GMR using GMR2 method (via FIFO command)
    fn register_gmr2(&self, id: u32, _phys_addr: u64, _size: usize) {
        // GMR2 uses FIFO commands instead of registers
        // We'll handle this via the FIFO cmd_define_gmr2
        let _ = id;
    }

    /// Get a GMR by ID
    pub fn get(&self, id: u32) -> Option<&Gmr> {
        self.gmrs.iter().flatten().find(|g| g.id == id)
    }

    /// Get a mutable GMR by ID
    pub fn get_mut(&mut self, id: u32) -> Option<&mut Gmr> {
        self.gmrs.iter_mut().flatten().find(|g| g.id == id)
    }

    /// Free a GMR
    pub fn free(&mut self, id: u32) -> bool {
        if let Some(slot) = self.gmrs.iter().position(|g| g.as_ref().map(|x| x.id) == Some(id)) {
            self.gmrs[slot] = None;
            true
        } else {
            false
        }
    }

    /// Get the physical address of a GMR's buffer
    pub fn get_phys_addr(&self, id: u32) -> Option<u64> {
        self.get(id).map(|g| g.phys_addr)
    }

    /// Get a pointer to write data into a GMR
    pub fn get_write_ptr(&self, id: u32) -> Option<*mut u8> {
        self.get(id).map(|g| g.virt_addr as *mut u8)
    }

    /// Get GMR size
    pub fn get_size(&self, id: u32) -> Option<usize> {
        self.get(id).map(|g| g.size)
    }
}

/// Global GMR manager
pub static GMR_MANAGER: Mutex<GmrManager> = Mutex::new(GmrManager::new());

/// Initialize GMR support
pub fn init(io_base: u16, caps: u32) {
    GMR_MANAGER.lock().init(io_base, caps);
}

/// Allocate a GMR
pub fn alloc(size: usize) -> Option<u32> {
    GMR_MANAGER.lock().alloc(size)
}

/// Free a GMR
pub fn free(id: u32) -> bool {
    GMR_MANAGER.lock().free(id)
}

/// Get physical address of GMR
pub fn get_phys_addr(id: u32) -> Option<u64> {
    GMR_MANAGER.lock().get_phys_addr(id)
}

/// Get write pointer for GMR
pub fn get_write_ptr(id: u32) -> Option<*mut u8> {
    GMR_MANAGER.lock().get_write_ptr(id)
}

/// Get GMR size
pub fn get_size(id: u32) -> Option<usize> {
    GMR_MANAGER.lock().get_size(id)
}

/// Check if GMR is supported
pub fn is_supported() -> bool {
    let mgr = GMR_MANAGER.lock();
    mgr.supported || mgr.gmr2_supported
}
