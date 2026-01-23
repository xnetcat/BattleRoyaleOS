//! Page table manipulation for MMIO mapping
//!
//! This module provides functions to map device MMIO regions into the kernel's
//! address space with proper caching attributes (uncached/write-combining).

use crate::serial_println;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::PageTableFlags;

/// Page size (4KB)
pub const PAGE_SIZE: u64 = 4096;

/// Base address for MMIO mappings (use a dedicated region in kernel space)
/// We'll use addresses starting at 0xFFFF_FFFF_8000_0000 + 256MB
const MMIO_MAP_BASE: u64 = 0xFFFF_FFFF_9000_0000;

/// Counter for allocating MMIO virtual addresses
static MMIO_NEXT_ADDR: AtomicU64 = AtomicU64::new(MMIO_MAP_BASE);

/// HHDM offset (set during boot)
static HHDM_OFFSET: Mutex<u64> = Mutex::new(0);

/// Set the HHDM offset (call once during boot)
pub fn set_hhdm_offset(offset: u64) {
    *HHDM_OFFSET.lock() = offset;
}

/// Get the current page table root physical address
fn get_cr3() -> u64 {
    let (frame, _) = Cr3::read();
    frame.start_address().as_u64()
}

/// Convert physical address to virtual using HHDM
fn phys_to_virt(phys: u64) -> *mut u64 {
    let hhdm = *HHDM_OFFSET.lock();
    (phys + hhdm) as *mut u64
}

/// Page table entry indices from virtual address
fn pml4_index(virt: u64) -> usize {
    ((virt >> 39) & 0x1FF) as usize
}

fn pdpt_index(virt: u64) -> usize {
    ((virt >> 30) & 0x1FF) as usize
}

fn pd_index(virt: u64) -> usize {
    ((virt >> 21) & 0x1FF) as usize
}

fn pt_index(virt: u64) -> usize {
    ((virt >> 12) & 0x1FF) as usize
}

/// Flags for MMIO pages: present, writable, no-execute, uncached
const MMIO_FLAGS: u64 = PageTableFlags::PRESENT.bits()
    | PageTableFlags::WRITABLE.bits()
    | PageTableFlags::NO_EXECUTE.bits()
    | PageTableFlags::NO_CACHE.bits()
    | PageTableFlags::WRITE_THROUGH.bits();

/// Allocate a page for page tables from DMA pool
fn alloc_page_table_page() -> Option<u64> {
    use crate::memory::dma::alloc_dma_page;
    let (phys, _virt) = alloc_dma_page()?;
    Some(phys)
}

/// Map a physical MMIO address to a virtual address
/// Returns the virtual address that can be used to access the device
pub fn map_mmio(phys_addr: u64, size: usize) -> Option<u64> {
    let hhdm = *HHDM_OFFSET.lock();
    if hhdm == 0 {
        serial_println!("PAGING: HHDM not set!");
        return None;
    }

    // Align physical address to page boundary
    let phys_aligned = phys_addr & !0xFFF;
    let offset_in_page = phys_addr & 0xFFF;

    // Calculate number of pages needed
    let pages_needed = ((size as u64 + offset_in_page + PAGE_SIZE - 1) / PAGE_SIZE) as usize;

    // Allocate virtual address space
    let virt_base = MMIO_NEXT_ADDR.fetch_add(pages_needed as u64 * PAGE_SIZE, Ordering::SeqCst);

    serial_println!(
        "PAGING: Mapping MMIO {:#x} -> {:#x} ({} pages)",
        phys_aligned,
        virt_base,
        pages_needed
    );

    // Get CR3 (PML4 physical address)
    let pml4_phys = get_cr3();
    let pml4 = phys_to_virt(pml4_phys);

    // Map each page
    for i in 0..pages_needed {
        let virt = virt_base + i as u64 * PAGE_SIZE;
        let phys = phys_aligned + i as u64 * PAGE_SIZE;

        if !map_page(pml4, virt, phys, hhdm) {
            serial_println!("PAGING: Failed to map page {:#x} -> {:#x}", virt, phys);
            return None;
        }
    }

    // Return virtual address with original offset
    Some(virt_base + offset_in_page)
}

/// Map a single 4KB page
fn map_page(pml4: *mut u64, virt: u64, phys: u64, hhdm: u64) -> bool {
    // Get PML4 entry
    let pml4e = unsafe { pml4.add(pml4_index(virt)) };
    let pml4_entry = unsafe { core::ptr::read_volatile(pml4e) };

    // Get or create PDPT
    let pdpt_phys = if pml4_entry & PageTableFlags::PRESENT.bits() != 0 {
        pml4_entry & 0x000F_FFFF_FFFF_F000
    } else {
        // Allocate new PDPT
        let new_pdpt = match alloc_page_table_page() {
            Some(p) => p,
            None => return false,
        };
        // Clear the new page table
        let pdpt_virt = (new_pdpt + hhdm) as *mut u8;
        unsafe {
            core::ptr::write_bytes(pdpt_virt, 0, PAGE_SIZE as usize);
        }
        // Write PML4 entry
        unsafe {
            core::ptr::write_volatile(
                pml4e,
                new_pdpt | PageTableFlags::PRESENT.bits() | PageTableFlags::WRITABLE.bits(),
            );
        }
        new_pdpt
    };

    let pdpt = (pdpt_phys + hhdm) as *mut u64;
    let pdpte = unsafe { pdpt.add(pdpt_index(virt)) };
    let pdpt_entry = unsafe { core::ptr::read_volatile(pdpte) };

    // Get or create PD
    let pd_phys = if pdpt_entry & PageTableFlags::PRESENT.bits() != 0 {
        pdpt_entry & 0x000F_FFFF_FFFF_F000
    } else {
        let new_pd = match alloc_page_table_page() {
            Some(p) => p,
            None => return false,
        };
        let pd_virt = (new_pd + hhdm) as *mut u8;
        unsafe {
            core::ptr::write_bytes(pd_virt, 0, PAGE_SIZE as usize);
        }
        unsafe {
            core::ptr::write_volatile(
                pdpte,
                new_pd | PageTableFlags::PRESENT.bits() | PageTableFlags::WRITABLE.bits(),
            );
        }
        new_pd
    };

    let pd = (pd_phys + hhdm) as *mut u64;
    let pde = unsafe { pd.add(pd_index(virt)) };
    let pd_entry = unsafe { core::ptr::read_volatile(pde) };

    // Get or create PT
    let pt_phys = if pd_entry & PageTableFlags::PRESENT.bits() != 0 {
        pd_entry & 0x000F_FFFF_FFFF_F000
    } else {
        let new_pt = match alloc_page_table_page() {
            Some(p) => p,
            None => return false,
        };
        let pt_virt = (new_pt + hhdm) as *mut u8;
        unsafe {
            core::ptr::write_bytes(pt_virt, 0, PAGE_SIZE as usize);
        }
        unsafe {
            core::ptr::write_volatile(
                pde,
                new_pt | PageTableFlags::PRESENT.bits() | PageTableFlags::WRITABLE.bits(),
            );
        }
        new_pt
    };

    let pt = (pt_phys + hhdm) as *mut u64;
    let pte = unsafe { pt.add(pt_index(virt)) };

    // Write PT entry with MMIO flags
    unsafe {
        core::ptr::write_volatile(pte, phys | MMIO_FLAGS);
    }

    // Flush TLB for this page
    unsafe {
        core::arch::asm!("invlpg [{}]", in(reg) virt, options(nostack, preserves_flags));
    }

    true
}
