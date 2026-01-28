//! GPU 3D Rendering using SVGA3D
//!
//! This module provides hardware-accelerated 3D rendering using the
//! VMware SVGA3D protocol. It integrates with the game's rendering
//! pipeline to offload rasterization to the GPU.

use crate::drivers::vmsvga;
use crate::drivers::vmsvga::svga3d::{
    self, DeclType, DeclUsage, Matrix4x4, PrimitiveType, RenderStateId,
    SurfaceFormat, TransformType, CullMode, FillMode,
};
use crate::serial_println;
use alloc::vec::Vec;
use spin::Mutex;

/// GPU 3D rendering state
pub struct Gpu3dState {
    /// Render context ID
    pub context_id: Option<u32>,
    /// Color render target surface ID
    pub color_target: Option<u32>,
    /// Depth buffer surface ID
    pub depth_target: Option<u32>,
    /// Vertex buffer surface ID
    pub vertex_buffer: Option<u32>,
    /// Index buffer surface ID
    pub index_buffer: Option<u32>,
    /// Screen width
    pub width: u32,
    /// Screen height
    pub height: u32,
    /// Is GPU 3D initialized and ready
    pub ready: bool,
}

impl Gpu3dState {
    pub const fn new() -> Self {
        Self {
            context_id: None,
            color_target: None,
            depth_target: None,
            vertex_buffer: None,
            index_buffer: None,
            width: 0,
            height: 0,
            ready: false,
        }
    }
}

/// Global GPU 3D state
pub static GPU3D_STATE: Mutex<Gpu3dState> = Mutex::new(Gpu3dState::new());

/// Initialize GPU 3D rendering
/// Returns true if GPU 3D is available and initialized
pub fn init(width: u32, height: u32) -> bool {
    // Check if SVGA3D is available
    if !vmsvga::is_3d_available() {
        serial_println!("GPU3D: SVGA3D not available");
        return false;
    }

    // Create rendering context
    let cid = match vmsvga::create_3d_context() {
        Some(id) => id,
        None => {
            serial_println!("GPU3D: Failed to create context");
            return false;
        }
    };

    serial_println!("GPU3D: Created context {}", cid);

    // Create color render target surface
    let color_sid = match vmsvga::create_3d_surface(
        SurfaceFormat::A8R8G8B8,
        width,
        height,
        1,
        svga3d::surface_flags::HINT_RENDERTARGET,
        1,
    ) {
        Some(id) => id,
        None => {
            serial_println!("GPU3D: Failed to create color target");
            vmsvga::destroy_3d_context(cid);
            return false;
        }
    };

    serial_println!("GPU3D: Created color target {}", color_sid);

    // Create depth buffer surface
    let depth_sid = match vmsvga::create_3d_surface(
        SurfaceFormat::ZD24S8,
        width,
        height,
        1,
        svga3d::surface_flags::HINT_DEPTHSTENCIL,
        1,
    ) {
        Some(id) => id,
        None => {
            serial_println!("GPU3D: Failed to create depth buffer");
            vmsvga::destroy_3d_surface(color_sid);
            vmsvga::destroy_3d_context(cid);
            return false;
        }
    };

    serial_println!("GPU3D: Created depth buffer {}", depth_sid);

    // Set render targets
    if !vmsvga::set_3d_render_target(cid, color_sid, Some(depth_sid)) {
        serial_println!("GPU3D: Failed to set render targets");
        vmsvga::destroy_3d_surface(depth_sid);
        vmsvga::destroy_3d_surface(color_sid);
        vmsvga::destroy_3d_context(cid);
        return false;
    }

    // Set viewport
    if !vmsvga::set_3d_viewport(cid, 0.0, 0.0, width as f32, height as f32) {
        serial_println!("GPU3D: Failed to set viewport");
        vmsvga::destroy_3d_surface(depth_sid);
        vmsvga::destroy_3d_surface(color_sid);
        vmsvga::destroy_3d_context(cid);
        return false;
    }

    // Set default render states
    set_default_render_states(cid);

    // Update global state
    let mut state = GPU3D_STATE.lock();
    state.context_id = Some(cid);
    state.color_target = Some(color_sid);
    state.depth_target = Some(depth_sid);
    state.width = width;
    state.height = height;
    state.ready = true;

    serial_println!("GPU3D: Initialized {}x{}", width, height);
    true
}

/// Set default render states for 3D rendering
fn set_default_render_states(cid: u32) {
    let device = vmsvga::VMSVGA_DEVICE.lock();

    // Enable depth testing
    let states = [
        (RenderStateId::ZEnable as u32, 1),           // Enable z-buffer
        (RenderStateId::ZWriteEnable as u32, 1),       // Enable z-write
        (RenderStateId::CullMode as u32, CullMode::Ccw as u32), // Back-face culling
        (RenderStateId::FillMode as u32, FillMode::Solid as u32), // Solid fill
        (RenderStateId::ColorWriteEnable as u32, 0x0F), // Write all color channels
        (RenderStateId::BlendEnable as u32, 0),        // Disable blending
        (RenderStateId::AlphaTestEnable as u32, 0),    // Disable alpha test
    ];

    device.fifo().cmd_3d_set_render_state(cid, &states);
}

/// Check if GPU 3D is ready
pub fn is_ready() -> bool {
    let state = GPU3D_STATE.lock();
    state.ready
}

/// Begin a new frame
pub fn begin_frame() -> bool {
    let state = GPU3D_STATE.lock();
    if !state.ready {
        return false;
    }

    let cid = match state.context_id {
        Some(id) => id,
        None => return false,
    };

    // Clear to sky blue color with max depth
    vmsvga::clear_3d(cid, 0xFF87CEEB, 1.0)
}

/// Clear with specific color
pub fn clear(color: u32, depth: f32) -> bool {
    let state = GPU3D_STATE.lock();
    if !state.ready {
        return false;
    }

    let cid = match state.context_id {
        Some(id) => id,
        None => return false,
    };

    vmsvga::clear_3d(cid, color, depth)
}

/// Set the view transformation matrix
pub fn set_view_matrix(matrix: &Matrix4x4) -> bool {
    let state = GPU3D_STATE.lock();
    if !state.ready {
        return false;
    }

    let cid = match state.context_id {
        Some(id) => id,
        None => return false,
    };

    vmsvga::set_3d_transform(cid, TransformType::View, matrix)
}

/// Set the projection transformation matrix
pub fn set_projection_matrix(matrix: &Matrix4x4) -> bool {
    let state = GPU3D_STATE.lock();
    if !state.ready {
        return false;
    }

    let cid = match state.context_id {
        Some(id) => id,
        None => return false,
    };

    vmsvga::set_3d_transform(cid, TransformType::Projection, matrix)
}

/// Set the world transformation matrix
pub fn set_world_matrix(matrix: &Matrix4x4) -> bool {
    let state = GPU3D_STATE.lock();
    if !state.ready {
        return false;
    }

    let cid = match state.context_id {
        Some(id) => id,
        None => return false,
    };

    vmsvga::set_3d_transform(cid, TransformType::World, matrix)
}

/// End frame and present to screen
pub fn end_frame() -> bool {
    let state = GPU3D_STATE.lock();
    if !state.ready {
        return false;
    }

    let sid = match state.color_target {
        Some(id) => id,
        None => return false,
    };

    let width = state.width;
    let height = state.height;
    drop(state);

    // Present the render target to screen
    let result = vmsvga::present_3d(sid, width, height);

    // Sync to ensure frame is displayed
    vmsvga::sync_3d();

    result
}

/// Create a vertex buffer for mesh data
/// Returns surface ID on success
pub fn create_vertex_buffer(size_bytes: u32) -> Option<u32> {
    vmsvga::create_3d_surface(
        SurfaceFormat::Buffer,
        size_bytes,
        1,
        1,
        svga3d::surface_flags::HINT_VERTEXBUFFER,
        1,
    )
}

/// Create an index buffer for mesh indices
/// Returns surface ID on success
pub fn create_index_buffer(size_bytes: u32) -> Option<u32> {
    vmsvga::create_3d_surface(
        SurfaceFormat::Buffer,
        size_bytes,
        1,
        1,
        svga3d::surface_flags::HINT_INDEXBUFFER,
        1,
    )
}

/// Destroy a buffer
pub fn destroy_buffer(sid: u32) -> bool {
    vmsvga::destroy_3d_surface(sid)
}

/// Shutdown GPU 3D rendering
pub fn shutdown() {
    let mut state = GPU3D_STATE.lock();

    if let Some(cid) = state.context_id.take() {
        vmsvga::destroy_3d_context(cid);
    }

    if let Some(sid) = state.vertex_buffer.take() {
        vmsvga::destroy_3d_surface(sid);
    }

    if let Some(sid) = state.index_buffer.take() {
        vmsvga::destroy_3d_surface(sid);
    }

    if let Some(sid) = state.depth_target.take() {
        vmsvga::destroy_3d_surface(sid);
    }

    if let Some(sid) = state.color_target.take() {
        vmsvga::destroy_3d_surface(sid);
    }

    state.ready = false;
}

/// Vertex format for GPU rendering
/// Matches the format expected by the 3D pipeline
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GpuVertex {
    /// Position (x, y, z)
    pub position: [f32; 3],
    /// Color (packed ARGB)
    pub color: u32,
}

impl GpuVertex {
    pub fn new(x: f32, y: f32, z: f32, color: u32) -> Self {
        Self {
            position: [x, y, z],
            color,
        }
    }
}

/// Triangle for GPU rendering
#[repr(C)]
#[derive(Clone, Copy, Debug)]
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
