//! Translates the core's key tokens into macroquad KeyCodes and polls actions.

use galebound_core::input::{Action, InputState, Layout, ALL_ACTIONS};
use macroquad::prelude::{is_key_down, is_key_pressed, KeyCode};

pub fn keycode(token: &str) -> Option<KeyCode> {
    use KeyCode::*;
    Some(match token {
        "A" => A,
        "B" => B,
        "C" => C,
        "D" => D,
        "E" => E,
        "F" => F,
        "G" => G,
        "H" => H,
        "I" => I,
        "J" => J,
        "K" => K,
        "L" => L,
        "M" => M,
        "N" => N,
        "O" => O,
        "P" => P,
        "Q" => Q,
        "R" => R,
        "S" => S,
        "T" => T,
        "U" => U,
        "V" => V,
        "W" => W,
        "X" => X,
        "Y" => Y,
        "Z" => Z,
        "Space" => Space,
        "Left" => Left,
        "Right" => Right,
        "Up" => Up,
        "Down" => Down,
        "LeftShift" => LeftShift,
        "RightShift" => RightShift,
        "LeftAlt" => LeftAlt,
        "RightAlt" => RightAlt,
        "LeftControl" => LeftControl,
        "RightControl" => RightControl,
        "Escape" => Escape,
        "Enter" => Enter,
        "Tab" => Tab,
        "Delete" => Delete,
        "End" => End,
        "Key1" => Key1,
        "Key2" => Key2,
        "Key3" => Key3,
        "Key4" => Key4,
        "Key5" => Key5,
        "Key6" => Key6,
        "Key7" => Key7,
        "Key8" => Key8,
        _ => return None,
    })
}

/// Collects input every frame. "Pressed" flags accumulate until a
/// simulation tick consumes them, so no key press is ever lost even when the
/// render rate is faster than the 120 Hz simulation.
#[derive(Default)]
pub struct InputCollector {
    state: InputState,
}

impl InputCollector {
    /// Keyboard, controller and touch controls together.
    pub fn poll(&mut self, layout: Layout) {
        crate::pad::update();
        for &a in ALL_ACTIONS {
            let keys = layout.keys(a);
            let (ph, pp) = crate::pad::action(a);
            let (th, tp) = crate::touch::action(a);
            let held = keys.iter().filter_map(|k| keycode(k)).any(is_key_down) || ph || th;
            let pressed = keys.iter().filter_map(|k| keycode(k)).any(is_key_pressed) || pp || tp;
            self.state.set(a, held, pressed);
        }
    }
    /// Input for the next tick. Call `consumed()` after a tick has used it.
    pub fn current(&self) -> InputState {
        self.state
    }
    pub fn consumed(&mut self) {
        self.state.clear_pressed();
    }
    pub fn pressed(&self, a: Action) -> bool {
        self.state.pressed(a)
    }
}
