# BattleRoyaleOS Implementation Plan

## Overview

This document tracks the implementation status of BattleRoyaleOS, a bare-metal Rust unikernel designed to run a 100-player Battle Royale game on local network.

**Current Status**: Phase 1-3 Core Complete, Phase 4-5 Scaffolded

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
| `kernel/src/main.rs` | ✅ | Entry point `_start`, panic handler, main loop |
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

#### Key Structures
```rust
// TX Descriptor (16 bytes)
struct TxDescriptor {
    buffer_addr: u64,
    length: u16,
    cso: u8,
    cmd: u8,       // EOP | IFCS | RS
    status: u8,    // DD bit
    css: u8,
    special: u16,
}

// RX Descriptor (16 bytes)
struct RxDescriptor {
    buffer_addr: u64,
    length: u16,
    checksum: u16,
    status: u8,    // DD | EOP bits
    errors: u8,
    special: u16,
}
```

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
- [ ] Test actual packet transmission/reception

#### Verification
```bash
# In one terminal:
make run
# Kernel should print "Link up" and IP address

# In another terminal:
ping -c 3 10.0.2.15
# Should receive responses (requires ICMP implementation)
```

---

### Phase 3: Software Rasterizer ✅ COMPLETE (Core Implementation)

**Goal**: Render 3D graphics at >30 FPS

#### Completed Items

| File | Status | Description |
|------|--------|-------------|
| `kernel/src/graphics/framebuffer.rs` | ✅ | Limine framebuffer wrapper, pixel operations |
| `kernel/src/graphics/zbuffer.rs` | ✅ | Depth buffer with test_and_set |
| `kernel/src/graphics/rasterizer.rs` | ✅ | Scanline triangle rasterization, Gouraud shading |
| `kernel/src/graphics/tiles.rs` | ✅ | Tile work queue, triangle binning structures |
| `kernel/src/graphics/pipeline.rs` | ✅ | MVP transformation, backface culling |
| `renderer/src/lib.rs` | ✅ | Renderer crate root |
| `renderer/src/vertex.rs` | ✅ | Vertex struct (position, normal, color, uv) |
| `renderer/src/math.rs` | ✅ | Direction calculation, rotation helpers |
| `renderer/src/mesh.rs` | ✅ | Procedural mesh generation |

#### Rendering Pipeline
1. **Model Transform**: Local → World space
2. **View Transform**: World → Camera space
3. **Projection**: Camera → Clip space (perspective)
4. **NDC**: Clip → Normalized Device Coordinates
5. **Viewport**: NDC → Screen coordinates
6. **Backface Cull**: Reject back-facing triangles
7. **Rasterize**: Scanline with z-buffer testing
8. **Shade**: Gouraud interpolation

#### Procedural Meshes

| Mesh | Function | Triangles | Description |
|------|----------|-----------|-------------|
| Cube | `create_cube()` | 12 | Unit cube for testing |
| Player | `create_player_mesh()` | ~24 | Capsule (box body + head) |
| Wall | `create_wall_mesh()` | 12 | 4×4×0.2 wall piece |
| Ramp | `create_ramp_mesh()` | 8 | Triangular prism |
| Battle Bus | `create_battle_bus_mesh()` | ~36 | Bus + balloon |
| Ground | `create_ground_mesh()` | 2 | Large ground plane |

#### Tile Configuration
- Tile size: 64×64 pixels
- L1 cache fit: 64×64×4 = 16KB per tile z-buffer
- Work distribution: Atomic counter

#### TODO - Phase 3 Polish
- [ ] Implement parallel tile rendering on cores 1-3
- [ ] Add texture mapping support
- [ ] Implement simple lighting (directional)
- [ ] Add fog for distance culling
- [ ] Optimize triangle clipping
- [ ] Add FPS counter overlay
- [ ] Double buffering for tear-free rendering

#### Verification
```bash
make run
# Visual: Spinning cube on screen
# Serial: FPS counter should show >30
```

**Result**: ✅ Main loop renders spinning cube and ground plane

---

### Phase 4: Game Logic & Multiplayer 🟡 SCAFFOLDED

**Goal**: Multiple clients connect and see each other move

#### Completed Items

| File | Status | Description |
|------|--------|-------------|
| `protocol/src/packets.rs` | ✅ | PlayerState (24 bytes), ClientInput, WorldStateDelta |
| `protocol/src/codec.rs` | ✅ | Binary serialization helpers |
| `kernel/src/game/world.rs` | ✅ | GameWorld state, player management, delta tracking |
| `kernel/src/game/player.rs` | ✅ | Player entity with physics, input handling |
| `kernel/src/game/input.rs` | ✅ | PS/2 keyboard polling, key state |
| `kernel/src/net/protocol.rs` | ✅ | Packet encode/decode, handler scaffold |

#### Network Protocol

##### Packet Types
```rust
enum Packet {
    JoinRequest { name: String },
    JoinResponse { player_id: u8 },
    ClientInput(ClientInput),
    WorldStateDelta(WorldStateDelta),
    Ping { timestamp: u64 },
    Pong { timestamp: u64 },
}
```

##### PlayerState (24 bytes, wire format)
```rust
#[repr(C, packed)]
struct PlayerState {
    player_id: u8,
    x: i32,           // Fixed-point 16.16
    y: i32,
    z: i32,
    yaw: i16,         // Degrees × 100
    pitch: i16,
    health: u8,
    weapon_id: u8,
    state: u8,        // Flags
    _padding: u8,
}
```

##### State Flags
```rust
const ALIVE: u8 = 1 << 0;
const JUMPING: u8 = 1 << 1;
const CROUCHING: u8 = 1 << 2;
const BUILDING: u8 = 1 << 3;
const IN_BUS: u8 = 1 << 4;
const PARACHUTE: u8 = 1 << 5;
```

#### Server Loop (20Hz)
```
1. Receive all client inputs
2. Update player positions (physics)
3. Check collisions
4. Compute delta (changed players)
5. Broadcast WorldStateDelta to all clients
```

#### Client Loop
```
1. Poll keyboard input
2. Send ClientInput to server
3. Receive WorldStateDelta
4. Store in interpolation buffer
5. Interpolate between T-1 and T for rendering
```

#### TODO - Phase 4 Implementation

##### High Priority
- [ ] **Client/Server Mode Detection**: Command-line or config to select mode
- [ ] **Server Discovery**: Broadcast to find server on LAN
- [ ] **Connection Handshake**: Proper JoinRequest/JoinResponse flow
- [ ] **Player Interpolation**: Buffer 2 ticks, lerp between states
- [ ] **Input Prediction**: Client-side movement prediction
- [ ] **Lag Compensation**: Server rewind for hit detection

##### Medium Priority
- [ ] **Player Rendering**: Render all connected players
- [ ] **Name Tags**: Display player names above heads
- [ ] **Kill Feed**: Show elimination messages
- [ ] **Player Count UI**: Display alive/total players
- [ ] **Disconnect Handling**: Graceful player removal

##### Low Priority
- [ ] **Spectator Mode**: Watch other players after death
- [ ] **Replay System**: Record and playback games
- [ ] **Anti-Cheat**: Basic sanity checks on client input

#### Verification
```bash
# Start two QEMU instances with socket networking:
# Instance 1 (server):
make run-network

# Instance 2 (client):
make run-network-client

# Both should show player entities
# Moving in one should update the other
```

---

### Phase 5: Battle Royale Mechanics 🟡 SCAFFOLDED

**Goal**: Complete game with drop, building, storm

#### Completed Items

| File | Status | Description |
|------|--------|-------------|
| `kernel/src/game/bus.rs` | ✅ | BattleBus entity, path, drop mechanics |
| `kernel/src/game/storm.rs` | ✅ | Storm phases, shrinking, damage |
| `kernel/src/game/building.rs` | ✅ | BuildPiece types, grid snapping |

#### Battle Bus System

```rust
const BUS_HEIGHT: f32 = 5000.0;
const BUS_SPEED: f32 = 100.0;
const MAP_SIZE: f32 = 2000.0;

// Bus travels across map at constant height
// Players press Space to exit
// Path randomized each game
```

#### Drop Mechanics
```
1. Player exits bus at current position
2. Freefall at 50 units/sec until y=100
3. Parachute auto-deploys
4. Descend at 10 units/sec until landing
```

#### Storm System

| Phase | Radius | Shrink Time | Wait Time | Damage |
|-------|--------|-------------|-----------|--------|
| 1 | 1000m | 0s | 120s | 1/tick |
| 2 | 500m | 60s | 90s | 2/tick |
| 3 | 250m | 45s | 60s | 5/tick |
| 4 | 100m | 30s | 30s | 10/tick |
| 5 | 25m | 15s | 15s | 15/tick |
| 6 | 0m | 10s | 0s | 20/tick |

#### Building System

| Piece | Dimensions | Cost | Health |
|-------|------------|------|--------|
| Wall | 4×4×0.2 | 10 | 150 |
| Floor | 4×0.2×4 | 10 | 140 |
| Ramp | 4×4×4 | 10 | 140 |

```rust
// Build placement
let forward = player.forward();
let build_pos = player.position + forward * 4.0;
let piece = BuildPiece::wall(snap_to_grid(build_pos), player.yaw);
```

#### TODO - Phase 5 Implementation

##### Core Mechanics
- [ ] **Bus Path Visualization**: Draw bus route on minimap
- [ ] **Drop UI**: Show "Press SPACE to drop" prompt
- [ ] **Parachute Animation**: Visual feedback during descent
- [ ] **Landing Detection**: Proper ground collision
- [ ] **Storm Visualization**: Render storm circle on ground
- [ ] **Storm Damage**: Apply damage outside safe zone
- [ ] **Building Placement Preview**: Ghost piece before placing
- [ ] **Building Collision**: Players can't walk through builds
- [ ] **Building Damage**: Weapons can destroy builds

##### Combat
- [ ] **Weapon System**: Pickaxe, shotgun, rifle
- [ ] **Hit Detection**: Ray-triangle intersection
- [ ] **Damage Numbers**: Floating damage text
- [ ] **Health Bar**: Player health UI
- [ ] **Shield System**: Additional HP layer
- [ ] **Loot System**: Floor loot spawns

##### Victory Condition
- [ ] **Last Player Standing**: Detect winner
- [ ] **Victory Royale Screen**: End game display
- [ ] **Game Reset**: Return to lobby/restart

##### Polish
- [ ] **Minimap**: Show zone, players, bus
- [ ] **Inventory UI**: Show materials, weapons
- [ ] **Sound Effects**: (Would need audio driver)
- [ ] **Particle Effects**: Muzzle flash, build placement

---

## SMP Implementation Status

### Core Assignment

| Core | Role | Status |
|------|------|--------|
| 0 | Game Logic | ✅ Implemented |
| 1-3 | Rasterizers | 🟡 Entry points exist, work distribution TODO |
| 4 | Network | ✅ Entry point exists |

### TODO - SMP
- [ ] **Tile Work Distribution**: Cores 1-3 pull tiles from atomic queue
- [ ] **Frame Synchronization**: Barrier between cores at frame end
- [ ] **Double Buffering**: Prevent tearing with buffer swap
- [ ] **Load Balancing**: Dynamic tile assignment based on complexity

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

3. **Single-Core Rendering**: Tile parallel rendering not yet implemented, all rendering on core 0.

4. **No Frame Timing**: No precise timing, frame rate depends on CPU speed.

5. **Keyboard Only**: No mouse input support yet.

---

## Performance Budget

**Target**: 30 FPS at 1280×720

| Component | Budget | Current |
|-----------|--------|---------|
| Triangle binning | 2ms | Not measured |
| Rasterization | 20ms | Single-core, unmeasured |
| Buffer swap | 2ms | Direct write |
| Game logic | 5ms | Minimal |
| Network | 4ms | Polling |
| **Total** | 33ms | ~30 FPS observed |

---

## Next Steps (Priority Order)

### Immediate (Get Multiplayer Working)
1. Implement proper client/server mode selection
2. Test actual UDP packet transmission
3. Get two instances communicating
4. Render remote players

### Short Term (Playable Game)
1. Implement parallel rendering on cores 1-3
2. Add storm visualization and damage
3. Implement building collision
4. Add basic weapon/combat

### Medium Term (Polish)
1. Add minimap UI
2. Implement spectator mode
3. Add proper game loop (lobby → game → victory)
4. Performance optimization

### Long Term (Full Game)
1. Multiple weapon types
2. Loot system
3. Sound (would need audio driver)
4. More map features

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

### Phase 3 🟡
- [x] Framebuffer displays graphics
- [x] Z-buffer prevents overdraw
- [x] Triangle rasterization works
- [x] Cube renders correctly
- [ ] FPS counter shows >30
- [ ] Parallel rendering on multiple cores

### Phase 4 ⬜
- [ ] Players can join server
- [ ] Player positions sync
- [ ] Input reaches server
- [ ] State broadcasts to all clients
- [ ] Interpolation smooth

### Phase 5 ⬜
- [ ] Bus spawns and moves
- [ ] Players can exit bus
- [ ] Parachute deploys
- [ ] Storm circle visible
- [ ] Storm deals damage
- [ ] Building placement works
- [ ] Victory condition detected

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
