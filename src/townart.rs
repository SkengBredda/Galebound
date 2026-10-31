//! Pixel art for the class towns and shop interiors: storefronts with
//! doors, inns, class halls, signposts and indoor furniture.

use crate::pixel::{light, rgb, shadow, Canvas, Layer, Px, INK};
use galebound_core::world::Theme;

fn outlined(w: i32, h: i32, f: impl FnOnce(&mut Canvas)) -> Canvas {
    let mut l = Layer::new(w, h);
    f(&mut l.c);
    let mut out = Canvas::new(w, h);
    l.stamp(&mut out, INK);
    out
}

#[derive(Clone, Copy)]
pub enum Roof {
    /// Straight slope with shingles.
    Gable,
    /// Pagoda-like upturned eaves.
    Pagoda,
    /// Mossy leaf thatch.
    Leaf,
    /// Pointed spire.
    Spire,
    /// Flat battlement.
    Battlement,
}

pub struct Palette {
    pub wall: Px,
    pub trim: Px,
    pub roof: Px,
    pub window: Px,
    pub door: Px,
    pub roof_style: Roof,
    /// Stone blocks instead of timber framing.
    pub stone: bool,
}

pub fn palette(theme: Theme) -> Palette {
    match theme {
        Theme::Town => Palette { wall: rgb(178, 176, 182), trim: rgb(108, 104, 118), roof: rgb(78, 96, 146), window: rgb(150, 206, 240), door: rgb(110, 76, 50), roof_style: Roof::Battlement, stone: true },
        Theme::Woods => Palette { wall: rgb(92, 78, 112), trim: rgb(52, 42, 66), roof: rgb(46, 90, 104), window: rgb(255, 196, 110), door: rgb(70, 46, 50), roof_style: Roof::Gable, stone: false },
        Theme::Cliffs => Palette { wall: rgb(236, 220, 196), trim: rgb(176, 58, 54), roof: rgb(62, 112, 92), window: rgb(255, 224, 160), door: rgb(150, 50, 46), roof_style: Roof::Pagoda, stone: false },
        Theme::Glade => Palette { wall: rgb(170, 122, 80), trim: rgb(116, 78, 48), roof: rgb(92, 168, 84), window: rgb(200, 236, 170), door: rgb(110, 70, 42), roof_style: Roof::Leaf, stone: false },
        _ => Palette { wall: rgb(226, 222, 240), trim: rgb(132, 112, 186), roof: rgb(98, 72, 168), window: rgb(140, 236, 255), door: rgb(78, 60, 130), roof_style: Roof::Spire, stone: true },
    }
}

fn roof(c: &mut Canvas, p: &Palette, x0: i32, x1: i32, eave_y: i32, peak: i32) {
    let (x0f, x1f, ey, pk) = (x0 as f32, x1 as f32, eave_y as f32, peak as f32);
    let r = p.roof;
    match p.roof_style {
        Roof::Gable => {
            c.poly(&[(x0f - 10.0, ey), (x1f + 10.0, ey), (x1f - 30.0, pk), (x0f + 30.0, pk)], r);
            let mut y = pk as i32 + 8;
            while y < eave_y - 2 {
                c.line(x0 + 10 - (y - peak) / 3, y, x1 - 10 + (y - peak) / 3, y, shadow(r, 0.22));
                y += 9;
            }
        }
        Roof::Pagoda => {
            let mid = (x0f + x1f) * 0.5;
            c.poly(&[(x0f - 18.0, ey - 6.0), (x0f - 4.0, ey), (x1f + 4.0, ey), (x1f + 18.0, ey - 6.0), (x1f - 20.0, pk + 12.0), (x0f + 20.0, pk + 12.0)], r);
            c.poly(&[(x0f + 14.0, pk + 14.0), (x1f - 14.0, pk + 14.0), (mid + 18.0, pk), (mid - 18.0, pk)], light(r, 0.1));
            c.rect(x0 - 4, eave_y - 2, x1 - x0 + 8, 3, shadow(r, 0.3));
            c.rect(mid as i32 - 2, peak - 10, 4, 12, p.trim);
        }
        Roof::Leaf => {
            let mid = (x0f + x1f) * 0.5;
            let half = (x1f - x0f) * 0.5 + 12.0;
            c.ellipse(mid, ey - (ey - pk) * 0.45, half, (ey - pk) * 0.6, r);
            for i in 0..9 {
                let x = x0f - 6.0 + i as f32 * (x1f - x0f + 12.0) / 8.0;
                c.disc(x, ey - 2.0, 9.0, shadow(r, 0.12));
            }
            for i in 0..5 {
                c.disc(mid - half * 0.6 + i as f32 * half * 0.3, pk + 10.0, 4.0, light(r, 0.25));
            }
        }
        Roof::Spire => {
            let mid = (x0f + x1f) * 0.5;
            c.poly(&[(x0f - 8.0, ey), (x1f + 8.0, ey), (mid, pk - 20.0)], r);
            c.poly(&[(mid, pk - 20.0), (x1f + 8.0, ey), (mid + 6.0, ey)], shadow(r, 0.2));
            c.disc(mid, pk - 24.0, 5.0, rgb(160, 240, 255));
        }
        Roof::Battlement => {
            c.rect(x0 - 8, eave_y - 18, x1 - x0 + 16, 18, p.trim);
            let mut x = x0 - 8;
            while x < x1 + 8 {
                c.rect(x, eave_y - 28, 12, 10, p.trim);
                x += 20;
            }
            c.poly(&[(x0f + 20.0, ey - 18.0), (x1f - 20.0, ey - 18.0), ((x0f + x1f) * 0.5, pk)], r);
        }
    }
}

fn walls(c: &mut Canvas, p: &Palette, x0: i32, x1: i32, top: i32, bottom: i32) {
    c.rect(x0, top, x1 - x0, bottom - top, p.wall);
    if p.stone {
        let mut row = 0;
        let mut y = top + 8;
        while y < bottom {
            c.line(x0, y, x1 - 1, y, shadow(p.wall, 0.16));
            let off = if row % 2 == 0 { 0 } else { 11 };
            let mut x = x0 + off;
            while x < x1 {
                c.line(x, y - 8, x, y, shadow(p.wall, 0.16));
                x += 22;
            }
            y += 9;
            row += 1;
        }
    } else {
        let beam = p.trim;
        c.rect(x0, top, x1 - x0, 4, beam);
        c.rect(x0, top + (bottom - top) / 2, x1 - x0, 3, beam);
        let mut x = x0;
        while x <= x1 - 4 {
            c.rect(x, top, 4, bottom - top, beam);
            x += (x1 - x0 - 4) / 3;
        }
    }
    c.rim_shade(0, 1, 3, shadow(p.wall, 0.2));
}

fn door(c: &mut Canvas, p: &Palette, cx: i32, bottom: i32, w: i32, h: i32) {
    // Open doorway with warm light inside, so it reads as enterable.
    c.rect(cx - w / 2 - 3, bottom - h - 3, w + 6, h + 3, p.trim);
    c.disc(cx as f32, (bottom - h) as f32, (w / 2 + 3) as f32, p.trim);
    c.rect(cx - w / 2, bottom - h, w, h, rgb(255, 214, 140));
    c.disc(cx as f32, (bottom - h) as f32, (w / 2) as f32, rgb(255, 214, 140));
    c.rect(cx - w / 2, bottom - h / 3, w, h / 3, rgb(240, 180, 110));
    // the open door leaf
    c.rect(cx - w / 2, bottom - h, w / 4, h, p.door);
}

fn window(c: &mut Canvas, p: &Palette, x: i32, y: i32, w: i32, h: i32) {
    c.rect(x - 2, y - 2, w + 4, h + 4, p.trim);
    c.rect(x, y, w, h, p.window);
    c.rect(x + w / 2 - 1, y, 2, h, light(p.window, 0.5));
    c.rect(x, y + h / 2 - 1, w, 2, light(p.window, 0.5));
    c.rect(x + 2, y + 2, 3, 2, rgb(255, 255, 255));
}

/// Hanging sign icon: 0 potion, 1 sword, 2 shield, 3 bed, 4 class crest.
fn sign_icon(c: &mut Canvas, kind: u8, cx: i32, cy: i32, accent: Px) {
    c.rect(cx - 18, cy - 12, 36, 24, rgb(126, 84, 52));
    c.rect(cx - 16, cy - 10, 32, 20, rgb(236, 210, 160));
    match kind {
        0 => {
            c.disc(cx as f32, cy as f32 + 2.0, 6.0, rgb(220, 64, 84));
            c.rect(cx - 2, cy - 7, 4, 4, rgb(200, 200, 216));
            c.set(cx - 2, cy, rgb(255, 180, 190));
        }
        1 => {
            c.linef((cx as f32 - 8.0, cy as f32 + 7.0), (cx as f32 + 8.0, cy as f32 - 7.0), rgb(200, 210, 224));
            c.linef((cx as f32 - 7.0, cy as f32 + 7.0), (cx as f32 + 8.0, cy as f32 - 6.0), rgb(150, 160, 180));
            c.line(cx - 7, cy + 1, cx - 1, cy + 7, rgb(160, 110, 50));
        }
        2 => {
            c.poly(&[(cx as f32 - 7.0, cy as f32 - 7.0), (cx as f32 + 7.0, cy as f32 - 7.0), (cx as f32 + 7.0, cy as f32), (cx as f32, cy as f32 + 8.0), (cx as f32 - 7.0, cy as f32)], rgb(90, 120, 180));
            c.rect(cx - 1, cy - 6, 2, 12, rgb(230, 200, 90));
        }
        3 => {
            c.rect(cx - 9, cy, 18, 5, rgb(180, 70, 70));
            c.rect(cx - 9, cy - 3, 6, 3, rgb(250, 250, 250));
            c.rect(cx - 10, cy - 5, 2, 12, rgb(126, 84, 52));
        }
        _ => {
            c.disc(cx as f32, cy as f32, 7.0, accent);
            c.disc(cx as f32, cy as f32, 3.0, light(accent, 0.6));
        }
    }
}

fn building(theme: Theme, w: i32, h: i32, wall_top: i32, sign: u8, accent: Px, extra: impl FnOnce(&mut Canvas, &Palette)) -> Canvas {
    let p = palette(theme);
    let mut out = Canvas::new(w, h);
    let (x0, x1) = (20, w - 20);
    let bottom = h - 2;
    let body = outlined(w, h, |c| {
        walls(c, &p, x0, x1, wall_top, bottom);
    });
    out.draw(&body, 0, 0);
    let rf = outlined(w, h, |c| roof(c, &p, x0, x1, wall_top + 4, 14));
    out.draw(&rf, 0, 0);
    let det = outlined(w, h, |c| {
        let cx = w / 2;
        door(c, &p, cx, bottom, 30, 44);
        let wy = wall_top + (bottom - wall_top) / 2 - 6;
        window(c, &p, x0 + 16, wy, 24, 20);
        window(c, &p, x1 - 40, wy, 24, 20);
        // sign hanging above the door
        c.rect(cx - 1, wall_top + 2, 2, 8, rgb(70, 60, 60));
        sign_icon(c, sign, cx, wall_top + 22, accent);
        extra(c, &p);
    });
    out.draw(&det, 0, 0);
    out
}

/// Storefront: kind 0 general, 1 weapons, 2 armour.
pub fn storefront(theme: Theme, kind: u8) -> Canvas {
    let accent = [rgb(220, 64, 84), rgb(200, 210, 224), rgb(90, 120, 180)][kind as usize % 3];
    building(theme, 190, 176, 72, kind, accent, move |c, p| {
        // awning over the shop window
        let stripe = [rgb(200, 70, 70), rgb(70, 110, 170), rgb(70, 150, 90)][kind as usize % 3];
        for i in 0..8 {
            let col = if i % 2 == 0 { stripe } else { rgb(244, 236, 220) };
            c.rect(20 + i * 19, 58, 19, 10, col);
        }
        c.rect(20, 68, 150, 2, shadow(p.trim, 0.2));
        // goods out front
        match kind {
            0 => {
                c.rect(28, 160, 18, 14, rgb(150, 100, 60));
                c.disc(34.0, 158.0, 3.0, rgb(220, 64, 84));
                c.disc(41.0, 158.0, 3.0, rgb(80, 120, 230));
            }
            1 => {
                c.rect(150, 140, 3, 34, rgb(120, 80, 50));
                c.linef((146.0, 128.0), (146.0, 170.0), rgb(200, 210, 224));
                c.linef((156.0, 132.0), (156.0, 170.0), rgb(200, 210, 224));
            }
            _ => {
                c.rect(32, 138, 4, 36, rgb(120, 80, 50));
                c.rect(26, 140, 16, 18, rgb(150, 156, 170));
                c.disc(34.0, 134.0, 5.0, rgb(150, 156, 170));
            }
        }
    })
}

pub fn inn(theme: Theme) -> Canvas {
    building(theme, 230, 196, 80, 3, rgb(180, 70, 70), |c, _| {
        // flower boxes and a bench
        for x in [40, 166] {
            c.rect(x, 134, 26, 5, rgb(126, 84, 52));
            for k in 0..4 {
                c.disc((x + 4 + k * 6) as f32, 132.0, 2.5, [rgb(250, 120, 150), rgb(255, 220, 90)][k as usize % 2]);
            }
        }
        c.rect(150, 182, 40, 4, rgb(140, 96, 60));
        c.rect(154, 186, 3, 8, rgb(110, 76, 50));
        c.rect(183, 186, 3, 8, rgb(110, 76, 50));
    })
}

pub fn class_hall(theme: Theme, crest: Px) -> Canvas {
    let p = palette(theme);
    let (w, h) = (280, 240);
    let mut out = building(theme, w, h, 96, 4, crest, |_, _| {});
    // pillars and two big banners in the class colour
    let extra = outlined(w, h, |c| {
        for x in [26, w - 36] {
            c.rect(x, 96, 10, h - 98, light(p.wall, 0.15));
            c.rect(x - 3, 92, 16, 6, p.trim);
            c.rect(x - 3, h - 8, 16, 6, p.trim);
        }
        for x in [58, w - 82] {
            c.rect(x, 104, 24, 54, crest);
            c.poly(&[(x as f32, 158.0), (x as f32 + 24.0, 158.0), (x as f32 + 12.0, 168.0)], crest);
            c.disc(x as f32 + 12.0, 124.0, 6.0, light(crest, 0.5));
            c.rect(x - 2, 102, 28, 3, rgb(230, 200, 90));
        }
    });
    out.draw(&extra, 0, 0);
    out
}

pub fn signpost() -> Canvas {
    outlined(60, 110, |c| {
        let wood = rgb(150, 104, 64);
        c.rect(28, 20, 5, 88, wood);
        c.poly(&[(6.0, 22.0), (48.0, 22.0), (56.0, 30.0), (48.0, 38.0), (6.0, 38.0)], light(wood, 0.15));
        c.poly(&[(54.0, 46.0), (12.0, 46.0), (4.0, 54.0), (12.0, 62.0), (54.0, 62.0)], light(wood, 0.1));
        for y in [30, 54] {
            c.line(14, y, 44, y, shadow(wood, 0.35));
        }
        c.rect(22, 104, 17, 4, shadow(wood, 0.3));
    })
}

// =================================================================== interiors

pub fn counter() -> Canvas {
    outlined(150, 36, |c| {
        let wood = rgb(158, 104, 62);
        c.rect(2, 10, 146, 24, wood);
        c.rect(0, 4, 150, 8, light(wood, 0.25));
        for x in [14, 52, 90, 128] {
            c.rect(x, 16, 10, 14, shadow(wood, 0.18));
        }
        // bell and ledger on top
        c.disc(118.0, 2.0, 4.0, rgb(232, 196, 80));
        c.rect(26, 0, 22, 5, rgb(236, 226, 200));
    })
}

pub fn shelf() -> Canvas {
    let mut out = outlined(90, 130, |c| {
        let wood = rgb(140, 92, 56);
        c.rect(2, 2, 86, 126, shadow(wood, 0.25));
        c.rect(0, 0, 90, 6, wood);
        for y in [40, 80, 120] {
            c.rect(0, y, 90, 6, wood);
        }
        c.rect(0, 0, 6, 130, wood);
        c.rect(84, 0, 6, 130, wood);
    });
    let goods = [rgb(220, 64, 84), rgb(80, 120, 230), rgb(120, 200, 110), rgb(240, 200, 80), rgb(190, 110, 220)];
    for (row, &y) in [38i32, 78, 118].iter().enumerate() {
        for k in 0..6usize {
            let x = 12 + k as i32 * 12;
            let col = goods[(k + row * 2) % goods.len()];
            out.rect(x, y - 12, 8, 12, INK);
            out.rect(x + 1, y - 11, 6, 11, col);
            out.rect(x + 2, y - 15, 4, 4, rgb(220, 210, 190));
            out.set(x + 2, y - 9, light(col, 0.6));
        }
    }
    out
}

pub fn weapon_rack() -> Canvas {
    let mut out = outlined(100, 120, |c| {
        let wood = rgb(130, 86, 52);
        c.rect(4, 20, 92, 6, wood);
        c.rect(4, 92, 92, 6, wood);
        c.rect(8, 20, 6, 98, wood);
        c.rect(86, 20, 6, 98, wood);
    });
    let steel = rgb(206, 214, 228);
    let w = outlined(100, 120, |c| {
        // swords, a bow and a staff
        c.rect(24, 10, 4, 84, steel);
        c.rect(18, 70, 16, 3, rgb(200, 160, 70));
        c.rect(44, 14, 4, 80, steel);
        c.rect(38, 72, 16, 3, rgb(200, 160, 70));
        c.capsule((66.0, 10.0), (66.0, 100.0), 1.5, rgb(140, 96, 50));
        c.disc(66.0, 10.0, 5.0, rgb(150, 220, 255));
        c.taper((78.0, 14.0), (84.0, 56.0), 2.0, 2.0, rgb(160, 110, 60));
        c.taper((84.0, 56.0), (78.0, 98.0), 2.0, 2.0, rgb(160, 110, 60));
    });
    out.draw(&w, 0, 0);
    out
}

pub fn armor_stand() -> Canvas {
    outlined(56, 120, |c| {
        let wood = rgb(130, 86, 52);
        let plate = rgb(170, 178, 196);
        c.rect(26, 30, 4, 86, wood);
        c.rect(14, 114, 28, 4, wood);
        c.disc(28.0, 18.0, 10.0, plate);
        c.rect(20, 18, 16, 6, shadow(plate, 0.2));
        c.rect(10, 34, 36, 40, plate);
        c.rect(6, 34, 10, 12, light(plate, 0.15));
        c.rect(40, 34, 10, 12, light(plate, 0.15));
        c.rect(26, 38, 4, 30, rgb(200, 70, 70));
        c.rect(12, 72, 32, 8, shadow(plate, 0.15));
    })
}

pub fn rug() -> Canvas {
    let mut c = Canvas::new(220, 10);
    let red = rgb(160, 54, 64);
    c.rect(0, 2, 220, 8, INK);
    c.rect(1, 3, 218, 6, red);
    c.rect(6, 4, 208, 1, rgb(230, 190, 90));
    c.rect(6, 7, 208, 1, rgb(230, 190, 90));
    c
}

pub fn fireplace(frame: i32) -> Canvas {
    let mut out = outlined(110, 120, |c| {
        let stone = rgb(150, 140, 136);
        c.rect(4, 20, 102, 98, stone);
        c.rect(0, 14, 110, 10, light(stone, 0.2));
        c.rect(28, 56, 54, 62, rgb(40, 28, 30));
        c.disc(55.0, 56.0, 27.0, rgb(40, 28, 30));
        for y in (30..118).step_by(12) {
            c.line(4, y, 105, y, shadow(stone, 0.2));
        }
    });
    let f = frame as f32;
    for i in 0..5 {
        let x = 36.0 + i as f32 * 9.0;
        let ht = 14.0 + ((f * 1.7 + i as f32 * 2.1).sin() * 0.5 + 0.5) * 16.0;
        out.taper((x, 116.0), (x + (f + i as f32).sin() * 3.0, 116.0 - ht), 5.0, 1.0, rgb(255, 150, 60));
        out.taper((x, 116.0), (x, 116.0 - ht * 0.55), 3.0, 1.0, rgb(255, 230, 140));
    }
    out.rect(32, 112, 46, 5, rgb(110, 70, 40));
    out
}

pub fn wall_torch(frame: i32) -> Canvas {
    let mut out = outlined(20, 40, |c| {
        c.rect(8, 16, 4, 22, rgb(110, 76, 50));
        c.rect(5, 14, 10, 4, rgb(80, 80, 92));
    });
    let k = (frame % 4) as f32;
    out.ellipse(10.0, 9.0, 4.0 + k * 0.3, 7.0 - k * 0.4, rgb(255, 160, 60));
    out.ellipse(10.0, 10.0, 2.0, 4.0, rgb(255, 236, 150));
    out
}

pub fn indoor_window() -> Canvas {
    let mut out = outlined(70, 80, |c| {
        let wood = rgb(130, 86, 52);
        c.rect(0, 0, 70, 80, wood);
        c.rect(6, 6, 58, 68, rgb(150, 200, 240));
        c.rect(33, 6, 4, 68, wood);
        c.rect(6, 38, 58, 4, wood);
    });
    out.rect(10, 10, 6, 3, rgb(240, 250, 255));
    out.rect(42, 46, 8, 3, rgb(240, 250, 255));
    out
}

pub fn barrel() -> Canvas {
    outlined(44, 54, |c| {
        let wood = rgb(160, 106, 62);
        c.ellipse(22.0, 28.0, 19.0, 24.0, wood);
        c.rect(4, 12, 36, 4, rgb(100, 100, 110));
        c.rect(4, 40, 36, 4, rgb(100, 100, 110));
        c.rim_shade(1, 0, 4, shadow(wood, 0.25));
        c.ellipse(22.0, 6.0, 15.0, 4.0, light(wood, 0.2));
    })
}

/// The wall behind everything inside a shop (tiles horizontally, 128 px wide).
pub fn interior_wall() -> Canvas {
    let (w, h) = (128, 360);
    let mut c = Canvas::new(w, h);
    let plank = rgb(176, 128, 84);
    c.rect(0, 0, w, h, plank);
    for x in (0..w).step_by(16) {
        c.rect(x, 0, 1, h, shadow(plank, 0.28));
        c.rect(x + 1, 0, 1, h, light(plank, 0.12));
    }
    for y in (0..h).step_by(57) {
        for x in (0..w).step_by(16) {
            c.rect(x + 2, y + (x % 32), 12, 1, shadow(plank, 0.15));
        }
    }
    // wainscot
    c.rect(0, 250, w, 110, rgb(120, 78, 50));
    c.rect(0, 250, w, 4, rgb(96, 60, 38));
    for x in (0..w).step_by(32) {
        c.rect(x + 4, 262, 24, 80, rgb(136, 90, 58));
    }
    // ceiling beam
    c.rect(0, 0, w, 24, rgb(98, 64, 40));
    c.rect(0, 24, w, 3, rgb(60, 40, 28));
    c
}

pub fn interior_floor() -> Canvas {
    let (w, h) = (64, 64);
    let mut c = Canvas::new(w, h);
    let wood = rgb(132, 88, 56);
    c.rect(0, 0, w, h, wood);
    for y in (0..h).step_by(8) {
        c.rect(0, y, w, 1, shadow(wood, 0.3));
        let off = (y / 8 % 2) * 24;
        c.rect(off + 10, y, 1, 8, shadow(wood, 0.3));
        c.rect(off + 40, y, 1, 8, shadow(wood, 0.3));
    }
    c.rect(0, 0, w, 2, light(wood, 0.25));
    c
}

// =================================================================== climbables (tile vertically)

pub fn ladder_tile() -> Canvas {
    let (w, h) = (22, 16);
    let mut c = Canvas::new(w, h);
    let wood = rgb(170, 116, 66);
    for x in [1, 17] {
        c.rect(x, 0, 4, h, INK);
        c.rect(x + 1, 0, 2, h, wood);
        c.rect(x + 1, 0, 1, h, light(wood, 0.2));
    }
    c.rect(4, 6, 14, 4, INK);
    c.rect(4, 7, 14, 2, shadow(wood, 0.05));
    c.rect(4, 7, 14, 1, light(wood, 0.25));
    c
}

pub fn rope_tile() -> Canvas {
    let (w, h) = (8, 12);
    let mut c = Canvas::new(w, h);
    let rope = rgb(206, 170, 110);
    c.rect(2, 0, 4, h, INK);
    c.rect(3, 0, 2, h, rope);
    for y in (0..h).step_by(4) {
        c.set(3, y, shadow(rope, 0.3));
        c.set(4, y + 2, shadow(rope, 0.3));
    }
    c.rect(1, 5, 6, 3, INK);
    c.rect(2, 6, 4, 1, light(rope, 0.1));
    c
}

pub fn vine_tile() -> Canvas {
    let (w, h) = (20, 24);
    let mut c = Canvas::new(w, h);
    let stem = rgb(70, 130, 60);
    let leaf = rgb(110, 186, 80);
    for y in 0..h {
        let x = 9 + ((y as f32 / h as f32 * std::f32::consts::TAU).sin() * 2.0).round() as i32;
        c.rect(x - 1, y, 4, 1, INK);
        c.rect(x, y, 2, 1, stem);
    }
    for (x, y, flip) in [(3, 4, false), (12, 14, true)] {
        let mut l = Canvas::new(w, h);
        l.ellipse(x as f32 + 3.0, y as f32 + 2.0, 3.5, 2.0, leaf);
        l.set(if flip { x + 1 } else { x + 5 }, y + 1, light(leaf, 0.3));
        let o = l.outlined(INK);
        c.draw(&o, 0, 0);
    }
    c
}

pub fn rope_anchor() -> Canvas {
    let mut c = Canvas::new(16, 10);
    c.rect(0, 2, 16, 6, INK);
    c.rect(1, 3, 14, 4, rgb(120, 120, 134));
    c.rect(1, 3, 14, 1, rgb(170, 170, 186));
    c
}

/// Fast-travel waystone: a runed obelisk with a floating crystal (6 frames).
pub fn waystone(frame: i32) -> Canvas {
    let (w, h) = (48, 100);
    let mut out = outlined(w, h, |c| {
        let stone = rgb(150, 156, 176);
        c.rect(8, 92, 32, 6, shadow(stone, 0.2));
        c.poly(&[(14.0, 92.0), (34.0, 92.0), (30.0, 40.0), (18.0, 40.0)], stone);
        c.poly(&[(18.0, 40.0), (30.0, 40.0), (24.0, 32.0)], light(stone, 0.15));
        c.poly(&[(24.0, 40.0), (30.0, 40.0), (34.0, 92.0), (26.0, 92.0)], shadow(stone, 0.12));
    });
    let glow = rgb(130, 230, 255);
    let pulse = [0.0, 0.2, 0.4, 0.5, 0.4, 0.2][frame.rem_euclid(6) as usize];
    for (i, y) in [50, 60, 70, 80].iter().enumerate() {
        let col = if (i as i32 + frame) % 3 == 0 { light(glow, 0.4) } else { glow };
        out.rect(22, *y, 4, 2, col);
        out.set(21 + (i as i32 % 2) * 5, y + 3, col);
    }
    let cy = 18.0 - pulse * 6.0;
    let cr = outlined(w, h, |c| {
        c.poly(&[(24.0, cy - 10.0), (31.0, cy), (24.0, cy + 10.0), (17.0, cy)], glow);
        c.poly(&[(24.0, cy - 10.0), (31.0, cy), (24.0, cy)], light(glow, 0.4));
    });
    out.draw(&cr, 0, 0);
    out
}
