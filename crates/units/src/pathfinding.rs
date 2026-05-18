use hexx::Hex;
use kingdom_core::GridWorld;
use pathfinding::prelude::astar;

/// A* over the hex grid via the `pathfinding` crate. `passable(hex)` decides
/// which tiles a unit may enter. Returns the hexes to traverse from `start`
/// (exclusive) to `goal` (inclusive), or `None` if `goal` is unreachable.
/// `goal` itself must be passable.
pub fn find_path(
    start: Hex,
    goal: Hex,
    grid: &GridWorld,
    passable: impl Fn(Hex) -> bool,
) -> Option<Vec<Hex>> {
    if start == goal || !passable(goal) {
        return None;
    }
    let (path, _cost) = astar(
        &start,
        |&pos| {
            grid.neighbors(pos)
                .filter(|(n, _)| passable(*n))
                .map(|(n, _)| (n, 1u32))
        },
        |&pos| pos.unsigned_distance_to(goal),
        |&pos| pos == goal,
    )?;
    // `astar` includes `start` as `path[0]`; the unit is already standing
    // there, so the move order is everything after it.
    Some(path[1..].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;
    use hexx::Hex;
    use kingdom_core::{GridWorld, TerrainType, Tile};

    fn world_with(
        passable: &[Hex],
        blocked: &[Hex],
    ) -> (GridWorld, std::collections::HashMap<Hex, Tile>) {
        let mut grid = GridWorld::default();
        let mut tiles = std::collections::HashMap::new();
        for (i, &h) in passable.iter().enumerate() {
            grid.tiles.insert(h, Entity::from_bits((i + 1) as u64));
            tiles.insert(
                h,
                Tile {
                    terrain: TerrainType::Soil,
                    ..default()
                },
            );
        }
        for (i, &h) in blocked.iter().enumerate() {
            grid.tiles.insert(h, Entity::from_bits((i + 1000) as u64));
            tiles.insert(
                h,
                Tile {
                    terrain: TerrainType::Rock,
                    ..default()
                },
            );
        }
        (grid, tiles)
    }

    #[test]
    fn finds_a_straight_path() {
        let line: Vec<Hex> = (0..5).map(|q| Hex::new(q, 0)).collect();
        let (grid, tiles) = world_with(&line, &[]);
        let path = find_path(Hex::new(0, 0), Hex::new(4, 0), &grid, |h| {
            tiles.get(&h).is_some_and(|t| t.terrain.is_passable())
        });
        let path = path.expect("path exists");
        assert_eq!(*path.last().unwrap(), Hex::new(4, 0));
        assert!(
            !path.contains(&Hex::new(0, 0)),
            "path excludes the start hex"
        );
    }

    #[test]
    fn returns_none_for_unreachable_target() {
        let (grid, tiles) = world_with(&[Hex::new(0, 0)], &[Hex::new(5, 5)]);
        let path = find_path(Hex::new(0, 0), Hex::new(5, 5), &grid, |h| {
            tiles.get(&h).is_some_and(|t| t.terrain.is_passable())
        });
        assert!(path.is_none());
    }
}
