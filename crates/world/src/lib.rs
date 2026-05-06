use bevy::prelude::*;
use fungai_core::SimulationSystems;

mod region_tracking;
mod terrain_gen;

pub use region_tracking::region_tracking_system;
pub use terrain_gen::terrain_generation;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, terrain_generation)
            .add_systems(Update, region_tracking_system.in_set(SimulationSystems));
    }
}
