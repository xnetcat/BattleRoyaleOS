//! Game Rendering
//!
//! Handles all rendering: menus, lobby, test map, and game world.

extern crate alloc;

use core::sync::atomic::{AtomicBool, Ordering};
use glam::{Mat4, Vec3};
use renderer::mesh::Mesh;
use crate::game::input;
use crate::game::state::{PlayerPhase, PLAYER_CUSTOMIZATION};
use crate::game::world::GAME_WORLD;
use crate::graphics::culling::CullContext;
use crate::graphics::font;
use crate::graphics::framebuffer::{rgb, FRAMEBUFFER};
use crate::graphics::gpu;
use crate::graphics::gpu_batch;
use crate::graphics::gpu_render;
use crate::graphics::cursor;
use crate::graphics::pipeline::{look_at, transform_and_bin_fast, transform_triangle};
use crate::graphics::rasterizer::{rasterize_screen_triangle_simple, RenderContext};
use crate::graphics::tiles::{self, TILE_BINS_LOCKFREE, TILE_QUEUE};
use crate::graphics::ui::panel;
use crate::smp;
use crate::ui;

use super::hud::{
    draw_inventory_hotbar, draw_materials_hud, draw_minimap,
    draw_storm_overlay, draw_storm_timer, lerp_u8,
};

/// Global GPU batch enabled flag - checked once at init, used per-frame without locks
pub static GPU_BATCH_AVAILABLE: AtomicBool = AtomicBool::new(false);

/// Set GPU batch availability (called during init)
pub fn set_gpu_batch_available(available: bool) {
    GPU_BATCH_AVAILABLE.store(available, Ordering::Release);
}

/// Render a menu frame (2D UI only) with mouse cursor
pub fn render_menu_frame<F>(fb_width: usize, fb_height: usize, draw_fn: F)
where
    F: FnOnce(&RenderContext),
{
    // Acquire render context
    let render_ctx = match RenderContext::acquire() {
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
        let fb_guard = FRAMEBUFFER.lock();
        if let Some(fb) = fb_guard.as_ref() {
            // Draw mouse cursor on top of everything
            let mouse = input::get_mouse_state();
            cursor::draw_cursor(fb, mouse.x, mouse.y);
            drop(fb_guard);
            gpu::present();
        }
    }
}

/// Render the test map / model gallery
pub fn render_test_map_frame(
    fb_width: usize,
    fb_height: usize,
    test_map: &ui::test_map::TestMapScreen,
    projection: &Mat4,
) {
    use renderer::voxel_models;
    use renderer::voxel::CharacterCustomization;

    // Acquire render context
    let render_ctx = match RenderContext::acquire() {
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
    let ctx = match RenderContext::acquire() {
        Some(ctx) => ctx,
        None => return,
    };
    test_map.draw(&ctx, fb_width, fb_height);
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

/// Render the lobby frame with 3D player preview (supports up to 4 team members)
pub fn render_lobby_frame(
    fb_width: usize,
    fb_height: usize,
    lobby: &ui::fortnite_lobby::FortniteLobby,
    projection: &Mat4,
) {
    use renderer::voxel_models;
    use renderer::mesh;

    // Acquire render context
    let render_ctx = match RenderContext::acquire() {
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
pub fn draw_sunset_gradient(_ctx: &RenderContext, fb_width: usize, fb_height: usize) {
    let fb_guard = FRAMEBUFFER.lock();
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

/// Render a game frame (3D world + HUD)
pub fn render_game_frame(
    fb_width: usize,
    fb_height: usize,
    terrain: &Mesh,
    player_mesh: &Mesh,
    wall_mesh: &Mesh,
    bus_mesh: &Mesh,
    glider_mesh: &Mesh,
    tree_pine_mesh: &Mesh,
    tree_oak_mesh: &Mesh,
    rock_mesh: &Mesh,
    chest_mesh: &Mesh,
    house_mesh: &Mesh,
    storm_wall_mesh: &Mesh,
    projection: &Mat4,
    local_player_id: Option<u8>,
    rotation: f32,
    current_fps: u32,
) {
    // Acquire render context for this frame
    let render_ctx = match RenderContext::acquire() {
        Some(ctx) => ctx,
        None => return,
    };

    // Clear back buffer and z-buffer (double buffering prevents flicker)
    render_ctx.clear(rgb(50, 70, 100)); // Sky blue background
    render_ctx.clear_zbuffer();

    // Get camera position from local player (or default orbit)
    let (camera_pos, camera_target, local_player_phase) = {
        let world = GAME_WORLD.lock();
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
        render_game_gpu(
            fb_width, fb_height,
            terrain, player_mesh, wall_mesh, bus_mesh,
            glider_mesh, tree_pine_mesh, tree_oak_mesh, rock_mesh,
            chest_mesh, house_mesh, storm_wall_mesh,
            &view, projection, camera_pos, rotation,
        );
        drop(render_ctx);
    } else {
        // === SOFTWARE RENDERING PATH ===
        render_game_software(
            fb_width, fb_height,
            terrain, player_mesh, wall_mesh, bus_mesh,
            glider_mesh, tree_pine_mesh, tree_oak_mesh, rock_mesh,
            chest_mesh, house_mesh, storm_wall_mesh,
            &view, projection, camera_pos, rotation,
        );
        drop(render_ctx);
    }

    // === 2D UI RENDERING ===

    // Draw FPS counter
    font::draw_fps(current_fps, fb_width);

    // Draw crosshair at center of screen
    {
        let fb_guard = FRAMEBUFFER.lock();
        if let Some(fb) = fb_guard.as_ref() {
            panel::draw_crosshair_raw(fb, fb_width, fb_height, 0xFFFFFFFF);
        }
    }

    // Draw storm indicator if player is in storm
    {
        let world_guard = GAME_WORLD.lock();
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
        let world_guard = GAME_WORLD.lock();
        if let Some(world) = world_guard.as_ref() {
            let (health, shield, materials, inventory) = if let Some(id) = local_player_id {
                if let Some(player) = world.get_player(id) {
                    (player.health, player.shield, player.inventory.materials.clone(), Some(&player.inventory))
                } else {
                    (100, 0, crate::game::inventory::Materials::default(), None)
                }
            } else {
                (100, 0, crate::game::inventory::Materials::default(), None)
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
    gpu_render::end_frame();
}

/// GPU rendering path for game frame
fn render_game_gpu(
    fb_width: usize,
    fb_height: usize,
    terrain: &Mesh,
    player_mesh: &Mesh,
    wall_mesh: &Mesh,
    bus_mesh: &Mesh,
    glider_mesh: &Mesh,
    tree_pine_mesh: &Mesh,
    tree_oak_mesh: &Mesh,
    rock_mesh: &Mesh,
    chest_mesh: &Mesh,
    house_mesh: &Mesh,
    storm_wall_mesh: &Mesh,
    view: &Mat4,
    projection: &Mat4,
    camera_pos: Vec3,
    rotation: f32,
) {
    // Begin GPU batch (clears GPU buffers)
    gpu_batch::begin_batch();

    // Create culling context for frustum + distance culling
    let cull_ctx = CullContext::new(view, projection, camera_pos)
        .with_distances(0.5, 500.0);

    // Transform and batch terrain
    let terrain_model = Mat4::from_translation(Vec3::new(0.0, 0.0, 0.0));
    bin_mesh_gpu(terrain, &terrain_model, view, projection, fb_width as f32, fb_height as f32);

    // Batch game world entities with frustum culling
    {
        let world = GAME_WORLD.lock();
        if let Some(w) = world.as_ref() {
            // Render battle bus if active and visible
            if w.bus.active && cull_ctx.should_render(w.bus.position, 10.0) {
                let bus_model = Mat4::from_translation(w.bus.position);
                bin_mesh_gpu(bus_mesh, &bus_model, view, projection, fb_width as f32, fb_height as f32);
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
                    bin_mesh_gpu(house_mesh, &model, view, projection, fb_width as f32, fb_height as f32);
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
                        crate::game::map::VegetationType::TreePine => {
                            bin_mesh_gpu(tree_pine_mesh, &model, view, projection, fb_width as f32, fb_height as f32);
                        }
                        crate::game::map::VegetationType::TreeOak | crate::game::map::VegetationType::TreeBirch => {
                            bin_mesh_gpu(tree_oak_mesh, &model, view, projection, fb_width as f32, fb_height as f32);
                        }
                        crate::game::map::VegetationType::Rock => {
                            bin_mesh_gpu(rock_mesh, &model, view, projection, fb_width as f32, fb_height as f32);
                        }
                        crate::game::map::VegetationType::Bush => {
                            let bush_model = model * Mat4::from_scale(Vec3::splat(0.5));
                            bin_mesh_gpu(tree_oak_mesh, &bush_model, view, projection, fb_width as f32, fb_height as f32);
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
                bin_mesh_gpu(chest_mesh, &model, view, projection, fb_width as f32, fb_height as f32);
            }

            // Render all players (always render, they're important)
            for player in &w.players {
                if !player.is_alive() || player.phase == PlayerPhase::OnBus {
                    continue;
                }

                let model = Mat4::from_translation(player.position)
                    * Mat4::from_rotation_y(player.yaw + core::f32::consts::PI);
                bin_mesh_gpu(player_mesh, &model, view, projection, fb_width as f32, fb_height as f32);

                if player.phase == PlayerPhase::Gliding {
                    let glider_offset = Vec3::new(0.0, 2.5, 0.0);
                    let glider_model = Mat4::from_translation(player.position + glider_offset)
                        * Mat4::from_rotation_y(player.yaw + core::f32::consts::PI);
                    bin_mesh_gpu(glider_mesh, &glider_model, view, projection, fb_width as f32, fb_height as f32);
                }
            }

            // Render player-built buildings with culling
            for building in &w.buildings {
                if !cull_ctx.should_render(building.position, 5.0) {
                    continue;
                }
                let model = Mat4::from_translation(building.position)
                    * Mat4::from_rotation_y(building.rotation);
                bin_mesh_gpu(wall_mesh, &model, view, projection, fb_width as f32, fb_height as f32);
            }

            // Render 3D storm wall (always render, important visual)
            let storm_model = Mat4::from_translation(Vec3::new(w.storm.center.x, 0.0, w.storm.center.z))
                * Mat4::from_scale(Vec3::new(w.storm.radius, 1.0, w.storm.radius));
            bin_mesh_gpu(storm_wall_mesh, &storm_model, view, projection, fb_width as f32, fb_height as f32);
        }
    }

    // End GPU batch (flushes remaining triangles and presents)
    gpu_batch::end_batch();
}

/// Software rendering path for game frame
fn render_game_software(
    fb_width: usize,
    fb_height: usize,
    terrain: &Mesh,
    player_mesh: &Mesh,
    wall_mesh: &Mesh,
    bus_mesh: &Mesh,
    glider_mesh: &Mesh,
    tree_pine_mesh: &Mesh,
    tree_oak_mesh: &Mesh,
    rock_mesh: &Mesh,
    chest_mesh: &Mesh,
    house_mesh: &Mesh,
    storm_wall_mesh: &Mesh,
    view: &Mat4,
    projection: &Mat4,
    camera_pos: Vec3,
    rotation: f32,
) {
    // 1. Clear lock-free bins and reset triangle buffer
    tiles::clear_lockfree_bins();
    tiles::reset_triangle_buffer();

    // 2. Create culling context for frustum + distance culling
    let cull_ctx = CullContext::new(view, projection, camera_pos)
        .with_distances(0.5, 500.0); // Near 0.5, Far 300 units

    // 3. Transform and bin terrain (always render, but reduced complexity)
    let terrain_model = Mat4::from_translation(Vec3::new(0.0, 0.0, 0.0));
    bin_mesh(terrain, &terrain_model, view, projection, fb_width as f32, fb_height as f32);

    // 4. Render game world entities with frustum culling
    {
        let world = GAME_WORLD.lock();
        if let Some(w) = world.as_ref() {
            // Render battle bus if active and visible
            if w.bus.active && cull_ctx.should_render(w.bus.position, 10.0) {
                let bus_model = Mat4::from_translation(w.bus.position);
                bin_mesh(bus_mesh, &bus_model, view, projection, fb_width as f32, fb_height as f32);
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
                    bin_mesh(house_mesh, &model, view, projection, fb_width as f32, fb_height as f32);
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
                        crate::game::map::VegetationType::TreePine => {
                            bin_mesh(tree_pine_mesh, &model, view, projection, fb_width as f32, fb_height as f32);
                        }
                        crate::game::map::VegetationType::TreeOak | crate::game::map::VegetationType::TreeBirch => {
                            bin_mesh(tree_oak_mesh, &model, view, projection, fb_width as f32, fb_height as f32);
                        }
                        crate::game::map::VegetationType::Rock => {
                            bin_mesh(rock_mesh, &model, view, projection, fb_width as f32, fb_height as f32);
                        }
                        crate::game::map::VegetationType::Bush => {
                            let bush_model = model * Mat4::from_scale(Vec3::splat(0.5));
                            bin_mesh(tree_oak_mesh, &bush_model, view, projection, fb_width as f32, fb_height as f32);
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
                bin_mesh(chest_mesh, &model, view, projection, fb_width as f32, fb_height as f32);
            }

            // Render all players (always render, they're important)
            for player in &w.players {
                if !player.is_alive() || player.phase == PlayerPhase::OnBus {
                    continue;
                }

                let model = Mat4::from_translation(player.position)
                    * Mat4::from_rotation_y(player.yaw + core::f32::consts::PI);
                bin_mesh(player_mesh, &model, view, projection, fb_width as f32, fb_height as f32);

                if player.phase == PlayerPhase::Gliding {
                    let glider_offset = Vec3::new(0.0, 2.5, 0.0);
                    let glider_model = Mat4::from_translation(player.position + glider_offset)
                        * Mat4::from_rotation_y(player.yaw + core::f32::consts::PI);
                    bin_mesh(glider_mesh, &glider_model, view, projection, fb_width as f32, fb_height as f32);
                }
            }

            // Render player-built buildings with culling
            for building in &w.buildings {
                if !cull_ctx.should_render(building.position, 5.0) {
                    continue;
                }
                let model = Mat4::from_translation(building.position)
                    * Mat4::from_rotation_y(building.rotation);
                bin_mesh(wall_mesh, &model, view, projection, fb_width as f32, fb_height as f32);
            }

            // Render 3D storm wall (always render, important visual)
            let storm_model = Mat4::from_translation(Vec3::new(w.storm.center.x, 0.0, w.storm.center.z))
                * Mat4::from_scale(Vec3::new(w.storm.radius, 1.0, w.storm.radius));
            bin_mesh(storm_wall_mesh, &storm_model, view, projection, fb_width as f32, fb_height as f32);
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
}

/// Transform mesh triangles, create ScreenTriangles, and bin them to tiles
/// Uses GPU batch rendering when available, falls back to software rasterization
/// Returns the number of triangles successfully processed
pub fn bin_mesh(
    mesh: &Mesh,
    model: &Mat4,
    view: &Mat4,
    projection: &Mat4,
    fb_width: f32,
    fb_height: f32,
) -> usize {
    let mut binned = 0;

    // Precompute MVP matrix ONCE per mesh (instead of 3 matrix muls per vertex!)
    let mvp = *projection * *view * *model;

    // Use the simple software path - GPU batch will be used when SVGA3D is available
    // The is_enabled() check is done once at startup, not per-triangle
    for i in 0..mesh.triangle_count() {
        if let Some((v0, v1, v2)) = mesh.get_triangle(i) {
            // Transform and create ScreenTriangle using precomputed MVP
            if let Some(screen_tri) = transform_and_bin_fast(
                v0,
                v1,
                v2,
                &mvp,
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
pub fn bin_mesh_gpu(
    mesh: &Mesh,
    model: &Mat4,
    view: &Mat4,
    projection: &Mat4,
    fb_width: f32,
    fb_height: f32,
) -> usize {
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
    let ctx = match RenderContext::acquire() {
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
    ctx: &RenderContext,
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
                rasterize_screen_triangle_simple(
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
