# BattleRoyaleOS

A bare-metal Rust unikernel operating system designed to run a 100-player Battle Royale game on a local network. The OS and game compile into a single binary targeting x86_64 with software-rendered 3D graphics.

## Features

- **Bare-metal execution**: No underlying OS, boots directly via Limine bootloader
- **Software 3D rendering**: Custom rasterizer with z-buffering and Gouraud shading
- **Multicore support**: SMP with dedicated cores for game logic, rendering, and networking
- **Network stack**: E1000 NIC driver with smoltcp TCP/IP stack
- **Battle Royale mechanics**: 100 players, storm system, building, battle bus

## Technical Specifications

| Component | Specification |
|-----------|---------------|
| Target | `x86_64-unknown-none` |
| Bootloader | Limine v8.x |
| Memory | 64MB heap (Talc allocator) |
| Graphics | Software rasterization, tile-based parallel rendering |
| Networking | E1000 NIC driver + smoltcp UDP |
| Tick Rate | 20Hz server, 60Hz client render target |
| Max Players | 100 |

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
│       │   └── dma.rs            # DMA memory for NIC
│       ├── drivers/
│       │   ├── serial.rs         # COM1 debug output
│       │   ├── pci.rs            # PCI enumeration
│       │   └── e1000/            # Intel E1000 NIC driver
│       │       ├── mod.rs        # Driver core
│       │       ├── regs.rs       # Register definitions
│       │       ├── descriptors.rs # TX/RX descriptors
│       │       └── ring.rs       # Ring buffer management
│       ├── net/
│       │   ├── device.rs         # smoltcp Device trait
│       │   ├── stack.rs          # Network interface wrapper
│       │   └── protocol.rs       # Game protocol handler
│       ├── graphics/
│       │   ├── framebuffer.rs    # Limine framebuffer wrapper
│       │   ├── zbuffer.rs        # Depth buffer
│       │   ├── rasterizer.rs     # Triangle rasterization
│       │   ├── tiles.rs          # Tile-based rendering
│       │   └── pipeline.rs       # Vertex transformation
│       ├── game/
│       │   ├── world.rs          # Game state management
│       │   ├── player.rs         # Player entity
│       │   ├── input.rs          # Keyboard input
│       │   ├── storm.rs          # Zone/storm system
│       │   ├── bus.rs            # Battle bus
│       │   └── building.rs       # Building system
│       └── smp/
│           ├── scheduler.rs      # Core assignment
│           └── sync.rs           # Spinlocks, barriers
├── renderer/                     # Software renderer library
│   └── src/
│       ├── lib.rs
│       ├── vertex.rs             # Vertex format
│       ├── math.rs               # Math utilities
│       └── mesh.rs               # Procedural meshes
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
- 5 CPU cores
- 512MB RAM
- E1000 network card
- Serial output to terminal

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

| Key | Action |
|-----|--------|
| W/A/S/D | Move |
| Space | Jump / Exit Bus |
| Ctrl | Crouch |
| B | Build Wall |
| Shift | Fire |
| Escape | Quit |

## Architecture

### Core Assignment (SMP)

| Core | Role | Description |
|------|------|-------------|
| 0 | Game Logic | Main loop, input, frame orchestration |
| 1-3 | Rasterizers | Tile-based parallel rendering |
| 4 | Network | E1000 polling, packet processing |

### Rendering Pipeline

1. **Vertex Transformation**: Model → World → View → Clip → NDC → Screen
2. **Backface Culling**: Reject triangles facing away from camera
3. **Triangle Binning**: Assign triangles to 64x64 pixel tiles
4. **Rasterization**: Scanline algorithm with z-buffer testing
5. **Shading**: Gouraud shading with per-vertex colors

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
| Triangle binning | 2ms | Single-threaded |
| Rasterization | 20ms | 3 cores parallel |
| Buffer swap | 2ms | Direct framebuffer |
| Game logic | 5ms | Physics, input |
| Network | 4ms | Dedicated core |
| **Total** | 33ms | ~30 FPS target |

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
