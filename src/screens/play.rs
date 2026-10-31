//! The in-game screen: runs the fixed-timestep simulation, renders the world
//! and HUD, autosaves, and hosts the pause menu.

use crate::gfx::{self, rgb, rgba};
use crate::hud;
use crate::keys::InputCollector;
use crate::render::{self, Camera};
use crate::screens::{login::LoginScreen, select::SelectScreen};
use crate::settings_panel::SettingsPanel;
use crate::widgets::{self, Ui};
use crate::windows::{self, Windows};
use crate::{App, Go, Screen};
use galebound_core::appearance::Appearance;
use galebound_core::input::Action;
use galebound_core::save::DISPLAY_MODES;
use galebound_core::sim::{PState, Sim, TICK, TOMB_DELAY};
use macroquad::prelude::{get_frame_time, is_key_pressed, KeyCode};

const AUTOSAVE_SECS: f32 = 30.0;
const MAX_TICKS_PER_FRAME: u32 = 12;

#[derive(Clone, Copy, PartialEq)]
enum MenuItem {
    Resume,
    Map,
    Inventory,
    Character,
    Abilities,
    Pets,
    Settings,
    Graphics,
    Select,
    Logout,
    Quit,
}

#[derive(PartialEq)]
enum Menu {
    Closed,
    Pause,
    Settings,
    Graphics,
}

pub struct PlayScreen {
    sim: Sim,
    slot: usize,
    look: Appearance,
    cam: Camera,
    input: InputCollector,
    acc: f32,
    autosave: f32,
    menu: Menu,
    settings_panel: Option<SettingsPanel>,
    show_controls: bool,
    saved_flash: f32,
    /// Ignore the Esc press that opened the pause menu.
    menu_just_opened: bool,
    windows: Windows,
    menu_anim: f32,
    menu_sel: usize,
}

impl PlayScreen {
    pub fn new(app: &App, slot: usize) -> Self {
        let acc = app.account.as_ref().expect("logged in");
        let save = acc.characters[slot].clone();
        let look = save.look;
        let seed = (app.time * 1_000_003.0) as u64 ^ 0xA5A5;
        let sim = Sim::new(save, acc.settings.clone(), seed);
        crate::display::apply(acc.settings.display);
        let mut cam = Camera { x: 0.0, y: 0.0 };
        cam.snap(&sim);
        PlayScreen {
            sim,
            slot,
            look,
            cam,
            input: InputCollector::default(),
            acc: 0.0,
            autosave: AUTOSAVE_SECS,
            menu: Menu::Closed,
            settings_panel: None,
            show_controls: true,
            saved_flash: 0.0,
            menu_just_opened: false,
            windows: Windows::default(),
            menu_anim: 0.0,
            menu_sel: 0,
        }
    }

    /// Direct access to the simulation (debug tools / screenshots).
    #[allow(dead_code)]
    pub fn sim_mut(&mut self) -> &mut Sim {
        &mut self.sim
    }

    #[allow(dead_code)]
    pub fn open_settings(&mut self) {
        self.menu = Menu::Settings;
    }

    #[allow(dead_code)]
    pub fn open_menu(&mut self) {
        self.menu = Menu::Pause;
        self.menu_anim = 1.0;
        self.menu_sel = 1;
    }

    #[allow(dead_code)]
    pub fn windows_mut(&mut self) -> &mut Windows {
        &mut self.windows
    }

    /// Write the character back into the account and to disk.
    pub fn save_now(&mut self, app: &mut App) {
        let snap = self.sim.snapshot();
        if let Some(acc) = app.account.as_mut() {
            if self.slot < acc.characters.len() {
                acc.characters[self.slot] = snap;
            }
            acc.settings = self.sim.settings.clone();
        }
        app.save_account();
        self.sim.wants_save = false;
        self.autosave = AUTOSAVE_SECS;
        self.saved_flash = 1.5;
    }

    pub fn frame(&mut self, app: &mut App) -> Go {
        let dt = get_frame_time().min(0.1);
        let layout = self.sim.settings.layout;
        self.input.poll(layout);

        if is_key_pressed(KeyCode::F1) {
            self.show_controls = !self.show_controls;
        }

        // Window toggles
        if self.menu == Menu::Closed && self.sim.dialog.is_none() {
            let (i, c, a) = (self.input.pressed(Action::Inventory), self.input.pressed(Action::Character), self.input.pressed(Action::Abilities));
            self.windows.toggle_keys(i, c, a);
            if self.input.pressed(Action::WorldMap) {
                self.windows.toggle_map();
            }
            if self.input.pressed(Action::Pets) {
                self.windows.toggle_pets(&self.sim);
            }
            if self.input.pressed(Action::Minimap) {
                crate::minimap::cycle(&mut self.sim.settings);
                self.sim.wants_save = true;
            }
        }
        // Pause menu toggling (only when no dialog is open; Esc closes dialogs and windows first).
        if self.menu == Menu::Closed && self.input.pressed(Action::Menu) && self.sim.dialog.is_none() {
            if self.sim.quest_log_open {
                self.sim.quest_log_open = false;
            } else if self.windows.close_one(&mut self.sim) {
            } else {
                self.menu = Menu::Pause;
                self.menu_just_opened = true;
                self.menu_anim = 0.0;
                self.menu_sel = 0;
            }
            self.input.consumed();
        }

        // ---------------------------------------------- simulation
        // The world never pauses (it's an MMO): with the menu open your
        // character just stands still while everything else keeps going.
        {
            let menu_open = self.menu != Menu::Closed;
            self.acc += dt;
            let mut n = 0;
            while self.acc >= TICK && n < MAX_TICKS_PER_FRAME {
                let input = if menu_open { galebound_core::input::InputState::default() } else { self.input.current() };
                self.sim.tick(&input);
                self.input.consumed();
                self.acc -= TICK;
                n += 1;
            }
            if n == MAX_TICKS_PER_FRAME {
                self.acc = 0.0; // we fell far behind (window dragged etc.) - don't spiral
            }
            if self.sim.map_changed {
                self.sim.map_changed = false;
                self.cam.snap(&self.sim);
            }
            if self.sim.look_changed {
                self.sim.look_changed = false;
                self.look = windows::gear_look(&self.sim.save.look, &self.sim);
            }
            // Walking away from the shopkeeper closes the shop.
            if self.sim.shop_open.is_some() && self.sim.nearest_npc().is_none() {
                self.sim.shop_open = None;
            }
            self.cam.follow(&self.sim, dt);
            self.autosave -= dt;
            if self.sim.wants_save || self.autosave <= 0.0 {
                self.save_now(app);
            }
            if menu_open {
                self.input.consumed();
            }
        }
        self.saved_flash = (self.saved_flash - dt).max(0.0);

        // ---------------------------------------------- render
        gfx::begin_frame(rgb(120, 180, 240));
        let time = self.sim.time;
        let shake = if self.sim.settings.screen_shake { self.sim.shake } else { 0.0 };
        let sx = (time * 91.0).sin() * shake;
        let sy = (time * 77.0).cos() * shake;
        let cam = Camera { x: self.cam.x + sx, y: self.cam.y + sy };
        render::background(self.sim.map.theme, &cam, time);
        render::decos(&self.sim, &cam, 0, time);
        render::terrain(&self.sim, &cam);
        render::climbs(&self.sim, &cam);
        render::decos(&self.sim, &cam, 1, time);
        render::signs_and_portals(&self.sim, &cam, time);
        render::npcs(&self.sim, &cam, time);
        render::drops(&self.sim, &cam, time);
        render::pet(&self.sim, &cam, time);
        render::enemies(&self.sim, &cam);
        render::player(&self.sim, &cam, &self.look);
        render::tufts(&self.sim, &cam);
        render::projectiles(&self.sim, &cam);
        render::particles(&self.sim, &cam);
        render::decos(&self.sim, &cam, 2, time);
        render::lighting(&self.sim, &cam, time);
        render::drop_labels(&self.sim, &cam);
        render::pet_label(&self.sim, &cam, time);
        hud::draw(&self.sim, self.show_controls && !self.windows.any_open(&self.sim), time);
        {
            let ui = Ui { enabled: self.menu == Menu::Closed, ..Ui::begin() };
            let act = crate::minimap::frame(&self.sim, &ui);
            if ui.enabled && crate::minimap::apply(&mut self.sim.settings, act) {
                self.sim.wants_save = true;
            }
        }
        if self.menu == Menu::Closed && self.sim.dialog.is_none() {
            self.windows.frame(&mut self.sim, dt);
        }
        if self.sim.player.state == PState::Dead && self.sim.player.state_t > TOMB_DELAY && self.menu == Menu::Closed {
            self.death_panel();
        }
        if self.saved_flash > 0.0 {
            gfx::text_outlined("Saved", gfx::VW - 80.0, gfx::VH - 50.0, 15.0, rgba(180, 255, 200, (255.0 * self.saved_flash.min(1.0)) as u8));
        }
        if let Some(e) = &app.save_error {
            gfx::text_centered_outlined(&format!("Save failed: {}", e), gfx::VW * 0.5, 120.0, 16.0, widgets::danger());
        }

        // ---------------------------------------------- menus
        match self.menu {
            Menu::Closed => Go::Stay,
            Menu::Pause => self.pause_menu(app),
            Menu::Graphics => {
                if self.graphics_panel() {
                    self.save_now(app);
                }
                Go::Stay
            }
            Menu::Settings => {
                let panel = self.settings_panel.get_or_insert_with(SettingsPanel::new);
                let (changed, close) = panel.frame(&mut self.sim.settings);
                if changed {
                    self.save_now(app);
                }
                if close {
                    self.settings_panel = None;
                    self.menu = Menu::Pause;
                }
                Go::Stay
            }
        }
    }

    /// Sleek side menu. The game keeps running behind it.
    fn pause_menu(&mut self, app: &mut App) -> Go {
        let ui = Ui::begin();
        // Slide in from the right over ~0.15 s.
        self.menu_anim = (self.menu_anim + get_frame_time() * 7.0).min(1.0);
        let ease = 1.0 - (1.0 - self.menu_anim).powi(3);
        let w = 340.0;
        let x = gfx::VW - w * ease;
        gfx::rect(0.0, 0.0, gfx::VW, gfx::VH, rgba(8, 10, 22, (90.0 * ease) as u8));
        gfx::vgradient(x, 0.0, w, gfx::VH, rgba(18, 22, 40, 235), rgba(30, 24, 52, 235), 8);
        gfx::rect(x, 0.0, 2.0, gfx::VH, rgba(120, 220, 255, 120));
        gfx::text_outlined("GALEBOUND", x + 34.0, 40.0, 26.0, rgb(255, 236, 170));
        gfx::text(&format!("{}  -  Lv.{} {}", self.sim.save.name, self.sim.save.level, self.sim.player.info.name), x + 36.0, 80.0, 14.0, rgb(190, 200, 225));
        gfx::rect(x + 34.0, 106.0, w - 68.0, 1.0, rgba(255, 255, 255, 50));
        let items: [(&str, MenuItem); 11] = [
            ("Resume", MenuItem::Resume),
            ("World Map", MenuItem::Map),
            ("Inventory", MenuItem::Inventory),
            ("Character", MenuItem::Character),
            ("Abilities", MenuItem::Abilities),
            ("Pets", MenuItem::Pets),
            ("Controls & Settings", MenuItem::Settings),
            ("Graphics", MenuItem::Graphics),
            ("Character Select", MenuItem::Select),
            ("Log Out", MenuItem::Logout),
            ("Quit Game", MenuItem::Quit),
        ];
        // keyboard navigation
        if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S) {
            self.menu_sel = (self.menu_sel + 1) % items.len();
        }
        if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W) {
            self.menu_sel = (self.menu_sel + items.len() - 1) % items.len();
        }
        let mut chosen: Option<MenuItem> = None;
        if is_key_pressed(KeyCode::Enter) {
            chosen = Some(items[self.menu_sel].1);
        }
        let mut y = 130.0;
        for (i, (label, item)) in items.iter().enumerate() {
            if i == 8 {
                y += 18.0;
                gfx::rect(x + 34.0, y - 10.0, w - 68.0, 1.0, rgba(255, 255, 255, 40));
            }
            let h = 38.0;
            let hover = gfx::mouse_in(x, y, w, h);
            if hover {
                self.menu_sel = i;
            }
            let sel = self.menu_sel == i;
            if sel {
                gfx::rect(x + 2.0, y, w - 2.0, h, rgba(120, 220, 255, 34));
                gfx::rect(x + 2.0, y, 4.0, h, rgb(120, 220, 255));
            }
            let col = match item {
                MenuItem::Quit => rgb(255, 150, 140),
                _ if sel => rgb(255, 255, 255),
                _ => rgb(196, 204, 226),
            };
            gfx::text(label, x + 40.0 + if sel { 6.0 } else { 0.0 }, y + 9.0, 18.0, col);
            if hover && ui.clicked {
                chosen = Some(*item);
            }
            y += h + 4.0;
        }
        gfx::text_wrapped("The world keeps moving while this menu is open.", x + 36.0, gfx::VH - 70.0, w - 72.0, 12.0, rgb(150, 160, 190));
        let esc = is_key_pressed(KeyCode::Escape) && !self.menu_just_opened;
        self.menu_just_opened = false;
        if esc {
            chosen = Some(MenuItem::Resume);
        }
        match chosen {
            Some(MenuItem::Resume) => self.menu = Menu::Closed,
            Some(MenuItem::Map) => {
                self.menu = Menu::Closed;
                self.windows.world_map = true;
            }
            Some(MenuItem::Inventory) => {
                self.menu = Menu::Closed;
                self.windows.inventory = true;
            }
            Some(MenuItem::Character) => {
                self.menu = Menu::Closed;
                self.windows.character = true;
            }
            Some(MenuItem::Abilities) => {
                self.menu = Menu::Closed;
                self.windows.abilities = true;
            }
            Some(MenuItem::Pets) => {
                self.menu = Menu::Closed;
                self.windows.toggle_pets(&self.sim);
                self.windows.pets = true;
            }
            Some(MenuItem::Settings) => self.menu = Menu::Settings,
            Some(MenuItem::Graphics) => self.menu = Menu::Graphics,
            Some(MenuItem::Select) => {
                self.save_now(app);
                let mut s = SelectScreen::new(app);
                s.select_slot(self.slot);
                return Go::To(Screen::Select(s));
            }
            Some(MenuItem::Logout) => {
                self.save_now(app);
                app.account = None;
                return Go::To(Screen::Login(LoginScreen::new(app)));
            }
            Some(MenuItem::Quit) => {
                self.save_now(app);
                app.quit = true;
            }
            None => {}
        }
        if self.menu == Menu::Closed {
            self.menu_anim = 0.0;
        }
        Go::Stay
    }

    /// Graphics options (from the pause menu). Returns true if a setting changed.
    fn graphics_panel(&mut self) -> bool {
        let ui = Ui::begin();
        let mut changed = false;
        gfx::rect(0.0, 0.0, gfx::VW, gfx::VH, rgba(10, 14, 30, 160));
        let (w, h) = (520.0, 360.0);
        let (x, y) = ((gfx::VW - w) * 0.5, (gfx::VH - h) * 0.5);
        widgets::panel(x, y, w, h);
        widgets::title("Graphics", x + w * 0.5, y + 22.0, 28.0);
        gfx::text("Display mode", x + 40.0, y + 74.0, 18.0, widgets::ink());
        let help = [
            "A normal window you can move and resize.",
            "A borderless window that covers the whole screen. Alt-Tab friendly.",
            "Takes over the whole screen.",
        ];
        let mut hovered = None;
        for (i, &m) in DISPLAY_MODES.iter().enumerate() {
            let by = y + 104.0 + i as f32 * 56.0;
            let cur = self.sim.settings.display == m;
            let style = if cur { widgets::ButtonStyle::Primary } else { widgets::ButtonStyle::Secondary };
            if widgets::button(&ui, x + 40.0, by, w - 80.0, 44.0, m.name(), style, true) && !cur {
                self.sim.settings.display = m;
                crate::display::apply(m);
                changed = true;
            }
            if gfx::mouse_in(x + 40.0, by, w - 80.0, 44.0) {
                hovered = Some(i);
            }
        }
        let cur = DISPLAY_MODES.iter().position(|m| *m == self.sim.settings.display).unwrap_or(0);
        gfx::text_wrapped(help[hovered.unwrap_or(cur)], x + 40.0, y + h - 82.0, w - 200.0, 14.0, widgets::ink_soft());
        let back = widgets::button(&ui, x + w - 150.0, y + h - 70.0, 110.0, 40.0, "Back", widgets::ButtonStyle::Secondary, true);
        if back || is_key_pressed(KeyCode::Escape) {
            self.menu = Menu::Pause;
            self.menu_anim = 1.0;
        }
        changed
    }

    /// Shown while you lie as a tombstone: Respawn, or read a Revive Scroll.
    fn death_panel(&mut self) {
        let ui = Ui::begin();
        let (w, h) = (500.0, 210.0);
        let (x, y) = ((gfx::VW - w) * 0.5, 170.0);
        let a = ((self.sim.player.state_t - TOMB_DELAY) / 0.4).clamp(0.0, 1.0);
        gfx::rrect(x, y, w, h, 12.0, rgba(24, 14, 26, (225.0 * a) as u8));
        gfx::rect_lines(x + 3.0, y + 3.0, w - 6.0, h - 6.0, 1.0, rgba(255, 180, 180, (70.0 * a) as u8));
        gfx::text_centered_outlined("You have been defeated", x + w * 0.5, y + 20.0, 32.0, gfx::alpha(rgb(255, 214, 214), a));
        let lost = self.sim.death_exp_lost();
        let sub = if lost > 0 {
            format!("You lost {} EXP. A Revive Scroll will restore it.", lost)
        } else if self.sim.save.level > galebound_core::sim::DEATH_PENALTY_LEVEL {
            "No EXP lost this time.".to_string()
        } else {
            format!("No EXP is lost until level {}.", galebound_core::sim::DEATH_PENALTY_LEVEL + 1)
        };
        gfx::text_centered(&sub, x + w * 0.5, y + 66.0, 15.0, gfx::alpha(rgb(230, 220, 236), a));
        let scrolls = self.sim.save.inventory.count("revive_scroll");
        let (bw, by) = (210.0, y + 104.0);
        let respawn = widgets::button(&ui, x + 30.0, by, bw, 46.0, "Respawn", widgets::ButtonStyle::Primary, true);
        let label = format!("Revive Scroll ({})", scrolls);
        let scroll = widgets::button(&ui, x + w - 30.0 - bw, by, bw, 46.0, &label, widgets::ButtonStyle::Secondary, scrolls > 0);
        gfx::text_centered("Respawn: back to the last checkpoint.", x + w * 0.5, y + h - 50.0, 12.0, rgb(190, 180, 200));
        gfx::text_centered("Other players can also revive you.", x + w * 0.5, y + h - 30.0, 12.0, rgb(190, 180, 200));
        if respawn || is_key_pressed(KeyCode::Enter) {
            self.sim.respawn();
        } else if scroll {
            if let Err(e) = self.sim.revive_with_scroll() {
                self.sim.toast(&e, "");
            }
        }
    }
}
