//! Core assignment and scheduling

use crate::boot::SMP_REQUEST;
use crate::serial_println;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use limine::mp::Cpu;
use spin::Mutex;

/// Core role assignments
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CoreRole {
    /// Main game logic, input, frame orchestration (Core 0)
    GameLogic,
    /// Triangle rasterization (Cores 1-3)
    Rasterizer(u8), // Which rasterizer (0, 1, 2)
    /// Network polling and packet processing (Core 4)
    Network,
}

/// Per-core data
pub struct CoreData {
    pub id: u32,
    pub role: CoreRole,
    pub running: AtomicBool,
}

impl CoreData {
    pub const fn new(id: u32, role: CoreRole) -> Self {
        Self {
            id,
            role,
            running: AtomicBool::new(false),
        }
    }
}

/// Global core data array (max 8 cores)
static CORE_DATA: [Mutex<CoreData>; 8] = [
    Mutex::new(CoreData::new(0, CoreRole::GameLogic)),
    Mutex::new(CoreData::new(1, CoreRole::Rasterizer(0))),
    Mutex::new(CoreData::new(2, CoreRole::Rasterizer(1))),
    Mutex::new(CoreData::new(3, CoreRole::Rasterizer(2))),
    Mutex::new(CoreData::new(4, CoreRole::Network)),
    Mutex::new(CoreData::new(5, CoreRole::GameLogic)), // Unused
    Mutex::new(CoreData::new(6, CoreRole::GameLogic)), // Unused
    Mutex::new(CoreData::new(7, CoreRole::GameLogic)), // Unused
];

/// Number of active cores
static ACTIVE_CORES: AtomicU32 = AtomicU32::new(1);

/// Frame counter for synchronization
pub static FRAME_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Flag to signal render cores to start
pub static RENDER_START: AtomicBool = AtomicBool::new(false);

/// Flag to signal all cores to stop
pub static SHUTDOWN: AtomicBool = AtomicBool::new(false);

/// Get the number of available CPU cores
pub fn cpu_count() -> u32 {
    if let Some(response) = SMP_REQUEST.get_response() {
        response.cpus().len() as u32
    } else {
        1
    }
}

/// Initialize SMP and start worker cores
pub fn init() {
    let response = match SMP_REQUEST.get_response() {
        Some(r) => r,
        None => {
            serial_println!("SMP: No SMP response, running single-core");
            return;
        }
    };

    let cpus = response.cpus();
    let cpu_count = cpus.len();

    serial_println!("SMP: {} CPUs available", cpu_count);
    ACTIVE_CORES.store(cpu_count as u32, Ordering::Release);

    // Start worker cores (skip BSP which is core 0)
    for (i, cpu) in cpus.iter().enumerate() {
        if i == 0 {
            // BSP runs game logic
            CORE_DATA[0].lock().running.store(true, Ordering::Release);
            continue;
        }

        let core_id = i as u32;
        serial_println!("SMP: Starting core {}", core_id);

        // Set up the core's entry point based on role
        match core_id {
            1..=3 => {
                // Rasterizer cores
                cpu.goto_address
                    .write(rasterizer_entry);
            }
            4 => {
                // Network core
                cpu.goto_address
                    .write(network_entry);
            }
            _ => {
                // Additional cores (idle)
                cpu.goto_address.write(idle_entry);
            }
        }
    }

    serial_println!("SMP: All cores started");
}

/// Entry point for rasterizer cores
unsafe extern "C" fn rasterizer_entry(cpu: &Cpu) -> ! {
    let core_id = cpu.id;
    let rasterizer_id = (core_id - 1) as u8;

    serial_println!("Rasterizer {} started on core {}", rasterizer_id, core_id);

    if let Some(data) = CORE_DATA.get(core_id as usize) {
        data.lock().running.store(true, Ordering::Release);
    }

    loop {
        // Wait for render signal
        while !RENDER_START.load(Ordering::Acquire) {
            if SHUTDOWN.load(Ordering::Acquire) {
                halt_loop();
            }
            core::hint::spin_loop();
        }

        // Do rendering work
        crate::app::render_worker(rasterizer_id);

        // Signal completion via barrier
        crate::smp::sync::RENDER_BARRIER.wait();

        // Wait for next frame
        while RENDER_START.load(Ordering::Acquire) {
            core::hint::spin_loop();
        }
    }
}

/// Entry point for network core
unsafe extern "C" fn network_entry(cpu: &Cpu) -> ! {
    let core_id = cpu.id;
    serial_println!("Network core started on core {}", core_id);

    if let Some(data) = CORE_DATA.get(core_id as usize) {
        data.lock().running.store(true, Ordering::Release);
    }

    loop {
        if SHUTDOWN.load(Ordering::Acquire) {
            halt_loop();
        }

        // Poll network
        crate::app::network_worker();

        // Small delay to prevent busy-waiting
        for _ in 0..1000 {
            core::hint::spin_loop();
        }
    }
}

/// Entry point for idle cores
unsafe extern "C" fn idle_entry(cpu: &Cpu) -> ! {
    let core_id = cpu.id;
    serial_println!("Idle core {} started", core_id);

    halt_loop();
}

/// Halt loop for unused cores
fn halt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

/// Signal render cores to start
pub fn start_render() {
    RENDER_START.store(true, Ordering::Release);
}

/// Signal render cores that frame is complete
pub fn end_render() {
    RENDER_START.store(false, Ordering::Release);
}

/// Increment frame counter
pub fn next_frame() {
    FRAME_COUNTER.fetch_add(1, Ordering::Release);
}

/// Get current frame number
pub fn current_frame() -> u32 {
    FRAME_COUNTER.load(Ordering::Acquire)
}

/// Signal all cores to shutdown
pub fn shutdown() {
    SHUTDOWN.store(true, Ordering::Release);
}

/// Check if shutdown was requested
pub fn should_shutdown() -> bool {
    SHUTDOWN.load(Ordering::Acquire)
}
