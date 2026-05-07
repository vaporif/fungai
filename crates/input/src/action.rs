use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

#[derive(Actionlike, Clone, Copy, Hash, PartialEq, Eq, Debug, Reflect)]
pub enum Action {
    #[actionlike(DualAxis)]
    CameraMove,
    #[actionlike(Axis)]
    Zoom,

    SelectTile,

    SetPriority,
    ClearPriority,

    TogglePause,
    SpeedUp,
    SlowDown,
}

pub fn default_input_map() -> InputMap<Action> {
    let mut map = InputMap::default();

    map.insert_dual_axis(Action::CameraMove, VirtualDPad::wasd());
    map.insert_dual_axis(Action::CameraMove, VirtualDPad::arrow_keys());

    map.insert_axis(Action::Zoom, MouseScrollAxis::Y);

    map.insert(Action::SelectTile, MouseButton::Left);

    // PrioritizeLongest (the leafwing default) suppresses SetPriority whenever
    // the Shift+P chord matches, so plain P never fires as a clear.
    map.insert(Action::SetPriority, KeyCode::KeyP);
    map.insert(
        Action::ClearPriority,
        ButtonlikeChord::modified(ModifierKey::Shift, KeyCode::KeyP),
    );

    map.insert(Action::TogglePause, KeyCode::Space);
    map.insert(Action::SpeedUp, KeyCode::Equal);
    map.insert(Action::SpeedUp, KeyCode::NumpadAdd);
    map.insert(Action::SlowDown, KeyCode::Minus);
    map.insert(Action::SlowDown, KeyCode::NumpadSubtract);

    map
}
