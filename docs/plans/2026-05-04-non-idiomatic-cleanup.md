# Non-Idiomatic Code Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or `/team-feature` to implement this plan task-by-task, per the Execution Strategy below. Steps use checkbox (`- [ ]`) syntax — these are **persistent durable state**, not visual decoration. The executor edits the plan file in place: `- [ ]` → `- [x]` the instant a step verifies, before moving on. On resume (new session, crash, takeover), the executor scans existing `- [x]` marks and skips them — these steps are NOT redone. TodoWrite mirrors this state in-session; the plan file is the source of truth across sessions.

**Goal:** Apply the non-idiomatic-code findings from the workspace review — fix one correctness bug in terrain generation, add Bevy change-detection guards to per-frame render and UI systems, tighten over-broad query/resource access, and replace a handful of unidiomatic patterns (glob exports, double negations, manual loops where iterator combinators fit).

**Architecture:** Each task targets a small, independent slice of the codebase, almost always one or two files. Most tasks are pure refactors with the existing test suite as the safety net. The one correctness fix (`terrain_gen.rs` Tile-overwrite) gets a dedicated regression test before the change goes in.

**Tech Stack:** Rust 2024, Bevy 0.18, `cargo nextest` for tests, `just` recipes (`just test`, `just lint`, `just fmt`) for the standard CI loop. The workspace uses crate names `fungai_core`, `fungai_world`, `fungai_growth`, etc. (the project README/CLAUDE.md still references the legacy `shroom_*` names — leave that alone for this plan).

## Execution Strategy

**Subagents** — default — no spec override. Tasks are independent file-level edits that fit the subagent-per-task model with two-stage review.

## Task Dependency Graph

- Task 1 [AFK]: depends on `none` → batch 1
- Task 2 [AFK]: depends on `none` → batch 1 (parallel with Task 1)
- Task 3 [AFK]: depends on `none` → batch 1 (parallel)
- Task 4 [AFK]: depends on `none` → batch 1 (parallel)
- Task 5 [AFK]: depends on `none` → batch 1 (parallel)
- Task 6 [AFK]: depends on `none` → batch 1 (parallel)
- Task 7 [AFK]: depends on `none` → batch 1 (parallel)
- Task 8 [AFK]: depends on `Task 2, Task 4` → batch 2 (Task 8 edits `data_layer.rs` (Task 2 also), `rival.rs` and `decay.rs` (Task 4 also); must land after them to avoid merge conflicts)
- Polish [AFK]: depends on `Task 1, Task 2, Task 3, Task 4, Task 5, Task 6, Task 7, Task 8` → batch 3

Tasks 1–7 touch disjoint files and batch together safely. Task 8 batches after Tasks 2 and 4 because it edits the same three files (`crates/render/src/data_layer.rs`, `crates/ai/src/rival.rs`, `crates/growth/src/decay.rs`). The Polish step waits for the full set to land.

## Agent Assignments

- Task 1: terrain_gen Tile-overwrite fix → rust-engineer (Rust + Bevy)
- Task 2: entity_render change detection → rust-engineer (Rust + Bevy)
- Task 3: UI change-detection guards → rust-engineer (Rust + Bevy)
- Task 4: Resource/query tightening → rust-engineer (Rust + Bevy)
- Task 5: Control-flow idioms → rust-engineer (Rust)
- Task 6: Random selection idioms → rust-engineer (Rust)
- Task 7: Misc cleanup → rust-engineer (Rust)
- Task 8: Glob exports/imports → rust-engineer (Rust)
- Polish: post-implementation-polish → general-purpose

---

### Task 1: Fix `terrain_gen.rs` Tile-overwrite bug

When `terrain_generation` places fragments, decomposables, neutral fungi, and plant roots, it does `commands.entity(entity).insert(Tile { contents: ..., ..default() })`. `insert` *replaces* the whole component, so the random `nutrient_level`, `moisture`, and `terrain` set during the initial spawn loop are wiped to `Tile::default()` (which has `nutrient_level: 0.5`, `moisture: 0.5`) for every special tile. Fix by restructuring the system into three passes: (1) compute every hex's terrain/moisture/nutrient into a local map using RNG, (2) pick placements (which can now query terrain from the map), (3) spawn each tile once with both terrain and contents.

**Files:**
- Modify: `crates/world/src/terrain_gen.rs`
- Test: `crates/world/src/terrain_gen.rs` (add to existing `#[cfg(test)] mod tests` at line 234)

- [x] **Step 1: Write a failing regression test**

Add to the existing `#[cfg(test)] mod tests` in `crates/world/src/terrain_gen.rs`. Reuse the existing `test_app()` helper at line 239 — it already inserts `GridWorld`, `GameState`, `RegionStates`, and `TerrainSeed`. The test asserts that fragment tiles do NOT have `nutrient_level == 0.5 && moisture == 0.5` simultaneously — `Tile::default()` produces exactly that pair, while RNG-generated values almost never hit 0.5 on both axes (`nutrient_level = 0.2 + rng * 0.6`, `moisture = 0.3 + 0.5*y/H + rng * 0.2`, both clamped to [0,1]).

```rust
#[test]
fn fragment_tiles_preserve_rng_nutrient_and_moisture() {
    let mut app = test_app();
    app.add_systems(Startup, terrain_generation);
    app.update();

    let mut fragment_count = 0;
    for tile in app.world_mut().query::<&Tile>().iter(app.world()) {
        if matches!(tile.contents, Some(TileContents::Fragment(_))) {
            fragment_count += 1;
            assert!(
                (tile.nutrient_level - 0.5).abs() > f32::EPSILON
                    || (tile.moisture - 0.5).abs() > f32::EPSILON,
                "fragment tile reset to Tile::default() — nutrient {} moisture {}",
                tile.nutrient_level,
                tile.moisture,
            );
        }
    }
    assert!(fragment_count > 0, "expected at least one fragment");
}
```

- [x] **Step 2: Run the test and confirm it fails**

```
cargo nextest run -p fungai_world fragment_tiles_preserve_rng_nutrient_and_moisture
```
Expected: FAIL with the "fragment tile reset to Tile::default()" assertion firing.

- [x] **Step 3: Refactor `terrain_generation` into three passes (precompute → place → spawn)**

In `crates/world/src/terrain_gen.rs`, replace the body of `terrain_generation` (lines 36–221) with the three-pass structure below. Pass 1 runs the existing terrain/moisture/nutrient RNG draws into a local `HashMap` (no spawning yet). Pass 2 runs the existing placement RNG draws (`fragment_count`, `decomp_count`, `fungi_count`, `plant_count`, `bacteria_count` — preserve order to keep determinism) and writes into a placements map. Pass 3 spawns every tile once, reading both the precomputed data and the placements map.

```rust
use std::collections::HashMap;

let mut rng = StdRng::seed_from_u64(seed.0);
grid.width = MAP_WIDTH;
grid.height = MAP_HEIGHT;

// Pass 1: precompute terrain, moisture, nutrient_level for every hex.
let mut tile_data: HashMap<Hex, (TerrainType, f32, f32)> = HashMap::new();
for y in 0..MAP_HEIGHT {
    for x in 0..MAP_WIDTH {
        let hex = offset_to_hex(x, y);
        let depth_ratio = 1.0 - (y as f32 / MAP_HEIGHT as f32);
        let terrain = if y == MAP_HEIGHT - 1 {
            TerrainType::Surface
        } else if rng.random::<f32>() < 0.08 * depth_ratio {
            TerrainType::Rock
        } else if rng.random::<f32>() < 0.04 {
            TerrainType::Water
        } else if y > MAP_HEIGHT / 2 && rng.random::<f32>() < 0.03 {
            TerrainType::Root
        } else if rng.random::<f32>() < 0.02 * depth_ratio {
            TerrainType::Ruin
        } else if rng.random::<f32>() < 0.01 * depth_ratio {
            TerrainType::Toxic
        } else {
            TerrainType::Soil
        };
        let moisture = (0.3 + 0.5 * (y as f32 / MAP_HEIGHT as f32) + rng.random::<f32>() * 0.2)
            .clamp(0.0, 1.0);
        let nutrient_level = 0.2 + rng.random::<f32>() * 0.6;
        tile_data.insert(hex, (terrain, moisture, nutrient_level));
    }
}

// Pass 2: pick placements. random_soil_pos_pre_spawn filters by terrain via tile_data
// AND avoids hexes already claimed in `placements`.
let mut placements: HashMap<Hex, TileContents> = HashMap::new();
let mut terrain_overrides: HashMap<Hex, TerrainType> = HashMap::new();
let mut fragment_spawns: Vec<(Hex, FragmentId)> = Vec::new();
let mut fungus_spawns: Vec<(Hex, u32)> = Vec::new();
let mut plant_spawns: Vec<(Hex, u32)> = Vec::new();
let mut bacteria_spawns: Vec<Hex> = Vec::new();

let fragment_count = rng.random_range(3u32..=5);
game_state.fragments_total = fragment_count;
game_state.mushrooms_required = fragment_count;
for i in 0..fragment_count {
    let pos = random_soil_pos_pre_spawn(&tile_data, &mut rng, &placements);
    placements.insert(pos, TileContents::Fragment(FragmentId(i)));
    fragment_spawns.push((pos, FragmentId(i)));
}

let decomp_count = rng.random_range(3u32..=5);
for i in 0..decomp_count {
    let pos = random_soil_pos_pre_spawn(&tile_data, &mut rng, &placements);
    placements.insert(pos, TileContents::UniqueDecomposable(i));
}

let fungi_count = rng.random_range(2u32..=4);
for i in 0..fungi_count {
    let pos = random_soil_pos_pre_spawn(&tile_data, &mut rng, &placements);
    placements.insert(pos, TileContents::NeutralFungus(i));
    fungus_spawns.push((pos, i));
}

let plant_count = rng.random_range(3u32..=6);
for i in 0..plant_count {
    let x = rng.random_range(0..MAP_WIDTH);
    let y = rng.random_range(MAP_HEIGHT / 2..MAP_HEIGHT - 1);
    let pos = offset_to_hex(x, y);
    if placements.contains_key(&pos) {
        continue;
    }
    placements.insert(pos, TileContents::PlantRoot(i));
    terrain_overrides.insert(pos, TerrainType::Root);
    plant_spawns.push((pos, i));
}

let bacteria_count = rng.random_range(1u32..=2);
for _ in 0..bacteria_count {
    let pos = random_soil_pos_pre_spawn(&tile_data, &mut rng, &placements);
    bacteria_spawns.push(pos);
}

// Player and rival start positions: derived deterministically, override entire Tile.
let player_rid = region_states.create_region();
if let Some(state) = region_states.get_mut(player_rid) {
    state.nutrients = 100.0;
    state.energy = 20.0;
    state.specialization = Some(SpecializationType::Decomposer);
    state.target_specialization = Some(SpecializationType::Decomposer);
}
let player_start = offset_to_hex(MAP_WIDTH / 2, MAP_HEIGHT / 2);
let player_hexes: Vec<Hex> = player_start.range(2).collect();
let rival_id = RivalId(0);
let rival_start = offset_to_hex(MAP_WIDTH / 4, MAP_HEIGHT / 4);
let rival_hexes: Vec<Hex> = rival_start.range(1).collect();

// Pass 3: spawn every tile in one go. Player/rival hexes get their full override.
for y in 0..MAP_HEIGHT {
    for x in 0..MAP_WIDTH {
        let hex = offset_to_hex(x, y);
        let (mut terrain, moisture, nutrient_level) = tile_data[&hex];
        if let Some(override_terrain) = terrain_overrides.get(&hex).copied() {
            terrain = override_terrain;
        }
        let tile = if player_hexes.contains(&hex) {
            Tile {
                terrain: TerrainType::Soil,
                occupant: Occupant::Player(player_rid),
                nutrient_level: 0.8,
                moisture: 0.5,
                discovered: true,
                contents: None,
                biomass: 1.0,
                nutrient_gradient: Vec2::ZERO,
                priority_bias: Vec2::ZERO,
            }
        } else if rival_hexes.contains(&hex) {
            Tile {
                terrain: TerrainType::Soil,
                occupant: Occupant::Rival(rival_id),
                nutrient_level: 0.5,
                moisture: 0.5,
                discovered: false,
                contents: None,
                biomass: 1.5,
                nutrient_gradient: Vec2::ZERO,
                priority_bias: Vec2::ZERO,
            }
        } else {
            Tile {
                terrain,
                nutrient_level,
                moisture,
                contents: placements.remove(&hex),
                ..default()
            }
        };
        let entity = commands.spawn((GridPos(hex), tile)).id();
        grid.tiles.insert(hex, entity);
    }
}

// Pass 4: spawn agent entities (these are separate entities, not tile components).
for (pos, fid) in fragment_spawns {
    commands.spawn((GridPos(pos), FragmentAgent { fragment_id: fid, fused: false }));
}
for (pos, fungus_id) in fungus_spawns {
    commands.spawn((GridPos(pos), NeutralFungusAgent { fungus_id, merge_progress: 0.0 }));
}
for (pos, plant_id) in plant_spawns {
    commands.spawn((
        GridPos(pos),
        PlantRootAgent {
            plant_id,
            health: 1.0,
            trade_active: false,
            nutrient_intake: 0.0,
            sugar_output: 0.0,
            neglect_timer: 0,
        },
    ));
}
for pos in bacteria_spawns {
    commands.spawn((
        GridPos(pos),
        BacteriaColonyAgent { spread_timer: 0, spread_interval: 10 },
    ));
}
for neighbor in player_start.all_neighbors() {
    if grid.tiles.contains_key(&neighbor) {
        commands.spawn((GridPos(neighbor), HyphalTip { region_id: player_rid, age: 0 }));
    }
}
```

Then add the helper near `random_soil_pos` (which can stay or be deleted — no longer used):

```rust
fn random_soil_pos_pre_spawn(
    tile_data: &HashMap<Hex, (TerrainType, f32, f32)>,
    rng: &mut StdRng,
    placements: &HashMap<Hex, TileContents>,
) -> Hex {
    loop {
        let x = rng.random_range(1..MAP_WIDTH - 1);
        let y = rng.random_range(1..MAP_HEIGHT - 2);
        let hex = offset_to_hex(x, y);
        if placements.contains_key(&hex) {
            continue;
        }
        if let Some((TerrainType::Soil, _, _)) = tile_data.get(&hex) {
            return hex;
        }
    }
}
```

This is a strict improvement over the original `random_soil_pos` which never actually checked terrain. Note: RNG draw count for placement picks may now differ from the original (the rejection-sample loop iterates more on non-soil hexes). Existing tests don't depend on exact placement positions, but `places_fragments` (terrain_gen.rs:277) just asserts `(3..=5).contains(&fragment_count)` — still passes.

- [x] **Step 4: Run the regression test and confirm it passes**

```
cargo nextest run -p fungai_world fragment_tiles_preserve_rng_nutrient_and_moisture
```
Expected: PASS.

- [x] **Step 5: Run the full crate test suite and lint**

```
cargo nextest run -p fungai_world
just lint
```
Expected: all tests pass, no clippy warnings.

- [x] **Step 6: Commit**

```
git add crates/world/src/terrain_gen.rs
git commit -m "fix: preserve RNG nutrient/moisture on fragment/decomposable/fungus/plant tiles"
```

---

### Task 2: Change-detection guards in `entity_render.rs`

`tip_render_system`, `priority_arrow_render_system`, and `region_highlight_render_system` despawn and rebuild every sprite every frame, even when their input resources haven't changed. Adding `is_changed()` guards alone is insufficient — the upstream extraction systems in `data_layer.rs` (`extract_tip_positions`, `extract_priority_bias_map`, `extract_selected_region_tiles`) unconditionally call `.clear()` on their output resources every frame, which marks the `ResMut` as changed regardless of whether the new content differs. We fix the extraction layer first (compare-before-assign), then the guards on the consumer systems become real.

`organism_render_system` already incrementally spawns sprites via `Without<OrganismSprite>` filters — it is not a full rebuild. The `Added<T>` / `RemovedComponents<T>` refactor still has merit (cleaner intent, decouples spawn from despawn, lets `RemovedComponents` clean orphans without iterating linked sprites every frame) but the justification is "clearer architecture", not "fix a perf bug".

**Files:**
- Modify: `crates/render/src/data_layer.rs`
- Modify: `crates/render/src/entity_render.rs`
- Modify: `crates/render/src/lib.rs` (system registration)

- [x] **Step 0: Make `extract_tip_positions`, `extract_priority_bias_map`, `extract_selected_region_tiles` skip mutation when content is unchanged**

In `crates/render/src/data_layer.rs`, replace each extraction system's `clear()` + `push()`/`insert()` pattern with build-locally-then-assign-if-different. Triggering change detection requires `DerefMut`; reading via `Deref` does not, so `tip_positions.tips != new_tips` is read-only.

Replace `extract_tip_positions` (lines 116–128):
```rust
pub fn extract_tip_positions(
    tips: Query<(&GridPos, &HyphalTip)>,
    region_states: Res<RegionStates>,
    mut tip_positions: ResMut<TipPositions>,
) {
    let new_tips: Vec<(Hex, Option<SpecializationType>)> = tips
        .iter()
        .map(|(gpos, tip)| {
            let spec = region_states
                .get(tip.region_id)
                .and_then(|r| r.specialization);
            (gpos.0, spec)
        })
        .collect();
    if tip_positions.tips != new_tips {
        tip_positions.tips = new_tips;
    }
}
```

Replace `extract_priority_bias_map` (lines 273–283):
```rust
pub fn extract_priority_bias_map(
    tiles: Query<(&GridPos, &Tile)>,
    mut bias_map: ResMut<PriorityBiasMap>,
) {
    let new_biases: HashMap<Hex, Vec2> = tiles
        .iter()
        .filter_map(|(gpos, tile)| {
            (tile.priority_bias.length_squared() > 0.001)
                .then_some((gpos.0, tile.priority_bias))
        })
        .collect();
    if bias_map.biases != new_biases {
        bias_map.biases = new_biases;
    }
}
```

Replace `extract_selected_region_tiles` (lines 285–298):
```rust
pub fn extract_selected_region_tiles(
    tiles: Query<(&GridPos, &Tile)>,
    selected: Res<SelectedRegion>,
    mut selected_tiles: ResMut<SelectedRegionTiles>,
) {
    let new_tiles: Vec<Hex> = match selected.region_id {
        Some(rid) => tiles
            .iter()
            .filter_map(|(gpos, tile)| (tile.occupant.region_id() == Some(rid)).then_some(gpos.0))
            .collect(),
        None => Vec::new(),
    };
    if selected_tiles.tiles != new_tiles {
        selected_tiles.tiles = new_tiles;
    }
}
```

If `SpecializationType` doesn't already derive `PartialEq`, add `#[derive(PartialEq)]` (and `Eq`) where it's defined in `crates/core/`. The compiler will tell you on first build.

- [x] **Step 1: Add `is_changed()` guard to `tip_render_system`**

At the top of `tip_render_system` (around line 26 — insert as the first statement of the body), add:
```rust
if !tip_positions.is_changed() {
    return;
}
```

- [x] **Step 2: Add `is_changed()` guard to `priority_arrow_render_system`**

At the top of `priority_arrow_render_system` (around line 191 — first statement of body), add:
```rust
if !bias_map.is_changed() {
    return;
}
```

- [x] **Step 3: Add `is_changed()` guard to `region_highlight_render_system`**

At the top of `region_highlight_render_system` (around line 274 — first statement of body), add:
```rust
if !selected_tiles.is_changed() {
    return;
}
```

- [x] **Step 4: Run the workspace tests to confirm nothing broke**

```
just test
```
Expected: all tests pass. The existing `data_layer.rs` tests (`tip_positions_extracts_tips` etc.) still cover the new compare-then-assign paths.

- [x] **Step 5: Refactor `organism_render_system` to react to component changes**

Split `organism_render_system` into two narrower systems and replace the `Without<OrganismSprite>` polling with `Added<T>`:

```rust
pub fn spawn_organism_sprites(
    mut commands: Commands,
    sprites: Res<EntitySprites>,
    layout: Res<HexLayout>,
    new_fragments: Query<(Entity, &GridPos), Added<FragmentAgent>>,
    new_plants: Query<(Entity, &GridPos), Added<PlantRootAgent>>,
    new_fauna: Query<(Entity, &GridPos), Added<FaunaAgent>>,
    new_fruiting: Query<(Entity, &FruitingBody), Added<FruitingBody>>,
    new_mushrooms: Query<(Entity, &MushroomEntity), Added<MushroomEntity>>,
    new_neutral_fungi: Query<(Entity, &GridPos), Added<NeutralFungusAgent>>,
) {
    let size = organism_sprite_size(&layout);
    for (source, gpos) in new_fragments.iter() {
        let world_pos = layout.hex_to_world_pos(gpos.0);
        commands.spawn((
            OrganismSprite,
            OrganismSpriteLink(source),
            Sprite {
                image: sprites.fragment.clone(),
                color: Color::srgb(0.9, 0.7, 1.0),
                custom_size: Some(size),
                ..default()
            },
            Transform::from_translation(world_pos.extend(2.0)),
        ));
    }
    // ... repeat the same shape for plants, fauna, fruiting bodies, mushrooms, neutral fungi.
    // Each block reuses the existing colour and sprite asset choices from the original system.
}

pub fn despawn_orphaned_organism_sprites(
    mut commands: Commands,
    mut removed_fragments: RemovedComponents<FragmentAgent>,
    mut removed_plants: RemovedComponents<PlantRootAgent>,
    mut removed_fauna: RemovedComponents<FaunaAgent>,
    mut removed_fruiting: RemovedComponents<FruitingBody>,
    mut removed_mushrooms: RemovedComponents<MushroomEntity>,
    mut removed_neutral_fungi: RemovedComponents<NeutralFungusAgent>,
    linked_sprites: Query<(Entity, &OrganismSpriteLink), With<OrganismSprite>>,
) {
    use std::collections::HashSet;
    let mut removed: HashSet<Entity> = HashSet::new();
    removed.extend(removed_fragments.read());
    removed.extend(removed_plants.read());
    removed.extend(removed_fauna.read());
    removed.extend(removed_fruiting.read());
    removed.extend(removed_mushrooms.read());
    removed.extend(removed_neutral_fungi.read());

    if removed.is_empty() {
        return;
    }
    for (sprite, link) in linked_sprites.iter() {
        if removed.contains(&link.0) {
            commands.entity(sprite).despawn();
        }
    }
}
```

Delete the old `organism_render_system` function and the `OrganismQueries` `SystemParam` once both new systems are in.

- [x] **Step 6: Update the plugin registration with explicit ordering**

Find the `add_systems` call that registers `organism_render_system` (search the crate's `lib.rs` or `RenderPlugin`) and replace it with `(despawn_orphaned_organism_sprites, spawn_organism_sprites).chain()`. Despawn runs first so that on a same-frame remove-then-respawn (rare but possible), the new sprite spawned by `Added<T>` doesn't get despawned by stale `RemovedComponents` data — and so a removal followed by a re-add of the same source entity in successive frames can't leave an orphan sprite.

```
rg -n 'organism_render_system' crates/render/src
```
Update each match.

- [x] **Step 7: Run tests, lint, and a smoke build**

```
just test
just lint
just build
```
Expected: green tests, no clippy warnings, clean build.

- [x] **Step 8: Commit**

```
git add crates/render/src/data_layer.rs crates/render/src/entity_render.rs crates/render/src/lib.rs
git commit -m "perf(render): make extraction layer compare-before-mutate, gate consumers on change detection"
```

---

### Task 3: Change-detection guards on UI systems

`update_ability_bar`, `spec_picker_system`, and `spec_picker_highlight_system` rebuild UI nodes or recolour buttons every frame. Guard each on the resources that drive its output.

**Files:**
- Modify: `crates/ui/src/ability_bar.rs`
- Modify: `crates/ui/src/spec_picker.rs`

- [x] **Step 1: Guard `update_ability_bar`**

The system reads three resources (`selected`, `region_states`, `spore_action`) AND a `Query<&MushroomEntity>` (line 112: spore button visibility depends on `mushrooms.iter().next().is_some()`). Mushrooms can spawn or despawn without touching any of the three resources, so the guard must include mushroom set changes — otherwise the spore button won't appear when the first mushroom forms.

Update the system signature to add `Added<MushroomEntity>` and `RemovedComponents<MushroomEntity>`, then guard on all four signals. Insert as the first statements of the body (after the existing parameter list, before the despawn loops at line 49):
```rust
pub fn update_ability_bar(
    region_states: Res<RegionStates>,
    selected: Res<SelectedRegion>,
    spore_action: Res<SporeAction>,
    mushrooms: Query<&MushroomEntity>,
    new_mushrooms: Query<(), Added<MushroomEntity>>,
    mut removed_mushrooms: RemovedComponents<MushroomEntity>,
    entities: AbilityBarEntities,
    mut commands: Commands,
) {
    // Drain RemovedComponents whether or not we early-return so the buffer doesn't carry over.
    let removed_count = removed_mushrooms.read().count();
    let mushroom_set_changed = new_mushrooms.iter().next().is_some() || removed_count > 0;
    if !selected.is_changed()
        && !region_states.is_changed()
        && !spore_action.is_changed()
        && !mushroom_set_changed
    {
        return;
    }
    // ... existing body unchanged ...
}
```

- [x] **Step 2: Guard `spec_picker_system`**

At the top of `spec_picker_system` (line 57 in `crates/ui/src/spec_picker.rs`), add:
```rust
if !selected.is_changed() && !region_states.is_changed() {
    return;
}
```

- [x] **Step 3: Guard `spec_picker_highlight_system`**

Same pattern at the top of that system:
```rust
if !selected.is_changed() && !region_states.is_changed() {
    return;
}
```

- [x] **Step 4: Run tests and lint**

```
just test
just lint
```
Expected: all tests pass, no clippy warnings.

- [x] **Step 5: Commit**

```
git add crates/ui/src/ability_bar.rs crates/ui/src/spec_picker.rs
git commit -m "perf(ui): skip ability bar and spec picker rebuilds when inputs unchanged"
```

---

### Task 4: Tighten resource and query types

Three independent over-broad accesses: `rival_ai_system` takes `ResMut` but only reads, `decay_system` accepts an unused `Commands` and queries `Entity` and `GridPos` it never touches, and `priority_system` walks every tile twice when one pass works.

**Files:**
- Modify: `crates/ai/src/rival.rs`
- Modify: `crates/growth/src/decay.rs`
- Modify: `crates/input/src/priority.rs`

- [x] **Step 1: Downgrade `rival_state` from `ResMut` to `Res` in `rival_ai_system`**

In `crates/ai/src/rival.rs:35`, change the system signature:
```rust
pub fn rival_ai_system(
    mut tiles: Query<(&GridPos, &mut Tile)>,
    grid: Res<GridWorld>,
    rival_state: Res<RivalState>,   // was: ResMut<RivalState>
    mut rng: ResMut<RivalRng>,
)
```
Verify the body never assigns to `rival_state.*`; it only reads `rival_state.rival_id` on line 38.

- [x] **Step 2: Slim `decay_system` query and drop unused `Commands`**

In `crates/growth/src/decay.rs:4`, replace:
```rust
pub fn decay_system(
    mut tiles: Query<&mut Tile>,
    region_states: Res<RegionStates>,
) {
    for mut tile in tiles.iter_mut() {
        if let Occupant::Player(rid) = tile.occupant {
            let starved = region_states.get(rid).is_none_or(|r| r.nutrients <= 0.0);
            if starved {
                tile.biomass -= 0.1;
                if tile.biomass <= 0.0 {
                    tile.biomass = 0.0;
                    tile.occupant = Occupant::Empty;
                }
            }
        }
    }
}
```
The query no longer needs `Entity` or `GridPos` — neither is used in the body. The `_commands` parameter is gone entirely.

- [x] **Step 3: Collapse `priority_system` two-pass loop into one**

In `crates/input/src/priority.rs`, replace the body that runs after the early-returns (lines 25–43) with a single pass:
```rust
let Some(target_hex) = selected.selected_pos else {
    return;
};

for (gpos, mut tile) in &mut tiles {
    let dist = gpos.0.distance_to(target_hex);
    tile.priority_bias = if dist <= PRIORITY_RADIUS {
        let dir = layout.hex_to_world_pos(target_hex) - layout.hex_to_world_pos(gpos.0);
        if dir.length_squared() > 0.01 {
            dir.normalize() * 0.5
        } else {
            Vec2::ZERO
        }
    } else {
        Vec2::ZERO
    };
}
```
The Shift+P clear-all branch above stays as-is.

- [x] **Step 4: Run tests for each crate and lint**

```
cargo nextest run -p fungai_ai
cargo nextest run -p fungai_growth
cargo nextest run -p fungai_input
just lint
```
Expected: all green, no clippy warnings. The existing `priority.rs` tests (`p_key_sets_bias_around_selected_tile`, `shift_p_clears_bias`, `p_with_no_selection_is_noop`) cover the changed branch.

- [x] **Step 5: Commit**

```
git add crates/ai/src/rival.rs crates/growth/src/decay.rs crates/input/src/priority.rs
git commit -m "refactor: tighten ResMut and query types in rival, decay, priority"
```

---

### Task 5: Control-flow idioms

Two small readability fixes: a double-negated condition in `effects.rs` and an eight-arm `else if` chain in `specialization_input.rs`.

**Files:**
- Modify: `crates/fruiting/src/effects.rs`
- Modify: `crates/input/src/specialization_input.rs`

- [x] **Step 1: Flip the double negation in `mufungai_effect_system`**

In `crates/fruiting/src/effects.rs:21`, replace:
```rust
if !(bonus_region.is_none() && dist <= 3) {
    continue;
}

if let Occupant::Player(rid) = tile.occupant {
    bonus_region = Some(rid);
}
```
with:
```rust
if bonus_region.is_none()
    && dist <= 3
    && let Occupant::Player(rid) = tile.occupant
{
    bonus_region = Some(rid);
}
```
The `let` chain in `if` was stabilised in Rust 2024; this crate already uses edition 2024 per the workspace `Cargo.toml`.

- [x] **Step 2: Replace the `else if` chain with a table lookup**

In `crates/input/src/specialization_input.rs`, replace lines 12–30 with:
```rust
const KEY_SPECS: &[(KeyCode, SpecializationType)] = &[
    (KeyCode::Digit1, SpecializationType::Decomposer),
    (KeyCode::Digit2, SpecializationType::Parasite),
    (KeyCode::Digit3, SpecializationType::Symbiont),
    (KeyCode::Digit4, SpecializationType::Explorer),
    (KeyCode::Digit5, SpecializationType::Hunter),
    (KeyCode::Digit6, SpecializationType::Transporter),
    (KeyCode::Digit7, SpecializationType::Infiltrator),
    (KeyCode::Digit8, SpecializationType::Researcher),
];

let Some(target) = KEY_SPECS
    .iter()
    .copied()
    .find_map(|(key, spec)| keyboard.just_pressed(key).then_some(spec))
else {
    return;
};
```
The trailing `selected.region_id` / `region_states.get_mut` block stays as-is, with `target` used directly.

- [x] **Step 3: Run the affected tests and lint**

```
cargo nextest run -p fungai_fruiting
cargo nextest run -p fungai_input
just lint
```
Expected: PASS, no clippy warnings.

- [x] **Step 4: Commit**

```
git add crates/fruiting/src/effects.rs crates/input/src/specialization_input.rs
git commit -m "refactor: simplify mufungai_effect_system region check and digit key dispatch"
```

---

### Task 6: Random selection idioms

Three sites collect into `Vec` only to pick one or several entries by index — `IteratorRandom::choose` and `SliceRandom::choose_multiple` express the intent without the allocation.

**Files:**
- Modify: `crates/fruiting/src/spores.rs`
- Modify: `crates/regions/src/slot_machine.rs`

- [x] **Step 1: Replace the mushroom pick in `spore_system`**

In `crates/fruiting/src/spores.rs`, replace lines 39–46:
```rust
use rand::seq::IteratorRandom;

let Some(mushroom) = mushrooms.iter().choose(&mut rng.0) else {
    spore_action.triggered = false;
    return;
};
```
Add the `IteratorRandom` import at the top of the file if not already in scope.

- [x] **Step 2: Replace the landing-tile pick in `pick_spore_landing`**

In `crates/fruiting/src/spores.rs`, replace lines 87–108:
```rust
use rand::seq::IteratorRandom;

fn pick_spore_landing(
    _grid: &GridWorld,
    tiles: &Query<(&GridPos, &Tile)>,
    origin: Hex,
    rng: &mut StdRng,
) -> Option<Hex> {
    let radius = SPORE_RELAY_ACCURACY_RADIUS as u32;
    tiles
        .iter()
        .filter_map(|(gpos, tile)| {
            let dist = gpos.0.unsigned_distance_to(origin);
            (dist <= radius
                && tile.terrain.is_passable()
                && !tile.occupant.is_player()
                && !tile.occupant.is_rival())
            .then_some(gpos.0)
        })
        .choose(rng)
}
```

- [x] **Step 3: Replace manual partial Fisher-Yates in `slot_machine.rs`**

In `crates/regions/src/slot_machine.rs:25`, find the manual shuffle that picks 3 options and replace with:
```rust
use rand::seq::IndexedRandom;

let picks: Vec<UnlockOption> = pool_options
    .choose_multiple(&mut rng.0, 3)
    .cloned()
    .collect();
```
**Note:** `rand` 0.9 (workspace pin) moved slice `choose`/`choose_multiple` from `SliceRandom` to the new `IndexedRandom` trait. `SliceRandom` only retains `shuffle`/`partial_shuffle` in 0.9 — using it here will fail to compile. Iterator `choose` (Steps 1 and 2) stays on `IteratorRandom`.

- [x] **Step 4: Run tests and lint**

```
cargo nextest run -p fungai_fruiting
cargo nextest run -p fungai_regions
just lint
```
Expected: PASS. `IteratorRandom::choose` consumes the RNG via reservoir sampling, which differs from `random_range(0..len)` once `len > 1`. The existing fruiting tests (`spore_spawns_tip_near_mushroom`, `spore_cooldown_prevents_rapid_fire`) only assert tip count and region invariants — not exact landing positions — so they should still pass. The slot machine test (`slot_machine_produces_three_options`) is at higher risk: `choose_multiple` and the manual Fisher-Yates produce different picks for the same seed. If a seeded test breaks, update the fixture to match the new sequence rather than reverting the production change.

- [x] **Step 5: Commit**

```
git add crates/fruiting/src/spores.rs crates/regions/src/slot_machine.rs
git commit -m "refactor: use IteratorRandom::choose and IndexedRandom::choose_multiple"
```

---

### Task 7: Misc cleanup — drop unnecessary collect, document a necessary one, delete duplicated palette function

**Files:**
- Modify: `crates/regions/src/discovery.rs`
- Modify: `crates/world/src/region_tracking.rs`
- Modify: `crates/render/src/network_render.rs`

- [x] **Step 1: Drop the `Vec<Hex>` collect in `discovery.rs`**

In `crates/regions/src/discovery.rs:29`, replace:
```rust
let positions: Vec<Hex> = std::iter::once(gpos.0)
    .chain(grid.neighbors(gpos.0).map(|(p, _)| p))
    .collect();
for pos in positions {
    // ... body ...
}
```
with:
```rust
for pos in std::iter::once(gpos.0).chain(grid.neighbors(gpos.0).map(|(p, _)| p)) {
    // ... body unchanged ...
}
```

- [x] **Step 2: Document the necessary collect in `region_tracking.rs`**

In `crates/world/src/region_tracking.rs:80`, the existing `collect::<Vec<RegionId>>()` is correct because it releases the immutable iter borrow before the subsequent mutating remove. Add a one-line comment so the next reader doesn't try to "fix" it:
```rust
// Collect to drop the immutable borrow on `region_states.regions` before we mutate it below.
let empty_rids: Vec<RegionId> = region_states
    .regions
    .iter()
    .filter(|(_, s)| s.tile_count == 0)
    .map(|(id, _)| *id)
    .collect();
```

- [x] **Step 3: Delete duplicated `region_color` and update tests**

In `crates/render/src/network_render.rs`, the `#[cfg(test)] fn region_color(...) -> Color` duplicates the match in `region_color_linear`. Delete `region_color` (lines 62–78). The test `region_color_maps_specializations` at line 683 must switch to comparing in linear space directly — `Color::from(LinearRgba)` produces a `Color::LinearRgba` enum variant that does NOT compare equal to `Color::srgb(...)` (a `Color::Srgba` variant) under derived `PartialEq`, so do not route through `Color`.

Replace the test assertions to use `LinearRgba` directly:

```rust
#[test]
fn region_color_maps_specializations() {
    let explorer = region_color_linear(Some(SpecializationType::Explorer));
    assert_eq!(explorer, LinearRgba::new(1.0, 0.9, 0.3, 1.0));

    let parasite = region_color_linear(Some(SpecializationType::Parasite));
    assert_eq!(parasite, LinearRgba::new(0.8, 0.2, 0.2, 1.0));

    let none_color = region_color_linear(None);
    assert_eq!(none_color, LinearRgba::new(0.9, 0.85, 0.7, 1.0));

    let hunter = region_color_linear(Some(SpecializationType::Hunter));
    assert_eq!(hunter, LinearRgba::new(0.6, 0.4, 0.1, 1.0));
}
```

`region_color_linear` is currently private (`fn region_color_linear`). Since the test is in the same module's `#[cfg(test)] mod tests` (which has `use super::*`), no visibility change is needed.

Find any other call sites with:
```
rg -n 'region_color\(' crates/render/src/network_render.rs
```
Edit each matching test assertion to match the pattern above. Run the test:

```
cargo nextest run -p fungai_render
```
Expected: PASS.

- [x] **Step 4: Run the affected crate tests and lint**

```
cargo nextest run -p fungai_regions
cargo nextest run -p fungai_world
cargo nextest run -p fungai_render
just lint
```
Expected: all green, no clippy warnings.

- [x] **Step 5: Commit**

```
git add crates/regions/src/discovery.rs crates/world/src/region_tracking.rs crates/render/src/network_render.rs
git commit -m "refactor: drop and document collects, delete duplicated region_color"
```

---

### Task 8: Replace glob re-exports and a glob import

The `lib.rs` files for `fungai_ai` and `fungai_ui` use `pub use module::*`, which leaks every public item from each module into the crate's public surface. Four crate-internal files also use `use fungai_core::*` on the import side. Replace each with explicit names.

**Files:**
- Modify: `crates/ui/src/lib.rs` (glob re-exports)
- Modify: `crates/ai/src/lib.rs` (glob re-exports)
- Modify: `crates/growth/src/tip.rs` (glob import)
- Modify: `crates/growth/src/decay.rs` (glob import, line 2)
- Modify: `crates/ai/src/rival.rs` (glob import, line 4)
- Modify: `crates/render/src/data_layer.rs` (glob import, line 4)
- Verify only: `crates/fruiting/src/lib.rs` (already explicit; no edits)

- [x] **Step 1: Inventory which names each module actually exports outside its crate**

For each crate, run from the workspace root:
```
rg -n 'fungai_ui::' crates/
rg -n 'fungai_ai::' crates/
rg -n 'fungai_fruiting::' crates/
```
Note every name referenced. That set is the new explicit `pub use` list. The compiler will catch misses on the next `cargo check`.

- [x] **Step 2: Make `crates/ui/src/lib.rs` explicit**

Replace lines 9–12:
```rust
pub use ability_bar::{
    AbilityBarRoot, AbilityButton, SporeButton,
    spawn_ability_bar, update_ability_bar, ability_click_system, spore_button_system,
};
pub use hud::{spawn_hud, update_hud};
pub use slot_machine_ui::{
    SlotMachineState, slot_machine_ui_system, slot_machine_selection_system,
};
pub use spec_picker::{
    spec_picker_system, spec_picker_click_system, spec_picker_highlight_system,
};
```
Adjust the lists to match what the inventory in Step 1 shows. Anything unreferenced externally drops to `pub(crate)` or stays private.

- [x] **Step 3: Make `crates/ai/src/lib.rs` explicit**

Replace lines 10–13:
```rust
pub use combat::combat_resolution_system;
pub use environment::{EnvironmentRng, environment_threat_system};
pub use organisms::{
    bacteria_system, fauna_system, neutral_fungi_system, plant_system,
};
pub use rival::{RivalRng, RivalState, rival_ai_system};
```
Adjust to the inventory results.

- [x] **Step 4: Verify `crates/fruiting/src/lib.rs`**

Open the file. The current re-exports (`mufungai_effect_system`, `fruiting_system`, `SporeRng`, `spore_system`) are already explicit — no glob to fix. If new exports are missing per the inventory, add them; otherwise leave alone.

- [x] **Step 5: Replace the glob imports in `tip.rs`, `decay.rs`, `rival.rs`, `data_layer.rs`**

For each of these four files, replace `use fungai_core::*;` with an explicit list. The compiler will tell you what's missing — start with the names referenced in the file body and add anything `cargo check -p <crate>` reports as unresolved.

`crates/growth/src/tip.rs:4`:
```rust
use fungai_core::{
    ANASTOMOSIS_BIOMASS_BONUS, GridPos, GridWorld, Hex, HexLayout, HyphalTip,
    Occupant, RegionId, RegionStates, SpecializationType, Tile, create_hex_layout,
};
```

`crates/growth/src/decay.rs:2`:
```rust
use fungai_core::{GridPos, GridWorld, Occupant, RegionStates, Tile};
```
(`GridWorld` and `GridPos` and `Hex` are referenced in the test module via `Hex::ZERO` — check if `Hex` needs to be added too.)

`crates/ai/src/rival.rs:4` — uses many core types; run the rg first to enumerate:
```
rg -n 'fungai_core::|\b(Tile|GridPos|GridWorld|Occupant|RegionId|RegionStates|RivalId|Hex|HexLayout)\b' crates/ai/src/rival.rs
```
Then replace with the explicit list the rg surfaces.

`crates/render/src/data_layer.rs:4` — same approach. The file references `BranchGraph`/`HyphalTip`/`Occupant`/`RegionId`/`RegionStates`/`SpecializationType`/`Tile`/`GridPos`/`GridWorld`/`HexLayout`/`SelectedRegion`/`RivalId` and `create_hex_layout` (in tests). Build the explicit list.

For each file, verify by running `cargo check -p <crate>` and `cargo nextest run -p <crate>` after the change. If clippy complains about unused imports, drop them.

- [x] **Step 6: Run the workspace tests and lint**

```
just test
just lint
```
Expected: all green. `clippy` may fire `unused_imports` if any explicit name isn't actually used — drop it.

- [x] **Step 7: Commit**

```
git add crates/ui/src/lib.rs crates/ai/src/lib.rs crates/growth/src/tip.rs crates/growth/src/decay.rs crates/ai/src/rival.rs crates/render/src/data_layer.rs
git commit -m "refactor: replace glob re-exports and imports with explicit name lists"
```

---

### Polish

- [x] **Step 1: Run post-implementation-polish over the cumulative diff**

Dispatch `post-implementation-polish` against the merged set of changes from Tasks 1–8. The polish skill runs three review rounds with fixes, an idiomatic pass, `/cleanup`, and a comment-humanization pass.

- [x] **Step 2: Final verification**

```
just fmt
just lint
just test
just build
```
Expected: all four pass.

- [ ] **Step 3: Update the napkin and serena memory if anything notable surfaced**

Run `/docs` to refresh CLAUDE.md / serena memory if any of the changes shifted high-level conventions worth recording (e.g. "we now standardise change-detection guards on per-frame render systems"). If nothing notable, skip.
