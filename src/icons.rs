//! 18x18 pixel icons for items and abilities (drawn at 1 texture px = 1
//! canvas px, i.e. 36 UI units).

use crate::pixel::{light, rgb, shadow, Canvas, Layer, Px, INK};
use galebound_core::classes::Weapon;
use galebound_core::items::{self, Effect, EquipSlot, ItemDef, ItemKind};
use galebound_core::skills::{SkillDef, SkillEffect, SkillFx};

pub const ICON: i32 = 18;

fn outlined(f: impl FnOnce(&mut Canvas)) -> Canvas {
    let mut l = Layer::new(ICON, ICON);
    f(&mut l.c);
    let mut out = Canvas::new(ICON, ICON);
    l.stamp(&mut out, INK);
    out
}

pub fn tier_color(tier: u8) -> Px {
    match tier {
        0 => rgb(170, 160, 150),
        1 => rgb(110, 190, 110),
        2 => rgb(90, 150, 240),
        _ => rgb(200, 120, 240),
    }
}

fn flask(c: &mut Canvas, liquid: Px) {
    c.rect(7, 2, 4, 4, rgb(214, 214, 228));
    c.rect(6, 1, 6, 2, rgb(150, 110, 70));
    c.disc(9.0, 11.0, 5.5, rgb(214, 226, 240));
    c.disc(9.0, 11.5, 4.5, liquid);
    c.rect(5, 9, 8, 2, light(liquid, 0.25));
}

pub fn item(def: &ItemDef) -> Canvas {
    let accent = tier_color(def.tier);
    let mut out = match def.kind {
        ItemKind::Consumable(Effect::Restore { hp, mp }) => outlined(|c| {
            let liquid = match (hp > 0.0, mp > 0.0) {
                (true, true) => rgb(190, 100, 230),
                (false, true) => rgb(70, 120, 236),
                _ => rgb(222, 58, 74),
            };
            flask(c, liquid);
            if hp >= 0.6 || mp >= 0.6 {
                c.rect(7, 6, 4, 1, rgb(240, 200, 80));
            }
        }),
        ItemKind::Consumable(Effect::Buff { mods, .. }) => outlined(|c| {
            let liquid = if mods.crit > 0.0 { rgb(120, 200, 90) } else if mods.atk_pct > 0.0 { rgb(240, 140, 50) } else { rgb(150, 150, 160) };
            if mods.crit > 0.0 {
                // a cup of tea with a clover
                c.rect(4, 8, 10, 7, rgb(240, 236, 226));
                c.rect(13, 9, 3, 4, rgb(240, 236, 226));
                c.rect(5, 8, 8, 2, liquid);
                for (x, y) in [(8.0, 4.0), (10.0, 4.0), (9.0, 2.5)] {
                    c.disc(x, y, 1.6, rgb(80, 170, 70));
                }
            } else {
                flask(c, liquid);
            }
        }),
        ItemKind::Material => outlined(|c| material(c, def.id)),
        ItemKind::Special(items::Special::Revive) => outlined(|c| {
            // a rolled parchment scroll tied with a red ribbon
            let paper = rgb(236, 222, 180);
            c.rect(3, 5, 12, 8, paper);
            c.disc(3.5, 9.0, 3.0, shadow(paper, 0.15));
            c.disc(14.5, 9.0, 3.0, shadow(paper, 0.15));
            c.disc(14.5, 9.0, 1.2, shadow(paper, 0.4));
            c.rect(5, 7, 7, 1, rgb(170, 140, 100));
            c.rect(5, 10, 6, 1, rgb(170, 140, 100));
            c.rect(8, 4, 2, 10, rgb(210, 60, 70));
            c.rect(7, 13, 1, 3, rgb(210, 60, 70));
            c.rect(10, 13, 1, 3, rgb(210, 60, 70));
            c.set(9, 8, rgb(255, 240, 150));
        }),
        ItemKind::Special(items::Special::SkillReset) => outlined(|c| {
            // a golden ticket with a circular arrow
            let t = rgb(240, 200, 90);
            c.rect(2, 4, 14, 10, t);
            c.rect(2, 8, 1, 2, rgb(0, 0, 0));
            c.rect(15, 8, 1, 2, rgb(0, 0, 0));
            c.rect(3, 5, 12, 1, light(t, 0.4));
            let arrow = rgb(140, 70, 190);
            for (x, y) in [(7, 6), (8, 6), (9, 6), (10, 7), (11, 8), (11, 9), (10, 10), (9, 11), (8, 11), (7, 11), (6, 10), (6, 9)] {
                c.set(x, y, arrow);
            }
            c.set(6, 6, arrow);
            c.set(6, 7, arrow);
            c.set(5, 7, arrow);
        }),
        ItemKind::Equip(EquipSlot::Weapon) => weapon_icon(items::weapon_art(def.id).unwrap_or(Weapon::Club), accent),
        ItemKind::Equip(EquipSlot::Armor) => outlined(|c| {
            let body = match def.look {
                items::Look::Top(t, d) => {
                    let dye = galebound_core::appearance::DYES[(d as usize) % galebound_core::appearance::DYES.len()].1;
                    let _ = t;
                    rgb(dye[0], dye[1], dye[2])
                }
                _ => rgb(160, 120, 80),
            };
            let plate = def.mods.pdef >= 16.0;
            let robe = def.mods.mdef > def.mods.pdef;
            let col = if plate { rgb(176, 184, 200) } else { body };
            c.rect(5, 4, 8, if robe { 13 } else { 10 }, col);
            c.rect(2, 4, 4, 6, shadow(col, 0.1));
            c.rect(12, 4, 4, 6, shadow(col, 0.1));
            c.rect(7, 3, 4, 2, shadow(col, 0.3));
            if plate {
                c.rect(8, 6, 2, 7, light(col, 0.35));
            } else {
                c.rect(5, 11, 8, 1, accent);
            }
        }),
        ItemKind::Equip(EquipSlot::Boots) => outlined(|c| {
            let col = if def.mods.pdef >= 10.0 { rgb(170, 176, 190) } else if def.mods.speed_pct > 0.05 { rgb(120, 170, 120) } else { rgb(150, 100, 60) };
            c.rect(4, 3, 4, 10, col);
            c.rect(4, 12, 8, 3, col);
            c.rect(10, 5, 4, 8, shadow(col, 0.2));
            c.rect(10, 12, 7, 3, shadow(col, 0.2));
            c.rect(4, 3, 4, 1, light(col, 0.3));
            if def.mods.speed_pct > 0.05 {
                c.rect(2, 4, 2, 3, rgb(250, 250, 250));
            }
        }),
        ItemKind::Equip(EquipSlot::Accessory) => outlined(|c| {
            let gem = if def.mods.main.str > 0 {
                rgb(220, 70, 70)
            } else if def.mods.main.dex > 0 {
                rgb(90, 200, 110)
            } else if def.mods.main.int > 0 {
                rgb(90, 140, 240)
            } else {
                rgb(240, 200, 70)
            };
            if def.id.contains("amulet") || def.id.contains("pendant") || def.id.contains("charm") {
                c.line(4, 2, 9, 9, rgb(220, 190, 90));
                c.line(14, 2, 9, 9, rgb(220, 190, 90));
                c.disc(9.0, 11.0, 4.0, gem);
                c.set(8, 10, light(gem, 0.6));
            } else if def.id.contains("coin") {
                c.disc(9.0, 9.0, 6.0, rgb(232, 196, 80));
                c.disc(9.0, 9.0, 4.0, rgb(250, 220, 110));
                c.rect(8, 6, 2, 6, rgb(200, 160, 60));
            } else {
                c.disc(9.0, 11.0, 5.0, rgb(214, 170, 80));
                c.disc(9.0, 11.0, 3.0, rgb(0, 0, 0));
                c.disc(9.0, 5.0, 3.0, gem);
            }
        }),
    };
    // Remove the hole of rings (black placeholder) for a clean look.
    for p in out.px.iter_mut() {
        if *p == rgb(0, 0, 0) {
            *p = [0, 0, 0, 0];
        }
    }
    out
}

fn weapon_icon(w: Weapon, accent: Px) -> Canvas {
    outlined(|c| {
        let steel = rgb(210, 220, 236);
        match w {
            Weapon::Stick => {
                c.taper((3.0, 16.0), (14.0, 3.0), 1.4, 1.0, rgb(132, 92, 56));
                c.line(9, 9, 13, 10, rgb(132, 92, 56));
                c.disc(14.0, 10.0, 1.5, rgb(110, 180, 80));
            }
            Weapon::WoodSword => {
                c.taper((5.0, 13.0), (15.0, 3.0), 2.4, 1.2, rgb(196, 150, 96));
                c.line(2, 10, 8, 16, rgb(126, 84, 50));
                c.line(3, 15, 5, 13, rgb(126, 84, 50));
            }
            Weapon::Club => {
                c.taper((4.0, 15.0), (13.0, 4.0), 1.5, 3.5, rgb(156, 104, 60));
                c.disc(13.0, 4.0, 2.5, rgb(156, 104, 60));
            }
            Weapon::SwordShield => {
                c.taper((4.0, 15.0), (15.0, 2.0), 2.0, 1.0, steel);
                c.line(3, 11, 8, 16, accent);
                c.poly(&[(1.0, 4.0), (8.0, 4.0), (8.0, 9.0), (4.5, 13.0), (1.0, 9.0)], rgb(90, 120, 180));
            }
            Weapon::TwinDaggers => {
                c.taper((3.0, 15.0), (10.0, 4.0), 1.5, 0.6, steel);
                c.taper((8.0, 16.0), (16.0, 6.0), 1.5, 0.6, steel);
                c.rect(3, 13, 3, 2, accent);
                c.rect(8, 14, 3, 2, accent);
            }
            Weapon::Gauntlets => {
                c.rect(4, 6, 10, 9, rgb(190, 130, 70));
                c.rect(4, 4, 10, 3, steel);
                for x in [5, 8, 11] {
                    c.rect(x, 4, 2, 2, light(steel, 0.3));
                }
                c.rect(4, 12, 10, 2, accent);
            }
            Weapon::Bow => {
                for i in 0..8 {
                    let t = i as f32 / 7.0 * 2.0 - 1.0;
                    let x = 5.0 + (1.0 - t * t) * 5.0;
                    c.disc(x, 9.0 + t * 7.0, 1.2, rgb(150, 96, 50));
                }
                c.line(5, 2, 5, 16, rgb(240, 240, 240));
                c.rect(9, 8, 2, 2, accent);
            }
            Weapon::Staff => {
                c.taper((4.0, 16.0), (12.0, 5.0), 1.2, 1.2, rgb(150, 100, 56));
                c.disc(13.0, 4.0, 3.0, accent);
                c.set(12, 3, rgb(255, 255, 255));
            }
        }
    })
}

pub fn fx_color(fx: SkillFx) -> Px {
    match fx {
        SkillFx::Impact => rgb(240, 150, 60),
        SkillFx::Steel => rgb(170, 186, 210),
        SkillFx::Shadow => rgb(140, 90, 200),
        SkillFx::Storm => rgb(250, 220, 70),
        SkillFx::Wind => rgb(120, 220, 190),
        SkillFx::Arcane => rgb(110, 200, 255),
        SkillFx::Nature => rgb(110, 200, 90),
        SkillFx::Holy => rgb(255, 236, 150),
    }
}

pub fn skill(s: &SkillDef) -> Canvas {
    let col = fx_color(s.fx);
    let mut c = Canvas::new(ICON, ICON);
    // rounded tile background
    c.rect(1, 0, ICON - 2, ICON, INK);
    c.rect(0, 1, ICON, ICON - 2, INK);
    c.rect(1, 1, ICON - 2, ICON - 2, shadow(col, 0.55));
    c.rect(1, 1, ICON - 2, 6, shadow(col, 0.45));
    let white = rgb(255, 255, 255);
    match s.effect {
        SkillEffect::Strike { proj: Some(p), .. } => {
            let k = (p.count.max(1) as i32).min(5);
            for i in 0..k {
                let ty = 9 + (i * 2 - (k - 1)) * 3 / 2;
                c.line(3, 9, 14, ty, col);
                c.set(15, ty, white);
            }
            if p.pierce {
                c.line(3, 9, 15, 9, white);
            }
        }
        SkillEffect::Strike { hits, both_sides, .. } => {
            if both_sides {
                for a in 0..12 {
                    let t = a as f32 / 12.0 * std::f32::consts::TAU;
                    c.set((9.0 + t.cos() * 6.0) as i32, (9.0 + t.sin() * 5.0) as i32, col);
                }
                c.disc(9.0, 9.0, 2.0, white);
            } else {
                let n = hits.clamp(1, 4) as i32;
                for i in 0..n {
                    let o = i * 3 - (n - 1) * 3 / 2;
                    c.line(4 + o, 14, 13 + o, 4, if i == 0 { white } else { col });
                }
            }
        }
        SkillEffect::Buff { .. } => {
            c.poly(&[(9.0, 2.5), (15.0, 9.0), (11.5, 9.0), (11.5, 15.0), (6.5, 15.0), (6.5, 9.0), (3.0, 9.0)], col);
            c.rect(8, 6, 2, 8, light(col, 0.5));
        }
        SkillEffect::Heal { .. } => {
            c.rect(7, 3, 4, 12, col);
            c.rect(3, 7, 12, 4, col);
            c.rect(8, 4, 2, 10, white);
        }
    }
    c
}

fn material(c: &mut Canvas, id: &str) {
    match id {
        "moss_clump" => {
            c.ellipse(9.0, 11.0, 6.0, 4.5, rgb(90, 170, 80));
            c.disc(7.0, 9.0, 2.0, rgb(130, 210, 110));
        }
        "beak_shard" | "bramble_thorn" | "storm_horn" => {
            let col = match id {
                "beak_shard" => rgb(240, 196, 90),
                "bramble_thorn" => rgb(150, 110, 70),
                _ => rgb(250, 230, 120),
            };
            c.poly(&[(4.0, 15.0), (12.0, 13.0), (14.0, 3.0)], col);
            c.line(6, 14, 13, 5, light(col, 0.35));
        }
        "wool_tuft" => {
            for (x, y) in [(7.0, 10.0), (11.0, 9.0), (9.0, 7.0), (12.0, 12.0), (6.0, 13.0)] {
                c.disc(x, y, 3.2, rgb(244, 240, 232));
            }
        }
        "gust_wisp" | "wisp_ember" | "thunder_core" | "star_dust" => {
            let col = match id {
                "gust_wisp" => rgb(140, 230, 210),
                "wisp_ember" => rgb(255, 170, 90),
                "thunder_core" => rgb(250, 230, 80),
                _ => rgb(200, 170, 255),
            };
            c.disc(9.0, 9.0, 5.5, col);
            c.disc(9.0, 9.0, 3.0, light(col, 0.5));
            c.set(7, 7, rgb(255, 255, 255));
        }
        "cap_spore" => {
            c.ellipse(9.0, 8.0, 6.0, 3.5, rgb(140, 110, 190));
            c.rect(8, 9, 3, 6, rgb(230, 220, 200));
            c.set(7, 7, rgb(220, 200, 255));
        }
        "rotten_bark" => {
            c.rect(4, 5, 10, 9, rgb(110, 80, 56));
            c.line(6, 6, 6, 12, rgb(80, 56, 40));
            c.line(10, 6, 11, 12, rgb(80, 56, 40));
        }
        "crab_shell" => {
            c.ellipse(9.0, 10.0, 6.5, 5.0, rgb(200, 110, 90));
            c.line(5, 9, 13, 9, rgb(150, 70, 60));
        }
        "hawk_feather" => {
            c.taper((4.0, 15.0), (14.0, 3.0), 1.0, 3.5, rgb(170, 190, 220));
            c.line(4, 15, 13, 4, rgb(240, 240, 250));
        }
        "iron_ore" => {
            c.poly(&[(4.0, 13.0), (6.0, 6.0), (11.0, 4.0), (15.0, 9.0), (12.0, 15.0)], rgb(110, 104, 112));
            c.set(8, 8, rgb(210, 160, 120));
            c.set(11, 11, rgb(210, 160, 120));
        }
        "shadow_silk" => {
            c.disc(9.0, 9.0, 5.0, rgb(80, 60, 120));
            c.line(5, 7, 13, 11, rgb(140, 110, 190));
            c.line(5, 11, 13, 7, rgb(140, 110, 190));
        }
        _ => c.disc(9.0, 9.0, 5.0, rgb(180, 180, 180)),
    }
}
