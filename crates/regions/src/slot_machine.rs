use bevy::ecs::message::{Message, MessageWriter};
use bevy::prelude::*;
use kingdom_core::{UnlockOption, UnlockPool};
use rand::SeedableRng;
use rand::rngs::StdRng;

#[derive(Message)]
pub struct SlotMachineTriggered {
    pub pool: UnlockPool,
    pub options: Vec<UnlockOption>,
}

#[derive(Resource)]
pub struct SlotMachineRng(pub StdRng);

impl Default for SlotMachineRng {
    fn default() -> Self {
        Self(StdRng::seed_from_u64(7))
    }
}

#[allow(unused_variables, clippy::needless_pass_by_value)]
pub fn slot_machine_system(
    slot_messages: MessageWriter<SlotMachineTriggered>,
    rng: ResMut<SlotMachineRng>,
) {
    // T4 will wire this to DecompositionComplete.
}
