use bevy::prelude::*;

use shroom_ai::AiPlugin;
use shroom_core::CorePlugin;
use shroom_fruiting::FruitingPlugin;
use shroom_growth::GrowthPlugin;
use shroom_input::InputPlugin;
use shroom_regions::RegionsPlugin;
use shroom_render::RenderPlugin;
use shroom_ui::UiPlugin;
use shroom_world::WorldPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((
            CorePlugin,
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
