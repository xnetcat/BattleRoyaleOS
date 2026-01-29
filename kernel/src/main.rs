//! BattleRoyaleOS Kernel
//!
//! A bare-metal unikernel OS for running a 100-player Battle Royale game.
//! The kernel initializes hardware and then dispatches to the appropriate app.

#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

extern crate alloc;

mod api;
mod app;
mod boot;
mod drivers;
mod game;
mod gfx;
mod graphics;
mod memory;
mod net;
mod smp;
mod ui;

use boot::{BASE_REVISION, HHDM_REQUEST, KERNEL_FILE_REQUEST, MEMORY_MAP_REQUEST};
use core::panic::PanicInfo;

/// Read the CPU timestamp counter
#[inline]
pub fn read_tsc() -> u64 {
    unsafe { core::arch::x86_64::_rdtsc() }
}

#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    // Verify Limine protocol
    assert!(BASE_REVISION.is_supported());

    // Initialize serial for debug output
    drivers::serial::SERIAL1.lock().init();
    serial_println!("BattleRoyaleOS Kernel Loaded");

    // Initialize memory allocator
    memory::allocator::init();
    serial_println!("Heap allocator initialized");

    // Get HHDM offset for physical memory access
    if let Some(hhdm) = HHDM_REQUEST.get_response() {
        let hhdm_offset = hhdm.offset();
        *memory::dma::HHDM_OFFSET.lock() = hhdm_offset;
        memory::paging::set_hhdm_offset(hhdm_offset);
        serial_println!("HHDM offset: {:#x}", hhdm_offset);
    }

    // Print memory map info and initialize DMA pool
    if let Some(memmap) = MEMORY_MAP_REQUEST.get_response() {
        let entries = memmap.entries();
        serial_println!("Memory map: {} entries", entries.len());

        let mut usable_memory = 0u64;
        for entry in entries {
            if entry.entry_type == limine::memory_map::EntryType::USABLE {
                usable_memory += entry.length;
            }
        }
        serial_println!("Usable memory: {} MB", usable_memory / 1024 / 1024);

        // Initialize DMA pool from memory map
        let hhdm_offset = *memory::dma::HHDM_OFFSET.lock();
        memory::dma::init_dma_pool(entries, hhdm_offset);
    }

    // Check kernel arguments for boot mode FIRST (before GPU init)
    // This way we can skip GPU initialization in server mode
    let mut is_server = false;
    let mut benchmark_mode = false;
    let mut test_mode = false;
    if let Some(file) = KERNEL_FILE_REQUEST.get_response() {
        let cmdline_bytes = file.file().cmdline();
        if let Ok(cmdline) = core::str::from_utf8(cmdline_bytes) {
            serial_println!("Kernel cmdline: {:?}", cmdline);
            if cmdline.contains("server") {
                is_server = true;
                serial_println!("SERVER MODE: Dedicated server (no rendering)");
            }
            if cmdline.contains("benchmark") {
                benchmark_mode = true;
                serial_println!("BENCHMARK MODE: Performance testing");
            }
            if cmdline.contains("test") {
                test_mode = true;
                serial_println!("TEST MODE: All items spawned");
            }
        }
    }

    // Initialize GPU (skip in server mode - dedicated server has no display)
    let (fb_width, fb_height, gpu_batch_available) = if is_server {
        serial_println!("SERVER MODE: Skipping GPU initialization");
        (0, 0, false)
    } else {
        // Normal GPU initialization (tries VMSVGA first, falls back to software framebuffer)
        let (w, h) = graphics::gpu::init();
        serial_println!("GPU: {} {}x{}", graphics::gpu::backend_name(), w, h);
        if w == 0 || h == 0 {
            serial_println!("ERROR: No framebuffer available");
            halt_loop();
        }

        // Initialize GPU rendering integration
        graphics::gpu_render::init();

        // Initialize GPU batch renderer
        let gpu_batch_ok = graphics::gpu_batch::init(w as u32, h as u32);

        // Initialize z-buffer
        graphics::zbuffer::init(w, h);
        serial_println!("Z-buffer initialized");

        // Initialize tile system
        graphics::tiles::init(w, h);
        if let Some(queue) = graphics::tiles::TILE_QUEUE.lock().as_ref() {
            serial_println!("Tile system: {} tiles", queue.tile_count());
            graphics::tiles::init_bins(queue.tile_count());
        }

        // Initialize vsync subsystem
        graphics::vsync::init();

        (w, h, gpu_batch_ok)
    };

    // Print CPU count
    let cpu_count = smp::scheduler::cpu_count();
    serial_println!("CPU count: {}", cpu_count);

    // Initialize PCI and find E1000
    serial_println!("Scanning PCI bus...");
    if let Some(e1000_dev) = drivers::pci::find_device(
        drivers::pci::INTEL_VENDOR_ID,
        drivers::pci::E1000_DEVICE_ID,
    ) {
        serial_println!(
            "Found E1000 at {:02x}:{:02x}.{} BAR0={:#x}",
            e1000_dev.bus,
            e1000_dev.slot,
            e1000_dev.function,
            e1000_dev.bar0
        );

        // Enable PCI bus mastering and memory space access
        e1000_dev.enable_bus_master();
        e1000_dev.enable_memory_space();

        // Get BAR0 physical address
        let bar0_phys = e1000_dev.bar0_address();

        // Map MMIO region into kernel address space with proper caching attributes
        // E1000 MMIO region is 128KB
        let mmio_base = match memory::paging::map_mmio(bar0_phys, 0x20000) {
            Some(virt) => virt,
            None => {
                serial_println!("E1000: Failed to map MMIO region");
                halt_loop();
            }
        };

        // Initialize E1000 driver
        if let Err(e) = drivers::e1000::init(mmio_base) {
            serial_println!("E1000 init failed: {}", e);
        } else {
            serial_println!("E1000 initialized successfully");
            // Initialize network stack
            net::stack::init();
        }
    } else {
        serial_println!("E1000 not found");
    }

    // Initialize game world (uses is_server flag from earlier cmdline parsing)
    serial_println!("Initializing game world...");
    game::world::init(is_server);
    serial_println!("Game world initialized (Server: {})", is_server);

    // Initialize SMP - start worker cores
    serial_println!("Initializing SMP...");
    smp::scheduler::init();
    serial_println!("SMP initialized");

    // Initialize mouse
    serial_println!("Initializing mouse...");
    game::input::init_mouse();
    serial_println!("Mouse initialized");

    serial_println!("Starting main loop...");

    // Branch based on server mode
    if is_server {
        // Dedicated server loop (no rendering)
        server_loop();
    } else {
        // Set mode flags for game client
        app::set_benchmark_mode(benchmark_mode);
        app::set_test_mode(test_mode);

        // Run game client
        app::run(fb_width, fb_height, gpu_batch_available);
    }
}

/// Dedicated server loop (no rendering)
/// Processes network traffic, updates game state, broadcasts to clients
fn server_loop() -> ! {
    serial_println!("=== DEDICATED SERVER STARTED ===");
    serial_println!("Server is running headless (no rendering)");
    serial_println!("Waiting for client connections...");

    let mut tick_count = 0u64;
    let tsc_per_second: u64 = 2_000_000_000;
    let start_tsc = read_tsc();
    let mut last_status_tsc = start_tsc;

    // Server tick rate: 60 ticks per second (same as client frame rate)
    let tsc_per_tick = tsc_per_second / 60;
    let mut next_tick_tsc = start_tsc + tsc_per_tick;

    // Initialize the game world in server mode
    if let Some(world) = game::world::GAME_WORLD.lock().as_mut() {
        world.spawn_bots(10); // Spawn 10 bots for the battle
        serial_println!("Spawned 10 bots for battle");
    }

    loop {
        let current_tsc = read_tsc();

        // Tick at fixed rate
        if current_tsc >= next_tick_tsc {
            tick_count += 1;
            next_tick_tsc = current_tsc + tsc_per_tick;

            // Process incoming network packets
            net::protocol::process_incoming();

            // Update game world physics
            if let Some(world) = game::world::GAME_WORLD.lock().as_mut() {
                world.update(1.0 / 60.0);
            }

            // Broadcast world state to clients every 6 ticks (~10 Hz)
            if tick_count % 6 == 0 {
                net::protocol::broadcast_world_state();
            }

            // Poll network stack
            net::stack::poll(tick_count as i64);

            // Print status every 10 seconds
            if current_tsc - last_status_tsc >= tsc_per_second * 10 {
                last_status_tsc = current_tsc;
                let elapsed_secs = (current_tsc - start_tsc) / tsc_per_second;

                // Get player count
                let player_count = if let Some(world) = game::world::GAME_WORLD.lock().as_ref() {
                    world.players.len()
                } else {
                    0
                };

                serial_println!("[SERVER] Uptime: {}s | Ticks: {} | Players: {}",
                    elapsed_secs, tick_count, player_count);
            }
        } else {
            // Idle CPU while waiting for next tick (saves power)
            unsafe { core::arch::asm!("hlt"); }
        }
    }
}

/// Panic handler
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("KERNEL PANIC: {}", info);
    halt_loop();
}

/// Halt the CPU
fn halt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
