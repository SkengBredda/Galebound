//! 0.13.0 windows: crafting, house furnishing, house storage and the
//! quest log (tracking, hide completed, available quests).

use super::*;
use galebound_core::crafting;
use galebound_core::housing;
use galebound_core::quests::{self, QuestDef};
use galebound_core::save::QuestState;

const ROWS: usize = 8;

impl Windows {
    // ------------------------------------------------------------------ crafting

    pub(super) fn craft_window(&mut self, sim: &mut Sim, ui: &Ui) {
        let Some(station) = sim.craft_open else { return };
        let crafts = station.crafts();
        self.craft_tab = self.craft_tab.min(crafts.len() - 1);
        let craft = crafts[self.craft_tab];
        let (x, y, w, h) = (24.0, 56.0, 820.0, 600.0);
        widgets::panel(x, y, w, h);
        self.hit((x, y, w, h), Target::Window);
        let title = match station {
            galebound_core::crafting::Station::Kitchen => "Kitchen",
            _ => "Crafting",
        };
        widgets::title(title, x + w * 0.5, y + 14.0, 22.0);
        if widgets::button(ui, x + w - 52.0, y + 12.0, 34.0, 30.0, "X", ButtonStyle::Secondary, true) {
            sim.craft_open = None;
            return;
        }
        // tabs: one per craft this station offers, with its level bar
        for (i, c) in crafts.iter().enumerate() {
            let tx = x + 24.0 + i as f32 * 250.0;
            let sk = sim.save.crafts[c.index()];
            let sel = i == self.craft_tab;
            if widgets::button(ui, tx, y + 48.0, 236.0, 30.0, &format!("{}  Lv.{}", c.name(), sk.level), if sel { ButtonStyle::Primary } else { ButtonStyle::Secondary }, true) && !sel {
                self.craft_tab = i;
                self.craft_page = 0;
                self.craft_sel = None;
            }
        }
        let sk = sim.save.crafts[craft.index()];
        let (bx, by, bw) = (x + 24.0, y + 86.0, w - 48.0);
        gfx::rrect(bx, by, bw, 12.0, 4.0, rgb(214, 196, 160));
        let need = crafting::craft_exp_to_next(sk.level);
        let frac = if sk.level >= crafting::MAX_CRAFT_LEVEL { 1.0 } else { sk.exp as f32 / need.max(1) as f32 };
        gfx::rrect(bx + 1.0, by + 1.0, (bw - 2.0) * frac.clamp(0.0, 1.0), 10.0, 3.0, rgb(214, 150, 50));
        let lbl = if sk.level >= crafting::MAX_CRAFT_LEVEL {
            format!("{} - mastered!", craft.name())
        } else {
            format!("{} EXP {}/{}", craft.name(), sk.exp, need)
        };
        gfx::text_centered(&lbl, bx + bw * 0.5, by + 16.0, 12.0, widgets::ink_soft());

        // recipe list
        let list = crafting::recipes_for(craft);
        let pages = list.len().div_ceil(ROWS).max(1);
        self.craft_page = self.craft_page.min(pages - 1);
        if self.craft_sel.map(|s| !list.iter().any(|r| r.output == s)).unwrap_or(true) {
            self.craft_sel = list.iter().rev().find(|r| r.level <= sk.level).or(list.first()).map(|r| r.output);
        }
        let (lx, ly, lw) = (x + 24.0, y + 126.0, 380.0);
        let hover = self.hover();
        for (i, r) in list.iter().skip(self.craft_page * ROWS).take(ROWS).enumerate() {
            let Some(def) = items::item(r.output) else { continue };
            let ry = ly + i as f32 * 48.0;
            let sel = self.craft_sel == Some(r.output);
            let hv = hover == Some(Target::Recipe(r.output));
            if sel || hv {
                gfx::rrect(lx - 4.0, ry - 3.0, lw, 44.0, 6.0, if sel { rgba(64, 156, 120, 60) } else { rgba(64, 156, 120, 30) });
            }
            slot_bg(lx, ry, 38.0, hv, Some(px_color(icons::tier_color(def.tier))));
            gfx::tex(&item_tex(def), lx + 1.0, ry + 1.0, 1.0, false, if sk.level >= r.level { WHITE } else { Color::new(1.0, 1.0, 1.0, 0.45) });
            let name = if r.count > 1 { format!("{} x{}", def.name, r.count) } else { def.name.to_string() };
            gfx::text(&clip(&name, lw - 130.0, 15.0), lx + 48.0, ry + 2.0, 15.0, if sk.level >= r.level { widgets::ink() } else { rgb(150, 140, 130) });
            let can = crafting::max_crafts(r, |id| sim.save.inventory.count(id), sim.save.gold);
            let sub = if sk.level < r.level {
                (format!("Needs Lv.{}", r.level), widgets::danger())
            } else if can > 0 {
                (format!("Can make {}", can), rgb(40, 140, 80))
            } else {
                ("Missing materials".to_string(), widgets::ink_soft())
            };
            gfx::text(&sub.0, lx + 48.0, ry + 22.0, 12.0, sub.1);
            let lv = format!("Lv.{}", r.level);
            gfx::text(&lv, lx + lw - 16.0 - gfx::text_w(&lv, 13.0), ry + 12.0, 13.0, widgets::ink_soft());
            self.hit((lx - 4.0, ry - 3.0, lw, 44.0), Target::Recipe(r.output));
        }
        if pages > 1 {
            let py = y + h - 54.0;
            if widgets::button(ui, lx, py, 80.0, 30.0, "< Prev", ButtonStyle::Secondary, self.craft_page > 0) {
                self.craft_page -= 1;
            }
            gfx::text_centered(&format!("{}/{}", self.craft_page + 1, pages), lx + lw * 0.5, py + 8.0, 13.0, widgets::ink_soft());
            if widgets::button(ui, lx + lw - 88.0, py, 80.0, 30.0, "Next >", ButtonStyle::Secondary, self.craft_page + 1 < pages) {
                self.craft_page += 1;
            }
        }

        // details of the selected recipe
        let Some(r) = self.craft_sel.and_then(crafting::recipe) else { return };
        let Some(def) = items::item(r.output) else { return };
        let (dx, dy, dw) = (x + 430.0, y + 126.0, w - 454.0);
        gfx::rrect(dx, dy, dw, h - 150.0, 8.0, rgb(240, 228, 200));
        slot_bg(dx + 14.0, dy + 14.0, SLOT, false, Some(px_color(icons::tier_color(def.tier))));
        gfx::tex(&item_tex(def), dx + 19.0, dy + 19.0, 1.0, false, WHITE);
        gfx::text(&clip(def.name, dw - 90.0, 17.0), dx + 72.0, dy + 16.0, 17.0, px_color(icons::tier_color(def.tier.max(1))));
        gfx::text(&format!("{} - {}", def.tier_name(), r.craft.name()), dx + 72.0, dy + 38.0, 12.0, widgets::ink_soft());
        let mut yy = dy + 72.0;
        for l in def.stat_lines().iter().take(5) {
            gfx::text(l, dx + 16.0, yy, 13.0, rgb(40, 120, 70));
            yy += 18.0;
        }
        if def.level > 1 && def.slot().is_some() {
            gfx::text(&format!("Equip at Lv.{}", def.level), dx + 16.0, yy, 13.0, if sim.save.level >= def.level { widgets::ink_soft() } else { widgets::danger() });
            yy += 18.0;
        }
        yy += 8.0;
        gfx::text("Materials", dx + 16.0, yy, 14.0, widgets::ink());
        yy += 22.0;
        for &(id, n) in r.inputs {
            let Some(m) = items::item(id) else { continue };
            let have = sim.save.inventory.count(id);
            slot_bg(dx + 16.0, yy - 2.0, 30.0, false, None);
            gfx::tex(&item_tex(m), dx + 12.0, yy - 6.0, 0.9, false, WHITE);
            gfx::text(&clip(m.name, dw - 140.0, 14.0), dx + 54.0, yy + 4.0, 14.0, widgets::ink());
            let t = format!("{}/{}", have, n);
            gfx::text(&t, dx + dw - 20.0 - gfx::text_w(&t, 14.0), yy + 4.0, 14.0, if have >= n { rgb(40, 140, 80) } else { widgets::danger() });
            self.hit((dx + 16.0, yy - 2.0, dw - 32.0, 30.0), Target::Shop(m.id));
            yy += 34.0;
        }
        yy += 4.0;
        let exp = crafting::exp_for(r, sk.level);
        gfx::text(&format!("Fee: {}g each     +{} {} EXP", r.fee, exp, craft.name()), dx + 16.0, yy, 13.0, widgets::ink_soft());
        let can = crafting::max_crafts(r, |id| sim.save.inventory.count(id), sim.save.gold);
        let ok = sk.level >= r.level && can > 0;
        let byy = y + h - 64.0;
        let mut made: Option<u32> = None;
        if widgets::button(ui, dx + 14.0, byy, 100.0, 36.0, "Craft", ButtonStyle::Primary, ok) {
            made = Some(1);
        }
        if widgets::button(ui, dx + 122.0, byy, 100.0, 36.0, "Craft x5", ButtonStyle::Secondary, ok && can >= 5) {
            made = Some(5);
        }
        let all = format!("All ({})", can);
        if widgets::button(ui, dx + 230.0, byy, dw - 244.0, 36.0, &all, ButtonStyle::Secondary, ok && can > 1) {
            made = Some(can);
        }
        if !ok {
            if let Some(p) = sim.craft_problem(r.output, 1) {
                gfx::text_centered(&p, dx + dw * 0.5, byy - 22.0, 13.0, widgets::danger());
            }
        }
        if let Some(n) = made {
            match sim.craft(r.output, n) {
                Ok(c) => self.say(format!("Crafted {}x {}", c.count, def.name), false),
                Err(e) => self.say(e, true),
            }
        }
    }

    // ------------------------------------------------------------------ house

    pub(super) fn house_window(&mut self, sim: &mut Sim, ui: &Ui) {
        let (x, y, w, h) = (140.0, 70.0, 1000.0, 470.0);
        widgets::panel(x, y, w, h);
        self.hit((x, y, w, h), Target::Window);
        widgets::title("Arrange Furniture", x + w * 0.5, y + 14.0, 22.0);
        if widgets::button(ui, x + w - 52.0, y + 12.0, 34.0, 30.0, "X", ButtonStyle::Secondary, true) {
            sim.house_open = false;
            return;
        }
        gfx::text_centered(
            "Pick a spot, then a piece of furniture from your bag. Beds, chests, stoves and workbenches do something when you stand by them.",
            x + w * 0.5,
            y + 46.0,
            13.0,
            widgets::ink_soft(),
        );
        let n = housing::SPOTS.len();
        self.house_sel = self.house_sel.min(n - 1);
        let sw = 110.0;
        for i in 0..n {
            let sx = x + 30.0 + i as f32 * (sw + 6.0);
            let sy = y + 80.0;
            let sel = i == self.house_sel;
            gfx::rrect(sx, sy, sw, 120.0, 8.0, if sel { rgb(120, 190, 150) } else { rgb(214, 196, 160) });
            gfx::text_centered(&housing::spot_name(i), sx + sw * 0.5, sy + 6.0, 13.0, widgets::ink());
            match housing::placed(&sim.save.house, i).and_then(items::item) {
                Some(d) => {
                    gfx::tex(&item_tex(d), sx + sw * 0.5 - 18.0, sy + 30.0, 1.0, false, WHITE);
                    gfx::text_centered(&clip(d.name, sw - 8.0, 12.0), sx + sw * 0.5, sy + 74.0, 12.0, widgets::ink());
                }
                None => gfx::text_centered("(empty)", sx + sw * 0.5, sy + 44.0, 13.0, widgets::ink_soft()),
            }
            if widgets::button(ui, sx + 8.0, sy + 92.0, sw - 16.0, 24.0, if sel { "Selected" } else { "Select" }, ButtonStyle::Secondary, !sel) {
                self.house_sel = i;
            }
        }
        let sel = self.house_sel;
        let has = housing::placed(&sim.save.house, sel).is_some();
        if widgets::button(ui, x + 30.0, y + 214.0, 230.0, 32.0, "Put back in bag", ButtonStyle::Secondary, has) {
            if let Err(e) = sim.take_furniture(sel) {
                self.say(e, true);
            }
        }
        gfx::text(&format!("Furniture in your bag (places it in {}):", housing::spot_name(sel)), x + 30.0, y + 262.0, 14.0, widgets::ink());
        // furniture you carry
        let mut owned: Vec<(&'static str, u32)> = vec![];
        for st in &sim.save.inventory.slots {
            if let Some(d) = items::item(&st.id) {
                if housing::furniture_of(d.id).is_some() {
                    match owned.iter_mut().find(|o| o.0 == d.id) {
                        Some(o) => o.1 += st.count,
                        None => owned.push((d.id, st.count)),
                    }
                }
            }
        }
        if owned.is_empty() {
            gfx::text_wrapped(
                "You have no furniture with you. Rosalind in Breezehollow sells furniture, and crafters can make beds, tables, workbenches and more.",
                x + 30.0,
                y + 292.0,
                w - 60.0,
                14.0,
                widgets::ink_soft(),
            );
        }
        let mut place: Option<&'static str> = None;
        for (k, (id, cnt)) in owned.iter().enumerate() {
            let Some(d) = items::item(id) else { continue };
            let fx = x + 30.0 + (k % 4) as f32 * 236.0;
            let fy = y + 290.0 + (k / 4) as f32 * 52.0;
            if fy > y + h - 50.0 {
                break;
            }
            slot_bg(fx, fy, 40.0, false, Some(px_color(icons::tier_color(d.tier))));
            gfx::tex(&item_tex(d), fx + 2.0, fy + 2.0, 1.0, false, WHITE);
            gfx::text(&clip(&format!("{} x{}", d.name, cnt), 110.0, 13.0), fx + 48.0, fy + 2.0, 13.0, widgets::ink());
            if widgets::button(ui, fx + 48.0, fy + 18.0, 120.0, 22.0, "Place", ButtonStyle::Primary, true) {
                place = Some(d.id);
            }
            self.hit((fx, fy, 40.0, 40.0), Target::Shop(d.id));
        }
        if let Some(id) = place {
            match sim.place_furniture(sel, id) {
                Ok(()) => self.say("Placed!", false),
                Err(e) => self.say(e, true),
            }
        }
    }

    // ------------------------------------------------------------------ storage

    pub(super) fn storage_window(&mut self, sim: &mut Sim, ui: &Ui) {
        let cols = 8;
        let (x, y, w, h) = (24.0, 56.0, 30.0 + cols as f32 * 52.0 + 24.0, 470.0);
        widgets::panel(x, y, w, h);
        self.hit((x, y, w, h), Target::Window);
        widgets::title("Storage Chest", x + w * 0.5, y + 14.0, 22.0);
        if widgets::button(ui, x + w - 52.0, y + 12.0, 34.0, 30.0, "X", ButtonStyle::Secondary, true) {
            sim.storage_open = false;
            self.inventory = false;
            return;
        }
        gfx::text(&format!("{}/{} slots", sim.save.storage.len(), housing::STORAGE_SLOTS), x + 30.0, y + 50.0, 14.0, widgets::ink_soft());
        let hover = self.hover();
        for i in 0..housing::STORAGE_SLOTS {
            let sx = x + 30.0 + (i % cols) as f32 * 52.0;
            let sy = y + 76.0 + (i / cols) as f32 * 52.0;
            let st = sim.save.storage.get(i);
            let def = st.and_then(|s| items::item(&s.id));
            slot_bg(sx, sy, SLOT, hover == Some(Target::Store(i)), def.map(|d| px_color(icons::tier_color(d.tier))));
            if let (Some(st), Some(d)) = (st, def) {
                gfx::tex(&item_tex(d), sx + 5.0, sy + 5.0, 1.0, false, WHITE);
                if st.count > 1 {
                    let t = format!("{}", st.count);
                    gfx::text_outlined(&t, sx + SLOT - 5.0 - gfx::text_w(&t, 13.0), sy + SLOT - 16.0, 13.0, WHITE);
                }
            }
            self.hit((sx, sy, SLOT, SLOT), Target::Store(i));
        }
        gfx::text("Click: take out (Shift: just one). Click items in your bag to store them.", x + 30.0, y + h - 34.0, 12.0, widgets::ink_soft());
    }

    // ------------------------------------------------------------------ quest log

    pub(super) fn quest_log_window(&mut self, sim: &mut Sim, ui: &Ui) {
        let layout = sim.settings.layout;
        let (x, y, w, h) = (230.0, 56.0, 820.0, 600.0);
        widgets::panel(x, y, w, h);
        self.hit((x, y, w, h), Target::Window);
        widgets::title("Quest Log", x + w * 0.5, y + 14.0, 24.0);
        if widgets::button(ui, x + w - 52.0, y + 12.0, 34.0, 30.0, "X", ButtonStyle::Secondary, true) {
            sim.quest_log_open = false;
            return;
        }
        let available = quests::available_quests(&sim.save);
        let mine = sim.save.quests.iter().filter(|q| q.state != QuestState::Done).count();
        let tabs = [format!("My Quests ({})", mine), format!("Available ({})", available.iter().filter(|a| a.need_level == 0).count())];
        for (i, t) in tabs.iter().enumerate() {
            let sel = self.quest_tab == i;
            if widgets::button(ui, x + 24.0 + i as f32 * 214.0, y + 50.0, 204.0, 32.0, t, if sel { ButtonStyle::Primary } else { ButtonStyle::Secondary }, true) && !sel {
                self.quest_tab = i;
                self.quest_page = 0;
            }
        }
        let list_y = y + 100.0;
        let row_h = 74.0;
        let per_page = 6;
        if self.quest_tab == 0 {
            if widgets::checkbox(ui, x + w - 230.0, y + 56.0, "Hide completed", sim.save.hide_done_quests) {
                sim.save.hide_done_quests = !sim.save.hide_done_quests;
                sim.wants_save = true;
                self.quest_page = 0;
            }
            // newest first, finished ones last
            let mut rows: Vec<_> = sim.save.quests.iter().rev().filter(|q| !(sim.save.hide_done_quests && q.state == QuestState::Done)).cloned().collect();
            rows.sort_by_key(|q| match q.state {
                QuestState::ReadyToTurnIn => 0,
                QuestState::Active => 1,
                QuestState::Done => 2,
            });
            if rows.is_empty() {
                gfx::text_centered("No quests yet. Look for NPCs with a '!' above their heads, or check the Available tab.", x + w * 0.5, list_y + 20.0, 15.0, widgets::ink_soft());
            }
            let pages = rows.len().div_ceil(per_page).max(1);
            self.quest_page = self.quest_page.min(pages - 1);
            let mut toggle: Option<(String, bool)> = None;
            for (i, q) in rows.iter().skip(self.quest_page * per_page).take(per_page).enumerate() {
                let Some(def) = quests::quest_def(&q.id) else { continue };
                let ry = list_y + i as f32 * row_h;
                gfx::rrect(x + 20.0, ry, w - 40.0, row_h - 8.0, 8.0, rgb(240, 228, 200));
                let (status, c) = match q.state {
                    QuestState::Active => ("In progress", rgb(200, 140, 40)),
                    QuestState::ReadyToTurnIn => ("Ready to turn in", widgets::accent()),
                    QuestState::Done => ("Completed", rgb(150, 140, 130)),
                };
                gfx::text(def.name, x + 36.0, ry + 8.0, 17.0, if q.state == QuestState::Done { rgb(150, 140, 130) } else { widgets::ink() });
                gfx::text(status, x + 36.0 + gfx::text_w(def.name, 17.0) + 14.0, ry + 11.0, 13.0, c);
                if q.state != QuestState::Done {
                    let txt = crate::hud::fill_keys(&quests::progress_text(q), layout);
                    gfx::text(&clip(&txt, w - 240.0, 13.0), x + 40.0, ry + 34.0, 13.0, widgets::ink_soft());
                    let tracked = sim.save.tracked(&q.id);
                    if widgets::checkbox(ui, x + w - 170.0, ry + 20.0, "Track", tracked) {
                        toggle = Some((q.id.clone(), !tracked));
                    }
                } else {
                    gfx::text(&format!("From {} - {}", quests::npc_display_name(def.giver), quests::npc_map_name(def.giver)), x + 40.0, ry + 34.0, 13.0, rgb(150, 140, 130));
                }
            }
            if let Some((id, on)) = toggle {
                sim.set_tracked(&id, on);
            }
            self.quest_pages(ui, x, y, w, h, pages);
            gfx::text("Tracked quests show on screen under the minimap.", x + 24.0, y + h - 30.0, 12.0, widgets::ink_soft());
        } else {
            if available.is_empty() {
                gfx::text_centered("Nothing new right now. Level up and explore - more quests will appear.", x + w * 0.5, list_y + 20.0, 15.0, widgets::ink_soft());
            }
            let pages = available.len().div_ceil(per_page).max(1);
            self.quest_page = self.quest_page.min(pages - 1);
            for (i, a) in available.iter().skip(self.quest_page * per_page).take(per_page).enumerate() {
                let ry = list_y + i as f32 * row_h;
                let ready = a.need_level == 0;
                gfx::rrect(x + 20.0, ry, w - 40.0, row_h - 8.0, 8.0, if ready { rgb(232, 240, 214) } else { rgb(228, 220, 206) });
                gfx::text(a.def.name, x + 36.0, ry + 8.0, 17.0, if ready { widgets::ink() } else { rgb(150, 140, 130) });
                let tag = if ready { "Available now".to_string() } else { format!("From level {}", a.need_level) };
                gfx::text(&tag, x + 36.0 + gfx::text_w(a.def.name, 17.0) + 14.0, ry + 11.0, 13.0, if ready { widgets::accent() } else { widgets::danger() });
                gfx::text(&format!("Talk to {} in {}", quests::npc_display_name(a.def.giver), a.map), x + 40.0, ry + 34.0, 13.0, widgets::ink_soft());
                let r = reward_summary(a.def);
                gfx::text(&clip(&r, 300.0, 13.0), x + w - 330.0, ry + 34.0, 13.0, rgb(140, 80, 200));
            }
            self.quest_pages(ui, x, y, w, h, pages);
            gfx::text("Quests marked '!' above an NPC's head are ready to take.", x + 24.0, y + h - 30.0, 12.0, widgets::ink_soft());
        }
        gfx::text(&format!("[{}] close", layout.key_label(galebound_core::input::Action::QuestLog)), x + w - 120.0, y + h - 30.0, 12.0, widgets::ink_soft());
    }

    fn quest_pages(&mut self, ui: &Ui, x: f32, y: f32, w: f32, h: f32, pages: usize) {
        if pages <= 1 {
            return;
        }
        let py = y + h - 70.0;
        if widgets::button(ui, x + w * 0.5 - 150.0, py, 90.0, 30.0, "< Prev", ButtonStyle::Secondary, self.quest_page > 0) {
            self.quest_page -= 1;
        }
        gfx::text_centered(&format!("Page {}/{}", self.quest_page + 1, pages), x + w * 0.5, py + 8.0, 13.0, widgets::ink_soft());
        if widgets::button(ui, x + w * 0.5 + 60.0, py, 90.0, 30.0, "Next >", ButtonStyle::Secondary, self.quest_page + 1 < pages) {
            self.quest_page += 1;
        }
    }
}

fn reward_summary(def: &QuestDef) -> String {
    quests::reward_list(def).into_iter().map(|(_, l)| l).collect::<Vec<_>>().join(", ")
}

