# BattleRoyaleOS

A bare-metal Rust unikernel operating system designed to run a 100-player Battle Royale game on a local network. The OS and game compile into a single binary targeting x86_64 with software-rendered 3D graphics.

## Features

- **Bare-metal execution**: No underlying OS, boots directly via Limine bootloader
- **Software 3D rendering**: Custom rasterizer with z-buffering and Gouraud shading
- **Multicore support**: SMP with dedicated cores for game logic, rendering, and networking
- **Network stack**: E1000 NIC driver with smoltcp TCP/IP stack
- **Battle Royale mechanics**: 100 players, storm system, building, battle bus
- **Complete weapon system**: Pickaxe, Pistol, Shotgun, Assault Rifle, SMG, Sniper with rarity tiers
- **Building system**: Walls, floors, ramps, roofs with material costs
- **Full HUD**: Health/shield bars, weapon hotbar, minimap, materials, kill feed
- **Bot AI**: Computer-controlled opponents with movement, combat, and building
- **VSync and frame timing**: 60 FPS target with proper frame pacing
- **Voxel-based models**: Procedural character, weapon, and environment models
- **VMSVGA GPU support**: Hardware-accelerated 2D rendering on QEMU/VirtualBox

## Technical Specifications

| Component | Specification |
|-----------|---------------|
| Target | `x86_64-unknown-none` |
| Bootloader | Limine v8.x |
| Memory | 64MB heap (Talc allocator) |
| Resolution | 1024x768x32 |
| Graphics | Software rasterization, tile-based parallel rendering |
| GPU Accel | VMSVGA 2D (QEMU `-vga vmware`, VirtualBox VMSVGA) |
| Networking | E1000 NIC driver + smoltcp UDP |
| Tick Rate | 20Hz server, 60Hz client render target |
| VSync | VGA vertical retrace + TSC frame timing |
| Max Players | 100 |
| Bot AI | Pathfinding, combat, building |

## Project Structure

```
fortnite-os/
├── Cargo.toml                    # Workspace root
├── .cargo/config.toml            # Build configuration
├── rust-toolchain.toml           # Nightly toolchain specification
├── limine.conf                   # Bootloader configuration
├── GNUmakefile                   # Build system
├── kernel/                       # Main kernel crate
│   ├── Cargo.toml
│   ├── linker-x86_64.ld          # Linker script
│   └── src/
│       ├── main.rs               # Entry point, main loop
│       ├── boot.rs               # Limine requests
│       ├── memory/
│       │   ├── allocator.rs      # Talc heap allocator
│       │   ├── dma.rs            # DMA memory for NIC
│       │   └── paging.rs         # MMIO page mapping
│       ├── drivers/
│       │   ├── serial.rs         # COM1 debug output
│       │   ├── pci.rs            # PCI enumeration
│       │   ├── e1000/            # Intel E1000 NIC driver
│       │   └── vmsvga/           # VMware SVGA GPU driver
│       │       ├── mod.rs        # Driver core
│       │       ├── regs.rs       # Register definitions
│       │       └── fifo.rs       # Command FIFO
│       ├── net/
│       │   ├── device.rs         # smoltcp Device trait
│       │   ├── stack.rs          # Network interface wrapper
│       │   └── protocol.rs       # Game protocol handler
│       ├── graphics/
│       │   ├── framebuffer.rs    # Limine framebuffer wrapper
│       │   ├── zbuffer.rs        # Depth buffer
│       │   ├── rasterizer.rs     # Triangle rasterization
│       │   ├── tiles.rs          # Tile-based rendering
│       │   ├── pipeline.rs       # Vertex transformation
│       │   ├── culling.rs        # Frustum and distance culling
│       │   ├── vsync.rs          # VSync and frame timing
│       │   ├── gpu.rs            # GPU backend abstraction
│       │   └── gpu_batch.rs      # GPU batched rendering
│       ├── game/
│       │   ├── world.rs          # Game state management
│       │   ├── player.rs         # Player entity
│       │   ├── input.rs          # Keyboard/mouse input (PS/2)
│       │   ├── storm.rs          # Zone/storm system
│       │   ├── bus.rs            # Battle bus
│       │   ├── building.rs       # Building system
│       │   ├── weapon.rs         # Weapon types and stats
│       │   ├── inventory.rs      # Player inventory
│       │   ├── combat.rs         # Hitscan and damage
│       │   ├── loot.rs           # Loot spawns and drops
│       │   ├── bot.rs            # Bot AI system
│       │   └── map.rs            # Map POIs and vegetation
│       ├── ui/
│       │   ├── main_menu.rs      # Title screen
│       │   ├── fortnite_lobby.rs # Party lobby
│       │   ├── game_ui.rs        # In-game HUD
│       │   └── customization.rs  # Character customization
│       └── smp/
│           ├── scheduler.rs      # Core assignment
│           └── sync.rs           # Spinlocks, barriers
├── renderer/                     # Software renderer library
│   └── src/
│       ├── lib.rs
│       ├── vertex.rs             # Vertex format
│       ├── math.rs               # Math utilities
│       ├── mesh.rs               # Procedural meshes
│       ├── voxel.rs              # Voxel model system
│       ├── voxel_models.rs       # Character, weapon, vehicle models
│       └── map_mesh.rs           # Map structure meshes
├── protocol/                     # Network protocol crate
│   └── src/
│       ├── lib.rs
│       ├── packets.rs            # Packet definitions
│       └── codec.rs              # Binary serialization
└── scripts/
    └── run-qemu.sh               # QEMU runner script
```

## Prerequisites

### macOS
```bash
# Install Rust nightly with required components
rustup install nightly
rustup component add rust-src llvm-tools-preview --toolchain nightly

# Install build dependencies
brew install xorriso qemu

# Clone limine bootloader (done automatically by make)
git clone https://github.com/limine-bootloader/limine.git --branch=v8.x-binary --depth=1
make -C limine
```

### Linux (Debian/Ubuntu)
```bash
# Install Rust nightly
rustup install nightly
rustup component add rust-src llvm-tools-preview --toolchain nightly

# Install build dependencies
sudo apt install xorriso qemu-system-x86 make git
```

## Building

```bash
# Build the kernel (creates target/x86_64-unknown-none/release/kernel)
cargo build --release

# Build the bootable ISO image
make image.iso

# Or just run (builds everything automatically)
make run
```

## Running

### Single Instance (Development)
```bash
make run
```
This boots the OS in QEMU with:
- 4 CPU cores (1 game logic + 3 rasterizers)
- 512MB RAM
- VMSVGA graphics (hardware 2D acceleration)
- E1000 network card
- Serial output to terminal

### Manual QEMU Command
```bash
qemu-system-x86_64 \
  -cdrom image.iso \
  -m 512M \
  -vga vmware \
  -smp 4 \
  -serial stdio \
  -device e1000,netdev=net0 \
  -netdev user,id=net0
```

### Networked Instances (Multiplayer Testing)
```bash
# Terminal 1 - Start server
make run-network

# Terminal 2 - Start client
make run-network-client
```

### QEMU Controls
- **Ctrl+A, X** - Quit QEMU
- **Ctrl+A, C** - QEMU monitor console

## Controls (In-Game)

### Movement
| Input | Action |
|-------|--------|
| W/A/S/D | Move forward/left/backward/right |
| Mouse | Look around (FPS-style camera) |
| Space | Jump / Exit Battle Bus / Deploy Glider |
| Ctrl | Crouch |

### Combat
| Input | Action |
|-------|--------|
| Left Click / Shift | Fire weapon |
| R | Reload |
| 1 | Select Pickaxe |
| 2-6 | Select weapon slot 1-5 |

### Building
| Input | Action |
|-------|--------|
| B / Right Click | Build wall |

### Interaction
| Input | Action |
|-------|--------|
| E | Pick up loot |
| Tab | Toggle minimap (in menus) |
| Escape | Return to menu / Quit |

### Gliding
| Input | Action |
|-------|--------|
| W (hold) | Dive faster (more speed, faster descent) |
| Normal | Glide (slower descent, less speed) |

## Architecture

### Core Assignment (SMP)

| Core | Role | Description |
|------|------|-------------|
| 0 | Game Logic | Main loop, input, physics, bot AI |
| 1-3 | Rasterizers | Tile-based parallel rendering |

Network polling is integrated into the main loop on core 0.

### Rendering Pipeline

1. **Distance Culling**: Skip objects beyond 500 units from camera
2. **Vertex Transformation**: Model → World → View → Clip → NDC → Screen
3. **Backface Culling**: Reject triangles facing away from camera (CCW winding)
4. **Triangle Binning**: Assign triangles to 64x64 pixel tiles
5. **Parallel Rasterization**: 3 cores process tiles concurrently
6. **Z-Buffer Testing**: Per-pixel depth comparison
7. **Shading**: Gouraud shading with per-vertex colors and directional light

### Game Mechanics

**Battle Bus**: Players start on a bus flying across the map. Press Space to drop.

**Skydiving**:
- Normal fall: 70 units/sec
- Dive (hold W): 120 units/sec
- Glider auto-deploys at 50m altitude

**Gliding**:
- Normal glide: 25 units/sec descent, 20 units/sec horizontal
- Dive (hold W): 45 units/sec descent, 35 units/sec horizontal

**Storm**: Circular safe zone that shrinks over time. Damages players outside (1 HP/sec initially, scaling up).

**Building**: Costs 10 wood per wall. Walls have 150 HP.

**Combat**: Hitscan weapons with headshot multipliers (1.5x-2.5x depending on weapon).

### Weapon System

| Weapon | Damage | Fire Rate | Magazine | Range |
|--------|--------|-----------|----------|-------|
| Pickaxe | 20 | 1.0/s | ∞ | 2m |
| Pistol | 23 | 6.75/s | 16 | 50m |
| Shotgun | 90 | 0.7/s | 5 | 15m |
| Assault Rifle | 30 | 5.5/s | 30 | 100m |
| SMG | 17 | 12.0/s | 30 | 40m |
| Sniper | 100 | 0.33/s | 1 | 500m |

**Rarity tiers**: Common (1.0x) → Uncommon (1.05x) → Rare (1.10x) → Epic (1.15x) → Legendary (1.21x) damage multiplier

### Network Protocol

- **Transport**: UDP over IPv4
- **Port**: 5000
- **Tick Rate**: 20Hz server updates

#### Packet Types
| Type | Description | Size |
|------|-------------|------|
| JoinRequest | Client requests to join | 2 + name length |
| JoinResponse | Server assigns player ID | 2 bytes |
| ClientInput | Player input state | 16 bytes |
| WorldStateDelta | Server state update | 17 + 24*players |

#### PlayerState Structure (24 bytes)
```rust
struct PlayerState {
    player_id: u8,
    x: i32,           // Fixed-point 16.16
    y: i32,
    z: i32,
    yaw: i16,         // Degrees * 100
    pitch: i16,
    health: u8,
    weapon_id: u8,
    state: u8,        // Flags (alive, jumping, etc.)
    _padding: u8,
}
```

## Performance Targets

| Component | Budget | Notes |
|-----------|--------|-------|
| Triangle binning | 2ms | Single-threaded with culling |
| Rasterization | 10ms | 3 cores parallel |
| Buffer swap | 2ms | VMSVGA or direct framebuffer |
| Game logic | 3ms | Physics, input, bot AI |
| Network | 2ms | Dedicated core |
| **Total** | 16ms | 60 FPS target with VSync |

### Actual Performance
- **Menu screens**: 60 FPS (VSync limited)
- **In-game (3200 terrain triangles + entities)**: 25-60 FPS depending on scene complexity
- **Distance culling**: 500 unit radius to reduce draw calls

### Bandwidth (100 players)
- Per client: 100 × 24 bytes × 20Hz = ~48 KB/s
- Total server: ~4.8 MB/s (well under 100 Mbps)

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| limine | 0.5 | Bootloader protocol |
| x86_64 | 0.15 | CPU instructions |
| spin | 0.9 | Spinlock mutexes |
| talc | 4.4 | Heap allocator |
| smoltcp | 0.12 | TCP/IP stack |
| glam | 0.29 | Math (vectors, matrices) |
| libm | 0.2 | Math functions (sin, cos, etc.) |

## Debugging

### Serial Output
All debug messages go to COM1 (serial stdio in QEMU):
```rust
serial_println!("Debug message: {}", value);
```

### QEMU Logs
Interrupt and CPU reset logs are written to `qemu.log`:
```bash
tail -f qemu.log
```

### Common Issues

**Kernel doesn't boot**
- Check limine.conf path matches kernel location
- Verify linker script has correct `.requests` section

**No network connectivity**
- Ensure E1000 is detected in PCI scan
- Check smoltcp features enabled in Cargo.toml
- Verify ARP resolution works (poll network stack during init)
- DMA memory must be above 16MB for reliable operation

**Graphics not rendering**
- Verify framebuffer response from Limine
- Check z-buffer is initialized

## License

This project is provided for educational purposes.

## Acknowledgments

- [Limine Bootloader](https://github.com/limine-bootloader/limine)
- [smoltcp TCP/IP Stack](https://github.com/smoltcp-rs/smoltcp)
- [OSDev Wiki](https://wiki.osdev.org/)
