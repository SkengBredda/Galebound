//! In-game HUD: status bars, quest tracker, toasts, boss bar, signs, dialog
//! box, quest log and on-screen control hints.

use crate::gfx::{self, alpha, rgb, rgba, shade};
use crate::widgets;
use galebound_core::input::{Action, Layout};
use galebound_core::quests;
use galebound_core::save::QuestState;
use galebound_core::sim::{exp_to_next, PState, Sim};
use macroquad::prelude::Color;

/// Replace {Action} placeholders with the current layout's key names.
pub fn fill_keys(text: &str, layout: Layout) -> String {
    let mut out = text.to_string();
    for (tag, a) in [
        ("{Left}", Action::Left),
        ("{Right}", Action::Right),
        ("{Up}", Action::Up),
        ("{Down}", Action::Down),
        ("{Jump}", Action::Jump),
        ("{Light}", Action::Light),
        ("{Heavy}", Action::Heavy),
        ("{Dodge}", Action::Dodge),
        ("{Interact}", Action::Interact),
        ("{Potion}", Action::Potion),
        ("{Inventory}", Action::Inventory),
        ("{Character}", Action::Character),
        ("{Abilities}", Action::Abilities),
        ("{QuestLog}", Action::QuestLog),
        ("{WorldMap}", Action::WorldMap),
        ("{Pickup}", Action::Pickup),
        ("{Minimap}", Action::Minimap),
        ("{Pets}", Action::Pets),
        ("{Chat}", Action::Chat),
    ] {
        if out.contains(tag) {
            out = out.replace(tag, &format!("[{}]", layout.key_label(a)));
        }
    }
    out
}

fn bar(x: f32, y: f32, w: f32, h: f32, frac: f32, c: Color, label: &str) {
    gfx::rrect(x, y, w, h, h * 0.5, rgba(10, 12, 24, 220));
    let fw = (w - 4.0) * frac.clamp(0.0, 1.0);
    if fw > 1.0 {
        gfx::rrect(x + 2.0, y + 2.0, fw, h - 4.0, (h - 4.0) * 0.5, c);
        gfx::rrect(x + 4.0, y + 3.0, (fw - 4.0).max(0.0), (h - 4.0) * 0.35, 3.0, alpha(rgb(255, 255, 255), 0.3));
    }
    gfx::text_centered_outlined(label, x + w * 0.5, y + h * 0.5 - 7.0, 14.0, rgb(255, 255, 255));
}

pub fn draw(sim: &Sim, show_controls: bool, time: f32) {
    gfx::clear_cam();
    let p = &sim.player;
    let layout = sim.settings.layout;

    // ---------------------------------------------- status (top-left)
    widgets::hud_panel(14.0, 14.0, 330.0, 124.0);
    let info = &p.info;
    gfx::text_outlined(&sim.save.name, 28.0, 24.0, 16.0, rgb(255, 255, 255));
    let lv = format!("Lv.{} {}", sim.save.level, info.name);
    gfx::text_outlined(&lv, 330.0 - gfx::text_w(&lv, 16.0), 24.0, 16.0, rgb(255, 220, 120));
    bar(26.0, 48.0, 306.0, 22.0, p.hp / p.max_hp, rgb(226, 72, 80), &format!("HP {}/{}", p.hp.ceil() as i32, p.max_hp as i32));
    bar(26.0, 74.0, 306.0, 22.0, p.mp / p.max_mp, rgb(70, 130, 236), &format!("MP {}/{}", p.mp.floor() as i32, p.max_mp as i32));
    // potions & gold
    let inv = &sim.save.inventory;
    let pots = inv.count("hp_potion") + inv.count("hp_potion_big") + inv.count("trainee_hp_potion");
    gfx::circle(36.0, 114.0, 8.0, rgb(230, 70, 90));
    gfx::rect(32.0, 102.0, 8.0, 6.0, rgb(200, 200, 210));
    gfx::text_outlined(&format!("x{}  [{}]", pots, layout.key_label(Action::Potion)), 52.0, 106.0, 16.0, rgb(255, 255, 255));
    gfx::circle(198.0, 114.0, 8.0, rgb(240, 196, 70));
    gfx::text_outlined(&format!("{} gold", sim.save.gold), 214.0, 106.0, 16.0, rgb(255, 236, 160));
    // unspent ability points and active buffs, under the status panel
    let mut bx = 16.0;
    if sim.save.ap > 0 {
        let pulse = ((time * 4.0).sin() * 0.5 + 0.5) * 0.4 + 0.6;
        let t = format!("+{} AP  [{}]", sim.save.ap, layout.key_label(Action::Character));
        let w = gfx::text_w(&t, 14.0) + 20.0;
        gfx::rrect(bx, 146.0, w, 26.0, 6.0, alpha(rgb(220, 140, 40), pulse));
        gfx::text_outlined(&t, bx + 10.0, 151.0, 14.0, rgb(255, 255, 255));
        bx += w + 8.0;
    }
    for b in &p.buffs {
        let c = crate::icons::fx_color(b.fx);
        let col = rgb(c[0], c[1], c[2]);
        gfx::rrect(bx, 146.0, 30.0, 30.0, 6.0, rgba(20, 24, 40, 200));
        gfx::rrect(bx + 3.0, 149.0, 24.0, 24.0 * (b.left / b.total).clamp(0.0, 1.0), 4.0, shade(col, 0.8));
        gfx::text_centered_outlined(&b.name[..1], bx + 15.0, 152.0, 16.0, rgb(255, 255, 255));
        gfx::text_centered_outlined(&format!("{}", b.left.ceil() as i32), bx + 15.0, 178.0, 12.0, rgb(230, 230, 240));
        bx += 36.0;
    }

    // ---------------------------------------------- pet (under the status panel)
    if let (Some(pet), Some(_)) = (sim.active_pet(), &sim.pet) {
        let (px, py) = (14.0, 184.0);
        widgets::hud_panel(px, py, 250.0, 46.0);
        let tex = crate::sprites::get(&format!("pet_{}_0", pet.species.key()), || crate::petart::pet(pet.species, 0));
        gfx::tex(&tex, px + 6.0, py + 2.0, 1.0, false, Color::new(1.0, 1.0, 1.0, 1.0));
        gfx::text_outlined(&format!("{} Lv.{}", pet.name, pet.level), px + 56.0, py + 5.0, 14.0, rgb(190, 230, 255));
        let f = pet.hunger / galebound_core::pets::HUNGER_MAX;
        let c = if f > 0.5 { rgb(110, 200, 110) } else if f > 0.2 { rgb(230, 180, 60) } else { rgb(230, 80, 70) };
        gfx::rrect(px + 56.0, py + 28.0, 150.0, 10.0, 4.0, rgba(10, 12, 24, 220));
        gfx::rrect(px + 57.0, py + 29.0, 148.0 * f.clamp(0.0, 1.0), 8.0, 3.0, c);
        let k = format!("[{}]", layout.key_label(Action::Pets));
        gfx::text_outlined(&k, px + 214.0, py + 25.0, 13.0, rgb(200, 210, 230));
    }

    // ---------------------------------------------- EXP bar (bottom, full width)
    let need = exp_to_next(sim.save.level);
    let frac = sim.save.exp as f32 / need.max(1) as f32;
    gfx::rect(0.0, gfx::VH - 14.0, gfx::VW, 14.0, rgba(10, 12, 24, 220));
    gfx::rect(2.0, gfx::VH - 12.0, (gfx::VW - 4.0) * frac.clamp(0.0, 1.0), 10.0, rgb(240, 200, 70));
    let et = format!("EXP {}/{} ({:.1}%)", sim.save.exp, need, frac * 100.0);
    gfx::text_outlined(&et, gfx::VW - 20.0 - gfx::text_w(&et, 13.0), gfx::VH - 32.0, 13.0, rgb(255, 245, 210));

    // ---------------------------------------------- quest tracker (top-right, under the minimap)
    let tx = gfx::VW - 360.0;
    let qlines = quests::tracker_lines(&sim.save);
    let mut y = crate::minimap::bottom(sim) + 6.0;
    if !qlines.is_empty() {
        let lh = gfx::line_h(14.0);
        let shown: Vec<_> = qlines.iter().take(4).collect();
        let h: f32 = 14.0 + shown.iter().map(|(_, d, _)| 24.0 + gfx::wrap(&fill_keys(d, layout), 330.0, 14.0).len() as f32 * lh + 6.0).sum::<f32>();
        widgets::hud_panel(tx - 10.0, y - 6.0, 356.0, h);
        y += 2.0;
        for (name, desc, ready) in shown {
            let c = if *ready { rgb(140, 240, 170) } else { rgb(255, 220, 120) };
            gfx::text_outlined(name, tx, y, 16.0, c);
            y += 24.0;
            for l in gfx::wrap(&fill_keys(desc, layout), 330.0, 14.0) {
                gfx::text(&l, tx + 4.0, y, 14.0, rgb(230, 234, 245));
                y += lh;
            }
            y += 6.0;
        }
        y += 8.0;
    }
    gfx::text_outlined(&format!("[{}] Quest log", layout.key_label(Action::QuestLog)), tx, y + 4.0, 13.0, rgb(200, 210, 230));

    // ---------------------------------------------- boss bar
    if let Some(b) = sim.enemies.iter().find(|e| e.kind.is_boss()) {
        let w = 560.0;
        let x = (gfx::VW - w) * 0.5;
        let name = if b.boss_phase2 { format!("{} (Enraged!)", b.kind.name()) } else { b.kind.name().to_string() };
        gfx::text_centered_outlined(&name, gfx::VW * 0.5, 136.0, 20.0, rgb(255, 200, 180));
        gfx::rrect(x, 162.0, w, 18.0, 6.0, rgba(10, 12, 24, 230));
        gfx::rrect(x + 2.0, 164.0, (w - 4.0) * (b.hp / b.max_hp).max(0.0), 14.0, 5.0, rgb(200, 60, 50));
    }

    // ---------------------------------------------- toasts
    let mut ty = 150.0 + if sim.enemies.iter().any(|e| e.kind.is_boss()) { 50.0 } else { 0.0 };
    for t in sim.toasts.iter().rev().take(3) {
        let a = (t.life / 0.4).min(1.0);
        let w = gfx::text_w(&t.title, 22.0).max(gfx::text_w(&t.body, 16.0)) + 50.0;
        let h = if t.body.is_empty() { 42.0 } else { 64.0 };
        gfx::rrect(gfx::VW * 0.5 - w * 0.5, ty, w, h, 10.0, rgba(20, 24, 44, (200.0 * a) as u8));
        gfx::rrect(gfx::VW * 0.5 - w * 0.5, ty, w, 4.0, 2.0, alpha(widgets::gold(), a));
        gfx::text_centered(&t.title, gfx::VW * 0.5, ty + 10.0, 22.0, alpha(rgb(255, 230, 150), a));
        if !t.body.is_empty() {
            gfx::text_centered(&t.body, gfx::VW * 0.5, ty + 38.0, 16.0, alpha(rgb(230, 236, 250), a));
        }
        ty += h + 8.0;
    }

    // ---------------------------------------------- portal hint / sign bubble
    if sim.dialog.is_none() {
        if let Some(i) = sim.nearby_sign() {
            let s = &sim.map.signs[i];
            let text = fill_keys(s.text, layout);
            let w = 520.0;
            let lines = gfx::wrap(&text, w - 40.0, 17.0);
            let h = 24.0 + lines.len() as f32 * 22.0;
            let x = (gfx::VW - w) * 0.5;
            let y = 400.0 - h; // above the characters' heads
            widgets::panel(x, y, w, h + 16.0);
            for (k, l) in lines.iter().enumerate() {
                gfx::text(l, x + 20.0, y + 18.0 + k as f32 * 22.0, 17.0, widgets::ink());
            }
        }
        if let Some(pi) = sim.nearby_portal() {
            let portal = &sim.map.portals[pi];
            let s = format!("[{}] {}", layout.key_label(Action::Up), portal.label);
            gfx::text_centered_outlined(&s, gfx::VW * 0.5, gfx::VH - 130.0, 20.0, rgb(160, 230, 255));
        }
    }

    // ---------------------------------------------- controls hint
    if show_controls && sim.dialog.is_none() {
        let rows = [
            (format!("{} {}", layout.key_label(Action::Left), layout.key_label(Action::Right)), "Move"),
            (layout.key_label(Action::Jump), "Jump (x2 in air)"),
            (layout.key_label(Action::Light), "Light attack"),
            (layout.key_label(Action::Heavy), "Heavy / signature"),
            (layout.key_label(Action::Dodge), "Dodge"),
            (layout.key_label(Action::Interact), "Talk"),
            (layout.key_label(Action::Pickup), "Pick up loot (hold)"),
            (layout.key_label(Action::Chat), "Chat  (/w /p /g)"),
            (
                format!("{} {} {}", layout.key_label(Action::Social), layout.key_label(Action::Guild), layout.key_label(Action::Achievements)),
                "Friends / Guild / Achievements",
            ),
            ("Right-click".to_string(), "A player: profile, trade..."),
        ];
        let h = 22.0 + rows.len() as f32 * 20.0 + 24.0;
        let y0 = gfx::VH - 40.0 - h;
        widgets::hud_panel(14.0, y0, 384.0, h);
        for (i, (k, d)) in rows.iter().enumerate() {
            let yy = y0 + 12.0 + i as f32 * 20.0;
            gfx::text(k, 26.0, yy, 14.0, rgb(255, 220, 120));
            gfx::text(d, 156.0, yy, 14.0, rgb(230, 234, 245));
        }
        gfx::text("F1: hide   Esc: menu", 26.0, y0 + h - 26.0, 12.0, rgb(170, 180, 200));
    }

    // ---------------------------------------------- dialog
    if let Some(d) = &sim.dialog {
        // Quest offers show their rewards in the same window, before you accept.
        let offer = match d.dialog.action {
            quests::DialogAction::OfferQuest(id) if d.on_last_line() => quests::quest_def(id),
            _ => None,
        };
        let (x, w) = (180.0, 920.0);
        let h = if offer.is_some() { 262.0 } else { 190.0 };
        let y = gfx::VH - h - 30.0;
        widgets::panel(x, y, w, h);
        // speaker tag
        let tagw = gfx::text_w(d.dialog.speaker, 20.0) + 30.0;
        gfx::rrect(x + 30.0, y - 18.0, tagw, 34.0, 8.0, widgets::wood());
        gfx::text(d.dialog.speaker, x + 45.0, y - 11.0, 20.0, rgb(255, 236, 170));
        let line = d.message.clone().unwrap_or_else(|| d.dialog.lines.get(d.line).cloned().unwrap_or_default());
        let line = fill_keys(&line, layout);
        gfx::text_wrapped(&line, x + 34.0, y + 34.0, w - 68.0, 19.0, widgets::ink());
        if let Some(def) = offer {
            quest_rewards(def, x + 30.0, y + h - 132.0, w - 60.0);
        }
        if let Some(choices) = d.choices() {
            for (i, c) in choices.iter().enumerate() {
                let bx = x + w - 470.0 + i as f32 * 230.0;
                let by = y + h - 58.0;
                let sel = d.choice == i;
                gfx::rrect(bx, by, 210.0, 38.0, 8.0, if sel { widgets::accent() } else { rgb(214, 196, 160) });
                gfx::text_centered(c, bx + 105.0, by + 9.0, 18.0, if sel { rgb(255, 255, 255) } else { widgets::ink() });
            }
            gfx::text(
                &format!("[{}/{}] choose   [{}] confirm", layout.key_label(Action::Left), layout.key_label(Action::Right), layout.key_label(Action::Interact)),
                x + 34.0,
                y + h - 44.0,
                14.0,
                widgets::ink_soft(),
            );
        } else {
            let bob = (time * 5.0).sin() * 3.0;
            gfx::text(&format!("[{}] next", layout.key_label(Action::Interact)), x + w - 130.0, y + h - 40.0 + bob, 15.0, widgets::ink_soft());
        }
    }

    // ---------------------------------------------- quest log
    if sim.quest_log_open {
        let (x, y, w, h) = (290.0, 90.0, 700.0, 520.0);
        widgets::panel(x, y, w, h);
        widgets::title("Quest Log", x + w * 0.5, y + 20.0, 28.0);
        let mut yy = y + 70.0;
        let mut any = false;
        for q in sim.save.quests.iter().rev() {
            let Some(def) = quests::quest_def(&q.id) else { continue };
            any = true;
            let (status, c) = match q.state {
                QuestState::Active => ("In progress", rgb(200, 140, 40)),
                QuestState::ReadyToTurnIn => ("Ready to turn in", widgets::accent()),
                QuestState::Done => ("Completed", rgb(150, 140, 130)),
            };
            gfx::text(def.name, x + 40.0, yy, 19.0, if q.state == QuestState::Done { rgb(150, 140, 130) } else { widgets::ink() });
            gfx::text(status, x + w - 220.0, yy + 2.0, 15.0, c);
            yy += 24.0;
            if q.state != QuestState::Done {
                yy += gfx::text_wrapped(&fill_keys(def.summary, layout), x + 52.0, yy, w - 100.0, 15.0, widgets::ink_soft());
            }
            yy += 8.0;
            if yy > y + h - 60.0 {
                break;
            }
        }
        if !any {
            gfx::text_centered("No quests yet. Look for NPCs with a '!' above their heads.", x + w * 0.5, yy, 17.0, widgets::ink_soft());
        }
        gfx::text_centered(&format!("[{}] close", layout.key_label(Action::QuestLog)), x + w * 0.5, y + h - 40.0, 15.0, widgets::ink_soft());
    }

    // ---------------------------------------------- death overlay
    if p.state == PState::Dead {
        let a = (p.state_t / 0.8).min(1.0);
        gfx::rect(0.0, 0.0, gfx::VW, gfx::VH, rgba(20, 0, 10, (90.0 * a) as u8));
    }
}

/// "Rewards" strip: EXP, gold and item icons with names.
pub fn quest_rewards(def: &quests::QuestDef, x: f32, y: f32, w: f32) {
    gfx::rrect(x, y, w, 64.0, 8.0, rgb(232, 216, 182));
    gfx::text("Rewards", x + 14.0, y + 6.0, 14.0, widgets::ink_soft());
    let mut cx = x + 14.0;
    let cy = y + 26.0;
    for (id, label) in quests::reward_list(def) {
        let item = galebound_core::items::item(id);
        let tw = gfx::text_w(&label, 16.0);
        let iw = if item.is_some() { 46.0 } else if label.ends_with("gold") { 22.0 } else { 0.0 };
        if cx + iw + tw > x + w - 10.0 {
            break;
        }
        match item {
            Some(d) => {
                let t = crate::sprites::get(&format!("icon_i_{}", d.id), || crate::icons::item(d));
                let tier = crate::icons::tier_color(d.tier);
                gfx::rrect(cx, cy - 4.0, 36.0, 36.0, 5.0, rgb(92, 64, 42));
                gfx::rrect(cx + 2.0, cy - 2.0, 32.0, 32.0, 4.0, rgb(250, 240, 214));
                gfx::rect(cx + 4.0, cy + 26.0, 28.0, 2.0, rgb(tier[0], tier[1], tier[2]));
                gfx::tex(&t, cx, cy - 4.0, 1.0, false, Color::new(1.0, 1.0, 1.0, 1.0));
            }
            None if label.ends_with("gold") => gfx::circle(cx + 8.0, cy + 14.0, 8.0, rgb(240, 196, 70)),
            None => {}
        }
        let col = if label.ends_with("EXP") { rgb(140, 80, 200) } else if label.ends_with("gold") { rgb(170, 120, 20) } else { widgets::ink() };
        gfx::text(&label, cx + iw, cy + 6.0, 16.0, col);
        cx += iw + tw + 26.0;
    }
}
