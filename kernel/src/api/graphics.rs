//! Graphics API
//!
//! Vulkan-inspired graphics API for GPU communication.
//! Provides a clean abstraction over software and hardware rendering backends.

use super::types::{Color, Dimensions, Handle, KernelError, KernelResult, Rect, Viewport};
use alloc::vec::Vec;

/// Graphics device - main entry point for graphics operations
pub struct GraphicsDevice {
    width: u32,
    height: u32,
    initialized: bool,
}

impl GraphicsDevice {
    /// Create a new graphics device
    pub fn new() -> KernelResult<Self> {
        let (w, h) = crate::graphics::gpu::dimensions();
        if w == 0 || h == 0 {
            return Err(KernelError::DeviceNotAvailable);
        }

        Ok(Self {
            width: w as u32,
            height: h as u32,
            initialized: true,
        })
    }

    /// Get device dimensions
    pub fn dimensions(&self) -> Dimensions {
        Dimensions::new(self.width, self.height)
    }

    /// Check if GPU acceleration is available
    pub fn has_hardware_acceleration(&self) -> bool {
        crate::graphics::gpu::has_hw_accel()
    }

    /// Check if 3D GPU rendering is available
    pub fn has_3d(&self) -> bool {
        crate::graphics::gpu::has_3d()
    }

    /// Get backend name
    pub fn backend_name(&self) -> &'static str {
        crate::graphics::gpu::backend_name()
    }

    /// Create a buffer
    pub fn create_buffer(&mut self, desc: &BufferDesc) -> KernelResult<Buffer> {
        if !self.initialized {
            return Err(KernelError::NotInitialized);
        }

        static NEXT_ID: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(1);
        let id = NEXT_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed);

        Ok(Buffer {
            handle: Handle::new(id),
            size: desc.size,
            usage: desc.usage,
        })
    }

    /// Create a render pipeline
    pub fn create_pipeline(&mut self, desc: &PipelineDesc) -> KernelResult<Pipeline> {
        if !self.initialized {
            return Err(KernelError::NotInitialized);
        }

        static NEXT_ID: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(1);
        let id = NEXT_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed);

        Ok(Pipeline {
            handle: Handle::new(id),
            cull_mode: desc.cull_mode,
            depth_test: desc.depth_test,
            blend_mode: desc.blend_mode,
        })
    }

    /// Create a render pass
    pub fn create_render_pass(&mut self, desc: &RenderPassDesc) -> KernelResult<RenderPass> {
        if !self.initialized {
            return Err(KernelError::NotInitialized);
        }

        static NEXT_ID: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(1);
        let id = NEXT_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed);

        Ok(RenderPass {
            handle: Handle::new(id),
            clear_color: desc.clear_color,
            clear_depth: desc.clear_depth,
        })
    }

    /// Begin recording commands
    pub fn begin_command_buffer(&mut self) -> KernelResult<CommandEncoder> {
        if !self.initialized {
            return Err(KernelError::NotInitialized);
        }

        Ok(CommandEncoder::new(self.width, self.height))
    }

    /// Submit command buffers for execution
    pub fn submit(&mut self, cmd_buffers: &[CommandBuffer]) -> KernelResult<()> {
        if !self.initialized {
            return Err(KernelError::NotInitialized);
        }

        for cmd_buf in cmd_buffers {
            self.execute_commands(cmd_buf)?;
        }

        Ok(())
    }

    /// Present the current frame to the display
    pub fn present(&mut self) -> KernelResult<()> {
        if !self.initialized {
            return Err(KernelError::NotInitialized);
        }

        crate::graphics::gpu::present();
        Ok(())
    }

    /// Execute recorded commands
    fn execute_commands(&self, cmd_buf: &CommandBuffer) -> KernelResult<()> {
        for cmd in &cmd_buf.commands {
            match cmd {
                Command::Clear { color, depth } => {
                    if let Some(c) = color {
                        crate::graphics::gpu::clear(c.to_u32());
                    }
                    if depth.is_some() {
                        crate::graphics::zbuffer::clear();
                    }
                }
                Command::FillRect { rect, color } => {
                    crate::graphics::gpu::fill_rect(
                        rect.x as usize,
                        rect.y as usize,
                        rect.width as usize,
                        rect.height as usize,
                        color.to_u32(),
                    );
                }
                Command::DrawText { x, y, text, color, scale } => {
                    crate::graphics::font::draw_string(
                        *x as usize,
                        *y as usize,
                        text,
                        color.to_u32(),
                        *scale as usize,
                    );
                }
                // Other commands handled by the respective backends
                _ => {}
            }
        }

        Ok(())
    }
}

impl Default for GraphicsDevice {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            initialized: false,
        }
    }
}

/// Buffer handle
#[derive(Debug, Clone)]
pub struct Buffer {
    handle: Handle,
    size: usize,
    usage: BufferUsage,
}

impl Buffer {
    pub fn handle(&self) -> Handle {
        self.handle
    }

    pub fn size(&self) -> usize {
        self.size
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

/// Pipeline handle
#[derive(Debug, Clone)]
pub struct Pipeline {
    handle: Handle,
    cull_mode: CullMode,
    depth_test: bool,
    blend_mode: BlendMode,
}

impl Pipeline {
    pub fn handle(&self) -> Handle {
        self.handle
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
}

/// Pipeline descriptor
#[derive(Debug, Clone)]
pub struct PipelineDesc {
    pub cull_mode: CullMode,
    pub depth_test: bool,
    pub depth_write: bool,
    pub blend_mode: BlendMode,
}

impl Default for PipelineDesc {
    fn default() -> Self {
        Self {
            cull_mode: CullMode::Back,
            depth_test: true,
            depth_write: true,
            blend_mode: BlendMode::Opaque,
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
    pub fn handle(&self) -> Handle {
        self.handle
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

/// Clear value for render pass begin
#[derive(Debug, Clone, Copy)]
pub struct ClearValue {
    pub color: Option<Color>,
    pub depth: Option<f32>,
}

impl Default for ClearValue {
    fn default() -> Self {
        Self {
            color: Some(Color::BLACK),
            depth: Some(1.0),
        }
    }
}

/// Command buffer - recorded GPU commands
#[derive(Debug, Clone)]
pub struct CommandBuffer {
    commands: Vec<Command>,
}

impl CommandBuffer {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }
}

impl Default for CommandBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// GPU command
#[derive(Debug, Clone)]
enum Command {
    Clear {
        color: Option<Color>,
        depth: Option<f32>,
    },
    SetViewport(Viewport),
    BeginRenderPass {
        pass: Handle,
        clear: ClearValue,
    },
    EndRenderPass,
    BindPipeline(Handle),
    BindVertexBuffer {
        binding: u32,
        buffer: Handle,
    },
    BindIndexBuffer(Handle),
    Draw {
        vertex_count: u32,
        first_vertex: u32,
    },
    DrawIndexed {
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        vertex_offset: i32,
    },
    FillRect {
        rect: Rect,
        color: Color,
    },
    DrawText {
        x: i32,
        y: i32,
        text: alloc::string::String,
        color: Color,
        scale: u32,
    },
}

/// Command encoder for recording GPU commands
pub struct CommandEncoder {
    commands: Vec<Command>,
    width: u32,
    height: u32,
    in_render_pass: bool,
}

impl CommandEncoder {
    fn new(width: u32, height: u32) -> Self {
        Self {
            commands: Vec::new(),
            width,
            height,
            in_render_pass: false,
        }
    }

    /// Begin recording
    pub fn begin(&mut self) {
        self.commands.clear();
    }

    /// Clear color and/or depth buffers
    pub fn clear(&mut self, color: Option<Color>, depth: Option<f32>) {
        self.commands.push(Command::Clear { color, depth });
    }

    /// Set viewport
    pub fn set_viewport(&mut self, viewport: Viewport) {
        self.commands.push(Command::SetViewport(viewport));
    }

    /// Begin a render pass
    pub fn begin_render_pass(&mut self, pass: &RenderPass, clear: &ClearValue) {
        self.in_render_pass = true;
        self.commands.push(Command::BeginRenderPass {
            pass: pass.handle,
            clear: *clear,
        });
    }

    /// Bind a pipeline
    pub fn bind_pipeline(&mut self, pipeline: &Pipeline) {
        self.commands.push(Command::BindPipeline(pipeline.handle));
    }

    /// Bind a vertex buffer
    pub fn bind_vertex_buffer(&mut self, binding: u32, buffer: &Buffer) {
        self.commands.push(Command::BindVertexBuffer {
            binding,
            buffer: buffer.handle,
        });
    }

    /// Bind an index buffer
    pub fn bind_index_buffer(&mut self, buffer: &Buffer) {
        self.commands.push(Command::BindIndexBuffer(buffer.handle));
    }

    /// Draw non-indexed primitives
    pub fn draw(&mut self, vertex_count: u32, first_vertex: u32) {
        self.commands.push(Command::Draw {
            vertex_count,
            first_vertex,
        });
    }

    /// Draw indexed primitives
    pub fn draw_indexed(
        &mut self,
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        vertex_offset: i32,
    ) {
        self.commands.push(Command::DrawIndexed {
            index_count,
            instance_count,
            first_index,
            vertex_offset,
        });
    }

    /// End the current render pass
    pub fn end_render_pass(&mut self) {
        if self.in_render_pass {
            self.in_render_pass = false;
            self.commands.push(Command::EndRenderPass);
        }
    }

    /// Fill a rectangle with a color (2D operation)
    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.commands.push(Command::FillRect { rect, color });
    }

    /// Draw text (2D operation)
    pub fn draw_text(&mut self, x: i32, y: i32, text: &str, color: Color, scale: u32) {
        self.commands.push(Command::DrawText {
            x,
            y,
            text: alloc::string::String::from(text),
            color,
            scale,
        });
    }

    /// Finish recording and return the command buffer
    pub fn finish(self) -> CommandBuffer {
        CommandBuffer {
            commands: self.commands,
        }
    }
}
