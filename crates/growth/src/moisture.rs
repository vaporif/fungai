use bevy::prelude::*;
use kingdom_core::{GridPos, GridWorld, TerrainType, Tile};

const DIFFUSION_RATE: f32 = 0.05;

pub fn moisture_diffusion_system(mut tiles: Query<(&GridPos, &mut Tile)>, grid: Res<GridWorld>) {
    let snapshot: std::collections::HashMap<_, _> =
        tiles.iter().map(|(gp, t)| (gp.0, t.moisture)).collect();

    for (gpos, mut tile) in tiles.iter_mut() {
        if tile.terrain == TerrainType::Water {
            tile.moisture = 1.0;
            continue;
        }
        let mut total_diff = 0.0_f32;
        let mut count = 0_f32;
        for (npos, _) in grid.neighbors(gpos.0) {
            if let Some(&n_moist) = snapshot.get(&npos) {
                total_diff += n_moist - tile.moisture;
                count += 1.0;
            }
        }
        if count > 0.0 {
            tile.moisture += DIFFUSION_RATE * (total_diff / count);
            tile.moisture = tile.moisture.clamp(0.0, 1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kingdom_core::{Hex, create_hex_layout};

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GridWorld>();
        app.insert_resource(create_hex_layout());
        app.add_systems(Update, moisture_diffusion_system);
        app
    }

    fn spawn(app: &mut App, pos: Hex, tile: Tile) -> Entity {
        let e = app.world_mut().spawn((GridPos(pos), tile)).id();
        app.world_mut()
            .resource_mut::<GridWorld>()
            .tiles
            .insert(pos, e);
        e
    }

    #[test]
    fn water_terrain_stays_at_one() {
        let mut app = test_app();
        let e = spawn(
            &mut app,
            Hex::ZERO,
            Tile {
                terrain: TerrainType::Water,
                moisture: 0.5,
                ..default()
            },
        );
        app.update();
        assert_eq!(app.world().get::<Tile>(e).unwrap().moisture, 1.0);
    }

    #[test]
    fn dry_tile_adjacent_to_wet_neighbor_gains_moisture() {
        let mut app = test_app();
        let center = Hex::new(5, 5);
        let neighbor = center.all_neighbors()[0];
        let dry = spawn(
            &mut app,
            center,
            Tile {
                moisture: 0.0,
                ..default()
            },
        );
        spawn(
            &mut app,
            neighbor,
            Tile {
                moisture: 1.0,
                ..default()
            },
        );
        app.update();
        let m = app.world().get::<Tile>(dry).unwrap().moisture;
        assert!(
            m > 0.0 && m < 1.0,
            "moisture should rise toward wet neighbor: {m}"
        );
    }

    #[test]
    fn moisture_clamps_non_negative() {
        let mut app = test_app();
        let e = spawn(
            &mut app,
            Hex::ZERO,
            Tile {
                moisture: 0.0,
                ..default()
            },
        );
        app.update();
        assert!(app.world().get::<Tile>(e).unwrap().moisture >= 0.0);
    }
}
