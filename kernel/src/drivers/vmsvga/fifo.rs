//! VMSVGA FIFO command buffer management
//!
//! The FIFO is a circular buffer used to send commands to the SVGA device.
//! Commands are written to the FIFO and the device processes them asynchronously.

use super::regs::{self, SvgaReg};
use core::sync::atomic::{fence, Ordering};

/// FIFO register offsets (indices into FIFO memory)
pub mod fifo_reg {
    /// Minimum valid offset in FIFO (start of command area)
    pub const MIN: usize = 0;
    /// Maximum valid offset in FIFO (end of command area)
    pub const MAX: usize = 1;
    /// Next command write offset
    pub const NEXT_CMD: usize = 2;
    /// Next command read offset (device controlled)
    pub const STOP: usize = 3;
    /// Capabilities (extended FIFO)
    pub const CAPABILITIES: usize = 4;
    /// Flags
    pub const FLAGS: usize = 5;
    /// Fence value
    pub const FENCE: usize = 6;
    /// 3D hardware version
    pub const HWVERSION_3D: usize = 7;
    /// Pitch lock
    pub const PITCHLOCK: usize = 8;
    /// Cursor count
    pub const CURSOR_COUNT: usize = 9;
    /// Cursor last updated
    pub const CURSOR_LAST_UPDATED: usize = 10;
    /// Reserved area
    pub const RESERVED: usize = 11;
    /// Cursor screen ID
    pub const CURSOR_SCREEN_ID: usize = 12;
    /// Dead space (unused)
    pub const DEAD: usize = 13;
    /// 3D capabilities record (variable size)
    pub const CAPS_3D_RECORD: usize = 14;
    /// Minimum registers for basic operation
    pub const NUM_REGS: usize = 4;
}

/// FIFO capabilities
pub mod fifo_cap {
    /// FIFO has fence capability
    pub const FENCE: u32 = 0x01;
    /// FIFO has ACCEL_FRONT capability
    pub const ACCEL_FRONT: u32 = 0x02;
    /// FIFO has PITCHLOCK capability
    pub const PITCHLOCK: u32 = 0x04;
    /// FIFO has VIDEO capability
    pub const VIDEO: u32 = 0x08;
    /// FIFO has CURSOR_BYPASS_3 capability
    pub const CURSOR_BYPASS_3: u32 = 0x10;
    /// FIFO has ESCAPE capability
    pub const ESCAPE: u32 = 0x20;
    /// FIFO has RESERVE capability
    pub const RESERVE: u32 = 0x40;
    /// FIFO has SCREEN_OBJECT capability
    pub const SCREEN_OBJECT: u32 = 0x80;
    /// FIFO has GMR2 capability
    pub const GMR2: u32 = 0x100;
    /// FIFO has 3D_HWVERSION_REVISED capability
    pub const HWVERSION_3D_REVISED: u32 = 0x200;
    /// FIFO has SCREEN_OBJECT_2 capability
    pub const SCREEN_OBJECT_2: u32 = 0x400;
    /// FIFO has DEAD capability (dead space)
    pub const DEAD: u32 = 0x800;
}

/// VMSVGA FIFO structure
pub struct VmsvgaFifo {
    /// Virtual address of FIFO memory
    base: *mut u32,
    /// Size of FIFO memory in bytes
    size: usize,
    /// I/O base for register access
    io_base: u16,
    /// Cached capabilities
    capabilities: u32,
}

// Safety: FIFO is memory-mapped I/O and access is single-threaded
unsafe impl Send for VmsvgaFifo {}
unsafe impl Sync for VmsvgaFifo {}

impl VmsvgaFifo {
    /// Create a new FIFO instance (uninitialized)
    pub const fn new() -> Self {
        Self {
            base: core::ptr::null_mut(),
            size: 0,
            io_base: 0,
            capabilities: 0,
        }
    }

    /// Initialize the FIFO
    ///
    /// # Arguments
    /// * `fifo_virt` - Virtual address of mapped FIFO memory
    /// * `fifo_size` - Size of FIFO memory in bytes
    /// * `io_base` - I/O port base address
    /// * `device_caps` - Device capabilities from SVGA_REG_CAPABILITIES
    pub fn init(&mut self, fifo_virt: u64, fifo_size: usize, io_base: u16, device_caps: u32) {
        self.base = fifo_virt as *mut u32;
        self.size = fifo_size;
        self.io_base = io_base;

        // Set up FIFO header
        let min_offset = fifo_reg::NUM_REGS as u32 * 4;
        let max_offset = fifo_size as u32;

        unsafe {
            // FIFO[MIN] = start of command area
            core::ptr::write_volatile(self.base.add(fifo_reg::MIN), min_offset);
            // FIFO[MAX] = end of FIFO
            core::ptr::write_volatile(self.base.add(fifo_reg::MAX), max_offset);
            // FIFO[NEXT_CMD] = start of command area (empty)
            core::ptr::write_volatile(self.base.add(fifo_reg::NEXT_CMD), min_offset);
            // FIFO[STOP] = start of command area (nothing to process)
            core::ptr::write_volatile(self.base.add(fifo_reg::STOP), min_offset);
        }

        // Memory barrier to ensure writes are visible
        fence(Ordering::SeqCst);

        // Check for extended FIFO capabilities
        if regs::has_capability(device_caps, regs::cap::EXTENDED_FIFO) {
            self.capabilities = unsafe {
                core::ptr::read_volatile(self.base.add(fifo_reg::CAPABILITIES))
            };
        } else {
            self.capabilities = 0;
        }
    }

    /// Check if FIFO is initialized
    #[inline]
    pub fn is_initialized(&self) -> bool {
        !self.base.is_null()
    }

    /// Get FIFO capabilities
    #[inline]
    pub fn capabilities(&self) -> u32 {
        self.capabilities
    }

    /// Check if a FIFO capability is supported
    #[inline]
    pub fn has_cap(&self, cap: u32) -> bool {
        (self.capabilities & cap) != 0
    }

    /// Read a FIFO register
    #[inline]
    fn read_reg(&self, index: usize) -> u32 {
        unsafe { core::ptr::read_volatile(self.base.add(index)) }
    }

    /// Write a FIFO register
    #[inline]
    fn write_reg(&self, index: usize, value: u32) {
        unsafe { core::ptr::write_volatile(self.base.add(index), value) }
    }

    /// Reserve space in the FIFO for a command
    /// Returns the offset where the command should be written
    fn reserve(&self, bytes: usize) -> Option<u32> {
        if !self.is_initialized() {
            return None;
        }

        let bytes = bytes as u32;
        let min = self.read_reg(fifo_reg::MIN);
        let max = self.read_reg(fifo_reg::MAX);
        let next_cmd = self.read_reg(fifo_reg::NEXT_CMD);

        // Check if there's enough space
        // This is a simplified check - a full implementation would handle wrapping
        let stop = self.read_reg(fifo_reg::STOP);

        if next_cmd >= stop {
            // Data wraps around or FIFO is empty
            let space_at_end = max - next_cmd;
            let space_at_start = stop - min;

            if space_at_end >= bytes {
                // Enough space at end
                Some(next_cmd)
            } else if space_at_start >= bytes {
                // Need to wrap - insert NOP to fill end
                // For simplicity, we'll just fail if we need to wrap
                None
            } else {
                // Not enough space
                None
            }
        } else {
            // FIFO has contiguous free space
            let space = stop - next_cmd;
            if space >= bytes {
                Some(next_cmd)
            } else {
                None
            }
        }
    }

    /// Commit a command that was written to the FIFO
    fn commit(&self, bytes: usize) {
        if !self.is_initialized() {
            return;
        }

        let bytes = bytes as u32;
        let max = self.read_reg(fifo_reg::MAX);
        let min = self.read_reg(fifo_reg::MIN);
        let mut next_cmd = self.read_reg(fifo_reg::NEXT_CMD);

        // Advance NEXT_CMD
        next_cmd += bytes;

        // Handle wrap-around
        if next_cmd >= max {
            next_cmd = min + (next_cmd - max);
        }

        // Memory barrier before updating NEXT_CMD
        fence(Ordering::SeqCst);

        // Write new NEXT_CMD
        self.write_reg(fifo_reg::NEXT_CMD, next_cmd);

        // Memory barrier after updating NEXT_CMD
        fence(Ordering::SeqCst);
    }

    /// Write a command to the FIFO
    /// Returns true if the command was written successfully
    pub fn write_cmd(&self, cmd: &[u32]) -> bool {
        let bytes = cmd.len() * 4;

        let offset = match self.reserve(bytes) {
            Some(off) => off,
            None => {
                // Try to sync and retry once
                self.sync();
                match self.reserve(bytes) {
                    Some(off) => off,
                    None => return false,
                }
            }
        };

        // Write command data
        let ptr = self.base as usize + offset as usize;
        for (i, &word) in cmd.iter().enumerate() {
            unsafe {
                core::ptr::write_volatile((ptr as *mut u32).add(i), word);
            }
        }

        // Commit the command
        self.commit(bytes);

        true
    }

    /// Synchronize - wait for all commands to complete
    pub fn sync(&self) {
        if !self.is_initialized() {
            return;
        }

        // Write to SYNC register
        regs::write_reg(self.io_base, SvgaReg::Sync, 1);

        // Wait for BUSY to clear
        loop {
            let busy = regs::read_reg(self.io_base, SvgaReg::Busy);
            if busy == 0 {
                break;
            }
            core::hint::spin_loop();
        }
    }

    /// Send UPDATE command to refresh a screen region
    pub fn cmd_update(&self, x: u32, y: u32, width: u32, height: u32) -> bool {
        let cmd = [
            regs::cmd::UPDATE,
            x,
            y,
            width,
            height,
        ];
        self.write_cmd(&cmd)
    }

    /// Send RECT_COPY command to copy a rectangle
    pub fn cmd_rect_copy(&self, src_x: u32, src_y: u32, dst_x: u32, dst_y: u32, width: u32, height: u32) -> bool {
        let cmd = [
            regs::cmd::RECT_COPY,
            src_x,
            src_y,
            dst_x,
            dst_y,
            width,
            height,
        ];
        self.write_cmd(&cmd)
    }

    /// Send FRONT_ROP_FILL command to fill a rectangle
    /// color is ARGB format
    pub fn cmd_rect_fill(&self, color: u32, x: u32, y: u32, width: u32, height: u32) -> bool {
        let cmd = [
            regs::cmd::FRONT_ROP_FILL,
            color,
            x,
            y,
            width,
            height,
            0xCC, // SRCCOPY ROP (direct copy)
        ];
        self.write_cmd(&cmd)
    }

    /// Send UPDATE command to refresh the entire screen
    pub fn cmd_update_full(&self, width: u32, height: u32) -> bool {
        self.cmd_update(0, 0, width, height)
    }
}
