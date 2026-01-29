//! BattleRoyaleOS Kernel
//!
//! A bare-metal unikernel OS for running a 100-player Battle Royale game.

#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

extern crate alloc;

use core::sync::atomic::{AtomicBool, Ordering};

/// Global benchmark mode flag - set by kernel_main, read by main_loop
static BENCHMARK_MODE: AtomicBool = AtomicBool::new(false);

/// Global server mode flag - disables all rendering when true
static SERVER_MODE: AtomicBool = AtomicBool::new(false);

/// Global test mode flag - spawns all items for testing
static TEST_MODE: AtomicBool = AtomicBool::new(false);

/// Global GPU batch enabled flag - checked once at init, used per-frame without locks
static GPU_BATCH_AVAILABLE: AtomicBool = AtomicBool::new(false);

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
    culling::CullContext,
    font,
    framebuffer::rgb,
    gpu_batch,
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
    let (fb_width, fb_height) = if is_server {
        serial_println!("SERVER MODE: Skipping GPU initialization");
        (0, 0)
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

        // Initialize GPU batch renderer - sets GPU_BATCH_AVAILABLE flag
        let gpu_batch_ok = graphics::gpu_batch::init(w as u32, h as u32);
        GPU_BATCH_AVAILABLE.store(gpu_batch_ok, Ordering::Release);

        // Initialize z-buffer
        zbuffer::init(w, h);
        serial_println!("Z-buffer initialized");

        // Initialize tile system
        tiles::init(w, h);
        if let Some(queue) = tiles::TILE_QUEUE.lock().as_ref() {
            serial_println!("Tile system: {} tiles", queue.tile_count());
            tiles::init_bins(queue.tile_count());
        }

        // Initialize vsync subsystem
        graphics::vsync::init();

        (w, h)
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

    // Store mode flags for main loop (global statics)
    BENCHMARK_MODE.store(benchmark_mode, core::sync::atomic::Ordering::SeqCst);
    SERVER_MODE.store(is_server, core::sync::atomic::Ordering::SeqCst);
    TEST_MODE.store(test_mode, core::sync::atomic::Ordering::SeqCst);

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
        // Main game loop with rendering
        main_loop(fb_width, fb_height);
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

/// Main game loop (runs on Core 0)
fn main_loop(fb_width: usize, fb_height: usize) -> ! {
    let mut frame_count = 0u32;
    let mut rotation = 0.0f32;

    // Frame timer with vsync support (replaces manual FPS tracking and busy-waiting)
    // Uses HLT instruction for CPU idle when waiting, reducing power consumption
    let mut frame_timer = graphics::vsync::FrameTimer::new();

    // TSC frequency for benchmark reporting (assume ~2GHz for QEMU)
    let tsc_per_second: u64 = 2_000_000_000;

    // Create reusable meshes for game entities using VOXEL MODELS
    // Terrain: 3D heightmap with proper hills
    let terrain = create_3d_terrain(2000.0, 50); // 50 subdivisions for good balance

    // Player mesh from detailed voxel model (use default customization for now)
    let default_custom = renderer::voxel::CharacterCustomization::default();
    let player_mesh = renderer::voxel_models::create_player_model(&default_custom).to_mesh(0.15);

    // Building pieces from voxel models
    let wall_mesh = renderer::voxel_models::create_wall_wood().to_mesh(0.25);

    // Battle bus from detailed voxel model (includes balloon)
    let bus_mesh = renderer::voxel_models::create_battle_bus().to_mesh(0.15);

    // Additional meshes for complete game rendering
    let glider_mesh = renderer::voxel_models::create_glider_model(0).to_mesh(0.15);
    let tree_pine_mesh = renderer::voxel_models::create_pine_tree().to_mesh(0.5);
    let tree_oak_mesh = renderer::voxel_models::create_oak_tree().to_mesh(0.5);
    let rock_mesh = renderer::voxel_models::create_rock(0).to_mesh(0.4);
    let chest_mesh = renderer::voxel_models::create_chest().to_mesh(0.15);
    let house_mesh = renderer::map_mesh::create_house_mesh_simple(Vec3::new(0.7, 0.6, 0.5));
    let storm_wall_mesh = mesh::create_storm_wall(48, 200.0); // 48 segments, 200 units tall

    // Weapon meshes from detailed voxel models
    let shotgun_mesh = renderer::voxel_models::create_shotgun_model().to_mesh(0.08);
    let ar_mesh = renderer::voxel_models::create_ar_model().to_mesh(0.08);
    let sniper_mesh = renderer::voxel_models::create_sniper_model().to_mesh(0.08);

    serial_println!("Meshes: terrain={} player={} wall={} bus={} glider={} tree={} chest={}",
        terrain.triangle_count(), player_mesh.triangle_count(),
        wall_mesh.triangle_count(), bus_mesh.triangle_count(),
        glider_mesh.triangle_count(), tree_pine_mesh.triangle_count(),
        chest_mesh.triangle_count());

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

    // Check for benchmark/test mode - auto-start game
    let benchmark = BENCHMARK_MODE.load(core::sync::atomic::Ordering::SeqCst);
    let test_mode = TEST_MODE.load(core::sync::atomic::Ordering::SeqCst);
    let auto_start = benchmark || test_mode;
    let mut auto_started = false;
    let mut benchmark_frames = 0u32;
    let mut benchmark_start_time = 0u64;

    loop {
        // Auto-start mode (benchmark or test): start game after a few frames
        if auto_start && !auto_started && frame_count > 10 {
            auto_started = true;
            benchmark_start_time = read_tsc();

            if test_mode {
                serial_println!("TEST MODE: Starting with all items spawned...");
            } else {
                serial_println!("BENCHMARK: Starting InGame test...");
            }

            // Create a local player and put them in the game
            if let Some(world) = game::world::GAME_WORLD.lock().as_mut() {
                // Add a player if none exists
                if world.players.is_empty() {
                    use smoltcp::wire::Ipv4Address;
                    let player_name = if test_mode { "TestPlayer" } else { "Benchmark" };
                    let player = game::player::Player::new(0, player_name, Ipv4Address::new(127, 0, 0, 1), 5000);
                    world.players.push(player);
                    world.local_player_id = Some(0);
                    local_player_id = Some(0);
                }

                // Set player to grounded (not on bus)
                if let Some(p) = world.players.get_mut(0) {
                    p.phase = game::state::PlayerPhase::Grounded;
                    p.position = Vec3::new(50.0, 5.0, 50.0);

                    // Test mode: give player all weapons
                    if test_mode {
                        use game::weapon::{WeaponType, Weapon, Rarity};
                        p.inventory.add_weapon(Weapon::new(WeaponType::AssaultRifle, Rarity::Legendary));
                        p.inventory.add_weapon(Weapon::new(WeaponType::Shotgun, Rarity::Epic));
                        p.inventory.add_weapon(Weapon::new(WeaponType::Sniper, Rarity::Legendary));
                        p.inventory.add_weapon(Weapon::new(WeaponType::Smg, Rarity::Rare));
                        p.inventory.add_weapon(Weapon::new(WeaponType::Pistol, Rarity::Uncommon));
                        p.inventory.materials.wood = 500;
                        p.inventory.materials.brick = 500;
                        p.inventory.materials.metal = 500;
                        serial_println!("TEST: Gave player all weapons and materials");
                    }
                }

                // Test mode: spawn all item types in a grid around the player
                if test_mode {
                    use game::weapon::{WeaponType, Weapon, Rarity};
                    use game::loot::{LootItem, ChestTier};

                    // Spawn weapons in a circle around the player
                    let weapons = [
                        WeaponType::Pistol,
                        WeaponType::Smg,
                        WeaponType::AssaultRifle,
                        WeaponType::Shotgun,
                        WeaponType::Sniper,
                    ];

                    let rarities = [
                        Rarity::Common,
                        Rarity::Uncommon,
                        Rarity::Rare,
                        Rarity::Epic,
                        Rarity::Legendary,
                    ];

                    let center = Vec3::new(50.0, 0.5, 50.0);
                    let mut spawn_count = 0;

                    // Spawn each weapon type in each rarity
                    for (i, weapon_type) in weapons.iter().enumerate() {
                        for (j, rarity) in rarities.iter().enumerate() {
                            let angle = (i * 5 + j) as f32 * 0.4;
                            let radius = 10.0 + (i as f32 * 3.0);
                            let pos = Vec3::new(
                                center.x + libm::cosf(angle) * radius,
                                0.5,
                                center.z + libm::sinf(angle) * radius,
                            );
                            let weapon = Weapon::new(*weapon_type, *rarity);
                            world.loot.spawn_drop(pos, LootItem::Weapon(weapon), false);
                            spawn_count += 1;
                        }
                    }

                    // Spawn chest loot (simulates opened chests) in a ring
                    for i in 0..12 {
                        let angle = i as f32 * (core::f32::consts::TAU / 12.0);
                        let pos = Vec3::new(
                            center.x + libm::cosf(angle) * 30.0,
                            0.5,
                            center.z + libm::sinf(angle) * 30.0,
                        );
                        // Spawn chest loot (weapon + ammo + maybe healing)
                        world.loot.spawn_chest_loot(pos, ChestTier::Rare);
                        spawn_count += 3; // Chest spawns ~3 items
                    }

                    // Spawn healing items
                    for i in 0..8 {
                        let angle = i as f32 * (core::f32::consts::TAU / 8.0);
                        let pos = Vec3::new(
                            center.x + libm::cosf(angle) * 20.0,
                            0.5,
                            center.z + libm::sinf(angle) * 20.0,
                        );
                        // Alternate between health and shield items
                        let item = if i % 2 == 0 {
                            LootItem::Health { amount: 100, use_time: 10.0, max_health: 100 }
                        } else {
                            LootItem::Shield { amount: 50, use_time: 5.0 }
                        };
                        world.loot.spawn_drop(pos, item, false);
                        spawn_count += 1;
                    }

                    // Spawn materials
                    for i in 0..6 {
                        let angle = i as f32 * (core::f32::consts::TAU / 6.0);
                        let pos = Vec3::new(
                            center.x + libm::cosf(angle) * 15.0,
                            0.5,
                            center.z + libm::sinf(angle) * 15.0,
                        );
                        let item = LootItem::Materials {
                            wood: 100,
                            brick: 100,
                            metal: 100,
                        };
                        world.loot.spawn_drop(pos, item, false);
                        spawn_count += 1;
                    }

                    serial_println!("TEST: Spawned {} items around player", spawn_count);
                }
            }

            // Jump straight to InGame state
            set_state(GameState::InGame);
        }

        // Benchmark: report FPS every 60 frames
        if benchmark && auto_started {
            benchmark_frames += 1;
            if benchmark_frames % 60 == 0 {
                let elapsed = read_tsc().wrapping_sub(benchmark_start_time);
                let secs = elapsed as f64 / tsc_per_second as f64;
                let avg_fps = benchmark_frames as f64 / secs;
                serial_println!("BENCHMARK: {} frames in {:.2}s = {:.1} avg FPS (current: {})",
                    benchmark_frames, secs, avg_fps, frame_timer.fps());
            }
        }

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
                        drop(fb_guard);
                        graphics::gpu::present();
                    }
                }
                // Draw cursor and present
                {
                    let fb_guard = graphics::framebuffer::FRAMEBUFFER.lock();
                    if let Some(fb) = fb_guard.as_ref() {
                        let mouse = game::input::get_mouse_state();
                        graphics::cursor::draw_cursor(fb, mouse.x, mouse.y);
                        drop(fb_guard);
                        graphics::gpu::present();
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
                    // Mouse look sensitivity (adjusted for smooth camera)
                    const MOUSE_SENSITIVITY: f32 = 0.002;

                    // Update camera rotation with mouse movement ONLY
                    // Mouse delta is accumulated, so we use it and reset
                    player_yaw += mouse.delta_x as f32 * MOUSE_SENSITIVITY;
                    // Subtract delta_y: mouse up (positive screen delta) = look up (positive pitch)
                    player_pitch -= mouse.delta_y as f32 * MOUSE_SENSITIVITY;

                    // Clamp pitch to prevent camera flipping (roughly -85 to +85 degrees)
                    player_pitch = player_pitch.clamp(-1.48, 1.48);

                    // Reset mouse deltas after reading (important!)
                    game::input::reset_mouse_deltas();

                    // Create input from keyboard state
                    // WASD = movement, Mouse = camera
                    input_sequence += 1;
                    let input = protocol::packets::ClientInput {
                        player_id: id,
                        sequence: input_sequence,
                        forward: if key_state.w { 1 } else if key_state.s { -1 } else { 0 },
                        strafe: if key_state.d { 1 } else if key_state.a { -1 } else { 0 }, // A/D for strafe
                        jump: key_state.space,
                        crouch: key_state.ctrl,
                        // Fire with left click OR shift key
                        fire: mouse.left_button || key_state.shift,
                        build: key_state.b || mouse.right_button, // Right click also builds
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

                    // Transition from BusPhase to InGame when bus finishes or all players have jumped
                    if current_state == GameState::BusPhase {
                        let all_jumped = world.players.iter().all(|p| p.phase != PlayerPhase::OnBus);
                        if !world.bus.active || all_jumped {
                            set_state(GameState::InGame);
                        }
                    }

                    // Check for victory condition (skip in benchmark mode)
                    if !BENCHMARK_MODE.load(core::sync::atomic::Ordering::Relaxed) {
                        if let Some(id) = world.check_victory() {
                            set_state(GameState::Victory { winner_id: Some(id) });
                        }
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
                    &glider_mesh, &tree_pine_mesh, &tree_oak_mesh, &rock_mesh,
                    &chest_mesh, &house_mesh, &storm_wall_mesh,
                    &projection, local_player_id, rotation,
                    frame_timer.fps(),
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

        frame_count = frame_count.wrapping_add(1);

        // End frame - handles vsync/frame timing with HLT for CPU idle
        // Uses VGA vertical retrace if available, otherwise timer-based sync
        let on_time = frame_timer.end_frame();

        // Log FPS periodically (FrameTimer tracks this internally)
        let current_fps = frame_timer.fps();
        if frame_count % 60 == 0 && current_fps > 0 {
            serial_println!("FPS: {} (state: {:?}) vsync:{} on_time:{}",
                current_fps, current_state, frame_timer.vsync_enabled(), on_time);
        }

        // Begin next frame timing
        frame_timer.begin_frame();
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
            drop(fb_guard);
                        graphics::gpu::present();
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
            drop(fb_guard);
                        graphics::gpu::present();
        }
    }
}

/// Render the lobby frame with 3D player preview (supports up to 4 team members)
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

    // Get current player customization for the local player
    let custom = PLAYER_CUSTOMIZATION.lock();
    let renderer_custom = custom.to_renderer();
    drop(custom);

    // Create player mesh from voxel model
    let player_mesh = voxel_models::create_player_model(&renderer_custom).to_mesh(0.15);

    // Calculate layout based on number of players
    let player_count = lobby.player_count();
    let spacing = 2.0; // Distance between players
    let total_width = (player_count as f32 - 1.0) * spacing;
    let start_x = -total_width / 2.0;

    // Adjust camera distance based on player count
    let camera_dist = 6.0 + (player_count as f32 - 1.0) * 1.5;
    let camera_height = 2.0 + (player_count as f32 - 1.0) * 0.3;

    // Create a simple platform mesh (size based on player count)
    let platform_width = 3.0 + (player_count as f32 - 1.0) * spacing;
    let platform_mesh = mesh::create_terrain_grid(platform_width, 2, Vec3::new(0.2, 0.3, 0.5));

    // Camera setup - fixed angle view (no orbit since rotation is fixed)
    let rotation = lobby.get_rotation();
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

    // Transform and bin the platform (centered)
    let platform_model = Mat4::from_translation(Vec3::new(0.0, -0.1, 0.0));
    bin_mesh(&platform_mesh, &platform_model, &view, projection, fb_width as f32, fb_height as f32);

    // Transform and bin each player model in the party
    for i in 0..player_count {
        let player_x = start_x + i as f32 * spacing;
        let player_model = Mat4::from_translation(Vec3::new(player_x, 0.0, 0.0));
        bin_mesh(&player_mesh, &player_model, &view, projection, fb_width as f32, fb_height as f32);
    }

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
    glider_mesh: &mesh::Mesh,
    tree_pine_mesh: &mesh::Mesh,
    tree_oak_mesh: &mesh::Mesh,
    rock_mesh: &mesh::Mesh,
    chest_mesh: &mesh::Mesh,
    house_mesh: &mesh::Mesh,
    storm_wall_mesh: &mesh::Mesh,
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
    render_ctx.clear(rgb(50, 70, 100)); // Sky blue background
    render_ctx.clear_zbuffer();

    // Get camera position from local player (or default orbit)
    let (camera_pos, camera_target, local_player_phase) = {
        let world = game::world::GAME_WORLD.lock();
        if let (Some(w), Some(id)) = (world.as_ref(), local_player_id) {
            if let Some(player) = w.get_player(id) {
                // Camera distance based on phase
                let cam_dist = match player.phase {
                    PlayerPhase::OnBus => 15.0,
                    PlayerPhase::Freefall | PlayerPhase::Gliding => 10.0,
                    _ => 5.0,
                };
                let cam_height = match player.phase {
                    PlayerPhase::OnBus => 5.0,
                    PlayerPhase::Freefall | PlayerPhase::Gliding => 4.0,
                    _ => 3.0,
                };
                // Third-person camera behind and above player
                // Incorporates both yaw (horizontal) and pitch (vertical) for proper look
                let cam_offset = Vec3::new(
                    -libm::sinf(player.yaw) * libm::cosf(player.pitch) * cam_dist,
                    cam_height + libm::sinf(player.pitch) * cam_dist * 0.5,
                    -libm::cosf(player.yaw) * libm::cosf(player.pitch) * cam_dist,
                );
                let pos = player.position + cam_offset;

                // Look target: where the player is aiming (uses pitch for up/down look)
                let look_dist = 10.0;
                let target = player.position + Vec3::new(
                    libm::sinf(player.yaw) * libm::cosf(player.pitch) * look_dist,
                    1.5 + libm::sinf(player.pitch) * look_dist, // Eye height + pitch
                    libm::cosf(player.yaw) * libm::cosf(player.pitch) * look_dist,
                );
                (pos, target, Some(player.phase))
            } else {
                let dist = 20.0;
                (Vec3::new(libm::sinf(rotation) * dist, 10.0, libm::cosf(rotation) * dist), Vec3::ZERO, None)
            }
        } else {
            let dist = 20.0;
            (Vec3::new(libm::sinf(rotation) * dist, 10.0, libm::cosf(rotation) * dist), Vec3::ZERO, None)
        }
    };
    let view = look_at(camera_pos, camera_target, Vec3::Y);

    // Check GPU batch availability ONCE at frame start (lock-free atomic read)
    let use_gpu_batch = GPU_BATCH_AVAILABLE.load(Ordering::Acquire);

    if use_gpu_batch {
        // === GPU RENDERING PATH ===
        // Hardware-accelerated rasterization via SVGA3D

        // Begin GPU batch (clears GPU buffers)
        gpu_batch::begin_batch();

        // Create culling context for frustum + distance culling
        let cull_ctx = CullContext::new(&view, projection, camera_pos)
            .with_distances(0.5, 500.0);

        // Transform and batch terrain
        let terrain_model = Mat4::from_translation(Vec3::new(0.0, 0.0, 0.0));
        bin_mesh_gpu(terrain, &terrain_model, &view, projection, fb_width as f32, fb_height as f32);

        // Batch game world entities with frustum culling
        {
            let world = game::world::GAME_WORLD.lock();
            if let Some(w) = world.as_ref() {
                // Render battle bus if active and visible
                if w.bus.active && cull_ctx.should_render(w.bus.position, 10.0) {
                    let bus_model = Mat4::from_translation(w.bus.position);
                    bin_mesh_gpu(bus_mesh, &bus_model, &view, projection, fb_width as f32, fb_height as f32);
                }

                // Render map buildings with frustum culling
                for i in 0..w.map.building_count {
                    if let Some(building) = &w.map.buildings[i] {
                        if !cull_ctx.should_render(building.position, 15.0) {
                            continue;
                        }
                        let model = Mat4::from_translation(building.position)
                            * Mat4::from_rotation_y(building.rotation)
                            * Mat4::from_scale(Vec3::splat(1.5));
                        bin_mesh_gpu(house_mesh, &model, &view, projection, fb_width as f32, fb_height as f32);
                    }
                }

                // Render vegetation with frustum culling
                for i in 0..w.map.vegetation_count {
                    if let Some(veg) = &w.map.vegetation[i] {
                        if !cull_ctx.should_render(veg.position, 5.0 * veg.scale) {
                            continue;
                        }

                        let model = Mat4::from_translation(veg.position)
                            * Mat4::from_scale(Vec3::splat(veg.scale));

                        match veg.veg_type {
                            game::map::VegetationType::TreePine => {
                                bin_mesh_gpu(tree_pine_mesh, &model, &view, projection, fb_width as f32, fb_height as f32);
                            }
                            game::map::VegetationType::TreeOak | game::map::VegetationType::TreeBirch => {
                                bin_mesh_gpu(tree_oak_mesh, &model, &view, projection, fb_width as f32, fb_height as f32);
                            }
                            game::map::VegetationType::Rock => {
                                bin_mesh_gpu(rock_mesh, &model, &view, projection, fb_width as f32, fb_height as f32);
                            }
                            game::map::VegetationType::Bush => {
                                let bush_model = model * Mat4::from_scale(Vec3::splat(0.5));
                                bin_mesh_gpu(tree_oak_mesh, &bush_model, &view, projection, fb_width as f32, fb_height as f32);
                            }
                        }
                    }
                }

                // Render loot drops with culling
                for drop in w.loot.get_active_drops() {
                    if !cull_ctx.should_render(drop.position, 2.0) {
                        continue;
                    }
                    let model = Mat4::from_translation(drop.position)
                        * Mat4::from_rotation_y(rotation * 2.0);
                    bin_mesh_gpu(chest_mesh, &model, &view, projection, fb_width as f32, fb_height as f32);
                }

                // Render all players (always render, they're important)
                for player in &w.players {
                    if !player.is_alive() || player.phase == PlayerPhase::OnBus {
                        continue;
                    }

                    let model = Mat4::from_translation(player.position)
                        * Mat4::from_rotation_y(player.yaw);
                    bin_mesh_gpu(player_mesh, &model, &view, projection, fb_width as f32, fb_height as f32);

                    if player.phase == PlayerPhase::Gliding {
                        let glider_offset = Vec3::new(0.0, 2.5, 0.0);
                        let glider_model = Mat4::from_translation(player.position + glider_offset)
                            * Mat4::from_rotation_y(player.yaw);
                        bin_mesh_gpu(glider_mesh, &glider_model, &view, projection, fb_width as f32, fb_height as f32);
                    }
                }

                // Render player-built buildings with culling
                for building in &w.buildings {
                    if !cull_ctx.should_render(building.position, 5.0) {
                        continue;
                    }
                    let model = Mat4::from_translation(building.position)
                        * Mat4::from_rotation_y(building.rotation);
                    bin_mesh_gpu(wall_mesh, &model, &view, projection, fb_width as f32, fb_height as f32);
                }

                // Render 3D storm wall (always render, important visual)
                let storm_model = Mat4::from_translation(Vec3::new(w.storm.center.x, 0.0, w.storm.center.z))
                    * Mat4::from_scale(Vec3::new(w.storm.radius, 1.0, w.storm.radius));
                bin_mesh_gpu(storm_wall_mesh, &storm_model, &view, projection, fb_width as f32, fb_height as f32);
            }
        }

        // End GPU batch (flushes remaining triangles and presents)
        gpu_batch::end_batch();

        // Drop render context (not used in GPU path, but acquired for API consistency)
        drop(render_ctx);

    } else {
        // === SOFTWARE RENDERING PATH ===
        // Parallel tile-based software rasterization (4 cores)

        // 1. Clear lock-free bins and reset triangle buffer
        tiles::clear_lockfree_bins();
        tiles::reset_triangle_buffer();

        // 2. Create culling context for frustum + distance culling
        let cull_ctx = CullContext::new(&view, projection, camera_pos)
            .with_distances(0.5, 500.0); // Near 0.5, Far 300 units

        // 3. Transform and bin terrain (always render, but reduced complexity)
        let terrain_model = Mat4::from_translation(Vec3::new(0.0, 0.0, 0.0));
        bin_mesh(terrain, &terrain_model, &view, projection, fb_width as f32, fb_height as f32);

        // 4. Render game world entities with frustum culling
        {
            let world = game::world::GAME_WORLD.lock();
            if let Some(w) = world.as_ref() {
                // Render battle bus if active and visible
                if w.bus.active && cull_ctx.should_render(w.bus.position, 10.0) {
                    let bus_model = Mat4::from_translation(w.bus.position);
                    bin_mesh(bus_mesh, &bus_model, &view, projection, fb_width as f32, fb_height as f32);
                }

                // Render map buildings with frustum culling
                for i in 0..w.map.building_count {
                    if let Some(building) = &w.map.buildings[i] {
                        // Cull buildings outside view frustum
                        if !cull_ctx.should_render(building.position, 15.0) {
                            continue;
                        }
                        let model = Mat4::from_translation(building.position)
                            * Mat4::from_rotation_y(building.rotation)
                            * Mat4::from_scale(Vec3::splat(1.5));
                        bin_mesh(house_mesh, &model, &view, projection, fb_width as f32, fb_height as f32);
                    }
                }

                // Render vegetation with frustum + distance culling
                for i in 0..w.map.vegetation_count {
                    if let Some(veg) = &w.map.vegetation[i] {
                        // Combined frustum + distance culling (100m for vegetation)
                        if !cull_ctx.should_render(veg.position, 5.0 * veg.scale) {
                            continue;
                        }

                        let model = Mat4::from_translation(veg.position)
                            * Mat4::from_scale(Vec3::splat(veg.scale));

                        match veg.veg_type {
                            game::map::VegetationType::TreePine => {
                                bin_mesh(tree_pine_mesh, &model, &view, projection, fb_width as f32, fb_height as f32);
                            }
                            game::map::VegetationType::TreeOak | game::map::VegetationType::TreeBirch => {
                                bin_mesh(tree_oak_mesh, &model, &view, projection, fb_width as f32, fb_height as f32);
                            }
                            game::map::VegetationType::Rock => {
                                bin_mesh(rock_mesh, &model, &view, projection, fb_width as f32, fb_height as f32);
                            }
                            game::map::VegetationType::Bush => {
                                let bush_model = model * Mat4::from_scale(Vec3::splat(0.5));
                                bin_mesh(tree_oak_mesh, &bush_model, &view, projection, fb_width as f32, fb_height as f32);
                            }
                        }
                    }
                }

                // Render loot drops with culling
                for drop in w.loot.get_active_drops() {
                    if !cull_ctx.should_render(drop.position, 2.0) {
                        continue;
                    }
                    let model = Mat4::from_translation(drop.position)
                        * Mat4::from_rotation_y(rotation * 2.0);
                    bin_mesh(chest_mesh, &model, &view, projection, fb_width as f32, fb_height as f32);
                }

                // Render all players (always render, they're important)
                for player in &w.players {
                    if !player.is_alive() || player.phase == PlayerPhase::OnBus {
                        continue;
                    }

                    let model = Mat4::from_translation(player.position)
                        * Mat4::from_rotation_y(player.yaw);
                    bin_mesh(player_mesh, &model, &view, projection, fb_width as f32, fb_height as f32);

                    if player.phase == PlayerPhase::Gliding {
                        let glider_offset = Vec3::new(0.0, 2.5, 0.0);
                        let glider_model = Mat4::from_translation(player.position + glider_offset)
                            * Mat4::from_rotation_y(player.yaw);
                        bin_mesh(glider_mesh, &glider_model, &view, projection, fb_width as f32, fb_height as f32);
                    }
                }

                // Render player-built buildings with culling
                for building in &w.buildings {
                    if !cull_ctx.should_render(building.position, 5.0) {
                        continue;
                    }
                    let model = Mat4::from_translation(building.position)
                        * Mat4::from_rotation_y(building.rotation);
                    bin_mesh(wall_mesh, &model, &view, projection, fb_width as f32, fb_height as f32);
                }

                // Render 3D storm wall (always render, important visual)
                let storm_model = Mat4::from_translation(Vec3::new(w.storm.center.x, 0.0, w.storm.center.z))
                    * Mat4::from_scale(Vec3::new(w.storm.radius, 1.0, w.storm.radius));
                bin_mesh(storm_wall_mesh, &storm_model, &view, projection, fb_width as f32, fb_height as f32);
            }
        }

        // 4. Reset tile work queue
        tiles::reset();

        // 5. Signal worker cores (1-3) to start rendering
        smp::scheduler::start_render();

        // 6. Core 0 also helps rasterize tiles
        render_worker(0);

        // 7. Wait for all cores (0-3) to finish at the barrier
        smp::sync::RENDER_BARRIER.wait();

        // 8. Signal render complete (allows worker cores to wait for next frame)
        smp::scheduler::end_render();

        // Drop render context before drawing 2D UI
        drop(render_ctx);
    }

    // === 2D UI RENDERING ===

    // Draw FPS counter
    font::draw_fps(current_fps, fb_width);

    // Draw crosshair at center of screen
    {
        let fb_guard = graphics::framebuffer::FRAMEBUFFER.lock();
        if let Some(fb) = fb_guard.as_ref() {
            graphics::ui::panel::draw_crosshair_raw(fb, fb_width, fb_height, 0xFFFFFFFF);
        }
    }

    // Draw storm indicator if player is in storm
    {
        let world_guard = game::world::GAME_WORLD.lock();
        if let Some(world) = world_guard.as_ref() {
            if let Some(id) = local_player_id {
                if let Some(player) = world.get_player(id) {
                    if !world.storm.contains(player.position) {
                        // Draw storm warning overlay
                        draw_storm_overlay(fb_width, fb_height);
                    }
                }
            }
        }
    }

    // Draw game HUD (health, shield, materials, alive count)
    {
        let world_guard = game::world::GAME_WORLD.lock();
        if let Some(world) = world_guard.as_ref() {
            let (health, shield, materials, inventory) = if let Some(id) = local_player_id {
                if let Some(player) = world.get_player(id) {
                    (player.health, player.shield, player.inventory.materials.clone(), Some(&player.inventory))
                } else {
                    (100, 0, game::inventory::Materials::default(), None)
                }
            } else {
                (100, 0, game::inventory::Materials::default(), None)
            };
            let alive = world.players.iter().filter(|p| p.health > 0).count();
            let total = world.players.len();

            // Draw main HUD
            font::draw_hud(health, shield as u32, alive, total, fb_width, fb_height);

            // Draw inventory hotbar
            if let Some(inv) = inventory {
                draw_inventory_hotbar(inv, fb_width, fb_height);
            }

            // Draw materials count
            draw_materials_hud(&materials, fb_width, fb_height);

            // Draw storm timer
            draw_storm_timer(&world.storm, fb_width, fb_height);

            // Draw minimap with storm circle
            draw_minimap(local_player_id, world, fb_width, fb_height);
        }
    }

    // End frame and present to display (uses GPU acceleration if available)
    graphics::gpu_render::end_frame();
}

/// Draw storm overlay effect when player is in storm
fn draw_storm_overlay(fb_width: usize, fb_height: usize) {
    if let Some(fb_guard) = graphics::framebuffer::FRAMEBUFFER.try_lock() {
        if let Some(fb) = fb_guard.as_ref() {
            // Draw purple tint on edges of screen
            let purple = rgb(128, 0, 128);
            let edge_width = 30;

            // Top edge
            for y in 0..edge_width {
                let alpha = (edge_width - y) as f32 / edge_width as f32;
                for x in 0..fb_width {
                    let idx = y * fb_width + x;
                    let existing = fb.pixel_at(idx);
                    let blended = blend_color(existing, purple, alpha * 0.5);
                    fb.set_pixel_at(idx, blended);
                }
            }

            // Bottom edge
            for y in (fb_height - edge_width)..fb_height {
                let alpha = (y - (fb_height - edge_width)) as f32 / edge_width as f32;
                for x in 0..fb_width {
                    let idx = y * fb_width + x;
                    let existing = fb.pixel_at(idx);
                    let blended = blend_color(existing, purple, alpha * 0.5);
                    fb.set_pixel_at(idx, blended);
                }
            }
        }
    }
}

/// Blend two colors
fn blend_color(base: u32, overlay: u32, alpha: f32) -> u32 {
    let br = ((base >> 16) & 0xFF) as f32;
    let bg = ((base >> 8) & 0xFF) as f32;
    let bb = (base & 0xFF) as f32;

    let or = ((overlay >> 16) & 0xFF) as f32;
    let og = ((overlay >> 8) & 0xFF) as f32;
    let ob = (overlay & 0xFF) as f32;

    let r = (br * (1.0 - alpha) + or * alpha) as u32;
    let g = (bg * (1.0 - alpha) + og * alpha) as u32;
    let b = (bb * (1.0 - alpha) + ob * alpha) as u32;

    (r << 16) | (g << 8) | b
}

/// Draw inventory hotbar
fn draw_inventory_hotbar(inv: &game::inventory::Inventory, fb_width: usize, fb_height: usize) {
    if let Some(fb_guard) = graphics::framebuffer::FRAMEBUFFER.try_lock() {
        if let Some(fb) = fb_guard.as_ref() {
            let slot_size = 50;
            let slot_spacing = 5;
            let total_width = 6 * slot_size + 5 * slot_spacing; // 6 slots (pickaxe + 5 weapons)
            let start_x = (fb_width - total_width) / 2;
            let start_y = fb_height - slot_size - 80; // Above health bar

            // Draw pickaxe slot
            let is_selected = inv.pickaxe_selected;
            let border_color = if is_selected { rgb(255, 200, 0) } else { rgb(100, 100, 100) };
            let bg_color = rgb(50, 50, 50);

            draw_slot(fb, start_x, start_y, slot_size, bg_color, border_color);
            font::draw_string_raw(fb, start_x + 15, start_y + 20, "P", rgb(200, 200, 200), 1);

            // Draw weapon slots
            for i in 0..5 {
                let x = start_x + (i + 1) * (slot_size + slot_spacing);
                let is_selected = !inv.pickaxe_selected && inv.selected_slot == i;
                let border_color = if is_selected { rgb(255, 200, 0) } else { rgb(100, 100, 100) };

                draw_slot(fb, x, start_y, slot_size, bg_color, border_color);

                // Draw weapon info if slot is filled
                if let Some(weapon) = &inv.slots[i] {
                    let rarity_color = match weapon.rarity {
                        game::weapon::Rarity::Common => rgb(150, 150, 150),
                        game::weapon::Rarity::Uncommon => rgb(50, 200, 50),
                        game::weapon::Rarity::Rare => rgb(50, 100, 255),
                        game::weapon::Rarity::Epic => rgb(200, 50, 200),
                        game::weapon::Rarity::Legendary => rgb(255, 180, 0),
                    };

                    // Draw rarity indicator bar at bottom of slot
                    for dy in (slot_size - 5)..slot_size {
                        for dx in 2..(slot_size - 2) {
                            fb.set_pixel(x + dx, start_y + dy, rarity_color);
                        }
                    }

                    // Draw weapon type letter
                    let letter = match weapon.weapon_type {
                        game::weapon::WeaponType::Pistol => "Pi",
                        game::weapon::WeaponType::Shotgun => "SG",
                        game::weapon::WeaponType::AssaultRifle => "AR",
                        game::weapon::WeaponType::Smg => "SM",
                        game::weapon::WeaponType::Sniper => "SR",
                        game::weapon::WeaponType::Pickaxe => "PX",
                    };
                    font::draw_string_raw(fb, x + 10, start_y + 15, letter, rgb(255, 255, 255), 1);

                    // Draw ammo count
                    let ammo_str = alloc::format!("{}", weapon.ammo);
                    font::draw_string_raw(fb, x + 15, start_y + 32, &ammo_str, rgb(200, 200, 200), 1);
                }

                // Draw slot number
                let num_str = alloc::format!("{}", i + 2);
                font::draw_string_raw(fb, x + 3, start_y + 3, &num_str, rgb(150, 150, 150), 1);
            }
        }
    }
}

/// Draw a UI slot/box
fn draw_slot(fb: &graphics::framebuffer::Framebuffer, x: usize, y: usize, size: usize, bg: u32, border: u32) {
    // Background
    for dy in 0..size {
        for dx in 0..size {
            fb.set_pixel(x + dx, y + dy, bg);
        }
    }
    // Border
    for dx in 0..size {
        fb.set_pixel(x + dx, y, border);
        fb.set_pixel(x + dx, y + size - 1, border);
    }
    for dy in 0..size {
        fb.set_pixel(x, y + dy, border);
        fb.set_pixel(x + size - 1, y + dy, border);
    }
}

/// Draw materials HUD
fn draw_materials_hud(materials: &game::inventory::Materials, fb_width: usize, fb_height: usize) {
    if let Some(fb_guard) = graphics::framebuffer::FRAMEBUFFER.try_lock() {
        if let Some(fb) = fb_guard.as_ref() {
            let x = fb_width - 150;
            let y = fb_height - 100;

            // Wood
            let wood_str = alloc::format!("W: {}", materials.wood);
            font::draw_string_raw(fb, x, y, &wood_str, rgb(180, 120, 60), 1);

            // Brick
            let brick_str = alloc::format!("B: {}", materials.brick);
            font::draw_string_raw(fb, x, y + 20, &brick_str, rgb(180, 80, 80), 1);

            // Metal
            let metal_str = alloc::format!("M: {}", materials.metal);
            font::draw_string_raw(fb, x, y + 40, &metal_str, rgb(150, 150, 170), 1);
        }
    }
}

/// Draw storm timer
fn draw_storm_timer(storm: &game::storm::Storm, fb_width: usize, _fb_height: usize) {
    if let Some(fb_guard) = graphics::framebuffer::FRAMEBUFFER.try_lock() {
        if let Some(fb) = fb_guard.as_ref() {
            let phase_str = if storm.shrinking {
                alloc::format!("STORM CLOSING: {:.0}s", storm.timer)
            } else {
                alloc::format!("SAFE ZONE: {:.0}s", storm.timer)
            };

            let x = (fb_width - phase_str.len() * 8) / 2;
            let color = if storm.shrinking { rgb(200, 50, 200) } else { rgb(255, 255, 255) };
            font::draw_string_raw(fb, x, 50, &phase_str, color, 1);
        }
    }
}

/// Draw minimap
fn draw_minimap(local_player_id: Option<u8>, world: &game::world::GameWorld, fb_width: usize, _fb_height: usize) {
    if let Some(fb_guard) = graphics::framebuffer::FRAMEBUFFER.try_lock() {
        if let Some(fb) = fb_guard.as_ref() {
            let map_size = 150;
            let map_x = fb_width - map_size - 20;
            let map_y = 20;

            // Draw map background
            for dy in 0..map_size {
                for dx in 0..map_size {
                    fb.set_pixel(map_x + dx, map_y + dy, rgb(20, 40, 20));
                }
            }

            // Draw map border
            for dx in 0..map_size {
                fb.set_pixel(map_x + dx, map_y, rgb(100, 100, 100));
                fb.set_pixel(map_x + dx, map_y + map_size - 1, rgb(100, 100, 100));
            }
            for dy in 0..map_size {
                fb.set_pixel(map_x, map_y + dy, rgb(100, 100, 100));
                fb.set_pixel(map_x + map_size - 1, map_y + dy, rgb(100, 100, 100));
            }

            // Scale: map is 2000 units, minimap is 150 pixels
            let scale = map_size as f32 / 2000.0;
            let offset = 1000.0; // Center offset

            // Draw storm circle
            let storm_cx = ((world.storm.center.x + offset) * scale) as i32;
            let storm_cz = ((world.storm.center.z + offset) * scale) as i32;
            let storm_r = (world.storm.radius * scale) as i32;

            // Draw circle outline (simplified)
            for angle in 0..64 {
                let a = (angle as f32 / 64.0) * core::f32::consts::TAU;
                let px = storm_cx + (libm::cosf(a) * storm_r as f32) as i32;
                let py = storm_cz + (libm::sinf(a) * storm_r as f32) as i32;
                if px >= 0 && px < map_size as i32 && py >= 0 && py < map_size as i32 {
                    fb.set_pixel(map_x + px as usize, map_y + py as usize, rgb(255, 255, 255));
                }
            }

            // Draw player positions
            for player in &world.players {
                if !player.is_alive() {
                    continue;
                }
                let px = ((player.position.x + offset) * scale) as usize;
                let py = ((player.position.z + offset) * scale) as usize;

                if px < map_size && py < map_size {
                    let color = if Some(player.id) == local_player_id {
                        rgb(0, 255, 0) // Green for local player
                    } else {
                        rgb(255, 0, 0) // Red for others
                    };

                    // Draw 3x3 dot
                    for dx in 0..3 {
                        for dy in 0..3 {
                            if px + dx < map_size && py + dy < map_size {
                                fb.set_pixel(map_x + px + dx, map_y + py + dy, color);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Transform mesh triangles, create ScreenTriangles, and bin them to tiles
/// Uses GPU batch rendering when available, falls back to software rasterization
/// Returns the number of triangles successfully processed
fn bin_mesh(
    mesh: &mesh::Mesh,
    model: &Mat4,
    view: &Mat4,
    projection: &Mat4,
    fb_width: f32,
    fb_height: f32,
) -> usize {
    let mut binned = 0;

    // Use the simple software path - GPU batch will be used when SVGA3D is available
    // The is_enabled() check is done once at startup, not per-triangle
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

/// Bin mesh triangles directly to GPU batch (GPU rendering path)
/// Transforms vertices and adds them to the GPU batch for hardware rasterization
/// This is the GPU-accelerated alternative to bin_mesh() for software rendering
fn bin_mesh_gpu(
    mesh: &mesh::Mesh,
    model: &Mat4,
    view: &Mat4,
    projection: &Mat4,
    fb_width: f32,
    fb_height: f32,
) -> usize {
    use graphics::pipeline::transform_triangle;

    let mut added = 0;

    for i in 0..mesh.triangle_count() {
        if let Some((v0, v1, v2)) = mesh.get_triangle(i) {
            // Transform and perform culling (same as software path)
            if let Some((tv0, tv1, tv2)) = transform_triangle(
                v0,
                v1,
                v2,
                model,
                view,
                projection,
                fb_width,
                fb_height,
            ) {
                // Add transformed triangle to GPU batch
                let success = gpu_batch::add_screen_triangle(
                    tv0.position.x, tv0.position.y, tv0.position.z,
                    tv0.color.x, tv0.color.y, tv0.color.z,
                    tv1.position.x, tv1.position.y, tv1.position.z,
                    tv1.color.x, tv1.color.y, tv1.color.z,
                    tv2.position.x, tv2.position.y, tv2.position.z,
                    tv2.color.x, tv2.color.y, tv2.color.z,
                );

                if success {
                    added += 1;
                    // Flush batch if full
                    if gpu_batch::needs_flush() {
                        gpu_batch::flush_batch();
                    }
                }
            }
        }
    }

    added
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

/// Create a 3D terrain mesh with proper hills and valleys
/// Uses Perlin-like noise for natural-looking terrain
fn create_3d_terrain(size: f32, subdivisions: usize) -> mesh::Mesh {
    use renderer::vertex::Vertex;
    use glam::Vec2;

    let mut terrain_mesh = mesh::Mesh::new();

    let half = size / 2.0;
    let step = size / subdivisions as f32;

    // Create vertices with height variation
    for z in 0..=subdivisions {
        for x in 0..=subdivisions {
            let fx = x as f32 * step - half;
            let fz = z as f32 * step - half;

            // Multi-octave noise for more natural terrain
            // Large hills
            let h1 = libm::sinf(fx * 0.01) * libm::cosf(fz * 0.01) * 15.0;
            // Medium bumps
            let h2 = libm::sinf(fx * 0.05) * libm::sinf(fz * 0.05) * 5.0;
            // Small details
            let h3 = libm::sinf(fx * 0.15 + fz * 0.1) * 2.0;
            // Add some valleys
            let h4 = libm::cosf((fx + fz) * 0.02) * 8.0;

            let height = h1 + h2 + h3 + h4;

            // Color variation based on height (grass -> dirt -> rock)
            let color = if height > 10.0 {
                // Rocky peaks - gray
                Vec3::new(0.5, 0.5, 0.45)
            } else if height > 5.0 {
                // High grass - darker green
                Vec3::new(0.2, 0.5, 0.2)
            } else if height > -5.0 {
                // Normal grass - bright green
                Vec3::new(0.3, 0.65, 0.25)
            } else {
                // Low areas - brownish
                Vec3::new(0.4, 0.35, 0.2)
            };

            terrain_mesh.vertices.push(Vertex::new(
                Vec3::new(fx, height, fz),
                Vec3::Y, // Will be recalculated
                color,
                Vec2::new(x as f32 / subdivisions as f32, z as f32 / subdivisions as f32),
            ));
        }
    }

    // Create indices for triangles
    let row_size = subdivisions + 1;
    for z in 0..subdivisions {
        for x in 0..subdivisions {
            let tl = (z * row_size + x) as u32;
            let tr = tl + 1;
            let bl = tl + row_size as u32;
            let br = bl + 1;

            // Two triangles per quad
            terrain_mesh.indices.extend([tl, bl, tr]);
            terrain_mesh.indices.extend([tr, bl, br]);
        }
    }

    // Recalculate normals for proper lighting
    let mut normals = alloc::vec![Vec3::ZERO; terrain_mesh.vertices.len()];

    for i in (0..terrain_mesh.indices.len()).step_by(3) {
        let i0 = terrain_mesh.indices[i] as usize;
        let i1 = terrain_mesh.indices[i + 1] as usize;
        let i2 = terrain_mesh.indices[i + 2] as usize;

        let v0 = terrain_mesh.vertices[i0].position;
        let v1 = terrain_mesh.vertices[i1].position;
        let v2 = terrain_mesh.vertices[i2].position;

        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let face_normal = edge1.cross(edge2);

        normals[i0] += face_normal;
        normals[i1] += face_normal;
        normals[i2] += face_normal;
    }

    // Normalize and apply
    for (i, normal) in normals.iter().enumerate() {
        let length = libm::sqrtf(normal.x * normal.x + normal.y * normal.y + normal.z * normal.z);
        let n = if length > 0.0001 {
            Vec3::new(normal.x / length, normal.y / length, normal.z / length)
        } else {
            Vec3::Y
        };
        terrain_mesh.vertices[i].normal = n;
    }

    terrain_mesh
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
