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

    Spec1,
    Spec2,
    Spec3,
    Spec4,
    Spec5,
    Spec6,
    Spec7,
    Spec8,

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

    map.insert(Action::Spec1, KeyCode::Digit1);
    map.insert(Action::Spec2, KeyCode::Digit2);
    map.insert(Action::Spec3, KeyCode::Digit3);
    map.insert(Action::Spec4, KeyCode::Digit4);
    map.insert(Action::Spec5, KeyCode::Digit5);
    map.insert(Action::Spec6, KeyCode::Digit6);
    map.insert(Action::Spec7, KeyCode::Digit7);
    map.insert(Action::Spec8, KeyCode::Digit8);

    map.insert(Action::TogglePause, KeyCode::Space);
    map.insert(Action::SpeedUp, KeyCode::Equal);
    map.insert(Action::SpeedUp, KeyCode::NumpadAdd);
    map.insert(Action::SlowDown, KeyCode::Minus);
    map.insert(Action::SlowDown, KeyCode::NumpadSubtract);

    map
}
