//! Pixel art for everything that isn't a player character: enemies, props,
//! terrain tiles and parallax backgrounds. Pure Rust (CPU canvases) so it
//! can be previewed and tested without a window; textures are made from
//! these once and cached by `sprites.rs`.

use crate::pixel::{light, mix, rgb, shadow, Canvas, Layer, Px, INK};
use galebound_core::world::Theme;

fn hash(x: i32, y: i32, seed: i32) -> f32 {
    let mut v = (x.wrapping_mul(374761393) ^ y.wrapping_mul(668265263) ^ seed.wrapping_mul(1442695041)) as u32;
    v = (v ^ (v >> 13)).wrapping_mul(1274126177);
    v ^= v >> 16;
    (v & 0xFFFF) as f32 / 65535.0
}

fn outlined(w: i32, h: i32, f: impl FnOnce(&mut Canvas)) -> Canvas {
    let mut l = Layer::new(w, h);
    f(&mut l.c);
    let mut out = Canvas::new(w, h);
    l.stamp(&mut out, INK);
    out
}

// =================================================================== enemies
// All facing right; feet at bottom-centre (x = w/2, y = h - 2).

pub fn dummy(wobble: i32) -> Canvas {
    let (w, h) = (36, 58);
    let mut out = Canvas::new(w, h);
    let wood = rgb(140, 92, 54);
    let straw = rgb(226, 196, 128);
    let lean = wobble as f32;
    let post = outlined(w, h, |c| {
        c.rect(16, 30, 4, 26, wood);
        c.rect(9, 53, 18, 3, shadow(wood, 0.2));
    });
    out.draw(&post, 0, 0);
    let body = outlined(w, h, |c| {
        c.ellipse(18.0 + lean, 30.0, 8.0, 11.0, straw);
        c.disc(18.0 + lean * 1.4, 13.0, 6.5, straw);
        c.capsule((8.0 + lean, 24.0), (28.0 + lean, 24.0), 4.0, shadow(straw, 0.15));
        // stitching / rope
        c.line(10 + wobble, 30, 26 + wobble, 30, rgb(160, 110, 70));
        c.rim_shade(-1, 0, 2, shadow(straw, 0.25));
    });
    out.draw(&body, 0, 0);
    // target
    let cx = 18.0 + lean;
    out.disc(cx, 33.0, 4.0, rgb(214, 70, 70));
    out.disc(cx, 33.0, 2.5, rgb(250, 240, 230));
    out.disc(cx, 33.0, 1.2, rgb(214, 70, 70));
    out
}

pub fn mossling(frame: i32, hop: f32, flash: bool) -> Canvas {
    let (w, h) = (40, 34);
    let squash = match frame % 4 { 0 => 0.0, 1 => 1.0, 2 => 0.0, _ => -1.0 } + hop;
    let rx = 14.0 + squash * 1.5;
    let ry = 10.0 - squash * 1.5;
    let body = if flash { rgb(255, 255, 255) } else { rgb(112, 204, 108) };
    let cy = h as f32 - 2.0 - ry;
    let mut out = outlined(w, h, |c| {
        c.ellipse(20.0, cy, rx, ry, body);
        c.rect(20 - rx as i32 + 1, cy as i32, (rx * 2.0) as i32 - 1, ry as i32 + 1, body);
        c.rim_shade(0, 1, 2, shadow(body, 0.3));
        c.rim_shade(-1, 0, 1, shadow(body, 0.18));
        // moss cap
        c.ellipse(18.0, cy - ry + 2.0, rx * 0.75, 4.5, rgb(64, 140, 70));
        c.disc(24.0, cy - ry - 1.0, 2.5, rgb(246, 120, 150));
        c.set(24, (cy - ry - 2.0) as i32, rgb(255, 230, 120));
    });
    // highlight + eyes
    out.ellipse(12.0, cy - 3.0, 2.0, 1.5, light(body, 0.5));
    let ey = (cy + 1.0) as i32;
    for ex in [21, 28] {
        out.rect(ex, ey - 2, 3, 4, INK);
        out.set(ex, ey - 2, rgb(255, 255, 255));
    }
    out.rect(25, ey + 3, 2, 1, shadow(body, 0.5));
    out
}

/// state: 0 walk, 1 telegraph (puffed up), 2 lunge
pub fn burrowbeak(frame: i32, state: i32, flash: bool) -> Canvas {
    let (w, h) = (48, 46);
    let puff = if state == 1 { 2.5 } else { 0.0 };
    let base = if flash { rgb(255, 255, 255) } else if state == 1 { rgb(232, 120, 84) } else { rgb(204, 146, 92) };
    let mut out = Canvas::new(w, h);
    let step = if state == 0 { [3.0, 0.0, -3.0, 0.0][(frame % 4) as usize] } else if state == 2 { 4.0 } else { 0.0 };
    let legs = outlined(w, h, |c| {
        let leg = rgb(236, 176, 64);
        c.capsule((20.0, 30.0), (20.0 + step, 43.0), 2.5, leg);
        c.capsule((26.0, 30.0), (26.0 - step, 43.0), 2.5, shadow(leg, 0.2));
        c.rect(18 + step as i32, 43, 5, 2, leg);
        c.rect(24 - step as i32, 43, 5, 2, shadow(leg, 0.2));
    });
    out.draw(&legs, 0, 0);
    let lean = if state == 2 { 3.0 } else { 0.0 };
    let body = outlined(w, h, |c| {
        c.ellipse(22.0 + lean, 25.0, 12.0 + puff, 10.0 + puff, base);
        // tail feathers
        c.poly(&[(10.0, 20.0), (3.0, 14.0), (5.0, 22.0), (11.0, 25.0)], shadow(base, 0.2));
        // wing
        c.ellipse(18.0 + lean, 26.0, 7.0, 5.0, shadow(base, 0.18));
        c.rim_shade(0, 1, 2, shadow(base, 0.3));
        // head
        c.disc(32.0 + lean * 1.5, 13.0, 7.5, base);
    });
    out.draw(&body, 0, 0);
    let beak = outlined(w, h, |c| {
        let hx = 32.0 + lean * 1.5;
        c.poly(&[(hx + 5.0, 10.0), (hx + 5.0, 16.0), (hx + 16.0, 13.5)], rgb(244, 206, 80));
    });
    out.draw(&beak, 0, 0);
    let hx = (32.0 + lean * 1.5) as i32;
    out.rect(hx + 1, 10, 2, 3, INK);
    out.set(hx + 1, 10, rgb(255, 255, 255));
    // crest
    out.line(hx - 3, 6, hx - 6, 2, INK);
    out.line(hx - 1, 6, hx - 2, 1, INK);
    out
}

/// state: 0 walk, 1 telegraph, 2 charge, 3 stunned, 4 enraged-walk
pub fn bramblehorn(frame: i32, state: i32, flash: bool, enraged: bool) -> Canvas {
    horned_beast(frame, state, flash, enraged, false)
}

/// The Thunderhorn: storm-blue cousin of the Bramblehorn with lightning horns.
pub fn thunderhorn(frame: i32, state: i32, flash: bool, enraged: bool) -> Canvas {
    horned_beast(frame, state, flash, enraged, true)
}

fn horned_beast(frame: i32, state: i32, flash: bool, enraged: bool, storm: bool) -> Canvas {
    let (w, h) = (112, 84);
    let base = if flash {
        rgb(255, 255, 255)
    } else if storm {
        if enraged { rgb(96, 92, 150) } else { rgb(98, 112, 140) }
    } else if enraged {
        rgb(140, 78, 72)
    } else {
        rgb(122, 92, 68)
    };
    let back_col = if storm { rgb(200, 206, 226) } else { rgb(78, 142, 72) };
    let thorn_col = if storm { rgb(244, 220, 90) } else { rgb(96, 168, 82) };
    let mut out = Canvas::new(w, h);
    let speed = if state == 2 { 2.0 } else { 1.0 };
    let ph = frame as f32 * 1.6 * speed;
    let shake = if state == 1 { if frame % 2 == 0 { 1.0 } else { -1.0 } } else { 0.0 };
    let cx = 52.0 + shake;
    // legs
    let legs = outlined(w, h, |c| {
        for (i, lx) in [-24.0f32, -10.0, 14.0, 28.0].iter().enumerate() {
            let swing = if state == 0 || state == 2 || state == 4 { (ph + i as f32 * 1.6).sin() * 4.0 } else { 0.0 };
            let col = if i % 2 == 0 { shadow(base, 0.25) } else { shadow(base, 0.1) };
            c.capsule((cx + lx, 58.0), (cx + lx + swing, 78.0), 7.0, col);
            c.rect((cx + lx + swing) as i32 - 4, 78, 8, 4, rgb(70, 54, 46));
        }
    });
    out.draw(&legs, 0, 0);
    let body = outlined(w, h, |c| {
        c.ellipse(cx, 50.0, 40.0, 22.0, base);
        c.rim_shade(0, 1, 3, shadow(base, 0.3));
        c.rim_shade(-1, 0, 2, shadow(base, 0.2));
        // mossy back (or storm cloud)
        c.ellipse(cx - 2.0, 33.0, 32.0, 8.0, back_col);
        if storm {
            for i in 0..5 {
                c.disc(cx - 26.0 + i as f32 * 13.0, 29.0 - (i % 2) as f32 * 3.0, 6.0, back_col);
            }
        }
    });
    out.draw(&body, 0, 0);
    // thorns along the back
    let thorns = outlined(w, h, |c| {
        for i in 0..8 {
            let tx = cx - 30.0 + i as f32 * 8.5;
            let th = 10.0 + (i % 2) as f32 * 5.0;
            if storm {
                // little lightning zig-zags rising from the cloud
                if i % 2 == 0 {
                    c.poly(&[(tx - 2.0, 28.0), (tx + 1.0, 28.0), (tx + 3.0, 20.0 - th * 0.3), (tx, 21.0 - th * 0.3), (tx + 2.0, 12.0 - th * 0.4), (tx - 3.0, 22.0)], thorn_col);
                }
            } else {
                c.poly(&[(tx - 3.5, 31.0), (tx + 3.5, 31.0), (tx - 3.0, 31.0 - th)], thorn_col);
            }
        }
    });
    out.draw(&thorns, 0, 0);
    // head
    let hx = cx + 38.0;
    let head = outlined(w, h, |c| {
        c.ellipse(hx, 50.0, 16.0, 15.0, light(base, 0.08));
        c.ellipse(hx + 13.0, 56.0, 7.0, 6.0, rgb(214, 156, 146)); // snout
        c.rim_shade(0, 1, 2, shadow(base, 0.25));
    });
    out.draw(&head, 0, 0);
    let horns = outlined(w, h, |c| {
        let bone = if storm { rgb(250, 226, 96) } else { rgb(244, 234, 214) };
        c.taper((hx + 2.0, 38.0), (hx + 18.0, 18.0), 6.0, 1.0, bone);
        c.taper((hx + 8.0, 60.0), (hx + 22.0, 50.0), 4.5, 1.0, bone);
    });
    out.draw(&horns, 0, 0);
    let eye = if state == 1 || state == 2 || enraged { rgb(255, 70, 50) } else { INK };
    out.rect(hx as i32 + 2, 45, 3, 3, eye);
    if state == 1 || state == 2 || enraged {
        out.set(hx as i32 + 2, 45, rgb(255, 230, 160));
    }
    out.set(hx as i32 + 15, 55, INK);
    out.set(hx as i32 + 15, 57, INK);
    if state == 3 {
        for i in 0..3 {
            let a = frame as f32 * 0.8 + i as f32 * 2.1;
            out.disc(hx + a.cos() * 12.0, 24.0 + a.sin() * 3.0, 2.0, rgb(255, 230, 90));
        }
    }
    out
}

// =================================================================== props / decorations
// Anchor: bottom-centre at (w/2, h-1).

pub fn tree_tall(seed: i32) -> Canvas {
    let (w, h) = (120, 190);
    let mut out = Canvas::new(w, h);
    let bark = rgb(126, 86, 58);
    let trunk = outlined(w, h, |c| {
        c.poly(&[(54.0, 189.0), (66.0, 189.0), (64.0, 90.0), (56.0, 90.0)], bark);
        c.poly(&[(46.0, 189.0), (74.0, 189.0), (66.0, 178.0), (54.0, 178.0)], bark);
        c.rim_shade(-1, 0, 3, shadow(bark, 0.3));
        for i in 0..6 {
            let y = 100 + i * 14;
            c.line(60, y, 61, y + 6, shadow(bark, 0.35));
        }
    });
    out.draw(&trunk, 0, 0);
    let leaves = rgb(78, 160, 88);
    let canopy = outlined(w, h, |c| {
        let blobs = [(60.0, 52.0, 34.0), (34.0, 80.0, 26.0), (86.0, 78.0, 28.0), (60.0, 96.0, 30.0), (44.0, 30.0, 20.0), (78.0, 32.0, 22.0)];
        for (x, y, r) in blobs {
            c.disc(x, y, r, leaves);
        }
        // leaf shading: lower right darker, dithered highlight upper left
        c.rim_shade(0, 1, 6, shadow(leaves, 0.28));
        c.rim_shade(1, 0, 3, shadow(leaves, 0.2));
        for y in 0..h {
            for x in 0..w {
                if c.opaque(x, y) && hash(x / 3, y / 3, seed) > 0.86 {
                    c.set(x, y, light(leaves, 0.3));
                }
                if c.opaque(x, y) && hash(x, y, seed + 7) > 0.93 {
                    c.set(x, y, shadow(leaves, 0.3));
                }
            }
        }
    });
    out.draw(&canopy, 0, 0);
    out
}

pub fn tree_round(seed: i32) -> Canvas {
    let (w, h) = (110, 150);
    let mut out = Canvas::new(w, h);
    let bark = rgb(136, 94, 62);
    let trunk = outlined(w, h, |c| {
        c.poly(&[(50.0, 149.0), (60.0, 149.0), (59.0, 80.0), (51.0, 80.0)], bark);
        c.rim_shade(-1, 0, 2, shadow(bark, 0.3));
    });
    out.draw(&trunk, 0, 0);
    let leaves = rgb(110, 190, 98);
    let canopy = outlined(w, h, |c| {
        c.disc(55.0, 58.0, 44.0, leaves);
        c.disc(30.0, 76.0, 22.0, leaves);
        c.disc(82.0, 74.0, 22.0, leaves);
        c.rim_shade(0, 1, 7, shadow(leaves, 0.25));
        c.rim_shade(1, 0, 3, shadow(leaves, 0.15));
        for y in 0..h {
            for x in 0..w {
                if c.opaque(x, y) && hash(x / 3, y / 3, seed) > 0.85 && y < 60 {
                    c.set(x, y, light(leaves, 0.35));
                }
            }
        }
    });
    out.draw(&canopy, 0, 0);
    // blossoms
    for i in 0..8 {
        let x = 20.0 + hash(i, 1, seed) * 70.0;
        let y = 30.0 + hash(i, 2, seed) * 60.0;
        if out.opaque(x as i32, y as i32) {
            out.disc(x, y, 2.0, rgb(246, 120, 150));
            out.set(x as i32, y as i32, rgb(255, 220, 230));
        }
    }
    out
}

pub fn bush() -> Canvas {
    outlined(70, 34, |c| {
        let g = rgb(82, 168, 90);
        c.disc(18.0, 20.0, 13.0, g);
        c.disc(36.0, 15.0, 15.0, g);
        c.disc(54.0, 21.0, 12.0, g);
        c.rect(6, 22, 60, 11, g);
        c.rim_shade(0, 1, 4, shadow(g, 0.3));
        c.ellipse(30.0, 10.0, 6.0, 3.0, light(g, 0.3));
    })
}

pub fn rock() -> Canvas {
    outlined(56, 30, |c| {
        let s = rgb(150, 150, 164);
        c.ellipse(28.0, 18.0, 25.0, 12.0, s);
        c.rect(4, 18, 49, 11, s);
        c.rim_shade(0, 1, 4, shadow(s, 0.3));
        c.ellipse(20.0, 12.0, 9.0, 4.0, light(s, 0.3));
        c.line(30, 12, 36, 20, shadow(s, 0.35));
    })
}

pub fn flowers(seed: i32) -> Canvas {
    let mut out = Canvas::new(44, 16);
    let cols = [rgb(255, 120, 150), rgb(255, 222, 90), rgb(170, 150, 255), rgb(255, 255, 255)];
    for i in 0..5 {
        let x = 4 + i * 8 + (hash(i, 0, seed) * 3.0) as i32;
        let top = 4 + (hash(i, 1, seed) * 6.0) as i32;
        out.line(x, top + 2, x, 15, rgb(70, 140, 64));
        let col = cols[((hash(i, 2, seed) * 4.0) as usize).min(3)];
        out.rect(x - 1, top, 3, 3, col);
        out.set(x, top + 1, rgb(255, 236, 140));
        out.set(x - 1, top - 1, shadow(col, 0.0));
    }
    out.outlined(INK)
}

pub fn fence() -> Canvas {
    outlined(130, 34, |c| {
        let w = rgb(214, 172, 118);
        for i in 0..6 {
            let x = 4 + i * 23;
            c.poly(&[(x as f32, 33.0), (x as f32 + 7.0, 33.0), (x as f32 + 7.0, 6.0), (x as f32 + 3.5, 2.0), (x as f32, 6.0)], w);
        }
        c.rect(0, 11, 130, 4, shadow(w, 0.1));
        c.rect(0, 23, 130, 4, shadow(w, 0.1));
        c.rim_shade(0, 1, 1, shadow(w, 0.3));
    })
}

pub fn tent() -> Canvas {
    let (w, h) = (128, 100);
    let mut out = Canvas::new(w, h);
    let cloth = rgb(228, 206, 160);
    let t = outlined(w, h, |c| {
        c.poly(&[(4.0, 98.0), (124.0, 98.0), (64.0, 20.0)], cloth);
        c.rim_shade(-1, 0, 4, shadow(cloth, 0.2));
        c.poly(&[(46.0, 98.0), (82.0, 98.0), (64.0, 52.0)], rgb(96, 70, 50));
        c.line(64, 20, 64, 98, shadow(cloth, 0.25));
    });
    out.draw(&t, 0, 0);
    let flag = outlined(w, h, |c| {
        c.rect(63, 4, 2, 18, rgb(110, 76, 48));
        c.poly(&[(65.0, 4.0), (82.0, 8.0), (65.0, 13.0)], rgb(220, 76, 76));
    });
    out.draw(&flag, 0, 0);
    out
}

pub fn campfire(frame: i32) -> Canvas {
    let (w, h) = (40, 40);
    let mut out = Canvas::new(w, h);
    let logs = outlined(w, h, |c| {
        c.capsule((6.0, 36.0), (32.0, 32.0), 5.0, rgb(120, 78, 46));
        c.capsule((8.0, 31.0), (34.0, 36.0), 5.0, rgb(140, 92, 54));
    });
    out.draw(&logs, 0, 0);
    let f = frame as f32;
    let flame = outlined(w, h, |c| {
        c.poly(&[(10.0, 32.0), (30.0, 32.0), (22.0 + (f * 1.7).sin() * 2.0, 6.0 + (f % 2.0) * 3.0)], rgb(250, 140, 50));
        c.poly(&[(9.0, 32.0), (18.0, 32.0), (12.0, 16.0 + f % 3.0)], rgb(250, 120, 40));
    });
    out.draw(&flame, 0, 0);
    out.poly(&[(14.0, 32.0), (26.0, 32.0), (20.0 + (f * 2.3).cos(), 16.0 + (f % 2.0) * 2.0)], rgb(255, 228, 120));
    out
}

pub fn house(shop: bool) -> Canvas {
    let (w, h) = (200, 170);
    let mut out = Canvas::new(w, h);
    let wall = if shop { rgb(240, 224, 194) } else { rgb(234, 212, 182) };
    let roof = if shop { rgb(78, 140, 194) } else { rgb(206, 92, 72) };
    let walls = outlined(w, h, |c| {
        c.rect(20, 76, 160, 92, wall);
        // timber frame
        let beam = rgb(126, 84, 52);
        c.rect(20, 76, 160, 4, beam);
        c.rect(20, 120, 160, 3, beam);
        for x in [20, 98, 176] {
            c.rect(x, 76, 4, 92, beam);
        }
        c.line(24, 80, 96, 118, beam);
        c.line(102, 118, 174, 80, beam);
        c.rim_shade(0, 1, 3, shadow(wall, 0.2));
    });
    out.draw(&walls, 0, 0);
    let rf = outlined(w, h, |c| {
        c.poly(&[(6.0, 80.0), (194.0, 80.0), (160.0, 20.0), (40.0, 20.0)], roof);
        // shingles
        for row in 0..6 {
            let y = 26 + row * 10;
            let off = (row % 2) * 8;
            let mut x = 30 - row * 4 + off;
            while x < 172 + row * 4 {
                c.set_on(x, y, shadow(roof, 0.3));
                c.set_on(x, y + 1, shadow(roof, 0.3));
                x += 16;
            }
            c.line(40 - row * 6, y + 8, 160 + row * 6, y + 8, shadow(roof, 0.2));
        }
        c.rect(6, 78, 188, 4, shadow(roof, 0.35));
        // chimney
        c.rect(140, 6, 14, 22, rgb(170, 110, 90));
    });
    out.draw(&rf, 0, 0);
    let details = outlined(w, h, |c| {
        // door
        c.rect(86, 126, 28, 42, rgb(132, 86, 52));
        c.disc(100.0, 126.0, 14.0, rgb(132, 86, 52));
        // windows
        for x in [40, 136] {
            c.rect(x, 128, 26, 22, rgb(140, 204, 240));
            c.rect(x, 150, 26, 3, rgb(150, 100, 60));
        }
    });
    out.draw(&details, 0, 0);
    out.disc(108.0, 146.0, 1.5, rgb(240, 210, 100));
    for x in [40, 136] {
        out.rect(x + 12, 128, 2, 22, rgb(250, 244, 230));
        out.rect(x, 138, 26, 2, rgb(250, 244, 230));
        out.rect(x + 2, 130, 4, 2, rgb(220, 240, 255));
    }
    if shop {
        let sign = outlined(w, h, |c| {
            c.rect(70, 56, 60, 16, rgb(120, 80, 50));
        });
        out.draw(&sign, 0, 0);
        // potion icon on the sign
        out.disc(100.0, 65.0, 4.0, rgb(230, 70, 90));
        out.rect(99, 58, 3, 3, rgb(220, 220, 230));
    }
    out
}

pub fn lamp() -> Canvas {
    let (w, h) = (24, 96);
    let mut out = outlined(w, h, |c| {
        let iron = rgb(64, 64, 78);
        c.rect(10, 20, 4, 74, iron);
        c.rect(6, 92, 12, 3, iron);
        c.rect(4, 4, 16, 18, rgb(255, 228, 150));
        c.rect(3, 2, 18, 3, iron);
        c.rect(3, 21, 18, 2, iron);
    });
    out.rect(6, 8, 3, 10, rgb(255, 250, 220));
    out
}

pub fn well() -> Canvas {
    let (w, h) = (90, 110);
    let mut out = Canvas::new(w, h);
    let posts = outlined(w, h, |c| {
        let wood = rgb(130, 90, 58);
        c.rect(14, 30, 6, 50, wood);
        c.rect(70, 30, 6, 50, wood);
        c.poly(&[(4.0, 34.0), (86.0, 34.0), (45.0, 6.0)], rgb(204, 92, 72));
        c.line(45, 34, 45, 60, rgb(90, 70, 50));
        c.rect(40, 58, 10, 9, wood);
    });
    out.draw(&posts, 0, 0);
    let base = outlined(w, h, |c| {
        let s = rgb(160, 160, 176);
        c.rect(8, 72, 74, 36, s);
        for row in 0..3 {
            for i in 0..5 {
                let x = 8 + i * 16 + (row % 2) * 8;
                c.set_on(x, 74 + row * 12, shadow(s, 0.3));
                c.line(8, 84 + row * 12, 81, 84 + row * 12, shadow(s, 0.25));
            }
        }
    });
    out.draw(&base, 0, 0);
    out
}

pub fn banner(frame: i32) -> Canvas {
    let (w, h) = (60, 130);
    let mut out = Canvas::new(w, h);
    let pole = outlined(w, h, |c| c.rect(6, 4, 4, 125, rgb(112, 80, 50)));
    out.draw(&pole, 0, 0);
    let f = frame as f32;
    let flag = outlined(w, h, |c| {
        let wave = (f * 1.3).sin() * 3.0;
        c.poly(&[(10.0, 8.0), (52.0, 14.0 + wave), (46.0, 30.0 + wave), (52.0, 46.0 + wave), (10.0, 44.0)], rgb(70, 150, 206));
        c.poly(&[(10.0, 20.0), (40.0, 24.0 + wave), (10.0, 30.0)], rgb(255, 255, 255));
    });
    out.draw(&flag, 0, 0);
    out
}

pub fn windmill_base() -> Canvas {
    let (w, h) = (110, 190);
    let mut out = Canvas::new(w, h);
    let t = outlined(w, h, |c| {
        let wall = rgb(238, 228, 208);
        c.poly(&[(20.0, 189.0), (90.0, 189.0), (70.0, 30.0), (40.0, 30.0)], wall);
        c.rim_shade(-1, 0, 5, shadow(wall, 0.18));
        c.poly(&[(34.0, 34.0), (76.0, 34.0), (55.0, 6.0)], rgb(150, 92, 64));
        c.rect(46, 150, 18, 39, rgb(130, 86, 52));
        c.rect(48, 80, 12, 14, rgb(140, 200, 240));
    });
    out.draw(&t, 0, 0);
    out
}

/// One blade-set frame (rotation step 0..7), centred at (60, 60).
pub fn windmill_blades(step: i32) -> Canvas {
    let (w, h) = (120, 120);
    outlined(w, h, |c| {
        let a0 = step as f32 / 8.0 * std::f32::consts::FRAC_PI_2;
        for i in 0..4 {
            let a = a0 + i as f32 * std::f32::consts::FRAC_PI_2;
            let (dx, dy) = (a.cos(), a.sin());
            let (px, py) = (-dy, dx);
            let base = (60.0 + dx * 10.0, 60.0 + dy * 10.0);
            let tip = (60.0 + dx * 56.0, 60.0 + dy * 56.0);
            c.capsule((60.0, 60.0), tip, 3.0, rgb(126, 86, 56));
            c.poly(&[(base.0 + px * 2.0, base.1 + py * 2.0), (tip.0 + px * 2.0, tip.1 + py * 2.0), (tip.0 + px * 14.0, tip.1 + py * 14.0), (base.0 + px * 10.0, base.1 + py * 10.0)], rgb(246, 240, 228));
        }
        c.disc(60.0, 60.0, 5.0, rgb(100, 70, 46));
    })
}

pub fn crate_box() -> Canvas {
    outlined(32, 32, |c| {
        let w = rgb(196, 146, 90);
        c.rect(1, 1, 30, 30, w);
        c.rect(1, 1, 30, 4, shadow(w, 0.25));
        c.rect(1, 27, 30, 4, shadow(w, 0.25));
        c.line(3, 26, 28, 5, shadow(w, 0.3));
        c.line(4, 26, 29, 5, shadow(w, 0.3));
    })
}

pub fn sign() -> Canvas {
    let (w, h) = (48, 56);
    let mut out = Canvas::new(w, h);
    let post = outlined(w, h, |c| c.rect(21, 22, 6, 33, rgb(120, 82, 52)));
    out.draw(&post, 0, 0);
    let board = outlined(w, h, |c| {
        let wd = rgb(200, 150, 96);
        c.rect(2, 2, 44, 24, wd);
        c.rim_shade(0, 1, 2, shadow(wd, 0.25));
    });
    out.draw(&board, 0, 0);
    for i in 0..3 {
        out.rect(8, 7 + i * 6, 32 - i * 8, 2, rgb(116, 78, 48));
    }
    out
}

pub fn gate(closed: bool) -> Canvas {
    let (w, h) = (110, 250);
    let mut out = Canvas::new(w, h);
    let stone = rgb(150, 150, 166);
    let pillars = outlined(w, h, |c| {
        c.rect(4, 30, 22, 219, stone);
        c.rect(84, 30, 22, 219, stone);
        c.rect(0, 14, 110, 22, light(stone, 0.12));
        for i in 0..5 {
            c.rect(2 + i * 22, 4, 14, 11, light(stone, 0.12));
        }
        for row in 0..14 {
            let y = 40 + row * 15;
            c.line(4, y, 25, y, shadow(stone, 0.3));
            c.line(84, y, 105, y, shadow(stone, 0.3));
        }
        c.rim_shade(-1, 0, 3, shadow(stone, 0.2));
    });
    out.draw(&pillars, 0, 0);
    if closed {
        let bars = outlined(w, h, |c| {
            let wood = rgb(112, 78, 48);
            for i in 0..4 {
                c.rect(30 + i * 14, 36, 6, 212, wood);
            }
            for j in 0..5 {
                c.rect(26, 60 + j * 40, 58, 6, shadow(wood, 0.15));
            }
        });
        out.draw(&bars, 0, 0);
        out.disc(55.0, 150.0, 6.0, rgb(214, 180, 80));
        out.rect(54, 150, 2, 5, INK);
    }
    out
}

pub fn gravestone() -> Canvas {
    outlined(36, 44, |c| {
        let s = rgb(170, 172, 186);
        c.rect(4, 12, 28, 31, s);
        c.disc(18.0, 13.0, 14.0, s);
        c.rim_shade(-1, 0, 3, shadow(s, 0.25));
        c.rect(12, 20, 12, 2, shadow(s, 0.4));
        c.rect(17, 15, 2, 12, shadow(s, 0.4));
    })
}

// =================================================================== terrain tiles

pub const TILE: i32 = 32;

/// Colours for a map theme.
pub struct ThemeArt {
    pub stone: Px,
    pub mortar: Px,
    pub grass: Px,
    pub grass_dk: Px,
    pub sky_top: Px,
    pub sky_bot: Px,
    pub far: Px,
    pub mid: Px,
    pub near: Px,
}

pub fn theme_art(theme: Theme) -> ThemeArt {
    match theme {
        Theme::Glade => ThemeArt {
            stone: rgb(184, 150, 108), mortar: rgb(104, 78, 56), grass: rgb(116, 196, 92), grass_dk: rgb(76, 150, 70),
            sky_top: rgb(96, 168, 236), sky_bot: rgb(196, 230, 248), far: rgb(154, 190, 222), mid: rgb(112, 176, 140), near: rgb(74, 146, 100),
        },
        Theme::Town => ThemeArt {
            stone: rgb(196, 176, 150), mortar: rgb(132, 110, 92), grass: rgb(132, 196, 96), grass_dk: rgb(92, 152, 72),
            sky_top: rgb(120, 150, 220), sky_bot: rgb(250, 214, 180), far: rgb(196, 168, 206), mid: rgb(180, 190, 140), near: rgb(132, 160, 104),
        },
        Theme::Fields => ThemeArt {
            stone: rgb(198, 160, 104), mortar: rgb(122, 88, 56), grass: rgb(138, 210, 96), grass_dk: rgb(96, 164, 72),
            sky_top: rgb(110, 182, 244), sky_bot: rgb(220, 240, 250), far: rgb(170, 204, 230), mid: rgb(150, 200, 116), near: rgb(116, 178, 88),
        },
        Theme::Woods => ThemeArt {
            stone: rgb(150, 128, 108), mortar: rgb(66, 54, 54), grass: rgb(176, 166, 92), grass_dk: rgb(118, 114, 62),
            sky_top: rgb(18, 20, 52), sky_bot: rgb(76, 62, 118), far: rgb(52, 50, 96), mid: rgb(40, 40, 74), near: rgb(30, 30, 52),
        },
        Theme::Cliffs => ThemeArt {
            stone: rgb(158, 146, 156), mortar: rgb(86, 76, 92), grass: rgb(150, 176, 98), grass_dk: rgb(102, 128, 72),
            sky_top: rgb(84, 70, 150), sky_bot: rgb(252, 176, 112), far: rgb(176, 120, 150), mid: rgb(128, 96, 128), near: rgb(92, 72, 100),
        },
        Theme::Arcane => ThemeArt {
            stone: rgb(196, 190, 220), mortar: rgb(104, 92, 146), grass: rgb(132, 214, 196), grass_dk: rgb(86, 158, 160),
            sky_top: rgb(34, 22, 86), sky_bot: rgb(150, 104, 200), far: rgb(110, 84, 170), mid: rgb(84, 64, 140), near: rgb(62, 48, 112),
        },
        Theme::Interior => ThemeArt {
            stone: rgb(132, 88, 56), mortar: rgb(86, 56, 36), grass: rgb(150, 100, 64), grass_dk: rgb(110, 72, 46),
            sky_top: rgb(120, 80, 52), sky_bot: rgb(150, 104, 70), far: rgb(120, 80, 52), mid: rgb(120, 80, 52), near: rgb(96, 64, 42),
        },
        Theme::Desert => ThemeArt {
            stone: rgb(214, 160, 104), mortar: rgb(146, 96, 60), grass: rgb(226, 196, 128), grass_dk: rgb(190, 150, 92),
            sky_top: rgb(104, 166, 232), sky_bot: rgb(252, 224, 168), far: rgb(222, 156, 118), mid: rgb(196, 132, 92), near: rgb(162, 104, 70),
        },
        Theme::Frost => ThemeArt {
            stone: rgb(184, 200, 222), mortar: rgb(112, 132, 166), grass: rgb(244, 250, 255), grass_dk: rgb(196, 218, 240),
            sky_top: rgb(118, 158, 220), sky_bot: rgb(222, 236, 250), far: rgb(192, 212, 238), mid: rgb(150, 180, 214), near: rgb(108, 144, 186),
        },
        Theme::Ember => ThemeArt {
            stone: rgb(108, 84, 80), mortar: rgb(56, 36, 36), grass: rgb(140, 112, 96), grass_dk: rgb(98, 72, 64),
            sky_top: rgb(48, 18, 24), sky_bot: rgb(214, 86, 46), far: rgb(118, 48, 42), mid: rgb(82, 34, 32), near: rgb(54, 24, 26),
        },
        Theme::Sky => ThemeArt {
            stone: rgb(226, 222, 238), mortar: rgb(150, 142, 186), grass: rgb(168, 228, 178), grass_dk: rgb(118, 186, 146),
            sky_top: rgb(84, 146, 238), sky_bot: rgb(228, 240, 255), far: rgb(204, 218, 248), mid: rgb(180, 200, 242), near: rgb(152, 178, 228),
        },
    }
}

/// Seamlessly tiling cobblestone cliff face (WindSlayer-like chunky rock).
pub fn cobble_tile(variant: i32, theme: Theme) -> Canvas {
    let (w, h) = (TILE * 2, TILE * 2);
    let mut c = Canvas::new(w, h);
    let ta = theme_art(theme);
    let (mortar, stone) = (ta.mortar, ta.stone);
    c.rect(0, 0, w, h, mortar);
    // irregular stones on a jittered grid (wrapped so the tile repeats)
    let cells = 4;
    let cs = w / cells;
    for gy in 0..cells {
        for gx in 0..cells {
            let off = if gy % 2 == 1 { cs / 2 } else { 0 };
            let jx = (hash(gx, gy, variant) * 4.0) as i32 - 2;
            let jy = (hash(gx, gy, variant + 3) * 4.0) as i32 - 2;
            let cx = gx * cs + off + cs / 2 + jx;
            let cy = gy * cs + cs / 2 + jy;
            let rx = cs as f32 * 0.5 - 1.5 + hash(gx, gy, variant + 5) * 1.5;
            let ry = cs as f32 * 0.5 - 2.0 + hash(gx, gy, variant + 9) * 1.5;
            let tone = 0.85 + hash(gx, gy, variant + 11) * 0.3;
            let col = if tone > 1.0 { light(stone, tone - 1.0) } else { shadow(stone, 1.0 - tone) };
            for dy in -(ry as i32) - 1..=(ry as i32) + 1 {
                for dx in -(rx as i32) - 1..=(rx as i32) + 1 {
                    let nx = dx as f32 / rx;
                    let ny = dy as f32 / ry;
                    let d = nx * nx * nx * nx + ny * ny * ny * ny; // squircle
                    let px = (cx + dx).rem_euclid(w);
                    let py = (cy + dy).rem_euclid(h);
                    if d <= 1.0 {
                        let shade = if ny < -0.55 && nx < 0.3 {
                            light(col, 0.22)
                        } else if ny > 0.55 || nx > 0.7 {
                            shadow(col, 0.22)
                        } else {
                            col
                        };
                        c.px[(py * w + px) as usize] = shade;
                    } else if d <= 1.35 {
                        c.px[(py * w + px) as usize] = shadow(mortar, 0.3);
                    }
                }
            }
        }
    }
    c
}

/// Grass lip that sits on top of ground blocks (64 wide, tiles horizontally).
pub fn grass_top(variant: i32, theme: Theme) -> Canvas {
    let (w, h) = (TILE * 2, 22);
    let mut c = Canvas::new(w, h);
    let ta = theme_art(theme);
    let (g, gd) = (ta.grass, ta.grass_dk);
    c.rect(0, 6, w, 6, g);
    for x in 0..w {
        let drop = (hash(x / 3, 0, variant) * 5.0) as i32;
        for y in 12..12 + drop {
            c.set(x, y, if y == 11 + drop { shadow(gd, 0.2) } else { gd });
        }
        c.set(x, 6, light(g, 0.25));
        if hash(x, 1, variant) > 0.62 {
            let len = 2 + (hash(x, 2, variant) * 5.0) as i32;
            for y in (6 - len)..6 {
                c.set(x, y, if y == 6 - len { light(g, 0.2) } else { g });
            }
        }
    }
    let src = c.clone();
    for x in 0..w {
        for y in 0..h {
            if src.opaque(x, y) && !src.opaque(x, y + 1) {
                c.set(x, y + 1, INK);
            }
            if src.opaque(x, y) && !src.opaque(x, y - 1) && y < 6 {
                c.set(x, y - 1, INK);
            }
        }
    }
    c
}

/// Tall grass tuft decoration for ledges (like the reference video).
pub fn tuft(seed: i32) -> Canvas {
    let (w, h) = (40, 34);
    outlined(w, h, |c| {
        let cols = [rgb(214, 196, 90), rgb(176, 168, 70), rgb(120, 180, 80)];
        for i in 0..7 {
            let x = 6.0 + i as f32 * 4.5;
            let lean = (hash(i, 0, seed) - 0.5) * 18.0;
            let len = 14.0 + hash(i, 1, seed) * 16.0;
            c.taper((x, 33.0), (x + lean, 33.0 - len), 3.0, 0.5, cols[(i % 3) as usize]);
        }
    })
}

pub fn town_road_top() -> Canvas {
    let (w, h) = (TILE * 2, 12);
    let mut c = Canvas::new(w, h);
    c.rect(0, 0, w, h, rgb(176, 162, 146));
    for i in 0..4 {
        let x = i * 16 + 1;
        c.rect(x, 1, 14, 7, rgb(214, 204, 190));
        c.rect(x, 1, 14, 1, rgb(232, 224, 212));
        c.rect(x, 7, 14, 1, rgb(160, 148, 132));
    }
    c.rect(0, 0, w, 1, INK);
    c.rect(0, 10, w, 2, rgb(120, 104, 90));
    c
}

pub fn plank(w: i32) -> Canvas {
    let h = 14;
    outlined(w + 2, h + 2, |c| {
        let wood = rgb(186, 128, 76);
        c.rect(1, 1, w, 8, wood);
        c.rect(1, 1, w, 2, light(wood, 0.25));
        c.rect(1, 7, w, 2, shadow(wood, 0.3));
        let mut x = 14;
        while x < w {
            c.rect(x, 2, 1, 6, shadow(wood, 0.35));
            x += 18;
        }
        // support struts
        c.rect(6, 9, 3, 6, shadow(wood, 0.35));
        c.rect(w - 8, 9, 3, 6, shadow(wood, 0.35));
    })
}

// =================================================================== backgrounds

/// Far background strip: mountains (glade/town/fields), dark crags (woods)
/// or jagged sunset spires (cliffs).
pub fn bg_far(theme: Theme) -> Canvas {
    let (w, h) = (640, 170);
    let mut c = Canvas::new(w, h);
    let base = theme_art(theme).far;
    let spiky = matches!(theme, Theme::Cliffs | Theme::Arcane | Theme::Ember);
    let peaks: &[(f32, f32, f32)] = if spiky {
        &[(40.0, 20.0, 50.0), (120.0, 60.0, 60.0), (200.0, 10.0, 45.0), (290.0, 50.0, 70.0), (380.0, 30.0, 40.0), (460.0, 70.0, 60.0), (540.0, 15.0, 55.0), (610.0, 55.0, 40.0)]
    } else {
        &[(60.0, 40.0, 150.0), (200.0, 70.0, 130.0), (330.0, 20.0, 170.0), (470.0, 60.0, 140.0), (590.0, 35.0, 150.0)]
    };
    for &(px, py, half) in peaks {
        for k in [-1.0f32, 0.0, 1.0] {
            let x = px + k * w as f32;
            c.poly(&[(x - half, h as f32), (x, py), (x + half, h as f32)], base);
        }
    }
    for &(px, py, half) in peaks {
        for k in [-1.0f32, 0.0, 1.0] {
            let x = px + k * w as f32;
            c.poly(&[(x, py), (x + half, h as f32), (x + half * 0.35, h as f32)], shadow(base, 0.12));
            if theme == Theme::Arcane {
                // glowing crystal tips on the spires
                c.poly(&[(x - 5.0, py + 12.0), (x, py - 6.0), (x + 5.0, py + 12.0)], rgb(150, 236, 255));
            } else if theme == Theme::Ember {
                // a lava glow at each peak
                c.poly(&[(x - 4.0, py + 10.0), (x, py), (x + 4.0, py + 10.0)], rgb(255, 150, 60));
            } else if theme == Theme::Frost {
                // thick snowcaps
                c.poly(&[(x - 30.0, py + 34.0), (x, py), (x + 30.0, py + 34.0), (x + 10.0, py + 26.0), (x - 8.0, py + 30.0)], rgb(250, 252, 255));
            } else if theme != Theme::Woods {
                let cap = if spiky { light(base, 0.35) } else { light(base, 0.6) };
                let s = if spiky { 0.4 } else { 1.0 };
                c.poly(&[(x - 18.0 * s, py + 22.0 * s), (x, py), (x + 18.0 * s, py + 22.0 * s), (x + 6.0 * s, py + 16.0 * s), (x - 4.0 * s, py + 20.0 * s)], cap);
            }
        }
    }
    c
}

/// Mid/near background strips: rounded forests; haunted trunks with faces
/// (woods); pine silhouettes on rock (cliffs); rolling hills (fields).
pub fn bg_trees(theme: Theme, near: bool) -> Canvas {
    let (w, h) = (640, 200);
    let mut c = Canvas::new(w, h);
    let ta = theme_art(theme);
    let col = if near { ta.near } else { ta.mid };
    let ground_y = if near { 150 } else { 120 };
    c.rect(0, ground_y, w, h - ground_y, col);
    match theme {
        Theme::Woods => {
            let n = if near { 7 } else { 14 };
            for i in 0..n {
                let x = (i as f32 + 0.3 + hash(i, 0, near as i32) * 0.4) * (w as f32 / n as f32);
                let tw = if near { 30.0 + hash(i, 1, 3) * 16.0 } else { 14.0 + hash(i, 1, 5) * 8.0 };
                for k in [-1.0f32, 0.0, 1.0] {
                    let xx = x + k * w as f32;
                    // trunk flares at the roots, vanishes above the strip
                    c.poly(&[(xx - tw * 0.5, ground_y as f32), (xx + tw * 0.5, ground_y as f32), (xx + tw * 0.35, 0.0), (xx - tw * 0.35, 0.0)], col);
                    c.poly(&[(xx - tw * 0.9, ground_y as f32 + 2.0), (xx - tw * 0.4, ground_y as f32 - 20.0), (xx - tw * 0.3, ground_y as f32 + 2.0)], col);
                    c.poly(&[(xx + tw * 0.9, ground_y as f32 + 2.0), (xx + tw * 0.4, ground_y as f32 - 20.0), (xx + tw * 0.3, ground_y as f32 + 2.0)], col);
                    // bare branch
                    let by = 30.0 + hash(i, 4, 1) * 40.0;
                    c.taper((xx, by + 20.0), (xx - tw * 1.6, by), tw * 0.25, 1.0, col);
                    if near {
                        // a sad haunted face in the bark (like the reference forest)
                        let fy = 60.0 + hash(i, 2, 9) * 30.0;
                        let face = shadow(col, 0.5);
                        c.ellipse(xx - tw * 0.18, fy, 3.0, 5.0, face);
                        c.ellipse(xx + tw * 0.18, fy, 3.0, 5.0, face);
                        c.ellipse(xx, fy + 18.0, tw * 0.22, 4.0, face);
                        c.disc(xx - tw * 0.18, fy + 1.0, 1.0, rgb(140, 110, 200));
                        c.disc(xx + tw * 0.18, fy + 1.0, 1.0, rgb(140, 110, 200));
                    }
                }
            }
        }
        Theme::Cliffs => {
            let n = if near { 10 } else { 16 };
            for i in 0..n {
                let x = (i as f32 + hash(i, 0, near as i32 + 4) * 0.6) * (w as f32 / n as f32);
                let ht = if near { 60.0 + hash(i, 1, 7) * 50.0 } else { 40.0 + hash(i, 1, 9) * 40.0 };
                for k in [-1.0f32, 0.0, 1.0] {
                    let xx = x + k * w as f32;
                    // rock pillar + pine on top
                    c.rect((xx - 10.0) as i32, (ground_y as f32 - ht * 0.4) as i32, 20, (ht * 0.4) as i32 + 1, shadow(col, 0.1));
                    for l in 0..3 {
                        let lw = 14.0 - l as f32 * 4.0;
                        let ly = ground_y as f32 - ht * 0.4 - l as f32 * ht * 0.18;
                        c.poly(&[(xx - lw, ly), (xx + lw, ly), (xx, ly - ht * 0.3)], col);
                    }
                }
            }
        }
        Theme::Arcane => {
            // marble towers with glowing windows
            let n = if near { 6 } else { 10 };
            for i in 0..n {
                let x = (i as f32 + hash(i, 0, near as i32 + 8) * 0.5) * (w as f32 / n as f32);
                let tw = if near { 22.0 } else { 14.0 };
                let ht = if near { 90.0 + hash(i, 1, 3) * 40.0 } else { 60.0 + hash(i, 1, 4) * 40.0 };
                for k in [-1.0f32, 0.0, 1.0] {
                    let xx = x + k * w as f32;
                    let top = ground_y as f32 - ht;
                    c.rect((xx - tw * 0.5) as i32, top as i32, tw as i32, ht as i32 + 1, col);
                    c.poly(&[(xx - tw * 0.7, top), (xx + tw * 0.7, top), (xx, top - tw * 1.4)], col);
                    for j in 0..3 {
                        c.rect(xx as i32 - 1, (top + 12.0 + j as f32 * 22.0) as i32, 3, 5, rgb(150, 230, 255));
                    }
                }
            }
        }
        Theme::Desert => {
            // flat-topped mesas and saguaro cacti
            let n = if near { 5 } else { 8 };
            for i in 0..n {
                let x = (i as f32 + hash(i, 0, near as i32 + 20) * 0.5) * (w as f32 / n as f32);
                for k in [-1.0f32, 0.0, 1.0] {
                    let xx = x + k * w as f32;
                    if i % 2 == 0 {
                        let mw = if near { 70.0 } else { 46.0 };
                        let mh = if near { 70.0 } else { 54.0 } + hash(i, 1, 21) * 30.0;
                        c.poly(&[(xx - mw, ground_y as f32), (xx - mw * 0.7, ground_y as f32 - mh), (xx + mw * 0.7, ground_y as f32 - mh), (xx + mw, ground_y as f32)], col);
                    } else {
                        let ch = if near { 54.0 } else { 34.0 };
                        let cw = if near { 7.0 } else { 4.5 };
                        c.rect((xx - cw * 0.5) as i32, (ground_y as f32 - ch) as i32, cw as i32 + 1, ch as i32 + 1, shadow(col, 0.1));
                        c.rect((xx - cw * 2.2) as i32, (ground_y as f32 - ch * 0.6) as i32, cw as i32, (ch * 0.25) as i32, shadow(col, 0.1));
                        c.rect((xx - cw * 2.2) as i32, (ground_y as f32 - ch * 0.85) as i32, (cw * 0.7) as i32, (ch * 0.28) as i32, shadow(col, 0.1));
                        c.rect((xx + cw * 0.9) as i32, (ground_y as f32 - ch * 0.5) as i32, cw as i32, (ch * 0.22) as i32, shadow(col, 0.1));
                    }
                }
            }
        }
        Theme::Frost => {
            // snowy pines
            let n = if near { 11 } else { 18 };
            for i in 0..n {
                let x = (i as f32 + hash(i, 0, near as i32 + 30) * 0.6) * (w as f32 / n as f32);
                let ht = if near { 64.0 + hash(i, 1, 31) * 40.0 } else { 40.0 + hash(i, 1, 32) * 30.0 };
                for k in [-1.0f32, 0.0, 1.0] {
                    let xx = x + k * w as f32;
                    for l in 0..3 {
                        let lw = 15.0 - l as f32 * 4.0;
                        let ly = ground_y as f32 - l as f32 * ht * 0.24;
                        c.poly(&[(xx - lw, ly), (xx + lw, ly), (xx, ly - ht * 0.36)], col);
                        c.poly(&[(xx - lw * 0.45, ly - ht * 0.2), (xx + lw * 0.45, ly - ht * 0.2), (xx, ly - ht * 0.36)], rgb(246, 250, 255));
                    }
                }
            }
        }
        Theme::Ember => {
            // jagged basalt with lava seams
            let n = if near { 8 } else { 13 };
            for i in 0..n {
                let x = (i as f32 + hash(i, 0, near as i32 + 40) * 0.6) * (w as f32 / n as f32);
                let ht = if near { 60.0 + hash(i, 1, 41) * 50.0 } else { 40.0 + hash(i, 1, 42) * 40.0 };
                for k in [-1.0f32, 0.0, 1.0] {
                    let xx = x + k * w as f32;
                    c.poly(&[(xx - 22.0, ground_y as f32), (xx - 6.0, ground_y as f32 - ht), (xx + 4.0, ground_y as f32 - ht * 0.7), (xx + 20.0, ground_y as f32)], col);
                    c.line(xx as i32 - 4, (ground_y as f32 - ht * 0.6) as i32, xx as i32 + 2, ground_y - 4, rgb(255, 120, 40));
                }
            }
        }
        Theme::Sky => {
            // banks of cloud with little floating isles
            let n = if near { 6 } else { 9 };
            for i in 0..n {
                let x = (i as f32 + hash(i, 0, near as i32 + 50) * 0.5) * (w as f32 / n as f32);
                for k in [-1.0f32, 0.0, 1.0] {
                    let xx = x + k * w as f32;
                    let r = if near { 30.0 } else { 20.0 } + hash(i, 1, 51) * 14.0;
                    c.disc(xx, ground_y as f32 - r * 0.4, r, light(col, 0.25));
                    c.disc(xx - r, ground_y as f32 - r * 0.1, r * 0.7, light(col, 0.25));
                    c.disc(xx + r, ground_y as f32 - r * 0.1, r * 0.75, light(col, 0.25));
                    if i % 3 == 0 {
                        let iy = ground_y as f32 - r * 2.6;
                        c.poly(&[(xx - r * 0.8, iy), (xx + r * 0.8, iy), (xx, iy + r * 0.9)], shadow(col, 0.15));
                        c.rect((xx - r * 0.8) as i32, (iy - 3.0) as i32, (r * 1.6) as i32, 4, rgb(150, 210, 150));
                    }
                }
            }
        }
        Theme::Fields if near => {
            // rolling hedgerow hills
            for i in 0..6 {
                let x = i as f32 * 120.0 + hash(i, 0, 3) * 40.0;
                for k in [-1.0f32, 0.0, 1.0] {
                    c.ellipse(x + k * w as f32, ground_y as f32, 90.0, 26.0 + hash(i, 1, 2) * 14.0, col);
                }
            }
        }
        _ => {
            let n = if near { 14 } else { 22 };
            for i in 0..n {
                let x = (i as f32 + hash(i, 0, near as i32) * 0.6) * (w as f32 / n as f32);
                let r = if near { 26.0 + hash(i, 1, 3) * 18.0 } else { 18.0 + hash(i, 1, 5) * 16.0 };
                let top = ground_y as f32 - r * 1.2 - hash(i, 2, 7) * 20.0;
                for k in [-1.0f32, 0.0, 1.0] {
                    let xx = x + k * w as f32;
                    c.rect((xx - 2.0) as i32, top as i32, 5, ground_y - top as i32, shadow(col, 0.25));
                    c.disc(xx, top, r, col);
                    c.disc(xx - r * 0.6, top + r * 0.5, r * 0.7, col);
                    c.disc(xx + r * 0.6, top + r * 0.5, r * 0.7, col);
                }
            }
        }
    }
    let src = c.clone();
    for y in 1..h {
        for x in 0..w {
            if src.opaque(x, y) && !src.opaque(x, y - 2) && (x + y) % 2 == 0 {
                c.set(x, y, light(col, 0.15));
            }
        }
    }
    c
}

pub fn cloud(seed: i32) -> Canvas {
    let (w, h) = (96, 40);
    let mut c = Canvas::new(w, h);
    let white = rgb(255, 255, 255);
    c.ellipse(48.0, 28.0, 44.0, 10.0, white);
    c.disc(30.0 + hash(1, 0, seed) * 8.0, 22.0, 14.0, white);
    c.disc(54.0, 16.0, 16.0 + hash(2, 0, seed) * 4.0, white);
    c.disc(74.0, 24.0, 11.0, white);
    c.rim_shade(0, 1, 4, mix(white, rgb(200, 220, 246), 0.6));
    c
}

#[allow(dead_code)]
pub fn px_mix(a: Px, b: Px, t: f32) -> Px {
    mix(a, b, t)
}

// =================================================================== zone monsters
// Same conventions as above: facing right, feet at the bottom centre.

fn white_if(flash: bool, c: Px) -> Px {
    if flash { rgb(255, 255, 255) } else { c }
}

fn eyes(c: &mut Canvas, x: i32, y: i32, gap: i32, h: i32, col: Px) {
    for ex in [x, x + gap] {
        c.rect(ex, y, 2, h, col);
        c.set(ex, y, rgb(255, 255, 255));
    }
}

/// Fluffy sheep-ball. frame 0..3 = bounce squash.
pub fn puffwool(frame: i32, flash: bool) -> Canvas {
    let (w, h) = (48, 40);
    let sq = [0.0, 1.0, 0.0, -1.0][(frame & 3) as usize];
    let wool = white_if(flash, rgb(246, 238, 220));
    let mut out = outlined(w, h, |c| {
        // little legs
        for lx in [16.0f32, 30.0] {
            c.rect(lx as i32, 33, 3, 6, rgb(70, 56, 60));
        }
        let cy = 22.0 + sq;
        for (dx, dy, r) in [(-10.0, 2.0, 9.0), (0.0, -3.0, 11.0), (10.0, 1.0, 9.5), (-3.0, 6.0, 10.0), (7.0, 6.0, 9.0), (-14.0, -4.0, 6.0), (13.0, -6.0, 6.5)] {
            c.disc(24.0 + dx, cy + dy, r - sq * 0.3, wool);
        }
        c.rim_shade(0, 1, 3, shadow(wool, 0.2));
        // face
        c.ellipse(36.0, cy + 2.0, 6.0, 7.0, rgb(70, 56, 66));
        // curled horns
        c.disc(31.0, cy - 6.0, 3.0, rgb(214, 182, 120));
    });
    let cy = (22.0 + sq) as i32;
    eyes(&mut out, 35, cy, 3, 2, rgb(250, 230, 160));
    for i in 0..6 {
        let x = 10 + (i * 5) % 26;
        let y = cy - 8 + (i * 7) % 14;
        out.set_on(x, y, light(wool, 0.4));
    }
    out
}

/// Little wind spirit. state: 0 drift, 1 charging a shot.
pub fn gustling(frame: i32, state: i32, flash: bool) -> Canvas {
    let (w, h) = (48, 44);
    let body = white_if(flash, if state == 1 { rgb(196, 236, 255) } else { rgb(170, 214, 240) });
    let f = frame as f32;
    let puff = if state == 1 { 2.0 } else { 0.0 };
    let mut out = outlined(w, h, |c| {
        c.disc(24.0, 20.0, 12.0 + puff, body);
        c.disc(14.0, 24.0, 7.0 + puff * 0.5, body);
        c.disc(34.0, 24.0, 7.0 + puff * 0.5, body);
        // swirling tail
        let sw = (f * 1.5).sin() * 3.0;
        c.taper((18.0, 30.0), (10.0 + sw, 42.0), 8.0, 2.0, body);
        c.rim_shade(0, 1, 3, shadow(body, 0.18));
    });
    // swirl marks + face
    let s = shadow(body, 0.3);
    out.line(14, 16, 20, 13, s);
    out.line(16, 20, 21, 18, s);
    eyes(&mut out, 27, 18, 5, 3, rgb(40, 60, 110));
    out.rect(29, 24, 3, 1, rgb(40, 60, 110));
    if state == 1 {
        out.disc(36.0, 22.0, 3.0, rgb(255, 255, 255));
    }
    out
}

/// Purple glowing mushroom with an attitude.
pub fn gravecap(frame: i32, flash: bool) -> Canvas {
    let (w, h) = (48, 46);
    let cap = white_if(flash, rgb(130, 76, 160));
    let stem = white_if(flash, rgb(226, 214, 196));
    let sq = [0.0, 1.0, 0.0, -1.0][(frame & 3) as usize];
    let mut out = outlined(w, h, |c| {
        c.rect(15, 41, 6, 4, rgb(80, 60, 70));
        c.rect(27, 41, 6, 4, rgb(80, 60, 70));
        c.ellipse(24.0, 34.0 + sq * 0.5, 10.0, 9.0 - sq, stem);
        c.rim_shade(0, 1, 2, shadow(stem, 0.2));
    });
    let capl = outlined(w, h, |c| {
        c.ellipse(24.0, 20.0 + sq, 20.0 + sq, 12.0, cap);
        c.rect(5, 20 + sq as i32, 39, 4, cap);
        c.rim_shade(0, 1, 3, shadow(cap, 0.3));
    });
    out.draw(&capl, 0, 0);
    // glowing spots
    for (x, y, r) in [(16.0, 14.0, 3.0), (28.0, 11.0, 2.5), (36.0, 18.0, 2.5), (22.0, 20.0, 2.0)] {
        out.disc(x, y + sq, r, rgb(140, 250, 230));
    }
    // grumpy face on the stem
    let fy = (32.0 + sq * 0.5) as i32;
    out.line(26, fy - 2, 29, fy - 1, INK);
    out.line(31, fy - 1, 33, fy - 2, INK);
    eyes(&mut out, 27, fy, 4, 2, rgb(200, 40, 60));
    out.rect(28, fy + 4, 4, 1, INK);
    out
}

/// Haunted lantern with a flame face. state: 0 drift, 1 charging.
pub fn lanternwisp(frame: i32, state: i32, flash: bool) -> Canvas {
    let (w, h) = (40, 52);
    let f = frame as f32;
    let metal = white_if(flash, rgb(78, 70, 92));
    let mut out = Canvas::new(w, h);
    // ghostly tail
    let tail = outlined(w, h, |c| {
        let sw = (f * 1.3).sin() * 3.0;
        c.taper((20.0, 36.0), (16.0 + sw, 50.0), 10.0, 1.5, rgb(170, 160, 230));
    });
    out.draw(&tail, 0, 0);
    let frame_l = outlined(w, h, |c| {
        c.rect(10, 6, 20, 4, metal);
        c.rect(12, 10, 16, 22, rgb(255, 210, 120));
        c.rect(10, 31, 20, 4, metal);
        c.rect(12, 10, 2, 22, metal);
        c.rect(26, 10, 2, 22, metal);
        c.rect(18, 1, 4, 6, metal);
    });
    out.draw(&frame_l, 0, 0);
    // flame with a face
    let hot = if state == 1 { rgb(255, 250, 220) } else { rgb(255, 190, 90) };
    out.poly(&[(14.0, 30.0), (26.0, 30.0), (20.0 + (f * 2.0).sin(), 12.0)], hot);
    out.rect(17, 21, 2, 2, INK);
    out.rect(22, 21, 2, 2, INK);
    out.rect(19, 26, 3, 1, INK);
    out
}

/// Walking tree stump. state: 0 walk, 1 telegraph (rumble), 2 lunge.
pub fn rotstump(frame: i32, state: i32, flash: bool) -> Canvas {
    let (w, h) = (64, 62);
    let bark = white_if(flash, rgb(110, 82, 62));
    let shake = if state == 1 { (frame % 2) as f32 * 2.0 - 1.0 } else { 0.0 };
    let lean = if state == 2 { 4.0 } else { 0.0 };
    let step = if state == 0 { [3.0, 0.0, -3.0, 0.0][(frame & 3) as usize] } else { 0.0 };
    let mut out = Canvas::new(w, h);
    let roots = outlined(w, h, |c| {
        c.taper((24.0, 48.0), (16.0 + step, 61.0), 6.0, 2.5, shadow(bark, 0.2));
        c.taper((40.0, 48.0), (46.0 - step, 61.0), 6.0, 2.5, shadow(bark, 0.2));
    });
    out.draw(&roots, 0, 0);
    let trunk = outlined(w, h, |c| {
        let cx = 32.0 + shake + lean;
        c.poly(&[(cx - 15.0, 52.0), (cx + 15.0, 52.0), (cx + 13.0, 12.0), (cx - 13.0, 12.0)], bark);
        c.ellipse(cx, 12.0, 13.0, 4.0, rgb(196, 160, 110)); // cut top
        c.disc(cx, 12.0, 2.0, rgb(150, 112, 76));
        // branch arms
        c.taper((cx - 12.0, 26.0), (cx - 26.0, 18.0 + step), 4.0, 1.5, bark);
        c.taper((cx + 12.0, 28.0), (cx + 26.0, if state == 2 { 30.0 } else { 20.0 }), 4.0, 1.5, bark);
        for i in 0..4 {
            c.line((cx - 9.0 + i as f32 * 6.0) as i32, 18, (cx - 10.0 + i as f32 * 6.0) as i32, 48, shadow(bark, 0.3));
        }
        // moss
        c.ellipse(cx - 6.0, 14.0, 6.0, 2.0, rgb(96, 140, 76));
    });
    out.draw(&trunk, 0, 0);
    let cx = (32.0 + shake + lean) as i32;
    let eye = if state >= 1 { rgb(255, 80, 60) } else { rgb(150, 240, 120) };
    out.ellipse(cx as f32 + 1.0, 32.0, 9.0, 6.0, rgb(40, 28, 26));
    out.rect(cx - 4, 30, 3, 3, eye);
    out.rect(cx + 4, 30, 3, 3, eye);
    out
}

/// Rock crab. state: 0 walk, 1 claws raised, 2 lunge.
pub fn crag_crab(frame: i32, state: i32, flash: bool) -> Canvas {
    let (w, h) = (66, 44);
    let shell = white_if(flash, rgb(140, 130, 142));
    let claw = white_if(flash, rgb(206, 110, 90));
    let step = [2.0, 0.0, -2.0, 0.0][(frame & 3) as usize];
    let mut out = Canvas::new(w, h);
    let legs = outlined(w, h, |c| {
        for (i, lx) in [16.0f32, 24.0, 40.0, 48.0].iter().enumerate() {
            let s = if i % 2 == 0 { step } else { -step };
            c.taper((*lx, 30.0), (lx + if *lx < 32.0 { -5.0 } else { 5.0 } + s, 43.0), 3.0, 1.5, shadow(claw, 0.25));
        }
    });
    out.draw(&legs, 0, 0);
    let body = outlined(w, h, |c| {
        c.ellipse(32.0, 26.0, 20.0, 12.0, shell);
        c.rect(12, 26, 41, 6, shell);
        c.rim_shade(0, 1, 3, shadow(shell, 0.3));
        // rocky bumps + moss
        c.disc(24.0, 17.0, 5.0, light(shell, 0.12));
        c.disc(38.0, 15.0, 6.0, shell);
        c.ellipse(30.0, 14.0, 8.0, 2.5, rgb(110, 150, 90));
    });
    out.draw(&body, 0, 0);
    let up = if state == 1 { -8.0 } else if state == 2 { 2.0 } else { 0.0 };
    let reach = if state == 2 { 6.0 } else { 0.0 };
    let claws = outlined(w, h, |c| {
        c.taper((44.0, 28.0), (54.0 + reach, 20.0 + up), 4.0, 3.0, claw);
        c.disc(56.0 + reach, 18.0 + up, 5.0, claw);
        c.poly(&[(55.0 + reach, 16.0 + up), (64.0 + reach * 0.3, 12.0 + up), (60.0 + reach * 0.3, 18.0 + up)], claw);
    });
    out.draw(&claws, 0, 0);
    // eye stalks
    out.line(40, 14, 41, 8, INK);
    out.line(46, 15, 48, 9, INK);
    out.rect(40, 6, 2, 2, rgb(30, 20, 20));
    out.rect(47, 7, 2, 2, rgb(30, 20, 20));
    out
}

/// Diving hawk. state: 0 glide (frame flaps), 1 screech (wings spread), 2 dive.
pub fn galehawk(frame: i32, state: i32, flash: bool) -> Canvas {
    let (w, h) = (60, 44);
    let feather = white_if(flash, rgb(150, 110, 82));
    let belly = white_if(flash, rgb(236, 220, 190));
    let mut out = Canvas::new(w, h);
    let wing_up = state == 1 || (state == 0 && frame % 2 == 0);
    let back_wing = outlined(w, h, |c| {
        if state == 2 {
            c.poly(&[(22.0, 22.0), (8.0, 14.0), (26.0, 18.0)], shadow(feather, 0.25));
        } else if wing_up {
            c.poly(&[(24.0, 22.0), (14.0, 2.0), (32.0, 18.0)], shadow(feather, 0.25));
        } else {
            c.poly(&[(24.0, 22.0), (12.0, 38.0), (32.0, 24.0)], shadow(feather, 0.25));
        }
    });
    out.draw(&back_wing, 0, 0);
    let body = outlined(w, h, |c| {
        c.ellipse(30.0, 24.0, 14.0, 7.0, feather);
        c.ellipse(32.0, 27.0, 9.0, 4.0, belly);
        c.poly(&[(17.0, 22.0), (4.0, 18.0), (6.0, 28.0)], feather); // tail
        c.disc(44.0, 20.0, 6.0, feather);
        c.poly(&[(49.0, 18.0), (57.0, 22.0), (49.0, 23.0)], rgb(244, 200, 80)); // beak
        if state == 1 {
            c.poly(&[(49.0, 22.0), (55.0, 26.0), (49.0, 24.0)], rgb(244, 200, 80));
        }
    });
    out.draw(&body, 0, 0);
    let front_wing = outlined(w, h, |c| {
        if state == 2 {
            c.poly(&[(26.0, 22.0), (10.0, 18.0), (30.0, 26.0)], feather);
        } else if wing_up {
            c.poly(&[(28.0, 22.0), (22.0, 0.0), (40.0, 4.0), (36.0, 20.0)], feather);
        } else {
            c.poly(&[(28.0, 24.0), (20.0, 42.0), (38.0, 34.0), (36.0, 24.0)], feather);
        }
    });
    out.draw(&front_wing, 0, 0);
    out.rect(45, 18, 2, 2, if state == 1 { rgb(255, 80, 60) } else { INK });
    out
}

// =================================================================== zone props

pub fn haystack() -> Canvas {
    outlined(70, 50, |c| {
        let hay = rgb(232, 196, 96);
        c.ellipse(35.0, 30.0, 32.0, 19.0, hay);
        c.rect(3, 30, 65, 19, hay);
        c.rim_shade(0, 1, 4, shadow(hay, 0.25));
        for i in 0..9 {
            let x = 8 + i * 7;
            c.line(x, 20 + (i % 3), x + 2, 46, shadow(hay, 0.2));
        }
        c.line(6, 32, 64, 32, rgb(160, 100, 60));
    })
}

pub fn scarecrow() -> Canvas {
    let (w, h) = (56, 90);
    let mut out = outlined(w, h, |c| {
        let wood = rgb(130, 90, 58);
        c.rect(26, 22, 4, 67, wood);
        c.rect(6, 34, 44, 4, wood);
        c.poly(&[(14.0, 30.0), (42.0, 30.0), (40.0, 60.0), (16.0, 60.0)], rgb(150, 70, 70));
        c.rect(6, 34, 10, 6, rgb(150, 70, 70));
        c.rect(40, 34, 10, 6, rgb(150, 70, 70));
        c.disc(28.0, 18.0, 9.0, rgb(226, 200, 150));
        c.poly(&[(14.0, 12.0), (42.0, 12.0), (28.0, 0.0)], rgb(170, 130, 70));
        c.rect(12, 11, 32, 3, rgb(170, 130, 70));
    });
    out.rect(24, 17, 2, 2, INK);
    out.rect(30, 17, 2, 2, INK);
    out.line(24, 23, 32, 23, INK);
    out
}

/// Spooky leafless tree with a face (like the reference video's forest).
pub fn dead_tree(seed: i32) -> Canvas {
    let (w, h) = (130, 200);
    let mut out = outlined(w, h, |c| {
        let bark = rgb(92, 74, 82);
        c.poly(&[(50.0, 199.0), (80.0, 199.0), (74.0, 60.0), (58.0, 60.0)], bark);
        c.poly(&[(36.0, 199.0), (94.0, 199.0), (78.0, 180.0), (52.0, 180.0)], bark);
        for (i, (ex, ey)) in [(12.0, 30.0), (30.0, 6.0), (100.0, 10.0), (122.0, 40.0), (66.0, 0.0)].iter().enumerate() {
            let sy = 70.0 + i as f32 * 6.0;
            c.taper((66.0, sy), (*ex + hash(i as i32, 0, seed) * 8.0, *ey), 7.0, 1.0, bark);
        }
        c.rim_shade(-1, 0, 3, shadow(bark, 0.3));
    });
    // face
    let face = rgb(30, 22, 30);
    out.ellipse(58.0, 112.0, 4.0, 7.0, face);
    out.ellipse(74.0, 112.0, 4.0, 7.0, face);
    out.ellipse(66.0, 140.0, 8.0, 10.0, face);
    out.disc(58.0, 113.0, 1.5, rgb(170, 140, 230));
    out.disc(74.0, 113.0, 1.5, rgb(170, 140, 230));
    out
}

pub fn wooden_cross() -> Canvas {
    outlined(30, 48, |c| {
        let wood = rgb(150, 112, 78);
        c.rect(13, 2, 5, 45, wood);
        c.rect(3, 12, 25, 5, wood);
        c.rim_shade(0, 1, 1, shadow(wood, 0.3));
    })
}

pub fn mushrooms() -> Canvas {
    let mut out = outlined(48, 30, |c| {
        for (x, s) in [(10.0f32, 0.8f32), (24.0, 1.2), (38.0, 0.9)] {
            c.rect((x - 2.0) as i32, (29.0 - 10.0 * s) as i32, 4, (10.0 * s) as i32, rgb(226, 214, 196));
            c.ellipse(x, 29.0 - 10.0 * s, 8.0 * s, 5.0 * s, rgb(120, 70, 160));
        }
    });
    for (x, y) in [(9, 20), (22, 14), (27, 16), (37, 19)] {
        out.set(x, y, rgb(140, 250, 230));
    }
    out
}

pub fn pine() -> Canvas {
    outlined(70, 130, |c| {
        let g = rgb(70, 120, 96);
        c.rect(32, 100, 7, 29, rgb(110, 76, 54));
        for l in 0..4 {
            let y = 104.0 - l as f32 * 24.0;
            let wd = 32.0 - l as f32 * 6.0;
            c.poly(&[(35.0 - wd, y), (35.0 + wd, y), (35.0, y - 42.0)], g);
        }
        c.rim_shade(-1, 0, 3, shadow(g, 0.25));
    })
}

pub fn lantern_post() -> Canvas {
    let (w, h) = (34, 100);
    let mut out = outlined(w, h, |c| {
        let iron = rgb(60, 56, 72);
        c.rect(15, 18, 4, 81, iron);
        c.rect(9, 96, 16, 3, iron);
        c.rect(15, 16, 14, 3, iron);
        c.rect(22, 19, 10, 14, rgb(200, 170, 255));
        c.rect(21, 18, 12, 2, iron);
        c.rect(21, 32, 12, 2, iron);
    });
    out.rect(25, 22, 4, 8, rgb(240, 230, 255));
    out
}

pub fn crystal() -> Canvas {
    let mut out = outlined(50, 54, |c| {
        let cy = rgb(120, 210, 230);
        c.poly(&[(20.0, 53.0), (30.0, 53.0), (28.0, 10.0), (24.0, 2.0), (20.0, 10.0)], cy);
        c.poly(&[(8.0, 53.0), (18.0, 53.0), (14.0, 24.0)], shadow(cy, 0.15));
        c.poly(&[(32.0, 53.0), (44.0, 53.0), (42.0, 30.0), (36.0, 22.0)], light(cy, 0.1));
        c.rim_shade(-1, 0, 2, shadow(cy, 0.3));
    });
    out.line(24, 8, 25, 40, rgb(230, 250, 255));
    out
}
