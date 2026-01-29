//! Common types for kernel API
//!
//! This module contains fundamental types, handles, and error types
//! used across all kernel services.

use core::fmt;

/// Opaque handle for kernel resources
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Handle(u32);

impl Handle {
    pub const INVALID: Handle = Handle(0);

    #[inline]
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    #[inline]
    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }

    #[inline]
    pub fn raw(&self) -> u32 {
        self.0
    }
}

impl Default for Handle {
    fn default() -> Self {
        Self::INVALID
    }
}

/// Kernel API error type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelError {
    /// Invalid handle provided
    InvalidHandle,
    /// Resource not found
    NotFound,
    /// Out of memory
    OutOfMemory,
    /// Device not available
    DeviceNotAvailable,
    /// Invalid parameter
    InvalidParameter,
    /// Operation not supported
    NotSupported,
    /// Resource busy
    Busy,
    /// Operation timeout
    Timeout,
    /// Generic I/O error
    IoError,
    /// Resource limit reached
    ResourceLimit,
    /// Not initialized
    NotInitialized,
    /// Already initialized
    AlreadyInitialized,
    /// Permission denied
    PermissionDenied,
}

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHandle => write!(f, "invalid handle"),
            Self::NotFound => write!(f, "resource not found"),
            Self::OutOfMemory => write!(f, "out of memory"),
            Self::DeviceNotAvailable => write!(f, "device not available"),
            Self::InvalidParameter => write!(f, "invalid parameter"),
            Self::NotSupported => write!(f, "operation not supported"),
            Self::Busy => write!(f, "resource busy"),
            Self::Timeout => write!(f, "operation timeout"),
            Self::IoError => write!(f, "I/O error"),
            Self::ResourceLimit => write!(f, "resource limit reached"),
            Self::NotInitialized => write!(f, "not initialized"),
            Self::AlreadyInitialized => write!(f, "already initialized"),
            Self::PermissionDenied => write!(f, "permission denied"),
        }
    }
}

/// Result type for kernel operations
pub type KernelResult<T> = Result<T, KernelError>;

/// Application run mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    /// Game client with full rendering
    GameClient,
    /// Dedicated server (headless)
    GameServer,
    /// Performance benchmark
    Benchmark,
    /// Test harness
    TestHarness,
}

impl AppMode {
    /// Parse from command line string
    pub fn from_cmdline(cmdline: &str) -> Self {
        if cmdline.contains("server") {
            Self::GameServer
        } else if cmdline.contains("benchmark") {
            Self::Benchmark
        } else if cmdline.contains("test") {
            Self::TestHarness
        } else {
            Self::GameClient
        }
    }

    /// Whether this mode requires graphics
    pub fn needs_graphics(&self) -> bool {
        matches!(self, Self::GameClient | Self::Benchmark)
    }

    /// Whether this mode is headless
    pub fn is_headless(&self) -> bool {
        matches!(self, Self::GameServer | Self::TestHarness)
    }
}

/// Screen dimensions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
}

impl Dimensions {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }
}

/// RGBA color
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    pub const RED: Color = Color::rgb(255, 0, 0);
    pub const GREEN: Color = Color::rgb(0, 255, 0);
    pub const BLUE: Color = Color::rgb(0, 0, 255);
    pub const TRANSPARENT: Color = Color::rgba(0, 0, 0, 0);

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn to_u32(&self) -> u32 {
        ((self.a as u32) << 24)
            | ((self.r as u32) << 16)
            | ((self.g as u32) << 8)
            | (self.b as u32)
    }

    pub const fn from_u32(color: u32) -> Self {
        Self {
            a: ((color >> 24) & 0xFF) as u8,
            r: ((color >> 16) & 0xFF) as u8,
            g: ((color >> 8) & 0xFF) as u8,
            b: (color & 0xFF) as u8,
        }
    }

    /// Linear interpolation between two colors
    pub fn lerp(a: Color, b: Color, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let inv_t = 1.0 - t;
        Self {
            r: (a.r as f32 * inv_t + b.r as f32 * t) as u8,
            g: (a.g as f32 * inv_t + b.g as f32 * t) as u8,
            b: (a.b as f32 * inv_t + b.b as f32 * t) as u8,
            a: (a.a as f32 * inv_t + b.a as f32 * t) as u8,
        }
    }
}

/// Rectangle in screen coordinates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }

    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x
            && px < self.x + self.width as i32
            && py >= self.y
            && py < self.y + self.height as i32
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.x + other.width as i32
            && self.x + self.width as i32 > other.x
            && self.y < other.y + other.height as i32
            && self.y + self.height as i32 > other.y
    }
}

/// 2D viewport for rendering
#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub min_depth: f32,
    pub max_depth: f32,
}

impl Viewport {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width,
            height,
            min_depth: 0.0,
            max_depth: 1.0,
        }
    }

    pub fn with_offset(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            min_depth: 0.0,
            max_depth: 1.0,
        }
    }
}

impl Default for Viewport {
    fn default() -> Self {
        Self::new(1024.0, 768.0)
    }
}
