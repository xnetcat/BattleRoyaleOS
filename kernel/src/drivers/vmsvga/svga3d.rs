//! SVGA3D GPU acceleration support
//!
//! This module implements the VMware SVGA3D protocol for hardware-accelerated
//! 3D rendering. It provides context management, surface allocation, and
//! primitive drawing capabilities.

use alloc::vec::Vec;
use spin::Mutex;

/// SVGA3D Command IDs
pub mod cmd {
    // Context management
    pub const CONTEXT_DEFINE: u32 = 1045;
    pub const CONTEXT_DESTROY: u32 = 1046;

    // Surface management
    pub const SURFACE_DEFINE: u32 = 1040;
    pub const SURFACE_DESTROY: u32 = 1041;
    pub const SURFACE_COPY: u32 = 1042;
    pub const SURFACE_STRETCHBLT: u32 = 1043;
    pub const SURFACE_DMA: u32 = 1044;

    // Rendering state
    pub const SETTRANSFORM: u32 = 1047;
    pub const SETZRANGE: u32 = 1048;
    pub const SETRENDERSTATE: u32 = 1049;
    pub const SETRENDERTARGET: u32 = 1050;
    pub const SETTEXTURESTATE: u32 = 1051;
    pub const SETMATERIAL: u32 = 1052;
    pub const SETLIGHTDATA: u32 = 1053;
    pub const SETLIGHTENABLED: u32 = 1054;
    pub const SETVIEWPORT: u32 = 1055;
    pub const SETCLIPPLANE: u32 = 1056;

    // Drawing
    pub const CLEAR: u32 = 1057;
    pub const PRESENT: u32 = 1058;
    pub const DRAW_PRIMITIVES: u32 = 1063;

    // Shaders
    pub const SHADER_DEFINE: u32 = 1059;
    pub const SHADER_DESTROY: u32 = 1060;
    pub const SET_SHADER: u32 = 1061;
    pub const SET_SHADER_CONST: u32 = 1062;

    // Synchronization
    pub const FENCE: u32 = 1030;

    // Blitting
    pub const BLIT_SURFACE_TO_SCREEN: u32 = 1069;
}

/// SVGA3D surface formats
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SurfaceFormat {
    Invalid = 0,
    X8R8G8B8 = 1,
    A8R8G8B8 = 2,
    R5G6B5 = 3,
    X1R5G5B5 = 4,
    A1R5G5B5 = 5,
    A4R4G4B4 = 6,
    ZD32 = 7,
    ZD16 = 8,
    ZD24S8 = 9,
    ZD15S1 = 10,
    Luminance8 = 11,
    Luminance4Alpha4 = 12,
    Luminance16 = 13,
    Luminance8Alpha8 = 14,
    DXT1 = 15,
    DXT2 = 16,
    DXT3 = 17,
    DXT4 = 18,
    DXT5 = 19,
    BumpU8V8 = 20,
    BumpL6V5U5 = 21,
    BumpX8L8V8U8 = 22,
    Argb_S10E5 = 23,
    Argb_S23E8 = 24,
    Buffer = 37, // For vertex/index buffers
    ZD24X8 = 38,
    V16U16 = 39,
    G16R16 = 40,
    A16B16G16R16 = 41,
}

/// SVGA3D primitive types
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PrimitiveType {
    Invalid = 0,
    TriangleList = 1,
    PointList = 2,
    LineList = 3,
    LineStrip = 4,
    TriangleStrip = 5,
    TriangleFan = 6,
}

/// SVGA3D render state IDs
#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum RenderStateId {
    ZEnable = 1,
    ZWriteEnable = 2,
    AlphaTestEnable = 3,
    DitherEnable = 4,
    BlendEnable = 5,
    FogEnable = 6,
    SpecularEnable = 7,
    StencilEnable = 8,
    FogColor = 9,
    FogMode = 10,
    FillMode = 11,
    ShadeMode = 12,
    LinePattern = 13,
    SrcBlend = 14,
    DstBlend = 15,
    AlphaRef = 16,
    AlphaFunc = 17,
    ZFunc = 18,
    CullMode = 19,
    ZBias = 20,
    ColorWriteEnable = 21,
    OutputGamma = 22,
    BlendFactor = 23,
    NormalizeNormals = 24,
    PointSpriteEnable = 25,
    PointSize = 26,
    WrapU0 = 27,
}

/// SVGA3D cull modes
#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum CullMode {
    None = 0,
    Cw = 1,   // Clockwise
    Ccw = 2,  // Counter-clockwise
}

/// SVGA3D fill modes
#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum FillMode {
    Point = 1,
    Wireframe = 2,
    Solid = 3,
}

/// SVGA3D transform types
#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum TransformType {
    World = 0,
    View = 1,
    Projection = 2,
    Texture0 = 3,
    Texture1 = 4,
    Texture2 = 5,
    Texture3 = 6,
    Texture4 = 7,
    Texture5 = 8,
    Texture6 = 9,
    Texture7 = 10,
    World1 = 11,
    World2 = 12,
    World3 = 13,
}

/// SVGA3D render target types
#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum RenderTargetType {
    Color = 0,
    Depth = 1,
    Stencil = 2,
}

/// SVGA3D clear flags
pub mod clear_flags {
    pub const COLOR: u32 = 0x1;
    pub const DEPTH: u32 = 0x2;
    pub const STENCIL: u32 = 0x4;
}

/// SVGA3D vertex declaration usage types
#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum DeclUsage {
    Position = 0,
    BlendWeight = 1,
    BlendIndices = 2,
    Normal = 3,
    PointSize = 4,
    TexCoord = 5,
    Tangent = 6,
    Binormal = 7,
    TessFactor = 8,
    Color = 9,
}

/// SVGA3D vertex declaration types
#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum DeclType {
    Float1 = 0,
    Float2 = 1,
    Float3 = 2,
    Float4 = 3,
    D3DColor = 4,
    UByte4 = 5,
    Short2 = 6,
    Short4 = 7,
    UByte4N = 8,
    Short2N = 9,
    Short4N = 10,
    UShort2N = 11,
    UShort4N = 12,
    UDec3 = 13,
    Dec3N = 14,
    Float16_2 = 15,
    Float16_4 = 16,
}

/// SVGA3D surface flags
pub mod surface_flags {
    pub const CUBEMAP: u32 = 1 << 0;
    pub const HINT_STATIC: u32 = 1 << 1;
    pub const HINT_DYNAMIC: u32 = 1 << 2;
    pub const HINT_INDEXBUFFER: u32 = 1 << 3;
    pub const HINT_VERTEXBUFFER: u32 = 1 << 4;
    pub const HINT_TEXTURE: u32 = 1 << 5;
    pub const HINT_RENDERTARGET: u32 = 1 << 6;
    pub const HINT_DEPTHSTENCIL: u32 = 1 << 7;
    pub const HINT_WRITEONLY: u32 = 1 << 8;
    pub const AUTOGENMIPMAPS: u32 = 1 << 9;
}

/// SVGA3D shader types
#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum ShaderType {
    Vertex = 0,
    Pixel = 1,
}

/// 4x4 matrix for transforms
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Matrix4x4 {
    pub m: [[f32; 4]; 4],
}

impl Matrix4x4 {
    pub const fn identity() -> Self {
        Self {
            m: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Create perspective projection matrix
    pub fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Self {
        let f = 1.0 / libm::tanf(fov_y / 2.0);
        let nf = 1.0 / (near - far);

        Self {
            m: [
                [f / aspect, 0.0, 0.0, 0.0],
                [0.0, f, 0.0, 0.0],
                [0.0, 0.0, (far + near) * nf, -1.0],
                [0.0, 0.0, 2.0 * far * near * nf, 0.0],
            ],
        }
    }

    /// Create look-at view matrix
    pub fn look_at(eye: [f32; 3], target: [f32; 3], up: [f32; 3]) -> Self {
        let zaxis = normalize([
            eye[0] - target[0],
            eye[1] - target[1],
            eye[2] - target[2],
        ]);
        let xaxis = normalize(cross(up, zaxis));
        let yaxis = cross(zaxis, xaxis);

        Self {
            m: [
                [xaxis[0], yaxis[0], zaxis[0], 0.0],
                [xaxis[1], yaxis[1], zaxis[1], 0.0],
                [xaxis[2], yaxis[2], zaxis[2], 0.0],
                [-dot(xaxis, eye), -dot(yaxis, eye), -dot(zaxis, eye), 1.0],
            ],
        }
    }

    /// Create translation matrix
    pub fn translation(x: f32, y: f32, z: f32) -> Self {
        Self {
            m: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [x, y, z, 1.0],
            ],
        }
    }

    /// Create rotation around Y axis
    pub fn rotation_y(angle: f32) -> Self {
        let c = libm::cosf(angle);
        let s = libm::sinf(angle);
        Self {
            m: [
                [c, 0.0, -s, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [s, 0.0, c, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Multiply two matrices
    pub fn mul(&self, other: &Self) -> Self {
        let mut result = Self { m: [[0.0; 4]; 4] };
        for i in 0..4 {
            for j in 0..4 {
                for k in 0..4 {
                    result.m[i][j] += self.m[i][k] * other.m[k][j];
                }
            }
        }
        result
    }
}

// Helper math functions
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = libm::sqrtf(v[0] * v[0] + v[1] * v[1] + v[2] * v[2]);
    if len > 0.0001 {
        [v[0] / len, v[1] / len, v[2] / len]
    } else {
        [0.0, 0.0, 0.0]
    }
}

/// Surface descriptor
#[derive(Clone)]
pub struct Surface {
    pub id: u32,
    pub format: SurfaceFormat,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub flags: u32,
    pub num_mip_levels: u32,
}

/// Vertex buffer descriptor
#[derive(Clone)]
pub struct VertexBuffer {
    pub surface_id: u32,
    pub stride: u32,
    pub offset: u32,
}

/// Index buffer descriptor
#[derive(Clone)]
pub struct IndexBuffer {
    pub surface_id: u32,
    pub format: IndexFormat,
    pub offset: u32,
}

/// Index format
#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum IndexFormat {
    Index16 = 0,
    Index32 = 1,
}

/// Vertex declaration entry
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct VertexElement {
    pub stream: u32,
    pub offset: u32,
    pub decl_type: DeclType,
    pub method: u32,
    pub usage: DeclUsage,
    pub usage_index: u32,
}

/// Primitive range for drawing
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PrimitiveRange {
    pub prim_type: PrimitiveType,
    pub prim_count: u32,
    pub index_array: IndexArray,
}

/// Index array descriptor
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct IndexArray {
    pub surface_id: u32,
    pub offset: u32,
    pub stride: u32,
}

/// SVGA3D context state
pub struct Svga3dContext {
    /// Context ID
    pub id: u32,
    /// Current render target surface ID
    pub render_target: Option<u32>,
    /// Current depth buffer surface ID
    pub depth_buffer: Option<u32>,
    /// Current viewport
    pub viewport: Viewport,
    /// Z-buffer range
    pub z_range: (f32, f32),
    /// Is the context valid/defined
    pub defined: bool,
}

/// Viewport definition
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub min_depth: f32,
    pub max_depth: f32,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 1024.0,
            height: 768.0,
            min_depth: 0.0,
            max_depth: 1.0,
        }
    }
}

impl Svga3dContext {
    pub const fn new(id: u32) -> Self {
        Self {
            id,
            render_target: None,
            depth_buffer: None,
            viewport: Viewport {
                x: 0.0,
                y: 0.0,
                width: 1024.0,
                height: 768.0,
                min_depth: 0.0,
                max_depth: 1.0,
            },
            z_range: (0.0, 1.0),
            defined: false,
        }
    }
}

/// SVGA3D device state
pub struct Svga3dDevice {
    /// Whether 3D is available
    pub available: bool,
    /// 3D hardware version
    pub hw_version: u32,
    /// Next surface ID to allocate
    pub next_surface_id: u32,
    /// Next context ID to allocate
    pub next_context_id: u32,
    /// Allocated surfaces
    pub surfaces: Vec<Surface>,
    /// Active context
    pub context: Option<Svga3dContext>,
}

impl Svga3dDevice {
    pub const fn new() -> Self {
        Self {
            available: false,
            hw_version: 0,
            next_surface_id: 1,
            next_context_id: 1,
            surfaces: Vec::new(),
            context: None,
        }
    }

    /// Allocate a new surface ID
    pub fn alloc_surface_id(&mut self) -> u32 {
        let id = self.next_surface_id;
        self.next_surface_id += 1;
        id
    }

    /// Allocate a new context ID
    pub fn alloc_context_id(&mut self) -> u32 {
        let id = self.next_context_id;
        self.next_context_id += 1;
        id
    }
}

/// Global SVGA3D device state
pub static SVGA3D_DEVICE: Mutex<Svga3dDevice> = Mutex::new(Svga3dDevice::new());

/// SVGA3D hardware version constants
pub mod hw_version {
    pub const WS5_RC1: u32 = 0x00000001;
    pub const WS5_RC2: u32 = 0x00000002;
    pub const WS6_B1: u32 = 0x00010001;
    pub const WS65_B1: u32 = 0x00020000; // Recommended minimum
    pub const WS8_B1: u32 = 0x00020001;
    pub const CURRENT: u32 = WS8_B1;
}

/// FIFO 3D register offsets
pub mod fifo_3d_reg {
    pub const GUEST_3D_HWVERSION: usize = 7;
    pub const HWVERSION: usize = 6;
    pub const HWVERSION_REVISED: usize = 7;
}
