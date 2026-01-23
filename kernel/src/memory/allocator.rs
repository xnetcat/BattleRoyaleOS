//! Global heap allocator using Talc

use core::alloc::Layout;
use core::ptr::NonNull;
use spin::Mutex;
use talc::{ClaimOnOom, Span, Talc, Talck};

/// Heap size: 64 MB
const HEAP_SIZE: usize = 64 * 1024 * 1024;

/// Static heap memory
static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

/// Global allocator
#[global_allocator]
static ALLOCATOR: Talck<Mutex<()>, ClaimOnOom> = Talc::new(unsafe {
    ClaimOnOom::new(Span::from_array(core::ptr::addr_of!(HEAP) as *mut [u8; HEAP_SIZE]))
})
.lock();

/// Initialize the heap allocator
pub fn init() {
    // Talc with ClaimOnOom initializes itself on first allocation
    // Nothing to do here, but we keep this function for consistency
}

/// Allocate memory with a specific alignment
pub fn alloc_aligned(size: usize, align: usize) -> Option<NonNull<u8>> {
    let layout = Layout::from_size_align(size, align).ok()?;
    let ptr = unsafe { alloc::alloc::alloc(layout) };
    NonNull::new(ptr)
}
