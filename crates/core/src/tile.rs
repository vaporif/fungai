use bevy::prelude::*;

use crate::region::RegionId;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub struct RivalId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub struct FragmentId(pub u32);

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
