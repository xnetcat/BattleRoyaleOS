//! VMSVGA (VMware SVGA II) register definitions
//!
//! Based on VMware SVGA Device Developer Kit and OSDev Wiki

use x86_64::instructions::port::Port;

/// VMware vendor ID
pub const VMWARE_VENDOR_ID: u16 = 0x15AD;

/// VMware SVGA II device ID
pub const VMSVGA_DEVICE_ID: u16 = 0x0405;

/// I/O port offsets from BAR0 base
pub const SVGA_INDEX_PORT: u16 = 0;
pub const SVGA_VALUE_PORT: u16 = 1;
pub const SVGA_BIOS_PORT: u16 = 2;
pub const SVGA_IRQSTATUS_PORT: u16 = 8;

/// SVGA register indices (accessed via index/value ports)
#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum SvgaReg {
    /// SVGA_REG_ID - Version negotiation
    Id = 0,
    /// SVGA_REG_ENABLE - Enable/disable SVGA mode
    Enable = 1,
    /// SVGA_REG_WIDTH - Display width in pixels
    Width = 2,
    /// SVGA_REG_HEIGHT - Display height in pixels
    Height = 3,
    /// SVGA_REG_MAX_WIDTH - Maximum supported width
    MaxWidth = 4,
    /// SVGA_REG_MAX_HEIGHT - Maximum supported height
    MaxHeight = 5,
    /// SVGA_REG_DEPTH - Color depth (deprecated, use BitsPerPixel)
    Depth = 6,
    /// SVGA_REG_BITS_PER_PIXEL - Bits per pixel (8, 15, 16, 24, 32)
    BitsPerPixel = 7,
    /// SVGA_REG_PSEUDOCOLOR - Pseudocolor mode
    Pseudocolor = 8,
    /// SVGA_REG_RED_MASK - Red color mask
    RedMask = 9,
    /// SVGA_REG_GREEN_MASK - Green color mask
    GreenMask = 10,
    /// SVGA_REG_BLUE_MASK - Blue color mask
    BlueMask = 11,
    /// SVGA_REG_BYTES_PER_LINE - Bytes per scanline (pitch)
    BytesPerLine = 12,
    /// SVGA_REG_FB_START - Physical address of framebuffer
    FbStart = 13,
    /// SVGA_REG_FB_OFFSET - Offset to visible framebuffer
    FbOffset = 14,
    /// SVGA_REG_VRAM_SIZE - Total VRAM size in bytes
    VramSize = 15,
    /// SVGA_REG_FB_SIZE - Framebuffer size in bytes
    FbSize = 16,
    /// SVGA_REG_CAPABILITIES - Device capabilities
    Capabilities = 17,
    /// SVGA_REG_MEM_START - Physical address of FIFO memory
    MemStart = 18,
    /// SVGA_REG_MEM_SIZE - FIFO memory size in bytes
    MemSize = 19,
    /// SVGA_REG_CONFIG_DONE - Signal configuration complete
    ConfigDone = 20,
    /// SVGA_REG_SYNC - Synchronization register
    Sync = 21,
    /// SVGA_REG_BUSY - Device busy status
    Busy = 22,
    /// SVGA_REG_GUEST_ID - Guest OS identification
    GuestId = 23,
    /// SVGA_REG_SCRATCH_SIZE - Size of scratch registers
    ScratchSize = 29,
    /// SVGA_REG_MEM_REGS - Number of FIFO registers
    MemRegs = 30,
}

/// SVGA version IDs for negotiation
/// Formula: (0x900000 << 8) | version
pub const SVGA_ID_2: u32 = 0x90000002; // SVGA II
pub const SVGA_ID_1: u32 = 0x90000001; // SVGA I
pub const SVGA_ID_0: u32 = 0x90000000; // SVGA 0

/// Magic number to detect SVGA device (high 24 bits)
pub const SVGA_MAGIC: u32 = 0x90000000;

/// Capability flags
pub mod cap {
    /// No capabilities
    pub const NONE: u32 = 0x00000000;
    /// SVGA_CAP_RECT_COPY - Rectangle copy acceleration
    pub const RECT_COPY: u32 = 0x00000002;
    /// SVGA_CAP_CURSOR - Hardware cursor
    pub const CURSOR: u32 = 0x00000020;
    /// SVGA_CAP_CURSOR_BYPASS - Cursor bypass
    pub const CURSOR_BYPASS: u32 = 0x00000040;
    /// SVGA_CAP_CURSOR_BYPASS_2 - Enhanced cursor bypass
    pub const CURSOR_BYPASS_2: u32 = 0x00000080;
    /// SVGA_CAP_8BIT_EMULATION - 8-bit emulation
    pub const EMULATION_8BIT: u32 = 0x00000100;
    /// SVGA_CAP_ALPHA_CURSOR - Alpha-blended cursor
    pub const ALPHA_CURSOR: u32 = 0x00000200;
    /// SVGA_CAP_3D - 3D acceleration support
    pub const THREE_D: u32 = 0x00004000;
    /// SVGA_CAP_EXTENDED_FIFO - Extended FIFO
    pub const EXTENDED_FIFO: u32 = 0x00008000;
    /// SVGA_CAP_MULTIMON - Multiple monitors
    pub const MULTIMON: u32 = 0x00010000;
    /// SVGA_CAP_PITCHLOCK - Pitch locking
    pub const PITCHLOCK: u32 = 0x00020000;
    /// SVGA_CAP_IRQMASK - IRQ masking
    pub const IRQMASK: u32 = 0x00040000;
    /// SVGA_CAP_DISPLAY_TOPOLOGY - Display topology
    pub const DISPLAY_TOPOLOGY: u32 = 0x00080000;
    /// SVGA_CAP_GMR - Guest Memory Regions
    pub const GMR: u32 = 0x00100000;
    /// SVGA_CAP_TRACES - Command tracing
    pub const TRACES: u32 = 0x00200000;
    /// SVGA_CAP_GMR2 - Enhanced GMR
    pub const GMR2: u32 = 0x00400000;
    /// SVGA_CAP_SCREEN_OBJECT_2 - Screen object 2
    pub const SCREEN_OBJECT_2: u32 = 0x00800000;
}

/// FIFO commands
pub mod cmd {
    /// SVGA_CMD_INVALID_CMD - Invalid command
    pub const INVALID: u32 = 0;
    /// SVGA_CMD_UPDATE - Update screen region
    pub const UPDATE: u32 = 1;
    /// SVGA_CMD_RECT_COPY - Copy rectangle
    pub const RECT_COPY: u32 = 3;
    /// SVGA_CMD_DEFINE_CURSOR - Define cursor
    pub const DEFINE_CURSOR: u32 = 19;
    /// SVGA_CMD_DEFINE_ALPHA_CURSOR - Define alpha cursor
    pub const DEFINE_ALPHA_CURSOR: u32 = 22;
    /// SVGA_CMD_UPDATE_VERBOSE - Verbose update
    pub const UPDATE_VERBOSE: u32 = 25;
    /// SVGA_CMD_FRONT_ROP_FILL - ROP fill
    pub const FRONT_ROP_FILL: u32 = 29;
    /// SVGA_CMD_FENCE - Fence command
    pub const FENCE: u32 = 30;
    /// SVGA_CMD_ESCAPE - Escape command
    pub const ESCAPE: u32 = 33;
    /// SVGA_CMD_DEFINE_SCREEN - Define screen
    pub const DEFINE_SCREEN: u32 = 34;
    /// SVGA_CMD_DESTROY_SCREEN - Destroy screen
    pub const DESTROY_SCREEN: u32 = 35;
    /// SVGA_CMD_DEFINE_GMRFB - Define GMR framebuffer
    pub const DEFINE_GMRFB: u32 = 36;
    /// SVGA_CMD_BLIT_GMRFB_TO_SCREEN - Blit from GMR to screen
    pub const BLIT_GMRFB_TO_SCREEN: u32 = 37;
    /// SVGA_CMD_BLIT_SCREEN_TO_GMRFB - Blit from screen to GMR
    pub const BLIT_SCREEN_TO_GMRFB: u32 = 38;
    /// SVGA_CMD_ANNOTATION_FILL - Annotation fill
    pub const ANNOTATION_FILL: u32 = 39;
    /// SVGA_CMD_ANNOTATION_COPY - Annotation copy
    pub const ANNOTATION_COPY: u32 = 40;
}

/// Read a SVGA register
///
/// # Safety
/// This performs port I/O and must only be called when io_base is valid
#[inline]
pub fn read_reg(io_base: u16, reg: SvgaReg) -> u32 {
    unsafe {
        let mut index_port = Port::<u32>::new(io_base + SVGA_INDEX_PORT);
        let mut value_port = Port::<u32>::new(io_base + SVGA_VALUE_PORT);
        index_port.write(reg as u32);
        value_port.read()
    }
}

/// Write a SVGA register
///
/// # Safety
/// This performs port I/O and must only be called when io_base is valid
#[inline]
pub fn write_reg(io_base: u16, reg: SvgaReg, value: u32) {
    unsafe {
        let mut index_port = Port::<u32>::new(io_base + SVGA_INDEX_PORT);
        let mut value_port = Port::<u32>::new(io_base + SVGA_VALUE_PORT);
        index_port.write(reg as u32);
        value_port.write(value);
    }
}

/// Read a raw register by index
#[inline]
pub fn read_reg_raw(io_base: u16, index: u32) -> u32 {
    unsafe {
        let mut index_port = Port::<u32>::new(io_base + SVGA_INDEX_PORT);
        let mut value_port = Port::<u32>::new(io_base + SVGA_VALUE_PORT);
        index_port.write(index);
        value_port.read()
    }
}

/// Write a raw register by index
#[inline]
pub fn write_reg_raw(io_base: u16, index: u32, value: u32) {
    unsafe {
        let mut index_port = Port::<u32>::new(io_base + SVGA_INDEX_PORT);
        let mut value_port = Port::<u32>::new(io_base + SVGA_VALUE_PORT);
        index_port.write(index);
        value_port.write(value);
    }
}

/// Negotiate SVGA version
/// Returns the highest supported version ID, or None if device is not compatible
pub fn negotiate_version(io_base: u16) -> Option<u32> {
    // Try SVGA II first (preferred)
    write_reg(io_base, SvgaReg::Id, SVGA_ID_2);
    if read_reg(io_base, SvgaReg::Id) == SVGA_ID_2 {
        return Some(SVGA_ID_2);
    }

    // Try SVGA I
    write_reg(io_base, SvgaReg::Id, SVGA_ID_1);
    if read_reg(io_base, SvgaReg::Id) == SVGA_ID_1 {
        return Some(SVGA_ID_1);
    }

    // Try SVGA 0
    write_reg(io_base, SvgaReg::Id, SVGA_ID_0);
    if read_reg(io_base, SvgaReg::Id) == SVGA_ID_0 {
        return Some(SVGA_ID_0);
    }

    // Device not compatible
    None
}

/// Check if a capability is supported
#[inline]
pub fn has_capability(caps: u32, cap: u32) -> bool {
    (caps & cap) != 0
}
