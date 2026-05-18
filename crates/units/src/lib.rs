use bevy::prelude::*;
use kingdom_core::{
    FoundNetworkRequest, HiveCaptured, NetworkFounded, SelectedUnit, SimulationSystems,
};
use kingdom_world::region_tracking_system;

mod founding;
mod hive;
mod movement;
mod pathfinding;
mod production;

pub use founding::{founding_system, is_valid_site};
pub use hive::hive_capture_system;
pub use movement::unit_movement_system;
pub use pathfinding::find_path;
pub use production::{hive_production_system, unit_upkeep_system};

pub struct UnitsPlugin;

impl Plugin for UnitsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<HiveCaptured>()
            .init_resource::<SelectedUnit>()
            .add_systems(
                Update,
                (
                    hive_capture_system,
                    hive_production_system,
                    unit_upkeep_system,
                )
                    .chain()
                    .in_set(SimulationSystems)
                    .after(region_tracking_system),
            )
            // Runs every frame, ungated by `SimulationSystems` and with no
            // explicit ordering: units move in real time, decoupled from the
            // simulation tick. A 1-frame ordering jitter against
            // `pointer_system` (which writes the path) is harmless — the unit
            // simply starts moving on the next frame.
            .add_systems(Update, unit_movement_system);

        app.init_resource::<FoundNetworkRequest>()
            .add_message::<NetworkFounded>()
            // Runs every frame, ungated by `SimulationSystems`. The request
            // flag is consumed once per frame: a press with no valid founder
            // selected is simply dropped, and 1-frame ordering jitter against
            // the input/HUD writers is harmless.
            .add_systems(Update, founding_system);
    }
}
