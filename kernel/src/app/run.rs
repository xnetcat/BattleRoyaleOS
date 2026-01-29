//! Game Client Entry Point
//!
//! Main entry point for the game client. Called from kernel after hardware init.

extern crate alloc;

use core::sync::atomic::{AtomicBool, Ordering};
use glam::{Mat4, Vec3};
use renderer::mesh;
use crate::game::input::{self, KeyState};
use crate::game::state::{GameState, PlayerPhase, get_state, set_state, MenuAction};
use crate::game::world::GAME_WORLD;
use crate::graphics::framebuffer::FRAMEBUFFER;
use crate::graphics::gpu;
use crate::graphics::cursor;
use crate::graphics::pipeline::{look_at, perspective};
use crate::graphics::rasterizer::RenderContext;
use crate::graphics::vsync::FrameTimer;
use crate::net;
use crate::ui;
use crate::{halt_loop, read_tsc};
use crate::serial_println;

use super::input::get_menu_action;
use super::render::{
    render_game_frame, render_lobby_frame, render_menu_frame, render_test_map_frame,
    set_gpu_batch_available, GPU_BATCH_AVAILABLE,
};
use super::terrain::create_3d_terrain;

/// Global benchmark mode flag
static BENCHMARK_MODE: AtomicBool = AtomicBool::new(false);

/// Global test mode flag
static TEST_MODE: AtomicBool = AtomicBool::new(false);

/// Set benchmark mode
pub fn set_benchmark_mode(enabled: bool) {
    BENCHMARK_MODE.store(enabled, Ordering::SeqCst);
}

/// Set test mode
pub fn set_test_mode(enabled: bool) {
    TEST_MODE.store(enabled, Ordering::SeqCst);
}

/// Main game loop entry point (runs on Core 0)
/// Called from kernel after hardware initialization is complete.
pub fn run(fb_width: usize, fb_height: usize, gpu_batch_available: bool) -> ! {
    set_gpu_batch_available(gpu_batch_available);

    let mut frame_count = 0u32;
    let mut rotation = 0.0f32;

    // Frame timer with vsync support (replaces manual FPS tracking and busy-waiting)
    // Uses HLT instruction for CPU idle when waiting, reducing power consumption
    let mut frame_timer = FrameTimer::new();

    // TSC frequency for benchmark reporting (assume ~2GHz for QEMU)
    let tsc_per_second: u64 = 2_000_000_000;

    // Create reusable meshes for game entities using VOXEL MODELS
    // Terrain: 3D heightmap with proper hills
    let terrain = create_3d_terrain(2000.0, 15); // 15 subdivisions for 60 FPS target

    // Player mesh from detailed voxel model (use default customization for now)
    let default_custom = renderer::voxel::CharacterCustomization::default();
    let player_mesh = renderer::voxel_models::create_player_model(&default_custom).to_mesh(0.15);

    // Building pieces from voxel models
    let wall_mesh = renderer::voxel_models::create_wall_wood().to_mesh(0.25);

    // Battle bus from voxel model (includes balloon)
    let bus_mesh = renderer::voxel_models::create_battle_bus().to_mesh(0.30);

    // Additional meshes for complete game rendering
    let glider_mesh = renderer::voxel_models::create_glider_model(0).to_mesh(0.15);
    let tree_pine_mesh = renderer::voxel_models::create_pine_tree().to_mesh(0.5);
    let tree_oak_mesh = renderer::voxel_models::create_oak_tree().to_mesh(0.5);
    let rock_mesh = renderer::voxel_models::create_rock(0).to_mesh(0.4);
    let chest_mesh = renderer::voxel_models::create_chest().to_mesh(0.15);
    let house_mesh = renderer::map_mesh::create_house_mesh_simple(Vec3::new(0.7, 0.6, 0.5));
    let storm_wall_mesh = mesh::create_storm_wall(24, 200.0); // 24 segments for performance

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
    let mut prev_key_state = KeyState::default();

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
    let benchmark = BENCHMARK_MODE.load(Ordering::SeqCst);
    let test_mode = TEST_MODE.load(Ordering::SeqCst);
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
            if let Some(world) = GAME_WORLD.lock().as_mut() {
                // Add a player if none exists
                if world.players.is_empty() {
                    use smoltcp::wire::Ipv4Address;
                    let player_name = if test_mode { "TestPlayer" } else { "Benchmark" };
                    let player = crate::game::player::Player::new(0, player_name, Ipv4Address::new(127, 0, 0, 1), 5000);
                    world.players.push(player);
                    world.local_player_id = Some(0);
                    local_player_id = Some(0);
                }

                // Set player to grounded (not on bus)
                if let Some(p) = world.players.get_mut(0) {
                    p.phase = PlayerPhase::Grounded;
                    p.position = Vec3::new(50.0, 5.0, 50.0);

                    // Test mode: give player all weapons
                    if test_mode {
                        use crate::game::weapon::{WeaponType, Weapon, Rarity};
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
                    spawn_test_items(world);
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
        input::poll_keyboard();
        let key_state = input::KEY_STATE.lock().clone();

        // Sync local player ID from world if not set
        if local_player_id.is_none() {
            if let Some(world) = GAME_WORLD.lock().as_ref() {
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
                handle_party_lobby(
                    &mut fortnite_lobby,
                    &key_state,
                    &prev_key_state,
                    menu_action,
                    &mut countdown_timer,
                    &mut local_player_id,
                    fb_width,
                    fb_height,
                    &projection,
                );
            }

            GameState::ServerSelect => {
                // Update server select screen
                if let Some(new_state) = server_select_screen.update(menu_action) {
                    set_state(new_state);
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
                set_state(GameState::LobbyCountdown { remaining_secs: 10 });
            }

            GameState::LobbyCountdown { remaining_secs } => {
                countdown_timer -= 1.0 / 60.0;

                if countdown_timer <= 0.0 {
                    set_state(GameState::BusPhase);
                    // Spawn bots for single-player mode
                    if let Some(world) = GAME_WORLD.lock().as_mut() {
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
                handle_gameplay(
                    &key_state,
                    &prev_key_state,
                    menu_action,
                    &mut local_player_id,
                    &mut player_yaw,
                    &mut player_pitch,
                    &mut input_sequence,
                    current_state,
                    fb_width,
                    fb_height,
                    &terrain,
                    &player_mesh,
                    &wall_mesh,
                    &bus_mesh,
                    &glider_mesh,
                    &tree_pine_mesh,
                    &tree_oak_mesh,
                    &rock_mesh,
                    &chest_mesh,
                    &house_mesh,
                    &storm_wall_mesh,
                    &projection,
                    rotation,
                    &frame_timer,
                    frame_count,
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
        let on_time = frame_timer.end_frame();

        // Log FPS periodically
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

/// Handle party lobby state
fn handle_party_lobby(
    fortnite_lobby: &mut ui::fortnite_lobby::FortniteLobby,
    key_state: &KeyState,
    prev_key_state: &KeyState,
    menu_action: MenuAction,
    countdown_timer: &mut f32,
    local_player_id: &mut Option<u8>,
    fb_width: usize,
    fb_height: usize,
    projection: &Mat4,
) {
    // Check for 'T' key to enter test map
    if key_state.t && !prev_key_state.t {
        set_state(GameState::TestMap);
        return;
    }

    // Update Fortnite-style party lobby
    fortnite_lobby.tick();
    if let Some(new_state) = fortnite_lobby.update(menu_action) {
        set_state(new_state);

        // If starting matchmaking, prepare for game
        if matches!(new_state, GameState::Matchmaking { .. }) {
            // In offline mode, skip matchmaking and go straight to countdown
            *countdown_timer = 5.0;
            crate::game::world::init(true);

            // Add local player
            *local_player_id = {
                let mut world = GAME_WORLD.lock();
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
    render_lobby_frame(fb_width, fb_height, fortnite_lobby, projection);

    // Then draw lobby UI overlay on top (skip background since 3D is rendered)
    let ctx = match RenderContext::acquire() {
        Some(ctx) => ctx,
        None => return,
    };
    fortnite_lobby.draw_ui_only(&ctx, fb_width, fb_height, true);
    drop(ctx);

    // Draw cursor and present
    {
        let fb_guard = FRAMEBUFFER.lock();
        if let Some(fb) = fb_guard.as_ref() {
            let mouse = input::get_mouse_state();
            cursor::draw_cursor(fb, mouse.x, mouse.y);
            drop(fb_guard);
            gpu::present();
        }
    }
}

/// Handle gameplay state (BusPhase and InGame)
fn handle_gameplay(
    key_state: &KeyState,
    prev_key_state: &KeyState,
    menu_action: MenuAction,
    local_player_id: &mut Option<u8>,
    player_yaw: &mut f32,
    player_pitch: &mut f32,
    input_sequence: &mut u32,
    current_state: GameState,
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
    rotation: f32,
    frame_timer: &FrameTimer,
    frame_count: u32,
) {
    // Check for escape to return to party lobby
    if menu_action == MenuAction::Back {
        set_state(GameState::PartyLobby);
        return;
    }

    // Get mouse state for camera control
    let mouse = input::get_mouse_state();

    // Apply keyboard and mouse input to local player
    if let Some(id) = *local_player_id {
        // Mouse look sensitivity (adjusted for smooth camera)
        const MOUSE_SENSITIVITY: f32 = 0.002;

        // Update camera rotation with mouse movement ONLY
        *player_yaw += mouse.delta_x as f32 * MOUSE_SENSITIVITY;
        *player_pitch -= mouse.delta_y as f32 * MOUSE_SENSITIVITY;

        // Clamp pitch to prevent camera flipping (roughly -85 to +85 degrees)
        *player_pitch = player_pitch.clamp(-1.48, 1.48);

        // Reset mouse deltas after reading (important!)
        input::reset_mouse_deltas();

        // Create input from keyboard state
        *input_sequence += 1;
        let input = protocol::packets::ClientInput {
            player_id: id,
            sequence: *input_sequence,
            forward: if key_state.w { 1 } else if key_state.s { -1 } else { 0 },
            strafe: if key_state.d { 1 } else if key_state.a { -1 } else { 0 },
            jump: key_state.space,
            crouch: key_state.ctrl,
            fire: mouse.left_button || key_state.shift,
            build: key_state.b || mouse.right_button,
            exit_bus: key_state.space,
            yaw: (player_yaw.to_degrees() * 100.0) as i16,
            pitch: (player_pitch.to_degrees() * 100.0) as i16,
        };

        // Apply input to game world
        if let Some(world) = GAME_WORLD.lock().as_mut() {
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
    input::reset_mouse_deltas();

    // Update game world physics and check for victory
    if let Some(world) = GAME_WORLD.lock().as_mut() {
        world.update(1.0 / 60.0);

        // Transition from BusPhase to InGame when bus finishes or all players have jumped
        if current_state == GameState::BusPhase {
            let all_jumped = world.players.iter().all(|p| p.phase != PlayerPhase::OnBus);
            if !world.bus.active || all_jumped {
                set_state(GameState::InGame);
            }
        }

        // Check for victory condition (skip in benchmark mode)
        if !BENCHMARK_MODE.load(Ordering::Relaxed) {
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
        terrain, player_mesh, wall_mesh, bus_mesh,
        glider_mesh, tree_pine_mesh, tree_oak_mesh, rock_mesh,
        chest_mesh, house_mesh, storm_wall_mesh,
        projection, *local_player_id, rotation,
        frame_timer.fps(),
    );
}

/// Spawn test items for test mode
fn spawn_test_items(world: &mut crate::game::world::GameWorld) {
    use crate::game::weapon::{WeaponType, Weapon, Rarity};
    use crate::game::loot::{LootItem, ChestTier};

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

    // Spawn bots for player to fight (so they can win by eliminating them)
    world.spawn_bots(5);
    serial_println!("TEST: Spawned 5 bots to fight - eliminate them to win!");
}

/// Network worker for network core
pub fn network_worker() {
    // Poll network stack with TSC-based timestamp
    let timestamp = (read_tsc() / 1_000_000) as i64; // Rough ms approximation
    net::stack::poll(timestamp);

    // Process incoming packets
    net::protocol::process_incoming();
}
