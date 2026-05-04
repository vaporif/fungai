use bevy::prelude::*;
use fungai_core::{RegionStates, SpecializationType};

use crate::SelectedRegion;

/// Keys 1-8 assign a target specialization to the selected region.
pub fn specialization_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    selected: Res<SelectedRegion>,
    mut region_states: ResMut<RegionStates>,
) {
    const KEY_SPECS: &[(KeyCode, SpecializationType)] = &[
        (KeyCode::Digit1, SpecializationType::Decomposer),
        (KeyCode::Digit2, SpecializationType::Parasite),
        (KeyCode::Digit3, SpecializationType::Symbiont),
        (KeyCode::Digit4, SpecializationType::Explorer),
        (KeyCode::Digit5, SpecializationType::Hunter),
        (KeyCode::Digit6, SpecializationType::Transporter),
        (KeyCode::Digit7, SpecializationType::Infiltrator),
        (KeyCode::Digit8, SpecializationType::Researcher),
    ];

    let Some(target) = KEY_SPECS
        .iter()
        .copied()
        .find_map(|(key, spec)| keyboard.just_pressed(key).then_some(spec))
    else {
        return;
    };

    let Some(rid) = selected.region_id else {
        return;
    };
    let Some(state) = region_states.get_mut(rid) else {
        return;
    };

    state.target_specialization = Some(target);
}
