# BattleRoyaleOS Development Guidelines

## Code Quality Standards

### No Placeholders or TODOs
- **Every feature must be fully implemented** - no `TODO`, `FIXME`, `// in production`, or placeholder comments
- Code must be production-ready at all times
- If a feature is complex, break it into smaller fully-implemented pieces
- No "stub" implementations - either implement fully or don't implement at all

### Implementation Philosophy
- Take the time needed to implement things correctly
- Don't cut corners for speed
- Every line of code should serve a purpose
- All edge cases must be handled

## Testing Requirements

### Component Tests
Every kernel component must have corresponding tests:

1. **Memory Subsystem** (`kernel/src/memory/`)
   - Heap allocator stress tests
   - DMA allocator physical address validation
   - Memory leak detection

2. **Drivers** (`kernel/src/drivers/`)
   - Serial output verification
   - PCI device enumeration validation
   - E1000 transmit/receive tests

3. **Graphics** (`kernel/src/graphics/`)
   - Framebuffer write tests
   - Z-buffer depth ordering
   - Rasterizer triangle coverage
   - Pipeline transformation accuracy

4. **Game Logic** (`kernel/src/game/`)
   - Player movement physics
   - World state serialization
   - Input handling response

5. **Network** (`kernel/src/net/`)
   - Packet encoding/decoding
   - Protocol message flow
   - Connection handling

### E2E Testing via Serial Bus

All E2E tests use the serial port for communication with a Python test harness.

#### Serial Protocol
- Test commands are sent to the kernel via serial input
- Results are returned via serial output
- Format: `TEST:<test_name>:<args>` → `RESULT:<test_name>:<pass|fail>:<details>`

#### Python Test Framework (`tests/`)
```
tests/
├── conftest.py           # pytest fixtures for QEMU
├── test_boot.py          # Kernel boot validation
├── test_memory.py        # Memory subsystem tests
├── test_graphics.py      # Graphics pipeline tests
├── test_network.py       # Network stack tests
├── test_game.py          # Game logic tests
└── e2e/
    ├── test_full_game.py # Complete game flow
    └── test_multiplayer.py # Multi-instance tests
```

#### Running Tests
```bash
# Run all tests
make test

# Run specific test suite
make test-memory
make test-graphics
make test-network

# Run E2E tests
make test-e2e
```

### Validation Requirements

1. **Boot Validation**
   - Kernel must print boot messages in correct order
   - All subsystems must initialize without errors
   - Memory must be properly allocated

2. **Graphics Validation**
   - Frame must render within time budget
   - No visual artifacts (z-fighting, tearing)
   - Correct color output

3. **Network Validation**
   - Packets must be correctly formed
   - Two instances must communicate
   - Latency within acceptable bounds

4. **Game Validation**
   - Player physics correct
   - State synchronization accurate
   - Win conditions detected

## Architecture Principles

### Memory Safety
- Use Rust's ownership system properly
- No unsafe code without clear justification
- All unsafe blocks must have safety comments

### Performance
- Target 30 FPS minimum
- Profile before optimizing
- Document performance-critical sections

### Error Handling
- All errors must be handled explicitly
- No panics in production code (except early boot)
- Errors must be reported via serial

## Commit Standards

- Each commit must compile and run
- Commit messages describe what and why
- No broken intermediate states
