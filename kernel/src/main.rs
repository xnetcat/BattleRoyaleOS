//! BattleRoyaleOS Kernel
//!
//! A bare-metal unikernel OS for running a 100-player Battle Royale game.

#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

extern crate alloc;

mod boot;
mod drivers;
mod game;
mod graphics;
mod memory;
mod net;
mod smp;

use boot::{BASE_REVISION, FRAMEBUFFER_REQUEST, HHDM_REQUEST, MEMORY_MAP_REQUEST, SMP_REQUEST};
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicU64, Ordering};
use glam::{Mat4, Vec3};
use graphics::{
    font,
    framebuffer::rgb,
    pipeline::{look_at, perspective, transform_triangle},
    rasterizer::{self, rasterize_triangle_shaded, rasterize_triangle_with_context},
    tiles,
    zbuffer,
};
use renderer::mesh;

/// Simple timestamp counter for timing using TSC
static TICKS: AtomicU64 = AtomicU64::new(0);

/// Read the CPU timestamp counter
#[inline]
fn read_tsc() -> u64 {
    unsafe {
        core::arch::x86_64::_rdtsc()
    }
}

/// Get current tick count (approximate milliseconds)
fn get_ticks() -> u64 {
    TICKS.load(Ordering::Relaxed)
}

/// Increment tick counter
fn tick() {
    TICKS.fetch_add(1, Ordering::Relaxed);
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

    // Initialize framebuffer
    let (fb_width, fb_height) = if let Some((w, h)) = graphics::framebuffer::init() {
        serial_println!("Framebuffer: {}x{}", w, h);
        (w, h)
    } else {
        serial_println!("ERROR: No framebuffer available");
        halt_loop();
    };

    // Initialize z-buffer
    zbuffer::init(fb_width, fb_height);
    serial_println!("Z-buffer initialized");

    // Initialize tile system
    tiles::init(fb_width, fb_height);
    if let Some(queue) = tiles::TILE_QUEUE.lock().as_ref() {
        serial_println!("Tile system: {} tiles", queue.tile_count());
        tiles::init_bins(queue.tile_count());
    }

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

    // Initialize game world
    serial_println!("Initializing game world...");
    game::world::init(true); // Server mode
    serial_println!("Game world initialized");

    // Initialize SMP - start worker cores
    serial_println!("Initializing SMP...");
    smp::scheduler::init();
    serial_println!("SMP initialized");

    serial_println!("Starting main loop...");

    // Main game loop
    main_loop(fb_width, fb_height);
}

/// Main game loop (runs on Core 0)
fn main_loop(fb_width: usize, fb_height: usize) -> ! {
    let mut frame_count = 0u32;
    let mut rotation = 0.0f32;

    // FPS tracking
    let mut last_fps_time = read_tsc();
    let mut fps_frame_count = 0u32;
    let mut current_fps = 0u32;
    // Estimate TSC frequency (assume ~2GHz for QEMU)
    let tsc_per_second: u64 = 2_000_000_000;

    serial_println!("Creating test meshes...");

    // Create a test cube
    let cube = mesh::create_cube(Vec3::new(0.8, 0.2, 0.2));
    serial_println!("Cube created: {} triangles", cube.triangle_count());

    // Create a small ground for testing (reduced from 100 to 10)
    let ground = mesh::create_ground_mesh(10.0, Vec3::new(0.2, 0.5, 0.2));
    serial_println!("Ground created: {} triangles", ground.triangle_count());

    // Camera setup
    let aspect = fb_width as f32 / fb_height as f32;
    // 60 degrees in radians = 60 * PI / 180 = PI/3 ≈ 1.0472
    let fov_radians = core::f32::consts::PI / 3.0;
    // Use reasonable near/far for better depth precision
    let projection = perspective(fov_radians, aspect, 1.0, 100.0);

    serial_println!("Entering main loop...");

    loop {
        // Poll keyboard
        game::input::poll_keyboard();

        // Check for escape to quit
        if game::input::escape_pressed() {
            serial_println!("Escape pressed, shutting down...");
            break;
        }

        // Update game world (every 5 frames ~= 2Hz at 10 FPS)
        if frame_count % 5 == 0 {
            if let Some(world) = game::world::GAME_WORLD.lock().as_mut() {
                world.update(0.5);
            }

            // Process network packets
            net::protocol::process_incoming();
            net::protocol::broadcast_world_state();
        }

        // Poll network stack every frame
        net::stack::poll(frame_count as i64);

        // Acquire render context for this frame (optimized - no locks during rendering)
        let render_ctx = match rasterizer::RenderContext::acquire() {
            Some(ctx) => ctx,
            None => continue,
        };

        // Clear framebuffer and z-buffer using render context
        render_ctx.clear(rgb(30, 30, 50)); // Dark blue-gray background
        render_ctx.clear_zbuffer();

        // Update rotation for spinning cube
        rotation += 0.02;

        // Camera position (orbit around origin)
        let camera_dist = 5.0;
        let camera_pos = Vec3::new(
            libm::sinf(rotation * 0.3) * camera_dist,
            2.0,
            libm::cosf(rotation * 0.3) * camera_dist,
        );
        let view = look_at(camera_pos, Vec3::new(0.0, 0.0, 0.0), Vec3::Y);

        // Render ground
        let ground_model = Mat4::from_translation(Vec3::new(0.0, -1.0, 0.0)); // Below the cube
        render_mesh_with_ctx(&render_ctx, &ground, &ground_model, &view, &projection, fb_width, fb_height);

        // Render spinning cube
        let cube_model = Mat4::from_rotation_y(rotation) * Mat4::from_rotation_x(rotation * 0.7);
        render_mesh_with_ctx(&render_ctx, &cube, &cube_model, &view, &projection, fb_width, fb_height);

        // Drop render context before drawing FPS (font uses its own lock)
        drop(render_ctx);

        // Draw FPS counter
        font::draw_fps(current_fps, fb_width);

        // Update FPS counter
        fps_frame_count += 1;
        let now = read_tsc();
        let elapsed = now.wrapping_sub(last_fps_time);
        if elapsed >= tsc_per_second {
            current_fps = fps_frame_count;
            fps_frame_count = 0;
            last_fps_time = now;
        }

        // Print FPS to serial every 100 frames
        frame_count = frame_count.wrapping_add(1);
        if frame_count % 100 == 0 {
            serial_println!("Frame {} FPS: {}", frame_count, current_fps);
        }
    }

    halt_loop();
}

/// Render a mesh using a pre-acquired render context (optimized)
fn render_mesh_with_ctx(
    ctx: &rasterizer::RenderContext,
    mesh: &mesh::Mesh,
    model: &Mat4,
    view: &Mat4,
    projection: &Mat4,
    fb_width: usize,
    fb_height: usize,
) {
    for i in 0..mesh.triangle_count() {
        if let Some((v0, v1, v2)) = mesh.get_triangle(i) {
            if let Some((tv0, tv1, tv2)) = transform_triangle(
                v0,
                v1,
                v2,
                model,
                view,
                projection,
                fb_width as f32,
                fb_height as f32,
            ) {
                rasterize_triangle_with_context(ctx, &tv0, &tv1, &tv2);
            }
        }
    }
}

/// Render all players
fn render_players(view: &Mat4, projection: &Mat4, fb_width: usize, fb_height: usize) {
    let world_guard = game::world::GAME_WORLD.lock();
    if let Some(world) = world_guard.as_ref() {
        let player_mesh = mesh::create_player_mesh(
            Vec3::new(0.3, 0.3, 0.8), // Blue body
            Vec3::new(0.9, 0.7, 0.6), // Skin head
        );

        for player in &world.players {
            if !player.is_alive() || player.in_bus {
                continue;
            }

            let model = Mat4::from_translation(player.position)
                * Mat4::from_rotation_y(player.yaw);

            for i in 0..player_mesh.triangle_count() {
                if let Some((v0, v1, v2)) = player_mesh.get_triangle(i) {
                    if let Some((tv0, tv1, tv2)) = transform_triangle(
                        v0,
                        v1,
                        v2,
                        &model,
                        view,
                        projection,
                        fb_width as f32,
                        fb_height as f32,
                    ) {
                        rasterize_triangle_shaded(&tv0, &tv1, &tv2);
                    }
                }
            }
        }
    }
}

/// Render worker for rasterizer cores
pub fn render_worker(_rasterizer_id: u8) {
    // In a full implementation, this would:
    // 1. Get tiles from TILE_QUEUE
    // 2. Rasterize triangles for those tiles
    // 3. Write to framebuffer

    // For now, the main loop does all rendering single-threaded
}

/// Network worker for network core
pub fn network_worker() {
    // Poll network stack
    let ticks = get_ticks() as i64;
    net::stack::poll(ticks);

    // Process incoming packets
    net::protocol::process_incoming();
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
