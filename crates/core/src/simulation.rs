use std::collections::HashMap;

use bevy::prelude::*;

use crate::grid::Hex;

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

#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum GamePhase {
    #[default]
    Title,
    Playing,
    Victory,
    Defeat,
    Restarting,
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

#[derive(Resource, Default, Debug)]
pub struct TerrainSpriteMap {
    pub sprites: HashMap<Hex, Entity>,
}

#[derive(Resource, Debug, Reflect)]
pub struct HintsVisible(pub bool);

impl Default for HintsVisible {
    fn default() -> Self {
        Self(true)
    }
}
