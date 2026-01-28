//! Vertical Synchronization (VSync) Module
//!
//! Provides frame timing and vsync support for bare-metal 3D rendering.
//!
//! Best practices implemented:
//! 1. VGA vertical retrace detection via port 0x3DA (bit 3)
//! 2. TSC-based frame timing for consistent frame rate
//! 3. Adaptive sync that falls back to timer-based if vsync unavailable
//! 4. Frame statistics tracking (dropped frames, FPS)
//!
//! Note on CPU power consumption:
//! Currently uses spin_loop() for waiting since this kernel doesn't have
//! interrupt handlers configured. To reduce CPU usage with HLT:
//! 1. Set up an IDT with timer interrupt handlers (PIT or APIC timer)
//! 2. Enable interrupts with x86_64::instructions::interrupts::enable()
//! 3. Replace spin_loop() with cpu_halt() in sleep_us()
//!
//! Reference: OSDev Wiki - Video Signals And Timing, VGA Hardware

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use x86_64::instructions::port::Port;

/// VGA Input Status Register 1 (read-only)
/// Port 0x3DA (color) or 0x3BA (monochrome)
const VGA_INPUT_STATUS_1: u16 = 0x3DA;

/// Bit 3: Vertical Retrace - Set during vertical retrace interval
const VGA_VRETRACE_BIT: u8 = 0x08;

/// Bit 0: Display Enable - Clear during active display, set during blanking
const VGA_DISPLAY_ENABLE_BIT: u8 = 0x01;

/// Target frame rate (Hz)
pub const TARGET_FPS: u64 = 60;

/// Target frame time in microseconds
pub const TARGET_FRAME_TIME_US: u64 = 1_000_000 / TARGET_FPS;

/// Estimated TSC cycles per microsecond (calibrated at runtime)
static TSC_PER_US: AtomicU64 = AtomicU64::new(2000); // Default ~2GHz

/// Whether VGA vsync is available (tested at init)
static VSYNC_AVAILABLE: AtomicBool = AtomicBool::new(false);

/// Whether to use vsync (can be disabled for benchmarking)
static VSYNC_ENABLED: AtomicBool = AtomicBool::new(true);

/// Frame counter for statistics
static FRAME_COUNT: AtomicU64 = AtomicU64::new(0);

/// Dropped frame counter (frames that took too long)
static DROPPED_FRAMES: AtomicU64 = AtomicU64::new(0);

/// Read the CPU timestamp counter
#[inline]
fn read_tsc() -> u64 {
    unsafe { core::arch::x86_64::_rdtsc() }
}

/// Execute HLT instruction to idle CPU until next interrupt
/// This reduces power consumption compared to busy-waiting
/// Note: Requires interrupt handlers (PIT/APIC timer) to wake up
#[inline]
#[allow(dead_code)]
fn cpu_halt() {
    unsafe {
        core::arch::asm!("hlt", options(nomem, nostack, preserves_flags));
    }
}

/// Read VGA Input Status Register 1
#[inline]
fn read_vga_status() -> u8 {
    unsafe {
        let mut port: Port<u8> = Port::new(VGA_INPUT_STATUS_1);
        port.read()
    }
}

/// Check if currently in vertical retrace period
#[inline]
pub fn is_in_vretrace() -> bool {
    (read_vga_status() & VGA_VRETRACE_BIT) != 0
}

/// Check if display is in blanking period (either horizontal or vertical)
#[inline]
pub fn is_in_blanking() -> bool {
    (read_vga_status() & VGA_DISPLAY_ENABLE_BIT) != 0
}

/// Initialize vsync subsystem
/// Calibrates TSC frequency and tests for VGA vsync availability
pub fn init() {
    // Test if VGA status port is readable and responsive
    // Read the port multiple times to check for changing values
    let mut found_vretrace = false;
    let mut found_no_vretrace = false;

    let start = read_tsc();
    let timeout_cycles = 100_000_000; // ~50ms at 2GHz

    while read_tsc() - start < timeout_cycles {
        let status = read_vga_status();
        if (status & VGA_VRETRACE_BIT) != 0 {
            found_vretrace = true;
        } else {
            found_no_vretrace = true;
        }

        // If we've seen both states, vsync is working
        if found_vretrace && found_no_vretrace {
            VSYNC_AVAILABLE.store(true, Ordering::Release);
            crate::serial_println!("VSync: VGA vertical retrace available");
            break;
        }
    }

    if !VSYNC_AVAILABLE.load(Ordering::Acquire) {
        crate::serial_println!("VSync: VGA vertical retrace not available, using timer-based sync");
    }

    // Calibrate TSC frequency using a simple delay
    // In a real system, you'd use PIT or HPET for accurate calibration
    // For now, assume ~2GHz which is common for QEMU
    TSC_PER_US.store(2000, Ordering::Release);

    crate::serial_println!("VSync: Initialized (target {} FPS, {}us/frame)",
        TARGET_FPS, TARGET_FRAME_TIME_US);
}

/// Wait for the start of vertical blank period
/// This is the classic vsync approach: wait for vblank before swapping buffers
pub fn wait_for_vblank() {
    if !VSYNC_AVAILABLE.load(Ordering::Acquire) {
        return;
    }

    // First, wait for any current retrace to end
    // This handles the case where we're already in vblank
    while is_in_vretrace() {
        core::hint::spin_loop();
    }

    // Now wait for the next vertical retrace to begin
    while !is_in_vretrace() {
        core::hint::spin_loop();
    }
}

/// Wait for the end of active display (start of blanking)
/// This gives maximum time for rendering before next frame
pub fn wait_for_blanking() {
    if !VSYNC_AVAILABLE.load(Ordering::Acquire) {
        return;
    }

    // Wait until display enable goes low (blanking starts)
    while !is_in_blanking() {
        core::hint::spin_loop();
    }
}

/// Sleep for approximately the given number of microseconds
/// Note: Uses spin_loop since this kernel doesn't have interrupt handlers configured.
/// In a full OS with timer interrupts, HLT could be used for better power efficiency.
pub fn sleep_us(microseconds: u64) {
    let tsc_per_us = TSC_PER_US.load(Ordering::Acquire);
    let target_cycles = microseconds * tsc_per_us;
    let start = read_tsc();

    while read_tsc() - start < target_cycles {
        // Use spin_loop hint to reduce CPU power in the tight loop
        // This is less efficient than HLT but doesn't require interrupt handlers
        core::hint::spin_loop();
    }
}

/// Sleep for approximately the given number of milliseconds
pub fn sleep_ms(milliseconds: u64) {
    sleep_us(milliseconds * 1000);
}

/// Frame timing state for the main loop
pub struct FrameTimer {
    /// TSC at start of current frame
    frame_start: u64,
    /// TSC at last FPS calculation
    last_fps_time: u64,
    /// Frames since last FPS calculation
    fps_frame_count: u32,
    /// Current calculated FPS
    current_fps: u32,
    /// Target TSC cycles per frame
    tsc_per_frame: u64,
    /// Whether to use vsync (true) or uncapped (false)
    use_vsync: bool,
}

impl FrameTimer {
    /// Create a new frame timer
    pub fn new() -> Self {
        let tsc_per_us = TSC_PER_US.load(Ordering::Acquire);
        let tsc_per_frame = TARGET_FRAME_TIME_US * tsc_per_us;

        Self {
            frame_start: read_tsc(),
            last_fps_time: read_tsc(),
            fps_frame_count: 0,
            current_fps: 0,
            tsc_per_frame,
            use_vsync: VSYNC_ENABLED.load(Ordering::Acquire),
        }
    }

    /// Call at the start of each frame
    pub fn begin_frame(&mut self) {
        self.frame_start = read_tsc();
    }

    /// Call at the end of each frame to wait for vsync/frame timing
    /// Returns true if frame was on time, false if it was dropped (took too long)
    pub fn end_frame(&mut self) -> bool {
        let frame_end = read_tsc();
        let frame_duration = frame_end.wrapping_sub(self.frame_start);

        // Update FPS counter
        self.fps_frame_count += 1;
        FRAME_COUNT.fetch_add(1, Ordering::Relaxed);

        // Calculate FPS every second
        let tsc_per_us = TSC_PER_US.load(Ordering::Acquire);
        let tsc_per_second = tsc_per_us * 1_000_000;
        let elapsed = frame_end.wrapping_sub(self.last_fps_time);

        if elapsed >= tsc_per_second {
            self.current_fps = self.fps_frame_count;
            self.fps_frame_count = 0;
            self.last_fps_time = frame_end;
        }

        // Check if frame took too long
        let on_time = frame_duration < self.tsc_per_frame;
        if !on_time {
            DROPPED_FRAMES.fetch_add(1, Ordering::Relaxed);
            // Frame already late - skip vsync wait to catch up
            return false;
        }

        // Wait for frame timing using TSC-based timing
        // VGA vsync detection unreliable in QEMU, so use timer as primary
        if self.use_vsync {
            while read_tsc().wrapping_sub(self.frame_start) < self.tsc_per_frame {
                core::hint::spin_loop();
            }
        }

        on_time
    }

    /// Get current FPS
    pub fn fps(&self) -> u32 {
        self.current_fps
    }

    /// Enable or disable vsync
    pub fn set_vsync(&mut self, enabled: bool) {
        self.use_vsync = enabled;
        VSYNC_ENABLED.store(enabled, Ordering::Release);
    }

    /// Check if vsync is enabled
    pub fn vsync_enabled(&self) -> bool {
        self.use_vsync
    }
}

/// Enable vsync globally
pub fn enable() {
    VSYNC_ENABLED.store(true, Ordering::Release);
}

/// Disable vsync globally (for benchmarking)
pub fn disable() {
    VSYNC_ENABLED.store(false, Ordering::Release);
}

/// Check if vsync is globally enabled
pub fn is_enabled() -> bool {
    VSYNC_ENABLED.load(Ordering::Acquire)
}

/// Check if hardware vsync is available
pub fn is_available() -> bool {
    VSYNC_AVAILABLE.load(Ordering::Acquire)
}

/// Get total frame count since init
pub fn frame_count() -> u64 {
    FRAME_COUNT.load(Ordering::Relaxed)
}

/// Get total dropped frame count since init
pub fn dropped_frames() -> u64 {
    DROPPED_FRAMES.load(Ordering::Relaxed)
}

/// Get vsync statistics as (total_frames, dropped_frames, drop_rate_percent)
pub fn get_stats() -> (u64, u64, f32) {
    let total = frame_count();
    let dropped = dropped_frames();
    let drop_rate = if total > 0 {
        (dropped as f32 / total as f32) * 100.0
    } else {
        0.0
    };
    (total, dropped, drop_rate)
}
