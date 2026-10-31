//! Small immediate-mode UI kit: panels, buttons, checkboxes and text fields,
//! styled like a bright fantasy MMO (parchment panels, wooden borders).

use crate::gfx::{self, alpha, rgb, rgba, shade};
use macroquad::prelude::{
    get_char_pressed, get_frame_time, is_key_down, is_key_pressed, is_mouse_button_pressed, Color,
    KeyCode, MouseButton,
};

pub fn ink() -> Color {
    rgb(54, 40, 30)
}
pub fn ink_soft() -> Color {
    rgb(112, 92, 72)
}
pub fn parchment() -> Color {
    rgb(250, 242, 222)
}
pub fn wood() -> Color {
    rgb(104, 70, 44)
}
pub fn accent() -> Color {
    rgb(64, 156, 120)
}
pub fn danger() -> Color {
    rgb(196, 72, 64)
}
pub fn gold() -> Color {
    rgb(232, 182, 72)
}

/// Clicks are only accepted once per frame; set false while a modal is open
/// to block clicks on what's behind it.
pub struct Ui {
    pub chars: Vec<char>,
    pub clicked: bool,
    pub enabled: bool,
}

impl Ui {
    pub fn begin() -> Ui {
        let mut chars = Vec::new();
        while let Some(c) = get_char_pressed() {
            chars.push(c);
        }
        Ui { chars, clicked: is_mouse_button_pressed(MouseButton::Left), enabled: true }
    }
    fn click_in(&self, x: f32, y: f32, w: f32, h: f32) -> bool {
        self.enabled && self.clicked && gfx::mouse_in(x, y, w, h)
    }
    fn hover(&self, x: f32, y: f32, w: f32, h: f32) -> bool {
        self.enabled && gfx::mouse_in(x, y, w, h)
    }
}

pub fn panel(x: f32, y: f32, w: f32, h: f32) {
    gfx::rrect(x + 4.0, y + 6.0, w, h, 14.0, rgba(0, 0, 0, 60));
    gfx::rrect(x, y, w, h, 14.0, wood());
    gfx::rrect(x + 4.0, y + 4.0, w - 8.0, h - 8.0, 11.0, shade(wood(), 1.25));
    gfx::rrect(x + 8.0, y + 8.0, w - 16.0, h - 16.0, 9.0, parchment());
    // corner studs
    for (cx, cy) in [(x + 14.0, y + 14.0), (x + w - 14.0, y + 14.0), (x + 14.0, y + h - 14.0), (x + w - 14.0, y + h - 14.0)] {
        gfx::circle(cx, cy, 3.5, gold());
    }
}

/// A dark translucent panel for in-game HUD elements.
pub fn hud_panel(x: f32, y: f32, w: f32, h: f32) {
    gfx::rrect(x, y, w, h, 8.0, rgba(20, 24, 40, 190));
    gfx::rect_lines(x + 2.0, y + 2.0, w - 4.0, h - 4.0, 1.0, rgba(255, 255, 255, 40));
}

pub fn title(s: &str, cx: f32, y: f32, size: f32) {
    gfx::text_centered(s, cx, y, size, ink());
}

#[derive(Clone, Copy, PartialEq)]
pub enum ButtonStyle {
    Primary,
    Secondary,
    Danger,
}

pub fn button(ui: &Ui, x: f32, y: f32, w: f32, h: f32, label: &str, style: ButtonStyle, enabled: bool) -> bool {
    let hover = enabled && ui.hover(x, y, w, h);
    let base = match style {
        ButtonStyle::Primary => accent(),
        ButtonStyle::Secondary => rgb(196, 170, 130),
        ButtonStyle::Danger => danger(),
    };
    let base = if !enabled { rgb(170, 164, 156) } else if hover { shade(base, 1.15) } else { base };
    let press = hover && is_mouse_down();
    let off = if press { 2.0 } else { 0.0 };
    gfx::rrect(x, y + 4.0, w, h, 10.0, shade(base, 0.6));
    gfx::rrect(x, y + off, w, h, 10.0, base);
    gfx::rrect(x + 3.0, y + 3.0 + off, w - 6.0, h * 0.42, 7.0, alpha(rgb(255, 255, 255), 0.18));
    let tc = if style == ButtonStyle::Secondary { ink() } else { rgb(255, 255, 255) };
    let size = (h * 0.42).min(24.0);
    gfx::text_centered(label, x + w * 0.5, y + h * 0.5 - size * 0.55 + off, size, tc);
    enabled && ui.click_in(x, y, w, h)
}

fn is_mouse_down() -> bool {
    macroquad::prelude::is_mouse_button_down(MouseButton::Left)
}

/// Small arrow button used by the character creator ("<" / ">").
pub fn arrow_button(ui: &Ui, x: f32, y: f32, size: f32, left: bool) -> bool {
    let hover = ui.hover(x, y, size, size);
    let c = if hover { shade(accent(), 1.15) } else { accent() };
    gfx::rrect(x, y, size, size, size * 0.3, c);
    let m = size * 0.3;
    if left {
        gfx::tri(x + size - m, y + m, x + size - m, y + size - m, x + m, y + size * 0.5, rgb(255, 255, 255));
    } else {
        gfx::tri(x + m, y + m, x + m, y + size - m, x + size - m, y + size * 0.5, rgb(255, 255, 255));
    }
    ui.click_in(x, y, size, size)
}

pub fn checkbox(ui: &Ui, x: f32, y: f32, label: &str, value: bool) -> bool {
    let s = 24.0;
    let w = s + 10.0 + gfx::text_w(label, 18.0);
    let hover = ui.hover(x, y, w, s);
    gfx::rrect(x, y, s, s, 5.0, if hover { shade(wood(), 1.2) } else { wood() });
    gfx::rrect(x + 3.0, y + 3.0, s - 6.0, s - 6.0, 3.0, rgb(255, 252, 244));
    if value {
        gfx::line(x + 6.0, y + 12.0, x + 10.5, y + 17.0, 3.5, accent());
        gfx::line(x + 10.0, y + 17.0, x + 19.0, y + 6.0, 3.5, accent());
    }
    gfx::text(label, x + s + 10.0, y + 3.0, 18.0, ink());
    ui.click_in(x, y, w, s)
}

pub struct TextField {
    pub text: String,
    pub password: bool,
    pub placeholder: &'static str,
    pub max_len: usize,
    pub reveal: bool,
    backspace_hold: f32,
}

impl TextField {
    pub fn new(placeholder: &'static str, password: bool, max_len: usize) -> Self {
        TextField { text: String::new(), password, placeholder, max_len, reveal: false, backspace_hold: 0.0 }
    }

    /// Process typing for the focused field.
    pub fn type_into(&mut self, ui: &Ui) {
        for &c in &ui.chars {
            if !c.is_control() && self.text.chars().count() < self.max_len {
                self.text.push(c);
            }
        }
        let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl) || is_key_down(KeyCode::LeftSuper) || is_key_down(KeyCode::RightSuper);
        if ctrl && is_key_pressed(KeyCode::V) {
            if let Some(clip) = macroquad::miniquad::window::clipboard_get() {
                for c in clip.chars() {
                    if !c.is_control() && self.text.chars().count() < self.max_len {
                        self.text.push(c);
                    }
                }
            }
        }
        if is_key_pressed(KeyCode::Backspace) {
            if ctrl {
                self.text.clear();
            } else {
                self.text.pop();
            }
            self.backspace_hold = 0.0;
        } else if is_key_down(KeyCode::Backspace) {
            self.backspace_hold += get_frame_time();
            if self.backspace_hold > 0.45 {
                self.text.pop();
                self.backspace_hold -= 0.035;
            }
        }
    }

    /// Draw the field; returns true if it was clicked (to take focus).
    pub fn draw(&self, ui: &Ui, x: f32, y: f32, w: f32, h: f32, label: &str, focused: bool, time: f64) -> bool {
        gfx::text(label, x, y - 24.0, 18.0, ink_soft());
        let border = if focused { accent() } else { shade(wood(), 1.3) };
        gfx::rrect(x - 2.0, y - 2.0, w + 4.0, h + 4.0, 9.0, border);
        gfx::rrect(x, y, w, h, 7.0, rgb(255, 253, 248));
        let shown: String = if self.password && !self.reveal {
            "*".repeat(self.text.chars().count())
        } else {
            self.text.clone()
        };
        let size = 20.0;
        let ty = y + h * 0.5 - size * 0.55;
        let max_w = w - 24.0;
        // Scroll long text so the end (caret) is visible.
        let mut vis = shown.clone();
        while gfx::text_w(&vis, size) > max_w && !vis.is_empty() {
            vis.remove(0);
        }
        if self.text.is_empty() && !focused {
            gfx::text(self.placeholder, x + 12.0, ty, size, rgb(170, 160, 150));
        } else {
            gfx::text(&vis, x + 12.0, ty, size, ink());
        }
        if focused && (time * 2.0) as i64 % 2 == 0 {
            let cx = x + 12.0 + gfx::text_w(&vis, size) + 2.0;
            gfx::rect(cx, y + 9.0, 2.0, h - 18.0, ink());
        }
        ui.click_in(x, y, w, h)
    }
}

/// Toast-style error / info line.
pub fn message(s: &str, cx: f32, y: f32, is_error: bool) {
    let c = if is_error { danger() } else { accent() };
    gfx::text_centered(s, cx, y, 18.0, c);
}
