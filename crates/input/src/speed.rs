use std::time::Duration;

use bevy::prelude::*;
use fungai_core::{SimulationSpeed, TickTimer};
use leafwing_input_manager::prelude::*;

use crate::action::Action;

pub fn speed_input_system(
    actions: Res<ActionState<Action>>,
    mut speed: ResMut<SimulationSpeed>,
    mut tick_timer: ResMut<TickTimer>,
) {
    let mut changed = false;

    if actions.just_pressed(&Action::TogglePause) {
        *speed = if speed.is_paused() {
            SimulationSpeed::Normal
        } else {
            SimulationSpeed::Paused
        };
        changed = true;
    }

    if actions.just_pressed(&Action::SpeedUp) {
        *speed = speed.speed_up();
        changed = true;
    }

    if actions.just_pressed(&Action::SlowDown) {
        *speed = speed.slow_down();
        changed = true;
    }

    if changed && !speed.is_paused() {
        tick_timer
            .timer
            .set_duration(Duration::from_secs_f32(speed.duration_secs()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::input::InputPlugin as BevyInputPlugin;

    use crate::action::{Action, default_input_map};

    fn setup_app(initial_speed: SimulationSpeed) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(BevyInputPlugin);
        app.add_plugins(InputManagerPlugin::<Action>::default());
        app.insert_resource(default_input_map());
        app.init_resource::<ActionState<Action>>();
        app.insert_resource(initial_speed);
        app.insert_resource(TickTimer::default());
        app.add_systems(Update, speed_input_system);
        app
    }

    #[test]
    fn space_toggles_pause() {
        let mut app = setup_app(SimulationSpeed::Normal);
        KeyCode::Space.press(app.world_mut());
        app.update();
        assert_eq!(
            *app.world().resource::<SimulationSpeed>(),
            SimulationSpeed::Paused
        );
    }

    #[test]
    fn plus_speeds_up() {
        let mut app = setup_app(SimulationSpeed::Normal);
        KeyCode::Equal.press(app.world_mut());
        app.update();
        assert_eq!(
            *app.world().resource::<SimulationSpeed>(),
            SimulationSpeed::Fast
        );
    }

    #[test]
    fn minus_slows_down() {
        let mut app = setup_app(SimulationSpeed::Fast);
        KeyCode::Minus.press(app.world_mut());
        app.update();
        assert_eq!(
            *app.world().resource::<SimulationSpeed>(),
            SimulationSpeed::Normal
        );
    }
}
