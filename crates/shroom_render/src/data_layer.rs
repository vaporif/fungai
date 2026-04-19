use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use shroom_core::*;

#[derive(Resource, Default, Debug)]
pub struct BranchGraph {
    pub nodes: HashMap<IVec2, BranchNode>,
    pub edges: Vec<BranchEdge>,
}

#[derive(Debug)]
pub struct BranchNode {
    pub pos: IVec2,
    pub biomass: f32,
    pub specialization: Option<SpecializationType>,
    pub region_id: RegionId,
}

#[derive(Debug)]
pub struct BranchEdge {
    pub from: IVec2,
    pub to: IVec2,
    pub thickness: f32,
}

#[derive(Resource, Default, Debug)]
pub struct TipPositions {
    pub tips: Vec<(IVec2, Option<SpecializationType>)>,
}

#[derive(Resource, Default, Debug)]
pub struct RegionHulls {
    pub hulls: HashMap<RegionId, Vec<Vec2>>,
}

#[derive(Resource, Default, Debug)]
pub struct DiscoveryMap {
    /// Maps tile position to discovery level (0.0 = fully hidden, 1.0 = fully revealed).
    /// Tiles near the network get higher values; tiles far away get lower values.
    pub discovered: HashMap<IVec2, f32>,
}

#[derive(Resource, Default, Debug)]
pub struct RivalBranchGraph {
    pub nodes: HashMap<IVec2, RivalBranchNode>,
    pub edges: Vec<BranchEdge>,
}

#[derive(Debug)]
pub struct RivalBranchNode {
    pub pos: IVec2,
    pub biomass: f32,
    pub rival_id: RivalId,
}

pub fn extract_branch_graph(
    tiles: Query<(&GridPos, &Tile)>,
    grid: Res<GridWorld>,
    region_states: Res<RegionStates>,
    mut graph: ResMut<BranchGraph>,
) {
    graph.nodes.clear();
    graph.edges.clear();

    for (gpos, tile) in tiles.iter() {
        if let Occupant::Player(rid) = tile.occupant {
            let spec = region_states.get(rid).and_then(|r| r.specialization);
            graph.nodes.insert(
                gpos.0,
                BranchNode {
                    pos: gpos.0,
                    biomass: tile.biomass,
                    specialization: spec,
                    region_id: rid,
                },
            );
        }
    }

    let mut seen_edges: HashSet<(IVec2, IVec2)> = HashSet::default();
    let node_keys: Vec<IVec2> = graph.nodes.keys().copied().collect();
    for pos in node_keys {
        for (npos, _) in grid.neighbors(pos) {
            if graph.nodes.contains_key(&npos) {
                let edge_key = if pos.x < npos.x || (pos.x == npos.x && pos.y < npos.y) {
                    (pos, npos)
                } else {
                    (npos, pos)
                };
                if seen_edges.insert(edge_key) {
                    let from_biomass = graph.nodes[&pos].biomass;
                    let to_biomass = graph.nodes[&npos].biomass;
                    graph.edges.push(BranchEdge {
                        from: pos,
                        to: npos,
                        thickness: (from_biomass + to_biomass) * 0.5,
                    });
                }
            }
        }
    }
}

pub fn extract_tip_positions(
    tips: Query<(&GridPos, &HyphalTip)>,
    region_states: Res<RegionStates>,
    mut tip_positions: ResMut<TipPositions>,
) {
    tip_positions.tips.clear();
    for (gpos, tip) in tips.iter() {
        let spec = region_states
            .get(tip.region_id)
            .and_then(|r| r.specialization);
        tip_positions.tips.push((gpos.0, spec));
    }
}

pub fn extract_region_hulls(tiles: Query<(&GridPos, &Tile)>, mut hulls: ResMut<RegionHulls>) {
    hulls.hulls.clear();

    let mut region_positions: HashMap<RegionId, Vec<Vec2>> = HashMap::default();
    for (gpos, tile) in tiles.iter() {
        if let Occupant::Player(rid) = tile.occupant {
            region_positions
                .entry(rid)
                .or_default()
                .push(gpos.0.as_vec2());
        }
    }

    for (rid, positions) in region_positions {
        if positions.len() < 2 {
            hulls.hulls.insert(rid, positions);
            continue;
        }
        let min_x = positions.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
        let max_x = positions
            .iter()
            .map(|p| p.x)
            .fold(f32::NEG_INFINITY, f32::max);
        let min_y = positions.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
        let max_y = positions
            .iter()
            .map(|p| p.y)
            .fold(f32::NEG_INFINITY, f32::max);
        hulls.hulls.insert(
            rid,
            vec![
                Vec2::new(min_x - 0.5, min_y - 0.5),
                Vec2::new(max_x + 0.5, min_y - 0.5),
                Vec2::new(max_x + 0.5, max_y + 0.5),
                Vec2::new(min_x - 0.5, max_y + 0.5),
            ],
        );
    }
}

pub fn extract_discovery_map(graph: Res<BranchGraph>, mut discovery: ResMut<DiscoveryMap>) {
    discovery.discovered.clear();

    let radius: i32 = 8;
    let fully_hidden_threshold: f32 = 0.05;
    let fully_visible_threshold: f32 = 0.3;

    let mut influence_map: HashMap<IVec2, f32> = HashMap::new();

    for &node_pos in graph.nodes.keys() {
        for dx in -radius..=radius {
            for dy in -radius..=radius {
                let tile = node_pos + IVec2::new(dx, dy);

                // Noise displacement using integer hash
                let noise_x = (tile.x.wrapping_mul(73_856_093) ^ tile.y.wrapping_mul(19_349_663))
                    as f32
                    / (i32::MAX as f32);
                let noise_y = (tile.x.wrapping_mul(19_349_663) ^ tile.y.wrapping_mul(73_856_093))
                    as f32
                    / (i32::MAX as f32);
                let noise_offset = Vec2::new(noise_x, noise_y) * 1.5;

                let displaced = tile.as_vec2() + noise_offset;
                let dist = displaced.distance(node_pos.as_vec2());

                if dist > radius as f32 {
                    continue;
                }

                let influence = 1.0 / (dist * dist + 1.0);
                *influence_map.entry(tile).or_default() += influence;
            }
        }
    }

    for (tile, influence) in &influence_map {
        let discovered = if *influence <= fully_hidden_threshold {
            0.0
        } else if *influence >= fully_visible_threshold {
            1.0
        } else {
            (*influence - fully_hidden_threshold)
                / (fully_visible_threshold - fully_hidden_threshold)
        };

        if discovered > 0.0 {
            discovery.discovered.insert(*tile, discovered);
        }
    }
}

pub fn extract_rival_branch_graph(
    tiles: Query<(&GridPos, &Tile)>,
    grid: Res<GridWorld>,
    mut graph: ResMut<RivalBranchGraph>,
) {
    graph.nodes.clear();
    graph.edges.clear();

    for (gpos, tile) in tiles.iter() {
        if let Occupant::Rival(rid) = tile.occupant {
            graph.nodes.insert(
                gpos.0,
                RivalBranchNode {
                    pos: gpos.0,
                    biomass: tile.biomass,
                    rival_id: rid,
                },
            );
        }
    }

    let mut seen_edges: HashSet<(IVec2, IVec2)> = HashSet::default();
    let node_keys: Vec<IVec2> = graph.nodes.keys().copied().collect();
    for pos in node_keys {
        for (npos, _) in grid.neighbors(pos) {
            if graph.nodes.contains_key(&npos) {
                let edge_key = if pos.x < npos.x || (pos.x == npos.x && pos.y < npos.y) {
                    (pos, npos)
                } else {
                    (npos, pos)
                };
                if seen_edges.insert(edge_key) {
                    let from_biomass = graph.nodes[&pos].biomass;
                    let to_biomass = graph.nodes[&npos].biomass;
                    graph.edges.push(BranchEdge {
                        from: pos,
                        to: npos,
                        thickness: (from_biomass + to_biomass) * 0.5,
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GridWorld>();
        app.init_resource::<RegionStates>();
        app.init_resource::<BranchGraph>();
        app.init_resource::<TipPositions>();
        app.init_resource::<RegionHulls>();
        app
    }

    #[test]
    fn branch_graph_extracts_player_network() {
        let mut app = test_app();
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();

        for x in 0..3 {
            let pos = IVec2::new(x, 0);
            let e = app
                .world_mut()
                .spawn((
                    GridPos(pos),
                    Tile {
                        occupant: Occupant::Player(rid),
                        biomass: 1.0,
                        ..default()
                    },
                ))
                .id();
            app.world_mut()
                .resource_mut::<GridWorld>()
                .tiles
                .insert(pos, e);
        }

        app.add_systems(Update, extract_branch_graph);
        app.update();

        let graph = app.world().resource::<BranchGraph>();
        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.edges.len(), 2);
    }

    #[test]
    fn tip_positions_extracts_tips() {
        let mut app = test_app();
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();

        let pos = IVec2::new(5, 5);
        app.world_mut().spawn((
            GridPos(pos),
            HyphalTip {
                region_id: rid,
                age: 0,
            },
        ));

        app.add_systems(Update, extract_tip_positions);
        app.update();

        let tips = app.world().resource::<TipPositions>();
        assert_eq!(tips.tips.len(), 1);
        assert_eq!(tips.tips[0].0, pos);
    }

    #[test]
    fn region_hulls_produces_bounding_box() {
        let mut app = test_app();
        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();

        for x in 0..3 {
            for y in 0..2 {
                let pos = IVec2::new(x, y);
                app.world_mut().spawn((
                    GridPos(pos),
                    Tile {
                        occupant: Occupant::Player(rid),
                        ..default()
                    },
                ));
            }
        }

        app.add_systems(Update, extract_region_hulls);
        app.update();

        let hulls = app.world().resource::<RegionHulls>();
        assert!(hulls.hulls.contains_key(&rid));
        let hull = &hulls.hulls[&rid];
        assert_eq!(hull.len(), 4); // bounding box has 4 corners
    }

    #[test]
    fn discovery_map_inverse_square_gradient() {
        let mut app = test_app();
        app.init_resource::<DiscoveryMap>();

        let rid = app
            .world_mut()
            .resource_mut::<RegionStates>()
            .create_region();

        let pos = IVec2::new(10, 10);
        let e = app
            .world_mut()
            .spawn((
                GridPos(pos),
                Tile {
                    occupant: Occupant::Player(rid),
                    biomass: 1.0,
                    ..default()
                },
            ))
            .id();
        app.world_mut()
            .resource_mut::<GridWorld>()
            .tiles
            .insert(pos, e);

        app.add_systems(
            Update,
            (extract_branch_graph, extract_discovery_map).chain(),
        );
        app.update();

        let discovery = app.world().resource::<DiscoveryMap>();

        // Center tile: fully discovered
        let center = discovery
            .discovered
            .get(&IVec2::new(10, 10))
            .copied()
            .unwrap_or(0.0);
        assert_eq!(center, 1.0);

        // Distance 2: partially discovered (inverse-square drops quickly)
        let near = discovery
            .discovered
            .get(&IVec2::new(12, 10))
            .copied()
            .unwrap_or(0.0);
        assert!(near > 0.0, "near tile should be discovered: {near}");

        // Distance 7: barely discovered (near edge of radius 8)
        let far = discovery
            .discovered
            .get(&IVec2::new(17, 10))
            .copied()
            .unwrap_or(0.0);
        assert!(far < 0.5, "far tile should be dimly discovered: {far}");

        // Distance 9: outside radius 8
        assert!(
            discovery.discovered.get(&IVec2::new(19, 10)).is_none(),
            "tiles outside radius 8 should not be in the map"
        );
    }

    #[test]
    fn rival_branch_graph_extracts_rival_tiles() {
        let mut app = test_app();
        app.init_resource::<RivalBranchGraph>();

        let rival_id = RivalId(0);
        for x in 0..3 {
            let pos = IVec2::new(x, 5);
            let e = app
                .world_mut()
                .spawn((
                    GridPos(pos),
                    Tile {
                        occupant: Occupant::Rival(rival_id),
                        biomass: 1.0,
                        ..default()
                    },
                ))
                .id();
            app.world_mut()
                .resource_mut::<GridWorld>()
                .tiles
                .insert(pos, e);
        }

        app.add_systems(Update, extract_rival_branch_graph);
        app.update();

        let graph = app.world().resource::<RivalBranchGraph>();
        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.edges.len(), 2);
    }
}
