# Leafwing Input Manager Integration Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or `/team-feature` to implement this plan task-by-task, per the Execution Strategy below. Steps use checkbox (`- [ ]`) syntax — these are **persistent durable state**, not visual decoration. The executor edits the plan file in place: `- [ ]` → `- [x]` the instant a step verifies, before moving on. On resume (new session, crash, takeover), the executor scans existing `- [x]` marks and skips them — these steps are NOT redone. TodoWrite mirrors this state in-session; the plan file is the source of truth across sessions.

**Goal:** Replace the raw `ButtonInput<KeyCode>` and `ButtonInput<MouseButton>` reads in the five `fungai_input` systems with a single action-based layer driven by `leafwing-input-manager` 0.20.

**Architecture:** Add `leafwing-input-manager` 0.20 as a workspace dep. Define one `Action` enum and one `default_input_map()` in a new `crates/input/src/action.rs`. Register `InputManagerPlugin::<Action>` from `InputPlugin`. Each existing system (`camera`, `selection`, `priority`, `specialization_input`, `speed`) drops its raw `ButtonInput`/`MouseWheel` reads and reads `Res<ActionState<Action>>` instead. The bin crate needs no changes.

**Tech Stack:** Rust (edition 2024), Bevy 0.18, `leafwing-input-manager` 0.20, `cargo nextest`.

## Execution Strategy

**Subagents** — six tasks, mostly parallel after the foundation. The system migrations touch disjoint files and do not need to read each other's output.

Reason: tasks 2–6 each touch a single system file and depend only on the `Action` enum and registered plugin from task 1. After task 1 lands, they can dispatch in parallel.

## Task Dependency Graph

- T1 [AFK]: depends on `none` → first batch
- T2 [AFK]: depends on `T1` → second batch
- T3 [AFK]: depends on `T1` → second batch (parallel with T2)
- T4 [AFK]: depends on `T1` → second batch (parallel with T2)
- T5 [AFK]: depends on `T1` → second batch (parallel with T2)
- T6 [AFK]: depends on `T1` → second batch (parallel with T2)
- Polish [AFK]: depends on `T2, T3, T4, T5, T6` → third batch

T2–T6 dispatch in parallel after T1's review passes. Polish runs after all migrations land.

## Agent Assignments

- T1: Foundation (deps + Action enum + plugin)            → rust-engineer
- T2: Camera migration                                    → rust-engineer
- T3: Selection migration                                 → rust-engineer
- T4: Priority migration + test rewrite                   → rust-engineer
- T5: Specialization migration                            → rust-engineer
- T6: Speed migration + test rewrite                      → rust-engineer
- Polish: post-implementation-polish                      → general-purpose

The diff is uniformly Rust, but the polish phase runs cleanup and idiomatic passes that aren't language-specific, so `general-purpose` is fine there.

---

## File Structure

| File | Status | Responsibility |
|------|--------|----------------|
| `Cargo.toml` (workspace) | Modify | Adds `leafwing-input-manager = "0.20"` to `[workspace.dependencies]`. |
| `crates/input/Cargo.toml` | Modify | Adds `leafwing-input-manager = { workspace = true }` to `[dependencies]`. |
| `crates/input/src/action.rs` | Create | Defines `Action` enum (with `Actionlike` derive) and `default_input_map()`. |
| `crates/input/src/lib.rs` | Modify | Declares `mod action`, re-exports `Action` and `default_input_map`, registers `InputManagerPlugin::<Action>`, inserts default map and `ActionState`. |
| `crates/input/src/camera.rs` | Modify | Drops `ButtonInput<KeyCode>` + `MessageReader<MouseWheel>` for `Res<ActionState<Action>>`. |
| `crates/input/src/selection.rs` | Modify | Drops `ButtonInput<MouseButton>` from `PointerInput` for `Res<ActionState<Action>>`. |
| `crates/input/src/priority.rs` | Modify | Drops manual shift handling for `ClearPriority`/`SetPriority` actions. Tests rewritten to drive `ActionState` through `Buttonlike::press`. |
| `crates/input/src/specialization_input.rs` | Modify | Replaces `(KeyCode, SpecializationType)` table with `(Action, SpecializationType)` over `Spec1..Spec8`. |
| `crates/input/src/speed.rs` | Modify | Reads `TogglePause`/`SpeedUp`/`SlowDown` from `ActionState`. Tests rewritten to drive `ActionState` through `Buttonlike::press`. |

## Cross-cutting notes (read before starting any task)

- The `Action` enum lives in one place. Every binding goes through `default_input_map()` — do not add `KeyCode::*` reads back into a system. If a new binding is needed mid-task, extend the enum and the map together.
- `ClashStrategy::PrioritizeLongest` is the leafwing default. With both `SetPriority` (`P`) and `ClearPriority` (`Shift+P`) registered, pressing Shift+P triggers only `ClearPriority`. The priority system still checks `ClearPriority` first as a defensive guard if that default ever changes.
- `ActionState::axis_pair(&Action::CameraMove)` returns `Vec2` (not `Option<Vec2>`); zero when no input. `ActionState::value(&Action::Zoom)` returns `f32`.
- For tests, `ActionState::press` direct calls do NOT survive `update_action_state` (which runs in `PreUpdate` and overwrites the resource). Use `Buttonlike::press(world)` from the leafwing prelude — it sends the raw input message, which Bevy's input plugin and leafwing's update system then turn into a real `just_pressed` reading the next `app.update()`.
- `MinimalPlugins` does NOT include Bevy's `InputPlugin`. Tests must add `bevy::input::InputPlugin` explicitly so leafwing's `update_action_state` system can read input messages.

## Pros / Cons

**Pros**
- One source of truth for bindings — change a key in one place.
- Free chord support (`Shift+P`) instead of hand-rolled modifier handling.
- Path to gamepad and rebinding without further input refactor.
- Standard, maintained Bevy ecosystem crate.

**Cons**
- One added dependency.
- Marginal benefit for the mouse-selection system, whose complexity is in viewport math and UI gating, not click detection.
- Slight per-system boilerplate increase (one extra `Res<ActionState<Action>>` parameter).

---

## Task 1: Foundation — deps, `Action` enum, plugin registration

**Files:**
- Modify: `Cargo.toml` (workspace)
- Modify: `crates/input/Cargo.toml`
- Create: `crates/input/src/action.rs`
- Modify: `crates/input/src/lib.rs`

- [x] **Step 1: Add workspace dependency**

In `Cargo.toml`, add to `[workspace.dependencies]` after the existing `hexx` line:

```toml
leafwing-input-manager = "0.20"
```

- [x] **Step 2: Add crate dependency**

In `crates/input/Cargo.toml`, add to `[dependencies]`:

```toml
leafwing-input-manager = { workspace = true }
```

The block becomes:

```toml
[dependencies]
bevy = { workspace = true }
fungai_core = { workspace = true }
leafwing-input-manager = { workspace = true }
```

- [x] **Step 3: Verify the new dep compiles**

Run: `cargo build -p fungai_input`
Expected: success. (May take a minute on first build while `leafwing-input-manager` compiles.)

- [x] **Step 4: Create `crates/input/src/action.rs` with the `Action` enum and `default_input_map()`**

```rust
use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

#[derive(Actionlike, Clone, Copy, Hash, PartialEq, Eq, Debug, Reflect)]
pub enum Action {
    #[actionlike(DualAxis)]
    CameraMove,
    #[actionlike(Axis)]
    Zoom,

    SelectTile,

    SetPriority,
    ClearPriority,

    Spec1,
    Spec2,
    Spec3,
    Spec4,
    Spec5,
    Spec6,
    Spec7,
    Spec8,

    TogglePause,
    SpeedUp,
    SlowDown,
}

pub fn default_input_map() -> InputMap<Action> {
    let mut map = InputMap::default();

    // Camera: WASD and arrow keys both produce a normalised dual-axis pair.
    map.insert_dual_axis(Action::CameraMove, VirtualDPad::wasd());
    map.insert_dual_axis(Action::CameraMove, VirtualDPad::arrow_keys());

    // Zoom: vertical mouse scroll.
    map.insert_axis(Action::Zoom, MouseScrollAxis::Y);

    // Mouse selection.
    map.insert(Action::SelectTile, MouseButton::Left);

    // Priority: bare P sets bias, Shift+P clears. PrioritizeLongest clash strategy
    // suppresses SetPriority while the longer chord matches.
    map.insert(Action::SetPriority, KeyCode::KeyP);
    map.insert(
        Action::ClearPriority,
        ButtonlikeChord::modified(ModifierKey::Shift, KeyCode::KeyP),
    );

    // Specialization 1-8.
    map.insert(Action::Spec1, KeyCode::Digit1);
    map.insert(Action::Spec2, KeyCode::Digit2);
    map.insert(Action::Spec3, KeyCode::Digit3);
    map.insert(Action::Spec4, KeyCode::Digit4);
    map.insert(Action::Spec5, KeyCode::Digit5);
    map.insert(Action::Spec6, KeyCode::Digit6);
    map.insert(Action::Spec7, KeyCode::Digit7);
    map.insert(Action::Spec8, KeyCode::Digit8);

    // Speed.
    map.insert(Action::TogglePause, KeyCode::Space);
    map.insert(Action::SpeedUp, KeyCode::Equal);
    map.insert(Action::SpeedUp, KeyCode::NumpadAdd);
    map.insert(Action::SlowDown, KeyCode::Minus);
    map.insert(Action::SlowDown, KeyCode::NumpadSubtract);

    map
}
```

- [x] **Step 5: Wire the plugin and resources in `crates/input/src/lib.rs`**

Replace the file contents with:

```rust
use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

mod action;
mod camera;
mod priority;
mod selection;
mod specialization_input;
mod speed;

pub use action::{Action, default_input_map};
pub use camera::{GameCamera, camera_system, spawn_camera};
pub use fungai_core::SelectedRegion;
pub use priority::priority_system;
pub use selection::selection_system;
pub use specialization_input::specialization_input_system;
pub use speed::speed_input_system;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InputManagerPlugin::<Action>::default())
            .insert_resource(default_input_map())
            .init_resource::<ActionState<Action>>()
            .add_systems(Startup, spawn_camera)
            .add_systems(
                Update,
                (
                    camera_system,
                    selection_system,
                    priority_system,
                    speed_input_system,
                    specialization_input_system,
                ),
            );
    }
}
```

- [x] **Step 6: Build the crate to confirm wiring compiles**

Run: `cargo build -p fungai_input`
Expected: success. The five system fns still take their old signatures at this point — they will compile because nothing has changed in their bodies yet.

- [x] **Step 7: Build the workspace**

Run: `cargo build`
Expected: success.

- [x] **Step 8: Lint check**

Run: `just lint`
Expected: clean (no fmt or clippy warnings introduced).

- [x] **Step 9: Commit**

Run:
```
git add Cargo.toml crates/input/Cargo.toml crates/input/src/action.rs crates/input/src/lib.rs
git commit -m "input: add leafwing action enum and plugin"
```

---

## Task 2: Camera migration

**Files:**
- Modify: `crates/input/src/camera.rs`

- [x] **Step 1: Replace the system signature and body**

Replace the existing `pub fn camera_system(...)` with:

```rust
use bevy::prelude::*;
use fungai_core::{Hex, HexLayout, HexOrientation, OffsetHexMode};
use leafwing_input_manager::prelude::*;

use crate::action::Action;

#[derive(Component, Debug)]
pub struct GameCamera;

const CAMERA_SPEED: f32 = 300.0;
const ZOOM_SPEED: f32 = 0.1;
const MIN_ZOOM: f32 = 0.15;
const MAX_ZOOM: f32 = 4.0;

pub fn spawn_camera(mut commands: Commands, layout: Res<HexLayout>) {
    let center_hex =
        Hex::from_offset_coordinates([40, 30], OffsetHexMode::Odd, HexOrientation::Pointy);
    let center = layout.hex_to_world_pos(center_hex);
    commands.spawn((
        Camera2d,
        GameCamera,
        Transform::from_xyz(center.x, center.y, 0.0),
    ));
}

pub fn camera_system(
    time: Res<Time>,
    actions: Res<ActionState<Action>>,
    mut query: Query<(&mut Transform, &mut Projection), With<GameCamera>>,
) {
    let Ok((mut transform, mut projection)) = query.single_mut() else {
        return;
    };
    let delta = time.delta_secs();

    let direction = actions.axis_pair(&Action::CameraMove);
    if direction.length_squared() > 0.0 {
        transform.translation += (direction.normalize() * CAMERA_SPEED * delta).extend(0.0);
    }

    if let Projection::Orthographic(ref mut ortho) = *projection {
        let zoom_delta = actions.value(&Action::Zoom);
        if zoom_delta != 0.0 {
            ortho.scale = (ortho.scale - zoom_delta * ZOOM_SPEED).clamp(MIN_ZOOM, MAX_ZOOM);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_range_matches_spec() {
        assert_eq!(MIN_ZOOM, 0.15);
        assert_eq!(MAX_ZOOM, 4.0);
    }
}
```

Note: `axis_pair` already returns a clamped/normalised pair from `VirtualDPad`. Diagonals are unit-length, so the `normalize()` call here mostly handles the WASD-only case where the raw pair is on an axis. Keeping `normalize()` matches the existing constant-speed behavior.

- [x] **Step 2: Build the crate**

Run: `cargo build -p fungai_input`
Expected: success.

- [x] **Step 3: Run unit tests**

Run: `cargo nextest run -p fungai_input camera`
Expected: `zoom_range_matches_spec` passes.

- [ ] **Step 4: Run the game to verify panning and zoom**

Run: `just dev`
Verify:
- W/A/S/D pans the camera.
- Arrow keys also pan.
- Mouse wheel zooms within the 0.15–4.0 range.

- [x] **Step 5: Lint**

Run: `just lint`
Expected: clean.

- [x] **Step 6: Commit**

Run:
```
git add crates/input/src/camera.rs
git commit -m "input: migrate camera_system to ActionState"
```

---

## Task 3: Selection migration

**Files:**
- Modify: `crates/input/src/selection.rs`

- [x] **Step 1: Replace mouse field with action state**

Replace the file contents with:

```rust
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use fungai_core::*;
use leafwing_input_manager::prelude::*;

use crate::action::Action;
use crate::camera::GameCamera;

#[derive(SystemParam)]
pub struct PointerInput<'w, 's> {
    actions: Res<'w, ActionState<Action>>,
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    camera_q: Query<'w, 's, (&'static Camera, &'static GlobalTransform), With<GameCamera>>,
    ui_interactions: Query<'w, 's, &'static Interaction, With<Button>>,
}

pub fn selection_system(
    pointer: PointerInput,
    grid: Res<GridWorld>,
    tiles: Query<&Tile>,
    mut selected: ResMut<SelectedRegion>,
    layout: Res<HexLayout>,
) {
    if !pointer.actions.just_pressed(&Action::SelectTile) {
        return;
    }

    // Skip world clicks while a UI button is being interacted with.
    for interaction in pointer.ui_interactions.iter() {
        if *interaction != Interaction::None {
            return;
        }
    }

    let Ok(window) = pointer.windows.single() else {
        return;
    };
    let Ok((camera, cam_transform)) = pointer.camera_q.single() else {
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let Ok(world_pos) = camera.viewport_to_world_2d(cam_transform, cursor_pos) else {
        return;
    };

    let hex = layout.world_pos_to_hex(world_pos);

    let Some(&entity) = grid.tiles.get(&hex) else {
        return;
    };

    if let Ok(tile) = tiles.get(entity) {
        selected.selected_pos = Some(hex);
        selected.region_id = tile.occupant.region_id();
    }
}
```

- [x] **Step 2: Build the crate**

Run: `cargo build -p fungai_input`
Expected: success.

- [ ] **Step 3: Run the game and verify selection**

Run: `just dev`
Verify:
- Left-click on a tile selects it (existing visual feedback updates).
- Left-click on a UI button does not select a tile underneath.

- [x] **Step 4: Lint**

Run: `just lint`
Expected: clean.

- [x] **Step 5: Commit**

Run:
```
git add crates/input/src/selection.rs
git commit -m "input: migrate selection_system to ActionState"
```

---

## Task 4: Priority migration + test rewrite

**Files:**
- Modify: `crates/input/src/priority.rs`

- [x] **Step 1: Rewrite `priority_system`**

Replace the system signature and body (keep `PRIORITY_RADIUS` and the imports for `GridPos`, `HexLayout`, `SelectedRegion`, `Tile`):

```rust
use bevy::prelude::*;
use fungai_core::{GridPos, HexLayout, SelectedRegion, Tile};
use leafwing_input_manager::prelude::*;

use crate::action::Action;

const PRIORITY_RADIUS: i32 = 3;

pub fn priority_system(
    actions: Res<ActionState<Action>>,
    selected: Res<SelectedRegion>,
    layout: Res<HexLayout>,
    mut tiles: Query<(&GridPos, &mut Tile)>,
) {
    // ClearPriority is the longer chord; with PrioritizeLongest clash strategy
    // it suppresses SetPriority. Check it first as a defensive guard against
    // strategy changes.
    if actions.just_pressed(&Action::ClearPriority) {
        for (_gpos, mut tile) in &mut tiles {
            tile.priority_bias = Vec2::ZERO;
        }
        return;
    }

    if !actions.just_pressed(&Action::SetPriority) {
        return;
    }

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
}
```

- [x] **Step 2: Replace the test module**

Replace the entire `#[cfg(test)] mod tests { ... }` block with:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use bevy::input::InputPlugin as BevyInputPlugin;
    use fungai_core::{GridPos, GridWorld, Hex, Tile, create_hex_layout};
    use leafwing_input_manager::prelude::*;

    use crate::action::{Action, default_input_map};

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(BevyInputPlugin);
        app.add_plugins(InputManagerPlugin::<Action>::default());
        app.insert_resource(default_input_map());
        app.init_resource::<ActionState<Action>>();
        app.init_resource::<GridWorld>();
        app.init_resource::<SelectedRegion>();
        app.insert_resource(create_hex_layout());
        app.add_systems(Update, priority_system);
        app
    }

    fn spawn_tile(app: &mut App, hex: Hex) -> Entity {
        let entity = app
            .world_mut()
            .spawn((
                GridPos(hex),
                Tile {
                    priority_bias: Vec2::ZERO,
                    ..Default::default()
                },
            ))
            .id();
        app.world_mut()
            .resource_mut::<GridWorld>()
            .tiles
            .insert(hex, entity);
        entity
    }

    #[test]
    fn p_key_sets_bias_around_selected_tile() {
        let mut app = test_app();
        let target = Hex::new(8, -3);
        let near = Hex::new(5, -3);
        let _ = spawn_tile(&mut app, target);
        let near_entity = spawn_tile(&mut app, near);

        app.world_mut()
            .resource_mut::<SelectedRegion>()
            .selected_pos = Some(target);

        KeyCode::KeyP.press(app.world_mut());
        app.update();

        let tile = app.world().get::<Tile>(near_entity).expect("tile exists");
        assert!(
            tile.priority_bias.length_squared() > 0.0,
            "near tile should have bias"
        );
    }

    #[test]
    fn shift_p_clears_bias() {
        let mut app = test_app();
        let hex = Hex::new(0, 0);
        let entity = spawn_tile(&mut app, hex);
        app.world_mut()
            .get_mut::<Tile>(entity)
            .unwrap()
            .priority_bias = Vec2::new(0.5, 0.0);

        KeyCode::ShiftLeft.press(app.world_mut());
        KeyCode::KeyP.press(app.world_mut());
        app.update();

        let tile = app.world().get::<Tile>(entity).unwrap();
        assert_eq!(tile.priority_bias, Vec2::ZERO);
    }

    #[test]
    fn p_with_no_selection_is_noop() {
        let mut app = test_app();
        let hex = Hex::new(0, 0);
        let entity = spawn_tile(&mut app, hex);
        app.world_mut()
            .get_mut::<Tile>(entity)
            .unwrap()
            .priority_bias = Vec2::new(0.5, 0.0);

        KeyCode::KeyP.press(app.world_mut());
        app.update();

        let tile = app.world().get::<Tile>(entity).unwrap();
        assert_eq!(tile.priority_bias, Vec2::new(0.5, 0.0));
    }
}
```

Key points:
- `bevy::input::InputPlugin` is added explicitly because `MinimalPlugins` does not include it, and leafwing's `update_action_state` reads input messages produced there.
- `KeyCode::KeyP.press(app.world_mut())` uses the `Buttonlike::press` trait method (re-exported from `leafwing_input_manager::prelude`) to send the actual `KeyboardInput` message that the leafwing system will turn into a `just_pressed` reading on the next `app.update()`.
- For the chord test, `ShiftLeft` and `KeyP` are pressed in the same frame so `ButtonlikeChord::modified(ModifierKey::Shift, KeyCode::KeyP)` matches.

- [x] **Step 3: Build**

Run: `cargo build -p fungai_input --tests`
Expected: success.

- [x] **Step 4: Run priority tests**

Run: `cargo nextest run -p fungai_input priority`
Expected: all three tests pass.

- [ ] **Step 5: Smoke-check in `just dev`**

Run: `just dev`
Verify:
- Select a tile, press P → bias appears (visible via existing rendering or by re-selecting and seeing growth direction).
- Press Shift+P → bias clears across the map.
- Press P with nothing selected → nothing changes.

- [x] **Step 6: Lint**

Run: `just lint`
Expected: clean.

- [x] **Step 7: Commit**

Run:
```
git add crates/input/src/priority.rs
git commit -m "input: migrate priority_system and tests to ActionState"
```

---

## Task 5: Specialization migration

**Files:**
- Modify: `crates/input/src/specialization_input.rs`

- [x] **Step 1: Replace the system body**

Replace the file contents with:

```rust
use bevy::prelude::*;
use fungai_core::{RegionStates, SpecializationType};
use leafwing_input_manager::prelude::*;

use crate::SelectedRegion;
use crate::action::Action;

/// Keys 1-8 (mapped via `Action::Spec1..Spec8`) assign a target specialization
/// to the selected region.
pub fn specialization_input_system(
    actions: Res<ActionState<Action>>,
    selected: Res<SelectedRegion>,
    mut region_states: ResMut<RegionStates>,
) {
    const ACTION_SPECS: &[(Action, SpecializationType)] = &[
        (Action::Spec1, SpecializationType::Decomposer),
        (Action::Spec2, SpecializationType::Parasite),
        (Action::Spec3, SpecializationType::Symbiont),
        (Action::Spec4, SpecializationType::Explorer),
        (Action::Spec5, SpecializationType::Hunter),
        (Action::Spec6, SpecializationType::Transporter),
        (Action::Spec7, SpecializationType::Infiltrator),
        (Action::Spec8, SpecializationType::Researcher),
    ];

    let Some(target) = ACTION_SPECS
        .iter()
        .copied()
        .find_map(|(action, spec)| actions.just_pressed(&action).then_some(spec))
    else {
        return;
    };

    let Some(rid) = selected.region_id else {
        return;
    };
    let Some(state) = region_states.get_mut(rid) else {
        return;
    };

    state.target_specialization = Some(target);
}
```

- [x] **Step 2: Build**

Run: `cargo build -p fungai_input`
Expected: success.

- [ ] **Step 3: Smoke-check in `just dev`**

Run: `just dev`
Verify:
- Select a region, press digit `1` → that region's target specialization becomes Decomposer (visible in HUD).
- Each of `2..8` produces the matching specialization (Parasite, Symbiont, Explorer, Hunter, Transporter, Infiltrator, Researcher).

- [x] **Step 4: Lint**

Run: `just lint`
Expected: clean.

- [x] **Step 5: Commit**

Run:
```
git add crates/input/src/specialization_input.rs
git commit -m "input: migrate specialization_input_system to ActionState"
```

---

## Task 6: Speed migration + test rewrite

**Files:**
- Modify: `crates/input/src/speed.rs`

- [x] **Step 1: Rewrite the system**

Replace the system body (keep the `Duration` import, the `SimulationSpeed`/`TickTimer` imports, and the `tick_timer.timer.set_duration(...)` block):

```rust
use std::time::Duration;

use bevy::prelude::*;
use fungai_core::{SimulationSpeed, TickTimer};
use leafwing_input_manager::prelude::*;

use crate::action::Action;

pub fn speed_input_system(
    actions: Res<ActionState<Action>>,
    mut speed: ResMut<SimulationSpeed>,
    mut tick_timer: ResMut<TickTimer>,
) {
    let mut changed = false;

    if actions.just_pressed(&Action::TogglePause) {
        *speed = if speed.is_paused() {
            SimulationSpeed::Normal
        } else {
            SimulationSpeed::Paused
        };
        changed = true;
    }

    if actions.just_pressed(&Action::SpeedUp) {
        *speed = speed.speed_up();
        changed = true;
    }

    if actions.just_pressed(&Action::SlowDown) {
        *speed = speed.slow_down();
        changed = true;
    }

    if changed && !speed.is_paused() {
        tick_timer
            .timer
            .set_duration(Duration::from_secs_f32(speed.duration_secs()));
    }
}
```

- [x] **Step 2: Replace the test module**

Replace the existing `#[cfg(test)] mod tests { ... }` with:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use bevy::input::InputPlugin as BevyInputPlugin;
    use leafwing_input_manager::prelude::*;

    use crate::action::{Action, default_input_map};

    fn setup_app(initial_speed: SimulationSpeed) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(BevyInputPlugin);
        app.add_plugins(InputManagerPlugin::<Action>::default());
        app.insert_resource(default_input_map());
        app.init_resource::<ActionState<Action>>();
        app.insert_resource(initial_speed);
        app.insert_resource(TickTimer::default());
        app.add_systems(Update, speed_input_system);
        app
    }

    #[test]
    fn space_toggles_pause() {
        let mut app = setup_app(SimulationSpeed::Normal);
        KeyCode::Space.press(app.world_mut());
        app.update();
        assert_eq!(
            *app.world().resource::<SimulationSpeed>(),
            SimulationSpeed::Paused
        );
    }

    #[test]
    fn plus_speeds_up() {
        let mut app = setup_app(SimulationSpeed::Normal);
        KeyCode::Equal.press(app.world_mut());
        app.update();
        assert_eq!(
            *app.world().resource::<SimulationSpeed>(),
            SimulationSpeed::Fast
        );
    }

    #[test]
    fn minus_slows_down() {
        let mut app = setup_app(SimulationSpeed::Fast);
        KeyCode::Minus.press(app.world_mut());
        app.update();
        assert_eq!(
            *app.world().resource::<SimulationSpeed>(),
            SimulationSpeed::Normal
        );
    }
}
```

- [x] **Step 3: Build**

Run: `cargo build -p fungai_input --tests`
Expected: success.

- [x] **Step 4: Run speed tests**

Run: `cargo nextest run -p fungai_input speed`
Expected: all three tests pass.

- [ ] **Step 5: Smoke-check in `just dev`**

Run: `just dev`
Verify:
- Space toggles pause / resume.
- `=` (or `Numpad+`) speeds up.
- `-` (or `Numpad-`) slows down.

- [x] **Step 6: Lint**

Run: `just lint`
Expected: clean.

- [x] **Step 7: Commit**

Run:
```
git add crates/input/src/speed.rs
git commit -m "input: migrate speed_input_system and tests to ActionState"
```

---

## Final verification (after all tasks)

- [x] **Step 1: Full lint**

Run: `just lint`
Expected: clean.

- [x] **Step 2: Full test suite**

Run: `just test`
Expected: green across the workspace.

- [ ] **Step 3: End-to-end smoke test**

Run: `just dev`
Verify each input behavior:
- WASD and arrow keys pan the camera at the same speed as before.
- Mouse scroll zooms inside the 0.15–4.0 clamp.
- Left-click selects a tile and is suppressed when over a UI button.
- `P` sets priority bias around the selected tile; `Shift+P` clears all bias; `P` with no selection is a no-op.
- Digits 1–8 set the selected region's target specialization.
- `Space` toggles pause; `=`/`Numpad+` speeds up; `-`/`Numpad-` slows down.

- [x] **Step 4: Confirm no leftover raw input reads**

Run: `rg 'ButtonInput<KeyCode>|ButtonInput<MouseButton>|MessageReader<MouseWheel>' crates/input/src/`
Expected: no results.
