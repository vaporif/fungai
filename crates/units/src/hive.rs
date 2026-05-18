use bevy::prelude::*;
use kingdom_core::{GridPos, GridWorld, Hive, HiveCaptured, Tile};

pub fn hive_capture_system(
    mut hives: Query<(&GridPos, &mut Hive)>,
    grid: Res<GridWorld>,
    tiles: Query<&Tile>,
    mut captured: MessageWriter<HiveCaptured>,
) {
    for (gpos, mut hive) in &mut hives {
        let new_owner = grid
            .tiles
            .get(&gpos.0)
            .and_then(|&e| tiles.get(e).ok())
            .filter(|t| t.is_owned())
            .and_then(|t| t.region_id);

        if new_owner != hive.captured_by {
            if let Some(region_id) = new_owner {
                captured.write(HiveCaptured {
                    hive_pos: gpos.0,
                    region_id,
                });
            }
            hive.captured_by = new_owner;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kingdom_core::{GameState, RegionId, RegionStates};

    /// Collects every `HiveCaptured` message emitted across the run so tests
    /// can assert on the capture *transition*, not just the resulting state.
    #[derive(Resource, Default)]
    struct CapturedLog(Vec<(hexx::Hex, RegionId)>);

    fn collect_captures(mut log: ResMut<CapturedLog>, mut reader: MessageReader<HiveCaptured>) {
        for msg in reader.read() {
            log.0.push((msg.hive_pos, msg.region_id));
        }
    }

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GridWorld>();
        app.init_resource::<RegionStates>();
        app.init_resource::<GameState>();
        app.init_resource::<CapturedLog>();
        app.add_message::<HiveCaptured>();
        app.add_systems(Update, (hive_capture_system, collect_captures).chain());
        app
    }

    fn spawn_tile(app: &mut App, pos: hexx::Hex, region: Option<RegionId>, biomass: f32) {
        let e = app
            .world_mut()
            .spawn((
                GridPos(pos),
                Tile {
                    region_id: region,
                    biomass,
                    ..default()
                },
            ))
            .id();
        app.world_mut()
            .resource_mut::<GridWorld>()
            .tiles
            .insert(pos, e);
    }

    fn set_tile_region(app: &mut App, pos: hexx::Hex, region: Option<RegionId>) {
        let entity = app.world().resource::<GridWorld>().tiles[&pos];
        app.world_mut().get_mut::<Tile>(entity).unwrap().region_id = region;
    }

    fn captures(app: &App) -> Vec<(hexx::Hex, RegionId)> {
        app.world().resource::<CapturedLog>().0.clone()
    }

    #[test]
    fn hive_on_owned_tile_is_captured() {
        let mut app = test_app();
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();
        let pos = hexx::Hex::new(3, 3);
        spawn_tile(&mut app, pos, Some(rid), 1.0);
        let hive = app
            .world_mut()
            .spawn((
                GridPos(pos),
                Hive {
                    captured_by: None,
                    production: 0.0,
                },
            ))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<Hive>(hive).unwrap().captured_by,
            Some(rid)
        );
    }

    #[test]
    fn hive_on_unowned_tile_is_neutral() {
        let mut app = test_app();
        let pos = hexx::Hex::new(4, 4);
        spawn_tile(&mut app, pos, None, 0.0);
        let hive = app
            .world_mut()
            .spawn((
                GridPos(pos),
                Hive {
                    captured_by: Some(RegionId(7)),
                    production: 0.0,
                },
            ))
            .id();
        app.update();
        assert_eq!(app.world().get::<Hive>(hive).unwrap().captured_by, None);
    }

    #[test]
    fn capture_fires_message_on_none_to_some() {
        let mut app = test_app();
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();
        let pos = hexx::Hex::new(2, 5);
        spawn_tile(&mut app, pos, Some(rid), 1.0);
        app.world_mut().spawn((
            GridPos(pos),
            Hive {
                captured_by: None,
                production: 0.0,
            },
        ));
        app.update();
        assert_eq!(
            captures(&app),
            vec![(pos, rid)],
            "a None -> Some capture must emit exactly one HiveCaptured",
        );
    }

    #[test]
    fn owner_change_updates_state_and_refires() {
        let mut app = test_app();
        let (rid_a, rid_b) = {
            let mut rs = app.world_mut().resource_mut::<RegionStates>();
            (rs.create_region(), rs.create_region())
        };
        let pos = hexx::Hex::new(6, 1);
        spawn_tile(&mut app, pos, Some(rid_a), 1.0);
        let hive = app
            .world_mut()
            .spawn((
                GridPos(pos),
                Hive {
                    captured_by: None,
                    production: 0.0,
                },
            ))
            .id();

        app.update();
        assert_eq!(
            app.world().get::<Hive>(hive).unwrap().captured_by,
            Some(rid_a)
        );

        set_tile_region(&mut app, pos, Some(rid_b));
        app.update();
        assert_eq!(
            app.world().get::<Hive>(hive).unwrap().captured_by,
            Some(rid_b),
            "captured_by must follow the new tile owner",
        );
        assert_eq!(
            captures(&app),
            vec![(pos, rid_a), (pos, rid_b)],
            "each owner change must emit a fresh HiveCaptured",
        );
    }

    #[test]
    fn quiet_tick_fires_no_message() {
        let mut app = test_app();
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();
        let pos = hexx::Hex::new(8, 8);
        spawn_tile(&mut app, pos, Some(rid), 1.0);
        app.world_mut().spawn((
            GridPos(pos),
            Hive {
                captured_by: None,
                production: 0.0,
            },
        ));

        app.update();
        assert_eq!(captures(&app).len(), 1, "first tick captures the hive");

        // Nothing changes on the tile: a quiet tick must stay silent.
        app.update();
        assert_eq!(
            captures(&app).len(),
            1,
            "a tick with no ownership change must not emit HiveCaptured",
        );
    }
}
