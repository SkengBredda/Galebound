//! Window display mode (Windowed / Windowed Fullscreen / Fullscreen).
//!
//! The choice is stored with the account's settings and also in a small
//! `display.cfg` next to the saves, so the game starts in the right mode
//! before anyone has logged in.
//!
//! Note: the graphics library (miniquad) only has a borderless fullscreen
//! window - it can't switch the monitor into exclusive fullscreen - so
//! "Fullscreen" and "Windowed Fullscreen" currently look the same. The
//! setting is kept separate so true exclusive fullscreen can be added later.

use galebound_core::save::{data_dir, write_atomic, DisplayMode};
use std::cell::Cell;

pub const WINDOW_W: u32 = 1280;
pub const WINDOW_H: u32 = 720;

thread_local! {
    static CURRENT: Cell<Option<DisplayMode>> = const { Cell::new(None) };
}

fn cfg_path() -> std::path::PathBuf {
    data_dir().join("display.cfg")
}

/// The mode saved on this computer (used for the very first frame).
pub fn saved_mode() -> DisplayMode {
    std::fs::read_to_string(cfg_path()).ok().and_then(|s| DisplayMode::from_key(s.trim())).unwrap_or(DisplayMode::Windowed)
}

/// Note which mode the window was created in.
pub fn started_in(mode: DisplayMode) {
    CURRENT.with(|c| c.set(Some(mode)));
}

/// Switch the window to `mode` (does nothing if it's already in it).
pub fn apply(mode: DisplayMode) {
    let cur = CURRENT.with(|c| c.get());
    if cur == Some(mode) {
        return;
    }
    let was_full = matches!(cur, Some(DisplayMode::WindowedFullscreen) | Some(DisplayMode::Fullscreen));
    match mode {
        DisplayMode::Windowed => {
            macroquad::window::set_fullscreen(false);
            if was_full || cur.is_none() {
                // Leaving fullscreen keeps the screen-sized window; put it back to a normal size.
                macroquad::window::request_new_screen_size(WINDOW_W as f32, WINDOW_H as f32);
                macroquad::miniquad::window::set_window_position(80, 60);
            }
        }
        DisplayMode::WindowedFullscreen | DisplayMode::Fullscreen => {
            if !was_full {
                macroquad::window::set_fullscreen(true);
            }
        }
    }
    CURRENT.with(|c| c.set(Some(mode)));
    let _ = write_atomic(&cfg_path(), mode.key());
}
