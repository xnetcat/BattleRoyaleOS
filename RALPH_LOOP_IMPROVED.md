# Improved Ralph Loop Prompt: BattleRoyaleOS Integration & Completion

## Command
```bash
/ralph-loop "$(cat << 'EOF'

# BATTLEROYALE-OS: SYSTEM INTEGRATION PROJECT

## PROJECT CONTEXT - READ THIS CAREFULLY

This is a **bare-metal Rust unikernel operating system** (NOT a game engine).
The codebase has many isolated modules that were NEVER INTEGRATED together.

**Current State**: Walking simulator. You can boot, ride the bus, jump out, and walk around. That's it.

**The Problem**: Systems exist but aren't wired together:
- Combat code exists but fire input is ignored
- Weapon code is perfect but never called
- Building code works but players have 0 materials
- Loot system complete but nothing spawns
- Storm logic correct but never damages anyone

**Your Job**: INTEGRATE the systems to make a playable game.

## TECH STACK (FIXED - DO NOT CHANGE)
- Rust nightly, `no_std`, x86_64 bare-metal
- Custom software rasterizer (NO GPU, no external graphics)
- smoltcp for networking
- Limine bootloader
- Build: `make` | Run: `make run`

## CRITICAL RULES
- NO TODOS, NO PLACEHOLDERS, NO STUBS
- COMPILE CHECK after EVERY change: `make`
- DO NOT CHANGE ARCHITECTURE - bare-metal, no new crates
- INTEGRATE existing code - don't rewrite working systems
- PRESERVE what works: rendering, state machine, physics

## PHASE 1: COMBAT INTEGRATION (CRITICAL)

### Problem: Fire input is completely ignored

**Files to modify**: `kernel/src/game/player.rs`, `kernel/src/game/world.rs`

**Current flow**:
1. Input sets `input.fire = true` ✅
2. `apply_ground_input()` receives input ✅
3. **Fire is never checked** ❌
4. `hitscan()` in combat.rs is never called ❌
5. Weapons never fire ❌

**Fix needed in `player.rs::apply_ground_input()`**:
```rust
// Add fire handling - this is MISSING from the function
if input.fire {
    // Get current weapon from inventory
    if let Some(weapon) = &mut self.inventory.weapons[self.inventory.selected_slot] {
        // Check if weapon can fire (ammo, cooldown)
        if weapon.can_fire() {
            weapon.fire();  // Consume ammo, reset cooldown
            // Return fire event to world for hitscan processing
        }
    }
}
```

**Then in `world.rs::update()`**:
- Process fire events from players
- Call `combat::hitscan()` with camera direction
- Apply damage to hit players
- Trigger hit markers, damage numbers, kill feed

### Verify Combat Works:
1. Shoot at bot/player
2. Damage numbers appear
3. Health decreases
4. Elimination triggers on 0 HP

**COMPILE CHECK** - `make` must succeed

---

## PHASE 2: STARTING MATERIALS

### Problem: Players spawn with 0 materials, can't build

**Files to modify**: `kernel/src/game/inventory.rs` or `kernel/src/game/world.rs`

**Current code**:
```rust
// Inventory::new() or player spawn
materials: Materials::default()  // Returns 0, 0, 0
```

**Fix Option A - Change default**:
```rust
impl Default for Materials {
    fn default() -> Self {
        Self { wood: 100, brick: 50, metal: 50 }
    }
}
```

**Fix Option B - Set on spawn**:
```rust
// In player spawn logic
player.inventory.materials = Materials { wood: 100, brick: 50, metal: 50 };
```

### Verify Building Works:
1. Press B to enter build mode
2. Select wall/floor/ramp
3. Place structure
4. Material count decreases
5. Structure appears in world

**COMPILE CHECK** - `make` must succeed

---

## PHASE 3: LOOT SPAWNING & PICKUP

### Problem: Loot system exists but nothing spawns

**Files to modify**: `kernel/src/game/loot.rs`, `kernel/src/game/world.rs`, `kernel/src/game/map.rs`

**Fix Part 1 - Spawn loot on match start**:
In world initialization or when entering InGame state:
```rust
fn spawn_world_loot(&mut self) {
    // For each POI
    for poi in &self.map.pois {
        // Spawn chests at chest_locations
        for chest_pos in &poi.chest_spawns {
            self.loot_items.push(LootItem::chest(*chest_pos));
        }
        // Spawn floor loot randomly
        for _ in 0..5 {
            let pos = random_pos_in_poi(poi);
            let weapon = random_weapon();
            self.loot_items.push(LootItem::weapon(pos, weapon));
        }
    }
}
```

**Fix Part 2 - Pickup input**:
In player input processing:
```rust
if input.interact {  // E key
    // Find nearest loot within pickup range (2.5 units)
    if let Some(loot) = self.find_nearest_loot(player.position, 2.5) {
        // Add to inventory
        match loot.kind {
            LootKind::Weapon(w) => player.inventory.add_weapon(w),
            LootKind::Ammo(a, count) => player.inventory.add_ammo(a, count),
            LootKind::Materials(m) => player.inventory.add_materials(m),
            // etc
        }
        // Remove from world
        self.remove_loot(loot.id);
    }
}
```

### Verify Loot Works:
1. Land near POI
2. See weapon/chest on ground
3. Press E to pickup
4. Weapon appears in inventory
5. Ammo count updates

**COMPILE CHECK** - `make` must succeed

---

## PHASE 4: HARVESTING MATERIALS

### Problem: No way to get more materials after spawn

**Files to modify**: `kernel/src/game/player.rs`, `kernel/src/game/world.rs`

Pickaxe is weapon slot 0. When equipped and fire pressed on a harvestable object:
```rust
// In fire handling, check if using pickaxe
if let Some(Weapon::Pickaxe) = current_weapon {
    // Raycast for harvestable objects (trees, rocks, builds, cars)
    if let Some(hit) = raycast_harvestables(player.position, player.look_dir) {
        // Damage the object
        hit.object.take_damage(pickaxe_damage);
        // Give materials
        player.inventory.materials.wood += hit.object.material_yield();
        // Play harvest effect
        if hit.object.health <= 0 {
            self.destroy_object(hit.object);
        }
    }
}
```

### Verify Harvesting Works:
1. Equip pickaxe (slot 1)
2. Hit tree
3. Wood count increases
4. Tree eventually breaks

**COMPILE CHECK** - `make` must succeed

---

## PHASE 5: DEATH & ELIMINATION

### Problem: No way to die or win

**Files to modify**: `kernel/src/game/player.rs`, `kernel/src/game/world.rs`, `kernel/src/game/state.rs`

**Fix Part 1 - Death check**:
```rust
// In player.take_damage() or world.update()
if player.health <= 0 && player.phase == PlayerPhase::Grounded {
    player.phase = PlayerPhase::Eliminated;
    // Drop inventory as loot
    self.spawn_death_loot(player.position, &player.inventory);
    // Add to kill feed
    self.kill_feed.push(KillFeedEntry { killer, victim: player.id });
    // Increment killer's kill count
    if let Some(killer_id) = killer {
        self.players[killer_id].kills += 1;
    }
}
```

**Fix Part 2 - Victory check**:
```rust
// In world.update()
let alive_count = self.players.iter().filter(|p| p.phase != PlayerPhase::Eliminated).count();
if alive_count <= 1 {
    // Trigger victory for remaining player
    return GameEvent::Victory(winner_id);
}
```

**Fix Part 3 - Storm damage actually kills**:
Storm damage is already being applied but ensure health check triggers death.

### Verify Death Works:
1. Take enough damage (from storm or bot)
2. Screen shows "You placed #X"
3. Can spectate remaining players
4. OR if last one: Victory screen

**COMPILE CHECK** - `make` must succeed

---

## PHASE 6: WEAPON HOTBAR & SWITCHING

### Problem: Can switch weapons in code but no input/UI

**Files to modify**: `kernel/src/game/input.rs`, `kernel/src/game/player.rs`, `kernel/src/ui/game_ui.rs`

**Fix Part 1 - Number key input**:
```rust
// In input handling
match scancode {
    0x02 => input.select_slot = Some(0),  // 1 key
    0x03 => input.select_slot = Some(1),  // 2 key
    0x04 => input.select_slot = Some(2),  // 3 key
    0x05 => input.select_slot = Some(3),  // 4 key
    0x06 => input.select_slot = Some(4),  // 5 key
    // ...
}
```

**Fix Part 2 - Apply weapon switch**:
```rust
// In player input processing
if let Some(slot) = input.select_slot {
    self.inventory.selected_slot = slot;
}
```

**Fix Part 3 - UI shows hotbar**:
Draw 5 slots at bottom, highlight selected, show weapon icon/name.

### Verify Weapon Switching Works:
1. Pick up multiple weapons
2. Press 1-5 to switch
3. UI highlights selected slot
4. Correct weapon is equipped

**COMPILE CHECK** - `make` must succeed

---

## PHASE 7: BOT AI (Single Player)

### Problem: Need enemies to shoot at

**Files to modify**: `kernel/src/game/world.rs` or new `kernel/src/game/bot.rs`

Spawn bots on match start:
```rust
fn spawn_bots(&mut self, count: usize) {
    for i in 0..count {
        let spawn_pos = random_map_position();
        let bot = Player::new_bot(i as u8, spawn_pos);
        self.players.push(bot);
    }
}
```

Bot behavior per tick:
```rust
fn update_bot(&mut self, bot: &mut Player, dt: f32) {
    // Simple state machine: Wander, Chase, Attack, Flee
    match bot.ai_state {
        AiState::Wander => {
            // Move toward random waypoint
            // If see player, switch to Chase
        }
        AiState::Chase => {
            // Move toward target player
            // If in range, switch to Attack
        }
        AiState::Attack => {
            // Aim at player, fire weapon
            // If low health, Flee
        }
        AiState::Flee => {
            // Run away, seek cover/healing
        }
    }
    // Always move toward safe zone if outside storm
}
```

### Verify Bots Work:
1. Start match
2. Bots spawn around map
3. Bots move around
4. Bots shoot at you
5. You can eliminate bots
6. Bots can eliminate you

**COMPILE CHECK** - `make` must succeed

---

## PHASE 8: POLISH & CLEANUP

1. **Kill feed updates** - Show eliminations in top-left
2. **Damage numbers** - Floating text on hits
3. **Hit markers** - Crosshair flash on hit
4. **Low health warning** - Red screen edge when HP < 25
5. **Reload mechanic** - R key to reload, animation/time
6. **Storm visual** - Purple tint when inside storm

### Final cleanup:
```bash
grep -rn "TODO\|FIXME\|placeholder\|unimplemented\|todo!" kernel/src/
```
Fix ALL instances found.

**FINAL COMPILE CHECK** - `make` must succeed with no warnings

---

## COMPLETION CRITERIA

The project is COMPLETE when ALL of the following work in a playthrough:

1. ✅ Boot to menu and start match
2. ✅ Ride bus and jump out
3. ✅ Land and move around
4. ✅ Have starting materials (100 wood, 50 brick, 50 metal)
5. ✅ Build walls, floors, ramps
6. ✅ Find and pickup weapons from ground/chests
7. ✅ Switch weapons with 1-5 keys
8. ✅ Shoot weapons (bullets hit, damage applies)
9. ✅ Harvest materials from trees/objects
10. ✅ Take damage from storm when outside zone
11. ✅ Die when health reaches 0
12. ✅ Bots spawn, move, and fight
13. ✅ Victory screen when last alive
14. ✅ `make` builds with no errors
15. ✅ No TODOs/FIXMEs/placeholders in code

When ALL criteria met: <promise>BATTLEROYALE_OS_PLAYABLE</promise>

---

## ITERATION STRATEGY

Each iteration:
1. Pick ONE broken system from the list
2. Read the existing code carefully
3. Find where integration is missing
4. Add the minimal code to wire systems together
5. Run `make` to verify compile
6. Test in QEMU if possible
7. Move to next system

**Priority order**:
1. Combat (can't play without shooting)
2. Starting materials (can't build otherwise)
3. Death/victory (no win condition)
4. Loot spawning (need weapons)
5. Bots (need enemies)
6. Polish (UI, effects)

---

## IF STUCK AFTER 35 ITERATIONS

Document:
- Which systems are now integrated
- Which systems remain broken
- What specific code change is blocking
- Error messages from `make`

Then output: <promise>BATTLEROYALE_OS_BLOCKED</promise>

---

## KEY FILES QUICK REFERENCE

| Purpose | File |
|---------|------|
| Entry/game loop | `kernel/src/main.rs` |
| Player & input | `kernel/src/game/player.rs` |
| World state | `kernel/src/game/world.rs` |
| Combat/hitscan | `kernel/src/game/combat.rs` |
| Weapons | `kernel/src/game/weapon.rs` |
| Inventory | `kernel/src/game/inventory.rs` |
| Building | `kernel/src/game/building.rs` |
| Loot | `kernel/src/game/loot.rs` |
| Storm | `kernel/src/game/storm.rs` |
| Game states | `kernel/src/game/state.rs` |
| Input handling | `kernel/src/game/input.rs` |
| Game UI | `kernel/src/ui/game_ui.rs` |
| Map/POIs | `kernel/src/game/map.rs` |

EOF
)" --max-iterations 40 --completion-promise "BATTLEROYALE_OS_PLAYABLE"
```

---

## Why This Prompt Is Correct

### Previous Problems:
1. **Assumed systems worked** - They don't, they're isolated
2. **Focused on placeholders** - Real issue is missing integration
3. **Underestimated work** - 25 iterations too few for integration

### This Prompt:
1. **Correctly identifies the problem** - Systems exist but aren't wired together
2. **Provides specific integration points** - Exact files and functions
3. **Prioritizes correctly** - Combat first, then building, then game loop
4. **Realistic iteration count** - 40 for major integration work
5. **Clear verification steps** - How to test each fix works

### Estimated Effort Per Phase:
| Phase | Iterations | Complexity |
|-------|------------|------------|
| 1. Combat | 5-8 | High - multiple files |
| 2. Materials | 1-2 | Easy - one line change |
| 3. Loot | 5-8 | Medium - spawn + pickup |
| 4. Harvesting | 3-5 | Medium |
| 5. Death/Victory | 3-5 | Medium |
| 6. Weapon Hotbar | 2-3 | Easy |
| 7. Bots | 8-12 | High - AI logic |
| 8. Polish | 3-5 | Easy |

**Total: 30-48 iterations**

---

## Quick Test: Is Combat Working?

After Phase 1, you can verify with this test:
1. Start match with bots (or second player)
2. Land near an enemy
3. Press fire (left click or Shift)
4. Expected: Damage numbers appear, enemy health decreases
5. If nothing happens: Combat still broken, check:
   - Is `input.fire` being set?
   - Is `apply_ground_input()` checking fire?
   - Is `hitscan()` being called?
   - Is damage being applied?
