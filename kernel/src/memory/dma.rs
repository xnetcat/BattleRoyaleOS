//! DMA memory management for device drivers

use alloc::vec::Vec;
use core::ptr::NonNull;
use spin::Mutex;

/// DMA buffer pool for network driver
pub struct DmaPool {
    buffers: Vec<DmaBuffer>,
}

/// A DMA-capable buffer
pub struct DmaBuffer {
    pub virt: NonNull<u8>,
    pub phys: u64,
    pub size: usize,
}

// Safety: DMA buffers are designed to be shared between CPU and devices
unsafe impl Send for DmaBuffer {}
unsafe impl Sync for DmaBuffer {}

impl DmaPool {
    pub const fn new() -> Self {
        Self {
            buffers: Vec::new(),
        }
    }

    /// Allocate a DMA buffer
    /// Note: In a real implementation, we'd need to ensure the physical address
    /// is accessible by the device. With HHDM, we can compute physical from virtual.
    pub fn alloc(&mut self, size: usize, hhdm_offset: u64) -> Option<DmaBuffer> {
        let layout = core::alloc::Layout::from_size_align(size, 4096).ok()?;
        let ptr = unsafe { alloc::alloc::alloc_zeroed(layout) };
        let virt = NonNull::new(ptr)?;

        // Calculate physical address from virtual address using HHDM offset
        let phys = (ptr as u64).wrapping_sub(hhdm_offset);

        Some(DmaBuffer { virt, phys, size })
    }
}

/// Global DMA pool
pub static DMA_POOL: Mutex<DmaPool> = Mutex::new(DmaPool::new());

/// Get the HHDM offset (set during boot)
pub static HHDM_OFFSET: Mutex<u64> = Mutex::new(0);

/// Allocate a DMA buffer
pub fn alloc_dma(size: usize) -> Option<DmaBuffer> {
    let hhdm = *HHDM_OFFSET.lock();
    DMA_POOL.lock().alloc(size, hhdm)
}

/// Convert virtual address to physical address
pub fn virt_to_phys(virt: *const u8) -> u64 {
    let hhdm = *HHDM_OFFSET.lock();
    (virt as u64).wrapping_sub(hhdm)
}

/// Convert physical address to virtual address
pub fn phys_to_virt(phys: u64) -> *mut u8 {
    let hhdm = *HHDM_OFFSET.lock();
    (phys + hhdm) as *mut u8
}
