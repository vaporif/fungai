use bevy::prelude::*;

use fungai_core::SimulationSet;

mod combat;
mod environment;
mod organisms;
mod rival;

pub use combat::combat_resolution_system;
pub use environment::{EnvironmentRng, environment_threat_system};
pub use organisms::{bacteria_system, fauna_system, neutral_fungi_system, plant_system};
pub use rival::{RivalRng, RivalState, rival_ai_system};

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum AiSystems {
    Rival,
    Organisms,
    Environment,
    Combat,
}

pub struct RivalAiPlugin;

impl Plugin for RivalAiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RivalRng>()
            .init_resource::<RivalState>()
            .add_systems(Update, rival_ai_system.in_set(AiSystems::Rival));
    }
}

pub struct OrganismsPlugin;

impl Plugin for OrganismsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                neutral_fungi_system,
                plant_system,
                fauna_system,
                bacteria_system,
            )
                .chain()
                .in_set(AiSystems::Organisms),
        );
    }
}

pub struct EnvironmentPlugin;

impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EnvironmentRng>().add_systems(
            Update,
            environment_threat_system.in_set(AiSystems::Environment),
        );
    }
}

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, combat_resolution_system.in_set(AiSystems::Combat));
    }
}

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Update,
            (
                AiSystems::Rival,
                AiSystems::Organisms,
                AiSystems::Environment,
                AiSystems::Combat,
            )
                .chain()
                .in_set(SimulationSet),
        )
        .add_plugins((
            RivalAiPlugin,
            OrganismsPlugin,
            EnvironmentPlugin,
            CombatPlugin,
        ));
    }
}
