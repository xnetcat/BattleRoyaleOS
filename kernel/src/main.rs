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
mod ui;

use boot::{BASE_REVISION, HHDM_REQUEST, KERNEL_FILE_REQUEST, MEMORY_MAP_REQUEST};
use core::panic::PanicInfo;
use core::ffi::CStr;
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
use game::state::{GameState, PlayerPhase, get_state, set_state, MenuAction};

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
    // Check kernel arguments for server/client mode
    let mut is_server = false; // Default to client
    if let Some(file) = KERNEL_FILE_REQUEST.get_response() {
        // Explicitly handle CStr from Limine 0.5
        let cmd_cstr: &CStr = file.file().string();
        if let Ok(cmd_str) = cmd_cstr.to_str() {
            serial_println!("Kernel args: {:?}", cmd_cstr);
            if cmd_str.contains("server") {
                is_server = true;
            }
        }
    }
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

    // Create reusable meshes for game entities
    // Terrain must match MAP_SIZE (2000.0) so players can see/land on it from bus
    let terrain = mesh::create_terrain_grid(2000.0, 100, Vec3::new(0.2, 0.6, 0.3));
    let player_mesh = mesh::create_player_mesh(Vec3::new(0.3, 0.3, 0.8), Vec3::new(0.9, 0.7, 0.6));
    let wall_mesh = mesh::create_wall_mesh(Vec3::new(0.6, 0.5, 0.4));
    let bus_mesh = mesh::create_battle_bus_mesh();

    serial_println!("Meshes: terrain={} player={} wall={} bus={}",
        terrain.triangle_count(), player_mesh.triangle_count(),
        wall_mesh.triangle_count(), bus_mesh.triangle_count());

    // Camera setup
    // Far plane increased to 3000.0 to see across the 2000x2000 map from bus height
    let aspect = fb_width as f32 / fb_height as f32;
    let fov_radians = core::f32::consts::PI / 3.0;
    let projection = perspective(fov_radians, aspect, 0.1, 3000.0);

    serial_println!("Parallel rendering: 4 cores active");

    // Menu state
    let mut main_menu = ui::main_menu::MainMenuScreen::new(fb_width, fb_height);
    let mut settings_screen = ui::settings::SettingsScreen::new(fb_width, fb_height);
    let mut customization_screen = ui::customization::CustomizationScreen::new(fb_width, fb_height);
    let mut server_select_screen = ui::server_select::ServerSelectScreen::new(fb_width, fb_height);
    let mut fortnite_lobby = ui::fortnite_lobby::FortniteLobby::new(fb_width, fb_height);
    let mut test_map_screen = ui::test_map::TestMapScreen::new(fb_width, fb_height);
    let mut lobby_screen = ui::lobby::LobbyScreen::new(fb_width, fb_height);

    // Previous key state for edge detection
    let mut prev_key_state = game::input::KeyState::default();

    // Local player tracking
    let mut local_player_id: Option<u8> = None;
    let mut player_yaw: f32 = 0.0;
    let mut player_pitch: f32 = 0.0;
    let mut input_sequence: u32 = 0;

    // Previous mouse state for click detection
    let mut prev_mouse_left = false;

    // Countdown timer
    let mut countdown_timer = 0.0f32;

    loop {
        // Poll keyboard
        game::input::poll_keyboard();
        let key_state = game::input::KEY_STATE.lock().clone();

        // Sync local player ID from world if not set
        if local_player_id.is_none() {
            if let Some(world) = game::world::GAME_WORLD.lock().as_ref() {
                local_player_id = world.local_player_id;
            }
        }

        // Get menu action from key state (edge-triggered)
        let menu_action = get_menu_action(&key_state, &prev_key_state);
        prev_key_state = key_state.clone();

        // Handle game state
        let current_state = get_state();

        match current_state {
            GameState::PartyLobby => {
                // Check for 'T' key to enter test map
                if key_state.t && !prev_key_state.t {
                    set_state(GameState::TestMap);
                    continue;
                }

                // Update Fortnite-style party lobby
                fortnite_lobby.tick();
                if let Some(new_state) = fortnite_lobby.update(menu_action) {
                    set_state(new_state);

                    // If starting matchmaking, prepare for game
                    if matches!(new_state, GameState::Matchmaking { .. }) {
                        // In offline mode, skip matchmaking and go straight to countdown
                        countdown_timer = 5.0;
                        game::world::init(true);

                        // Add local player
                        local_player_id = {
                            let mut world = game::world::GAME_WORLD.lock();
                            if let Some(w) = world.as_mut() {
                                let id = w.add_player("LocalPlayer", smoltcp::wire::Ipv4Address::new(127, 0, 0, 1), 5000);
                                w.local_player_id = id;
                                id
                            } else {
                                None
                            }
                        };

                        // Skip matchmaking in offline mode - go directly to countdown
                        set_state(GameState::LobbyCountdown { remaining_secs: 5 });
                    }
                }

                // First render 3D player preview (includes sunset background)
                render_lobby_frame(fb_width, fb_height, &fortnite_lobby, &projection);

                // Then draw lobby UI overlay on top (skip background since 3D is rendered)
                let ctx = match rasterizer::RenderContext::acquire() {
                    Some(ctx) => ctx,
                    None => continue,
                };
                fortnite_lobby.draw_ui_only(&ctx, fb_width, fb_height, true);
                drop(ctx);

                // Draw cursor and present
                {
                    let fb_guard = graphics::framebuffer::FRAMEBUFFER.lock();
                    if let Some(fb) = fb_guard.as_ref() {
                        let mouse = game::input::get_mouse_state();
                        graphics::cursor::draw_cursor(fb, mouse.x, mouse.y);
                        fb.present();
                    }
                }
                // Draw cursor and present
                {
                    let fb_guard = graphics::framebuffer::FRAMEBUFFER.lock();
                    if let Some(fb) = fb_guard.as_ref() {
                        let mouse = game::input::get_mouse_state();
                        graphics::cursor::draw_cursor(fb, mouse.x, mouse.y);
                        fb.present();
                    }
                }
            }

            GameState::ServerSelect => {
                // Update server select screen
                if let Some(new_state) = server_select_screen.update(menu_action) {
                    set_state(new_state);
                    
                    // If returning to matchmaking or starting, re-init world if needed
                    // handled by update() setting network mode
                }

                // Render server select
                render_menu_frame(fb_width, fb_height, |ctx| {
                    server_select_screen.draw(ctx, fb_width, fb_height);
                });
            }

            GameState::Settings => {
                // Update settings screen
                if let Some(new_state) = settings_screen.update(menu_action) {
                    set_state(new_state);
                }

                // Render settings
                render_menu_frame(fb_width, fb_height, |ctx| {
                    settings_screen.draw(ctx, fb_width, fb_height);
                });
            }

            GameState::Customization => {
                // Update customization screen
                if let Some(new_state) = customization_screen.update(menu_action) {
                    set_state(new_state);
                }

                // Render customization with 3D preview
                render_menu_frame(fb_width, fb_height, |ctx| {
                    customization_screen.draw(ctx, fb_width, fb_height, rotation);
                });
                rotation += 0.02;
            }

            GameState::Matchmaking { elapsed_secs } => {
                // Show matchmaking screen
                // In offline mode, this is skipped, but keeping for future multiplayer
                render_menu_frame(fb_width, fb_height, |ctx| {
                    ui::game_ui::draw_matchmaking(ctx, fb_width, fb_height, elapsed_secs);
                });

                // ESC to cancel
                if menu_action == MenuAction::Back {
                    set_state(GameState::PartyLobby);
                }
            }

            GameState::LobbyIsland => {
                // Warmup island - for multiplayer (skip in offline mode)
                // For now, just go to countdown
                set_state(GameState::LobbyCountdown { remaining_secs: 10 });
            }

            GameState::LobbyCountdown { remaining_secs } => {
                countdown_timer -= 1.0 / 60.0;

                if countdown_timer <= 0.0 {
                    set_state(GameState::BusPhase);
                    // Spawn bots for single-player mode
                    if let Some(world) = game::world::GAME_WORLD.lock().as_mut() {
                        world.spawn_bots(10); // 10 bots for a battle
                    }
                } else {
                    let new_secs = libm::ceilf(countdown_timer) as u8;
                    if new_secs != remaining_secs {
                        set_state(GameState::LobbyCountdown { remaining_secs: new_secs });
                    }
                }

                // Render countdown
                render_menu_frame(fb_width, fb_height, |ctx| {
                    ui::game_ui::draw_countdown(ctx, fb_width, fb_height, remaining_secs);
                });
            }

            GameState::TestMap => {
                // Update test map
                test_map_screen.tick();
                if let Some(new_state) = test_map_screen.update(menu_action) {
                    set_state(new_state);
                }

                // Render test map with 3D model preview
                render_test_map_frame(
                    fb_width, fb_height,
                    &test_map_screen,
                    &projection,
                );
            }

            GameState::BusPhase | GameState::InGame => {
                // Check for escape to return to party lobby
                if menu_action == MenuAction::Back {
                    set_state(GameState::PartyLobby);
                    continue;
                }

                // Get mouse state for camera control
                let mouse = game::input::get_mouse_state();

                // Apply keyboard and mouse input to local player
                if let Some(id) = local_player_id {
                    // Mouse look sensitivity
                    const MOUSE_SENSITIVITY: f32 = 0.003;

                    // Update player rotation with mouse movement
                    player_yaw += mouse.delta_x as f32 * MOUSE_SENSITIVITY;
                    player_pitch += mouse.delta_y as f32 * MOUSE_SENSITIVITY;
                    // Clamp pitch to prevent camera flipping
                    player_pitch = player_pitch.clamp(-1.5, 1.5);

                    // Also support A/D keys for rotation (tank controls)
                    if key_state.a {
                        player_yaw -= 0.05;
                    }
                    if key_state.d {
                        player_yaw += 0.05;
                    }

                    // Create input from keyboard + mouse state
                    input_sequence += 1;
                    let input = protocol::packets::ClientInput {
                        player_id: id,
                        sequence: input_sequence,
                        forward: if key_state.w { 1 } else if key_state.s { -1 } else { 0 },
                        strafe: 0, // A/D used for rotation
                        jump: key_state.space,
                        crouch: key_state.ctrl,
                        // Fire with left click OR shift key
                        fire: mouse.left_button || key_state.shift,
                        build: key_state.b,
                        exit_bus: key_state.space,
                        yaw: (player_yaw.to_degrees() * 100.0) as i16,
                        pitch: (player_pitch.to_degrees() * 100.0) as i16,
                    };

                    // Apply input to game world
                    if let Some(world) = game::world::GAME_WORLD.lock().as_mut() {
                        world.apply_input(id, &input);

                        // Handle weapon slot selection (1-5 keys)
                        if let Some(player) = world.get_player_mut(id) {
                            if key_state.one && !prev_key_state.one {
                                player.inventory.select_pickaxe();
                            } else if key_state.two && !prev_key_state.two {
                                player.inventory.select_slot(0);
                            } else if key_state.three && !prev_key_state.three {
                                player.inventory.select_slot(1);
                            } else if key_state.four && !prev_key_state.four {
                                player.inventory.select_slot(2);
                            } else if key_state.five && !prev_key_state.five {
                                player.inventory.select_slot(3);
                            }

                            // Handle reload (R key)
                            if key_state.r && !prev_key_state.r {
                                player.inventory.reload_current();
                            }
                        }

                        // Handle loot pickup (E key)
                        if key_state.e && !prev_key_state.e {
                            world.try_pickup(id);
                        }
                    }
                }

                // Reset mouse deltas after use
                game::input::reset_mouse_deltas();

                // Update game world physics and check for victory
                if let Some(world) = game::world::GAME_WORLD.lock().as_mut() {
                    world.update(1.0 / 60.0);

                    // Check for victory condition
                    if let Some(id) = world.check_victory() {
                        set_state(GameState::Victory { winner_id: Some(id) });
                    }
                }

                // Process network (less frequently)
                if frame_count % 10 == 0 {
                    net::protocol::process_incoming();
                    net::protocol::broadcast_world_state();
                }

                // Poll network stack every frame
                net::stack::poll(frame_count as i64);

                // Render game world
                render_game_frame(
                    fb_width, fb_height,
                    &terrain, &player_mesh, &wall_mesh, &bus_mesh,
                    &projection, local_player_id, rotation,
                    current_fps,
                );
                rotation += 0.01;
            }

            GameState::Victory { winner_id } => {
                // Check for any key to return to party lobby
                if menu_action == MenuAction::Select || menu_action == MenuAction::Back {
                    set_state(GameState::PartyLobby);
                }

                // Render victory screen
                render_menu_frame(fb_width, fb_height, |ctx| {
                    ui::game_ui::draw_victory(ctx, fb_width, fb_height, winner_id);
                });
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

/// Get menu action from key state (edge-triggered)
fn get_menu_action(current: &game::input::KeyState, prev: &game::input::KeyState) -> MenuAction {
    // Edge detection - only trigger on key press, not hold
    if current.w && !prev.w || current.up && !prev.up {
        return MenuAction::Up;
    }
    if current.s && !prev.s || current.down && !prev.down {
        return MenuAction::Down;
    }
    if current.a && !prev.a || current.left && !prev.left {
        return MenuAction::Left;
    }
    if current.d && !prev.d || current.right && !prev.right {
        return MenuAction::Right;
    }
    if current.enter && !prev.enter || current.space && !prev.space {
        return MenuAction::Select;
    }
    if current.escape && !prev.escape {
        return MenuAction::Back;
    }
    MenuAction::None
}

/// Render a menu frame (2D UI only) with mouse cursor
fn render_menu_frame<F>(fb_width: usize, fb_height: usize, draw_fn: F)
where
    F: FnOnce(&rasterizer::RenderContext),
{
    // Acquire render context
    let render_ctx = match rasterizer::RenderContext::acquire() {
        Some(ctx) => ctx,
        None => return,
    };

    // Clear to dark background
    render_ctx.clear(rgb(20, 25, 40));

    // Draw menu content
    draw_fn(&render_ctx);

    // Drop context
    drop(render_ctx);

    // Draw cursor and present
    {
        let fb_guard = graphics::framebuffer::FRAMEBUFFER.lock();
        if let Some(fb) = fb_guard.as_ref() {
            // Draw mouse cursor on top of everything
            let mouse = game::input::get_mouse_state();
            graphics::cursor::draw_cursor(fb, mouse.x, mouse.y);
            fb.present();
        }
    }
}

/// Render the test map / model gallery
fn render_test_map_frame(
    fb_width: usize,
    fb_height: usize,
    test_map: &ui::test_map::TestMapScreen,
    projection: &Mat4,
) {
    use renderer::voxel_models;
    use renderer::voxel::CharacterCustomization;

    // Acquire render context
    let render_ctx = match rasterizer::RenderContext::acquire() {
        Some(ctx) => ctx,
        None => return,
    };

    // Clear to dark background
    render_ctx.clear(rgb(20, 25, 40));
    render_ctx.clear_zbuffer();

    // Get current model mesh
    let model_index = test_map.get_model_index();
    let rotation = test_map.get_rotation();
    let zoom = test_map.get_zoom();

    // Create mesh based on model index
    let model_mesh = match model_index {
        0 => voxel_models::create_player_model(&CharacterCustomization::default()).to_mesh(0.1 * zoom),
        1 => voxel_models::create_shotgun_model().to_mesh(0.15 * zoom),
        2 => voxel_models::create_ar_model().to_mesh(0.15 * zoom),
        3 => voxel_models::create_pistol_model().to_mesh(0.2 * zoom),
        4 => voxel_models::create_smg_model().to_mesh(0.15 * zoom),
        5 => voxel_models::create_sniper_model().to_mesh(0.12 * zoom),
        6 => voxel_models::create_pickaxe_model().to_mesh(0.15 * zoom),
        7 => voxel_models::create_glider_model(0).to_mesh(0.08 * zoom),
        8 => voxel_models::create_glider_model(1).to_mesh(0.08 * zoom),
        9 => voxel_models::create_glider_model(2).to_mesh(0.08 * zoom),
        10 => voxel_models::create_glider_model(3).to_mesh(0.08 * zoom),
        11 => voxel_models::create_pine_tree().to_mesh(0.1 * zoom),
        12 => voxel_models::create_oak_tree().to_mesh(0.1 * zoom),
        13 => voxel_models::create_rock(0).to_mesh(0.2 * zoom),
        14 => voxel_models::create_wall_wood().to_mesh(0.1 * zoom),
        15 => voxel_models::create_wall_brick().to_mesh(0.1 * zoom),
        16 => voxel_models::create_wall_metal().to_mesh(0.1 * zoom),
        17 => voxel_models::create_floor_wood().to_mesh(0.1 * zoom),
        18 => voxel_models::create_ramp_wood().to_mesh(0.1 * zoom),
        19 => voxel_models::create_battle_bus().to_mesh(0.05 * zoom),
        20 => voxel_models::create_chest().to_mesh(0.2 * zoom),
        21 => voxel_models::create_backpack_model(1).to_mesh(0.2 * zoom),
        22 => voxel_models::create_backpack_model(2).to_mesh(0.2 * zoom),
        _ => voxel_models::create_backpack_model(3).to_mesh(0.2 * zoom),
    };

    // Camera setup - orbit around the model
    let camera_dist = 8.0;
    let camera_height = 3.0;
    let camera_pos = Vec3::new(
        libm::sinf(rotation) * camera_dist,
        camera_height,
        libm::cosf(rotation) * camera_dist,
    );
    let camera_target = Vec3::new(0.0, 1.0, 0.0);
    let view = look_at(camera_pos, camera_target, Vec3::Y);

    // Clear tile bins
    tiles::clear_lockfree_bins();
    tiles::reset_triangle_buffer();

    // Transform and bin the model
    let model_matrix = Mat4::IDENTITY;
    bin_mesh(&model_mesh, &model_matrix, &view, projection, fb_width as f32, fb_height as f32);

    // Reset and render tiles
    tiles::reset();
    smp::scheduler::start_render();
    render_worker(0);
    smp::sync::RENDER_BARRIER.wait();
    smp::scheduler::end_render();

    drop(render_ctx);

    // Draw UI overlay
    let ctx = match rasterizer::RenderContext::acquire() {
        Some(ctx) => ctx,
        None => return,
    };
    test_map.draw(&ctx, fb_width, fb_height);
    drop(ctx);

    // Draw cursor and present
    {
        let fb_guard = graphics::framebuffer::FRAMEBUFFER.lock();
        if let Some(fb) = fb_guard.as_ref() {
            let mouse = game::input::get_mouse_state();
            graphics::cursor::draw_cursor(fb, mouse.x, mouse.y);
            fb.present();
        }
    }
}

/// Render the lobby frame with 3D player preview
fn render_lobby_frame(
    fb_width: usize,
    fb_height: usize,
    lobby: &ui::fortnite_lobby::FortniteLobby,
    projection: &Mat4,
) {
    use renderer::voxel_models;
    use game::state::PLAYER_CUSTOMIZATION;

    // Acquire render context
    let render_ctx = match rasterizer::RenderContext::acquire() {
        Some(ctx) => ctx,
        None => return,
    };

    // Draw sunset gradient background first
    draw_sunset_gradient(&render_ctx, fb_width, fb_height);

    // Clear z-buffer for 3D rendering
    render_ctx.clear_zbuffer();

    // Get current player customization
    let custom = PLAYER_CUSTOMIZATION.lock();
    let renderer_custom = custom.to_renderer();
    drop(custom);

    // Create player mesh from voxel model
    let player_mesh = voxel_models::create_player_model(&renderer_custom).to_mesh(0.15);

    // Create a simple platform mesh (flat quad)
    let platform_mesh = mesh::create_terrain_grid(3.0, 2, Vec3::new(0.2, 0.3, 0.5));

    // Camera setup - orbit around the player
    let rotation = lobby.get_rotation();
    let camera_dist = 6.0;
    let camera_height = 2.0;
    let camera_pos = Vec3::new(
        libm::sinf(rotation) * camera_dist,
        camera_height,
        libm::cosf(rotation) * camera_dist,
    );
    let camera_target = Vec3::new(0.0, 1.2, 0.0);
    let view = look_at(camera_pos, camera_target, Vec3::Y);

    // Clear tile bins
    tiles::clear_lockfree_bins();
    tiles::reset_triangle_buffer();

    // Transform and bin the platform
    let platform_model = Mat4::from_translation(Vec3::new(0.0, -0.1, 0.0));
    bin_mesh(&platform_mesh, &platform_model, &view, projection, fb_width as f32, fb_height as f32);

    // Transform and bin the player model (standing on platform)
    let player_model = Mat4::from_translation(Vec3::new(0.0, 0.0, 0.0));
    bin_mesh(&player_mesh, &player_model, &view, projection, fb_width as f32, fb_height as f32);

    // Reset and render tiles
    tiles::reset();
    smp::scheduler::start_render();
    render_worker(0);
    smp::sync::RENDER_BARRIER.wait();
    smp::scheduler::end_render();

    drop(render_ctx);
}

/// Draw sunset gradient background for lobby
fn draw_sunset_gradient(_ctx: &rasterizer::RenderContext, fb_width: usize, fb_height: usize) {
    let fb_guard = graphics::framebuffer::FRAMEBUFFER.lock();
    let fb = match fb_guard.as_ref() {
        Some(f) => f,
        None => return,
    };

    // Sunset gradient: orange -> pink -> purple -> dark blue
    let colors_top = [0xFFu8, 0x8C, 0x00]; // Orange
    let colors_mid1 = [0xFF, 0x69, 0xB4]; // Pink
    let colors_mid2 = [0x94, 0x00, 0xD3]; // Purple
    let colors_bot = [0x19, 0x19, 0x70];  // Dark blue

    for y in 0..fb_height.min(fb.height) {
        let t = y as f32 / fb_height as f32;

        let (r, g, b) = if t < 0.3 {
            let local_t = t / 0.3;
            (
                lerp_u8(colors_top[0], colors_mid1[0], local_t),
                lerp_u8(colors_top[1], colors_mid1[1], local_t),
                lerp_u8(colors_top[2], colors_mid1[2], local_t),
            )
        } else if t < 0.6 {
            let local_t = (t - 0.3) / 0.3;
            (
                lerp_u8(colors_mid1[0], colors_mid2[0], local_t),
                lerp_u8(colors_mid1[1], colors_mid2[1], local_t),
                lerp_u8(colors_mid1[2], colors_mid2[2], local_t),
            )
        } else {
            let local_t = (t - 0.6) / 0.4;
            (
                lerp_u8(colors_mid2[0], colors_bot[0], local_t),
                lerp_u8(colors_mid2[1], colors_bot[1], local_t),
                lerp_u8(colors_mid2[2], colors_bot[2], local_t),
            )
        };

        let color = ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);

        for x in 0..fb_width.min(fb.width) {
            fb.put_pixel(x, y, color);
        }
    }
}

/// Linear interpolation for u8
fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    ((a as f32) + (b as f32 - a as f32) * t) as u8
}

/// Render a game frame (3D world + HUD)
fn render_game_frame(
    fb_width: usize,
    fb_height: usize,
    terrain: &mesh::Mesh,
    player_mesh: &mesh::Mesh,
    wall_mesh: &mesh::Mesh,
    bus_mesh: &mesh::Mesh,
    projection: &Mat4,
    local_player_id: Option<u8>,
    rotation: f32,
    current_fps: u32,
) {
    // Acquire render context for this frame
    let render_ctx = match rasterizer::RenderContext::acquire() {
        Some(ctx) => ctx,
        None => return,
    };

    // Clear back buffer and z-buffer (double buffering prevents flicker)
    render_ctx.clear(rgb(30, 30, 50));
    render_ctx.clear_zbuffer();

    // Get camera position from local player (or default orbit)
    let (camera_pos, camera_target) = {
        let world = game::world::GAME_WORLD.lock();
        if let (Some(w), Some(id)) = (world.as_ref(), local_player_id) {
            if let Some(player) = w.get_player(id) {
                // Third-person camera behind player
                let cam_offset = Vec3::new(
                    -libm::sinf(player.yaw) * 5.0,
                    3.0,
                    -libm::cosf(player.yaw) * 5.0,
                );
                let pos = player.position + cam_offset;
                let target = player.position + Vec3::new(0.0, 1.0, 0.0);
                (pos, target)
            } else {
                // Default orbit camera
                let dist = 20.0;
                (Vec3::new(libm::sinf(rotation) * dist, 10.0, libm::cosf(rotation) * dist), Vec3::ZERO)
            }
        } else {
            let dist = 20.0;
            (Vec3::new(libm::sinf(rotation) * dist, 10.0, libm::cosf(rotation) * dist), Vec3::ZERO)
        }
    };
    let view = look_at(camera_pos, camera_target, Vec3::Y);

    // === PARALLEL RENDERING (4 cores) ===

    // 1. Clear lock-free bins and reset triangle buffer
    tiles::clear_lockfree_bins();
    tiles::reset_triangle_buffer();

    // 2. Transform and bin terrain
    let terrain_model = Mat4::from_translation(Vec3::new(0.0, 0.0, 0.0));
    bin_mesh(terrain, &terrain_model, &view, projection, fb_width as f32, fb_height as f32);

    // 3. Render game world entities
    {
        let world = game::world::GAME_WORLD.lock();
        if let Some(w) = world.as_ref() {
            // Render battle bus if active
            if w.bus.active {
                let bus_model = Mat4::from_translation(w.bus.position);
                bin_mesh(bus_mesh, &bus_model, &view, projection, fb_width as f32, fb_height as f32);
            }

            // Render all players
            for player in &w.players {
                if !player.is_alive() || player.phase == PlayerPhase::OnBus {
                    continue;
                }
                let model = Mat4::from_translation(player.position)
                    * Mat4::from_rotation_y(player.yaw);
                bin_mesh(player_mesh, &model, &view, projection, fb_width as f32, fb_height as f32);
            }

            // Render buildings
            for building in &w.buildings {
                let model = Mat4::from_translation(building.position)
                    * Mat4::from_rotation_y(building.rotation);
                bin_mesh(wall_mesh, &model, &view, projection, fb_width as f32, fb_height as f32);
            }
        }
    }

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

    // Draw game HUD (health, materials, alive count)
    {
        let world_guard = game::world::GAME_WORLD.lock();
        if let Some(world) = world_guard.as_ref() {
            let (health, shield, _materials) = if let Some(id) = local_player_id {
                if let Some(player) = world.get_player(id) {
                    (player.health, player.shield, player.inventory.materials.total())
                } else {
                    (100, 0, 0)
                }
            } else {
                (100, 0, 0)
            };
            let alive = world.players.iter().filter(|p| p.health > 0).count();
            let total = world.players.len();
            let total = world.players.len();
            font::draw_hud(health, shield as u32, alive, total, fb_width, fb_height);

            // Render name tags
            if let Some(fb_guard) = graphics::framebuffer::FRAMEBUFFER.try_lock() {
                if let Some(fb) = fb_guard.as_ref() {
                    for player in &world.players {
                        if !player.is_alive() || player.phase == PlayerPhase::OnBus {
                            continue;
                        }

                        // Don't draw own name tag
                        if let Some(local_id) = local_player_id {
                            if player.id == local_id {
                                continue;
                            }
                        }

                        // Project position
                        let head_pos = player.position + Vec3::new(0.0, 2.2, 0.0);
                        let model = Mat4::IDENTITY; // World space
                        if let Some(screen_pos) = graphics::pipeline::project_point(
                            head_pos,
                            &model,
                            &view,
                            projection,
                            fb_width as f32,
                            fb_height as f32
                        ) {
                            // Check distance for scaling/culling
                            // screen_pos.z is NDC depth (0-1).
                            if screen_pos.z >= 0.0 && screen_pos.z <= 1.0 {
                                let name = &player.name;
                                let color = crate::graphics::ui::colors::WHITE;
                                font::draw_string_centered_raw(
                                    fb,
                                    screen_pos.y as usize,
                                    name,
                                    color,
                                    1
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    // Present: copy back buffer to display
    {
        let fb_guard = graphics::framebuffer::FRAMEBUFFER.lock();
        if let Some(fb) = fb_guard.as_ref() {
            fb.present();
        }
    }
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
