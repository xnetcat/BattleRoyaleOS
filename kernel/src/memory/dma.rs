//! DMA memory management for device drivers
//!
//! This module provides DMA-capable memory allocation by tracking physical pages
//! from the memory map and returning both physical addresses (for devices) and
//! HHDM-mapped virtual addresses (for CPU access).

use core::ptr::NonNull;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use spin::Mutex;

/// Page size for DMA allocations
pub const PAGE_SIZE: usize = 4096;

/// Maximum number of DMA pages we can track (1MB total DMA memory)
const MAX_DMA_PAGES: usize = 256;

/// A DMA-capable buffer
#[derive(Debug)]
pub struct DmaBuffer {
    pub virt: NonNull<u8>,
    pub phys: u64,
    pub size: usize,
}

// Safety: DMA buffers are designed to be shared between CPU and devices
unsafe impl Send for DmaBuffer {}
unsafe impl Sync for DmaBuffer {}

/// Simple physical page allocator for DMA
/// Uses a static array of page physical addresses
pub struct DmaAllocator {
    /// Physical addresses of available DMA pages
    pages: [AtomicU64; MAX_DMA_PAGES],
    /// Number of pages available
    count: AtomicUsize,
    /// Next page to allocate
    next: AtomicUsize,
}

impl DmaAllocator {
    pub const fn new() -> Self {
        // Initialize all pages to 0 (invalid)
        const ZERO: AtomicU64 = AtomicU64::new(0);
        Self {
            pages: [ZERO; MAX_DMA_PAGES],
            count: AtomicUsize::new(0),
            next: AtomicUsize::new(0),
        }
    }

    /// Add a physical page to the DMA pool
    pub fn add_page(&self, phys_addr: u64) {
        let idx = self.count.fetch_add(1, Ordering::SeqCst);
        if idx < MAX_DMA_PAGES {
            self.pages[idx].store(phys_addr, Ordering::SeqCst);
        }
    }

    /// Allocate a single DMA page
    /// Returns (physical_address, virtual_address) or None if out of pages
    pub fn alloc_page(&self, hhdm_offset: u64) -> Option<(u64, *mut u8)> {
        let idx = self.next.fetch_add(1, Ordering::SeqCst);
        let count = self.count.load(Ordering::SeqCst);

        if idx >= count {
            return None;
        }

        let phys = self.pages[idx].load(Ordering::SeqCst);
        if phys == 0 {
            return None;
        }

        let virt = (phys + hhdm_offset) as *mut u8;

        // Zero the page
        unsafe {
            core::ptr::write_bytes(virt, 0, PAGE_SIZE);
        }

        Some((phys, virt))
    }

    /// Allocate multiple contiguous pages (best effort - may not be physically contiguous)
    /// For E1000, we allocate individual descriptor buffers, so contiguous is not required
    pub fn alloc_pages(&self, count: usize, hhdm_offset: u64) -> Option<alloc::vec::Vec<(u64, *mut u8)>> {
        let mut pages = alloc::vec::Vec::with_capacity(count);
        for _ in 0..count {
            if let Some(page) = self.alloc_page(hhdm_offset) {
                pages.push(page);
            } else {
                // Out of pages
                return None;
            }
        }
        Some(pages)
    }

    /// Get the number of available pages
    pub fn available(&self) -> usize {
        let count = self.count.load(Ordering::SeqCst);
        let next = self.next.load(Ordering::SeqCst);
        count.saturating_sub(next)
    }
}

/// Global DMA allocator
pub static DMA_ALLOCATOR: DmaAllocator = DmaAllocator::new();

/// Get the HHDM offset (set during boot)
pub static HHDM_OFFSET: Mutex<u64> = Mutex::new(0);

/// Initialize the DMA allocator with pages from the memory map
/// Call this after setting HHDM_OFFSET
pub fn init_dma_pool(memory_map: &[&limine::memory_map::Entry], hhdm_offset: u64) {
    use crate::serial_println;

    let mut pages_added = 0;

    for entry in memory_map {
        if entry.entry_type != limine::memory_map::EntryType::USABLE {
            continue;
        }

        // Skip first 16MB (to avoid any legacy issues and ensure good DMA access)
        let start = entry.base.max(0x1000000);
        let end = entry.base + entry.length;

        if start >= end {
            continue;
        }

        // Align to page boundary
        let aligned_start = (start + PAGE_SIZE as u64 - 1) & !(PAGE_SIZE as u64 - 1);

        // Add pages from this region (limit to what we need)
        let mut addr = aligned_start;
        while addr + PAGE_SIZE as u64 <= end && pages_added < MAX_DMA_PAGES {
            DMA_ALLOCATOR.add_page(addr);
            pages_added += 1;
            addr += PAGE_SIZE as u64;

            // Stop after getting enough pages for E1000
            // (256 RX buffers + 128 TX buffers + descriptors = ~400 pages)
            if pages_added >= 400 {
                break;
            }
        }

        if pages_added >= 400 {
            break;
        }
    }

    serial_println!("DMA: Initialized {} pages ({} KB)", pages_added, pages_added * 4);
}

/// Allocate a DMA buffer of the given size
pub fn alloc_dma(size: usize) -> Option<DmaBuffer> {
    let hhdm = *HHDM_OFFSET.lock();

    // Round up to page size
    let pages_needed = (size + PAGE_SIZE - 1) / PAGE_SIZE;

    if pages_needed == 1 {
        let (phys, virt) = DMA_ALLOCATOR.alloc_page(hhdm)?;
        Some(DmaBuffer {
            virt: NonNull::new(virt)?,
            phys,
            size,
        })
    } else {
        // For larger allocations, we allocate multiple pages
        // Note: These may not be physically contiguous
        let pages = DMA_ALLOCATOR.alloc_pages(pages_needed, hhdm)?;
        // Return the first page - caller must handle multi-page specially
        let (phys, virt) = pages[0];
        Some(DmaBuffer {
            virt: NonNull::new(virt)?,
            phys,
            size: PAGE_SIZE, // Only first page
        })
    }
}

/// Allocate a single DMA page
pub fn alloc_dma_page() -> Option<(u64, *mut u8)> {
    let hhdm = *HHDM_OFFSET.lock();
    DMA_ALLOCATOR.alloc_page(hhdm)
}

/// Convert virtual address (in HHDM range) to physical address
pub fn virt_to_phys(virt: *const u8) -> u64 {
    let hhdm = *HHDM_OFFSET.lock();
    (virt as u64).wrapping_sub(hhdm)
}

/// Convert physical address to virtual address (via HHDM)
pub fn phys_to_virt(phys: u64) -> *mut u8 {
    let hhdm = *HHDM_OFFSET.lock();
    (phys + hhdm) as *mut u8
}
