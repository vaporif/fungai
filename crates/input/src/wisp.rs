use std::collections::HashSet;

use bevy::ecs::message::{Message, MessageWriter};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use kingdom_core::{
    BIAS_MAGNITUDE_CAP, BIAS_STROKE_INTENSITY, DRAG_THRESHOLD_PX, GamePhase, GridPos, GridWorld,
    Hex, HexLayout, SAMPLE_HEX_DISTANCE, SAMPLE_INTERVAL_MS, TAP_TIME_MS, Tile,
    WISP_SENSE_RADIUS_HEX,
};
use leafwing_input_manager::prelude::*;

use crate::action::Action;
use crate::camera::GameCamera;

#[derive(Default, Clone, Copy, Debug)]
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
    owned: HashSet<Hex>,
}

#[derive(Message)]
pub struct TileTapped {
    pub pos: Hex,
}

#[allow(clippy::too_many_arguments)]
pub fn wisp_input_system(
    actions: Res<ActionState<Action>>,
    time: Res<Time>,
    phase: Res<GamePhase>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<GameCamera>>,
    ui_interactions: Query<&Interaction, With<Button>>,
    layout: Res<HexLayout>,
    grid: Res<GridWorld>,
    mut tiles: Query<(&GridPos, &mut Tile)>,
    mut wisp: ResMut<WispState>,
    mut taps: MessageWriter<TileTapped>,
) {
    if *phase != GamePhase::Playing {
        wisp.phase = WispPhase::Idle;
        return;
    }
    if ui_interactions
        .iter()
        .any(|i| !matches!(i, Interaction::None))
    {
        return;
    }

    let Some(cursor_world) = cursor_world_position(&windows, &cameras) else {
        return;
    };
    let now = time.elapsed_secs();

    let pressed = actions.pressed(&Action::Paint);
    let just_pressed = actions.just_pressed(&Action::Paint);
    let just_released = actions.just_released(&Action::Paint);

    if !pressed && !just_pressed && !just_released {
        return;
    }

    wisp.phase = match wisp.phase {
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
                refresh_owned(&mut wisp.owned, &tiles);
                write_segment(
                    start_pos,
                    cursor_world,
                    &layout,
                    &grid,
                    &wisp.owned,
                    &mut tiles,
                );
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
                    refresh_owned(&mut wisp.owned, &tiles);
                    write_segment(
                        last_sample_pos,
                        cursor_world,
                        &layout,
                        &grid,
                        &wisp.owned,
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
}

fn cursor_world_position(
    windows: &Query<&Window, With<PrimaryWindow>>,
    cameras: &Query<(&Camera, &GlobalTransform), With<GameCamera>>,
) -> Option<Vec2> {
    let window = windows.single().ok()?;
    let cursor = window.cursor_position()?;
    let (camera, cam_xform) = cameras.single().ok()?;
    camera.viewport_to_world_2d(cam_xform, cursor).ok()
}

fn refresh_owned(owned: &mut HashSet<Hex>, tiles: &Query<(&GridPos, &mut Tile)>) {
    owned.clear();
    owned.extend(
        tiles
            .iter()
            .filter_map(|(gp, t)| t.region_id.is_some().then_some(gp.0)),
    );
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
    if direction == Vec2::ZERO {
        return;
    }
    let hex = layout.world_pos_to_hex(p2);
    let Some(&entity) = grid.tiles.get(&hex) else {
        return;
    };
    let falloff = network_proximity_factor(hex, owned);
    if falloff <= 0.0 {
        return;
    }
    let Ok((_, mut tile)) = tiles.get_mut(entity) else {
        return;
    };
    let candidate = tile.priority_bias + direction * BIAS_STROKE_INTENSITY * falloff;
    let mag = candidate.length();
    let new_bias = if mag > BIAS_MAGNITUDE_CAP {
        candidate * (BIAS_MAGNITUDE_CAP / mag)
    } else {
        candidate
    };
    if tile.priority_bias != new_bias {
        tile.priority_bias = new_bias;
    }
}

fn network_proximity_factor(hex: Hex, owned: &HashSet<Hex>) -> f32 {
    if owned.is_empty() {
        return 0.0;
    }
    let radius = WISP_SENSE_RADIUS_HEX as u32;
    let nearest = owned
        .iter()
        .map(|o| hex.unsigned_distance_to(*o))
        .min()
        .unwrap_or(u32::MAX);
    if nearest > radius {
        return 0.0;
    }
    1.0 - (nearest as f32) / (radius as f32 + 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wisp_state_default_is_idle() {
        let s = WispState::default();
        assert!(matches!(s.phase, WispPhase::Idle));
    }
}
