# BattleRoyaleOS Implementation Plan

## Overview

This document tracks the implementation status of BattleRoyaleOS, a bare-metal Rust unikernel designed to run a 100-player Battle Royale game on local network.

**Current Status**: Phase 1-3 Complete, Phase 4 Partially Complete, Phase 5 Mostly Complete

**Latest Update**: Full Fortnite-style game implementation with voxel rendering, menu system, combat, loot, and map. Game state machine flows through MainMenu → Lobby → Countdown → BusPhase → InGame → Victory.

---

## Implementation Phases

### Phase 1: Kernel Boot & Serial Output ✅ COMPLETE

**Goal**: Boot via Limine, initialize serial, print "BattleRoyaleOS Kernel Loaded"

#### Completed Items

| File | Status | Description |
|------|--------|-------------|
| `Cargo.toml` | ✅ | Workspace with kernel, renderer, protocol crates |
| `.cargo/config.toml` | ✅ | Build target x86_64-unknown-none, build-std |
| `rust-toolchain.toml` | ✅ | Nightly channel with rust-src |
| `kernel/Cargo.toml` | ✅ | Dependencies: limine, x86_64, spin, talc, smoltcp, glam |
| `kernel/linker-x86_64.ld` | ✅ | Linker script with .requests section at 0xffffffff80000000 |
| `kernel/src/main.rs` | ✅ | Entry point `_start`, panic handler, main loop with state machine |
| `kernel/src/boot.rs` | ✅ | Limine requests (BaseRevision, Framebuffer, MemoryMap, HHDM, MP) |
| `kernel/src/drivers/serial.rs` | ✅ | COM1 serial output with `serial_println!` macro |
| `kernel/src/memory/allocator.rs` | ✅ | Talc heap allocator (64MB) |
| `kernel/src/memory/dma.rs` | ✅ | DMA buffer management for NIC |
| `limine.conf` | ✅ | Bootloader configuration |
| `GNUmakefile` | ✅ | ISO build, QEMU targets |

#### Verification
```bash
make run
# Expected: "BattleRoyaleOS Kernel Loaded" in serial output
# Expected: Framebuffer dimensions printed
# Expected: CPU count printed (should be 5)
```

**Result**: ✅ Kernel boots successfully, timer interrupts serviced

---

### Phase 2: E1000 Network Stack ✅ COMPLETE (Core Implementation)

**Goal**: Initialize E1000 NIC, establish UDP communication

#### Completed Items

| File | Status | Description |
|------|--------|-------------|
| `kernel/src/drivers/pci.rs` | ✅ | PCI enumeration, find device by vendor:device ID |
| `kernel/src/drivers/e1000/mod.rs` | ✅ | E1000 driver: init, reset, MAC read, TX/RX |
| `kernel/src/drivers/e1000/regs.rs` | ✅ | All E1000 register definitions |
| `kernel/src/drivers/e1000/descriptors.rs` | ✅ | TxDescriptor, RxDescriptor (16 bytes each) |
| `kernel/src/drivers/e1000/ring.rs` | ✅ | TX/RX ring buffer management (256 RX, 128 TX) |
| `kernel/src/net/device.rs` | ✅ | smoltcp Device trait implementation |
| `kernel/src/net/stack.rs` | ✅ | NetworkStack wrapper, UDP socket |
| `kernel/src/net/protocol.rs` | ✅ | Game protocol handler scaffold |

#### Configuration
- RX Ring: 256 descriptors, 2048-byte buffers
- TX Ring: 128 descriptors, 2048-byte buffers
- IP Address: 10.0.2.15 (QEMU user networking)
- Gateway: 10.0.2.2
- UDP Port: 5000

#### TODO - Phase 2 Polish
- [ ] Implement ICMP ping response for debugging
- [ ] Add packet checksum validation
- [ ] Implement link status monitoring
- [ ] Add network statistics tracking

---

### Phase 3: Software Rasterizer ✅ COMPLETE

**Goal**: Render 3D graphics at >30 FPS

#### Completed Items

| File | Status | Description |
|------|--------|-------------|
| `kernel/src/graphics/framebuffer.rs` | ✅ | Limine framebuffer wrapper, pixel operations |
| `kernel/src/graphics/zbuffer.rs` | ✅ | Depth buffer with test_and_set |
| `kernel/src/graphics/rasterizer.rs` | ✅ | Edge-function triangle rasterization |
| `kernel/src/graphics/tiles.rs` | ✅ | Tile work queue, triangle binning structures |
| `kernel/src/graphics/pipeline.rs` | ✅ | MVP transformation, backface culling |
| `kernel/src/graphics/font.rs` | ✅ | Full alphabet (A-Z), numbers (0-9), punctuation |
| `kernel/src/graphics/ui/mod.rs` | ✅ | UI primitives module |
| `kernel/src/graphics/ui/button.rs` | ✅ | Interactive button widget |
| `kernel/src/graphics/ui/panel.rs` | ✅ | Panels, gradients, rectangles |
| `kernel/src/graphics/ui/list.rs` | ✅ | Scrollable player list |
| `kernel/src/graphics/ui/colors.rs` | ✅ | Fortnite-style color palette |
| `renderer/src/lib.rs` | ✅ | Renderer crate root |
| `renderer/src/vertex.rs` | ✅ | Vertex struct (position, normal, color, uv) |
| `renderer/src/math.rs` | ✅ | Direction calculation, rotation helpers |
| `renderer/src/mesh.rs` | ✅ | Procedural mesh generation |
| `renderer/src/voxel.rs` | ✅ | VoxelModel with greedy meshing |
| `renderer/src/voxel_models.rs` | ✅ | Character, weapon, and object models |
| `renderer/src/voxel_world.rs` | ✅ | Terrain chunks, buildings, vegetation |

#### Voxel System Features
- Greedy mesh optimization for efficient rendering
- Character models with customizable colors (skin, shirt, pants, shoes)
- Weapon models (pickaxe, pistol, shotgun, AR, sniper, SMG)
- Building structures (houses, warehouses, towers, fences, crates)
- Procedural terrain with height-based surface types
- Cloud generation

#### Tile Configuration
- Tile size: 64×64 pixels
- L1 cache fit: 64×64×4 = 16KB per tile z-buffer
- Work distribution: Atomic counter
- 4-core parallel rendering

**Result**: ✅ 60 FPS at 1280×800 with voxel world rendering

---

### Phase 4: Game Logic & UI ✅ MOSTLY COMPLETE

**Goal**: Complete game state machine with menus and UI

#### Completed Items

| File | Status | Description |
|------|--------|-------------|
| `kernel/src/game/state.rs` | ✅ | GameState enum, PlayerPhase, PlayerCustomization |
| `kernel/src/game/input.rs` | ✅ | PS/2 keyboard + mouse polling, MenuAction |
| `kernel/src/game/world.rs` | ✅ | GameWorld state, player management, delta tracking |
| `kernel/src/game/player.rs` | ✅ | Player entity with phases, inventory, physics |
| `kernel/src/game/weapon.rs` | ✅ | 6 weapon types with damage, fire rate, magazine |
| `kernel/src/game/inventory.rs` | ✅ | 5 weapon slots, ammo reserves, materials |
| `kernel/src/game/combat.rs` | ✅ | Hitscan ray-player intersection, headshots |
| `kernel/src/game/loot.rs` | ✅ | Loot drops, chest spawning, floor loot |
| `kernel/src/game/map.rs` | ✅ | 10 POIs, terrain generation, building placement |
| `kernel/src/ui/main_menu.rs` | ✅ | PLAY, SETTINGS, QUIT buttons |
| `kernel/src/ui/lobby.rs` | ✅ | Player list, ready system, countdown |
| `kernel/src/ui/settings.rs` | ✅ | FPS toggle, invert Y, sensitivity, render distance |
| `kernel/src/ui/customization.rs` | ✅ | 3D player preview, color selection |
| `kernel/src/ui/game_ui.rs` | ✅ | HUD, minimap, weapon hotbar, countdown, victory |
| `protocol/src/packets.rs` | ✅ | PlayerState, ClientInput, WorldStateDelta |

#### Game State Flow
```
MainMenu → Lobby → Countdown(5) → BusPhase → InGame → Victory → MainMenu
         ↓
      Settings
         ↓
    Customization
```

#### Player Phases
| Phase | Description |
|-------|-------------|
| OnBus | Riding the battle bus, press Space to drop |
| Freefall | Falling at 50-80 units/sec, can dive or slow |
| Gliding | Glider deployed at <200m, 10 units/sec descent |
| Grounded | Normal gameplay, combat, building |
| Eliminated | Dead, waiting to spectate |
| Spectating | Watching other players |

#### Weapon Types
| Type | Damage | Fire Rate | Magazine | Range |
|------|--------|-----------|----------|-------|
| Pickaxe | 20 | 1.0 | ∞ | 2m |
| Pistol | 23 | 6.75 | 16 | 50m |
| Shotgun | 90 | 0.7 | 5 | 15m |
| Assault Rifle | 30 | 5.5 | 30 | 100m |
| Sniper | 100 | 0.33 | 1 | 500m |
| SMG | 17 | 12.0 | 30 | 40m |

#### Map POIs (Points of Interest)
| Name | Position | Radius | Loot Tier |
|------|----------|--------|-----------|
| Pleasant Park | (-400, 0, -400) | 150 | Normal |
| Tilted Towers | (0, 0, 0) | 200 | High |
| Retail Row | (500, 0, -300) | 120 | Normal |
| Salty Springs | (-200, 0, 300) | 100 | Normal |
| Lonely Lodge | (600, 0, 500) | 80 | Low |
| Loot Lake | (-100, 0, -600) | 180 | Normal |
| Fatal Fields | (400, 0, 400) | 140 | Normal |
| Wailing Woods | (-600, 0, 200) | 160 | Low |
| Dusty Depot | (200, 0, -100) | 100 | Normal |
| Tomato Town | (-300, 0, -700) | 90 | Low |

#### TODO - Phase 4 Remaining
- [ ] **Client/Server Mode Detection**: Command-line or config to select mode
- [ ] **Server Discovery**: Broadcast to find server on LAN
- [ ] **Connection Handshake**: Proper JoinRequest/JoinResponse flow
- [ ] **Player Interpolation**: Buffer 2 ticks, lerp between states
- [ ] **Name Tags**: Display player names above heads

---

### Phase 5: Battle Royale Mechanics ✅ MOSTLY COMPLETE

**Goal**: Complete game with drop, building, storm, combat

#### Completed Items

| File | Status | Description |
|------|--------|-------------|
| `kernel/src/game/bus.rs` | ✅ | BattleBus entity, path, drop mechanics |
| `kernel/src/game/storm.rs` | ✅ | Storm phases, shrinking, damage |
| `kernel/src/game/building.rs` | ✅ | BuildPiece types, grid snapping |
| `kernel/src/game/combat.rs` | ✅ | Hitscan with headshot detection |
| `kernel/src/game/loot.rs` | ✅ | Chest loot, floor loot, death drops |

#### Storm System

| Phase | Radius | Shrink Time | Wait Time | Damage |
|-------|--------|-------------|-----------|--------|
| 1 | 1000m | 0s | 120s | 1/tick |
| 2 | 500m | 60s | 90s | 2/tick |
| 3 | 250m | 45s | 60s | 5/tick |
| 4 | 100m | 30s | 30s | 10/tick |
| 5 | 25m | 15s | 15s | 15/tick |
| 6 | 0m | 10s | 0s | 20/tick |

#### Combat System
- [x] **Hitscan Weapons**: Ray-player intersection for hit detection
- [x] **Headshot Detection**: 2x damage multiplier for head hits
- [x] **Damage Falloff**: Reduced damage at range
- [x] **Shotgun Spread**: Multiple pellets with spread pattern
- [x] **Shield System**: Shield absorbs damage before health
- [x] **Elimination Tracking**: Track kills and damage dealt

#### Loot System
- [x] **Chest Spawning**: Spawn weapons + ammo/materials + healing
- [x] **Floor Loot**: Random item spawns on ground
- [x] **Death Drops**: Players drop inventory on death
- [x] **Rarity Tiers**: Common, Uncommon, Rare, Epic, Legendary

#### Drop Mechanics
- [x] **Bus Exit**: Space to exit bus
- [x] **Freefall**: 50-80 units/sec depending on dive angle
- [x] **Glider Deploy**: Auto at 100m, manual at 200m+
- [x] **Landing**: Transition to grounded phase

#### TODO - Phase 5 Remaining
- [ ] **Building Collision**: Players can't walk through builds
- [ ] **Building Damage**: Weapons can destroy builds
- [ ] **Damage Numbers**: Floating damage text
- [ ] **Kill Feed**: Show elimination messages

---

## SMP Implementation Status ✅ COMPLETE

### Core Assignment

| Core | Role | Status |
|------|------|--------|
| 0 | Game Logic + Rasterizer | ✅ Implemented |
| 1-3 | Rasterizers | ✅ Fully working with render_worker() |
| 4 | Network | ✅ Implemented |

### SMP Features ✅
- [x] **Tile Work Distribution**: Lock-free atomic counter, work-stealing
- [x] **Frame Synchronization**: CoreBarrier with 4 cores
- [x] **Double Buffering**: Back buffer + present() for flicker-free
- [x] **60 FPS Frame Limiter**: TSC-based timing

---

## Input Support ✅ COMPLETE

### Keyboard
- [x] PS/2 keyboard driver with scan code handling
- [x] WASD movement
- [x] Arrow keys for camera
- [x] Space for jump/drop
- [x] Enter/Escape for menu navigation
- [x] Number keys for weapon slots

### Mouse
- [x] PS/2 mouse driver
- [x] Mouse movement for camera look
- [x] Mouse buttons for fire/build
- [x] Sensitivity configuration

---

## Memory Map

```
Virtual Address Space:
0xFFFFFFFF80000000 - Kernel base (linker script)

Physical Memory (from Limine):
- Usable RAM for allocator
- Framebuffer (linear, HHDM mapped)
- E1000 MMIO (BAR0, HHDM mapped)
```

---

## Known Issues & Bugs

1. **Serial Output Not Visible in Background**: When running via `make run` in background, serial output doesn't show. Use foreground terminal.

2. **No ICMP Support**: Can't ping the kernel yet (smoltcp ICMP feature not enabled).

3. ~~**Single-Core Rendering**~~: ✅ RESOLVED - Now parallel on 4 cores (0-3).

4. ~~**No Frame Timing**~~: ✅ RESOLVED - 60 FPS frame limiter using TSC.

5. ~~**Keyboard Only**~~: ✅ RESOLVED - Mouse input now supported.

---

## Performance Budget

**Target**: 60 FPS at 1280×800 ✅ ACHIEVED

| Component | Budget | Current |
|-----------|--------|---------|
| Triangle binning | 2ms | Lock-free atomic |
| Rasterization | 10ms | 4-core parallel |
| Buffer swap | 2ms | Double buffering + present() |
| Game logic | 5ms | Player input + game world |
| Network | 4ms | Polling on core 4 |
| **Total** | 16.6ms | 60 FPS (frame-limited) |

**Scene Complexity**: Voxel terrain + players + buildings + vegetation

---

## Next Steps (Priority Order)

### Immediate (Network Multiplayer)
1. Implement proper client/server mode selection
2. Test actual UDP packet transmission between instances
3. Get two instances communicating
4. Player interpolation for smooth remote movement

### Short Term (Polish)
1. Building collision detection
2. Kill feed UI
3. Damage numbers
4. Sound (would need audio driver)

### Medium Term (Full Game)
1. More weapon types and balancing
2. Vehicles
3. Spectator camera improvements
4. Replay system

---

## Testing Checklist

### Phase 1 ✅
- [x] Kernel boots in QEMU
- [x] Serial output visible
- [x] Framebuffer initialized
- [x] Memory allocator works
- [x] SMP cores detected

### Phase 2 🟡
- [x] E1000 detected on PCI bus
- [x] E1000 initialized (link up)
- [x] smoltcp stack initialized
- [ ] UDP packets send successfully
- [ ] UDP packets receive successfully
- [ ] Can communicate between two instances

### Phase 3 ✅
- [x] Framebuffer displays graphics
- [x] Z-buffer prevents overdraw
- [x] Triangle rasterization works
- [x] Voxel models render correctly
- [x] FPS counter shows 60 FPS
- [x] Parallel rendering on multiple cores (4 cores active)

### Phase 4 ✅
- [x] Game state machine works
- [x] Main menu navigable
- [x] Settings persist
- [x] Customization preview renders
- [x] Lobby shows players
- [x] Countdown transitions to bus

### Phase 5 🟡
- [x] Bus spawns and moves
- [x] Players can exit bus
- [x] Glider deploys
- [x] Storm circle logic works
- [x] Storm deals damage
- [x] Combat system works
- [x] Loot system works
- [ ] Building collision
- [x] Victory condition detected

---

## How to Run

```bash
# Build and create ISO
make image.iso

# Run in QEMU
qemu-system-x86_64 -M q35 -m 512M -smp 5 -cdrom image.iso \
    -device e1000,netdev=net0 -netdev user,id=net0 \
    -serial stdio -no-reboot

# Or use makefile target
make run
```

---

## Contributing

When implementing a feature:

1. Update this PLAN.md with status
2. Add `serial_println!` debug output
3. Test in QEMU single instance first
4. Test networked if applicable
5. Update README if user-facing

---

## References

- [OSDev Wiki - Bare Bones](https://wiki.osdev.org/Bare_Bones)
- [Limine Protocol Specification](https://github.com/limine-bootloader/limine/blob/trunk/PROTOCOL.md)
- [Intel E1000 Developer Manual](https://www.intel.com/content/dam/doc/manual/pci-pci-x-family-gbe-controllers-software-dev-manual.pdf)
- [smoltcp Documentation](https://docs.rs/smoltcp/)
- [Software Rasterization Tutorial](https://www.scratchapixel.com/lessons/3d-basic-rendering/rasterization-practical-implementation)
