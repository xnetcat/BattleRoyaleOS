//! E1000 Ring Buffer Management

use super::descriptors::{RxDescriptor, TxDescriptor};
use super::regs::*;
use super::{BUFFER_SIZE, RX_RING_SIZE, TX_RING_SIZE};
use crate::memory::dma::virt_to_phys;
use alloc::vec;
use alloc::vec::Vec;
use core::alloc::Layout;

/// Transmit ring buffer
pub struct TxRing {
    descriptors: *mut TxDescriptor,
    buffers: Vec<*mut u8>,
}

/// Receive ring buffer
pub struct RxRing {
    descriptors: *mut RxDescriptor,
    buffers: Vec<*mut u8>,
}

impl TxRing {
    pub const fn new() -> Self {
        Self {
            descriptors: core::ptr::null_mut(),
            buffers: Vec::new(),
        }
    }

    pub fn init(&mut self) -> Result<(), &'static str> {
        // Allocate descriptor ring (aligned to 16 bytes)
        let desc_layout =
            Layout::from_size_align(TX_RING_SIZE * core::mem::size_of::<TxDescriptor>(), 16)
                .map_err(|_| "Invalid layout")?;
        let desc_ptr = unsafe { alloc::alloc::alloc_zeroed(desc_layout) };
        if desc_ptr.is_null() {
            return Err("Failed to allocate TX descriptors");
        }
        self.descriptors = desc_ptr as *mut TxDescriptor;

        // Allocate packet buffers
        self.buffers = Vec::with_capacity(TX_RING_SIZE);
        for i in 0..TX_RING_SIZE {
            let buf_layout =
                Layout::from_size_align(BUFFER_SIZE, 16).map_err(|_| "Invalid layout")?;
            let buf_ptr = unsafe { alloc::alloc::alloc_zeroed(buf_layout) };
            if buf_ptr.is_null() {
                return Err("Failed to allocate TX buffer");
            }
            self.buffers.push(buf_ptr);

            // Initialize descriptor
            unsafe {
                let desc = &mut *self.descriptors.add(i);
                desc.buffer_addr = virt_to_phys(buf_ptr);
                desc.status = TX_STATUS_DD; // Mark as available
            }
        }

        Ok(())
    }

    pub fn descriptor_phys_addr(&self) -> u64 {
        virt_to_phys(self.descriptors as *const u8)
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
            descriptors: core::ptr::null_mut(),
            buffers: Vec::new(),
        }
    }

    pub fn init(&mut self) -> Result<(), &'static str> {
        // Allocate descriptor ring (aligned to 16 bytes)
        let desc_layout =
            Layout::from_size_align(RX_RING_SIZE * core::mem::size_of::<RxDescriptor>(), 16)
                .map_err(|_| "Invalid layout")?;
        let desc_ptr = unsafe { alloc::alloc::alloc_zeroed(desc_layout) };
        if desc_ptr.is_null() {
            return Err("Failed to allocate RX descriptors");
        }
        self.descriptors = desc_ptr as *mut RxDescriptor;

        // Allocate packet buffers
        self.buffers = Vec::with_capacity(RX_RING_SIZE);
        for i in 0..RX_RING_SIZE {
            let buf_layout =
                Layout::from_size_align(BUFFER_SIZE, 16).map_err(|_| "Invalid layout")?;
            let buf_ptr = unsafe { alloc::alloc::alloc_zeroed(buf_layout) };
            if buf_ptr.is_null() {
                return Err("Failed to allocate RX buffer");
            }
            self.buffers.push(buf_ptr);

            // Initialize descriptor with buffer address
            unsafe {
                let desc = &mut *self.descriptors.add(i);
                desc.buffer_addr = virt_to_phys(buf_ptr);
                desc.status = 0;
            }
        }

        Ok(())
    }

    pub fn descriptor_phys_addr(&self) -> u64 {
        virt_to_phys(self.descriptors as *const u8)
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
