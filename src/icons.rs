//! 18x18 pixel icons for items and abilities (drawn at 1 texture px = 1
//! canvas px, i.e. 36 UI units).

use crate::pixel::{light, rgb, shadow, Canvas, Layer, Px, INK};
use galebound_core::classes::Weapon;
use galebound_core::items::{self, Effect, GearKind, ItemDef, ItemKind};
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
        3 => rgb(200, 120, 240),
        // Gold and Red tiers (co-op dungeon gear)
        4 => rgb(250, 196, 50),
        _ => rgb(236, 56, 56),
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
        ItemKind::Special(items::Special::Pet(species)) => {
            // the pet itself, shrunk to icon size on a soft badge
            let src = crate::petart::pet(species, 0);
            let mut c = Canvas::new(ICON, ICON);
            c.disc(9.0, 9.5, 8.5, rgb(255, 236, 190));
            for y in 0..ICON {
                for x in 0..ICON {
                    let px = src.get(x * src.w / ICON, y * src.h / ICON);
                    if px[3] > 0 {
                        c.set(x, y + 1, px);
                    }
                }
            }
            c
        }
        ItemKind::Special(items::Special::PetFood) => outlined(|c| {
            // a bowl of kibble
            let bowl = rgb(200, 80, 70);
            c.ellipse(9.0, 9.0, 7.0, 2.5, rgb(170, 110, 60));
            for (x, y) in [(5, 8), (7, 7), (9, 8), (11, 7), (13, 8), (8, 9), (10, 9), (6, 9), (12, 9)] {
                c.set(x, y, rgb(232, 170, 100));
            }
            c.rect(3, 10, 13, 4, bowl);
            c.rect(4, 14, 11, 2, shadow(bowl, 0.25));
            c.rect(4, 11, 11, 1, light(bowl, 0.35));
            c.disc(9.0, 4.0, 1.4, rgb(250, 140, 160));
            c.set(8, 5, rgb(250, 140, 160));
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
        ItemKind::Consumable(Effect::Food { .. }) => outlined(|c| food_icon(c, def.id)),
        ItemKind::Special(items::Special::HouseDeed) => outlined(|c| {
            // a deed with a little house stamped on it
            let paper = rgb(240, 228, 196);
            c.rect(3, 2, 12, 15, paper);
            c.rect(3, 2, 12, 1, shadow(paper, 0.2));
            c.poly(&[(5.0, 9.0), (9.0, 5.0), (13.0, 9.0)], rgb(190, 70, 60));
            c.rect(6, 9, 6, 4, rgb(200, 160, 110));
            c.rect(8, 10, 2, 3, rgb(120, 80, 50));
            c.disc(12.5, 14.5, 2.0, rgb(200, 50, 60));
        }),
        ItemKind::Special(items::Special::Cache) => outlined(|c| {
            // a storm-blue chest with gold bands
            let wood = rgb(70, 90, 160);
            c.rect(2, 7, 14, 9, wood);
            c.rect(2, 4, 14, 4, light(wood, 0.2));
            c.rect(2, 8, 14, 1, rgb(240, 200, 70));
            c.rect(8, 6, 2, 5, rgb(240, 200, 70));
            c.set(9, 8, rgb(255, 255, 220));
            c.rect(4, 2, 1, 2, rgb(150, 230, 255));
            c.rect(13, 1, 1, 2, rgb(150, 230, 255));
        }),
        ItemKind::Furniture(f) => outlined(|c| furniture_icon(c, f)),
        ItemKind::Equip(GearKind::Weapon) => weapon_icon(items::weapon_art(def.id).unwrap_or(Weapon::Club), accent),
        ItemKind::Equip(GearKind::Torso) => outlined(|c| {
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
        ItemKind::Equip(GearKind::Helmet) => outlined(|c| {
            let hat = match def.look {
                items::Look::Hat(h) => h,
                _ => 2,
            };
            match hat {
                1 => {
                    // bandana: a band with knot tails
                    c.rect(3, 8, 12, 4, rgb(196, 60, 64));
                    c.rect(3, 8, 12, 1, rgb(232, 110, 110));
                    c.rect(14, 10, 3, 2, rgb(170, 48, 52));
                    c.rect(15, 12, 2, 3, rgb(170, 48, 52));
                }
                3 => {
                    // pointed hat
                    c.poly(&[(3.0, 14.0), (15.0, 14.0), (10.0, 2.0)], rgb(80, 80, 170));
                    c.rect(1, 13, 16, 3, rgb(66, 66, 150));
                    c.rect(6, 11, 7, 2, rgb(230, 196, 90));
                }
                4 => {
                    // iron sallet
                    c.disc(9.0, 10.0, 6.5, rgb(176, 184, 200));
                    c.rect(2, 10, 15, 6, rgb(0, 0, 0));
                    c.rect(3, 9, 13, 2, rgb(146, 154, 172));
                    c.rect(8, 3, 2, 7, rgb(214, 220, 234));
                }
                5 => {
                    // wide-brimmed hat with a feather
                    c.rect(1, 12, 16, 3, rgb(130, 90, 56));
                    c.disc(9.0, 10.0, 5.0, rgb(150, 104, 64));
                    c.rect(4, 10, 10, 2, rgb(196, 60, 64));
                    c.taper((12.0, 9.0), (16.0, 2.0), 2.0, 0.8, rgb(240, 240, 250));
                }
                6 => {
                    // hood
                    c.disc(9.0, 9.0, 7.0, rgb(70, 120, 76));
                    c.disc(10.0, 11.0, 4.0, rgb(0, 0, 0));
                    c.rect(4, 14, 10, 3, rgb(60, 104, 66));
                }
                7 => {
                    // circlet
                    c.ellipse(9.0, 10.0, 7.0, 3.0, rgb(214, 220, 236));
                    c.ellipse(9.0, 10.0, 5.0, 1.6, rgb(0, 0, 0));
                    c.disc(9.0, 7.0, 2.2, rgb(90, 160, 240));
                }
                _ => {
                    // leather cap
                    c.disc(9.0, 10.0, 6.5, rgb(150, 100, 60));
                    c.rect(2, 11, 15, 6, rgb(0, 0, 0));
                    c.rect(9, 12, 7, 2, rgb(120, 78, 44));
                    c.rect(5, 6, 3, 1, rgb(186, 136, 90));
                }
            }
        }),
        ItemKind::Equip(GearKind::Legs) => outlined(|c| {
            let col = match def.look {
                items::Look::Bottom(_, d) => {
                    let dye = galebound_core::appearance::DYES[(d as usize) % galebound_core::appearance::DYES.len()].1;
                    rgb(dye[0], dye[1], dye[2])
                }
                _ => rgb(110, 90, 70),
            };
            let col = if def.mods.pdef >= 10.0 { rgb(170, 178, 194) } else { col };
            c.rect(4, 2, 10, 4, col);
            c.rect(4, 5, 4, 12, col);
            c.rect(10, 5, 4, 12, shadow(col, 0.12));
            c.rect(4, 2, 10, 1, accent);
            c.rect(8, 6, 2, 3, shadow(col, 0.3));
        }),
        ItemKind::Equip(GearKind::Back) => outlined(|c| {
            let back = match def.look {
                items::Look::Back(b) => b,
                _ => 1,
            };
            match back {
                3 => {
                    for k in 0..4 {
                        let y = 4.0 + k as f32 * 2.6;
                        c.taper((8.0, y), (16.0 - k as f32, y + 4.0), 3.0, 1.0, rgb(244, 244, 252));
                        c.taper((10.0, y), (2.0 + k as f32, y + 4.0), 3.0, 1.0, rgb(226, 230, 244));
                    }
                }
                4 => {
                    c.rect(4, 4, 10, 12, rgb(150, 104, 62));
                    c.rect(4, 4, 10, 3, rgb(120, 80, 46));
                    c.rect(6, 10, 6, 4, rgb(176, 128, 80));
                    c.rect(3, 15, 12, 2, rgb(80, 120, 170));
                }
                _ => {
                    let col = if back == 2 { rgb(60, 110, 190) } else { rgb(150, 70, 60) };
                    c.poly(&[(6.0, 2.0), (12.0, 2.0), (16.0, if back == 2 { 17.0 } else { 13.0 }), (2.0, if back == 2 { 17.0 } else { 13.0 })], col);
                    c.rect(5, 2, 8, 2, rgb(230, 196, 90));
                    c.line(9, 4, 9, 12, shadow(col, 0.3));
                }
            }
        }),
        ItemKind::Equip(GearKind::Boots) => outlined(|c| {
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
        ItemKind::Equip(GearKind::Necklace) | ItemKind::Equip(GearKind::Ring) | ItemKind::Equip(GearKind::Earring) => outlined(|c| {
            let gem = if def.mods.main.str > 0 {
                rgb(220, 70, 70)
            } else if def.mods.main.dex > 0 {
                rgb(90, 200, 110)
            } else if def.mods.main.int > 0 {
                rgb(90, 140, 240)
            } else {
                rgb(240, 200, 70)
            };
            if def.slot() == Some(GearKind::Earring) {
                // a pair of hanging earrings
                for x in [5.0f32, 13.0] {
                    c.disc(x, 4.0, 1.6, rgb(220, 190, 90));
                    c.line(x as i32, 5, x as i32, 9, rgb(220, 190, 90));
                    c.disc(x, 12.0, 2.8, if def.id.contains("coin") { rgb(240, 200, 80) } else { gem });
                    c.set(x as i32 - 1, 11, light(gem, 0.6));
                }
            } else if def.slot() == Some(GearKind::Necklace) {
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
            // 0.15.0
            Weapon::DualPistols => {
                for (x, y) in [(2.0f32, 6.0f32), (7.0, 10.0)] {
                    c.rect(x as i32, y as i32, 9, 3, rgb(70, 70, 84));
                    c.rect(x as i32, y as i32 + 2, 3, 5, rgb(130, 84, 50));
                    c.set(x as i32 + 4, y as i32 + 1, accent);
                }
            }
            Weapon::Greatsword => {
                c.taper((4.0, 15.0), (16.0, 1.0), 3.4, 1.2, steel);
                c.line(2, 10, 9, 17, rgb(160, 60, 50));
                c.line(1, 16, 3, 14, accent);
            }
            Weapon::Greatshield => {
                c.ellipse(9.0, 9.0, 6.0, 8.0, rgb(150, 150, 168));
                c.ellipse(9.0, 9.0, 5.0, 7.0, rgb(230, 230, 240));
                c.rect(8, 4, 2, 10, accent);
                c.rect(5, 7, 8, 2, accent);
            }
            Weapon::Nunchucks => {
                c.taper((3.0, 15.0), (7.0, 7.0), 1.4, 1.4, rgb(70, 46, 40));
                c.line(7, 7, 11, 5, rgb(200, 200, 210));
                c.taper((11.0, 5.0), (16.0, 12.0), 1.4, 1.4, rgb(70, 46, 40));
                c.set(7, 7, accent);
            }
            Weapon::WarStaff => {
                c.taper((2.0, 16.0), (16.0, 2.0), 1.3, 1.3, rgb(96, 64, 40));
                c.disc(2.5, 15.5, 1.8, steel);
                c.disc(15.5, 2.5, 1.8, steel);
                c.disc(9.0, 9.0, 1.4, accent);
            }
            Weapon::Katana => {
                c.taper((5.0, 13.0), (15.0, 2.0), 1.7, 0.6, steel);
                c.disc(5.0, 13.0, 1.6, rgb(40, 36, 44));
                c.line(2, 16, 4, 14, accent);
            }
            Weapon::Kusarigama => {
                c.taper((4.0, 14.0), (10.0, 6.0), 1.2, 1.2, rgb(70, 50, 40));
                c.taper((10.0, 6.0), (15.0, 9.0), 1.4, 0.6, steel);
                for i in 0..4 {
                    c.set(3 - i / 2, 15 + i / 2, rgb(180, 184, 196));
                }
                c.set(10, 6, accent);
            }
            Weapon::Crossbow => {
                c.rect(3, 8, 11, 3, rgb(110, 72, 44));
                c.line(12, 3, 14, 9, rgb(70, 70, 84));
                c.line(12, 15, 14, 10, rgb(70, 70, 84));
                c.line(12, 3, 8, 9, rgb(236, 236, 236));
                c.line(12, 15, 8, 10, rgb(236, 236, 236));
                c.set(5, 9, accent);
            }
            Weapon::Greatbow => {
                for i in 0..10 {
                    let t = i as f32 / 9.0 * 2.0 - 1.0;
                    let x = 4.0 + (1.0 - t * t) * 6.0;
                    c.disc(x, 9.0 + t * 8.0, 1.4, rgb(84, 56, 40));
                }
                c.line(4, 1, 4, 17, rgb(240, 240, 240));
                c.rect(9, 8, 2, 2, accent);
            }
            Weapon::CosmicOrbs => {
                c.disc(6.0, 10.0, 3.5, rgb(150, 200, 255));
                c.disc(12.0, 6.0, 2.8, rgb(255, 210, 120));
                c.disc(13.0, 13.0, 2.0, accent);
                c.set(5, 9, rgb(255, 255, 255));
            }
            Weapon::Cards => {
                for (i, x) in [3, 7, 11].iter().enumerate() {
                    c.rect(*x, 4 + i as i32, 5, 8, rgb(250, 248, 240));
                    c.set(*x + 2, 7 + i as i32, if i == 1 { rgb(200, 40, 50) } else { rgb(30, 30, 40) });
                }
                c.rect(4, 13, 10, 2, accent);
            }
            Weapon::HandCannon => {
                c.rect(3, 6, 12, 6, rgb(70, 72, 84));
                c.rect(5, 6, 2, 6, rgb(200, 160, 70));
                c.rect(10, 6, 2, 6, rgb(200, 160, 70));
                c.disc(15.0, 9.0, 3.0, rgb(50, 50, 58));
                c.rect(3, 11, 3, 5, rgb(120, 76, 44));
            }
            Weapon::OmniRifle => {
                c.rect(1, 9, 7, 3, rgb(80, 70, 90));
                c.rect(7, 9, 10, 2, rgb(150, 156, 176));
                c.rect(6, 6, 5, 2, rgb(40, 40, 48));
                c.set(5, 10, rgb(120, 230, 255));
                c.rect(3, 11, 2, 4, accent);
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
        SkillEffect::Party { .. } => {
            // three figures under a halo
            for x in [4.0f32, 9.0, 14.0] {
                c.disc(x, 9.0, 1.8, col);
                c.rect(x as i32 - 1, 11, 3, 4, col);
            }
            c.ellipse(9.0, 4.5, 6.0, 1.6, white);
        }
        SkillEffect::Summon { .. } => {
            // a little turret / companion
            c.rect(5, 12, 8, 3, col);
            c.rect(7, 7, 5, 5, light(col, 0.3));
            c.rect(11, 8, 5, 2, white);
            c.disc(9.0, 4.0, 1.5, white);
        }
        SkillEffect::Gamble => {
            // a die
            c.rect(4, 4, 10, 10, white);
            for (x, y) in [(6, 6), (11, 6), (8, 9), (6, 12), (11, 12)] {
                c.set(x, y, shadow(col, 0.2));
            }
        }
    }
    c
}

fn food_icon(c: &mut Canvas, id: &str) {
    let plate = rgb(236, 236, 244);
    match id {
        "herb_flatbread" => {
            c.ellipse(9.0, 10.0, 7.0, 4.0, rgb(222, 180, 110));
            c.disc(7.0, 9.0, 1.4, rgb(90, 160, 70));
            c.disc(11.0, 10.0, 1.4, rgb(90, 160, 70));
        }
        "glade_omelette" => {
            c.ellipse(9.0, 12.0, 8.0, 3.0, plate);
            c.ellipse(9.0, 10.0, 6.0, 3.0, rgb(250, 214, 90));
            c.set(7, 9, rgb(90, 160, 70));
        }
        "hearty_stew" | "wayfarers_pie" => {
            let fill = if id == "hearty_stew" { rgb(160, 90, 50) } else { rgb(214, 160, 90) };
            c.rect(3, 9, 12, 6, rgb(150, 100, 70));
            c.ellipse(9.0, 9.0, 6.5, 2.5, fill);
            c.disc(7.0, 8.5, 1.2, rgb(230, 140, 60));
            c.disc(11.0, 9.0, 1.2, rgb(120, 180, 80));
        }
        "glowcap_skewers" => {
            c.line(3, 15, 15, 3, rgb(170, 130, 80));
            for (x, y) in [(6.0, 12.0), (9.0, 9.0), (12.0, 6.0)] {
                c.disc(x, y, 2.2, rgb(150, 120, 210));
                c.set(x as i32 - 1, y as i32 - 1, rgb(220, 200, 255));
            }
        }
        "honey_roast" => {
            c.ellipse(9.0, 12.0, 8.0, 3.0, plate);
            c.ellipse(9.0, 9.0, 6.0, 4.0, rgb(170, 90, 50));
            c.rect(5, 7, 8, 1, rgb(250, 190, 60));
        }
        "crab_cakes" => {
            c.ellipse(9.0, 12.0, 8.0, 3.0, plate);
            c.disc(6.0, 10.0, 3.0, rgb(214, 150, 70));
            c.disc(12.0, 10.0, 3.0, rgb(214, 150, 70));
        }
        "aether_tea" => {
            c.rect(4, 7, 9, 8, rgb(240, 236, 226));
            c.rect(12, 9, 3, 4, rgb(240, 236, 226));
            c.rect(5, 7, 7, 2, rgb(170, 140, 230));
            c.set(8, 4, rgb(220, 220, 255));
            c.set(9, 3, rgb(220, 220, 255));
        }
        "stormberry_tart" => {
            c.ellipse(9.0, 11.0, 7.0, 3.5, rgb(214, 170, 100));
            for (x, y) in [(6.0, 9.0), (9.0, 8.0), (12.0, 9.0), (8.0, 11.0), (11.0, 11.0)] {
                c.disc(x, y, 1.4, rgb(90, 90, 220));
            }
            c.set(9, 5, rgb(250, 230, 90));
        }
        _ => {
            // a feast platter with a glowing shard
            c.ellipse(9.0, 12.0, 8.0, 3.5, rgb(240, 200, 80));
            c.ellipse(7.0, 9.0, 4.0, 3.0, rgb(170, 90, 50));
            c.disc(12.0, 9.0, 2.5, rgb(214, 150, 70));
            c.disc(10.0, 5.0, 1.6, rgb(150, 230, 255));
        }
    }
}

fn furniture_icon(c: &mut Canvas, f: items::Furniture) {
    use items::Furniture as F;
    let wood = rgb(160, 106, 62);
    match f {
        F::Bed => {
            c.rect(2, 9, 14, 5, wood);
            c.rect(3, 7, 12, 3, rgb(240, 240, 250));
            c.rect(3, 7, 4, 2, rgb(250, 250, 255));
            c.rect(8, 7, 7, 3, rgb(90, 130, 210));
            c.rect(2, 14, 2, 2, shadow(wood, 0.3));
            c.rect(14, 14, 2, 2, shadow(wood, 0.3));
        }
        F::Chest => {
            c.rect(2, 7, 14, 9, wood);
            c.rect(2, 4, 14, 4, light(wood, 0.2));
            c.rect(2, 8, 14, 1, rgb(110, 110, 120));
            c.rect(8, 7, 2, 3, rgb(230, 190, 80));
        }
        F::Stove => {
            c.rect(3, 5, 12, 11, rgb(80, 80, 92));
            c.rect(5, 10, 8, 4, rgb(40, 30, 30));
            c.rect(6, 11, 6, 2, rgb(255, 150, 60));
            c.rect(11, 2, 2, 4, rgb(100, 100, 110));
        }
        F::Workbench => {
            c.rect(1, 7, 16, 3, wood);
            c.rect(2, 10, 2, 6, shadow(wood, 0.2));
            c.rect(14, 10, 2, 6, shadow(wood, 0.2));
            c.rect(5, 4, 6, 3, rgb(150, 150, 165));
            c.rect(11, 3, 1, 4, rgb(130, 90, 60));
        }
        F::Table => {
            c.rect(1, 7, 16, 3, wood);
            c.rect(3, 10, 2, 6, shadow(wood, 0.2));
            c.rect(13, 10, 2, 6, shadow(wood, 0.2));
        }
        F::Bookshelf => {
            c.rect(3, 1, 12, 16, wood);
            for (y, col) in [(3, rgb(200, 70, 70)), (8, rgb(70, 110, 200)), (13, rgb(90, 170, 90))] {
                c.rect(5, y, 8, 3, col);
            }
        }
        F::Plant => {
            c.rect(6, 11, 6, 5, rgb(190, 100, 70));
            for (x, y) in [(9.0, 6.0), (6.0, 8.0), (12.0, 8.0), (9.0, 3.0)] {
                c.disc(x, y, 2.4, rgb(80, 170, 80));
            }
        }
        F::Rug => {
            c.rect(1, 8, 16, 5, rgb(160, 54, 64));
            c.rect(2, 9, 14, 1, rgb(230, 190, 90));
            c.rect(2, 11, 14, 1, rgb(230, 190, 90));
        }
        F::Fireplace => {
            c.rect(2, 3, 14, 13, rgb(150, 140, 136));
            c.rect(5, 8, 8, 8, rgb(40, 28, 30));
            c.disc(9.0, 13.0, 2.5, rgb(255, 150, 60));
        }
        F::WeaponRack => {
            c.rect(2, 4, 14, 2, wood);
            c.rect(5, 2, 2, 14, rgb(210, 220, 236));
            c.rect(11, 2, 2, 14, rgb(210, 220, 236));
        }
        F::ArmorStand => {
            c.rect(8, 6, 2, 10, wood);
            c.rect(4, 5, 10, 6, rgb(170, 178, 196));
            c.disc(9.0, 3.0, 2.5, rgb(170, 178, 196));
        }
        F::Barrel => {
            c.ellipse(9.0, 10.0, 6.0, 7.0, wood);
            c.rect(3, 6, 12, 1, rgb(100, 100, 110));
            c.rect(3, 13, 12, 1, rgb(100, 100, 110));
        }
        F::Banner => {
            c.rect(4, 2, 10, 1, wood);
            c.rect(5, 3, 8, 11, rgb(70, 130, 200));
            c.poly(&[(5.0, 14.0), (9.0, 11.0), (13.0, 14.0)], rgb(0, 0, 0));
            c.disc(9.0, 7.0, 2.0, rgb(240, 240, 250));
        }
        F::Lamp => {
            c.rect(8, 6, 2, 10, rgb(80, 80, 90));
            c.disc(9.0, 4.0, 3.5, rgb(255, 220, 130));
        }
        F::Trophy => {
            c.rect(6, 13, 6, 3, rgb(120, 80, 50));
            c.rect(8, 9, 2, 4, rgb(240, 200, 70));
            c.disc(9.0, 6.0, 4.0, rgb(240, 200, 70));
            c.disc(9.0, 6.0, 1.8, rgb(236, 60, 60));
        }
    }
}

fn material(c: &mut Canvas, id: &str) {
    match id {
        "sturdy_branch" => {
            c.taper((3.0, 15.0), (15.0, 3.0), 1.8, 1.2, rgb(140, 96, 56));
            c.line(9, 9, 12, 12, rgb(140, 96, 56));
            c.disc(13.0, 12.5, 1.4, rgb(100, 170, 80));
        }
        "seasoned_plank" => {
            c.rect(2, 6, 14, 6, rgb(190, 140, 90));
            c.line(3, 8, 14, 8, rgb(150, 104, 62));
            c.line(4, 10, 12, 10, rgb(150, 104, 62));
        }
        "iron_ingot" | "storm_alloy" => {
            let col = if id == "iron_ingot" { rgb(170, 176, 190) } else { rgb(120, 150, 220) };
            c.poly(&[(2.0, 13.0), (5.0, 7.0), (15.0, 7.0), (16.0, 13.0)], col);
            c.rect(5, 7, 10, 2, light(col, 0.35));
            if id == "storm_alloy" {
                c.line(7, 10, 11, 10, rgb(250, 230, 90));
            }
        }
        "wool_cloth" | "silk_bolt" => {
            let col = if id == "wool_cloth" { rgb(236, 228, 214) } else { rgb(90, 66, 140) };
            c.rect(3, 4, 12, 10, col);
            c.rect(3, 6, 12, 1, shadow(col, 0.2));
            c.rect(3, 9, 12, 1, shadow(col, 0.2));
            c.rect(3, 12, 12, 1, shadow(col, 0.2));
        }
        "gale_shard" | "crimson_core" => {
            let col = if id == "gale_shard" { rgb(250, 210, 80) } else { rgb(236, 56, 56) };
            c.poly(&[(9.0, 1.0), (14.0, 9.0), (9.0, 17.0), (4.0, 9.0)], col);
            c.poly(&[(9.0, 3.0), (11.0, 9.0), (9.0, 14.0)], light(col, 0.5));
        }
        "wild_herbs" => {
            for (x, y) in [(6.0, 8.0), (9.0, 6.0), (12.0, 8.0), (9.0, 10.0)] {
                c.disc(x, y, 2.6, rgb(90, 170, 80));
            }
            c.line(9, 10, 9, 16, rgb(70, 120, 60));
        }
        "game_meat" => {
            c.ellipse(8.0, 9.0, 6.0, 4.5, rgb(200, 90, 90));
            c.ellipse(8.0, 9.0, 3.0, 2.0, rgb(240, 160, 160));
            c.rect(13, 11, 4, 2, rgb(240, 236, 226));
        }
        "speckled_egg" => {
            c.ellipse(9.0, 10.0, 4.5, 6.0, rgb(240, 232, 214));
            c.set(7, 8, rgb(160, 120, 90));
            c.set(10, 11, rgb(160, 120, 90));
            c.set(8, 13, rgb(160, 120, 90));
        }
        "glowcap" => {
            c.ellipse(9.0, 7.0, 6.0, 3.5, rgb(120, 220, 200));
            c.rect(8, 8, 3, 7, rgb(230, 220, 200));
            c.set(7, 6, rgb(220, 255, 250));
        }
        "crab_meat" => {
            c.taper((4.0, 14.0), (13.0, 5.0), 3.0, 2.0, rgb(240, 150, 120));
            c.disc(13.0, 5.0, 2.0, rgb(250, 240, 230));
        }
        "sky_honey" => {
            c.rect(5, 6, 8, 10, rgb(240, 180, 50));
            c.rect(5, 4, 8, 2, rgb(150, 100, 60));
            c.rect(6, 8, 2, 5, rgb(255, 230, 140));
        }
        "flour" => {
            c.ellipse(9.0, 11.0, 6.0, 5.5, rgb(236, 226, 200));
            c.rect(7, 4, 4, 3, rgb(200, 180, 140));
        }
        "fresh_milk" => {
            c.rect(5, 6, 8, 10, rgb(250, 250, 250));
            c.rect(6, 3, 6, 3, rgb(80, 130, 210));
        }
        "spice_pouch" => {
            c.ellipse(9.0, 11.0, 5.5, 5.0, rgb(190, 90, 60));
            c.rect(7, 4, 4, 3, rgb(150, 70, 50));
            c.set(9, 11, rgb(250, 210, 120));
        }
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
