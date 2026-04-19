use std::collections::HashMap;

use bevy::ecs::message::Message;
use bevy::prelude::*;

pub use hexx::{Hex, HexLayout, HexOrientation, OffsetHexMode};

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(create_hex_layout())
            .init_resource::<GridWorld>()
            .init_resource::<RegionStates>()
            .init_resource::<GameState>()
            .init_resource::<TickTimer>()
            .init_resource::<SimulationSpeed>()
            .init_resource::<GamePhase>()
            .init_resource::<MutationSelection>()
            .init_resource::<SporeAction>()
            .init_resource::<ActiveAbilityEffects>()
            .init_resource::<TerrainSpriteMap>()
            .init_resource::<HintsVisible>()
            .add_message::<TurnAdvanced>()
            .add_message::<TileDiscovered>()
            .add_message::<StudyComplete>()
            .add_message::<DecompositionComplete>()
            .add_message::<FragmentFused>()
            .add_message::<SlotMachineTriggered>()
            .add_message::<NeutralFungiMerged>();
    }
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GridPos(pub Hex);

#[derive(Resource, Default, Debug, Clone, Reflect)]
pub struct GridWorld {
    #[reflect(ignore)]
    pub tiles: HashMap<Hex, Entity>,
    pub width: i32,
    pub height: i32,
}

impl GridWorld {
    pub fn neighbors(&self, pos: Hex) -> impl Iterator<Item = (Hex, Entity)> + '_ {
        pos.all_neighbors()
            .into_iter()
            .filter_map(move |n| self.tiles.get(&n).map(|&e| (n, e)))
    }
}

#[must_use]
pub fn create_hex_layout() -> HexLayout {
    HexLayout {
        orientation: HexOrientation::Pointy,
        origin: hexx::Vec2::ZERO,
        scale: hexx::Vec2::splat(28.0),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect, Default)]
pub enum TerrainType {
    #[default]
    Soil,
    Rock,
    Water,
    Root,
    Ruin,
    Toxic,
    Surface,
}

impl TerrainType {
    pub fn is_passable(&self) -> bool {
        matches!(self, Self::Soil | Self::Root | Self::Ruin | Self::Surface)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect, Default)]
pub enum Occupant {
    #[default]
    Empty,
    Player(RegionId),
    Rival(RivalId),
}

impl Occupant {
    pub fn is_player(&self) -> bool {
        matches!(self, Self::Player(_))
    }

    pub fn is_rival(&self) -> bool {
        matches!(self, Self::Rival(_))
    }

    pub fn region_id(&self) -> Option<RegionId> {
        match self {
            Self::Player(id) => Some(*id),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub struct RegionId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub struct RivalId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub struct FragmentId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub enum TileContents {
    OrganicMatter,
    Mineral,
    Artifact,
    Fragment(FragmentId),
    UniqueDecomposable(u32),
    NeutralFungus(u32),
    PlantRoot(u32),
}

#[derive(Component, Clone, Debug, Reflect)]
pub struct Tile {
    pub terrain: TerrainType,
    pub occupant: Occupant,
    pub nutrient_level: f32,
    pub moisture: f32,
    pub discovered: bool,
    pub contents: Option<TileContents>,
    pub biomass: f32,
    pub nutrient_gradient: Vec2,
    pub priority_bias: Vec2,
}

impl Default for Tile {
    fn default() -> Self {
        Self {
            terrain: TerrainType::Soil,
            occupant: Occupant::Empty,
            nutrient_level: 0.5,
            moisture: 0.5,
            discovered: false,
            contents: None,
            biomass: 0.0,
            nutrient_gradient: Vec2::ZERO,
            priority_bias: Vec2::ZERO,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub enum SpecializationType {
    Decomposer,
    Parasite,
    Symbiont,
    Infiltrator,
    Hunter,
    Transporter,
    Explorer,
    Researcher,
}

pub const SPEC_TIER_1: f32 = 100.0;
pub const SPEC_TIER_2: f32 = 300.0;
pub const SPEC_TIER_3: f32 = 600.0;

#[derive(Clone, Debug, Reflect)]
pub struct RegionState {
    pub region_id: RegionId,
    pub specialization: Option<SpecializationType>,
    pub target_specialization: Option<SpecializationType>,
    pub nutrients: f32,
    pub energy: f32,
    pub biomass: f32,
    pub specialization_investment: f32,
    pub tile_count: u32,
    pub nutrient_bonus: f32,
}

impl RegionState {
    pub fn new(id: RegionId) -> Self {
        Self {
            region_id: id,
            specialization: None,
            target_specialization: None,
            nutrients: 10.0,
            energy: 0.0,
            biomass: 0.0,
            specialization_investment: 0.0,
            tile_count: 0,
            nutrient_bonus: 0.0,
        }
    }

    pub fn tier(&self) -> u8 {
        if self.specialization_investment >= SPEC_TIER_3 {
            3
        } else if self.specialization_investment >= SPEC_TIER_2 {
            2
        } else if self.specialization_investment >= SPEC_TIER_1 {
            1
        } else {
            0
        }
    }
}

#[derive(Resource, Default, Debug, Clone, Reflect)]
pub struct RegionStates {
    pub regions: HashMap<RegionId, RegionState>,
    next_id: u32,
}

impl RegionStates {
    pub fn create_region(&mut self) -> RegionId {
        let id = RegionId(self.next_id);
        self.next_id += 1;
        self.regions.insert(id, RegionState::new(id));
        id
    }

    pub fn get(&self, id: RegionId) -> Option<&RegionState> {
        self.regions.get(&id)
    }

    pub fn get_mut(&mut self, id: RegionId) -> Option<&mut RegionState> {
        self.regions.get_mut(&id)
    }

    pub fn remove(&mut self, id: RegionId) -> Option<RegionState> {
        self.regions.remove(&id)
    }
}

#[derive(Component, Clone, Debug, Reflect)]
pub struct HyphalTip {
    pub region_id: RegionId,
    pub age: u32,
}

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

#[derive(Clone, Debug, Reflect, PartialEq)]
pub struct ActiveAbility {
    pub name: String,
    pub energy_cost: f32,
    pub cooldown_max: u32,
    pub cooldown_remaining: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub enum UnlockPool {
    Organic,
    Mineral,
    Ruins,
    Decomposition,
}

#[derive(Clone, Debug, Reflect)]
pub struct UnlockOption {
    pub name: String,
    pub description: String,
    pub pool: UnlockPool,
}

#[derive(Resource, Default, Debug, Reflect)]
pub struct GameState {
    pub turn: u32,
    pub paused: bool,
    pub fragments_total: u32,
    pub fragments_fused: u32,
    pub mushrooms_fruited: u32,
    pub mushrooms_required: u32,
}

impl GameState {
    pub fn victory(&self) -> bool {
        self.fragments_fused >= self.fragments_total
            && self.mushrooms_fruited >= self.mushrooms_required
            && self.fragments_total > 0
    }
}

#[derive(Message)]
pub struct TurnAdvanced;

#[derive(Message)]
pub struct TileDiscovered {
    pub pos: Hex,
    pub contents: Option<TileContents>,
}

#[derive(Message)]
pub struct StudyComplete {
    pub pos: Hex,
    pub pool: UnlockPool,
}

#[derive(Message)]
pub struct DecompositionComplete {
    pub pos: Hex,
}

#[derive(Message)]
pub struct FragmentFused {
    pub fragment_id: FragmentId,
}

#[derive(Message)]
pub struct SlotMachineTriggered {
    pub pool: UnlockPool,
    pub options: Vec<UnlockOption>,
}

pub const BIOMASS_FLIP_RATIO: f32 = 1.5;
pub const ANASTOMOSIS_BIOMASS_BONUS: f32 = 0.5;
pub const MUSHROOM_MOISTURE_BONUS: f32 = 0.2;
pub const MUSHROOM_MOISTURE_RADIUS: i32 = 5;
pub const SPORE_RELAY_ACCURACY_RADIUS: i32 = 5;
pub const BACTERIA_BIOMASS_BLOCK_THRESHOLD: f32 = 5.0;
pub const TRADE_LINK_NEGLECT_LIMIT: u32 = 20;

// --- Tick System ---

#[derive(Resource)]
pub struct TickTimer {
    pub timer: Timer,
}

impl Default for TickTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(1.0, TimerMode::Repeating),
        }
    }
}

#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum SimulationSpeed {
    Paused,
    #[default]
    Normal,
    Fast,
    Fastest,
}

impl SimulationSpeed {
    #[must_use]
    pub fn duration_secs(self) -> f32 {
        match self {
            Self::Paused => 1.0,
            Self::Normal => 1.0,
            Self::Fast => 0.5,
            Self::Fastest => 0.25,
        }
    }

    #[must_use]
    pub fn is_paused(self) -> bool {
        matches!(self, Self::Paused)
    }

    #[must_use]
    pub fn cycle_next(self) -> Self {
        match self {
            Self::Paused => Self::Normal,
            Self::Normal => Self::Fast,
            Self::Fast => Self::Fastest,
            Self::Fastest => Self::Paused,
        }
    }

    #[must_use]
    pub fn speed_up(self) -> Self {
        match self {
            Self::Paused => Self::Normal,
            Self::Normal => Self::Fast,
            Self::Fast | Self::Fastest => Self::Fastest,
        }
    }

    #[must_use]
    pub fn slow_down(self) -> Self {
        match self {
            Self::Paused | Self::Normal => Self::Paused,
            Self::Fast => Self::Normal,
            Self::Fastest => Self::Fast,
        }
    }

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Paused => "\u{23f8} Paused",
            Self::Normal => "\u{25b6} 1x",
            Self::Fast => "\u{25b6}\u{25b6} 2x",
            Self::Fastest => "\u{25b6}\u{25b6}\u{25b6} 4x",
        }
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SimulationSet;

// --- Game Phase ---

#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum GamePhase {
    #[default]
    Title,
    Playing,
    Victory,
    Defeat,
    Restarting,
}

// --- Mutation Selection (Task 2) ---

#[derive(Resource, Default, Debug, Clone, Reflect)]
pub struct MutationSelection {
    pub selected_index: Option<usize>,
}

// --- Spore Action (Task 2) ---

#[derive(Resource, Debug, Clone, Reflect)]
pub struct SporeAction {
    pub cooldown_remaining: u32,
    pub cooldown_max: u32,
    pub triggered: bool,
}

impl Default for SporeAction {
    fn default() -> Self {
        Self {
            cooldown_remaining: 0,
            cooldown_max: 10,
            triggered: false,
        }
    }
}

// --- Active Ability Effects (Task 2) ---

#[derive(Resource, Default, Debug, Clone, Reflect)]
pub struct ActiveAbilityEffects {
    pub effects: Vec<ActiveEffect>,
}

#[derive(Debug, Clone, Reflect)]
pub struct ActiveEffect {
    pub region_id: RegionId,
    pub effect_type: AbilityEffectType,
    pub ticks_remaining: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum AbilityEffectType {
    DoubleNutrientProduction,
    StealBiomass,
    RevealRadius,
    DoubleTradeEnergy,
    KillFauna,
    InfiltrateRival,
    DoubleTransport,
    DoubleStudySpeed,
}

#[derive(Message)]
pub struct NeutralFungiMerged {
    pub fungus_id: u32,
    pub region_id: RegionId,
}

#[derive(Resource, Default, Debug)]
pub struct TerrainSpriteMap {
    pub sprites: HashMap<Hex, Entity>,
}

#[derive(Component, Debug)]
pub struct OrganismSpriteLink(pub Entity);

// --- Hints Visibility (Task 5) ---

#[derive(Resource, Debug, Reflect)]
pub struct HintsVisible(pub bool);

impl Default for HintsVisible {
    fn default() -> Self {
        Self(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_timer_defaults_to_one_second_repeating() {
        let tick = TickTimer::default();
        assert_eq!(tick.timer.duration().as_secs_f32(), 1.0);
        assert!(tick.timer.mode() == TimerMode::Repeating);
    }

    #[test]
    fn simulation_speed_duration() {
        assert_eq!(SimulationSpeed::Normal.duration_secs(), 1.0);
        assert_eq!(SimulationSpeed::Fast.duration_secs(), 0.5);
        assert_eq!(SimulationSpeed::Fastest.duration_secs(), 0.25);
    }

    #[test]
    fn simulation_speed_is_paused() {
        assert!(SimulationSpeed::Paused.is_paused());
        assert!(!SimulationSpeed::Normal.is_paused());
    }

    #[test]
    fn grid_pos_wraps_hex() {
        let h = Hex::new(3, -2);
        let gp = GridPos(h);
        assert_eq!(gp.0, h);
        assert_eq!(gp.0.x, 3);
        assert_eq!(gp.0.y, -2);
    }

    #[test]
    fn grid_world_neighbors_returns_six_when_all_exist() {
        let mut world = GridWorld::default();
        let center = Hex::ZERO;

        // Insert center and all 6 neighbors
        world.tiles.insert(center, Entity::from_bits(1));
        for (i, neighbor) in center.all_neighbors().into_iter().enumerate() {
            world
                .tiles
                .insert(neighbor, Entity::from_bits((i + 2) as u64));
        }

        let neighbors: Vec<_> = world.neighbors(center).collect();
        assert_eq!(neighbors.len(), 6);

        // Every returned position should be an actual hex neighbor of center
        for (pos, _entity) in &neighbors {
            assert!(center.all_neighbors().contains(pos));
        }
    }

    #[test]
    fn grid_world_neighbors_excludes_missing_tiles() {
        let mut world = GridWorld::default();
        let center = Hex::ZERO;
        world.tiles.insert(center, Entity::from_bits(1));

        // Insert only 2 of 6 neighbors
        let all = center.all_neighbors();
        world.tiles.insert(all[0], Entity::from_bits(2));
        world.tiles.insert(all[3], Entity::from_bits(3));

        let neighbors: Vec<_> = world.neighbors(center).collect();
        assert_eq!(neighbors.len(), 2);
    }

    #[test]
    fn hex_layout_coordinate_round_trip() {
        let layout = create_hex_layout();
        let original = Hex::new(5, -3);
        let world_pos = layout.hex_to_world_pos(original);
        let recovered = layout.world_pos_to_hex(world_pos);
        assert_eq!(recovered, original);
    }
}
