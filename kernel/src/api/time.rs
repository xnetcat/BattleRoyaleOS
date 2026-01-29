//! Time API
//!
//! Provides timing services for applications.

use super::types::{KernelError, KernelResult};

/// Time service for frame timing and timestamps
pub struct TimeService {
    tsc_frequency: u64,
    start_tsc: u64,
}

impl TimeService {
    /// Create a new time service
    pub fn new() -> KernelResult<Self> {
        Ok(Self {
            tsc_frequency: 2_000_000_000, // Assume ~2GHz for QEMU
            start_tsc: read_tsc(),
        })
    }

    /// Get current timestamp counter value
    pub fn tsc(&self) -> u64 {
        read_tsc()
    }

    /// Get elapsed time since service creation in seconds
    pub fn elapsed_secs(&self) -> f64 {
        let current = read_tsc();
        let elapsed = current.wrapping_sub(self.start_tsc);
        elapsed as f64 / self.tsc_frequency as f64
    }

    /// Get elapsed time since service creation in milliseconds
    pub fn elapsed_ms(&self) -> u64 {
        let current = read_tsc();
        let elapsed = current.wrapping_sub(self.start_tsc);
        (elapsed * 1000) / self.tsc_frequency
    }

    /// Sleep for approximately the given number of microseconds
    /// Uses HLT instruction for CPU efficiency
    pub fn sleep_us(&self, microseconds: u64) {
        let target_tsc = read_tsc() + (microseconds * self.tsc_frequency / 1_000_000);
        while read_tsc() < target_tsc {
            // Use HLT for power efficiency while waiting
            unsafe { core::arch::asm!("hlt"); }
        }
    }

    /// Create a frame timer for game loops
    pub fn create_frame_timer(&self, target_fps: u32) -> FrameTimer {
        FrameTimer::new(target_fps, self.tsc_frequency)
    }
}

impl Default for TimeService {
    fn default() -> Self {
        Self {
            tsc_frequency: 2_000_000_000,
            start_tsc: 0,
        }
    }
}

/// Read CPU timestamp counter
#[inline]
fn read_tsc() -> u64 {
    unsafe { core::arch::x86_64::_rdtsc() }
}

/// Frame timer for consistent game loop timing
pub struct FrameTimer {
    target_fps: u32,
    tsc_frequency: u64,
    frame_tsc: u64,
    last_frame_start: u64,
    frame_count: u64,
    fps_counter_start: u64,
    fps_frame_count: u32,
    current_fps: u32,
}

impl FrameTimer {
    /// Create a new frame timer targeting the given FPS
    pub fn new(target_fps: u32, tsc_frequency: u64) -> Self {
        let now = read_tsc();
        Self {
            target_fps,
            tsc_frequency,
            frame_tsc: tsc_frequency / target_fps as u64,
            last_frame_start: now,
            frame_count: 0,
            fps_counter_start: now,
            fps_frame_count: 0,
            current_fps: 0,
        }
    }

    /// Begin a new frame
    pub fn begin_frame(&mut self) {
        self.last_frame_start = read_tsc();
    }

    /// End the current frame, waiting if necessary to maintain target FPS
    /// Returns true if frame was on time, false if it ran long
    pub fn end_frame(&mut self) -> bool {
        self.frame_count += 1;
        self.fps_frame_count += 1;

        let current = read_tsc();
        let elapsed = current.wrapping_sub(self.last_frame_start);
        let on_time = elapsed < self.frame_tsc;

        // Wait for remaining frame time if we're ahead
        if on_time {
            let remaining = self.frame_tsc - elapsed;
            let target = current + remaining;
            while read_tsc() < target {
                unsafe { core::arch::asm!("hlt"); }
            }
        }

        // Update FPS counter every second
        let fps_elapsed = current.wrapping_sub(self.fps_counter_start);
        if fps_elapsed >= self.tsc_frequency {
            self.current_fps = (self.fps_frame_count as u64 * self.tsc_frequency / fps_elapsed) as u32;
            self.fps_frame_count = 0;
            self.fps_counter_start = current;
        }

        on_time
    }

    /// Get current FPS
    pub fn fps(&self) -> u32 {
        self.current_fps
    }

    /// Get total frame count
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Get target FPS
    pub fn target_fps(&self) -> u32 {
        self.target_fps
    }

    /// Get delta time in seconds (based on target FPS)
    pub fn delta_time(&self) -> f32 {
        1.0 / self.target_fps as f32
    }
}
