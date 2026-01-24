//! Limine bootloader requests and responses

use limine::request::{
    FramebufferRequest, HhdmRequest, KernelFileRequest, MemoryMapRequest, MpRequest,
    RequestsEndMarker, RequestsStartMarker,
};

#[used]
#[unsafe(link_section = ".requests_start_marker")]
static _START_MARKER: RequestsStartMarker = RequestsStartMarker::new();

#[used]
#[unsafe(link_section = ".requests_end_marker")]
static _END_MARKER: RequestsEndMarker = RequestsEndMarker::new();

/// Base revision request - ensures compatibility with Limine protocol
#[used]
#[unsafe(link_section = ".requests")]
pub static BASE_REVISION: limine::BaseRevision = limine::BaseRevision::new();

/// Framebuffer request for graphics output
#[used]
#[unsafe(link_section = ".requests")]
pub static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

/// Memory map request for physical memory information
#[used]
#[unsafe(link_section = ".requests")]
pub static MEMORY_MAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new();

/// Higher Half Direct Map request for physical memory access
#[used]
#[unsafe(link_section = ".requests")]
pub static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();

/// SMP request for multicore support
#[used]
#[unsafe(link_section = ".requests")]
pub static SMP_REQUEST: MpRequest = MpRequest::new();

/// Kernel file request to get command line arguments
#[used]
#[unsafe(link_section = ".requests")]
pub static KERNEL_FILE_REQUEST: KernelFileRequest = KernelFileRequest::new();
