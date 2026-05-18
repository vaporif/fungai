use bevy::prelude::*;

use crate::grid::Hex;
use crate::region::RegionId;
use crate::tile::FragmentId;

#[derive(Component, Clone, Debug, Reflect)]
pub struct FaunaAgent {
    pub health: f32,
    pub damage_per_tick: f32,
}

#[derive(Component, Clone, Debug, Reflect)]
pub struct BacteriaColonyAgent {
    pub spread_timer: u32,
    pub spread_interval: u32,
}

#[derive(Component, Clone, Debug, Reflect)]
pub struct PlantRootAgent {
    pub plant_id: u32,
    pub health: f32,
    pub trade_active: bool,
    pub nutrient_intake: f32,
    pub sugar_output: f32,
    pub neglect_timer: u32,
}

#[derive(Component, Clone, Debug, Reflect)]
pub struct NeutralFungusAgent {
    pub fungus_id: u32,
    pub merge_progress: f32,
}

#[derive(Component, Clone, Debug, Reflect)]
pub struct FragmentAgent {
    pub fragment_id: FragmentId,
    pub fused: bool,
}

#[derive(Component, Clone, Debug, Reflect)]
pub struct FruitingBody {
    pub region_id: RegionId,
    pub fragment_id: FragmentId,
    pub progress: f32,
    #[reflect(ignore)]
    pub column_top: Hex,
}

#[derive(Component, Clone, Debug, Reflect)]
pub struct MushroomEntity {
    pub fragment_id: FragmentId,
    #[reflect(ignore)]
    pub pos: Hex,
    pub vision_radius: f32,
}

#[derive(Component, Debug)]
pub struct OrganismSpriteLink(pub Entity);

#[derive(Component, Clone, Debug, Reflect)]
pub struct Hive {
    /// `None` = neutral; `Some` = the owning network.
    pub captured_by: Option<RegionId>,
    /// 0.0..=1.0 progress toward the next founder.
    pub production: f32,
}

#[derive(Resource, Default)]
pub struct SelectedRegion {
    pub region_id: Option<RegionId>,
    pub selected_pos: Option<Hex>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum UnitKind {
    /// Phase 1 ships only this variant; Scout/Soldier/Builder arrive later.
    Founder,
}

#[derive(Component, Clone, Debug, Reflect)]
pub struct Unit {
    pub kind: UnitKind,
    /// The network that produced the unit; pays its upkeep.
    pub owner: RegionId,
}

#[derive(Component, Clone, Debug, Reflect, Default)]
pub struct UnitMovement {
    /// Remaining hexes to traverse, in order; empty = idle.
    #[reflect(ignore)]
    pub path: Vec<Hex>,
    /// 0.0..1.0 progress along the edge from `GridPos` to `path[0]`.
    pub edge_progress: f32,
}

#[derive(Resource, Default)]
pub struct SelectedUnit(pub Option<Entity>);

/// Set for one frame to request that the selected founder found a network.
#[derive(Resource, Default)]
pub struct FoundNetworkRequest(pub bool);
