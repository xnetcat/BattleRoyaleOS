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

use boot::{BASE_REVISION, HHDM_REQUEST, MEMORY_MAP_REQUEST};
use core::panic::PanicInfo;
use core::sync::atomic::Ordering;
use glam::{Mat4, Vec3};
use graphics::{
    font,
    framebuffer::rgb,
    pipeline::{look_at, perspective, transform_and_bin},
    rasterizer::{self, rasterize_screen_triangle_in_tile},
    tiles::{self, TILE_BINS_LOCKFREE, TILE_QUEUE},
    zbuffer,
};
use renderer::mesh;

/// Read the CPU timestamp counter
#[inline]
fn read_tsc() -> u64 {
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

    // Frame limiter: target 60 FPS to prevent flickering
    let target_fps: u64 = 60;
    let tsc_per_frame = tsc_per_second / target_fps;
    let mut last_frame_time = read_tsc();

    // Create scene meshes
    let cube = mesh::create_cube(Vec3::new(0.8, 0.2, 0.2));
    let terrain = mesh::create_terrain_grid(30.0, 30, Vec3::new(0.2, 0.6, 0.3));
    let player = mesh::create_player_mesh(Vec3::new(0.3, 0.3, 0.8), Vec3::new(0.9, 0.7, 0.6));

    serial_println!("Scene: {} terrain + {} cube + {} player = {} triangles",
        terrain.triangle_count(), cube.triangle_count(), player.triangle_count(),
        terrain.triangle_count() + cube.triangle_count() + player.triangle_count());

    // Camera setup
    let aspect = fb_width as f32 / fb_height as f32;
    let fov_radians = core::f32::consts::PI / 3.0;
    let projection = perspective(fov_radians, aspect, 0.1, 100.0);

    serial_println!("Parallel rendering: 4 cores active");

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

        // Acquire render context for this frame
        let render_ctx = match rasterizer::RenderContext::acquire() {
            Some(ctx) => ctx,
            None => continue,
        };

        // Clear back buffer and z-buffer (double buffering prevents flicker)
        render_ctx.clear(rgb(30, 30, 50));
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

        // === PARALLEL RENDERING (4 cores) ===

        // 1. Clear lock-free bins and reset triangle buffer
        tiles::clear_lockfree_bins();
        tiles::reset_triangle_buffer();

        // 2. Transform and bin all triangles (Core 0 does this sequentially)
        let terrain_model = Mat4::from_translation(Vec3::new(0.0, -2.0, 0.0));
        bin_mesh(&terrain, &terrain_model, &view, &projection, fb_width as f32, fb_height as f32);

        let cube_model = Mat4::from_rotation_y(rotation) * Mat4::from_rotation_x(rotation * 0.7);
        bin_mesh(&cube, &cube_model, &view, &projection, fb_width as f32, fb_height as f32);

        // Add player at a fixed position
        let player_model = Mat4::from_translation(Vec3::new(2.0, -1.5, 0.0));
        bin_mesh(&player, &player_model, &view, &projection, fb_width as f32, fb_height as f32);

        // 3. Reset tile work queue
        tiles::reset();

        // 4. Signal worker cores (1-3) to start rendering
        smp::scheduler::start_render();

        // 5. Core 0 also helps rasterize tiles
        render_worker(0);

        // 6. Wait for all cores (0-3) to finish at the barrier
        smp::sync::RENDER_BARRIER.wait();

        // 7. Signal render complete (allows worker cores to wait for next frame)
        smp::scheduler::end_render();

        // Drop render context before drawing FPS (font uses its own lock)
        drop(render_ctx);

        // Draw FPS counter
        font::draw_fps(current_fps, fb_width);

        // Present: copy back buffer to display
        {
            let fb_guard = graphics::framebuffer::FRAMEBUFFER.lock();
            if let Some(fb) = fb_guard.as_ref() {
                fb.present();
            }
        }

        // Update FPS counter
        fps_frame_count += 1;
        let now = read_tsc();
        let elapsed = now.wrapping_sub(last_fps_time);
        if elapsed >= tsc_per_second {
            current_fps = fps_frame_count;
            fps_frame_count = 0;
            last_fps_time = now;
        }

        frame_count = frame_count.wrapping_add(1);

        // Frame limiter: wait until target frame time has elapsed
        loop {
            let now = read_tsc();
            let elapsed = now.wrapping_sub(last_frame_time);
            if elapsed >= tsc_per_frame {
                last_frame_time = now;
                break;
            }
            core::hint::spin_loop();
        }
    }

    halt_loop();
}

/// Transform mesh triangles, create ScreenTriangles, and bin them to tiles
/// Returns the number of triangles successfully binned
fn bin_mesh(
    mesh: &mesh::Mesh,
    model: &Mat4,
    view: &Mat4,
    projection: &Mat4,
    fb_width: f32,
    fb_height: f32,
) -> usize {
    let mut binned = 0;

    for i in 0..mesh.triangle_count() {
        if let Some((v0, v1, v2)) = mesh.get_triangle(i) {
            // Transform and create ScreenTriangle
            if let Some(screen_tri) = transform_and_bin(
                v0,
                v1,
                v2,
                model,
                view,
                projection,
                fb_width,
                fb_height,
            ) {
                // Add to frame buffer and get index
                if let Some(tri_idx) = tiles::add_triangle(screen_tri) {
                    // Bin to overlapping tiles
                    tiles::bin_triangle_lockfree(tri_idx, &screen_tri);
                    binned += 1;
                }
            }
        }
    }

    binned
}

/// Render worker for rasterizer cores (including Core 0)
/// Steals tiles from the work queue and rasterizes all triangles binned to each tile
/// IMPORTANT: This function must always complete normally - never return early
/// because all cores must hit the barrier after this returns
pub fn render_worker(_rasterizer_id: u8) {
    // Acquire render context for this worker
    let ctx = match rasterizer::RenderContext::acquire() {
        Some(c) => c,
        None => return, // Context not available - just return (barrier will be hit by caller)
    };

    // Work-stealing loop: grab tiles until none remain
    loop {
        // Get next tile from queue
        let tile_info = {
            let queue_guard = TILE_QUEUE.lock();
            match queue_guard.as_ref() {
                Some(queue) => {
                    match queue.get_next_tile_idx() {
                        Some(idx) => {
                            queue.get_tile(idx).map(|tile| {
                                (idx, tile.x as i32, tile.y as i32, tile.width as i32, tile.height as i32)
                            })
                        }
                        None => None, // No more tiles
                    }
                }
                None => None, // Queue not initialized
            }
        };

        match tile_info {
            Some((tile_idx, tile_x, tile_y, tile_w, tile_h)) => {
                // Rasterize all triangles in this tile's bin
                rasterize_tile(tile_idx, tile_x, tile_y, tile_w, tile_h, &ctx);
            }
            None => break, // No more tiles to process
        }
    }
}

/// Rasterize all triangles binned to a specific tile
fn rasterize_tile(
    tile_idx: usize,
    tile_x: i32,
    tile_y: i32,
    tile_w: i32,
    tile_h: i32,
    ctx: &rasterizer::RenderContext,
) {
    let bin = &TILE_BINS_LOCKFREE[tile_idx];
    let tri_count = bin.len();

    // Tile bounds
    let tile_min_x = tile_x;
    let tile_max_x = tile_x + tile_w - 1;
    let tile_min_y = tile_y;
    let tile_max_y = tile_y + tile_h - 1;

    // Rasterize each triangle in the bin
    for i in 0..tri_count {
        if let Some(tri_idx) = bin.get(i) {
            if let Some(tri) = tiles::get_triangle(tri_idx) {
                rasterize_screen_triangle_in_tile(
                    ctx,
                    &tri,
                    tile_min_x,
                    tile_max_x,
                    tile_min_y,
                    tile_max_y,
                );
            }
        }
    }
}

/// Network worker for network core
pub fn network_worker() {
    // Poll network stack with TSC-based timestamp
    let timestamp = (read_tsc() / 1_000_000) as i64; // Rough ms approximation
    net::stack::poll(timestamp);

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
