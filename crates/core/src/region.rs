use std::collections::HashMap;

use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub struct RegionId(pub u32);

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
