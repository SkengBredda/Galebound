//! On-screen touch controls (Android phones and tablets, and any touch
//! screen): a floating joystick on the left, action buttons on the right and
//! a few small buttons for the menu, bag, chat and quest log. Taps
//! elsewhere still act as mouse clicks, so every window works by touch.
//!
//! Also asks Android for its on-screen keyboard when a text box or the chat
//! line has focus.

use crate::gfx::{self, alpha, rgb, rgba};
use galebound_core::input::Action;
use macroquad::prelude::{touches, TouchPhase};
use std::cell::{Cell, RefCell};

/// Joystick: where a thumb may start it, how far it travels.
const STICK_ZONE_X: f32 = 470.0;
const STICK_ZONE_Y: f32 = 330.0;
const STICK_R: f32 = 70.0;

struct Button {
    x: f32,
    y: f32,
    r: f32,
    label: &'static str,
    action: Action,
}

/// The action buttons (virtual 1280x720 coordinates).
const BUTTONS: &[Button] = &[
    Button { x: 1180.0, y: 600.0, r: 58.0, label: "Jump", action: Action::Jump },
    Button { x: 1050.0, y: 640.0, r: 48.0, label: "Attack", action: Action::Light },
    Button { x: 1072.0, y: 520.0, r: 46.0, label: "Heavy", action: Action::Heavy },
    Button { x: 1190.0, y: 468.0, r: 42.0, label: "Dodge", action: Action::Dodge },
    Button { x: 955.0, y: 560.0, r: 38.0, label: "Talk", action: Action::Interact },
    Button { x: 960.0, y: 460.0, r: 34.0, label: "Loot", action: Action::Pickup },
];

/// Small buttons along the right edge.
const SMALL: &[Button] = &[
    Button { x: 1238.0, y: 140.0, r: 24.0, label: "Menu", action: Action::Menu },
    Button { x: 1238.0, y: 196.0, r: 24.0, label: "Bag", action: Action::Inventory },
    Button { x: 1238.0, y: 252.0, r: 24.0, label: "Chat", action: Action::Chat },
    Button { x: 1238.0, y: 308.0, r: 24.0, label: "Quest", action: Action::QuestLog },
];

#[derive(Default)]
struct State {
    /// Touch screen in use (always on Android; on a PC once you touch it).
    seen: bool,
    /// The thumb on the joystick: (touch id, origin, current).
    stick: Option<(u64, (f32, f32), (f32, f32))>,
    held: Vec<Action>,
    pressed: Vec<Action>,
    /// Buttons were shown this frame (hidden while windows are open).
    shown: bool,
}

thread_local! {
    static ST: RefCell<State> = RefCell::new(State { seen: cfg!(target_os = "android"), ..Default::default() });
    static KEYBOARD: Cell<(bool, bool)> = const { Cell::new((false, false)) };
}

/// Turn the touch controls on (screenshots / testing on a PC).
#[allow(dead_code)]
pub fn force_on() {
    ST.with(|s| s.borrow_mut().seen = true);
}

/// Are the touch controls on?
pub fn active() -> bool {
    ST.with(|s| s.borrow().seen)
}

/// Read the fingers. Call once per frame before polling input. `enabled`
/// is false while a window or menu is open (the buttons hide then).
pub fn update(enabled: bool) {
    let ts = touches();
    ST.with(|s| {
        let mut s = s.borrow_mut();
        if !ts.is_empty() {
            s.seen = true;
        }
        s.held.clear();
        s.pressed.clear();
        s.shown = enabled && s.seen;
        if !s.shown {
            s.stick = None;
            return;
        }
        // keep the stick finger, or let it go
        if let Some((id, o, _)) = s.stick {
            match ts.iter().find(|t| t.id == id && !matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled)) {
                Some(t) => s.stick = Some((id, o, gfx::screen_to_virtual(t.position.x, t.position.y))),
                None => s.stick = None,
            }
        }
        for t in &ts {
            if matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                continue;
            }
            if s.stick.map(|st| st.0 == t.id).unwrap_or(false) {
                continue;
            }
            let (x, y) = gfx::screen_to_virtual(t.position.x, t.position.y);
            let started = t.phase == TouchPhase::Started;
            let hit = BUTTONS.iter().chain(SMALL.iter()).find(|b| (x - b.x).powi(2) + (y - b.y).powi(2) < (b.r * 1.15).powi(2));
            match hit {
                Some(b) => {
                    s.held.push(b.action);
                    if started {
                        s.pressed.push(b.action);
                    }
                }
                None if started && s.stick.is_none() && x < STICK_ZONE_X && y > STICK_ZONE_Y => {
                    s.stick = Some((t.id, (x, y), (x, y)));
                }
                None => {}
            }
        }
        if let Some((_, o, c)) = s.stick {
            let (dx, dy) = ((c.0 - o.0) / STICK_R, (c.1 - o.1) / STICK_R);
            for (on, a) in [(dx < -0.35, Action::Left), (dx > 0.35, Action::Right), (dy < -0.6, Action::Up), (dy > 0.6, Action::Down)] {
                if on {
                    s.held.push(a);
                }
            }
        }
    });
}

/// What the touch controls do to one action: (held, pressed).
pub fn action(a: Action) -> (bool, bool) {
    ST.with(|s| {
        let s = s.borrow();
        (s.held.contains(&a), s.pressed.contains(&a))
    })
}

/// Is this screen point on a touch control (so a tap there isn't a click
/// on the world)?
pub fn on_control(x: f32, y: f32) -> bool {
    ST.with(|s| {
        let s = s.borrow();
        s.shown && (BUTTONS.iter().chain(SMALL.iter()).any(|b| (x - b.x).powi(2) + (y - b.y).powi(2) < (b.r * 1.15).powi(2)) || (x < STICK_ZONE_X && y > STICK_ZONE_Y))
    })
}

/// Draw the controls (UI space). Call after the HUD.
pub fn draw() {
    ST.with(|s| {
        let s = s.borrow();
        if !s.shown {
            return;
        }
        gfx::clear_cam();
        // joystick: a faint base where it rests, or under your thumb
        let (o, c) = match s.stick {
            Some((_, o, c)) => {
                let (dx, dy) = (c.0 - o.0, c.1 - o.1);
                let l = (dx * dx + dy * dy).sqrt().max(1.0);
                let k = l.min(STICK_R) / l;
                (o, (o.0 + dx * k, o.1 + dy * k))
            }
            None => ((150.0, 580.0), (150.0, 580.0)),
        };
        let on = s.stick.is_some();
        gfx::circle(o.0, o.1, STICK_R + 8.0, rgba(20, 24, 40, if on { 110 } else { 70 }));
        gfx::circle(o.0, o.1, STICK_R, rgba(255, 255, 255, if on { 40 } else { 24 }));
        gfx::circle(c.0, c.1, 30.0, rgba(255, 255, 255, if on { 170 } else { 90 }));
        if !on {
            gfx::text_centered_outlined("Move", o.0, o.1 + STICK_R + 14.0, 13.0, rgba(230, 236, 250, 160));
        }
        for b in BUTTONS.iter().chain(SMALL.iter()) {
            let held = s.held.contains(&b.action);
            let small = b.r < 30.0;
            let base = match b.action {
                Action::Jump => rgb(90, 170, 240),
                Action::Light => rgb(240, 120, 90),
                Action::Heavy => rgb(230, 170, 70),
                Action::Dodge => rgb(140, 220, 160),
                _ => rgb(170, 176, 200),
            };
            gfx::circle(b.x, b.y, b.r + 3.0, rgba(14, 16, 30, 120));
            gfx::circle(b.x, b.y, b.r, alpha(base, if held { 0.85 } else { 0.42 }));
            gfx::circle(b.x - b.r * 0.25, b.y - b.r * 0.3, b.r * 0.35, rgba(255, 255, 255, if held { 70 } else { 40 }));
            gfx::text_centered_outlined(b.label, b.x, b.y - if small { 7.0 } else { 9.0 }, if small { 12.0 } else { 16.0 }, rgb(255, 255, 255));
        }
    });
}

// ------------------------------------------------------------------ on-screen keyboard

/// A text box wants typing this frame (shows Android's keyboard).
pub fn want_keyboard() {
    KEYBOARD.with(|k| {
        let (_, shown) = k.get();
        k.set((true, shown));
    });
}

/// Show or hide the on-screen keyboard to match. Call once per frame.
pub fn sync_keyboard() {
    KEYBOARD.with(|k| {
        let (want, shown) = k.get();
        if want != shown && cfg!(target_os = "android") {
            macroquad::miniquad::window::show_keyboard(want);
        }
        k.set((false, want));
    });
}
