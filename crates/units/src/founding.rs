use bevy::prelude::*;
use kingdom_core::{
    FOUNDER_SEED_BIOMASS, FOUNDER_SEED_SUGARS, FoundNetworkRequest, GridPos, GridWorld, Hex,
    MIN_FOUNDING_DISTANCE, NetworkFounded, RegionStates, SelectedUnit, Tile, Unit, UnitKind,
    UnitMovement,
};

/// True when `hex` is a legal place to found a new network:
/// passable terrain, unclaimed, and at least `MIN_FOUNDING_DISTANCE` hexes
/// from the nearest owned tile of any region. The last check stops a founder
/// from seeding next to existing territory and triggering an instant merge.
pub fn is_valid_site(hex: Hex, grid: &GridWorld, tiles: &Query<&mut Tile>) -> bool {
    let Some(tile) = grid.tiles.get(&hex).and_then(|&e| tiles.get(e).ok()) else {
        return false;
    };
    // `TerrainType::is_passable()` is the single shared definition of passable,
    // so founding and pathfinding agree.
    if !tile.terrain.is_passable() || tile.region_id.is_some() {
        return false;
    }
    // No owned tile of any region may sit strictly within MIN_FOUNDING_DISTANCE.
    // Scanning only the hex disc keeps this independent of map size — `hex`
    // itself is in the disc but is already known unclaimed, so it can't reject.
    for pos in hex.range(MIN_FOUNDING_DISTANCE - 1) {
        if let Some(&entity) = grid.tiles.get(&pos)
            && let Ok(t) = tiles.get(entity)
            && t.is_owned()
        {
            return false;
        }
    }
    true
}

#[expect(clippy::too_many_arguments)]
pub fn founding_system(
    mut request: ResMut<FoundNetworkRequest>,
    mut selected: ResMut<SelectedUnit>,
    grid: Res<GridWorld>,
    mut region_states: ResMut<RegionStates>,
    units: Query<(&Unit, &GridPos, &UnitMovement)>,
    mut tiles: Query<&mut Tile>,
    mut founded: MessageWriter<NetworkFounded>,
    mut commands: Commands,
) {
    if !std::mem::take(&mut request.0) {
        return;
    }
    let Some(unit_entity) = selected.0 else {
        return;
    };
    let Ok((unit, gpos, movement)) = units.get(unit_entity) else {
        return;
    };
    if unit.kind != UnitKind::Founder || !movement.path.is_empty() {
        return;
    }
    let seed = gpos.0;
    if !is_valid_site(seed, &grid, &tiles) {
        return;
    }

    let region_id = region_states.create_region();
    if let Some(state) = region_states.get_mut(region_id) {
        state.sugars = FOUNDER_SEED_SUGARS;
    }
    if let Some(&tile_e) = grid.tiles.get(&seed)
        && let Ok(mut tile) = tiles.get_mut(tile_e)
    {
        tile.region_id = Some(region_id);
        tile.biomass = FOUNDER_SEED_BIOMASS;
    }
    // The founder is despawned, so SelectedUnit must be cleared with it; the
    // render selection-ring and unit panel already tolerate a stale Entity.
    commands.entity(unit_entity).despawn();
    selected.0 = None;
    founded.write(NetworkFounded { region_id, seed });
}

#[cfg(test)]
mod tests {
    use super::*;
    use hexx::Hex;
    use kingdom_core::{GridPos, GridWorld, RegionId, TerrainType, Tile};

    fn check_site(app: &mut App, hex: Hex) -> bool {
        let mut sys_state: bevy::ecs::system::SystemState<(Res<GridWorld>, Query<&mut Tile>)> =
            bevy::ecs::system::SystemState::new(app.world_mut());
        let (grid, tiles) = sys_state.get_mut(app.world_mut());
        is_valid_site(hex, &grid, &tiles)
    }

    fn base_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GridWorld>();
        app
    }

    fn add_tile(
        app: &mut App,
        pos: Hex,
        terrain: TerrainType,
        region: Option<RegionId>,
        biomass: f32,
    ) {
        let e = app
            .world_mut()
            .spawn((
                GridPos(pos),
                Tile {
                    terrain,
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

    #[test]
    fn accepts_unclaimed_passable_far_hex() {
        let mut app = base_app();
        // An owned tile far away; the candidate site is unclaimed and passable.
        add_tile(
            &mut app,
            Hex::new(0, 0),
            TerrainType::Soil,
            Some(RegionId(0)),
            1.0,
        );
        add_tile(&mut app, Hex::new(20, 0), TerrainType::Soil, None, 0.0);
        assert!(check_site(&mut app, Hex::new(20, 0)));
    }

    #[test]
    fn rejects_hex_near_an_owned_tile() {
        let mut app = base_app();
        // Owned tile at (0,0); candidate (3,0) is within MIN_FOUNDING_DISTANCE (6).
        add_tile(
            &mut app,
            Hex::new(0, 0),
            TerrainType::Soil,
            Some(RegionId(0)),
            1.0,
        );
        add_tile(&mut app, Hex::new(3, 0), TerrainType::Soil, None, 0.0);
        assert!(!check_site(&mut app, Hex::new(3, 0)));
    }

    #[test]
    fn rejects_claimed_or_impassable_hex() {
        let mut app = base_app();
        add_tile(&mut app, Hex::new(20, 0), TerrainType::Rock, None, 0.0);
        assert!(!check_site(&mut app, Hex::new(20, 0)));
        let mut app2 = base_app();
        add_tile(
            &mut app2,
            Hex::new(20, 0),
            TerrainType::Soil,
            Some(RegionId(0)),
            1.0,
        );
        assert!(!check_site(&mut app2, Hex::new(20, 0)));
    }
}
