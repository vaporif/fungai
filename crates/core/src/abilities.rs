use bevy::prelude::*;

use crate::region::RegionId;

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

#[derive(Resource, Default, Debug, Clone, Reflect)]
pub struct MutationSelection {
    pub selected_index: Option<usize>,
}

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
