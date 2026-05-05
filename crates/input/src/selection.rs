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
