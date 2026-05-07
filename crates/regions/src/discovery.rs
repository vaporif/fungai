use std::collections::HashMap;

use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;
use kingdom_core::{
    DecompositionComplete, GridPos, Hex, Occupant, RegionStates, Tile, TileContents, UnlockPool,
};

use crate::slot_machine::SlotMachineTriggered;

const DECOMP_RATE: f32 = 0.1;

#[derive(Resource, Default, Debug, Clone, Reflect)]
pub struct DecompProgress {
    pub entries: HashMap<Hex, f32>,
}

pub fn decomposer_discovery_system(
    mut tiles: Query<(&GridPos, &mut Tile)>,
    _region_states: Res<RegionStates>,
    mut progress: ResMut<DecompProgress>,
    mut decomp_messages: MessageWriter<DecompositionComplete>,
    mut slot_messages: MessageWriter<SlotMachineTriggered>,
) {
    for (gpos, mut tile) in tiles.iter_mut() {
        let Occupant::Player(_) = tile.occupant else {
            continue;
        };

        if !matches!(tile.contents, Some(TileContents::UniqueDecomposable(_))) {
            continue;
        }

        let prog = progress.entries.entry(gpos.0).or_insert(0.0);
        *prog += DECOMP_RATE;
        if *prog >= 1.0 {
            tile.contents = None;
            progress.entries.remove(&gpos.0);
            decomp_messages.write(DecompositionComplete { pos: gpos.0 });
            slot_messages.write(SlotMachineTriggered {
                pool: UnlockPool::Decomposition,
                options: Vec::new(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use kingdom_core::GridWorld;

    use super::*;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GridWorld>();
        app.init_resource::<RegionStates>();
        app.init_resource::<DecompProgress>();
        app.add_message::<DecompositionComplete>();
        app.add_message::<SlotMachineTriggered>();
        app
    }

    #[test]
    fn decomposer_breaks_down_unique_decomposable() {
        let mut app = test_app();
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();

        let pos = Hex::new(2, 2);
        let entity = app
            .world_mut()
            .spawn((
                GridPos(pos),
                Tile {
                    occupant: Occupant::Player(rid),
                    contents: Some(TileContents::UniqueDecomposable(0)),
                    ..default()
                },
            ))
            .id();
        app.world_mut()
            .resource_mut::<GridWorld>()
            .tiles
            .insert(pos, entity);

        app.world_mut()
            .resource_mut::<DecompProgress>()
            .entries
            .insert(pos, 0.95);

        app.add_systems(Update, decomposer_discovery_system);
        app.update();

        let tile = app.world().get::<Tile>(entity).unwrap();
        assert!(
            tile.contents.is_none()
                || !matches!(tile.contents, Some(TileContents::UniqueDecomposable(_))),
            "decomposable should be consumed on completion"
        );
    }
}
