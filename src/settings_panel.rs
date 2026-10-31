//! Controls & settings modal: choose a keyboard layout (WASD / Arrow keys /
//! Classic MMO), see every binding, and toggle a few gameplay options.

use crate::gfx::{self, rgb, rgba};
use crate::widgets::{self, ButtonStyle, Ui};
use galebound_core::input::{ALL_ACTIONS, ALL_LAYOUTS, HOTBAR_ACTIONS};
use galebound_core::save::Settings;
use macroquad::prelude::{is_key_pressed, KeyCode};

pub struct SettingsPanel {
    opened_frames: u32,
}

impl SettingsPanel {
    pub fn new() -> Self {
        SettingsPanel { opened_frames: 0 }
    }

    /// Returns (settings changed, close requested).
    pub fn frame(&mut self, s: &mut Settings) -> (bool, bool) {
        self.opened_frames += 1;
        let ui = Ui::begin();
        let mut changed = false;
        let mut close = false;
        gfx::rect(0.0, 0.0, gfx::VW, gfx::VH, rgba(10, 14, 30, 170));
        let (px, py, pw, ph) = (170.0, 50.0, 940.0, 620.0);
        widgets::panel(px, py, pw, ph);
        widgets::title("Controls & Settings", px + pw * 0.5, py + 22.0, 30.0);

        // Layout choice
        gfx::text("Keyboard layout", px + 40.0, py + 78.0, 20.0, widgets::ink());
        let bw = 200.0;
        for (i, &l) in ALL_LAYOUTS.iter().enumerate() {
            let bx = px + 40.0 + i as f32 * (bw + 16.0);
            let style = if s.layout == l { ButtonStyle::Primary } else { ButtonStyle::Secondary };
            if widgets::button(&ui, bx, py + 108.0, bw, 44.0, l.name(), style, true) && s.layout != l {
                s.layout = l;
                changed = true;
            }
        }

        // Binding table (three columns; the hotbar keys share one row)
        let ty = py + 178.0;
        gfx::rrect(px + 40.0, ty - 8.0, 630.0, 370.0, 8.0, rgb(242, 232, 208));
        let shown: Vec<_> = ALL_ACTIONS.iter().copied().filter(|a| !HOTBAR_ACTIONS[1..].contains(a)).collect();
        let per = shown.len().div_ceil(3);
        for (i, &a) in shown.iter().enumerate() {
            let col = i / per;
            let row = i % per;
            let x = px + 54.0 + col as f32 * 208.0;
            let y = ty + 2.0 + row as f32 * 52.0;
            let hotbar = a == HOTBAR_ACTIONS[0];
            gfx::text(if hotbar { "Hotbar slots" } else { a.label() }, x, y, 15.0, widgets::ink_soft());
            let keys: Vec<String> = if hotbar { vec!["1 - 8".to_string()] } else { s.layout.keys(a).iter().map(|k| pretty(k)).collect() };
            let mut kx = x;
            for k in keys.iter().take(2) {
                let w = gfx::text_w(k, 17.0) + 18.0;
                gfx::rrect(kx, y + 20.0, w, 26.0, 5.0, rgb(70, 60, 54));
                gfx::rrect(kx, y + 18.0, w, 26.0, 5.0, rgb(250, 248, 240));
                gfx::text(k, kx + 9.0, y + 22.0, 17.0, widgets::ink());
                kx += w + 8.0;
            }
        }

        // Options
        let ox = px + 690.0;
        let mut oy = py + 178.0;
        gfx::text("Options", ox, oy - 4.0, 20.0, widgets::ink());
        oy += 34.0;
        if widgets::checkbox(&ui, ox, oy, "Up also jumps", s.tap_up_to_jump) {
            s.tap_up_to_jump = !s.tap_up_to_jump;
            changed = true;
        }
        oy += 40.0;
        if widgets::checkbox(&ui, ox, oy, "Damage numbers", s.show_damage_numbers) {
            s.show_damage_numbers = !s.show_damage_numbers;
            changed = true;
        }
        oy += 40.0;
        if widgets::checkbox(&ui, ox, oy, "Screen shake", s.screen_shake) {
            s.screen_shake = !s.screen_shake;
            changed = true;
        }
        oy += 56.0;
        let tips = "Combat tips: Light and Heavy attacks change with the direction you hold \
                    and whether you're airborne. Heavy in the air is your recovery - one per jump. \
                    Dodge gives a moment of invulnerability.";
        gfx::text_wrapped(tips, ox, oy, 210.0, 14.0, widgets::ink_soft());

        if (widgets::button(&ui, px + pw * 0.5 - 110.0, py + ph - 72.0, 220.0, 48.0, "Done", ButtonStyle::Primary, true)
            || is_key_pressed(KeyCode::Escape))
            && self.opened_frames > 1
        {
            close = true;
        }
        (changed, close)
    }
}

fn pretty(k: &str) -> String {
    match k {
        "LeftShift" => "L-Shift".into(),
        "RightShift" => "R-Shift".into(),
        "LeftAlt" => "L-Alt".into(),
        "RightAlt" => "R-Alt".into(),
        "LeftControl" => "L-Ctrl".into(),
        "RightControl" => "R-Ctrl".into(),
        "Escape" => "Esc".into(),
        "Left" => "Left".into(),
        "Right" => "Right".into(),
        "Up" => "Up".into(),
        "Down" => "Down".into(),
        other => other.into(),
    }
}
