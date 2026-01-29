//! Command Recording
//!
//! Provides command buffers and encoders for recording GPU commands.

use super::device::{Backend, GpuTriangle, GpuVertex};
use super::pipeline::{Buffer, Pipeline, RenderPass};
use crate::api::types::{Color, Handle};
use alloc::vec::Vec;

/// GPU command
#[derive(Debug, Clone)]
pub enum Command {
    /// Clear color and/or depth buffers
    Clear {
        color: Option<Color>,
        depth: Option<f32>,
    },
    /// Set viewport
    SetViewport {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        min_depth: f32,
        max_depth: f32,
    },
    /// Begin a render pass
    BeginRenderPass {
        pass: Handle,
        clear_color: Option<Color>,
        clear_depth: Option<f32>,
    },
    /// End the current render pass
    EndRenderPass,
    /// Bind a pipeline
    BindPipeline(Handle),
    /// Bind a vertex buffer
    BindVertexBuffer {
        binding: u32,
        buffer: Handle,
    },
    /// Bind an index buffer
    BindIndexBuffer(Handle),
    /// Draw non-indexed primitives
    Draw {
        vertex_count: u32,
        first_vertex: u32,
    },
    /// Draw indexed primitives
    DrawIndexed {
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        vertex_offset: i32,
    },
    /// Fill a rectangle (2D operation)
    FillRect {
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        color: Color,
    },
    /// Draw triangles directly
    DrawTriangles {
        triangles: Vec<GpuTriangle>,
    },
}

/// Recorded command buffer
#[derive(Debug, Clone)]
pub struct CommandBuffer {
    commands: Vec<Command>,
}

impl CommandBuffer {
    pub(crate) fn new(commands: Vec<Command>) -> Self {
        Self { commands }
    }

    pub fn commands(&self) -> &[Command] {
        &self.commands
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }
}

impl Default for CommandBuffer {
    fn default() -> Self {
        Self { commands: Vec::new() }
    }
}

/// Command encoder for recording GPU commands
pub struct CommandEncoder {
    commands: Vec<Command>,
    width: u32,
    height: u32,
    backend: Backend,
    in_render_pass: bool,
    current_pipeline: Option<Handle>,
    pending_triangles: Vec<GpuTriangle>,
}

impl CommandEncoder {
    pub(crate) fn new(width: u32, height: u32, backend: Backend) -> Self {
        Self {
            commands: Vec::with_capacity(64),
            width,
            height,
            backend,
            in_render_pass: false,
            current_pipeline: None,
            pending_triangles: Vec::with_capacity(1024),
        }
    }

    /// Get encoder dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Begin recording
    pub fn begin(&mut self) {
        self.commands.clear();
        self.pending_triangles.clear();
    }

    /// Clear color and/or depth buffers
    pub fn clear(&mut self, color: Option<Color>, depth: Option<f32>) {
        self.flush_triangles();
        self.commands.push(Command::Clear { color, depth });
    }

    /// Set viewport
    pub fn set_viewport(&mut self, x: f32, y: f32, width: f32, height: f32, min_depth: f32, max_depth: f32) {
        self.commands.push(Command::SetViewport {
            x,
            y,
            width,
            height,
            min_depth,
            max_depth,
        });
    }

    /// Set viewport to full screen
    pub fn set_full_viewport(&mut self) {
        self.set_viewport(0.0, 0.0, self.width as f32, self.height as f32, 0.0, 1.0);
    }

    /// Begin a render pass
    pub fn begin_render_pass(&mut self, pass: &RenderPass) {
        self.flush_triangles();
        self.in_render_pass = true;
        self.commands.push(Command::BeginRenderPass {
            pass: pass.handle(),
            clear_color: pass.clear_color(),
            clear_depth: pass.clear_depth(),
        });
    }

    /// Bind a pipeline
    pub fn bind_pipeline(&mut self, pipeline: &Pipeline) {
        self.flush_triangles();
        self.current_pipeline = Some(pipeline.handle());
        self.commands.push(Command::BindPipeline(pipeline.handle()));
    }

    /// Bind a vertex buffer
    pub fn bind_vertex_buffer(&mut self, binding: u32, buffer: &Buffer) {
        self.commands.push(Command::BindVertexBuffer {
            binding,
            buffer: buffer.handle(),
        });
    }

    /// Bind an index buffer
    pub fn bind_index_buffer(&mut self, buffer: &Buffer) {
        self.commands.push(Command::BindIndexBuffer(buffer.handle()));
    }

    /// Draw non-indexed primitives
    pub fn draw(&mut self, vertex_count: u32, first_vertex: u32) {
        self.flush_triangles();
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
        self.flush_triangles();
        self.commands.push(Command::DrawIndexed {
            index_count,
            instance_count,
            first_index,
            vertex_offset,
        });
    }

    /// Add a triangle to the batch
    pub fn add_triangle(&mut self, v0: GpuVertex, v1: GpuVertex, v2: GpuVertex) {
        self.pending_triangles.push(GpuTriangle::new(v0, v1, v2));

        // Flush if we have too many triangles
        if self.pending_triangles.len() >= 1024 {
            self.flush_triangles();
        }
    }

    /// Add a screen-space triangle (already transformed)
    pub fn add_screen_triangle(
        &mut self,
        x0: f32, y0: f32, z0: f32, c0: u32,
        x1: f32, y1: f32, z1: f32, c1: u32,
        x2: f32, y2: f32, z2: f32, c2: u32,
    ) {
        let v0 = GpuVertex::new(x0, y0, z0, c0);
        let v1 = GpuVertex::new(x1, y1, z1, c1);
        let v2 = GpuVertex::new(x2, y2, z2, c2);
        self.add_triangle(v0, v1, v2);
    }

    /// End the current render pass
    pub fn end_render_pass(&mut self) {
        if self.in_render_pass {
            self.flush_triangles();
            self.in_render_pass = false;
            self.commands.push(Command::EndRenderPass);
        }
    }

    /// Fill a rectangle with a color (2D operation)
    pub fn fill_rect(&mut self, x: i32, y: i32, width: u32, height: u32, color: Color) {
        self.flush_triangles();
        self.commands.push(Command::FillRect {
            x,
            y,
            width,
            height,
            color,
        });
    }

    /// Flush pending triangles
    fn flush_triangles(&mut self) {
        if !self.pending_triangles.is_empty() {
            let triangles = core::mem::take(&mut self.pending_triangles);
            self.commands.push(Command::DrawTriangles { triangles });
        }
    }

    /// Finish recording and return the command buffer
    pub fn finish(mut self) -> CommandBuffer {
        self.flush_triangles();
        CommandBuffer::new(self.commands)
    }
}
