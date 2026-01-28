//! GPU Triangle Batch Renderer
//!
//! This module provides GPU-accelerated triangle rasterization using SVGA3D.
//! Triangles are collected during the frame, uploaded to GPU memory via GMR,
//! and rasterized with DRAW_PRIMITIVES commands.
//!
//! Flow:
//! 1. begin_batch() - Start collecting triangles
//! 2. add_triangle() - Add transformed screen-space triangles
//! 3. flush_batch() - Upload to GPU and draw
//! 4. end_batch() - Present to screen
//!
//! The renderer uses a hybrid approach:
//! - When GPU 3D is available: triangles are batched and drawn with DRAW_PRIMITIVES
//! - When GPU 3D is unavailable: falls back to software rasterization

use crate::drivers::vmsvga::{self, gmr, svga3d};
use crate::graphics::gpu;
use crate::serial_println;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use spin::Mutex;

/// Maximum triangles per batch (limited by GMR size)
/// Each triangle = 3 vertices * 16 bytes = 48 bytes
/// 64KB GMR = ~1365 triangles per batch
pub const MAX_TRIANGLES_PER_BATCH: usize = 1024;

/// Vertex size in bytes (position xyz + color)
pub const VERTEX_SIZE: usize = 16; // 3 floats (12) + 1 u32 color (4)

/// Triangle size in bytes
pub const TRIANGLE_SIZE: usize = VERTEX_SIZE * 3;

/// Batch buffer size
pub const BATCH_BUFFER_SIZE: usize = MAX_TRIANGLES_PER_BATCH * TRIANGLE_SIZE;

/// GPU vertex format (matches SVGA3D expectations)
/// Layout: float3 position, DWORD color (D3DCOLOR format)
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct GpuVertex {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub color: u32, // ARGB (D3DCOLOR)
}

impl GpuVertex {
    #[inline]
    pub fn new(x: f32, y: f32, z: f32, color: u32) -> Self {
        Self { x, y, z, color }
    }
}

/// GPU triangle (3 vertices)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GpuTriangle {
    pub v0: GpuVertex,
    pub v1: GpuVertex,
    pub v2: GpuVertex,
}

impl GpuTriangle {
    #[inline]
    pub fn new(v0: GpuVertex, v1: GpuVertex, v2: GpuVertex) -> Self {
        Self { v0, v1, v2 }
    }
}

/// GPU batch state
pub struct GpuBatch {
    /// Whether GPU batching is enabled
    enabled: bool,
    /// GMR ID for vertex buffer
    gmr_id: Option<u32>,
    /// Vertex buffer surface ID
    vertex_surface_id: Option<u32>,
    /// Pointer to write vertices
    vertex_ptr: Option<*mut u8>,
    /// Current triangle count in batch
    triangle_count: usize,
    /// Total triangles rendered this frame
    frame_triangle_count: usize,
    /// Number of batches flushed this frame
    batch_count: usize,
    /// Frame counter
    frame_count: u64,
    /// Screen width
    width: u32,
    /// Screen height
    height: u32,
    /// 3D context ID
    context_id: Option<u32>,
    /// Color render target surface ID
    color_target_id: Option<u32>,
    /// Depth buffer surface ID
    depth_target_id: Option<u32>,
    /// CPU fallback buffer (when GPU not available)
    cpu_triangles: Vec<GpuTriangle>,
    /// Whether 3D resources are initialized
    resources_initialized: bool,
}

impl GpuBatch {
    pub const fn new() -> Self {
        Self {
            enabled: false,
            gmr_id: None,
            vertex_surface_id: None,
            vertex_ptr: None,
            triangle_count: 0,
            frame_triangle_count: 0,
            batch_count: 0,
            frame_count: 0,
            width: 0,
            height: 0,
            context_id: None,
            color_target_id: None,
            depth_target_id: None,
            cpu_triangles: Vec::new(),
            resources_initialized: false,
        }
    }
}

// Safety: GpuBatch is protected by a Mutex and the vertex_ptr points to
// GMR (Guest Memory Region) which is valid memory-mapped GPU buffer that
// can be safely accessed from any CPU thread.
unsafe impl Send for GpuBatch {}

/// Global GPU batch state
static GPU_BATCH: Mutex<GpuBatch> = Mutex::new(GpuBatch::new());

/// Whether GPU batch rendering is active for this frame
static BATCH_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Triangle count for current batch (lock-free for hot path)
static BATCH_TRI_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Initialize GPU batch renderer
pub fn init(width: u32, height: u32) -> bool {
    let mut batch = GPU_BATCH.lock();
    batch.width = width;
    batch.height = height;

    // Check if GPU 3D is available
    if !gpu::has_3d() {
        serial_println!("GPU Batch: No GPU 3D support, using software fallback");
        batch.enabled = false;
        batch.cpu_triangles = Vec::with_capacity(MAX_TRIANGLES_PER_BATCH);
        return false;
    }

    // Initialize 3D resources
    if !init_3d_resources(&mut batch, width, height) {
        serial_println!("GPU Batch: Failed to initialize 3D resources, using software fallback");
        batch.enabled = false;
        batch.cpu_triangles = Vec::with_capacity(MAX_TRIANGLES_PER_BATCH);
        return false;
    }

    batch.enabled = true;
    batch.resources_initialized = true;
    serial_println!(
        "GPU Batch: Initialized with context {}, vertex surface {}, GMR {}",
        batch.context_id.unwrap_or(0),
        batch.vertex_surface_id.unwrap_or(0),
        batch.gmr_id.unwrap_or(0)
    );

    true
}

/// Initialize SVGA3D resources for batch rendering
fn init_3d_resources(batch: &mut GpuBatch, width: u32, height: u32) -> bool {
    // Create 3D context
    let cid = match vmsvga::create_3d_context() {
        Some(id) => id,
        None => {
            serial_println!("GPU Batch: Failed to create 3D context");
            return false;
        }
    };
    batch.context_id = Some(cid);

    // Create color render target surface
    let color_sid = match vmsvga::create_3d_surface(
        svga3d::SurfaceFormat::A8R8G8B8,
        width,
        height,
        1,
        svga3d::surface_flags::HINT_RENDERTARGET,
        1,
    ) {
        Some(id) => id,
        None => {
            serial_println!("GPU Batch: Failed to create color surface");
            return false;
        }
    };
    batch.color_target_id = Some(color_sid);

    // Create depth buffer surface
    let depth_sid = match vmsvga::create_3d_surface(
        svga3d::SurfaceFormat::ZD24S8,
        width,
        height,
        1,
        svga3d::surface_flags::HINT_DEPTHSTENCIL,
        1,
    ) {
        Some(id) => id,
        None => {
            serial_println!("GPU Batch: Failed to create depth surface");
            return false;
        }
    };
    batch.depth_target_id = Some(depth_sid);

    // Set render targets
    if !vmsvga::set_3d_render_target(cid, color_sid, Some(depth_sid)) {
        serial_println!("GPU Batch: Failed to set render targets");
        return false;
    }

    // Set viewport
    if !vmsvga::set_3d_viewport(cid, 0.0, 0.0, width as f32, height as f32) {
        serial_println!("GPU Batch: Failed to set viewport");
        return false;
    }

    // Set up orthographic projection for screen-space triangles
    // Screen coordinates: (0,0) top-left to (width, height) bottom-right
    // Depth: 0.0 (near) to 1.0 (far)
    let ortho = svga3d::Matrix4x4 {
        m: [
            [2.0 / width as f32, 0.0, 0.0, 0.0],
            [0.0, -2.0 / height as f32, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [-1.0, 1.0, 0.0, 1.0],
        ],
    };

    if !vmsvga::set_3d_transform(cid, svga3d::TransformType::Projection, &ortho) {
        serial_println!("GPU Batch: Failed to set projection");
        return false;
    }

    // Set identity transforms for view and world
    let identity = svga3d::Matrix4x4::identity();
    vmsvga::set_3d_transform(cid, svga3d::TransformType::View, &identity);
    vmsvga::set_3d_transform(cid, svga3d::TransformType::World, &identity);

    // Allocate GMR for vertex buffer
    if gmr::is_supported() {
        if let Some(gmr_id) = gmr::alloc(BATCH_BUFFER_SIZE) {
            batch.gmr_id = Some(gmr_id);
            batch.vertex_ptr = gmr::get_write_ptr(gmr_id);
            serial_println!("GPU Batch: Allocated GMR {} for vertex buffer", gmr_id);
        } else {
            serial_println!("GPU Batch: Failed to allocate GMR");
            return false;
        }
    } else {
        serial_println!("GPU Batch: GMR not supported");
        return false;
    }

    // Create vertex buffer surface
    if let Some(sid) = vmsvga::create_3d_surface(
        svga3d::SurfaceFormat::Buffer,
        BATCH_BUFFER_SIZE as u32,
        1,
        1,
        svga3d::surface_flags::HINT_VERTEXBUFFER | svga3d::surface_flags::HINT_DYNAMIC,
        1,
    ) {
        batch.vertex_surface_id = Some(sid);
        serial_println!("GPU Batch: Created vertex surface {}", sid);
    } else {
        serial_println!("GPU Batch: Failed to create vertex surface");
        return false;
    }

    // Set up render states
    setup_render_states(cid);

    true
}

/// Set up default render states for batch rendering
fn setup_render_states(cid: u32) {
    let device = vmsvga::VMSVGA_DEVICE.lock();

    // Enable depth testing
    device.fifo().cmd_3d_set_render_state(cid, &[
        (svga3d::RenderStateId::ZEnable as u32, 1),
        (svga3d::RenderStateId::ZWriteEnable as u32, 1),
        (svga3d::RenderStateId::ZFunc as u32, 4), // LESSEQUAL
        (svga3d::RenderStateId::CullMode as u32, svga3d::CullMode::None as u32),
        (svga3d::RenderStateId::FillMode as u32, svga3d::FillMode::Solid as u32),
        (svga3d::RenderStateId::ShadeMode as u32, 2), // GOURAUD
        (svga3d::RenderStateId::ColorWriteEnable as u32, 0xF), // RGBA
    ]);
}

/// Check if GPU batch rendering is enabled
pub fn is_enabled() -> bool {
    GPU_BATCH.lock().enabled
}

/// Begin a new batch (call at start of frame)
pub fn begin_batch() {
    let mut batch = GPU_BATCH.lock();
    batch.triangle_count = 0;
    batch.frame_triangle_count = 0;
    batch.batch_count = 0;
    batch.frame_count += 1;

    if !batch.enabled {
        batch.cpu_triangles.clear();
    } else {
        // Clear GPU render targets
        if let Some(cid) = batch.context_id {
            // Clear to sky blue
            vmsvga::clear_3d(cid, 0xFF87CEEB, 1.0);
        }
    }

    BATCH_TRI_COUNT.store(0, Ordering::Release);
    BATCH_ACTIVE.store(true, Ordering::Release);
}

/// Add a triangle to the current batch
/// Returns true if added, false if batch is full (caller should flush)
#[inline]
pub fn add_triangle(tri: GpuTriangle) -> bool {
    if !BATCH_ACTIVE.load(Ordering::Acquire) {
        return false;
    }

    let count = BATCH_TRI_COUNT.fetch_add(1, Ordering::AcqRel);
    if count >= MAX_TRIANGLES_PER_BATCH {
        BATCH_TRI_COUNT.fetch_sub(1, Ordering::AcqRel);
        return false;
    }

    let mut batch = GPU_BATCH.lock();

    if batch.enabled {
        // Write directly to GMR buffer
        if let Some(ptr) = batch.vertex_ptr {
            unsafe {
                let offset = count * TRIANGLE_SIZE;
                let dst = ptr.add(offset) as *mut GpuTriangle;
                core::ptr::write_volatile(dst, tri);
            }
        }
    } else {
        // CPU fallback
        batch.cpu_triangles.push(tri);
    }

    batch.triangle_count = count + 1;
    true
}

/// Add a triangle with vertex data
#[inline]
pub fn add_triangle_verts(
    x0: f32, y0: f32, z0: f32, c0: u32,
    x1: f32, y1: f32, z1: f32, c1: u32,
    x2: f32, y2: f32, z2: f32, c2: u32,
) -> bool {
    add_triangle(GpuTriangle::new(
        GpuVertex::new(x0, y0, z0, c0),
        GpuVertex::new(x1, y1, z1, c1),
        GpuVertex::new(x2, y2, z2, c2),
    ))
}

/// Flush the current batch to GPU
/// Call this when batch is full or at end of frame
pub fn flush_batch() -> usize {
    let mut batch = GPU_BATCH.lock();
    let count = batch.triangle_count;

    if count == 0 {
        return 0;
    }

    if batch.enabled {
        // GPU path: DMA upload and draw
        flush_gpu_batch(&batch, count);
    }
    // CPU fallback: triangles are already in cpu_triangles, caller handles rasterization

    batch.frame_triangle_count += count;
    batch.batch_count += 1;
    batch.triangle_count = 0;
    BATCH_TRI_COUNT.store(0, Ordering::Release);

    count
}

/// Flush batch using GPU
fn flush_gpu_batch(batch: &GpuBatch, count: usize) {
    let gmr_id = match batch.gmr_id {
        Some(id) => id,
        None => return,
    };

    let vertex_surface_id = match batch.vertex_surface_id {
        Some(id) => id,
        None => return,
    };

    let cid = match batch.context_id {
        Some(id) => id,
        None => return,
    };

    // Memory barrier to ensure all writes are visible
    core::sync::atomic::fence(Ordering::SeqCst);

    let device = vmsvga::VMSVGA_DEVICE.lock();
    let fifo = device.fifo();

    // Upload vertex data from GMR to vertex surface
    let data_size = count * TRIANGLE_SIZE;
    if !fifo.cmd_3d_upload_vertex_buffer(gmr_id, vertex_surface_id, data_size as u32) {
        serial_println!("GPU Batch: Failed to upload vertex buffer");
        return;
    }

    // Draw the triangles
    let num_vertices = (count * 3) as u32;
    if !fifo.cmd_3d_draw_primitives_simple(
        cid,
        vertex_surface_id,
        num_vertices,
        VERTEX_SIZE as u32,
    ) {
        serial_println!("GPU Batch: Failed to draw primitives");
        return;
    }

    // Sync to ensure drawing is complete before next batch
    // (Only needed if we're doing multiple batches per frame)
    if batch.batch_count > 0 {
        fifo.sync();
    }
}

/// End the batch and present
pub fn end_batch() {
    // Flush any remaining triangles
    flush_batch();

    BATCH_ACTIVE.store(false, Ordering::Release);

    let batch = GPU_BATCH.lock();

    if batch.enabled {
        // Present GPU render target to screen
        if let Some(color_sid) = batch.color_target_id {
            vmsvga::present_3d(color_sid, batch.width, batch.height);
        }
    }

    // Log stats periodically
    if batch.frame_count % 300 == 0 && batch.frame_triangle_count > 0 {
        serial_println!(
            "GPU Batch: frame {} - {} triangles in {} batches (gpu={})",
            batch.frame_count,
            batch.frame_triangle_count,
            batch.batch_count,
            batch.enabled
        );
    }
}

/// Get the CPU triangle buffer for software fallback
pub fn get_cpu_triangles() -> Vec<GpuTriangle> {
    let mut batch = GPU_BATCH.lock();
    core::mem::take(&mut batch.cpu_triangles)
}

/// Get frame statistics
pub fn get_stats() -> (u64, usize, usize, bool) {
    let batch = GPU_BATCH.lock();
    (batch.frame_count, batch.frame_triangle_count, batch.batch_count, batch.enabled)
}

/// Check if batch is active
pub fn is_active() -> bool {
    BATCH_ACTIVE.load(Ordering::Acquire)
}

/// Get current triangle count
pub fn current_count() -> usize {
    BATCH_TRI_COUNT.load(Ordering::Acquire)
}

/// Check if GPU batch is full and needs flushing
pub fn needs_flush() -> bool {
    BATCH_TRI_COUNT.load(Ordering::Acquire) >= MAX_TRIANGLES_PER_BATCH
}

/// Convert screen-space triangle from the tiles system to GPU batch format
/// The tiles system uses ScreenTriangle with pre-computed edge coefficients,
/// but we need screen-space vertex positions for GPU rendering
pub fn add_screen_triangle(
    x0: f32, y0: f32, z0: f32, r0: f32, g0: f32, b0: f32,
    x1: f32, y1: f32, z1: f32, r1: f32, g1: f32, b1: f32,
    x2: f32, y2: f32, z2: f32, r2: f32, g2: f32, b2: f32,
) -> bool {
    // Convert colors to ARGB format
    let color0 = color_to_argb(r0, g0, b0);
    let color1 = color_to_argb(r1, g1, b1);
    let color2 = color_to_argb(r2, g2, b2);

    add_triangle_verts(
        x0, y0, z0, color0,
        x1, y1, z1, color1,
        x2, y2, z2, color2,
    )
}

/// Convert RGB (0-1 range) to ARGB packed format
#[inline]
fn color_to_argb(r: f32, g: f32, b: f32) -> u32 {
    let ri = (r.clamp(0.0, 1.0) * 255.0) as u32;
    let gi = (g.clamp(0.0, 1.0) * 255.0) as u32;
    let bi = (b.clamp(0.0, 1.0) * 255.0) as u32;
    0xFF000000 | (ri << 16) | (gi << 8) | bi
}
