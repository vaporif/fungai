use bevy::ecs::message::MessageReader;
use bevy::prelude::*;
use kingdom_core::*;

use crate::wisp::TileTapped;

pub fn selection_system(
    mut taps: MessageReader<TileTapped>,
    grid: Res<GridWorld>,
    tiles: Query<&Tile>,
    mut selected: ResMut<SelectedRegion>,
) {
    // The wisp emits one TileTapped per confirmed tap (short press without
    // drag). If multiple arrive in a frame, the last one wins — clicks chain
    // about as well as anything else.
    let Some(tap) = taps.read().last() else {
        return;
    };
    let Some(&entity) = grid.tiles.get(&tap.pos) else {
        return;
    };
    if let Ok(tile) = tiles.get(entity) {
        selected.selected_pos = Some(tap.pos);
        selected.region_id = tile.region_id;
    }
}
