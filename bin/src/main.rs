use bevy::prelude::*;

use fungai_ai::AiPlugin;
use fungai_core::{CorePlugin, SimulationPlugin};
use fungai_fruiting::FruitingPlugin;
use fungai_growth::GrowthPlugin;
use fungai_input::InputPlugin;
use fungai_regions::RegionsPlugin;
use fungai_render::RenderPlugin;
use fungai_ui::UiPlugin;
use fungai_world::WorldPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((
            CorePlugin,
            SimulationPlugin,
            WorldPlugin,
            GrowthPlugin,
            RegionsPlugin,
            RenderPlugin,
            InputPlugin,
            AiPlugin,
            FruitingPlugin,
            UiPlugin,
        ))
        .run();
}
