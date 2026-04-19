use bevy::prelude::*;
use hexx::{Hex, HexOrientation, OffsetHexMode};
use rand::prelude::*;
use rand::rngs::StdRng;
use shroom_core::{
    BacteriaColonyAgent, FragmentAgent, FragmentId, GameState, GridPos, GridWorld, HyphalTip,
    NeutralFungusAgent, Occupant, PlantRootAgent, RegionStates, RivalId, TerrainType, Tile,
    TileContents,
};

const MAP_WIDTH: i32 = 80;
const MAP_HEIGHT: i32 = 60;

/// Converts offset grid coordinates (col, row) to axial hex coordinates.
/// Uses pointy-top orientation with odd-row offset, matching the project's hex layout.
fn offset_to_hex(col: i32, row: i32) -> Hex {
    Hex::from_offset_coordinates([col, row], OffsetHexMode::Odd, HexOrientation::Pointy)
}

#[derive(Resource)]
pub struct TerrainSeed(pub u64);

impl Default for TerrainSeed {
    fn default() -> Self {
        Self(42)
    }
}

pub fn terrain_generation(
    mut commands: Commands,
    mut grid: ResMut<GridWorld>,
    mut game_state: ResMut<GameState>,
    mut region_states: ResMut<RegionStates>,
    seed: Res<TerrainSeed>,
) {
    let mut rng = StdRng::seed_from_u64(seed.0);
    grid.width = MAP_WIDTH;
    grid.height = MAP_HEIGHT;

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

            let moisture = 0.3 + 0.5 * (y as f32 / MAP_HEIGHT as f32) + rng.random::<f32>() * 0.2;
            let nutrient_level = 0.2 + rng.random::<f32>() * 0.6;

            let entity = commands
                .spawn((
                    GridPos(hex),
                    Tile {
                        terrain,
                        nutrient_level,
                        moisture: moisture.clamp(0.0, 1.0),
                        ..default()
                    },
                ))
                .id();
            grid.tiles.insert(hex, entity);
        }
    }

    let fragment_count = rng.random_range(3u32..=5);
    game_state.fragments_total = fragment_count;
    game_state.mushrooms_required = fragment_count;
    for i in 0..fragment_count {
        let pos = random_soil_pos(&grid, &mut rng);
        if let Some(&entity) = grid.tiles.get(&pos) {
            commands.entity(entity).insert(Tile {
                contents: Some(TileContents::Fragment(FragmentId(i))),
                ..default()
            });
            commands.spawn((
                GridPos(pos),
                FragmentAgent {
                    fragment_id: FragmentId(i),
                    fused: false,
                },
            ));
        }
    }

    let decomp_count = rng.random_range(3u32..=5);
    for i in 0..decomp_count {
        let pos = random_soil_pos(&grid, &mut rng);
        if let Some(&entity) = grid.tiles.get(&pos) {
            commands.entity(entity).insert(Tile {
                contents: Some(TileContents::UniqueDecomposable(i)),
                ..default()
            });
        }
    }

    let fungi_count = rng.random_range(2u32..=4);
    for i in 0..fungi_count {
        let pos = random_soil_pos(&grid, &mut rng);
        if let Some(&entity) = grid.tiles.get(&pos) {
            commands.entity(entity).insert(Tile {
                contents: Some(TileContents::NeutralFungus(i)),
                ..default()
            });
            commands.spawn((
                GridPos(pos),
                NeutralFungusAgent {
                    fungus_id: i,
                    merge_progress: 0.0,
                },
            ));
        }
    }

    // Roots need proximity to surface
    let plant_count = rng.random_range(3u32..=6);
    for i in 0..plant_count {
        let x = rng.random_range(0..MAP_WIDTH);
        let y = rng.random_range(MAP_HEIGHT / 2..MAP_HEIGHT - 1);
        let pos = offset_to_hex(x, y);
        if let Some(&entity) = grid.tiles.get(&pos) {
            commands.entity(entity).insert(Tile {
                terrain: TerrainType::Root,
                contents: Some(TileContents::PlantRoot(i)),
                ..default()
            });
            commands.spawn((
                GridPos(pos),
                PlantRootAgent {
                    plant_id: i,
                    health: 1.0,
                    trade_active: false,
                    nutrient_intake: 0.0,
                    sugar_output: 0.0,
                    neglect_timer: 0,
                },
            ));
        }
    }

    let bacteria_count = rng.random_range(1u32..=2);
    for _i in 0..bacteria_count {
        let pos = random_soil_pos(&grid, &mut rng);
        commands.spawn((
            GridPos(pos),
            BacteriaColonyAgent {
                spread_timer: 0,
                spread_interval: 10,
            },
        ));
    }

    // Spawn player starting region near center
    let player_rid = region_states.create_region();
    if let Some(state) = region_states.get_mut(player_rid) {
        state.nutrients = 100.0;
        state.energy = 20.0;
    }
    let player_start = offset_to_hex(MAP_WIDTH / 2, MAP_HEIGHT / 2);
    for hex in player_start.range(2) {
        if let Some(&entity) = grid.tiles.get(&hex) {
            commands.entity(entity).insert(Tile {
                terrain: TerrainType::Soil,
                occupant: Occupant::Player(player_rid),
                nutrient_level: 0.8,
                moisture: 0.5,
                discovered: true,
                contents: None,
                biomass: 1.0,
                nutrient_gradient: Vec2::ZERO,
                priority_bias: Vec2::ZERO,
            });
        }
    }
    // Spawn initial hyphal tips at the neighbor ring around the starting hex
    for neighbor in player_start.all_neighbors() {
        if grid.tiles.contains_key(&neighbor) {
            commands.spawn((
                GridPos(neighbor),
                HyphalTip {
                    region_id: player_rid,
                    age: 0,
                },
            ));
        }
    }

    // Spawn rival in the opposite corner
    let rival_id = RivalId(0);
    let rival_start = offset_to_hex(MAP_WIDTH / 4, MAP_HEIGHT / 4);
    for hex in rival_start.range(1) {
        if let Some(&entity) = grid.tiles.get(&hex) {
            commands.entity(entity).insert(Tile {
                terrain: TerrainType::Soil,
                occupant: Occupant::Rival(rival_id),
                nutrient_level: 0.5,
                moisture: 0.5,
                discovered: false,
                contents: None,
                biomass: 1.5,
                nutrient_gradient: Vec2::ZERO,
                priority_bias: Vec2::ZERO,
            });
        }
    }
}

fn random_soil_pos(grid: &GridWorld, rng: &mut StdRng) -> Hex {
    let w = grid.width;
    let h = grid.height;
    loop {
        let hex = offset_to_hex(rng.random_range(1..w - 1), rng.random_range(1..h - 2));
        if grid.tiles.contains_key(&hex) {
            return hex;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shroom_core::RegionStates;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GridWorld>();
        app.init_resource::<GameState>();
        app.init_resource::<RegionStates>();
        app.insert_resource(TerrainSeed(12345));
        app
    }

    #[test]
    fn generates_grid_with_correct_dimensions() {
        let mut app = test_app();
        app.add_systems(Startup, terrain_generation);
        app.update();

        let grid = app.world().resource::<GridWorld>();
        assert_eq!(grid.width, MAP_WIDTH);
        assert_eq!(grid.height, MAP_HEIGHT);
        assert_eq!(grid.tiles.len(), (MAP_WIDTH * MAP_HEIGHT) as usize);
    }

    #[test]
    fn top_row_is_surface_terrain() {
        let mut app = test_app();
        app.add_systems(Startup, terrain_generation);
        app.update();

        let grid = app.world().resource::<GridWorld>();
        for x in 0..MAP_WIDTH {
            let hex = offset_to_hex(x, MAP_HEIGHT - 1);
            let entity = grid.tiles[&hex];
            let tile = app.world().get::<Tile>(entity).unwrap();
            assert_eq!(tile.terrain, TerrainType::Surface);
        }
    }

    #[test]
    fn places_fragments() {
        let mut app = test_app();
        app.add_systems(Startup, terrain_generation);
        app.update();

        let mut fragment_count = 0u32;
        for tile in app.world_mut().query::<&Tile>().iter(app.world()) {
            if matches!(tile.contents, Some(TileContents::Fragment(_))) {
                fragment_count += 1;
            }
        }
        assert!((3..=5).contains(&fragment_count));
    }

    #[test]
    fn moisture_higher_near_surface() {
        let mut app = test_app();
        app.add_systems(Startup, terrain_generation);
        app.update();

        let grid = app.world().resource::<GridWorld>();
        let near_surface = offset_to_hex(0, MAP_HEIGHT - 2);
        let deep = offset_to_hex(0, 0);
        let surface_tile = app.world().get::<Tile>(grid.tiles[&near_surface]).unwrap();
        let deep_tile = app.world().get::<Tile>(grid.tiles[&deep]).unwrap();
        assert!(surface_tile.moisture > deep_tile.moisture);
    }
}
