//! Graphics Pipeline Types
//!
//! Defines buffers, images, pipelines, and render passes.

use crate::api::types::{Color, Handle};

/// Buffer handle
#[derive(Debug, Clone)]
pub struct Buffer {
    handle: Handle,
    size: usize,
    usage: BufferUsage,
}

impl Buffer {
    pub(crate) fn new(handle: Handle, size: usize, usage: BufferUsage) -> Self {
        Self { handle, size, usage }
    }

    pub fn handle(&self) -> Handle {
        self.handle
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn usage(&self) -> BufferUsage {
        self.usage
    }
}

/// Buffer usage flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferUsage {
    /// Vertex buffer
    Vertex,
    /// Index buffer
    Index,
    /// Uniform buffer
    Uniform,
    /// Storage buffer
    Storage,
}

/// Buffer descriptor
#[derive(Debug, Clone)]
pub struct BufferDesc {
    pub size: usize,
    pub usage: BufferUsage,
}

impl BufferDesc {
    pub fn vertex(size: usize) -> Self {
        Self {
            size,
            usage: BufferUsage::Vertex,
        }
    }

    pub fn index(size: usize) -> Self {
        Self {
            size,
            usage: BufferUsage::Index,
        }
    }

    pub fn uniform(size: usize) -> Self {
        Self {
            size,
            usage: BufferUsage::Uniform,
        }
    }
}

/// Image format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Rgba8,
    Bgra8,
    R8,
    Depth32,
}

impl ImageFormat {
    pub fn bytes_per_pixel(&self) -> usize {
        match self {
            Self::Rgba8 | Self::Bgra8 | Self::Depth32 => 4,
            Self::R8 => 1,
        }
    }
}

/// Image handle
#[derive(Debug, Clone)]
pub struct Image {
    handle: Handle,
    width: u32,
    height: u32,
    format: ImageFormat,
}

impl Image {
    pub(crate) fn new(handle: Handle, width: u32, height: u32, format: ImageFormat) -> Self {
        Self {
            handle,
            width,
            height,
            format,
        }
    }

    pub fn handle(&self) -> Handle {
        self.handle
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn format(&self) -> ImageFormat {
        self.format
    }
}

/// Image descriptor
#[derive(Debug, Clone)]
pub struct ImageDesc {
    pub width: u32,
    pub height: u32,
    pub format: ImageFormat,
    pub usage: ImageUsage,
}

/// Image usage flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageUsage {
    /// Used as a texture
    Texture,
    /// Used as a render target
    RenderTarget,
    /// Used as a depth buffer
    DepthBuffer,
}

impl ImageDesc {
    pub fn texture(width: u32, height: u32, format: ImageFormat) -> Self {
        Self {
            width,
            height,
            format,
            usage: ImageUsage::Texture,
        }
    }

    pub fn render_target(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            format: ImageFormat::Rgba8,
            usage: ImageUsage::RenderTarget,
        }
    }

    pub fn depth_buffer(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            format: ImageFormat::Depth32,
            usage: ImageUsage::DepthBuffer,
        }
    }
}

/// Sampler handle
#[derive(Debug, Clone)]
pub struct Sampler {
    handle: Handle,
}

impl Sampler {
    pub(crate) fn new(handle: Handle) -> Self {
        Self { handle }
    }

    pub fn handle(&self) -> Handle {
        self.handle
    }
}

/// Sampler descriptor
#[derive(Debug, Clone)]
pub struct SamplerDesc {
    pub filter: FilterMode,
    pub address_mode: AddressMode,
}

/// Texture filter mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FilterMode {
    #[default]
    Nearest,
    Linear,
}

/// Texture address mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AddressMode {
    #[default]
    Clamp,
    Repeat,
    Mirror,
}

impl Default for SamplerDesc {
    fn default() -> Self {
        Self {
            filter: FilterMode::Nearest,
            address_mode: AddressMode::Clamp,
        }
    }
}

/// Culling mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CullMode {
    /// No culling
    None,
    /// Cull front faces
    Front,
    /// Cull back faces (default)
    #[default]
    Back,
}

/// Blend mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlendMode {
    /// No blending (opaque)
    #[default]
    Opaque,
    /// Alpha blending
    Alpha,
    /// Additive blending
    Additive,
    /// Multiplicative blending
    Multiply,
}

/// Depth compare function
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompareFunc {
    /// Never pass
    Never,
    /// Pass if less than
    #[default]
    Less,
    /// Pass if equal
    Equal,
    /// Pass if less than or equal
    LessEqual,
    /// Pass if greater
    Greater,
    /// Pass if not equal
    NotEqual,
    /// Pass if greater than or equal
    GreaterEqual,
    /// Always pass
    Always,
}

/// Pipeline handle
#[derive(Debug, Clone)]
pub struct Pipeline {
    handle: Handle,
    cull_mode: CullMode,
    depth_test: bool,
    blend_mode: BlendMode,
}

impl Pipeline {
    pub(crate) fn new(handle: Handle, cull_mode: CullMode, depth_test: bool, blend_mode: BlendMode) -> Self {
        Self {
            handle,
            cull_mode,
            depth_test,
            blend_mode,
        }
    }

    pub fn handle(&self) -> Handle {
        self.handle
    }

    pub fn cull_mode(&self) -> CullMode {
        self.cull_mode
    }

    pub fn depth_test(&self) -> bool {
        self.depth_test
    }

    pub fn blend_mode(&self) -> BlendMode {
        self.blend_mode
    }
}

/// Pipeline descriptor
#[derive(Debug, Clone)]
pub struct PipelineDesc {
    pub cull_mode: CullMode,
    pub depth_test: bool,
    pub depth_write: bool,
    pub depth_func: CompareFunc,
    pub blend_mode: BlendMode,
}

impl Default for PipelineDesc {
    fn default() -> Self {
        Self {
            cull_mode: CullMode::Back,
            depth_test: true,
            depth_write: true,
            depth_func: CompareFunc::Less,
            blend_mode: BlendMode::Opaque,
        }
    }
}

impl PipelineDesc {
    /// Create an opaque pipeline (default settings)
    pub fn opaque() -> Self {
        Self::default()
    }

    /// Create a transparent pipeline with alpha blending
    pub fn transparent() -> Self {
        Self {
            cull_mode: CullMode::None,
            depth_test: true,
            depth_write: false,
            depth_func: CompareFunc::Less,
            blend_mode: BlendMode::Alpha,
        }
    }

    /// Create a 2D pipeline (no depth test)
    pub fn ui() -> Self {
        Self {
            cull_mode: CullMode::None,
            depth_test: false,
            depth_write: false,
            depth_func: CompareFunc::Always,
            blend_mode: BlendMode::Alpha,
        }
    }
}

/// Render pass handle
#[derive(Debug, Clone)]
pub struct RenderPass {
    handle: Handle,
    clear_color: Option<Color>,
    clear_depth: Option<f32>,
}

impl RenderPass {
    pub(crate) fn new(handle: Handle, clear_color: Option<Color>, clear_depth: Option<f32>) -> Self {
        Self {
            handle,
            clear_color,
            clear_depth,
        }
    }

    pub fn handle(&self) -> Handle {
        self.handle
    }

    pub fn clear_color(&self) -> Option<Color> {
        self.clear_color
    }

    pub fn clear_depth(&self) -> Option<f32> {
        self.clear_depth
    }
}

/// Render pass descriptor
#[derive(Debug, Clone)]
pub struct RenderPassDesc {
    pub clear_color: Option<Color>,
    pub clear_depth: Option<f32>,
}

impl Default for RenderPassDesc {
    fn default() -> Self {
        Self {
            clear_color: Some(Color::BLACK),
            clear_depth: Some(1.0),
        }
    }
}

impl RenderPassDesc {
    /// Create a render pass that clears to a specific color
    pub fn clear(color: Color) -> Self {
        Self {
            clear_color: Some(color),
            clear_depth: Some(1.0),
        }
    }

    /// Create a render pass that doesn't clear (continues from previous)
    pub fn load() -> Self {
        Self {
            clear_color: None,
            clear_depth: None,
        }
    }
}
