//! E1000 Ring Buffer Management
//!
//! Uses the DMA allocator to get physical pages for descriptor rings and buffers.

use super::descriptors::{RxDescriptor, TxDescriptor};
use super::regs::*;
use super::{BUFFER_SIZE, RX_RING_SIZE, TX_RING_SIZE};
use crate::memory::dma::{alloc_dma_page, virt_to_phys};
use alloc::vec;
use alloc::vec::Vec;

/// Transmit ring buffer
pub struct TxRing {
    /// Physical address of descriptor ring
    desc_phys: u64,
    /// Virtual address of descriptor ring
    descriptors: *mut TxDescriptor,
    /// Virtual addresses of packet buffers
    buffers: Vec<*mut u8>,
    /// Physical addresses of packet buffers
    buffer_phys: Vec<u64>,
}

/// Receive ring buffer
pub struct RxRing {
    /// Physical address of descriptor ring
    desc_phys: u64,
    /// Virtual address of descriptor ring
    descriptors: *mut RxDescriptor,
    /// Virtual addresses of packet buffers
    buffers: Vec<*mut u8>,
    /// Physical addresses of packet buffers
    buffer_phys: Vec<u64>,
}

impl TxRing {
    pub const fn new() -> Self {
        Self {
            desc_phys: 0,
            descriptors: core::ptr::null_mut(),
            buffers: Vec::new(),
            buffer_phys: Vec::new(),
        }
    }

    pub fn init(&mut self) -> Result<(), &'static str> {
        // Allocate descriptor ring from DMA pool
        // TX_RING_SIZE descriptors * 16 bytes = 2048 bytes (fits in one page)
        let (desc_phys, desc_virt) = alloc_dma_page().ok_or("Failed to allocate TX descriptor page")?;
        self.desc_phys = desc_phys;
        self.descriptors = desc_virt as *mut TxDescriptor;

        // Allocate packet buffers from DMA pool
        // Each buffer is 2048 bytes, so 2 buffers per page
        self.buffers = Vec::with_capacity(TX_RING_SIZE);
        self.buffer_phys = Vec::with_capacity(TX_RING_SIZE);

        let mut current_page_virt: Option<*mut u8> = None;
        let mut current_page_phys: u64 = 0;
        let mut offset_in_page: usize = 0;

        for i in 0..TX_RING_SIZE {
            // Allocate a new page if needed (2 buffers per page)
            if i % 2 == 0 {
                let (phys, virt) = alloc_dma_page().ok_or("Failed to allocate TX buffer page")?;
                current_page_virt = Some(virt);
                current_page_phys = phys;
                offset_in_page = 0;
            }

            let virt = current_page_virt.unwrap();
            let buf_virt = unsafe { virt.add(offset_in_page) };
            let buf_phys = current_page_phys + offset_in_page as u64;

            self.buffers.push(buf_virt);
            self.buffer_phys.push(buf_phys);

            // Initialize descriptor
            unsafe {
                let desc = &mut *self.descriptors.add(i);
                desc.buffer_addr = buf_phys;
                desc.status = TX_STATUS_DD; // Mark as available
            }

            offset_in_page += BUFFER_SIZE;
        }

        Ok(())
    }

    pub fn descriptor_phys_addr(&self) -> u64 {
        self.desc_phys
    }

    pub fn get_descriptor(&self, index: usize) -> *mut TxDescriptor {
        unsafe { self.descriptors.add(index) }
    }

    pub fn prepare_send(&mut self, index: usize, data: &[u8]) {
        unsafe {
            // Copy data to buffer
            let buf = self.buffers[index];
            core::ptr::copy_nonoverlapping(data.as_ptr(), buf, data.len());

            // Update descriptor
            let desc = &mut *self.descriptors.add(index);
            desc.length = data.len() as u16;
            desc.cmd = TX_CMD_EOP | TX_CMD_IFCS | TX_CMD_RS;
            desc.status = 0;
        }
    }
}

impl RxRing {
    pub const fn new() -> Self {
        Self {
            desc_phys: 0,
            descriptors: core::ptr::null_mut(),
            buffers: Vec::new(),
            buffer_phys: Vec::new(),
        }
    }

    pub fn init(&mut self) -> Result<(), &'static str> {
        // Allocate descriptor ring from DMA pool
        // RX_RING_SIZE descriptors * 16 bytes = 4096 bytes (fits in one page)
        let (desc_phys, desc_virt) = alloc_dma_page().ok_or("Failed to allocate RX descriptor page")?;
        self.desc_phys = desc_phys;
        self.descriptors = desc_virt as *mut RxDescriptor;

        // Allocate packet buffers from DMA pool
        self.buffers = Vec::with_capacity(RX_RING_SIZE);
        self.buffer_phys = Vec::with_capacity(RX_RING_SIZE);

        let mut current_page_virt: Option<*mut u8> = None;
        let mut current_page_phys: u64 = 0;
        let mut offset_in_page: usize = 0;

        for i in 0..RX_RING_SIZE {
            // Allocate a new page if needed (2 buffers per page)
            if i % 2 == 0 {
                let (phys, virt) = alloc_dma_page().ok_or("Failed to allocate RX buffer page")?;
                current_page_virt = Some(virt);
                current_page_phys = phys;
                offset_in_page = 0;
            }

            let virt = current_page_virt.unwrap();
            let buf_virt = unsafe { virt.add(offset_in_page) };
            let buf_phys = current_page_phys + offset_in_page as u64;

            self.buffers.push(buf_virt);
            self.buffer_phys.push(buf_phys);

            // Initialize descriptor with buffer address
            unsafe {
                let desc = &mut *self.descriptors.add(i);
                desc.buffer_addr = buf_phys;
                desc.status = 0;
            }

            offset_in_page += BUFFER_SIZE;
        }

        Ok(())
    }

    pub fn descriptor_phys_addr(&self) -> u64 {
        self.desc_phys
    }

    pub fn get_descriptor(&self, index: usize) -> *mut RxDescriptor {
        unsafe { self.descriptors.add(index) }
    }

    pub fn read_packet(&self, index: usize, length: usize) -> Vec<u8> {
        let mut data = vec![0u8; length];
        unsafe {
            core::ptr::copy_nonoverlapping(self.buffers[index], data.as_mut_ptr(), length);
        }
        data
    }
}

// Safety: The rings are protected by the E1000 mutex
unsafe impl Send for TxRing {}
unsafe impl Send for RxRing {}
