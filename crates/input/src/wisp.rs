use std::collections::{HashSet, VecDeque};

use bevy::ecs::message::{Message, MessageWriter};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use kingdom_core::{
    BIAS_MAGNITUDE_CAP, BIAS_STROKE_INTENSITY, DRAG_THRESHOLD_PX, GridPos, GridWorld, Hex,
    HexLayout, SAMPLE_HEX_DISTANCE, SAMPLE_INTERVAL_MS, TAP_TIME_MS, Tile, WISP_SENSE_RADIUS_HEX,
};
use leafwing_input_manager::prelude::*;

use crate::action::Action;

#[derive(Default, Clone, Debug)]
pub enum WispPhase {
    #[default]
    Idle,
    Primed {
        start_pos: Vec2,
        start_time: f32,
    },
    Stroking {
        last_sample_pos: Vec2,
        last_sample_time: f32,
    },
}

#[derive(Resource, Default)]
pub struct WispState {
    pub phase: WispPhase,
}

#[derive(Message)]
pub struct TileTapped {
    pub pos: Hex,
}

// State-machine drives bias painting from cursor + time + tiles + grid; the
// 9-arg signature is the natural fit. `clippy::too_many_arguments` would
// force an artificial wrapper struct that hurts readability.
#[allow(clippy::too_many_arguments)]
pub fn wisp_input_system(
    actions: Res<ActionState<Action>>,
    time: Res<Time>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    layout: Res<HexLayout>,
    grid: Res<GridWorld>,
    mut tiles: Query<(&GridPos, &mut Tile)>,
    mut wisp: ResMut<WispState>,
    mut taps: MessageWriter<TileTapped>,
) {
    let Some(cursor_world) = cursor_world_position(&windows, &cameras) else {
        return;
    };
    let now = time.elapsed_secs();

    let pressed = actions.pressed(&Action::Paint);
    let just_pressed = actions.just_pressed(&Action::Paint);
    let just_released = actions.just_released(&Action::Paint);

    // Snapshot owned hex positions before any mutable tile borrow. The
    // proximity BFS later runs against this set, so the `tiles.get_mut(...)`
    // call inside `write_segment` doesn't fight the immutable iteration.
    let owned: HashSet<Hex> = tiles
        .iter()
        .filter_map(|(gp, t)| t.region_id.map(|_| gp.0))
        .collect();

    let prev = std::mem::take(&mut wisp.phase);
    let next = match prev {
        WispPhase::Idle => {
            if just_pressed {
                WispPhase::Primed {
                    start_pos: cursor_world,
                    start_time: now,
                }
            } else {
                WispPhase::Idle
            }
        }
        WispPhase::Primed {
            start_pos,
            start_time,
        } => {
            if just_released {
                if cursor_world.distance(start_pos) < DRAG_THRESHOLD_PX
                    && (now - start_time) * 1000.0 < TAP_TIME_MS as f32
                {
                    let hex = layout.world_pos_to_hex(start_pos);
                    taps.write(TileTapped { pos: hex });
                }
                WispPhase::Idle
            } else if pressed && cursor_world.distance(start_pos) > DRAG_THRESHOLD_PX {
                write_segment(start_pos, cursor_world, &layout, &grid, &owned, &mut tiles);
                WispPhase::Stroking {
                    last_sample_pos: cursor_world,
                    last_sample_time: now,
                }
            } else {
                WispPhase::Primed {
                    start_pos,
                    start_time,
                }
            }
        }
        WispPhase::Stroking {
            last_sample_pos,
            last_sample_time,
        } => {
            if just_released {
                WispPhase::Idle
            } else if pressed {
                let elapsed_ms = (now - last_sample_time) * 1000.0;
                let hex_size = layout.scale.x;
                if elapsed_ms > SAMPLE_INTERVAL_MS as f32
                    || cursor_world.distance(last_sample_pos) > SAMPLE_HEX_DISTANCE * hex_size
                {
                    write_segment(
                        last_sample_pos,
                        cursor_world,
                        &layout,
                        &grid,
                        &owned,
                        &mut tiles,
                    );
                    WispPhase::Stroking {
                        last_sample_pos: cursor_world,
                        last_sample_time: now,
                    }
                } else {
                    WispPhase::Stroking {
                        last_sample_pos,
                        last_sample_time,
                    }
                }
            } else {
                WispPhase::Idle
            }
        }
    };
    wisp.phase = next;
}

fn cursor_world_position(
    windows: &Query<&Window, With<PrimaryWindow>>,
    cameras: &Query<(&Camera, &GlobalTransform), With<Camera2d>>,
) -> Option<Vec2> {
    let window = windows.single().ok()?;
    let cursor = window.cursor_position()?;
    let (camera, cam_xform) = cameras.iter().next()?;
    camera.viewport_to_world_2d(cam_xform, cursor).ok()
}

fn write_segment(
    p1: Vec2,
    p2: Vec2,
    layout: &HexLayout,
    grid: &GridWorld,
    owned: &HashSet<Hex>,
    tiles: &mut Query<(&GridPos, &mut Tile)>,
) {
    let direction = (p2 - p1).normalize_or_zero();
    if direction.length_squared() < 1e-6 {
        return;
    }
    let hex = layout.world_pos_to_hex(p2);
    let Some(&entity) = grid.tiles.get(&hex) else {
        return;
    };
    let falloff = network_proximity_factor(hex, grid, owned);
    if falloff <= 0.0 {
        return;
    }
    let Ok((_, mut tile)) = tiles.get_mut(entity) else {
        return;
    };
    let new_bias = tile.priority_bias + direction * BIAS_STROKE_INTENSITY * falloff;
    let mag = new_bias.length();
    tile.priority_bias = if mag > BIAS_MAGNITUDE_CAP {
        new_bias * (BIAS_MAGNITUDE_CAP / mag)
    } else {
        new_bias
    };
}

fn network_proximity_factor(hex: Hex, grid: &GridWorld, owned: &HashSet<Hex>) -> f32 {
    if owned.is_empty() {
        return 0.0;
    }
    // BFS up to WISP_SENSE_RADIUS_HEX over the GridWorld topology. Returns a
    // 1.0..0.0 falloff scaled by hex distance to the nearest owned tile.
    let mut frontier: VecDeque<(Hex, i32)> = VecDeque::new();
    frontier.push_back((hex, 0));
    let mut seen = HashSet::new();
    seen.insert(hex);
    while let Some((current, dist)) = frontier.pop_front() {
        if dist > WISP_SENSE_RADIUS_HEX {
            continue;
        }
        if owned.contains(&current) {
            return 1.0 - (dist as f32) / (WISP_SENSE_RADIUS_HEX as f32 + 1.0);
        }
        for (npos, _) in grid.neighbors(current) {
            if seen.insert(npos) {
                frontier.push_back((npos, dist + 1));
            }
        }
    }
    0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    // Full state-machine timing tests live in T7 integration tests where the
    // cursor / window / time can be driven deterministically.

    #[test]
    fn wisp_state_default_is_idle() {
        let s = WispState::default();
        assert!(matches!(s.phase, WispPhase::Idle));
    }
}
