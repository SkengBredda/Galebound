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
use galebound_core::sim::{Sim, TICK};
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
    Settings,
    Select,
    Logout,
    Quit,
}

#[derive(PartialEq)]
enum Menu {
    Closed,
    Pause,
    Settings,
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
        render::enemies(&self.sim, &cam);
        render::player(&self.sim, &cam, &self.look);
        render::tufts(&self.sim, &cam);
        render::projectiles(&self.sim, &cam);
        render::particles(&self.sim, &cam);
        render::decos(&self.sim, &cam, 2, time);
        render::lighting(&self.sim, &cam, time);
        hud::draw(&self.sim, self.show_controls && !self.windows.any_open(&self.sim), time);
        if self.menu == Menu::Closed && self.sim.dialog.is_none() {
            self.windows.frame(&mut self.sim, dt);
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
        let items: [(&str, MenuItem); 9] = [
            ("Resume", MenuItem::Resume),
            ("World Map", MenuItem::Map),
            ("Inventory", MenuItem::Inventory),
            ("Character", MenuItem::Character),
            ("Abilities", MenuItem::Abilities),
            ("Controls & Settings", MenuItem::Settings),
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
            if i == 6 {
                y += 18.0;
                gfx::rect(x + 34.0, y - 10.0, w - 68.0, 1.0, rgba(255, 255, 255, 40));
            }
            let h = 44.0;
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
            gfx::text(label, x + 40.0 + if sel { 6.0 } else { 0.0 }, y + 12.0, 18.0, col);
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
            Some(MenuItem::Settings) => self.menu = Menu::Settings,
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
}
