//! Graphics Device
//!
//! Main entry point for graphics operations. The Device manages all GPU resources
//! and provides methods for creating buffers, pipelines, and submitting commands.

use super::commands::{CommandBuffer, CommandEncoder};
use super::pipeline::{
    BlendMode, Buffer, BufferDesc, BufferUsage, CullMode, Image, ImageDesc, ImageFormat,
    Pipeline, PipelineDesc, RenderPass, RenderPassDesc, Sampler, SamplerDesc,
};
use crate::api::types::{Color, Dimensions, Handle, KernelError, KernelResult};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};

/// Device information
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: &'static str,
    pub width: u32,
    pub height: u32,
    pub has_hardware_acceleration: bool,
    pub has_3d: bool,
}

/// Backend type for the device
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// Software rasterization
    Software,
    /// SVGA3D hardware acceleration
    Svga3D,
}

/// Graphics device - main entry point
pub struct Device {
    width: u32,
    height: u32,
    backend: Backend,
    next_buffer_id: AtomicU32,
    next_pipeline_id: AtomicU32,
    next_image_id: AtomicU32,
    next_render_pass_id: AtomicU32,
    buffers: Vec<BufferState>,
    pipelines: Vec<PipelineState>,
    images: Vec<ImageState>,
    render_passes: Vec<RenderPassState>,
}

struct BufferState {
    id: u32,
    desc: BufferDesc,
    data: Vec<u8>,
}

struct PipelineState {
    id: u32,
    desc: PipelineDesc,
}

struct ImageState {
    id: u32,
    desc: ImageDesc,
    data: Vec<u8>,
}

struct RenderPassState {
    id: u32,
    desc: RenderPassDesc,
}

impl Device {
    /// Create a new graphics device
    pub fn new() -> KernelResult<Self> {
        let (w, h) = crate::graphics::gpu::dimensions();
        if w == 0 || h == 0 {
            return Err(KernelError::DeviceNotAvailable);
        }

        let backend = if crate::graphics::gpu::has_3d() {
            Backend::Svga3D
        } else {
            Backend::Software
        };

        Ok(Self {
            width: w as u32,
            height: h as u32,
            backend,
            next_buffer_id: AtomicU32::new(1),
            next_pipeline_id: AtomicU32::new(1),
            next_image_id: AtomicU32::new(1),
            next_render_pass_id: AtomicU32::new(1),
            buffers: Vec::new(),
            pipelines: Vec::new(),
            images: Vec::new(),
            render_passes: Vec::new(),
        })
    }

    /// Get device information
    pub fn info(&self) -> DeviceInfo {
        DeviceInfo {
            name: crate::graphics::gpu::backend_name(),
            width: self.width,
            height: self.height,
            has_hardware_acceleration: crate::graphics::gpu::has_hw_accel(),
            has_3d: crate::graphics::gpu::has_3d(),
        }
    }

    /// Get device dimensions
    pub fn dimensions(&self) -> Dimensions {
        Dimensions::new(self.width, self.height)
    }

    /// Get the active backend
    pub fn backend(&self) -> Backend {
        self.backend
    }

    /// Create a buffer
    pub fn create_buffer(&mut self, desc: &BufferDesc) -> KernelResult<Buffer> {
        let id = self.next_buffer_id.fetch_add(1, Ordering::Relaxed);
        let data = alloc::vec![0u8; desc.size];

        self.buffers.push(BufferState {
            id,
            desc: desc.clone(),
            data,
        });

        Ok(Buffer::new(Handle::new(id), desc.size, desc.usage))
    }

    /// Upload data to a buffer
    pub fn write_buffer(&mut self, buffer: &Buffer, offset: usize, data: &[u8]) -> KernelResult<()> {
        let state = self.buffers
            .iter_mut()
            .find(|b| b.id == buffer.handle().raw())
            .ok_or(KernelError::InvalidHandle)?;

        if offset + data.len() > state.data.len() {
            return Err(KernelError::InvalidParameter);
        }

        state.data[offset..offset + data.len()].copy_from_slice(data);
        Ok(())
    }

    /// Create a pipeline
    pub fn create_pipeline(&mut self, desc: &PipelineDesc) -> KernelResult<Pipeline> {
        let id = self.next_pipeline_id.fetch_add(1, Ordering::Relaxed);

        self.pipelines.push(PipelineState {
            id,
            desc: desc.clone(),
        });

        Ok(Pipeline::new(Handle::new(id), desc.cull_mode, desc.depth_test, desc.blend_mode))
    }

    /// Create an image
    pub fn create_image(&mut self, desc: &ImageDesc) -> KernelResult<Image> {
        let id = self.next_image_id.fetch_add(1, Ordering::Relaxed);
        let pixel_size = match desc.format {
            ImageFormat::Rgba8 => 4,
            ImageFormat::Bgra8 => 4,
            ImageFormat::R8 => 1,
            ImageFormat::Depth32 => 4,
        };
        let data = alloc::vec![0u8; desc.width as usize * desc.height as usize * pixel_size];

        self.images.push(ImageState {
            id,
            desc: desc.clone(),
            data,
        });

        Ok(Image::new(Handle::new(id), desc.width, desc.height, desc.format))
    }

    /// Create a render pass
    pub fn create_render_pass(&mut self, desc: &RenderPassDesc) -> KernelResult<RenderPass> {
        let id = self.next_render_pass_id.fetch_add(1, Ordering::Relaxed);

        self.render_passes.push(RenderPassState {
            id,
            desc: desc.clone(),
        });

        Ok(RenderPass::new(Handle::new(id), desc.clear_color, desc.clear_depth))
    }

    /// Begin recording commands
    pub fn begin_commands(&mut self) -> CommandEncoder {
        CommandEncoder::new(self.width, self.height, self.backend)
    }

    /// Submit command buffers for execution
    pub fn submit(&mut self, cmd_buffers: &[CommandBuffer]) -> KernelResult<()> {
        for cmd_buf in cmd_buffers {
            self.execute_commands(cmd_buf)?;
        }
        Ok(())
    }

    /// Present the current frame to the display
    pub fn present(&mut self) -> KernelResult<()> {
        crate::graphics::gpu::present();
        Ok(())
    }

    /// Execute a command buffer
    fn execute_commands(&self, cmd_buf: &CommandBuffer) -> KernelResult<()> {
        use super::commands::Command;

        for cmd in cmd_buf.commands() {
            match cmd {
                Command::Clear { color, depth } => {
                    if let Some(c) = color {
                        crate::graphics::gpu::clear(c.to_u32());
                    }
                    if depth.is_some() {
                        crate::graphics::zbuffer::clear();
                    }
                }
                Command::SetViewport { x, y, width, height, .. } => {
                    // Viewport is primarily used for software rendering bounds
                    // Hardware handles this differently
                    let _ = (x, y, width, height);
                }
                Command::BeginRenderPass { clear_color, clear_depth, .. } => {
                    if let Some(c) = clear_color {
                        crate::graphics::gpu::clear(c.to_u32());
                    }
                    if clear_depth.is_some() {
                        crate::graphics::zbuffer::clear();
                    }
                }
                Command::EndRenderPass => {
                    // No-op for now
                }
                Command::BindPipeline(_) => {
                    // Pipeline state is tracked in command encoder
                }
                Command::BindVertexBuffer { .. } => {
                    // Buffer binding tracked in encoder
                }
                Command::BindIndexBuffer(_) => {
                    // Index buffer binding tracked in encoder
                }
                Command::Draw { .. } | Command::DrawIndexed { .. } => {
                    // Draw calls would dispatch to appropriate backend
                    // For now, these are handled by the existing rendering path
                }
                Command::FillRect { x, y, width, height, color } => {
                    crate::graphics::gpu::fill_rect(
                        *x as usize,
                        *y as usize,
                        *width as usize,
                        *height as usize,
                        color.to_u32(),
                    );
                }
                Command::DrawTriangles { triangles } => {
                    // Dispatch to appropriate backend
                    match self.backend {
                        Backend::Software => {
                            // Use existing software rasterizer
                            self.draw_triangles_software(triangles);
                        }
                        Backend::Svga3D => {
                            // Use GPU batch renderer
                            self.draw_triangles_gpu(triangles);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Draw triangles using software rasterization
    fn draw_triangles_software(&self, triangles: &[GpuTriangle]) {
        let ctx = match crate::graphics::rasterizer::RenderContext::acquire() {
            Some(ctx) => ctx,
            None => return,
        };

        for tri in triangles {
            let c0 = Color::from_u32(tri.v0.color);
            let c1 = Color::from_u32(tri.v1.color);
            let c2 = Color::from_u32(tri.v2.color);

            let v0 = renderer::vertex::Vertex::pos_color(
                glam::Vec3::new(tri.v0.x, tri.v0.y, tri.v0.z),
                glam::Vec3::new(c0.r as f32 / 255.0, c0.g as f32 / 255.0, c0.b as f32 / 255.0),
            );
            let v1 = renderer::vertex::Vertex::pos_color(
                glam::Vec3::new(tri.v1.x, tri.v1.y, tri.v1.z),
                glam::Vec3::new(c1.r as f32 / 255.0, c1.g as f32 / 255.0, c1.b as f32 / 255.0),
            );
            let v2 = renderer::vertex::Vertex::pos_color(
                glam::Vec3::new(tri.v2.x, tri.v2.y, tri.v2.z),
                glam::Vec3::new(c2.r as f32 / 255.0, c2.g as f32 / 255.0, c2.b as f32 / 255.0),
            );

            crate::graphics::rasterizer::rasterize_triangle_with_context(&ctx, &v0, &v1, &v2);
        }
    }

    /// Draw triangles using GPU acceleration
    fn draw_triangles_gpu(&self, triangles: &[GpuTriangle]) {
        use crate::graphics::gpu_batch;

        for tri in triangles {
            gpu_batch::add_triangle_verts(
                tri.v0.x, tri.v0.y, tri.v0.z, tri.v0.color,
                tri.v1.x, tri.v1.y, tri.v1.z, tri.v1.color,
                tri.v2.x, tri.v2.y, tri.v2.z, tri.v2.color,
            );
        }
    }
}

/// GPU vertex for triangle submission
#[derive(Debug, Clone, Copy)]
pub struct GpuVertex {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub color: u32,
}

impl GpuVertex {
    pub fn new(x: f32, y: f32, z: f32, color: u32) -> Self {
        Self { x, y, z, color }
    }
}

/// GPU triangle (3 vertices)
#[derive(Debug, Clone, Copy)]
pub struct GpuTriangle {
    pub v0: GpuVertex,
    pub v1: GpuVertex,
    pub v2: GpuVertex,
}

impl GpuTriangle {
    pub fn new(v0: GpuVertex, v1: GpuVertex, v2: GpuVertex) -> Self {
        Self { v0, v1, v2 }
    }
}

