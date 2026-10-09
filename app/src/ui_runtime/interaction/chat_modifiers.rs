//! Physical modifiers survive gameplay-button resets while chat or mod screens own input.
use super::*;

const MODIFIERS: [KeyCode; 8] = [
    KeyCode::ControlLeft,
    KeyCode::ControlRight,
    KeyCode::SuperLeft,
    KeyCode::SuperRight,
    KeyCode::AltLeft,
    KeyCode::AltRight,
    KeyCode::ShiftLeft,
    KeyCode::ShiftRight,
];

/// Seeds held modifiers from physical buttons before gameplay suppression.
pub(crate) fn capture(held: &mut ButtonInput<KeyCode>, gameplay: &ButtonInput<KeyCode>) {
    for key in MODIFIERS {
        if gameplay.pressed(key) {
            held.press(key);
        }
    }
}

/// Applies one physical modifier edge, retaining the other side independently.
pub(crate) fn track(held: &mut ButtonInput<KeyCode>, input: &KeyboardInput) {
    if !MODIFIERS.contains(&input.key_code) {
        return;
    }
    match input.state {
        ButtonState::Pressed => held.press(input.key_code),
        ButtonState::Released => held.release(input.key_code),
    }
}

pub(super) fn text_modified(held: &ButtonInput<KeyCode>) -> bool {
    MODIFIERS[..6].iter().any(|key| held.pressed(*key))
}

#[cfg(test)]
mod tests;
