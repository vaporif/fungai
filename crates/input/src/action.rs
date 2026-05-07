use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

#[derive(Actionlike, Clone, Copy, Hash, PartialEq, Eq, Debug, Reflect)]
pub enum Action {
    #[actionlike(DualAxis)]
    CameraMove,
    #[actionlike(Axis)]
    Zoom,

    SelectTile,
    Paint,

    TogglePause,
    SpeedUp,
    SlowDown,
}

pub fn default_input_map() -> InputMap<Action> {
    let mut map = InputMap::default();

    map.insert_dual_axis(Action::CameraMove, VirtualDPad::wasd());
    map.insert_dual_axis(Action::CameraMove, VirtualDPad::arrow_keys());

    map.insert_axis(Action::Zoom, MouseScrollAxis::Y);

    // SelectTile and Paint share the left mouse button. The wisp state machine
    // disambiguates tap-vs-drag after the fact and emits TileTapped for taps;
    // selection_system reads that message rather than just_pressed(SelectTile).
    map.insert(Action::SelectTile, MouseButton::Left);
    map.insert(Action::Paint, MouseButton::Left);

    map.insert(Action::TogglePause, KeyCode::Space);
    map.insert(Action::SpeedUp, KeyCode::Equal);
    map.insert(Action::SpeedUp, KeyCode::NumpadAdd);
    map.insert(Action::SlowDown, KeyCode::Minus);
    map.insert(Action::SlowDown, KeyCode::NumpadSubtract);

    map
}
