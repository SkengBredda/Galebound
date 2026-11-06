//! 0.15.0 monster art: the Dustwind and Highlands monsters, the dungeon
//! flyers and the new bosses. Same conventions as `art`: facing right, feet
//! at the bottom centre, an ink outline. Variants are drawn with these and
//! tinted (see `EnemyKind::tint`), so the base colours stay fairly neutral.

use crate::pixel::{light, rgb, shadow, Canvas, Layer, Px, INK};

fn outlined(w: i32, h: i32, f: impl FnOnce(&mut Canvas)) -> Canvas {
    let mut l = Layer::new(w, h);
    f(&mut l.c);
    let mut out = Canvas::new(w, h);
    l.stamp(&mut out, INK);
    out
}

fn white_if(flash: bool, c: Px) -> Px {
    if flash {
        rgb(255, 255, 255)
    } else {
        c
    }
}

fn eyes(c: &mut Canvas, x: i32, y: i32, gap: i32, h: i32, col: Px) {
    c.rect(x, y, 2, h, col);
    c.rect(x + gap, y, 2, h, col);
}

/// A hopping cactus with a flower on its head.
pub fn cactling(frame: i32, flash: bool) -> Canvas {
    let (w, h) = (44, 50);
    let g = white_if(flash, rgb(96, 168, 84));
    let sq = [0.0, 1.5, 0.0, -1.5][(frame & 3) as usize];
    let mut out = outlined(w, h, |c| {
        c.capsule((22.0, 46.0), (22.0, 16.0 + sq), 10.0, g);
        c.capsule((12.0, 30.0), (8.0, 20.0 + sq), 3.5, g);
        c.capsule((32.0, 26.0), (36.0, 16.0 - sq), 3.5, g);
        c.rect(14, 44, 16, 4, rgb(176, 120, 80));
        c.rim_shade(1, 0, 3, shadow(g, 0.25));
    });
    for (x, y) in [(18, 22), (26, 28), (20, 36), (27, 40), (16, 30)] {
        out.set(x, y + sq as i32, rgb(240, 236, 200));
    }
    out.disc(22.0, 8.0 + sq, 4.0, rgb(240, 90, 120));
    out.disc(22.0, 8.0 + sq, 1.6, rgb(255, 220, 100));
    eyes(&mut out, 22, 22 + sq as i32, 5, 3, INK);
    out.rect(23, 28 + sq as i32, 4, 1, INK);
    out
}

/// A scuttling scorpion. state: 0 walk, 1 telegraph (tail raised), 2 lunge.
pub fn scorpion(frame: i32, state: i32, flash: bool) -> Canvas {
    let (w, h) = (68, 44);
    let body = white_if(flash, rgb(196, 150, 96));
    let dark = shadow(body, 0.3);
    let step = if state == 0 { [2.0, 0.0, -2.0, 0.0][(frame & 3) as usize] } else { 0.0 };
    let mut out = Canvas::new(w, h);
    let legs = outlined(w, h, |c| {
        for i in 0..4 {
            let x = 22.0 + i as f32 * 7.0;
            let s = if i % 2 == 0 { step } else { -step };
            c.capsule((x, 32.0), (x - 6.0 + s, 41.0), 1.4, dark);
        }
    });
    out.draw(&legs, 0, 0);
    let shell = outlined(w, h, |c| {
        c.ellipse(34.0, 31.0, 17.0, 7.0, body);
        // claws
        let reach = if state == 2 { 8.0 } else { 0.0 };
        c.capsule((48.0, 30.0), (56.0 + reach, 27.0), 2.4, body);
        c.ellipse(60.0 + reach, 25.0, 5.0, 3.5, body);
        c.capsule((47.0, 33.0), (55.0 + reach, 35.0), 2.0, body);
        // tail arching over the back
        let lift = if state == 1 { 6.0 } else { 0.0 };
        let pts = [(18.0, 30.0), (10.0, 24.0), (9.0, 15.0 - lift), (15.0, 8.0 - lift), (24.0, 8.0 - lift)];
        for wnd in pts.windows(2) {
            c.capsule(wnd[0], wnd[1], 3.4, body);
        }
        c.rim_shade(0, 1, 2, dark);
    });
    out.draw(&shell, 0, 0);
    let lift = if state == 1 { 6.0 } else { 0.0 };
    out.poly(&[(23.0, 4.0 - lift), (30.0, 9.0 - lift), (23.0, 12.0 - lift)], rgb(250, 230, 160));
    for i in 0..3 {
        out.line(26 + i * 7, 26, 26 + i * 7, 36, dark);
    }
    eyes(&mut out, 44, 27, 3, 2, INK);
    out
}

/// A shaggy white yeti. state: 0 walk, 1 telegraph (arms up), 2 lunge.
pub fn yeti(frame: i32, state: i32, flash: bool) -> Canvas {
    let (w, h) = (64, 70);
    let fur = white_if(flash, rgb(236, 240, 250));
    let skin = rgb(120, 150, 200);
    let step = if state == 0 { [3.0, 0.0, -3.0, 0.0][(frame & 3) as usize] } else { 0.0 };
    let mut out = Canvas::new(w, h);
    let legs = outlined(w, h, |c| {
        c.capsule((24.0, 52.0), (22.0 + step, 66.0), 6.0, shadow(fur, 0.12));
        c.capsule((40.0, 52.0), (42.0 - step, 66.0), 6.0, shadow(fur, 0.12));
    });
    out.draw(&legs, 0, 0);
    let up = state == 1;
    let lunge = if state == 2 { 6.0 } else { 0.0 };
    let body = outlined(w, h, |c| {
        c.ellipse(32.0 + lunge * 0.5, 40.0, 18.0, 17.0, fur);
        c.disc(36.0 + lunge, 20.0, 11.0, fur);
        // horns
        c.taper((30.0 + lunge, 12.0), (24.0 + lunge, 4.0), 3.0, 1.0, rgb(230, 220, 190));
        c.taper((42.0 + lunge, 12.0), (48.0 + lunge, 4.0), 3.0, 1.0, rgb(230, 220, 190));
        // arms
        let (ay, by) = if up { (10.0, 12.0) } else { (50.0, 52.0) };
        c.capsule((18.0, 36.0), (12.0, ay), 5.0, fur);
        c.capsule((46.0 + lunge, 36.0), (54.0 + lunge, by), 5.0, fur);
        for x in [16, 22, 28, 34, 40, 46] {
            c.set(x, 56, shadow(fur, 0.2));
        }
        c.rim_shade(0, 1, 3, shadow(fur, 0.18));
    });
    out.draw(&body, 0, 0);
    out.ellipse(39.0 + lunge, 22.0, 7.0, 6.0, skin);
    eyes(&mut out, 37 + lunge as i32, 20, 4, 2, INK);
    out.rect(38 + lunge as i32, 25, 5, if up { 2 } else { 1 }, INK);
    out
}

/// A stone golem with glowing runes. `big` draws a boss-sized one.
pub fn golem(frame: i32, state: i32, flash: bool, big: bool, enraged: bool) -> Canvas {
    let s = if big { 2.0 } else { 1.0 };
    let (w, h) = ((66.0 * s) as i32, (74.0 * s) as i32);
    let stone = white_if(flash, rgb(160, 176, 196));
    let glow = if enraged { rgb(255, 120, 90) } else { rgb(140, 230, 255) };
    let step = if state == 0 { [3.0, 0.0, -3.0, 0.0][(frame & 3) as usize] * s } else { 0.0 };
    let shake = if state == 1 { ((frame % 2) as f32 * 2.0 - 1.0) * s } else { 0.0 };
    let lunge = if state == 2 { 6.0 * s } else { 0.0 };
    let p = |x: f32, y: f32| (x * s + shake + lunge, y * s);
    let mut out = Canvas::new(w, h);
    let legs = outlined(w, h, |c| {
        c.rect(((20.0 + step / s) * s) as i32, (54.0 * s) as i32, (10.0 * s) as i32, (18.0 * s) as i32, shadow(stone, 0.2));
        c.rect(((36.0 - step / s) * s) as i32, (54.0 * s) as i32, (10.0 * s) as i32, (18.0 * s) as i32, shadow(stone, 0.2));
    });
    out.draw(&legs, 0, 0);
    let body = outlined(w, h, |c| {
        let (a, b) = (p(14.0, 24.0), p(52.0, 58.0));
        c.poly(&[(a.0, a.1), (b.0, a.1), (b.0 - 4.0 * s, b.1), (a.0 + 4.0 * s, b.1)], stone);
        let hd = p(34.0, 16.0);
        c.rect((hd.0 - 9.0 * s) as i32, (hd.1 - 10.0 * s) as i32, (18.0 * s) as i32, (16.0 * s) as i32, stone);
        // boulder fists
        let arm_y = if state == 1 { 20.0 } else { 44.0 };
        let l = p(8.0, arm_y);
        let r = p(60.0, arm_y);
        c.capsule(p(14.0, 28.0), l, 5.0 * s, shadow(stone, 0.1));
        c.capsule(p(52.0, 28.0), r, 5.0 * s, shadow(stone, 0.1));
        c.disc(l.0, l.1, 7.0 * s, stone);
        c.disc(r.0, r.1, 7.0 * s, stone);
        // crystal shoulders
        c.poly(&[p(12.0, 26.0), p(16.0, 12.0), p(22.0, 24.0)], light(glow, 0.2));
        c.poly(&[p(44.0, 24.0), p(50.0, 12.0), p(54.0, 26.0)], light(glow, 0.2));
        c.rim_shade(0, 1, (3.0 * s) as i32, shadow(stone, 0.28));
    });
    out.draw(&body, 0, 0);
    // runes and eyes
    let e = p(36.0, 13.0);
    out.rect(e.0 as i32, e.1 as i32, (3.0 * s) as i32, (2.0 * s) as i32, glow);
    out.rect((e.0 + 5.0 * s) as i32, e.1 as i32, (3.0 * s) as i32, (2.0 * s) as i32, glow);
    let r0 = p(30.0, 34.0);
    let r1 = p(36.0, 44.0);
    out.line(r0.0 as i32, r0.1 as i32, r1.0 as i32, r1.1 as i32, glow);
    out.line(r1.0 as i32, r0.1 as i32, r0.0 as i32, r1.1 as i32, glow);
    out.disc(p(33.0, 39.0).0, p(33.0, 39.0).1, 2.0 * s, light(glow, 0.3));
    out
}

/// A fire lizard. state: 0 walk, 1 telegraph (mouth open).
pub fn salamander(frame: i32, state: i32, flash: bool) -> Canvas {
    let (w, h) = (68, 40);
    let skin = white_if(flash, rgb(214, 110, 70));
    let crest = rgb(255, 196, 90);
    let step = [2.0, 0.0, -2.0, 0.0][(frame & 3) as usize];
    let sway = (frame as f32 * 1.4).sin() * 2.0;
    let mut out = Canvas::new(w, h);
    let legs = outlined(w, h, |c| {
        for (x, s) in [(22.0f32, step), (40.0, -step)] {
            c.capsule((x, 30.0), (x - 4.0 + s, 37.0), 2.0, shadow(skin, 0.2));
            c.capsule((x + 6.0, 30.0), (x + 9.0 - s, 37.0), 2.0, shadow(skin, 0.1));
        }
    });
    out.draw(&legs, 0, 0);
    let body = outlined(w, h, |c| {
        c.taper((14.0, 28.0), (2.0, 24.0 + sway), 5.0, 1.0, skin);
        c.ellipse(32.0, 27.0, 18.0, 6.0, skin);
        c.ellipse(53.0, 23.0, 9.0, 6.0, skin);
        c.rim_shade(0, 1, 2, shadow(skin, 0.25));
    });
    out.draw(&body, 0, 0);
    for i in 0..6 {
        let x = 18.0 + i as f32 * 6.0;
        out.poly(&[(x - 2.0, 22.0), (x + 2.0, 22.0), (x, 15.0 - (i % 2) as f32 * 3.0)], crest);
    }
    eyes(&mut out, 55, 20, 3, 2, rgb(255, 240, 120));
    if state == 1 {
        out.rect(57, 25, 7, 3, rgb(255, 230, 120));
    } else {
        out.rect(56, 26, 6, 1, INK);
    }
    out
}

/// A bat. state: 0 flap, 1 telegraph (screech), 2 dive (wings back).
pub fn bat(frame: i32, state: i32, flash: bool) -> Canvas {
    let (w, h) = (60, 44);
    let fur = white_if(flash, rgb(150, 190, 230));
    let wing = shadow(fur, 0.2);
    let up = match state {
        2 => -10.0,
        _ => [10.0, 2.0, -6.0, 2.0][(frame & 3) as usize],
    };
    let mut out = outlined(w, h, |c| {
        // wings
        c.poly(&[(30.0, 20.0), (4.0, 18.0 - up), (10.0, 26.0), (16.0, 24.0), (22.0, 30.0)], wing);
        c.poly(&[(30.0, 20.0), (56.0, 18.0 - up), (50.0, 26.0), (44.0, 24.0), (38.0, 30.0)], wing);
        c.ellipse(30.0, 24.0, 8.0, 9.0, fur);
        c.poly(&[(25.0, 17.0), (27.0, 8.0), (30.0, 16.0)], fur);
        c.poly(&[(31.0, 16.0), (34.0, 8.0), (36.0, 17.0)], fur);
        c.rim_shade(0, 1, 2, shadow(fur, 0.25));
    });
    eyes(&mut out, 27, 21, 5, 2, if state == 1 { rgb(255, 80, 60) } else { rgb(255, 230, 120) });
    out.poly(&[(28.0, 28.0), (30.0, 31.0), (32.0, 28.0)], rgb(255, 255, 255));
    out
}

/// A serpentine wyrm with wings (Glacia; tinted for Ignaroth).
pub fn wyrm(frame: i32, state: i32, flash: bool, enraged: bool) -> Canvas {
    let (w, h) = (160, 104);
    let scale = white_if(flash, if enraged { rgb(170, 200, 246) } else { rgb(200, 226, 250) });
    let belly = light(scale, 0.3);
    let wingc = shadow(scale, 0.18);
    let f = frame as f32;
    let flap = (f * 1.5).sin() * 10.0;
    let mouth = matches!(state, 1 | 2);
    let mut out = Canvas::new(w, h);
    let wings = outlined(w, h, |c| {
        c.poly(&[(70.0, 44.0), (40.0, 6.0 - flap), (58.0, 22.0), (66.0, 12.0 - flap * 0.6), (84.0, 40.0)], wingc);
        for k in [48.0f32, 60.0] {
            c.line(70, 44, k as i32, (14.0 - flap * 0.8) as i32, shadow(wingc, 0.3));
        }
    });
    out.draw(&wings, 0, 0);
    let body = outlined(w, h, |c| {
        // tail curling back
        let pts = [(8.0, 70.0 + (f).sin() * 4.0), (22.0, 76.0), (40.0, 72.0), (58.0, 64.0)];
        for wnd in pts.windows(2) {
            c.capsule(wnd[0], wnd[1], 6.0, scale);
        }
        c.ellipse(80.0, 62.0, 30.0, 18.0, scale);
        c.ellipse(80.0, 70.0, 22.0, 9.0, belly);
        // neck and head
        c.capsule((100.0, 56.0), (124.0, 38.0), 9.0, scale);
        c.ellipse(134.0, 34.0, 15.0, 10.0, scale);
        c.ellipse(146.0, if mouth { 40.0 } else { 37.0 }, 8.0, if mouth { 5.0 } else { 4.0 }, scale);
        // horns
        c.taper((128.0, 26.0), (118.0, 12.0), 3.0, 1.0, rgb(240, 240, 255));
        c.taper((136.0, 25.0), (132.0, 10.0), 3.0, 1.0, rgb(240, 240, 255));
        // legs
        c.capsule((70.0, 74.0), (66.0, 96.0), 5.0, shadow(scale, 0.12));
        c.capsule((92.0, 74.0), (96.0, 96.0), 5.0, shadow(scale, 0.12));
        c.rim_shade(0, 1, 3, shadow(scale, 0.25));
    });
    out.draw(&body, 0, 0);
    // spines along the back
    for i in 0..7 {
        let x = 58.0 + i as f32 * 9.0;
        let y = 46.0 - (i as f32 - 3.0).abs() * 1.2;
        out.poly(&[(x - 3.0, y + 2.0), (x + 3.0, y + 2.0), (x, y - 7.0)], rgb(240, 250, 255));
    }
    eyes(&mut out, 136, 30, 4, 2, if enraged { rgb(255, 90, 70) } else { rgb(110, 220, 255) });
    if mouth {
        out.disc(154.0, 40.0, 4.0, rgb(220, 250, 255));
    }
    out
}

/// A floating armoured seraph with a halo (the Aether Twins).
pub fn seraph(frame: i32, state: i32, flash: bool, enraged: bool) -> Canvas {
    let (w, h) = (100, 112);
    let gold = white_if(flash, rgb(240, 206, 110));
    let cloth = rgb(250, 248, 236);
    let f = frame as f32;
    let bob = (f * 1.2).sin() * 2.0;
    let mut out = Canvas::new(w, h);
    let wings = outlined(w, h, |c| {
        let spread = if state == 1 { 10.0 } else { (f * 1.4).sin() * 4.0 };
        for s in [-1.0f32, 1.0] {
            let base = (50.0 + s * 8.0, 44.0 + bob);
            c.poly(&[base, (50.0 + s * (40.0 + spread), 18.0 + bob), (50.0 + s * 44.0, 40.0 + bob), (50.0 + s * 36.0, 60.0 + bob)], cloth);
        }
    });
    out.draw(&wings, 0, 0);
    let body = outlined(w, h, |c| {
        // a flowing robe instead of legs
        c.poly(&[(38.0, 50.0 + bob), (62.0, 50.0 + bob), (70.0, 100.0), (30.0, 100.0)], cloth);
        c.rect(40, (40.0 + bob) as i32, 20, 22, gold);
        c.disc(50.0, 30.0 + bob, 9.0, rgb(250, 230, 210));
        // arms
        let up = state == 1;
        c.capsule((40.0, 44.0 + bob), (if up { 30.0 } else { 32.0 }, if up { 24.0 } else { 62.0 } + bob), 3.5, gold);
        c.capsule((60.0, 44.0 + bob), (if up { 70.0 } else { 68.0 }, if up { 24.0 } else { 62.0 } + bob), 3.5, gold);
        // helmet crest
        c.poly(&[(44.0, 24.0 + bob), (50.0, 12.0 + bob), (56.0, 24.0 + bob)], gold);
        c.rim_shade(0, 1, 2, shadow(cloth, 0.2));
    });
    out.draw(&body, 0, 0);
    // halo
    let halo = if enraged { rgb(255, 140, 90) } else { rgb(255, 240, 160) };
    for i in 0..24 {
        let a = i as f32 / 24.0 * std::f32::consts::TAU;
        out.set((50.0 + a.cos() * 13.0) as i32, (12.0 + bob + a.sin() * 3.0) as i32, halo);
    }
    eyes(&mut out, 46, (29.0 + bob) as i32, 5, 2, rgb(60, 140, 220));
    out.rect(46, (48.0 + bob) as i32, 8, 2, light(gold, 0.4));
    out
}

/// One of Glacia's Frost Crystals.
pub fn frost_crystal(frame: i32, flash: bool) -> Canvas {
    let (w, h) = (44, 66);
    let ice = white_if(flash, rgb(170, 220, 255));
    let pulse = (frame as f32 * 1.5).sin() * 1.5;
    let mut out = outlined(w, h, |c| {
        c.poly(&[(22.0, 2.0 - pulse), (32.0, 30.0), (22.0, 62.0), (12.0, 30.0)], ice);
        c.poly(&[(10.0, 20.0), (16.0, 34.0), (8.0, 52.0), (4.0, 34.0)], shadow(ice, 0.1));
        c.poly(&[(34.0, 18.0), (40.0, 34.0), (36.0, 50.0), (28.0, 34.0)], shadow(ice, 0.1));
    });
    out.poly(&[(22.0, 8.0), (26.0, 30.0), (22.0, 50.0)], rgb(240, 250, 255));
    out.disc(22.0, 30.0, 3.0 + pulse.abs(), rgb(255, 255, 255));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every new monster draws something in every frame and state.
    #[test]
    fn new_monster_art_draws() {
        for f in 0..4 {
            for st in 0..3 {
                let arts = [
                    cactling(f, false),
                    scorpion(f, st, false),
                    yeti(f, st, false),
                    golem(f, st, false, false, false),
                    golem(f, st, true, true, true),
                    salamander(f, st.min(1), false),
                    bat(f, st, false),
                    wyrm(f, st, false, st == 2),
                    seraph(f, st, false, false),
                    frost_crystal(f, true),
                ];
                for c in arts {
                    assert!(c.px.iter().filter(|p| p[3] > 0).count() > 300);
                }
            }
        }
    }

    /// Art review sheet: `GB_MONSTERS=m.png cargo test monster_sheet -- --ignored`
    #[test]
    #[ignore]
    fn monster_sheet() {
        let arts = [cactling(0, false), scorpion(0, 1, false), yeti(1, 0, false), golem(0, 0, false, false, false), salamander(0, 1, false), bat(0, 0, false), frost_crystal(0, false), golem(0, 1, false, true, false), wyrm(0, 1, false, false), seraph(0, 1, false, false)];
        let (cw, ch, scale) = (170, 160, 2);
        let w = cw * arts.len() as i32;
        let mut out = vec![0u8; (w * scale * ch * scale * 4) as usize];
        for p in out.chunks_mut(4) {
            p.copy_from_slice(&[120, 160, 200, 255]);
        }
        for (i, a) in arts.iter().enumerate() {
            for y in 0..a.h {
                for x in 0..a.w {
                    let px = a.px[(y * a.w + x) as usize];
                    if px[3] == 0 {
                        continue;
                    }
                    for sy in 0..scale {
                        for sx in 0..scale {
                            let ox = (i as i32 * cw + 4 + x) * scale + sx;
                            let oy = (y + ch - a.h - 2) * scale + sy;
                            let k = ((oy * w * scale + ox) * 4) as usize;
                            out[k..k + 4].copy_from_slice(&px);
                        }
                    }
                }
            }
        }
        let path = std::env::var("GB_MONSTERS").unwrap_or("monsters.png".into());
        std::fs::write(&path, crate::pixel::png_rgba((w * scale) as u32, (ch * scale) as u32, &out)).unwrap();
    }
}
