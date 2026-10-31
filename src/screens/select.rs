//! Character select: up to 3 slots, create / delete / play, settings, log out.

use crate::gfx::{self, rgb, rgba};
use crate::charsprite::{self, Anim};
use crate::render::{self, Camera};
use crate::screens::{create::CreateScreen, login::LoginScreen, play::PlayScreen};
use crate::settings_panel::SettingsPanel;
use crate::widgets::{self, ButtonStyle, Ui};
use crate::{App, Go, Screen};
use galebound_core::save::MAX_CHARACTERS;
use galebound_core::world::{self, Theme};
use macroquad::prelude::{is_key_pressed, KeyCode};

pub struct SelectScreen {
    selected: usize,
    confirm_delete: bool,
    settings: Option<SettingsPanel>,
}

impl SelectScreen {
    pub fn new(_app: &App) -> Self {
        SelectScreen { selected: 0, confirm_delete: false, settings: None }
    }

    pub fn select_slot(&mut self, i: usize) {
        self.selected = i;
    }

    pub fn select_last(&mut self, app: &App) {
        self.selected = app.account.as_ref().map(|a| a.characters.len().saturating_sub(1)).unwrap_or(0);
    }

    pub fn frame(&mut self, app: &mut App) -> Go {
        gfx::begin_frame(rgb(120, 180, 240));
        let t = app.time as f32;
        render::background(Theme::Town, &Camera { x: t * 20.0, y: 0.0 }, t);
        gfx::clear_cam();
        gfx::rect(0.0, 0.0, gfx::VW, gfx::VH, rgba(20, 30, 60, 70));

        let Some(acc) = app.account.as_ref() else {
            return Go::To(Screen::Login(LoginScreen::new(app)));
        };
        let chars = acc.characters.clone();
        let email = acc.email.clone();
        if self.selected >= chars.len().max(1) {
            self.selected = chars.len().saturating_sub(1);
        }

        let mut ui = Ui::begin();
        if self.confirm_delete || self.settings.is_some() {
            ui.enabled = false;
        }

        gfx::text_centered_outlined("Choose Your Character", gfx::VW * 0.5, 34.0, 40.0, rgb(255, 244, 200));
        gfx::text_centered_outlined(&format!("Signed in as {}", email), gfx::VW * 0.5, 84.0, 17.0, rgb(230, 240, 255));

        // Slots
        let slot_w = 300.0;
        let gap = 30.0;
        let total = MAX_CHARACTERS as f32 * slot_w + (MAX_CHARACTERS as f32 - 1.0) * gap;
        let x0 = (gfx::VW - total) * 0.5;
        let sy = 130.0;
        let sh = 420.0;
        let mut go_create = false;
        for i in 0..MAX_CHARACTERS {
            let x = x0 + i as f32 * (slot_w + gap);
            let sel = i == self.selected && i < chars.len();
            if sel {
                gfx::rrect(x - 6.0, sy - 6.0, slot_w + 12.0, sh + 12.0, 18.0, widgets::gold());
            }
            widgets::panel(x, sy, slot_w, sh);
            if let Some(c) = chars.get(i) {
                let info = c.class.info();
                // Pedestal + character
                gfx::ellipse(x + slot_w * 0.5, sy + 252.0, 80.0, 18.0, 0.0, rgba(120, 90, 60, 60));
                let frame = charsprite::frame_for_loop(Anim::Idle, t + i as f32 * 0.3);
                let weapon = c.inventory.equipped(galebound_core::items::EquipSlot::Weapon).and_then(|d| galebound_core::items::weapon_art(d.id));
                let look = crate::windows::gear_look_save(&c.look, c);
                render::char_ui(&look, weapon, Anim::Idle, frame, x + slot_w * 0.5, sy + 250.0, 2.0, false);
                widgets::title(&c.name, x + slot_w * 0.5, sy + 290.0, 26.0);
                gfx::text_centered(&format!("Lv.{} {}", c.level, info.name), x + slot_w * 0.5, sy + 324.0, 19.0, widgets::accent());
                let map_name = world::load_map(&c.map).map(|m| m.name).unwrap_or("Unknown");
                gfx::text_centered(map_name, x + slot_w * 0.5, sy + 352.0, 16.0, widgets::ink_soft());
                let hrs = c.play_seconds / 3600;
                let mins = (c.play_seconds / 60) % 60;
                gfx::text_centered(&format!("Played {}h {:02}m", hrs, mins), x + slot_w * 0.5, sy + 376.0, 14.0, widgets::ink_soft());
                if ui.enabled && ui.clicked && gfx::mouse_in(x, sy, slot_w, sh) {
                    self.selected = i;
                }
            } else {
                widgets::title("Empty Slot", x + slot_w * 0.5, sy + 150.0, 24.0);
                if widgets::button(&ui, x + 50.0, sy + 200.0, slot_w - 100.0, 50.0, "+ Create", ButtonStyle::Primary, true) {
                    go_create = true;
                }
            }
        }
        if go_create {
            return Go::To(Screen::Create(CreateScreen::new(app)));
        }

        // Keyboard selection
        if ui.enabled && !chars.is_empty() {
            if is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::A) {
                self.selected = self.selected.saturating_sub(1);
            }
            if is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::D) {
                self.selected = (self.selected + 1).min(chars.len() - 1);
            }
        }

        // Bottom bar
        let by = sy + sh + 40.0;
        let has = !chars.is_empty();
        if widgets::button(&ui, gfx::VW * 0.5 - 130.0, by, 260.0, 56.0, "Enter World", ButtonStyle::Primary, has)
            || (ui.enabled && has && is_key_pressed(KeyCode::Enter))
        {
            let idx = self.selected.min(chars.len() - 1);
            return Go::To(Screen::Play(Box::new(PlayScreen::new(app, idx))));
        }
        if widgets::button(&ui, gfx::VW * 0.5 + 150.0, by + 8.0, 170.0, 42.0, "Delete", ButtonStyle::Danger, has) {
            self.confirm_delete = true;
        }
        if widgets::button(&ui, gfx::VW * 0.5 - 320.0, by + 8.0, 170.0, 42.0, "Controls", ButtonStyle::Secondary, true) {
            self.settings = Some(SettingsPanel::new());
        }
        if widgets::button(&ui, 24.0, gfx::VH - 64.0, 150.0, 40.0, "Log Out", ButtonStyle::Secondary, true) {
            app.account = None;
            return Go::To(Screen::Login(LoginScreen::new(app)));
        }
        if widgets::button(&ui, gfx::VW - 174.0, gfx::VH - 64.0, 150.0, 40.0, "Quit Game", ButtonStyle::Secondary, true) {
            app.quit = true;
        }

        if self.confirm_delete {
            let name = chars.get(self.selected).map(|c| c.name.clone()).unwrap_or_default();
            let ui = Ui::begin();
            gfx::rect(0.0, 0.0, gfx::VW, gfx::VH, rgba(10, 14, 30, 170));
            let (px, py, pw, ph) = (390.0, 240.0, 500.0, 220.0);
            widgets::panel(px, py, pw, ph);
            widgets::title(&format!("Delete {}?", name), px + pw * 0.5, py + 34.0, 28.0);
            gfx::text_centered("This cannot be undone.", px + pw * 0.5, py + 80.0, 18.0, widgets::danger());
            if widgets::button(&ui, px + 40.0, py + ph - 76.0, 190.0, 46.0, "Keep", ButtonStyle::Secondary, true) || is_key_pressed(KeyCode::Escape) {
                self.confirm_delete = false;
            }
            if widgets::button(&ui, px + pw - 230.0, py + ph - 76.0, 190.0, 46.0, "Delete forever", ButtonStyle::Danger, true) {
                if let Some(acc) = app.account.as_mut() {
                    if self.selected < acc.characters.len() {
                        acc.characters.remove(self.selected);
                    }
                }
                app.save_account();
                self.selected = 0;
                self.confirm_delete = false;
            }
        }

        if let Some(panel) = &mut self.settings {
            let mut settings = app.account.as_ref().map(|a| a.settings.clone()).unwrap_or_default();
            let (changed, close) = panel.frame(&mut settings);
            if changed {
                if let Some(acc) = app.account.as_mut() {
                    acc.settings = settings;
                }
                app.save_account();
            }
            if close {
                self.settings = None;
            }
        }
        if let Some(e) = &app.save_error {
            gfx::text_centered_outlined(e, gfx::VW * 0.5, gfx::VH - 30.0, 15.0, widgets::danger());
        }
        Go::Stay
    }
}
