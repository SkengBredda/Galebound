//! The Admin Panel (F9, or Esc > Admin Panel): only for players with the
//! admin role. Every button sends one of the admin commands in
//! `galebound_core::admin` (the same ones you can type in chat), and the
//! server checks the role before doing anything.

use super::*;
use galebound_core::admin;

const ROW_H: f32 = 34.0;

impl SocialUi {
    pub fn toggle_admin(&mut self, sim: &mut Sim) {
        self.admin = !self.admin;
        self.focus = None;
        if self.admin {
            sim.admin_cmd("players");
            self.admin_refresh = 5.0;
        }
    }

    fn admin_send(&mut self, sim: &mut Sim, line: String) {
        sim.admin_cmd(&line);
    }

    pub(super) fn admin_window(&mut self, sim: &mut Sim, ui: &Ui, dt: f32) {
        // keep the player list fresh while the panel is open
        self.admin_refresh -= dt;
        if self.admin_refresh <= 0.0 {
            sim.admin_cmd("players");
            self.admin_refresh = 5.0;
        }
        let time = sim.time as f64;
        let (x, y, w, h) = (40.0, 44.0, 1200.0, 636.0);
        widgets::panel(x, y, w, h);
        widgets::title("Admin Panel", x + w * 0.5, y + 14.0, 24.0);
        gfx::rrect(x + w * 0.5 - 150.0, y + 18.0, 40.0, 22.0, 5.0, rgb(236, 176, 40));
        gfx::text_centered("GM", x + w * 0.5 - 130.0, y + 21.0, 15.0, rgb(60, 30, 0));
        if widgets::button(ui, x + w - 52.0, y + 12.0, 34.0, 30.0, "X", ButtonStyle::Secondary, true) {
            self.admin = false;
            return;
        }

        // ---------------------------------------------- players
        let (lx, ly, lw) = (x + 20.0, y + 56.0, 360.0);
        gfx::text("Players online", lx, ly, 17.0, widgets::ink());
        if small(ui, lx + lw - 90.0, ly - 4.0, 90.0, "Refresh", ButtonStyle::Secondary, true) {
            sim.admin_cmd("players");
        }
        let rows = sim.admin_players().to_vec();
        if self.admin_sel.as_ref().map(|s| !rows.iter().any(|r| r.name.eq_ignore_ascii_case(s))).unwrap_or(false) {
            self.admin_sel = None;
        }
        let list_y = ly + 30.0;
        gfx::rrect(lx, list_y, lw, h - 110.0, 8.0, rgba(60, 50, 40, 30));
        for (k, r) in rows.iter().enumerate().take(15) {
            let ry = list_y + 4.0 + k as f32 * ROW_H;
            let sel = self.admin_sel.as_ref().map(|s| s.eq_ignore_ascii_case(&r.name)).unwrap_or(false);
            let hover = gfx::mouse_in(lx, ry, lw, ROW_H - 2.0);
            if sel || hover {
                gfx::rrect(lx + 2.0, ry, lw - 4.0, ROW_H - 2.0, 6.0, if sel { rgba(64, 156, 120, 90) } else { rgba(64, 156, 120, 40) });
            }
            let mut tx = lx + 10.0;
            if r.admin {
                gfx::rrect(tx, ry + 7.0, 30.0, 18.0, 4.0, rgb(236, 176, 40));
                gfx::text("GM", tx + 5.0, ry + 9.0, 13.0, rgb(60, 30, 0));
                tx += 36.0;
            }
            gfx::text(&r.name, tx, ry + 4.0, 15.0, widgets::ink());
            let sub = format!("Lv.{} {} - {}", r.level, class_name(&r.class), map_name(galebound_core::world::base_key(&r.map)));
            gfx::text(&clip_to(&sub, lw - 90.0, 12.0), lx + 10.0, ry + 20.0, 12.0, widgets::ink_soft());
            if r.muted > 0 {
                gfx::text(&format!("muted {}m", r.muted.div_ceil(60)), lx + lw - 78.0, ry + 9.0, 12.0, widgets::danger());
            }
            if hover && ui.clicked {
                self.admin_sel = Some(r.name.clone());
            }
        }
        if rows.is_empty() {
            gfx::text("Loading...", lx + 12.0, list_y + 12.0, 14.0, widgets::ink_soft());
        }

        // ---------------------------------------------- the chosen player
        let (mx, my, mw) = (x + 400.0, y + 56.0, 380.0);
        let who = self.admin_sel.clone().unwrap_or_else(|| sim.save.name.clone());
        let is_me = who.eq_ignore_ascii_case(&sim.save.name);
        gfx::text(&format!("Player: {}{}", who, if is_me { " (you)" } else { "" }), mx, my, 17.0, widgets::ink());
        gfx::text("Click a name on the left to pick someone else.", mx, my + 22.0, 12.0, widgets::ink_soft());
        let target = if is_me { "me".to_string() } else { who.clone() };
        let row = rows.iter().find(|r| r.name.eq_ignore_ascii_case(&who)).cloned();
        let bw = (mw - 16.0) / 3.0;
        let by = my + 46.0;
        let mut send: Option<String> = None;
        let b = |ui: &Ui, i: usize, j: usize, label: &str, style: ButtonStyle, on: bool| widgets::button(ui, mx + j as f32 * (bw + 8.0), by + i as f32 * 40.0, bw, 32.0, label, style, on);
        if b(ui, 0, 0, "Go to", ButtonStyle::Primary, !is_me) {
            send = Some(format!("goto {}", target));
        }
        if b(ui, 0, 1, "Summon", ButtonStyle::Primary, !is_me) {
            send = Some(format!("summon {}", target));
        }
        if b(ui, 0, 2, "Heal", ButtonStyle::Primary, true) {
            send = Some(format!("heal {}", target));
        }
        let muted = row.as_ref().map(|r| r.muted > 0).unwrap_or(false);
        if b(ui, 1, 0, if muted { "Unmute" } else { "Mute 10 min" }, ButtonStyle::Secondary, !is_me) {
            send = Some(format!("{} {}", if muted { "unmute" } else { "mute" }, target));
        }
        if b(ui, 1, 1, "Kick", ButtonStyle::Secondary, !is_me) {
            send = Some(format!("kick {}", target));
        }
        let their_admin = row.as_ref().map(|r| r.admin).unwrap_or(false);
        if b(ui, 1, 2, if their_admin { "Remove admin" } else { "Make admin" }, ButtonStyle::Secondary, !is_me) {
            send = Some(format!("{} {}", if their_admin { "unadmin" } else { "admin" }, target));
        }
        // amounts: gold, level, item count
        let fy = by + 96.0;
        gfx::text("Amount", mx, fy + 8.0, 15.0, widgets::ink());
        self.field(ui, Field::AdminAmount, mx + 80.0, fy, mw - 80.0, time);
        let amount: u32 = self.admin_amount.text.parse().unwrap_or(0);
        let bw2 = (mw - 16.0) / 3.0;
        if widgets::button(ui, mx, fy + 42.0, bw2, 32.0, "Give gold", ButtonStyle::Secondary, amount > 0) {
            send = Some(format!("gold {} {}", target, amount));
        }
        if widgets::button(ui, mx + bw2 + 8.0, fy + 42.0, bw2, 32.0, "Set level", ButtonStyle::Secondary, (1..=admin::MAX_LEVEL).contains(&amount)) {
            send = Some(format!("level {} {}", target, amount));
        }
        if widgets::button(ui, mx + 2.0 * (bw2 + 8.0), fy + 42.0, bw2, 32.0, "Full heal", ButtonStyle::Secondary, true) {
            send = Some(format!("heal {}", target));
        }
        // items, with suggestions as you type
        let iy = fy + 96.0;
        gfx::text("Item", mx, iy + 8.0, 15.0, widgets::ink());
        self.field(ui, Field::AdminItem, mx + 80.0, iy, mw - 80.0, time);
        let q = self.admin_item.text.trim().to_lowercase();
        let exact = items::item(&q).map(|d| d.id);
        let count = amount.clamp(1, 999);
        if widgets::button(ui, mx, iy + 42.0, mw, 32.0, &format!("Give item x{}", count), ButtonStyle::Primary, exact.is_some()) {
            send = Some(format!("give {} {} {}", target, q, count));
        }
        if q.len() >= 2 {
            let matches: Vec<&items::ItemDef> = items::ITEMS.iter().filter(|d| d.id.contains(&q) || d.name.to_lowercase().contains(&q)).take(5).collect();
            for (k, d) in matches.iter().enumerate() {
                let sy = iy + 82.0 + k as f32 * 28.0;
                let hover = gfx::mouse_in(mx, sy, mw, 26.0);
                if hover {
                    gfx::rrect(mx, sy, mw, 26.0, 5.0, rgba(64, 156, 120, 40));
                }
                gfx::tex(&item_tex(d), mx + 2.0, sy - 3.0, 0.75, false, WHITE);
                gfx::text(&format!("{}  ({})", d.name, d.id), mx + 34.0, sy + 5.0, 13.0, px_color(icons::tier_color(d.tier.max(1))));
                if hover && ui.clicked {
                    self.admin_item.text = d.id.to_string();
                }
            }
        } else {
            gfx::text_wrapped("Type part of a name (e.g. 'galeheart') to see matching items. 'Amount' sets how many.", mx, iy + 84.0, mw, 12.0, widgets::ink_soft());
        }

        // ---------------------------------------------- the world
        let (rx, ry, rw) = (x + 800.0, y + 56.0, 380.0);
        gfx::text("World", rx, ry, 17.0, widgets::ink());
        let god = sim.god;
        if widgets::button(ui, rx, ry + 26.0, (rw - 8.0) * 0.5, 32.0, if god { "God mode: ON" } else { "God mode: off" }, if god { ButtonStyle::Primary } else { ButtonStyle::Secondary }, true) {
            send = Some("god".into());
        }
        if widgets::button(ui, rx + (rw + 8.0) * 0.5, ry + 26.0, (rw - 8.0) * 0.5, 32.0, "Clear monsters", ButtonStyle::Secondary, true) {
            send = Some("clear".into());
        }
        let ay = ry + 72.0;
        gfx::text("Announce", rx, ay, 14.0, widgets::ink());
        let enter = self.field(ui, Field::AdminAnnounce, rx, ay + 20.0, rw - 90.0, time);
        let has = !self.admin_announce.text.trim().is_empty();
        if (widgets::button(ui, rx + rw - 82.0, ay + 20.0, 82.0, 32.0, "Send", ButtonStyle::Primary, has) || enter) && has {
            send = Some(format!("announce {}", self.take(Field::AdminAnnounce)));
        }
        let sy = ay + 66.0;
        gfx::text("Spawn monsters next to you", rx, sy, 14.0, widgets::ink());
        self.field(ui, Field::AdminMonster, rx, sy + 20.0, rw - 90.0, time);
        let kind = admin::monster(&self.admin_monster.text);
        let n = amount.clamp(1, 10);
        if widgets::button(ui, rx + rw - 82.0, sy + 20.0, 82.0, 32.0, &format!("x{}", n), ButtonStyle::Primary, kind.is_some()) {
            if let Some(k) = kind {
                send = Some(format!("spawn {} {}", k.key(), n));
            }
        }
        let mq = self.admin_monster.text.trim().to_lowercase();
        if mq.len() >= 2 && kind.is_none() {
            let found: Vec<&'static str> = galebound_core::enemies::ALL_KINDS.iter().map(|k| k.def().name).filter(|nm| nm.to_lowercase().contains(&mq)).take(4).collect();
            for (k, nm) in found.iter().enumerate() {
                let yy = sy + 56.0 + k as f32 * 22.0;
                let hover = gfx::mouse_in(rx, yy, rw, 20.0);
                gfx::text(nm, rx + 6.0, yy + 3.0, 13.0, if hover { widgets::accent() } else { widgets::ink_soft() });
                if hover && ui.clicked {
                    self.admin_monster.text = nm.to_string();
                }
            }
        }
        // any command at all
        let cy = sy + 150.0;
        gfx::text("Command", rx, cy, 14.0, widgets::ink());
        let enter = self.field(ui, Field::AdminCmd, rx, cy + 20.0, rw - 90.0, time);
        let has = !self.admin_line.text.trim().is_empty();
        if (widgets::button(ui, rx + rw - 82.0, cy + 20.0, 82.0, 32.0, "Run", ButtonStyle::Primary, has) || enter) && has {
            send = Some(self.take(Field::AdminCmd));
        }
        // results
        let gy = cy + 66.0;
        gfx::rrect(rx, gy, rw, y + h - gy - 20.0, 8.0, rgba(24, 22, 40, 220));
        let lines: Vec<&String> = sim.admin_log.iter().rev().take(10).collect();
        if lines.is_empty() {
            gfx::text("Results show here. Type 'help' above for every command.", rx + 10.0, gy + 10.0, 12.0, rgb(190, 200, 225));
        }
        for (k, l) in lines.iter().enumerate() {
            gfx::text(&clip_to(l, rw - 20.0, 12.0), rx + 10.0, gy + 8.0 + k as f32 * 18.0, 12.0, if k == 0 { rgb(255, 236, 170) } else { rgb(190, 200, 225) });
        }
        if let Some(line) = send {
            self.admin_send(sim, line);
        }
    }
}

/// Shorten text to fit a width, with an ellipsis.
fn clip_to(s: &str, w: f32, size: f32) -> String {
    if gfx::text_w(s, size) <= w {
        return s.to_string();
    }
    let mut out = String::new();
    for c in s.chars() {
        if gfx::text_w(&format!("{}{}...", out, c), size) > w {
            break;
        }
        out.push(c);
    }
    format!("{}...", out)
}
