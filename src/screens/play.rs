//! The in-game screen: runs the fixed-timestep simulation, renders the world
//! and HUD, autosaves, and hosts the pause menu.

use crate::gfx::{self, rgb, rgba};
use crate::hud;
use crate::keys::InputCollector;
use crate::render::{self, Camera};
use crate::screens::{login::LoginScreen, select::SelectScreen};
use crate::settings_panel::SettingsPanel;
use crate::social_ui::SocialUi;
use crate::widgets::{self, Ui};
use crate::windows::{self, Windows};
use crate::{App, Go, Screen, REPLY_WAIT};
use galebound_core::appearance::Appearance;
use galebound_core::net::{self, ClientMsg, PlayerInfo, PlayerPos, ServerMsg};
use galebound_core::input::Action;
use galebound_core::save::DISPLAY_MODES;
use galebound_core::sim::{PState, Sim, TICK, TOMB_DELAY};
use macroquad::prelude::{get_frame_time, is_key_pressed, KeyCode};

const AUTOSAVE_SECS: f32 = 30.0;
/// Online saves are at most this often (loot pick-ups ask for a save each).
const SAVE_GAP: f32 = 3.0;
/// Position updates per second.
const POS_HZ: f32 = 15.0;
/// Chat lines fade after this many seconds (unless the chat box is open).
const CHAT_FADE: f32 = 12.0;
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
    /// Typing a chat message.
    chat_open: bool,
    chat_text: String,
    pos_t: f32,
    ping_t: f32,
    since_save: f32,
    last_info: Option<PlayerInfo>,
    social: SocialUi,
    /// Which chat tab is showing (`net::CH_*`).
    chat_tab: u32,
    /// Who last whispered us (for /r).
    last_whisper: String,
}

const CHAT_TABS: [(&str, u32); 5] = [("All", net::CH_ALL), ("Party", net::CH_PARTY), ("Guild", net::CH_GUILD), ("Whisper", net::CH_WHISPER), ("System", net::CH_SYSTEM)];

fn channel_color(ch: u32) -> macroquad::prelude::Color {
    match ch {
        net::CH_PARTY => rgb(120, 220, 255),
        net::CH_GUILD => rgb(140, 240, 150),
        net::CH_WHISPER => rgb(255, 150, 230),
        net::CH_SYSTEM => rgb(255, 220, 130),
        net::CH_LOG => rgb(200, 206, 230),
        _ => rgb(240, 244, 255),
    }
}

/// Does a chat line belong on this tab? "All" shows everything except your
/// own EXP / loot log; "System" shows server notices and that log.
fn on_tab(tab: u32, ch: u32) -> bool {
    match tab {
        net::CH_ALL => ch != net::CH_LOG,
        net::CH_SYSTEM => ch == net::CH_SYSTEM || ch == net::CH_LOG,
        t => t == ch,
    }
}

impl PlayScreen {
    /// Enter the world as `slot`, playing on the server as player `my_id`.
    pub fn new(app: &App, slot: usize, my_id: u32, day: u32) -> Self {
        let acc = app.account.as_ref().expect("logged in");
        let save = acc.characters[slot].clone();
        let look = save.look;
        let seed = (app.time * 1_000_003.0) as u64 ^ 0xA5A5;
        let mut sim = Sim::new(save, acc.settings.clone(), seed);
        sim.go_online(my_id);
        sim.today = day;
        if let Some(n) = &app.net {
            if !n.motd.is_empty() {
                sim.push_chat("", &n.motd);
            }
        }
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
            chat_open: false,
            chat_text: String::new(),
            pos_t: 0.0,
            ping_t: 0.0,
            since_save: 0.0,
            last_info: None,
            social: SocialUi::default(),
            chat_tab: net::CH_ALL,
            last_whisper: String::new(),
        }
    }

    /// Direct access to the simulation (debug tools / screenshots).
    #[allow(dead_code)]
    pub fn sim_mut(&mut self) -> &mut Sim {
        &mut self.sim
    }

    #[allow(dead_code)]
    pub fn open_graphics(&mut self) {
        self.menu = Menu::Graphics;
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
    pub fn social_mut(&mut self) -> &mut SocialUi {
        &mut self.social
    }

    #[allow(dead_code)]
    pub fn menu_debug(&mut self, name: &str, x: f32, y: f32) {
        self.social.menu_for(&self.sim, name, x, y);
    }
    #[allow(dead_code)]
    pub fn close_social(&mut self) {
        self.social.close_one(&mut self.sim);
    }

    #[allow(dead_code)]
    pub fn chat_debug(&mut self, open: bool, tab: u32, text: &str) {
        self.chat_open = open;
        self.chat_tab = tab;
        self.chat_text = text.to_string();
    }

    #[allow(dead_code)]
    pub fn windows_mut(&mut self) -> &mut Windows {
        &mut self.windows
    }

    /// Send the character's progress (and settings) to the server.
    pub fn save_now(&mut self, app: &mut App) {
        let snap = self.sim.snapshot();
        if let Some(n) = &app.net {
            n.send(&ClientMsg::Save { save: galebound_core::save::encode_character(&snap) });
        }
        if let Some(acc) = app.account.as_mut() {
            if self.slot < acc.characters.len() {
                acc.characters[self.slot] = snap;
            }
            acc.settings = self.sim.settings.clone();
        }
        app.save_settings();
        self.sim.wants_save = false;
        self.autosave = AUTOSAVE_SECS;
        self.since_save = 0.0;
        self.saved_flash = 1.5;
    }

    /// Messages from the server. Returns where to go if we lost it.
    fn network_in(&mut self, app: &mut App) -> Option<Go> {
        let Some(n) = app.net.as_mut() else {
            return Some(Go::To(Screen::Login(LoginScreen::with_error(app, "Not connected to a server."))));
        };
        for m in n.poll() {
            match m {
                // kicked: signed in elsewhere, or the server is shutting down
                ServerMsg::Error { text } if text.contains("another computer") || text.contains("shutting down") => {
                    app.net = None;
                    app.account = None;
                    return Some(Go::To(Screen::Login(LoginScreen::with_error(app, text))));
                }
                ServerMsg::Error { text } => self.sim.push_chat("", &text),
                m => self.sim.net_apply(m),
            }
        }
        if n.is_closed() {
            app.net = None;
            app.account = None;
            return Some(Go::To(Screen::Login(LoginScreen::with_error(app, "Lost connection to the server. Your progress up to the last save is safe."))));
        }
        None
    }

    /// Tell the server what we did and where we are.
    fn network_out(&mut self, app: &mut App, dt: f32) {
        let Some(n) = app.net.as_ref() else { return };
        for m in self.sim.outbox.drain(..) {
            n.send(&m);
        }
        let info = self.sim.my_info(&self.look.encode());
        if self.last_info.as_ref() != Some(&info) {
            n.send(&ClientMsg::Info(info.clone()));
            self.last_info = Some(info);
        }
        self.pos_t -= dt;
        if self.pos_t <= 0.0 {
            self.pos_t = 1.0 / POS_HZ;
            let p = &self.sim.player;
            let (anim, frame) = render::player_anim(p);
            let f = p.body.feet();
            let mut pos = PlayerPos {
                id: self.sim.my_id,
                x: f.x,
                y: f.y,
                facing: if p.facing < 0.0 { -1 } else { 1 },
                anim: crate::online::anim_code(anim),
                frame: frame as u32,
                hp: (p.hp / p.max_hp).clamp(0.0, 1.0),
                dead: p.state == PState::Dead,
                ..Default::default()
            };
            if let Some(a) = &self.sim.pet {
                pos.pet_on = true;
                pos.pet_x = a.pos.x;
                pos.pet_y = a.pos.y;
                pos.pet_facing = if a.facing < 0.0 { -1 } else { 1 };
                pos.pet_frame = if a.moving { ((a.anim_t * 10.0) as u32) % 2 } else { 0 };
            }
            n.send(&ClientMsg::Pos { map: self.sim.map.key.to_string(), pos });
        }
        self.ping_t -= dt;
        if self.ping_t <= 0.0 {
            self.ping_t = 3.0;
            n.send(&ClientMsg::Ping { t: app.time });
        }
    }

    /// Typing in the chat box: Enter sends, Esc cancels. Returns true when
    /// it used this frame's keys (so they don't also open menus).
    fn chat_input(&mut self, app: &App) -> bool {
        if !self.chat_open {
            // leave typed characters for the windows' text boxes
            return false;
        }
        while let Some(c) = macroquad::prelude::get_char_pressed() {
            if !c.is_control() && self.chat_text.chars().count() < net::CHAT_MAX {
                self.chat_text.push(c);
            }
        }
        if is_key_pressed(KeyCode::Backspace) {
            self.chat_text.pop();
        }
        if is_key_pressed(KeyCode::Escape) {
            self.chat_open = false;
            self.chat_text.clear();
            self.input.consumed();
        } else if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
            let t = self.chat_text.trim().to_string();
            if !t.is_empty() {
                if let Some(m) = self.chat_message(&t) {
                    if let Some(n) = &app.net {
                        n.send(&m);
                    }
                }
            }
            self.chat_open = false;
            self.chat_text.clear();
            self.input.consumed();
        }
        true
    }

    /// Turn what was typed into a message: "/w Name text" whispers, "/r"
    /// replies, "/p" and "/g" talk to your party / guild, "/a" (or plain
    /// text on the All tab) talks to everyone. Plain text follows the tab.
    fn chat_message(&mut self, t: &str) -> Option<ClientMsg> {
        use net::SocialCmd as S;
        let lower = t.to_lowercase();
        let rest = |n: usize| t[n..].trim().to_string();
        if lower.starts_with("/w ") || lower.starts_with("/whisper ") {
            let body = rest(t.find(' ').unwrap_or(0));
            // names may contain spaces: match the longest name we know of
            let mut known: Vec<String> = self.sim.others.iter().map(|o| o.name.clone()).collect();
            known.extend(self.sim.social.friends.iter().map(|f| f.name.clone()));
            if let Some(p) = &self.sim.party {
                known.extend(p.members.iter().map(|m| m.name.clone()));
            }
            if let Some(g) = &self.sim.guild {
                known.extend(g.members.iter().map(|m| m.name.clone()));
            }
            known.sort_by_key(|n| std::cmp::Reverse(n.len()));
            let bl = body.to_lowercase();
            let (to, text) = match known.iter().find(|n| bl.starts_with(&format!("{} ", n.to_lowercase()))) {
                Some(n) => (n.clone(), body[n.len()..].trim().to_string()),
                None => match body.split_once(' ') {
                    Some((a, b)) => (a.to_string(), b.trim().to_string()),
                    None => (body.clone(), String::new()),
                },
            };
            self.last_whisper = to.clone();
            return (!text.is_empty()).then_some(ClientMsg::Social(S::Whisper { to, text }));
        }
        if lower.starts_with("/r ") {
            if self.last_whisper.is_empty() {
                self.sim.sys_line("Nobody to reply to yet.".into());
                return None;
            }
            return Some(ClientMsg::Social(S::Whisper { to: self.last_whisper.clone(), text: rest(3) }));
        }
        if lower.starts_with("/p ") {
            return Some(ClientMsg::Social(S::PartyChat { text: rest(3) }));
        }
        if lower.starts_with("/g ") {
            return Some(ClientMsg::Social(S::GuildChat { text: rest(3) }));
        }
        if lower.starts_with("/a ") {
            return Some(ClientMsg::Chat { text: rest(3) });
        }
        if t.starts_with('/') {
            self.sim.sys_line("Chat commands: /w Name message, /r message, /p party, /g guild, /a everyone".into());
            return None;
        }
        Some(match self.chat_tab {
            net::CH_PARTY => ClientMsg::Social(S::PartyChat { text: t.to_string() }),
            net::CH_GUILD => ClientMsg::Social(S::GuildChat { text: t.to_string() }),
            net::CH_WHISPER if !self.last_whisper.is_empty() => ClientMsg::Social(S::Whisper { to: self.last_whisper.clone(), text: t.to_string() }),
            _ => ClientMsg::Chat { text: t.to_string() },
        })
    }

    /// Recent chat (bottom-left) with channel tabs, and the text box while typing.
    fn draw_chat(&mut self) {
        let (x, w) = (14.0, 520.0);
        let bottom = if self.chat_open { gfx::VH - 120.0 } else { gfx::VH - 300.0 };
        // remember who whispered us last, for /r
        if let Some(l) = self.sim.chat.iter().rev().find(|l| l.channel == net::CH_WHISPER && !l.from.eq_ignore_ascii_case(&self.sim.save.name)) {
            if l.age < 0.05 {
                self.last_whisper = l.from.clone();
            }
        }
        let tab = self.chat_tab;
        let lines: Vec<&galebound_core::sim::ChatLine> =
            self.sim.chat.iter().rev().filter(|c| on_tab(tab, c.channel) && (self.chat_open || c.age < CHAT_FADE)).take(if self.chat_open { 12 } else { 6 }).collect();
        if self.chat_open {
            gfx::rrect(x, bottom - lines.len().max(1) as f32 * 22.0 - 10.0, w, lines.len().max(1) as f32 * 22.0 + 10.0, 6.0, rgba(14, 16, 30, 170));
        }
        // tabs (clickable while the chat box is open; always shown small)
        let ty = bottom - lines.len().max(1) as f32 * 22.0 - (if self.chat_open { 36.0 } else { 30.0 });
        if self.chat_open || !lines.is_empty() {
            let mut tx = x;
            for (label, ch) in CHAT_TABS {
                let tw = gfx::text_w(label, 13.0) + 16.0;
                let on = self.chat_tab == ch;
                let unread = !on && self.sim.chat.iter().any(|c| on_tab(ch, c.channel) && c.age < 4.0 && ch != net::CH_ALL);
                gfx::rrect(tx, ty, tw, 22.0, 5.0, if on { rgba(70, 120, 200, 220) } else { rgba(14, 16, 30, if self.chat_open { 190 } else { 120 }) });
                gfx::text(label, tx + 8.0, ty + 4.0, 13.0, if on { rgb(255, 255, 255) } else if unread { channel_color(ch) } else { rgb(170, 178, 200) });
                if self.chat_open && macroquad::prelude::is_mouse_button_pressed(macroquad::prelude::MouseButton::Left) && gfx::mouse_in(tx, ty, tw, 22.0) {
                    self.chat_tab = ch;
                }
                tx += tw + 4.0;
            }
        }
        let me = self.sim.save.name.clone();
        for (i, l) in lines.iter().enumerate() {
            let a = if self.chat_open { 1.0 } else { ((CHAT_FADE - l.age) / 2.0).clamp(0.0, 1.0) };
            let y = bottom - (i + 1) as f32 * 22.0;
            let col = channel_color(l.channel);
            let line = match l.channel {
                _ if l.from.is_empty() => l.text.clone(),
                net::CH_WHISPER if l.from.eq_ignore_ascii_case(&me) => format!("To {}: {}", l.to, l.text),
                net::CH_WHISPER => format!("From {}: {}", l.from, l.text),
                net::CH_PARTY => format!("[Party] {}: {}", l.from, l.text),
                net::CH_GUILD => format!("[Guild] {}: {}", l.from, l.text),
                _ => format!("{}: {}", l.from, l.text),
            };
            let line = gfx::wrap(&line, w - 16.0, 14.0).into_iter().next().unwrap_or_default();
            gfx::text_outlined(&line, x + 8.0, y, 14.0, gfx::alpha(col, a));
        }
        if self.chat_open {
            let y = gfx::VH - 112.0;
            gfx::rrect(x, y, w, 30.0, 6.0, rgba(14, 16, 30, 220));
            gfx::rect_lines(x + 1.0, y + 1.0, w - 2.0, 28.0, 1.0, rgba(120, 220, 255, 160));
            let caret = if (self.sim.time * 2.0) as i32 % 2 == 0 { "_" } else { "" };
            gfx::text(&format!("> {}{}", self.chat_text, caret), x + 10.0, y + 7.0, 14.0, rgb(255, 255, 255));
            let hint = match self.chat_tab {
                net::CH_PARTY => "To party.  /w Name, /g, /a",
                net::CH_GUILD => "To guild.  /w Name, /p, /a",
                net::CH_WHISPER => "Reply.  /w Name to pick someone",
                _ => "Enter: send  Esc: cancel  /w /p /g",
            };
            gfx::text_outlined(hint, x + w - gfx::text_w(hint, 12.0) - 8.0, y + 36.0, 12.0, rgb(190, 200, 220));
        }
    }

    pub fn frame(&mut self, app: &mut App) -> Go {
        let dt = get_frame_time().min(0.1);
        let layout = self.sim.settings.layout;
        if let Some(go) = self.network_in(app) {
            return go;
        }
        let typed = self.chat_input(app);
        self.input.poll(layout);
        if let Some(t) = self.social.start_chat.take() {
            self.chat_open = true;
            self.chat_text = t;
            self.input.consumed();
            while macroquad::prelude::get_char_pressed().is_some() {}
        }
        if self.social.typing() && !self.chat_open {
            // typing in a text box: keys spell words
            self.input.consumed();
            self.input = InputCollector::default();
        } else if typed && !self.chat_open {
            // the Enter / Esc that closed the chat box does nothing else
            self.input.consumed();
        } else if self.chat_open {
            // typing: the keys spell words, they don't move you
            self.input.consumed();
            self.input = InputCollector::default();
        } else if self.menu == Menu::Closed && self.sim.dialog.is_none() && self.input.pressed(Action::Chat) {
            self.chat_open = true;
            self.chat_text.clear();
            self.input.consumed();
            while macroquad::prelude::get_char_pressed().is_some() {}
        }

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
            if self.input.pressed(Action::Social) {
                self.social.toggle_social(&mut self.sim);
            }
            if self.input.pressed(Action::Guild) {
                self.social.toggle_guild(&mut self.sim);
            }
            if self.input.pressed(Action::Achievements) {
                self.social.toggle_achievements();
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
            } else if self.social.close_one(&mut self.sim) || self.windows.close_one(&mut self.sim) {
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
            self.since_save += dt;
            if (self.sim.wants_save && self.since_save > SAVE_GAP) || self.autosave <= 0.0 {
                self.save_now(app);
            }
            self.network_out(app, dt);
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
        render::others(&self.sim, &cam, time);
        render::enemies(&self.sim, &cam);
        render::player(&self.sim, &cam, &self.look);
        render::tufts(&self.sim, &cam);
        render::swings(&self.sim, &cam);
        render::projectiles(&self.sim, &cam);
        render::particles(&self.sim, &cam);
        render::decos(&self.sim, &cam, 2, time);
        render::lighting(&self.sim, &cam, time);
        render::drop_labels(&self.sim, &cam);
        render::pet_label(&self.sim, &cam, time);
        render::other_labels(&self.sim, &cam);
        hud::draw(&self.sim, self.show_controls && !self.chat_open && !self.windows.any_open(&self.sim) && !self.social.any_open(&self.sim), time);
        self.draw_chat();
        // one input snapshot (typed characters, clicks) for every window this frame
        let ui = Ui { enabled: self.menu == Menu::Closed, ..Ui::begin() };
        {
            let act = crate::minimap::frame(&self.sim, &ui);
            if ui.enabled && crate::minimap::apply(&mut self.sim.settings, act) {
                self.sim.wants_save = true;
            }
        }
        if self.menu == Menu::Closed && self.sim.dialog.is_none() {
            self.windows.frame(&mut self.sim, dt, &ui);
            self.social.right_click(&self.sim, self.cam.x, self.cam.y);
            self.social.frame(&mut self.sim, &ui, dt);
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
                if let Some(n) = app.net.as_mut() {
                    let _ = n.request(&ClientMsg::Leave, REPLY_WAIT, |m| matches!(m, ServerMsg::Left));
                }
                let mut s = SelectScreen::new(app);
                s.select_slot(self.slot);
                return Go::To(Screen::Select(s));
            }
            Some(MenuItem::Logout) => {
                self.save_now(app);
                if let Some(n) = app.net.as_mut() {
                    let _ = n.request(&ClientMsg::Leave, REPLY_WAIT, |m| matches!(m, ServerMsg::Left));
                }
                app.net = None;
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
        let (w, h) = (560.0, 500.0);
        let (x, y) = ((gfx::VW - w) * 0.5, (gfx::VH - h) * 0.5);
        widgets::panel(x, y, w, h);
        widgets::title("Graphics", x + w * 0.5, y + 22.0, 28.0);
        // render resolution
        gfx::text("Resolution", x + 40.0, y + 74.0, 18.0, widgets::ink());
        let cur_res = gfx::resolution();
        let bw = (w - 80.0 - 2.0 * 12.0) / gfx::RESOLUTIONS.len() as f32;
        let mut res_hover = false;
        for (i, &(rw, rh)) in gfx::RESOLUTIONS.iter().enumerate() {
            let bx = x + 40.0 + i as f32 * (bw + 12.0);
            let on = cur_res == (rw, rh);
            let style = if on { widgets::ButtonStyle::Primary } else { widgets::ButtonStyle::Secondary };
            if widgets::button(&ui, bx, y + 102.0, bw, 44.0, &format!("{} x {}", rw, rh), style, true) && !on {
                crate::display::set_resolution(rw, rh);
            }
            res_hover |= gfx::mouse_in(bx, y + 102.0, bw, 44.0);
        }
        let y = y + 100.0;
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
        let y = y - 100.0;
        let tip = if res_hover {
            "How finely the game is drawn. Higher looks smoother and sharper on big screens; the view and pixel art stay the same."
        } else {
            help[hovered.unwrap_or(cur)]
        };
        gfx::text_wrapped(tip, x + 40.0, y + h - 82.0, w - 200.0, 14.0, widgets::ink_soft());
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
        if respawn {
            self.sim.respawn();
        } else if scroll {
            if let Err(e) = self.sim.revive_with_scroll() {
                self.sim.toast(&e, "");
            }
        }
    }
}
