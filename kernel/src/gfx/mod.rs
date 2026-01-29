//! Graphics Abstraction Layer
//!
//! Vulkan-inspired graphics API providing a clean abstraction over
//! software and hardware rendering backends.

pub mod backends;
pub mod commands;
pub mod device;
pub mod pipeline;

pub use commands::{CommandBuffer, CommandEncoder};
pub use device::{Device, DeviceInfo};
pub use pipeline::{
    BlendMode, Buffer, BufferDesc, BufferUsage, CullMode, Image, ImageDesc, ImageFormat,
    Pipeline, PipelineDesc, RenderPass, RenderPassDesc, Sampler, SamplerDesc,
};
