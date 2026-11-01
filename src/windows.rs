//! In-game windows: hotbar, inventory & equipment, character stats,
//! abilities and shops. Mouse driven: click to use / equip / buy, drag
//! abilities and consumables onto the hotbar, right-click to sell or clear.

use crate::gfx::{self, alpha, rgb, rgba};
use crate::icons;
use crate::sprites;
use crate::widgets::{self, ButtonStyle, Ui};
use galebound_core::classes::{ClassId, MAIN_STATS};
use galebound_core::input::{Action, HOTBAR_ACTIONS};
use galebound_core::items::{self, EquipSlot, ItemDef, ItemKind, EQUIP_SLOTS, INVENTORY_SLOTS};
use galebound_core::skills::{self, HotbarSlot, SkillDef, SkillEffect, HOTBAR_SLOTS};
use galebound_core::sim::{Sim, WAYSTONE_COST};
use galebound_core::world::{self, EnemyKind};
use macroquad::prelude::{is_key_down, is_mouse_button_down, is_mouse_button_pressed, Color, KeyCode, MouseButton};

const WHITE: Color = Color::new(1.0, 1.0, 1.0, 1.0);
const SLOT: f32 = 46.0;

pub const BAR_Y: f32 = 642.0;
const BAR_STEP: f32 = 52.0;
fn bar_x0() -> f32 {
    (gfx::VW - BAR_STEP * HOTBAR_SLOTS as f32) * 0.5
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Target {
    Inv(usize),
    Equip(EquipSlot),
    Skill(&'static str),
    Bar(usize),
    Shop(&'static str),
    /// Anywhere inside a window (blocks "drop to clear").
    Window,
}

#[derive(Default)]
pub struct Windows {
    pub inventory: bool,
    pub character: bool,
    pub abilities: bool,
    prev_down: bool,
    press: Option<(Target, (f32, f32))>,
    dragging: bool,
    msg: Option<(String, f32, bool)>,
    hits: Vec<((f32, f32, f32, f32), Target)>,
    prev_hits: Vec<((f32, f32, f32, f32), Target)>,
    shop_was_open: bool,
    shop_page: usize,
    pub world_map: bool,
    pub pets: bool,
    pet_sel: usize,
    map_sel: Option<&'static str>,
    map_cache: Option<WorldCache>,
}

fn item_tex(def: &ItemDef) -> macroquad::prelude::Texture2D {
    sprites::get(&format!("icon_i_{}", def.id), || icons::item(def))
}
fn skill_tex(s: &SkillDef) -> macroquad::prelude::Texture2D {
    sprites::get(&format!("icon_s_{}", s.id), || icons::skill(s))
}

/// 10000 -> "10,000".
fn thousands(n: u32) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// Shorten text with "..." so it fits in `max_w`.
fn clip(s: &str, max_w: f32, size: f32) -> String {
    if gfx::text_w(s, size) <= max_w {
        return s.to_string();
    }
    let mut out: String = s.to_string();
    while !out.is_empty() && gfx::text_w(&format!("{}...", out), size) > max_w {
        out.pop();
    }
    format!("{}...", out.trim_end())
}

fn shift() -> bool {
    is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift)
}

fn slot_bg(x: f32, y: f32, s: f32, hover: bool, accent: Option<Color>) {
    gfx::rrect(x, y, s, s, 6.0, rgb(92, 64, 42));
    gfx::rrect(x + 2.0, y + 2.0, s - 4.0, s - 4.0, 5.0, if hover { rgb(255, 246, 214) } else { rgb(232, 218, 188) });
    if let Some(a) = accent {
        gfx::rect(x + 4.0, y + s - 6.0, s - 8.0, 2.0, a);
    }
}

fn px_color(p: crate::pixel::Px) -> Color {
    rgba(p[0], p[1], p[2], p[3])
}

impl Windows {
    pub fn any_open(&self, sim: &Sim) -> bool {
        self.inventory || self.character || self.abilities || self.world_map || self.pets || sim.shop_open.is_some() || sim.teleport_open
    }

    /// Esc: close the top-most window. Returns true if something closed.
    pub fn close_one(&mut self, sim: &mut Sim) -> bool {
        if sim.teleport_open {
            sim.teleport_open = false;
            return true;
        }
        if self.world_map {
            self.world_map = false;
            return true;
        }
        if self.pets {
            self.pets = false;
            return true;
        }
        if sim.shop_open.is_some() {
            sim.shop_open = None;
            self.inventory = false;
            return true;
        }
        if self.abilities {
            self.abilities = false;
            return true;
        }
        if self.character {
            self.character = false;
            return true;
        }
        if self.inventory {
            self.inventory = false;
            return true;
        }
        false
    }

    pub fn toggle_map(&mut self) {
        self.world_map = !self.world_map;
    }

    pub fn toggle_pets(&mut self, sim: &Sim) {
        self.pets = !self.pets;
        if self.pets {
            self.pet_sel = sim.save.active_pet.unwrap_or(0);
        }
    }

    pub fn toggle_keys(&mut self, inv: bool, chr: bool, abil: bool) {
        if inv {
            self.inventory = !self.inventory;
        }
        if chr {
            self.character = !self.character;
        }
        if abil {
            self.abilities = !self.abilities;
        }
    }

    fn say(&mut self, s: impl Into<String>, err: bool) {
        self.msg = Some((s.into(), 2.5, err));
    }

    fn hit(&mut self, r: (f32, f32, f32, f32), t: Target) {
        self.hits.push((r, t));
    }

    /// Target under the mouse using last frame's layout (for hover highlights while drawing).
    fn hover(&self) -> Option<Target> {
        Self::find(&self.prev_hits)
    }

    fn under_mouse(&self) -> Option<Target> {
        Self::find(&self.hits)
    }

    fn find(hits: &[((f32, f32, f32, f32), Target)]) -> Option<Target> {
        let (mx, my) = gfx::mouse();
        // Last drawn wins (windows on top), but prefer specific targets over Window.
        let mut found = None;
        for (r, t) in hits.iter() {
            if mx >= r.0 && mx <= r.0 + r.2 && my >= r.1 && my <= r.1 + r.3 {
                match (found, t) {
                    (Some(Target::Window), _) | (None, _) => found = Some(*t),
                    (Some(_), Target::Window) => {}
                    _ => found = Some(*t),
                }
            }
        }
        found
    }

    /// Draw everything and handle the mouse. Call after the HUD.
    pub fn frame(&mut self, sim: &mut Sim, dt: f32, ui: &Ui) {
        gfx::clear_cam();
        self.prev_hits = std::mem::take(&mut self.hits);
        if sim.shop_open.is_some() && !self.shop_was_open {
            self.inventory = true;
            self.abilities = false;
            self.shop_page = 0;
        }
        self.shop_was_open = sim.shop_open.is_some();
        let ui = Ui { chars: Vec::new(), clicked: ui.clicked, enabled: ui.enabled };
        self.hotbar(sim);
        if self.character {
            self.character_window(sim, &ui);
        }
        if self.abilities && sim.shop_open.is_none() {
            self.abilities_window(sim);
        }
        if self.pets {
            self.pets_window(sim, &ui);
        }
        if let Some(shop) = sim.shop_open {
            self.shop_window(sim, &ui, shop);
        }
        if self.inventory {
            self.inventory_window(sim, &ui);
        }
        if sim.teleport_open {
            self.teleport_window(sim, &ui);
        }
        if self.world_map {
            self.world_map_window(sim, &ui);
        }
        self.mouse(sim);
        self.tooltip(sim);
        if let Some((m, t, err)) = &mut self.msg {
            *t -= dt;
            let a = (*t).min(1.0).max(0.0);
            let c = if *err { rgb(255, 140, 130) } else { rgb(170, 255, 190) };
            gfx::text_centered_outlined(m, gfx::VW * 0.5, BAR_Y - 34.0, 16.0, alpha(c, a));
            if *t <= 0.0 {
                self.msg = None;
            }
        }
    }

    // ------------------------------------------------------------------ hotbar

    fn hotbar(&mut self, sim: &Sim) {
        let x0 = bar_x0();
        let layout = sim.settings.layout;
        widgets::hud_panel(x0 - 8.0, BAR_Y - 6.0, BAR_STEP * HOTBAR_SLOTS as f32 + 12.0, SLOT + 12.0);
        let hover = self.hover();
        for i in 0..HOTBAR_SLOTS {
            let x = x0 + i as f32 * BAR_STEP;
            let y = BAR_Y;
            let h = hover == Some(Target::Bar(i));
            gfx::rrect(x, y, SLOT, SLOT, 6.0, rgba(0, 0, 0, 120));
            gfx::rect_lines(x, y, SLOT, SLOT, 1.0, if h { rgb(255, 230, 150) } else { rgba(255, 255, 255, 70) });
            match sim.save.hotbar.get(i) {
                Some(HotbarSlot::Skill(id)) => {
                    if let Some(s) = skills::skill(id) {
                        let known = skills::knows(sim.save.class, sim.save.level, s);
                        let usable = known && sim.player.mp >= s.mp;
                        gfx::tex(&skill_tex(s), x + 5.0, y + 5.0, 1.0, false, if usable { WHITE } else { Color::new(0.5, 0.5, 0.55, 1.0) });
                        let cd = sim.player.cooldown(s.id);
                        if cd > 0.0 {
                            let f = (cd / s.cooldown).clamp(0.0, 1.0);
                            gfx::rect(x + 5.0, y + 5.0 + 36.0 * (1.0 - f), 36.0, 36.0 * f, rgba(0, 0, 0, 150));
                            gfx::text_centered_outlined(&format!("{}", cd.ceil() as i32), x + SLOT * 0.5, y + 14.0, 16.0, WHITE);
                        } else if !known {
                            gfx::text_centered_outlined(&format!("Lv{}", s.level), x + SLOT * 0.5, y + 16.0, 13.0, rgb(255, 200, 160));
                        }
                    }
                }
                Some(HotbarSlot::Item(id)) => {
                    if let Some(def) = items::item(id) {
                        let n = sim.save.inventory.count(id);
                        gfx::tex(&item_tex(def), x + 5.0, y + 5.0, 1.0, false, if n > 0 { WHITE } else { Color::new(0.5, 0.5, 0.55, 0.8) });
                        gfx::text_outlined(&format!("{}", n), x + SLOT - 6.0 - gfx::text_w(&format!("{}", n), 13.0), y + SLOT - 16.0, 13.0, WHITE);
                        if sim.player.potion_cd > 0.0 {
                            gfx::rect(x + 5.0, y + 5.0, 36.0, 36.0 * sim.player.potion_cd.min(1.0), rgba(0, 0, 0, 120));
                        }
                    }
                }
                _ => {}
            }
            gfx::text_outlined(&layout.key_label(HOTBAR_ACTIONS[i]), x + 3.0, y + 1.0, 13.0, rgb(255, 230, 150));
            self.hit((x, y, SLOT, SLOT), Target::Bar(i));
        }
        // Window shortcuts next to the bar.
        let hint = format!(
            "[{}] Bag  [{}] Stats  [{}] Abilities",
            layout.key_label(Action::Inventory),
            layout.key_label(Action::Character),
            layout.key_label(Action::Abilities)
        );
        gfx::text_outlined(&hint, x0 + BAR_STEP * HOTBAR_SLOTS as f32 + 16.0, BAR_Y + 14.0, 13.0, rgb(200, 210, 230));
    }

    // ------------------------------------------------------------------ inventory

    fn inventory_window(&mut self, sim: &mut Sim, ui: &Ui) {
        let (x, y, w, h) = (gfx::VW - 404.0, 56.0, 384.0, 572.0);
        widgets::panel(x, y, w, h);
        self.hit((x, y, w, h), Target::Window);
        widgets::title("Inventory", x + w * 0.5, y + 14.0, 22.0);
        if widgets::button(ui, x + w - 52.0, y + 12.0, 34.0, 30.0, "X", ButtonStyle::Secondary, true) {
            self.inventory = false;
            sim.shop_open = None;
        }
        let hover = self.hover();
        // equipment
        gfx::text("Equipped", x + 24.0, y + 50.0, 14.0, widgets::ink_soft());
        for (k, &slot) in EQUIP_SLOTS.iter().enumerate() {
            // two rows of six (the weapon first)
            let sx = x + 24.0 + (k % 6) as f32 * 58.0;
            let sy = y + 66.0 + (k / 6) as f32 * 70.0;
            let def = sim.save.inventory.equipped(slot);
            slot_bg(sx, sy, SLOT, hover == Some(Target::Equip(slot)), def.map(|d| px_color(icons::tier_color(d.tier))));
            match def {
                Some(d) => gfx::tex(&item_tex(d), sx + 5.0, sy + 5.0, 1.0, false, WHITE),
                None => gfx::text_centered(&slot.name()[..1], sx + SLOT * 0.5, sy + 14.0, 16.0, rgb(190, 170, 140)),
            }
            gfx::text_centered(slot.name(), sx + SLOT * 0.5, sy + SLOT + 3.0, 11.0, widgets::ink_soft());
            self.hit((sx, sy, SLOT, SLOT), Target::Equip(slot));
        }
        // bag grid
        let gy = y + 232.0;
        gfx::text(&format!("Bag ({}/{})", sim.save.inventory.slots.len(), INVENTORY_SLOTS), x + 24.0, gy - 20.0, 14.0, widgets::ink_soft());
        let cols = 6;
        for i in 0..INVENTORY_SLOTS {
            let sx = x + 24.0 + (i % cols) as f32 * 56.0;
            let sy = gy + (i / cols) as f32 * 56.0;
            let st = sim.save.inventory.slots.get(i);
            let def = st.and_then(|s| items::item(&s.id));
            let dragging_this = self.dragging && self.press.map(|p| p.0) == Some(Target::Inv(i));
            slot_bg(sx, sy, SLOT, hover == Some(Target::Inv(i)), def.map(|d| px_color(icons::tier_color(d.tier))));
            if let (Some(st), Some(d)) = (st, def) {
                let tint = if dragging_this { Color::new(1.0, 1.0, 1.0, 0.35) } else if sim.equip_problem(d.id).is_some() { Color::new(1.0, 0.6, 0.6, 1.0) } else { WHITE };
                gfx::tex(&item_tex(d), sx + 5.0, sy + 5.0, 1.0, false, tint);
                if st.count > 1 {
                    let t = format!("{}", st.count);
                    gfx::text_outlined(&t, sx + SLOT - 5.0 - gfx::text_w(&t, 13.0), sy + SLOT - 16.0, 13.0, WHITE);
                }
            }
            self.hit((sx, sy, SLOT, SLOT), Target::Inv(i));
        }
        gfx::circle(x + 34.0, y + h - 56.0, 8.0, rgb(240, 196, 70));
        gfx::text(&format!("{} gold", sim.save.gold), x + 48.0, y + h - 64.0, 16.0, widgets::ink());
        let help = if sim.shop_open.is_some() { "Click: use/equip. Right-click: sell" } else { "Click: use/equip. Drag to hotbar, or out to drop" };
        gfx::text(help, x + 24.0, y + h - 38.0, 12.0, widgets::ink_soft());
    }

    // ------------------------------------------------------------------ character

    fn character_window(&mut self, sim: &mut Sim, ui: &Ui) {
        let (x, y, w, h) = (20.0, 150.0, 372.0, 470.0);
        widgets::panel(x, y, w, h);
        self.hit((x, y, w, h), Target::Window);
        widgets::title("Character", x + w * 0.5, y + 14.0, 22.0);
        if widgets::button(ui, x + w - 52.0, y + 12.0, 34.0, 30.0, "X", ButtonStyle::Secondary, true) {
            self.character = false;
        }
        let info = sim.player.info.clone();
        gfx::text(&format!("{}  -  Lv.{} {}", sim.save.name, sim.save.level, info.name), x + 24.0, y + 46.0, 16.0, widgets::ink());
        let sub = if sim.save.class == ClassId::Novice {
            format!("Change class at Lv.{}", galebound_core::classes::CLASS_CHANGE_LEVEL)
        } else {
            format!("Main stats: {} / {}", info.primary.name(), info.secondary.name())
        };
        gfx::text(&sub, x + 24.0, y + 68.0, 13.0, widgets::accent());
        // main stats
        let mut yy = y + 96.0;
        let ap = sim.save.ap;
        gfx::text(&format!("Ability points: {}", ap), x + 24.0, yy, 16.0, if ap > 0 { rgb(200, 110, 30) } else { widgets::ink_soft() });
        if ap > 0 && widgets::button(ui, x + w - 120.0, yy - 6.0, 96.0, 30.0, "Auto", ButtonStyle::Secondary, true) {
            sim.auto_spend_ap();
        }
        yy += 32.0;
        let mut hovered_help: Option<&'static str> = None;
        for &st in MAIN_STATS {
            let v = sim.player.stats.main.get(st);
            let base = sim.save.stats.get(st);
            let main = st == info.primary || st == info.secondary;
            gfx::text(st.long_name(), x + 24.0, yy, 16.0, if main { widgets::ink() } else { widgets::ink_soft() });
            let vs = if v != base { format!("{} (+{})", base, v - base) } else { format!("{}", v) };
            gfx::text(&vs, x + 180.0, yy, 16.0, widgets::ink());
            if gfx::mouse_in(x + 20.0, yy - 4.0, 240.0, 26.0) {
                hovered_help = Some(st.help());
            }
            if ap > 0 && widgets::button(ui, x + w - 64.0, yy - 6.0, 40.0, 28.0, "+", ButtonStyle::Primary, true) {
                let n = if shift() { ap.min(5) } else { 1 };
                for _ in 0..n {
                    sim.spend_ap(st);
                }
            }
            yy += 34.0;
        }
        // derived
        yy += 4.0;
        gfx::rect(x + 24.0, yy, w - 48.0, 2.0, rgb(214, 196, 160));
        yy += 12.0;
        let d = sim.player.stats;
        let rows = [
            ("HP", format!("{}", d.max_hp.round() as i32)),
            ("Mana", format!("{}", d.max_mp.round() as i32)),
            ("Physical ATK", format!("{}", d.patk.round() as i32)),
            ("Magic ATK", format!("{}", d.matk.round() as i32)),
            ("Physical DEF", format!("{}", d.pdef.round() as i32)),
            ("Magical DEF", format!("{}", d.mdef.round() as i32)),
            ("Critical Chance", format!("{:.1}%", d.crit_chance)),
            ("Critical Damage", format!("{}%", d.crit_damage.round() as i32)),
        ];
        for (i, (k, v)) in rows.iter().enumerate() {
            let cx = x + 24.0 + (i % 2) as f32 * 168.0;
            let cy = yy + (i / 2) as f32 * 40.0;
            gfx::text(k, cx, cy, 12.0, widgets::ink_soft());
            gfx::text(v, cx, cy + 16.0, 16.0, widgets::ink());
        }
        if !sim.player.buffs.is_empty() {
            gfx::text("Buffs active: values include them", x + 24.0, y + h - 30.0, 12.0, widgets::accent());
        }
        if let Some(help) = hovered_help {
            let tw = 300.0;
            let (mx, my) = gfx::mouse();
            let lines = gfx::wrap(help, tw - 20.0, 14.0);
            let th = 16.0 + lines.len() as f32 * 18.0;
            gfx::rrect(mx + 16.0, my + 10.0, tw, th, 6.0, rgba(20, 24, 40, 235));
            for (i, l) in lines.iter().enumerate() {
                gfx::text(l, mx + 26.0, my + 18.0 + i as f32 * 18.0, 14.0, WHITE);
            }
        }
    }

    // ------------------------------------------------------------------ abilities

    fn abilities_window(&mut self, sim: &mut Sim) {
        let list = skills::class_skills(sim.save.class);
        let row_h = 70.0;
        let (w, h) = (480.0, 84.0 + list.len() as f32 * row_h);
        let (x, y) = ((gfx::VW - w) * 0.5, (BAR_Y - 20.0 - h).max(44.0));
        widgets::panel(x, y, w, h);
        self.hit((x, y, w, h), Target::Window);
        widgets::title("Abilities", x + w * 0.5, y + 14.0, 22.0);
        gfx::text_centered("Drag onto the hotbar, or click to add", x + w * 0.5, y + 44.0, 13.0, widgets::ink_soft());
        let hover = self.hover();
        for (i, s) in list.iter().enumerate() {
            let ry = y + 66.0 + i as f32 * row_h;
            let known = skills::knows(sim.save.class, sim.save.level, s);
            let hv = hover == Some(Target::Skill(s.id));
            if hv {
                gfx::rrect(x + 16.0, ry - 4.0, w - 32.0, row_h - 4.0, 6.0, rgba(64, 156, 120, 40));
            }
            slot_bg(x + 22.0, ry, SLOT, hv, None);
            gfx::tex(&skill_tex(s), x + 27.0, ry + 5.0, 1.0, false, if known { WHITE } else { Color::new(0.45, 0.45, 0.5, 1.0) });
            let nc = if known { widgets::ink() } else { rgb(150, 140, 130) };
            gfx::text(s.name, x + 80.0, ry, 16.0, nc);
            let meta = if known {
                format!("{}  -  {} MP  -  {}s cooldown", s.kind_label(), s.mp as i32, s.cooldown)
            } else {
                format!("Learn at Lv.{}{}", s.level, if s.class != sim.save.class { "" } else { "" })
            };
            gfx::text(&meta, x + 80.0, ry + 20.0, 12.0, if known { widgets::accent() } else { rgb(200, 110, 30) });
            for (k, l) in gfx::wrap(s.desc, w - 110.0, 12.0).iter().take(2).enumerate() {
                gfx::text(l, x + 80.0, ry + 34.0 + k as f32 * 16.0, 12.0, widgets::ink_soft());
            }
            self.hit((x + 16.0, ry - 4.0, w - 32.0, row_h - 4.0), Target::Skill(s.id));
        }
    }

    // ------------------------------------------------------------------ pets

    fn pets_window(&mut self, sim: &mut Sim, ui: &Ui) {
        use galebound_core::pets::{self, PetSave};
        let (w, h) = (640.0, 590.0);
        let (x, y) = ((gfx::VW - w) * 0.5, 44.0);
        widgets::panel(x, y, w, h);
        self.hit((x, y, w, h), Target::Window);
        widgets::title("Pets", x + w * 0.5, y + 14.0, 22.0);
        if widgets::button(ui, x + w - 52.0, y + 12.0, 34.0, 30.0, "X", ButtonStyle::Secondary, true) {
            self.pets = false;
            return;
        }
        let food = sim.save.inventory.count("pet_food");
        if sim.save.pets.is_empty() {
            gfx::text_wrapped(
                &format!(
                    "You don't have a pet yet. The Pet Shop in Breezehollow and in every class town sells pets for {} gold, and Pet Food for {} gold.",
                    pets::PET_PRICE, pets::FOOD_PRICE
                ),
                x + 40.0,
                y + 80.0,
                w - 80.0,
                16.0,
                widgets::ink(),
            );
            gfx::text_wrapped(
                "A pet follows you and picks up loot near you. Feed it Pet Food to level it up and unlock passive skills. Its hunger drops while it's out - a starving pet won't help.",
                x + 40.0,
                y + 170.0,
                w - 80.0,
                14.0,
                widgets::ink_soft(),
            );
            return;
        }
        self.pet_sel = self.pet_sel.min(sim.save.pets.len() - 1);

        // ---- list of pets
        for (i, p) in sim.save.pets.iter().enumerate() {
            let (rx, ry, rw, rh) = (x + 24.0, y + 56.0 + i as f32 * 62.0, 170.0, 56.0);
            let sel = self.pet_sel == i;
            let hover = gfx::mouse_in(rx, ry, rw, rh);
            gfx::rrect(rx, ry, rw, rh, 8.0, if sel { rgb(196, 226, 200) } else if hover { rgb(240, 228, 200) } else { rgb(232, 216, 182) });
            let tex = sprites::get(&format!("pet_{}_0", p.species.key()), || crate::petart::pet(p.species, 0));
            gfx::tex(&tex, rx + 6.0, ry + 8.0, 1.0, false, WHITE);
            gfx::text(&p.name, rx + 56.0, ry + 8.0, 15.0, widgets::ink());
            let out = sim.save.active_pet == Some(i);
            gfx::text(&format!("Lv.{}", p.level), rx + 56.0, ry + 30.0, 13.0, widgets::ink_soft());
            if out {
                gfx::text("Out", rx + rw - 40.0, ry + 30.0, 13.0, widgets::accent());
            }
            if hover && ui.clicked {
                self.pet_sel = i;
            }
        }

        // ---- details of the selected pet
        let i = self.pet_sel;
        let p: PetSave = sim.save.pets[i].clone();
        let out = sim.save.active_pet == Some(i);
        let dx = x + 216.0;
        let dw = w - 240.0;
        gfx::rrect(dx, y + 56.0, 120.0, 100.0, 10.0, rgb(214, 232, 240));
        let frame = if out && !p.starving() { ((sim.time * 4.0) as u8) % 2 } else { 0 };
        let tex = sprites::get(&format!("pet_{}_{}", p.species.key(), frame), || crate::petart::pet(p.species, frame));
        // k = 2 canvas pixels per sprite pixel = 4 UI units
        gfx::tex(&tex, dx + 60.0 - tex.width() * 2.0, y + 106.0 - tex.height() * 2.0, 2.0, false, if p.starving() { Color::new(0.75, 0.75, 0.8, 1.0) } else { WHITE });
        gfx::text(&p.name, dx + 136.0, y + 58.0, 20.0, widgets::ink());
        gfx::text(&format!("{}  -  Lv.{} / {}", p.species.name(), p.level, pets::MAX_LEVEL), dx + 136.0, y + 84.0, 13.0, widgets::ink_soft());
        let (status, col) = if p.starving() {
            ("Starving! It won't loot or help until fed.", widgets::danger())
        } else if out {
            ("Out with you, picking up loot.", widgets::accent())
        } else {
            ("Resting at home.", widgets::ink_soft())
        };
        gfx::text(status, dx + 136.0, y + 104.0, 13.0, col);
        gfx::text(&format!("Loot range: {}", p.loot_radius().round() as i32), dx + 136.0, y + 124.0, 12.0, widgets::ink_soft());

        // bars
        let bar = |by: f32, label: &str, frac: f32, c: Color| {
            gfx::text(label, dx, by, 13.0, widgets::ink());
            gfx::rrect(dx, by + 18.0, dw, 12.0, 5.0, rgb(196, 180, 150));
            gfx::rrect(dx + 2.0, by + 20.0, (dw - 4.0) * frac.clamp(0.0, 1.0), 8.0, 4.0, c);
        };
        if p.level >= pets::MAX_LEVEL {
            bar(y + 168.0, "EXP  (max level)", 1.0, rgb(150, 110, 220));
        } else {
            let need = pets::exp_to_next(p.level);
            bar(y + 168.0, &format!("EXP  {} / {}", p.exp, need), p.exp as f32 / need as f32, rgb(150, 110, 220));
        }
        let hf = p.hunger / pets::HUNGER_MAX;
        let hc = if hf > 0.5 { rgb(90, 180, 90) } else if hf > 0.2 { rgb(220, 170, 50) } else { rgb(210, 70, 60) };
        bar(y + 204.0, &format!("Hunger  {} / {}", p.hunger.ceil() as i32, pets::HUNGER_MAX as i32), hf, hc);

        // buttons
        let by = y + 248.0;
        if out {
            if widgets::button(ui, dx, by, 140.0, 36.0, "Send home", ButtonStyle::Secondary, true) {
                sim.dismiss_pet();
                self.say(format!("{} went home to rest", p.name), false);
            }
        } else if widgets::button(ui, dx, by, 140.0, 36.0, "Summon", ButtonStyle::Primary, true) {
            match sim.summon_pet(i) {
                Ok(()) => self.say(format!("{} is out with you", p.name), false),
                Err(e) => self.say(e, true),
            }
        }
        let feed_label = format!("Feed ({})", food);
        if widgets::button(ui, dx + 152.0, by, 140.0, 36.0, &feed_label, ButtonStyle::Primary, out && food > 0) {
            match sim.feed_pet() {
                Ok(n) if n > 0 => self.say(format!("{} reached Lv.{}!", p.name, p.level + n), false),
                Ok(_) => self.say(format!("{} enjoyed that", p.name), false),
                Err(e) => self.say(e, true),
            }
        }
        let hint = if !out {
            "Summon it to feed it.".to_string()
        } else if food == 0 {
            "Buy Pet Food at any Pet Shop.".to_string()
        } else {
            format!("Each Pet Food: +{} hunger, +{} pet EXP.", pets::FOOD_HUNGER as i32, pets::FOOD_EXP)
        };
        gfx::text(&hint, dx, by + 46.0, 12.0, widgets::ink_soft());

        // passives
        let mut py = y + 318.0;
        gfx::text("Passive skills", dx, py, 15.0, widgets::ink());
        py += 24.0;
        for ps in p.species.passives() {
            let unlocked = p.level >= ps.level;
            let working = unlocked && !p.starving() && out;
            let dot = if working { widgets::accent() } else if unlocked { rgb(200, 160, 60) } else { rgb(170, 160, 150) };
            gfx::circle(dx + 6.0, py + 8.0, 5.0, dot);
            gfx::text(ps.name, dx + 18.0, py, 14.0, if unlocked { widgets::ink() } else { rgb(150, 140, 130) });
            let tag = format!("Lv.{}", ps.level);
            gfx::text(&tag, dx + dw - gfx::text_w(&tag, 12.0), py + 2.0, 12.0, if unlocked { widgets::accent() } else { rgb(150, 140, 130) });
            gfx::text(&clip(ps.desc, dw - 18.0, 12.0), dx + 18.0, py + 17.0, 12.0, widgets::ink_soft());
            py += 38.0;
        }
        gfx::text("Green: active.  Gold: unlocked (pet must be out and fed).", x + 24.0, y + h - 36.0, 12.0, widgets::ink_soft());
    }

    // ------------------------------------------------------------------ shop

    fn shop_window(&mut self, sim: &mut Sim, ui: &Ui, shop_id: &'static str) {
        let Some(shop) = items::shop(shop_id) else { return };
        let row_h = 50.0;
        const PER_PAGE: usize = 8;
        let pages = shop.stock.len().div_ceil(PER_PAGE).max(1);
        self.shop_page = self.shop_page.min(pages - 1);
        let rows = PER_PAGE.min(shop.stock.len());
        let (x, y, w) = (360.0, 70.0, 500.0);
        let h = 120.0 + rows as f32 * row_h + if pages > 1 { 40.0 } else { 0.0 };
        widgets::panel(x, y, w, h);
        self.hit((x, y, w, h), Target::Window);
        widgets::title(shop.name, x + w * 0.5, y + 14.0, 22.0);
        gfx::text_centered(&format!("{}  -  you have {} gold", shop.kind.name(), thousands(sim.save.gold)), x + w * 0.5, y + 44.0, 13.0, widgets::ink_soft());
        if widgets::button(ui, x + w - 52.0, y + 12.0, 34.0, 30.0, "X", ButtonStyle::Secondary, true) {
            sim.shop_open = None;
            self.inventory = false;
            return;
        }
        let hover = self.hover();
        let mut buy: Option<(&'static str, u32)> = None;
        for (i, id) in shop.stock.iter().skip(self.shop_page * PER_PAGE).take(PER_PAGE).enumerate() {
            let Some(def) = items::item(id) else { continue };
            let ry = y + 70.0 + i as f32 * row_h;
            let hv = hover == Some(Target::Shop(def.id));
            if hv {
                gfx::rrect(x + 16.0, ry - 3.0, w - 32.0, row_h - 4.0, 6.0, rgba(64, 156, 120, 40));
            }
            slot_bg(x + 22.0, ry, 40.0, hv, Some(px_color(icons::tier_color(def.tier))));
            gfx::tex(&item_tex(def), x + 24.0, ry + 2.0, 1.0, false, WHITE);
            let text_w = w - 74.0 - 200.0;
            gfx::text(&clip(def.name, text_w + 40.0, 15.0), x + 74.0, ry + 2.0, 15.0, widgets::ink());
            let each = sim.unit_price(def.id);
            let discounted = each < def.price;
            let problem = sim.equip_problem(def.id);
            let (sub, sub_col) = match &problem {
                Some(p) => (p.clone(), widgets::danger()),
                None if discounted => (format!("40% off - {} left", sim.trainee_discounts_left()), rgb(40, 140, 80)),
                None if def.bound => ("Bound to you".to_string(), rgb(150, 90, 170)),
                None => (def.stat_lines().first().cloned().unwrap_or_else(|| def.desc.to_string()), widgets::ink_soft()),
            };
            let price = format!("{}g", thousands(each));
            let afford = sim.save.gold >= each;
            let px = x + w - 136.0;
            let sub_w = text_w.min(px - gfx::text_w(&price, 15.0) - 14.0 - (x + 74.0));
            gfx::text(&clip(&sub, sub_w, 12.0), x + 74.0, ry + 22.0, 12.0, sub_col);
            gfx::text(&price, px - gfx::text_w(&price, 15.0), if discounted { ry + 2.0 } else { ry + 10.0 }, 15.0, if afford { rgb(170, 120, 20) } else { widgets::danger() });
            if discounted {
                // the full price, struck through
                let old = format!("{}g", def.price);
                let ow = gfx::text_w(&old, 12.0);
                gfx::text(&old, px - ow, ry + 22.0, 12.0, rgb(150, 140, 130));
                gfx::rect(px - ow - 1.0, ry + 28.0, ow + 2.0, 2.0, rgb(170, 70, 60));
            }
            let label = if def.stackable() && shift() { "Buy 10" } else { "Buy" };
            if widgets::button(ui, x + w - 124.0, ry + 4.0, 98.0, 34.0, label, ButtonStyle::Primary, afford) {
                let n = if def.stackable() && shift() { 10 } else { 1 };
                buy = Some((def.id, n));
            }
            self.hit((x + 16.0, ry - 3.0, w - 160.0, row_h - 4.0), Target::Shop(def.id));
        }
        if pages > 1 {
            let py = y + h - 76.0;
            if widgets::button(ui, x + 24.0, py, 90.0, 32.0, "< Prev", ButtonStyle::Secondary, self.shop_page > 0) {
                self.shop_page -= 1;
            }
            gfx::text_centered(&format!("Page {}/{}", self.shop_page + 1, pages), x + w * 0.5, py + 8.0, 14.0, widgets::ink_soft());
            if widgets::button(ui, x + w - 114.0, py, 90.0, 32.0, "Next >", ButtonStyle::Secondary, self.shop_page + 1 < pages) {
                self.shop_page += 1;
            }
        }
        gfx::text_centered("Shift: buy x10. Right-click to sell.", x + w * 0.5, y + h - 34.0, 12.0, widgets::ink_soft());
        if let Some((id, n)) = buy {
            match sim.buy(id, n) {
                Ok(g) => self.say(format!("Bought {}x {} for {} gold", n, items::item(id).map(|d| d.name).unwrap_or(id), g), false),
                Err(e) => self.say(e, true),
            }
        }
    }

    // ------------------------------------------------------------------ waystone travel

    fn teleport_window(&mut self, sim: &mut Sim, ui: &Ui) {
        let list = world::WAYSTONES;
        let (w, h) = (420.0, 130.0 + list.len() as f32 * 54.0);
        let (x, y) = ((gfx::VW - w) * 0.5, 90.0);
        widgets::panel(x, y, w, h);
        self.hit((x, y, w, h), Target::Window);
        widgets::title("Waystone Travel", x + w * 0.5, y + 14.0, 22.0);
        gfx::text_centered(&format!("{} gold per trip  -  you have {} gold", WAYSTONE_COST, sim.save.gold), x + w * 0.5, y + 44.0, 13.0, widgets::ink_soft());
        if widgets::button(ui, x + w - 52.0, y + 12.0, 34.0, 30.0, "X", ButtonStyle::Secondary, true) {
            sim.teleport_open = false;
            return;
        }
        let attuned = sim.attuned_waystones();
        let mut go: Option<&'static str> = None;
        for (i, key) in list.iter().enumerate() {
            let ry = y + 72.0 + i as f32 * 54.0;
            let name = world::load_map(key).map(|m| m.name).unwrap_or(key);
            let known = attuned.contains(key);
            let here = sim.map.key == *key;
            gfx::circle(x + 40.0, ry + 18.0, 8.0, if known { rgb(120, 220, 255) } else { rgb(190, 180, 170) });
            gfx::text(name, x + 60.0, ry + 4.0, 16.0, if known { widgets::ink() } else { rgb(160, 150, 140) });
            let sub = if here { "You are here" } else if known { "Attuned" } else { "Not visited yet" };
            gfx::text(sub, x + 60.0, ry + 24.0, 12.0, widgets::ink_soft());
            let ok = known && !here && sim.save.gold >= WAYSTONE_COST;
            if widgets::button(ui, x + w - 130.0, ry + 2.0, 100.0, 34.0, "Travel", ButtonStyle::Primary, ok) {
                go = Some(key);
            }
        }
        if let Some(k) = go {
            match sim.teleport(k) {
                Ok(()) => self.say(format!("Travelled to {}", sim.map.name), false),
                Err(e) => self.say(e, true),
            }
        }
    }

    // ------------------------------------------------------------------ world map

    fn world_map_window(&mut self, sim: &mut Sim, ui: &Ui) {
        if self.map_cache.is_none() {
            self.map_cache = Some(WorldCache::build());
        }
        let cache = self.map_cache.take().unwrap();
        let (x, y, w, h) = (40.0, 40.0, 1200.0, 640.0);
        gfx::rrect(x - 4.0, y - 4.0, w + 8.0, h + 8.0, 16.0, rgba(0, 0, 0, 90));
        widgets::panel(x, y, w, h);
        self.hit((x, y, w, h), Target::Window);
        widgets::title("World Map", x + 430.0, y + 14.0, 26.0);
        if widgets::button(ui, x + w - 52.0, y + 12.0, 34.0, 30.0, "X", ButtonStyle::Secondary, true) {
            self.world_map = false;
        }
        // map area (parchment with a soft sea tint)
        let (mx, my, mw, mh) = (x + 24.0, y + 54.0, 820.0, 560.0);
        gfx::rrect(mx, my, mw, mh, 10.0, rgb(214, 226, 214));
        gfx::rrect(mx + 6.0, my + 6.0, mw - 12.0, mh - 12.0, 8.0, rgb(232, 230, 206));
        let sx = (mw - 60.0) / 960.0;
        let sy = (mh - 60.0) / 560.0;
        let at = |p: (f32, f32)| (mx + 30.0 + p.0 * sx, my + 30.0 + p.1 * sy);
        // routes
        for &(a, b) in &cache.edges {
            let (pa, pb) = (at(world::WORLD[a].pos), at(world::WORLD[b].pos));
            gfx::line(pa.0, pa.1, pb.0, pb.1, 5.0, rgb(196, 170, 120));
            gfx::line(pa.0, pa.1, pb.0, pb.1, 2.0, rgb(150, 116, 74));
        }
        let here = world::world_node_for(sim.map.key).map(|n| n.key);
        let (mxp, myp) = gfx::mouse();
        let mut hovered = None;
        for n in world::WORLD {
            let (px, py) = at(n.pos);
            let r: f32 = match n.kind {
                world::NodeKind::Town => 13.0,
                _ => 10.0,
            };
            let col = match n.kind {
                world::NodeKind::Town => rgb(232, 182, 72),
                world::NodeKind::Field => rgb(110, 176, 96),
                world::NodeKind::Hunting => rgb(200, 84, 70),
            };
            let hov = (mxp - px).powi(2) + (myp - py).powi(2) < (r + 6.0).powi(2);
            if hov {
                hovered = Some(n.key);
            }
            let sel = self.map_sel == Some(n.key);
            gfx::circle(px, py, r + 3.0, if sel || hov { rgb(255, 255, 255) } else { rgb(70, 52, 36) });
            gfx::circle(px, py, r, col);
            if sim.save.has_flag(&format!("ws_{}", n.key)) {
                gfx::circle(px + r, py - r, 5.0, rgb(120, 220, 255));
            }
            let name = cache.name(n.key);
            // Long names wrap onto two lines; every other column labels above the node.
            let lines: Vec<String> = match name.find(' ') {
                Some(_) if gfx::text_w(name, 13.0) > 96.0 => {
                    let mid = name.len() / 2;
                    let cut = name.match_indices(' ').map(|(i, _)| i).min_by_key(|i| (*i as i32 - mid as i32).abs()).unwrap();
                    vec![name[..cut].to_string(), name[cut + 1..].to_string()]
                }
                _ => vec![name.to_string()],
            };
            let col_idx = ((n.pos.0 - 110.0) / 190.0).round() as i32;
            let above = col_idx % 2 == 1 && n.kind != world::NodeKind::Hunting;
            let lh = 16.0;
            let tc = if Some(n.key) == here { rgb(255, 236, 140) } else { rgb(255, 255, 255) };
            for (li, l) in lines.iter().enumerate() {
                let ty = if above { py - r - 4.0 - (lines.len() - li) as f32 * lh } else { py + r + 4.0 + li as f32 * lh };
                gfx::text_centered_outlined(l, px, ty, 13.0, tc);
            }
            if Some(n.key) == here {
                let bob = (sim.time * 4.0).sin() * 3.0;
                gfx::tri(px - 8.0, py - r - 18.0 + bob, px + 8.0, py - r - 18.0 + bob, px, py - r - 6.0 + bob, rgb(220, 60, 60));
                gfx::text_centered_outlined("You", px, py - r - 36.0 + bob, 13.0, rgb(255, 220, 200));
            }
        }
        if ui.clicked {
            if let Some(k) = hovered {
                self.map_sel = Some(k);
            }
        }
        // legend
        let ly = my + mh - 30.0;
        for (i, (label, col)) in [("Town", rgb(232, 182, 72)), ("Field / road", rgb(110, 176, 96)), ("Hunting ground", rgb(200, 84, 70)), ("Waystone", rgb(120, 220, 255))].iter().enumerate() {
            let lx = mx + 20.0 + i as f32 * 170.0;
            gfx::circle(lx, ly + 8.0, 7.0, *col);
            gfx::text(label, lx + 14.0, ly, 13.0, widgets::ink_soft());
        }
        // info panel
        let key = hovered.or(self.map_sel).or(here).unwrap_or("breezehollow");
        let (ix, iy, iw) = (mx + mw + 16.0, my, w - mw - 64.0);
        let node = world::WORLD.iter().find(|n| n.key == key);
        let mut yy = iy;
        gfx::text(cache.name(key), ix, yy, 20.0, widgets::ink());
        yy += 30.0;
        if Some(key) == here {
            gfx::text("You are here", ix, yy, 13.0, rgb(200, 80, 60));
            yy += 22.0;
        }
        match node.and_then(|n| n.levels) {
            Some((a, b)) => {
                let lv = sim.save.level;
                let col = if lv + 2 < a {
                    widgets::danger()
                } else if lv > b + 3 {
                    rgb(140, 140, 140)
                } else {
                    widgets::accent()
                };
                gfx::text(&format!("Recommended: Lv {}-{}", a, b), ix, yy, 15.0, col);
            }
            None => gfx::text("Safe zone - no monsters", ix, yy, 15.0, widgets::accent()),
        }
        yy += 30.0;
        let mons = cache.monsters(key);
        if !mons.is_empty() {
            gfx::text("Monsters", ix, yy, 15.0, widgets::ink());
            yy += 22.0;
            for k in mons {
                let d = k.def();
                let tag = if d.boss { " (boss)" } else { "" };
                gfx::text(&format!("Lv.{} {}{}", d.level, d.name, tag), ix + 8.0, yy, 13.0, if d.boss { widgets::danger() } else { widgets::ink_soft() });
                yy += 18.0;
            }
            yy += 10.0;
            gfx::text("Possible loot", ix, yy, 15.0, widgets::ink());
            yy += 22.0;
            let mut loot: Vec<&'static str> = vec![];
            for k in mons {
                for (id, _) in k.drops() {
                    if !loot.contains(id) {
                        loot.push(id);
                    }
                }
            }
            for (i, id) in loot.iter().take(12).enumerate() {
                if let Some(d) = items::item(id) {
                    let col = px_color(icons::tier_color(d.tier.max(1)));
                    let cy = yy + i as f32 * 18.0;
                    gfx::rect(ix + 8.0, cy + 4.0, 6.0, 6.0, col);
                    gfx::text(d.name, ix + 20.0, cy, 12.0, widgets::ink_soft());
                }
            }
            let _ = iw;
        } else if node.is_some() {
            let services = if world::WAYSTONES.contains(&key) { "Shops, pet shop, inn and waystone" } else { "A place to rest" };
            gfx::text(services, ix, yy, 13.0, widgets::ink_soft());
        }
        let layout = sim.settings.layout;
        gfx::text(&format!("[{}] close. Click a place.", layout.key_label(Action::WorldMap)), ix, y + h - 46.0, 12.0, widgets::ink_soft());
        self.map_cache = Some(cache);
    }

    // ------------------------------------------------------------------ mouse

    fn mouse(&mut self, sim: &mut Sim) {
        let down = is_mouse_button_down(MouseButton::Left);
        let pressed = down && !self.prev_down;
        let released = !down && self.prev_down;
        self.prev_down = down;
        let under = self.under_mouse();
        let m = gfx::mouse();

        if is_mouse_button_pressed(MouseButton::Right) {
            match under {
                Some(Target::Inv(i)) if sim.shop_open.is_some() => {
                    let n = if shift() { u32::MAX } else { 1 };
                    match sim.sell(i, n) {
                        Ok(g) => self.say(format!("Sold for {} gold", g), false),
                        Err(e) => self.say(e, true),
                    }
                }
                Some(Target::Bar(i)) => sim.set_hotbar(i, HotbarSlot::Empty),
                Some(Target::Equip(s)) => {
                    if let Err(e) = sim.unequip(s) {
                        self.say(e, true);
                    }
                }
                _ => {}
            }
        }

        if pressed {
            self.press = under.filter(|t| *t != Target::Window).map(|t| (t, m));
            self.dragging = false;
        }
        if down {
            if let Some((t, p0)) = self.press {
                let moved = (m.0 - p0.0).abs() + (m.1 - p0.1).abs() > 8.0;
                let draggable = match t {
                    Target::Inv(i) => i < sim.save.inventory.slots.len(),
                    Target::Bar(i) => sim.save.hotbar.get(i).map(|h| *h != HotbarSlot::Empty).unwrap_or(false),
                    Target::Skill(_) | Target::Equip(_) => true,
                    _ => false,
                };
                if moved && draggable {
                    self.dragging = true;
                }
            }
            if self.dragging {
                self.draw_drag(sim, m);
            }
        }
        if released {
            if let Some((src, _)) = self.press.take() {
                if self.dragging {
                    self.drop(sim, src, under);
                } else if under == Some(src) {
                    self.click(sim, src);
                }
            }
            self.dragging = false;
        }
    }

    fn draw_drag(&self, sim: &Sim, m: (f32, f32)) {
        let Some((src, _)) = self.press else { return };
        let tex = match src {
            Target::Inv(i) => sim.save.inventory.slots.get(i).and_then(|s| items::item(&s.id)).map(item_tex),
            Target::Equip(s) => sim.save.inventory.equipped(s).map(item_tex),
            Target::Skill(id) => skills::skill(id).map(skill_tex),
            Target::Bar(i) => match sim.save.hotbar.get(i) {
                Some(HotbarSlot::Skill(id)) => skills::skill(id).map(skill_tex),
                Some(HotbarSlot::Item(id)) => items::item(id).map(item_tex),
                _ => None,
            },
            _ => None,
        };
        if let Some(t) = tex {
            gfx::tex(&t, m.0 - 18.0, m.1 - 18.0, 1.0, false, Color::new(1.0, 1.0, 1.0, 0.85));
        }
    }

    fn click(&mut self, sim: &mut Sim, t: Target) {
        match t {
            Target::Inv(i) => {
                let Some(id) = sim.save.inventory.slots.get(i).map(|s| s.id.clone()) else { return };
                if let Some(def) = items::item(&id) {
                    if let ItemKind::Equip(_) = def.kind {
                        match sim.equip_from_slot(i) {
                            Ok(()) => self.say(format!("Equipped {}", def.name), false),
                            Err(e) => self.say(e, true),
                        }
                    } else {
                        sim.use_item(&id);
                    }
                }
            }
            Target::Equip(s) => {
                if let Err(e) = sim.unequip(s) {
                    self.say(e, true);
                }
            }
            Target::Skill(id) => {
                let slot = HotbarSlot::Skill(id.to_string());
                if sim.save.hotbar.contains(&slot) {
                    self.say("Already on your hotbar", false);
                } else if let Some(i) = sim.save.hotbar.iter().position(|h| *h == HotbarSlot::Empty) {
                    sim.set_hotbar(i, slot);
                    self.say(format!("Added to slot {}", i + 1), false);
                } else {
                    self.say("Hotbar is full - drag onto a slot to replace it", true);
                }
            }
            Target::Bar(i) => sim.use_hotbar(i),
            _ => {}
        }
    }

    fn drop(&mut self, sim: &mut Sim, src: Target, dst: Option<Target>) {
        match (src, dst) {
            (Target::Inv(i), Some(Target::Bar(j))) => {
                let Some(id) = sim.save.inventory.slots.get(i).map(|s| s.id.clone()) else { return };
                match items::item(&id).map(|d| d.kind) {
                    Some(ItemKind::Consumable(_)) | Some(ItemKind::Special(_)) => sim.set_hotbar(j, HotbarSlot::Item(id)),
                    _ => self.say("Only consumables go on the hotbar", true),
                }
            }
            (Target::Inv(i), Some(Target::Equip(slot))) => {
                if let Err(e) = sim.equip_into(i, slot) {
                    self.say(e, true);
                }
            }
            (Target::Inv(i), Some(Target::Inv(j))) => sim.swap_inventory(i, j),
            // Dragged out of every window: drop it on the ground.
            (Target::Inv(i), None) => {
                let name = sim.save.inventory.slots.get(i).and_then(|st| items::item(&st.id)).map(|d| d.name).unwrap_or("item");
                let n = if shift() { 1 } else { u32::MAX };
                match sim.drop_item(i, n) {
                    Ok(()) => self.say(format!("Dropped {}", name), false),
                    Err(e) => self.say(e, true),
                }
            }
            (Target::Skill(id), Some(Target::Bar(j))) => sim.set_hotbar(j, HotbarSlot::Skill(id.to_string())),
            (Target::Bar(i), Some(Target::Bar(j))) => sim.swap_hotbar(i, j),
            (Target::Bar(i), None) => sim.set_hotbar(i, HotbarSlot::Empty),
            (Target::Equip(s), Some(Target::Inv(_))) | (Target::Equip(s), Some(Target::Window)) => {
                if let Err(e) = sim.unequip(s) {
                    self.say(e, true);
                }
            }
            _ => {}
        }
    }

    // ------------------------------------------------------------------ tooltip

    fn tooltip(&self, sim: &Sim) {
        if self.dragging {
            return;
        }
        let mut lines: Vec<(String, Color, f32)> = Vec::new();
        let item_lines = |def: &ItemDef, lines: &mut Vec<(String, Color, f32)>, selling: bool| {
            lines.push((def.name.to_string(), px_color(icons::tier_color(def.tier.max(1))), 17.0));
            let kind = match def.kind {
                ItemKind::Consumable(_) => "Consumable".to_string(),
                ItemKind::Material => "Monster loot - sells to any shop".to_string(),
                ItemKind::Special(items::Special::Revive) => "Special - use when defeated".to_string(),
                ItemKind::Special(items::Special::SkillReset) => "Special - click to use".to_string(),
                ItemKind::Special(items::Special::Pet(_)) => "Pet - click to adopt".to_string(),
                ItemKind::Special(items::Special::PetFood) => "Pet Food - click to feed your pet".to_string(),
                ItemKind::Equip(s) => match def.class {
                    _ if s == items::GearKind::Ring || s == items::GearKind::Earring => format!("{} - fits either {} slot", s.name(), s.name().to_lowercase()),
                    Some(c) => format!("{} - {}", s.name(), c.info().name),
                    None => s.name().to_string(),
                },
            };
            lines.push((kind, rgb(190, 200, 220), 13.0));
            for l in def.stat_lines() {
                lines.push((l, rgb(160, 240, 170), 14.0));
            }
            if def.level > 1 {
                let ok = sim.save.level >= def.level;
                lines.push((format!("Requires Lv.{}", def.level), if ok { rgb(200, 200, 210) } else { rgb(255, 120, 110) }, 13.0));
            }
            if let Some(p) = sim.equip_problem(def.id) {
                if !p.starts_with("Requires") {
                    lines.push((p, rgb(255, 120, 110), 13.0));
                }
            }
            lines.push((def.desc.to_string(), rgb(230, 230, 240), 13.0));
            if def.bound {
                lines.push(("Bound: can't be sold, traded or dropped".to_string(), rgb(210, 150, 255), 13.0));
            }
            if selling && def.can_sell() {
                lines.push((format!("Sells for {}g", def.sell_price()), rgb(240, 200, 90), 13.0));
            }
        };
        let skill_lines = |s: &SkillDef, lines: &mut Vec<(String, Color, f32)>| {
            lines.push((s.name.to_string(), px_color(icons::fx_color(s.fx)), 17.0));
            lines.push((format!("{} - {} MP - {}s cooldown", s.kind_label(), s.mp as i32, s.cooldown), rgb(190, 200, 220), 13.0));
            if let SkillEffect::Strike { hits, .. } = s.effect {
                if hits > 1 {
                    lines.push((format!("{} hits", hits), rgb(160, 240, 170), 13.0));
                }
            }
            lines.push((s.desc.to_string(), rgb(230, 230, 240), 13.0));
            if !skills::knows(sim.save.class, sim.save.level, s) {
                lines.push((format!("Learned at Lv.{}", s.level), rgb(255, 160, 110), 13.0));
            }
        };
        match self.under_mouse() {
            Some(Target::Inv(i)) => {
                if let Some(d) = sim.save.inventory.slots.get(i).and_then(|s| items::item(&s.id)) {
                    item_lines(d, &mut lines, sim.shop_open.is_some());
                }
            }
            Some(Target::Equip(s)) => {
                if let Some(d) = sim.save.inventory.equipped(s) {
                    item_lines(d, &mut lines, false);
                }
            }
            Some(Target::Shop(id)) => {
                if let Some(d) = items::item(id) {
                    item_lines(d, &mut lines, false);
                }
            }
            Some(Target::Skill(id)) => {
                if let Some(s) = skills::skill(id) {
                    skill_lines(s, &mut lines);
                }
            }
            Some(Target::Bar(i)) => match sim.save.hotbar.get(i) {
                Some(HotbarSlot::Skill(id)) => {
                    if let Some(s) = skills::skill(id) {
                        skill_lines(s, &mut lines);
                    }
                }
                Some(HotbarSlot::Item(id)) => {
                    if let Some(d) = items::item(id) {
                        item_lines(d, &mut lines, false);
                    }
                }
                _ => {}
            },
            _ => {}
        }
        if lines.is_empty() {
            return;
        }
        let tw = 280.0;
        let mut wrapped: Vec<(String, Color, f32)> = Vec::new();
        for (t, c, s) in lines {
            for l in gfx::wrap(&t, tw - 24.0, s) {
                wrapped.push((l, c, s));
            }
        }
        let th: f32 = 16.0 + wrapped.iter().map(|(_, _, s)| s + 6.0).sum::<f32>();
        let (mx, my) = gfx::mouse();
        let x = if mx + 20.0 + tw > gfx::VW { mx - tw - 12.0 } else { mx + 20.0 };
        let y = (my + 10.0).min(gfx::VH - th - 10.0).max(10.0);
        gfx::rrect(x, y, tw, th, 6.0, rgba(18, 20, 36, 240));
        gfx::rect_lines(x + 2.0, y + 2.0, tw - 4.0, th - 4.0, 1.0, rgba(255, 255, 255, 50));
        let mut yy = y + 8.0;
        for (l, c, s) in wrapped {
            gfx::text(&l, x + 12.0, yy, s, c);
            yy += s + 6.0;
        }
    }
}

/// The look shown in-game: armour and boots override the outfit's top and shoes.
pub fn gear_look(base: &galebound_core::appearance::Appearance, sim: &Sim) -> galebound_core::appearance::Appearance {
    gear_look_save(base, &sim.save)
}

pub fn gear_look_save(_base: &galebound_core::appearance::Appearance, save: &galebound_core::save::CharacterSave) -> galebound_core::appearance::Appearance {
    save.worn_look()
}

/// Names, monsters and routes for the world map, built once.
pub struct WorldCache {
    names: Vec<(&'static str, &'static str)>,
    monsters: Vec<(&'static str, Vec<EnemyKind>)>,
    edges: Vec<(usize, usize)>,
}

impl WorldCache {
    fn build() -> WorldCache {
        let mut names = vec![];
        let mut monsters = vec![];
        let mut edges = vec![];
        for (i, n) in world::WORLD.iter().enumerate() {
            if let Some(m) = world::load_map(n.key) {
                names.push((n.key, m.name));
                monsters.push((n.key, world::map_monsters(&m)));
                for p in &m.portals {
                    if let Some(j) = world::WORLD.iter().position(|o| o.key == p.target_map) {
                        let e = (i.min(j), i.max(j));
                        if i != j && !edges.contains(&e) {
                            edges.push(e);
                        }
                    }
                }
            }
        }
        WorldCache { names, monsters, edges }
    }
    fn name(&self, key: &str) -> &'static str {
        self.names.iter().find(|n| n.0 == key).map(|n| n.1).unwrap_or("?")
    }
    fn monsters(&self, key: &str) -> &[EnemyKind] {
        self.monsters.iter().find(|n| n.0 == key).map(|n| n.1.as_slice()).unwrap_or(&[])
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn thousands_separators() {
        assert_eq!(super::thousands(0), "0");
        assert_eq!(super::thousands(999), "999");
        assert_eq!(super::thousands(10_000), "10,000");
        assert_eq!(super::thousands(1_234_567), "1,234,567");
    }
}
