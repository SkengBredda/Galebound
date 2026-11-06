//! World rendering with pixel-art sprites: parallax backgrounds, cobblestone
//! terrain, props, characters, enemies, projectiles and effects.

use galebound_core::save::FxCat;
use crate::art;
use crate::charsprite::{self, Anim};
use crate::gfx::{self, alpha, mix, rgb, rgba, shade};
use crate::sprites;
use galebound_core::appearance::Appearance;
use galebound_core::classes::ProjStyle;
use galebound_core::math::Aabb;
use galebound_core::quests;
use galebound_core::sim::{AttackPhase, EState, FloatKind, FxKind, PState, Player, Sim, Team};
use galebound_core::world::{Deco, DecoKind, EnemyKind, Theme};
use macroquad::prelude::{Color, Texture2D};

const WHITE: Color = Color::new(1.0, 1.0, 1.0, 1.0);

pub struct Camera {
    pub x: f32,
    pub y: f32,
}

impl Camera {
    pub fn target(sim: &Sim) -> (f32, f32) {
        let p = &sim.player.body;
        let look = sim.player.facing * 40.0;
        let tx = p.pos.x + p.w * 0.5 - gfx::view_w() * 0.5 + look;
        // Keep the player's feet about 3/4 down the screen.
        let mut ty = p.pos.y + p.h - gfx::view_h() * 0.68;
        if sim.dialog.is_some() {
            ty += 90.0; // lift the action above the dialog box
        }
        let maxx = (sim.map.width - gfx::view_w()).max(0.0);
        (tx.clamp(0.0, maxx), ty.clamp(-300.0, (sim.map.height - gfx::view_h()).max(-300.0)))
    }
    pub fn snap(&mut self, sim: &Sim) {
        let (x, y) = Self::target(sim);
        self.x = x;
        self.y = y;
    }
    pub fn follow(&mut self, sim: &Sim, dt: f32) {
        let (x, y) = Self::target(sim);
        let k = 1.0 - (-dt * 7.0).exp();
        let ky = 1.0 - (-dt * 4.0).exp();
        self.x += (x - self.x) * k;
        self.y += (y - self.y) * ky;
    }
}

/// World units per canvas pixel.
fn u(px: f32) -> f32 {
    px / gfx::WORLD_PX
}

/// Draw a sprite anchored at its bottom-centre (world space).
fn sprite_at(t: &Texture2D, x: f32, y: f32, flip: bool, tint: Color) {
    gfx::tex(t, x - u(t.width() * 0.5), y - u(t.height()), 1.0, flip, tint);
}

// ---------------------------------------------------------------- background

fn tkey(theme: Theme) -> &'static str {
    match theme {
        Theme::Glade => "g",
        Theme::Town => "t",
        Theme::Fields => "f",
        Theme::Woods => "w",
        Theme::Cliffs => "c",
        Theme::Arcane => "a",
        Theme::Interior => "i",
        Theme::Desert => "d",
        Theme::Frost => "fr",
        Theme::Ember => "e",
        Theme::Sky => "s",
    }
}

fn px_color(p: crate::pixel::Px) -> Color {
    rgba(p[0], p[1], p[2], p[3])
}

pub fn background(theme: Theme, cam: &Camera, time: f32) {
    gfx::clear_cam();
    if theme == Theme::Interior {
        interior_background(cam);
        return;
    }
    let ta = art::theme_art(theme);
    gfx::vgradient(0.0, 0.0, gfx::VW, gfx::VH, px_color(ta.sky_top), px_color(ta.sky_bot), 12);
    let (sx, sy) = (1000.0 - cam.x * 0.02, 110.0 - cam.y * 0.02);
    match theme {
        Theme::Woods => {
            // stars + a big moon, like the reference forest
            for i in 0..60 {
                let x = ((i * 977) % 1280) as f32;
                let y = ((i * 613) % 380) as f32;
                let tw = ((time * 2.0 + i as f32).sin() * 0.5 + 0.5) * 0.7 + 0.3;
                gfx::rect(x, y, 2.0, 2.0, rgba(255, 250, 230, (200.0 * tw) as u8));
            }
            gfx::circle(sx, sy + 20.0, 90.0, rgba(220, 210, 255, 30));
            gfx::circle(sx, sy + 20.0, 54.0, rgb(250, 244, 220));
            gfx::circle(sx + 18.0, sy + 6.0, 46.0, px_color(ta.sky_top)); // crescent
        }
        Theme::Cliffs => {
            gfx::circle(sx - 100.0, sy + 250.0, 120.0, rgba(255, 210, 140, 60));
            gfx::circle(sx - 100.0, sy + 250.0, 70.0, rgb(255, 220, 150));
        }
        Theme::Desert => {
            // a big blazing sun
            gfx::circle(sx, sy, 110.0, rgba(255, 236, 170, 50));
            gfx::circle(sx, sy, 62.0, rgb(255, 240, 190));
        }
        Theme::Ember => {
            // a red moon behind the smoke
            gfx::circle(sx, sy + 40.0, 90.0, rgba(255, 120, 60, 40));
            gfx::circle(sx, sy + 40.0, 50.0, rgb(240, 110, 70));
        }
        Theme::Frost | Theme::Sky => {
            gfx::circle(sx, sy, 70.0, rgba(255, 255, 240, 60));
            gfx::circle(sx, sy, 44.0, rgb(255, 252, 230));
        }
        Theme::Arcane => {
            // dense stars, a ringed planet and falling stars
            for i in 0..90 {
                let x = ((i * 977) % 1280) as f32;
                let y = ((i * 613) % 420) as f32;
                let tw = ((time * 1.6 + i as f32 * 0.7).sin() * 0.5 + 0.5) * 0.8 + 0.2;
                let col = if i % 5 == 0 { rgba(170, 230, 255, (230.0 * tw) as u8) } else { rgba(255, 250, 240, (190.0 * tw) as u8) };
                gfx::rect(x, y, 2.0, 2.0, col);
            }
            gfx::circle(sx - 60.0, sy + 40.0, 60.0, rgba(200, 160, 255, 40));
            gfx::circle(sx - 60.0, sy + 40.0, 38.0, rgb(230, 196, 255));
            gfx::ellipse(sx - 60.0, sy + 40.0, 70.0, 10.0, -0.2, rgba(255, 240, 255, 150));
            for i in 0..3 {
                let t = (time * 0.35 + i as f32 * 0.37) % 1.0;
                let x = 200.0 + i as f32 * 380.0 + t * 260.0;
                let y = 40.0 + t * 200.0;
                gfx::line(x, y, x - 40.0, y - 30.0, 2.0, rgba(200, 240, 255, (220.0 * (1.0 - t)) as u8));
            }
        }
        _ => {
            gfx::circle(sx, sy, 64.0, rgba(255, 250, 220, 60));
            gfx::circle(sx, sy, 42.0, rgb(255, 246, 206));
        }
    }
    if !matches!(theme, Theme::Woods | Theme::Arcane | Theme::Ember) {
        let tint = if theme == Theme::Cliffs { Color::new(1.0, 0.82, 0.8, 1.0) } else { WHITE };
        for i in 0..6 {
            let t = sprites::get(&format!("cloud{}", i % 3), || art::cloud(i % 3));
            let span = gfx::VW + 400.0;
            let speed = 8.0 + (i as f32 * 3.7) % 9.0;
            let x = ((i as f32 * 271.0 + time * speed - cam.x * 0.1) % span + span) % span - 200.0;
            let y = 40.0 + (i as f32 * 53.0) % 160.0 - cam.y * 0.05;
            gfx::tex(&t, x, y, 1.0, false, tint);
        }
    }
    // parallax strips (each 640 canvas px wide, drawn repeatedly to wrap)
    let layer = |key: String, make: &dyn Fn() -> crate::pixel::Canvas, factor: f32, base_y: f32| {
        let t = sprites::get(&key, make);
        let w = t.width() * 2.0;
        let off = ((cam.x * gfx::WORLD_PX * factor * 2.0) % w + w) % w;
        let y = base_y - cam.y * gfx::WORLD_PX * factor * 2.0;
        let mut x = -off;
        while x < gfx::VW {
            gfx::tex(&t, x, y, 1.0, false, WHITE);
            x += w;
        }
        y + t.height() * 2.0
    };
    let k = tkey(theme);
    layer(format!("far_{}", k), &|| art::bg_far(theme), 0.12, 180.0);
    layer(format!("mid_{}", k), &|| art::bg_trees(theme, false), 0.3, 300.0);
    if theme == Theme::Woods {
        // drifting mist between the tree layers
        for i in 0..5 {
            let x = ((i as f32 * 300.0 + time * 12.0 - cam.x * 0.4) % 1600.0 + 1600.0) % 1600.0 - 200.0;
            gfx::ellipse(x, 470.0 - cam.y * 0.4, 200.0, 26.0, 0.0, rgba(180, 170, 230, 26));
        }
    }
    let b3 = layer(format!("near_{}", k), &|| art::bg_trees(theme, true), 0.55, 380.0);
    gfx::rect(0.0, b3 - 2.0, gfx::VW, gfx::VH, px_color(ta.near));
}

fn interior_background(cam: &Camera) {
    let wall = sprites::get("in_wall", crate::townart::interior_wall);
    let w = wall.width() * 2.0;
    let off = ((cam.x * gfx::WORLD_PX * 2.0) % w + w) % w;
    // The wall is anchored to the world floor so furniture lines up.
    let floor_screen = (galebound_core::world::GROUND - cam.y) * gfx::WORLD_PX * 2.0;
    let y = floor_screen - wall.height() * 2.0;
    gfx::rect(0.0, 0.0, gfx::VW, gfx::VH, rgb(70, 46, 30));
    let mut x = -off;
    while x < gfx::VW {
        gfx::tex(&wall, x, y, 1.0, false, WHITE);
        x += w;
    }
}

/// Night tint and glowing lights for dark zones, drawn after the world.
pub fn lighting(sim: &Sim, cam: &Camera, time: f32) {
    match sim.map.theme {
        Theme::Woods => {
            gfx::clear_cam();
            gfx::rect(0.0, 0.0, gfx::VW, gfx::VH, rgba(20, 16, 60, 70));
            gfx::set_cam(cam.x, cam.y);
            for d in &sim.map.decos {
                if d.kind == DecoKind::LanternPost {
                    let r = 46.0 + (time * 3.0 + d.x).sin() * 3.0;
                    gfx::circle(d.x + u(10.0), d.y - u(74.0), r, rgba(200, 170, 255, 34));
                }
            }
            for e in &sim.enemies {
                if e.kind == galebound_core::world::EnemyKind::Lanternwisp || e.kind == galebound_core::world::EnemyKind::Gravecap {
                    let c = e.body.center();
                    gfx::circle(c.x, c.y, 30.0, rgba(255, 200, 140, 18));
                }
            }
            // fireflies
            for i in 0..18 {
                let x = cam.x + ((i * 211) as f32 + (time * 0.4 + i as f32).sin() * 40.0) % gfx::view_w();
                let y = cam.y + 120.0 + ((i * 97) as f32 + (time * 0.7 + i as f32 * 1.3).cos() * 30.0) % (gfx::view_h() - 140.0);
                let a = ((time * 2.0 + i as f32).sin() * 0.5 + 0.5) * 220.0;
                gfx::rect(x, y, u(1.0), u(1.0), rgba(220, 255, 140, a as u8));
            }
        }
        Theme::Cliffs => {
            gfx::clear_cam();
            gfx::rect(0.0, 0.0, gfx::VW, gfx::VH, rgba(255, 140, 80, 18));
        }
        Theme::Desert => {
            gfx::clear_cam();
            gfx::rect(0.0, 0.0, gfx::VW, gfx::VH, rgba(255, 190, 110, 18));
        }
        Theme::Frost => {
            // falling snow
            gfx::clear_cam();
            gfx::rect(0.0, 0.0, gfx::VW, gfx::VH, rgba(170, 200, 255, 22));
            for i in 0..70 {
                let x = ((i * 211) as f32 + (time * 0.8 + i as f32).sin() * 30.0 + time * 14.0).rem_euclid(gfx::VW);
                let y = ((i * 97) as f32 + time * (40.0 + (i % 5) as f32 * 12.0)).rem_euclid(gfx::VH);
                gfx::rect(x, y, 3.0, 3.0, rgba(255, 255, 255, 200));
            }
        }
        Theme::Ember => {
            // red haze and rising embers
            gfx::clear_cam();
            gfx::rect(0.0, 0.0, gfx::VW, gfx::VH, rgba(140, 30, 10, 34));
            for i in 0..40 {
                let x = ((i * 233) as f32 + (time * 0.6 + i as f32).sin() * 40.0).rem_euclid(gfx::VW);
                let y = gfx::VH - ((time * (30.0 + (i % 4) as f32 * 14.0) + i as f32 * 61.0).rem_euclid(gfx::VH));
                let a = ((time * 3.0 + i as f32).sin() * 0.5 + 0.5) * 220.0;
                gfx::rect(x, y, 3.0, 3.0, rgba(255, 150, 60, a as u8));
            }
        }
        Theme::Sky => {
            gfx::clear_cam();
            gfx::rect(0.0, 0.0, gfx::VW, gfx::VH, rgba(200, 220, 255, 14));
        }
        Theme::Arcane => {
            gfx::clear_cam();
            gfx::rect(0.0, 0.0, gfx::VW, gfx::VH, rgba(60, 30, 120, 40));
            gfx::set_cam(cam.x, cam.y);
            for d in &sim.map.decos {
                if matches!(d.kind, DecoKind::Crystal | DecoKind::LanternPost) {
                    let r = 40.0 + (time * 2.0 + d.x).sin() * 4.0;
                    gfx::circle(d.x, d.y - 40.0, r, rgba(140, 230, 255, 26));
                }
            }
            // drifting motes
            for i in 0..16 {
                let x = cam.x + ((i * 233) as f32 + (time * 0.3 + i as f32).sin() * 30.0) % gfx::view_w();
                let y = cam.y + gfx::view_h() - ((time * 18.0 + i as f32 * 53.0) % gfx::view_h());
                gfx::rect(x, y, u(1.0), u(1.0), rgba(180, 240, 255, 200));
            }
        }
        Theme::Interior => {
            gfx::set_cam(cam.x, cam.y);
            for d in &sim.map.decos {
                match d.kind {
                    DecoKind::WallTorch => {
                        let r = 50.0 + (time * 7.0 + d.x).sin() * 3.0;
                        gfx::circle(d.x, d.y - u(30.0), r, rgba(255, 190, 110, 30));
                    }
                    DecoKind::Fireplace => {
                        let r = 110.0 + (time * 5.0).sin() * 6.0;
                        gfx::circle(d.x, d.y - u(30.0), r, rgba(255, 160, 80, 26));
                    }
                    _ => {}
                }
            }
            gfx::clear_cam();
            gfx::rect(0.0, 0.0, gfx::VW, gfx::VH, rgba(60, 30, 10, 26));
        }
        _ => {}
    }
}

// ---------------------------------------------------------------- terrain

fn visible(r: &Aabb, cam: &Camera) -> bool {
    r.right() > cam.x - 50.0 && r.left() < cam.x + gfx::view_w() + 50.0 && r.bottom() > cam.y - 50.0 && r.top() < cam.y + gfx::view_h() + 50.0
}

/// Fill a world rect with a repeating texture aligned to the global pixel grid.
fn tile_fill(t: &Texture2D, l: f32, top: f32, r: f32, bottom: f32, align_y: f32) {
    let tw = t.width();
    let th = t.height();
    // work in canvas pixels
    let (pl, pr) = ((l * gfx::WORLD_PX).round(), (r * gfx::WORLD_PX).round());
    let (pt, pb) = ((top * gfx::WORLD_PX).round(), (bottom * gfx::WORLD_PX).round());
    let ay = (align_y * gfx::WORLD_PX).round();
    let mut y = pt;
    while y < pb {
        let ty = ((y - ay) % th + th) % th;
        let h = (th - ty).min(pb - y);
        let mut x = pl;
        while x < pr {
            let tx = ((x % tw) + tw) % tw;
            let w = (tw - tx).min(pr - x);
            gfx::tex_part(t, x / gfx::WORLD_PX, y / gfx::WORLD_PX, (tx, ty, w, h), 1.0, WHITE);
            x += w;
        }
        y += h;
    }
}

pub fn terrain(sim: &Sim, cam: &Camera) {
    gfx::set_cam(cam.x, cam.y);
    let town = matches!(sim.map.theme, Theme::Town | Theme::Arcane);
    let stone = sprites::cobble(sim.map.theme);
    let ink = rgb(26, 18, 30);
    for s in &sim.map.solids {
        if !visible(s, cam) || s.top() <= 0.0 {
            continue;
        }
        let l = s.left().max(cam.x - 20.0);
        let r = s.right().min(cam.x + gfx::view_w() + 20.0);
        let bottom = (cam.y + gfx::view_h() + 10.0).min(s.bottom());
        tile_fill(&stone, l, s.top(), r, bottom, s.top());
        // outline exposed ledge sides
        let p1 = u(1.0);
        if s.left() > cam.x - 20.0 && s.left() > 0.0 {
            gfx::rect(s.left() - p1, s.top(), p1, bottom - s.top(), ink);
            gfx::rect(s.left(), s.top(), u(3.0), bottom - s.top(), rgba(0, 0, 0, 50));
        }
        if s.right() < cam.x + gfx::view_w() + 20.0 {
            gfx::rect(s.right(), s.top(), p1, bottom - s.top(), ink);
            gfx::rect(s.right() - u(4.0), s.top(), u(4.0), bottom - s.top(), rgba(0, 0, 0, 70));
        }
        // top surface
        if sim.map.theme == Theme::Interior {
            let fl = sprites::get("in_floor", crate::townart::interior_floor);
            tile_fill(&fl, l, s.top(), r, bottom, s.top());
            gfx::rect(l, s.top(), r - l, u(1.0), ink);
        } else if town && s.top() >= 600.0 {
            let rd = sprites::road();
            tile_fill(&rd, l, s.top(), r, s.top() + u(rd.height()), s.top());
        } else {
            let g = sprites::grass(sim.map.theme);
            tile_fill(&g, l, s.top() - u(6.0), r, s.top() - u(6.0) + u(g.height()), s.top() - u(6.0));
        }
    }
    // One-way platforms: wooden planks
    for p in &sim.map.platforms {
        if !visible(p, cam) {
            continue;
        }
        let wpx = (p.w * gfx::WORLD_PX).round() as i32;
        let t = sprites::get(&format!("plank{}", wpx), || art::plank(wpx));
        gfx::tex(&t, p.left() - u(1.0), p.top() - u(1.0), 1.0, false, WHITE);
    }
    // Gates
    for g in &sim.map.gates {
        let closed = !sim.save.has_flag(g.open_flag);
        let t = sprites::get(if closed { "gate_c" } else { "gate_o" }, || art::gate(closed));
        sprite_at(&t, g.solid.center().x, g.solid.bottom() + u(2.0), false, WHITE);
    }
}

/// Ladders, ropes and vines (drawn behind the actors).
pub fn climbs(sim: &Sim, cam: &Camera) {
    use galebound_core::world::ClimbKind;
    gfx::set_cam(cam.x, cam.y);
    for c in &sim.map.climbs {
        let r = Aabb::new(c.x - 20.0, c.top - 20.0, 40.0, c.bottom - c.top + 20.0);
        if !visible(&r, cam) {
            continue;
        }
        let (key, make): (&str, fn() -> crate::pixel::Canvas) = match c.kind {
            ClimbKind::Ladder => ("ladder", crate::townart::ladder_tile),
            ClimbKind::Rope => ("rope", crate::townart::rope_tile),
            ClimbKind::Vine => ("vine", crate::townart::vine_tile),
        };
        let t = sprites::get(key, make);
        let hw = u(t.width()) * 0.5;
        let top = if c.kind == ClimbKind::Ladder { c.top - u(10.0) } else { c.top };
        tile_fill(&t, c.x - hw, top, c.x + hw, c.bottom, top);
        if c.kind == ClimbKind::Rope {
            let a = sprites::get("rope_anchor", crate::townart::rope_anchor);
            sprite_at(&a, c.x, c.top + u(4.0), false, WHITE);
        }
    }
}

// ---------------------------------------------------------------- decorations

pub fn decos(sim: &Sim, cam: &Camera, layer: u8, time: f32) {
    gfx::set_cam(cam.x, cam.y);
    for d in sim.map.decos.iter().filter(|d| d.layer == layer) {
        if d.x < cam.x - 300.0 || d.x > cam.x + gfx::view_w() + 300.0 {
            continue;
        }
        deco(d, time, sim.map.theme);
    }
}

fn crest(theme: Theme) -> crate::pixel::Px {
    use crate::pixel::rgb as prgb;
    match theme {
        Theme::Town => prgb(176, 48, 52),
        Theme::Woods => prgb(96, 60, 150),
        Theme::Cliffs => prgb(230, 186, 50),
        Theme::Glade => prgb(70, 150, 80),
        Theme::Desert => prgb(200, 110, 50),
        Theme::Frost => prgb(110, 170, 230),
        Theme::Ember => prgb(220, 70, 40),
        _ => prgb(70, 170, 220),
    }
}

fn deco(d: &Deco, time: f32, theme: Theme) {
    let seed = (d.x as i32) % 7;
    let anim = |fps: f32, n: i32| ((time * fps) as i32).rem_euclid(n);
    let t: Texture2D = match d.kind {
        DecoKind::TreeTall => sprites::get(&format!("treeT{}", seed % 3), || art::tree_tall(seed % 3)),
        DecoKind::TreeRound => sprites::get(&format!("treeR{}", seed % 3), || art::tree_round(seed % 3)),
        DecoKind::Bush => sprites::get("bush", art::bush),
        DecoKind::Rock => sprites::get("rock", art::rock),
        DecoKind::Flowers => sprites::get(&format!("flowers{}", seed % 4), || art::flowers(seed % 4)),
        DecoKind::Fence => sprites::get("fence", art::fence),
        DecoKind::Tent => sprites::get("tent", art::tent),
        DecoKind::Campfire => {
            let f = anim(8.0, 4);
            let t = sprites::get(&format!("fire{}", f), || art::campfire(f));
            gfx::circle(d.x, d.y - 16.0, 46.0, rgba(255, 180, 80, 28));
            t
        }
        DecoKind::House => sprites::get("house", || art::house(false)),
        DecoKind::Shop => sprites::get("shop", || art::house(true)),
        DecoKind::Lamp => {
            gfx::circle(d.x, d.y - u(84.0), 30.0, rgba(255, 230, 150, 34));
            sprites::get("lamp", art::lamp)
        }
        DecoKind::Well => sprites::get("well", art::well),
        DecoKind::Banner => {
            let f = anim(3.0, 4);
            sprites::get(&format!("banner{}", f), || art::banner(f))
        }
        DecoKind::Windmill => {
            let base = sprites::get("windmill", art::windmill_base);
            sprite_at(&base, d.x, d.y + u(1.0), false, WHITE);
            let f = anim(6.0, 8);
            let blades = sprites::get(&format!("blades{}", f), || art::windmill_blades(f));
            let hub_y = d.y - u(base.height() - 32.0);
            gfx::tex(&blades, d.x - u(60.0), hub_y - u(60.0), 1.0, false, WHITE);
            return;
        }
        DecoKind::Crate => sprites::get("crate", art::crate_box),
        DecoKind::Haystack => sprites::get("haystack", art::haystack),
        DecoKind::Scarecrow => sprites::get("scarecrow", art::scarecrow),
        DecoKind::DeadTree => sprites::get(&format!("deadtree{}", seed % 2), || art::dead_tree(seed % 2)),
        DecoKind::Gravestone => sprites::get("gravestone", art::gravestone),
        DecoKind::Cross => sprites::get("cross", art::wooden_cross),
        DecoKind::Mushrooms => sprites::get("mushrooms", art::mushrooms),
        DecoKind::Pine => sprites::get("pine", art::pine),
        DecoKind::LanternPost => sprites::get("lanternpost", art::lantern_post),
        DecoKind::Crystal => {
            gfx::circle(d.x, d.y - 30.0, 40.0, rgba(140, 230, 255, 22));
            sprites::get("crystal", art::crystal)
        }
        DecoKind::StoreGeneral => sprites::get(&format!("store0_{}", tkey(theme)), || crate::townart::storefront(theme, 0)),
        DecoKind::StoreWeapons => sprites::get(&format!("store1_{}", tkey(theme)), || crate::townart::storefront(theme, 1)),
        DecoKind::StoreArmor => sprites::get(&format!("store2_{}", tkey(theme)), || crate::townart::storefront(theme, 2)),
        DecoKind::Inn => sprites::get(&format!("inn_{}", tkey(theme)), || crate::townart::inn(theme)),
        DecoKind::ClassHall => sprites::get(&format!("hall_{}", tkey(theme)), || crate::townart::class_hall(theme, crest(theme))),
        DecoKind::Signpost => sprites::get("signpost", crate::townart::signpost),
        DecoKind::Counter => sprites::get("counter", crate::townart::counter),
        DecoKind::Shelf => sprites::get("shelf", crate::townart::shelf),
        DecoKind::WeaponRack => sprites::get("weaponrack", crate::townart::weapon_rack),
        DecoKind::ArmorStand => sprites::get("armorstand", crate::townart::armor_stand),
        DecoKind::Rug => sprites::get("rug", crate::townart::rug),
        DecoKind::Fireplace => {
            let f = anim(8.0, 6);
            sprites::get(&format!("fireplace{}", f), || crate::townart::fireplace(f))
        }
        DecoKind::WallTorch => {
            let f = anim(8.0, 4);
            sprites::get(&format!("walltorch{}", f), || crate::townart::wall_torch(f))
        }
        DecoKind::Window => sprites::get("inwindow", crate::townart::indoor_window),
        DecoKind::Barrel => sprites::get("barrel", crate::townart::barrel),
        DecoKind::Bed => sprites::get("bed", crate::townart::bed),
        DecoKind::Chest => sprites::get("chest", crate::townart::chest),
        DecoKind::Stove => {
            let f = anim(8.0, 4);
            gfx::circle(d.x, d.y - 30.0, 40.0, rgba(255, 170, 80, 26));
            sprites::get(&format!("stove{}", f), || crate::townart::stove(f))
        }
        DecoKind::Workbench => sprites::get("workbench", crate::townart::workbench),
        DecoKind::Table => sprites::get("table", crate::townart::table),
        DecoKind::Bookshelf => sprites::get("bookshelf", crate::townart::bookshelf),
        DecoKind::Plant => sprites::get("plant", crate::townart::plant),
        DecoKind::Trophy => {
            gfx::circle(d.x, d.y - 40.0, 34.0, rgba(255, 90, 80, 24));
            sprites::get("trophy", crate::townart::trophy)
        }
        DecoKind::VaultDoor => {
            let f = anim(4.0, 8);
            gfx::circle(d.x, d.y - 80.0, 70.0, rgba(140, 220, 255, 22));
            sprites::get(&format!("vaultdoor{}", f), || crate::townart::vault_door(f))
        }
    };
    sprite_at(&t, d.x, d.y + u(1.0), false, WHITE);
}

/// Grass tufts along ledge edges (the reference's lush cliff tops).
pub fn tufts(sim: &Sim, cam: &Camera) {
    if matches!(sim.map.theme, Theme::Town | Theme::Interior | Theme::Arcane) {
        return;
    }
    gfx::set_cam(cam.x, cam.y);
    for (i, s) in sim.map.solids.iter().enumerate() {
        if !visible(s, cam) || s.top() <= 0.0 || s.w < 200.0 {
            continue;
        }
        let mut x = s.left() + 30.0 + (i as f32 * 37.0) % 60.0;
        let mut k = i as i32;
        while x < s.right() - 30.0 {
            if x > cam.x - 60.0 && x < cam.x + gfx::view_w() + 60.0 && (k * 7) % 5 < 2 {
                let t = sprites::get(&format!("tuft{}", k % 3), || art::tuft(k % 3));
                sprite_at(&t, x, s.top() + u(1.0), k % 2 == 0, WHITE);
            }
            x += 170.0 + ((k * 53) % 90) as f32;
            k += 1;
        }
    }
}

// ---------------------------------------------------------------- signs & portals

pub fn signs_and_portals(sim: &Sim, cam: &Camera, time: f32) {
    gfx::set_cam(cam.x, cam.y);
    let sign = sprites::get("sign", art::sign);
    for s in &sim.map.signs {
        sprite_at(&sign, s.x, s.y + u(1.0), false, WHITE);
    }
    for p in &sim.map.portals {
        let open = p.requires_flag.map(|f| sim.save.has_flag(f)).unwrap_or(true);
        let c = p.area.center();
        let base = if open { rgb(120, 200, 255) } else { rgb(150, 150, 160) };
        for i in 0..4 {
            let ph = time * 3.0 + i as f32 * 1.4;
            let ry = 54.0 - i as f32 * 9.0 + ph.sin() * 3.0;
            gfx::ellipse(c.x, c.y + 4.0, 26.0 - i as f32 * 5.0, ry, 0.0, alpha(base, 0.2 + i as f32 * 0.14));
        }
        if open {
            for i in 0..6 {
                let t = (time * 0.8 + i as f32 / 6.0).fract();
                gfx::rect(c.x + ((i as f32) * 2.1 + time).sin() * 18.0, c.y + 50.0 - t * 110.0, u(1.0), u(2.0), alpha(rgb(255, 255, 255), 1.0 - t));
            }
        } else {
            gfx::rrect(c.x - 9.0, c.y - 4.0, 18.0, 14.0, 3.0, rgb(214, 180, 80));
            gfx::rect(c.x - 1.0, c.y, u(1.0), u(3.0), rgb(26, 18, 30));
        }
        gfx::text_centered_outlined(p.label, c.x, p.area.top() - 30.0, 16.0, rgb(255, 255, 255));
    }
}

// ---------------------------------------------------------------- characters

/// An admin's name tag: a gold "GM" badge with a little crown in front of
/// a gold-edged name plate.
fn admin_tag(x: f32, y: f32, name: &str, title: Option<&str>) {
    let nw = gfx::text_w(name, 16.0) + u(6.0);
    let bw = u(17.0);
    let w = nw + bw + u(2.0);
    let left = x - w * 0.5;
    let t = (macroquad::time::get_time() as f32 * 2.0).sin() * 0.5 + 0.5;
    gfx::rrect(left - u(1.0), y + u(1.0), w + u(2.0), u(13.0), u(3.0), rgba(255, 200, 70, (120.0 + 80.0 * t) as u8));
    gfx::rrect(left, y + u(2.0), w, u(11.0), u(2.0), rgba(40, 26, 10, 230));
    // badge
    gfx::rrect(left + u(1.0), y + u(3.0), bw - u(1.0), u(9.0), u(2.0), rgb(236, 176, 40));
    let cx = left + u(4.5);
    let cy = y + u(7.5);
    gfx::tri(cx - u(2.5), cy + u(1.5), cx - u(2.5), cy - u(2.0), cx - u(0.8), cy, rgb(255, 250, 220));
    gfx::tri(cx - u(1.2), cy + u(1.5), cx, cy - u(2.6), cx + u(1.2), cy + u(1.5), rgb(255, 250, 220));
    gfx::tri(cx + u(2.5), cy + u(1.5), cx + u(2.5), cy - u(2.0), cx + u(0.8), cy, rgb(255, 250, 220));
    gfx::rect(cx - u(2.5), cy + u(1.0), u(5.0), u(1.2), rgb(255, 250, 220));
    gfx::text("GM", left + u(8.0), y + u(4.0), 14.0, rgb(60, 30, 0));
    gfx::text(name, left + bw + u(4.0), y + u(4.0), 16.0, rgb(255, 214, 90));
    if let Some(t) = title {
        name_tag_title(x, y, t);
    }
}

fn name_tag(x: f32, y: f32, name: &str, title: Option<&str>, col: Color) {
    let w = gfx::text_w(name, 16.0) + u(6.0);
    gfx::rrect(x - w * 0.5, y + u(2.0), w, u(11.0), u(2.0), rgba(24, 22, 40, 200));
    gfx::text_centered(name, x, y + u(4.0), 16.0, col);
    if let Some(t) = title {
        name_tag_title(x, y, t);
    }
}

fn name_tag_title(x: f32, y: f32, t: &str) {
    {
        if let Some(g) = t.strip_prefix('<').and_then(|t| t.strip_suffix('>')) {
            // guild tag: a little emblem and the guild's name, like the classics
            let gw = gfx::text_w(g, 14.0) + u(12.0);
            let gy = y + u(14.5);
            gfx::rrect(x - gw * 0.5, gy, gw, u(9.5), u(2.0), rgba(30, 26, 20, 200));
            let ex = x - gw * 0.5 + u(4.0);
            gfx::circle(ex, gy + u(4.75), u(2.6), rgb(196, 60, 64));
            gfx::circle(ex, gy + u(4.75), u(1.3), rgb(255, 230, 140));
            gfx::text_centered(g, x + u(3.0), gy + u(1.6), 14.0, rgb(255, 226, 110));
        } else {
            gfx::text_centered_outlined(t, x, y + u(15.0), 16.0, rgb(224, 228, 240));
        }
    }
}

pub fn npcs(sim: &Sim, cam: &Camera, time: f32) {
    gfx::set_cam(cam.x, cam.y);
    let talking_to = sim.dialog.as_ref().map(|d| d.dialog.npc_id);
    let near = sim.nearest_npc();
    for (i, n) in sim.map.npcs.iter().enumerate() {
        if n.x < cam.x - 100.0 || n.x > cam.x + gfx::view_w() + 100.0 {
            continue;
        }
        let facing = if talking_to == Some(n.id) {
            (sim.player.body.center().x - n.x).signum()
        } else if n.faces_left {
            -1.0
        } else {
            1.0
        };
        if n.id.starts_with("waystone_") {
            let f = ((time * 6.0) as i32).rem_euclid(6);
            let t = sprites::get(&format!("waystone{}", f), || crate::townart::waystone(f));
            gfx::circle(n.x, n.y - u(40.0), 60.0 + (time * 2.0).sin() * 4.0, rgba(120, 220, 255, 30));
            sprite_at(&t, n.x, n.y + u(1.0), false, WHITE);
            name_tag(n.x, n.y, n.name, Some(n.title), rgb(160, 230, 255));
            if near == Some(i) && sim.dialog.is_none() {
                let key = sim.settings.layout.key_label(galebound_core::input::Action::Interact);
                gfx::text_centered_outlined(&format!("[{}] Travel", key), n.x, n.y - u(96.0), 16.0, rgb(255, 255, 255));
            }
            continue;
        }
        let talking = talking_to == Some(n.id);
        let anim = if talking { Anim::Talk } else { Anim::Idle };
        let frame = charsprite::frame_for_loop(anim, time + i as f32 * 0.37);
        let t = sprites::character(&n.look, None, anim, frame);
        char_shadow(n.x, n.y, 1.0);
        draw_char(&t, n.x, n.y, facing < 0.0, WHITE);
        name_tag(n.x, n.y, n.name, Some(n.title), rgb(255, 236, 160));
        if let Some(m) = quests::npc_marker(n.id, &sim.save) {
            let by = n.y - u(66.0) + ((time * 4.0).sin() * 2.0).round() * u(1.0);
            let col = if m == '!' { rgb(255, 210, 60) } else { rgb(120, 220, 255) };
            gfx::rrect(n.x - u(5.0), by - u(6.0), u(10.0), u(12.0), u(3.0), rgb(26, 18, 30));
            gfx::rrect(n.x - u(4.0), by - u(5.0), u(8.0), u(10.0), u(2.0), col);
            gfx::text_centered(&m.to_string(), n.x + u(0.5), by - u(4.0), 16.0, rgb(40, 30, 20));
        }
        if near == Some(i) && sim.dialog.is_none() {
            let key = sim.settings.layout.key_label(galebound_core::input::Action::Interact);
            gfx::text_centered_outlined(&format!("[{}] Talk", key), n.x, n.y - u(84.0), 16.0, rgb(255, 255, 255));
        }
    }
}

/// Draw a character sprite with feet at world (x, y).
/// Soft ground shadow under a character standing at (x, y).
fn char_shadow(x: f32, y: f32, alpha_k: f32) {
    gfx::ellipse(x, y, u(10.0), u(2.5), 0.0, rgba(0, 0, 0, (64.0 * alpha_k) as u8));
}

fn draw_char(t: &Texture2D, x: f32, y: f32, flip: bool, tint: Color) {
    let fx = charsprite::FEET_X as f32;
    let fy = charsprite::FEET_Y as f32;
    // When flipped, the sprite mirrors around its centre (32), which is the feet column.
    gfx::tex(t, x - u(fx), y - u(fy), 1.0, flip, tint);
}

/// Draw a character in UI space (menus): feet at (cx, feet_y), each sprite
/// pixel k canvas pixels big.
pub fn char_ui(look: &Appearance, weapon: Option<galebound_core::classes::Weapon>, anim: Anim, frame: u8, cx: f32, feet_y: f32, k: f32, flip: bool) {
    gfx::clear_cam();
    let t = sprites::character(look, weapon, anim, frame);
    let fx = charsprite::FEET_X as f32;
    let fy = charsprite::FEET_Y as f32;
    gfx::ellipse(cx, feet_y, gfx::px_to_units(14.0 * k), gfx::px_to_units(3.0 * k), 0.0, rgba(0, 0, 0, 50));
    gfx::tex(&t, cx - gfx::px_to_units(fx * k), feet_y - gfx::px_to_units(fy * k), k, flip, WHITE);
}

/// Animation + frame for the player from simulation state.
pub fn player_anim(p: &Player) -> (Anim, u8) {
    match p.state {
        PState::Dead => (Anim::Dead, 0),
        PState::Dodge => (Anim::Dodge, if p.state_t < 0.12 { 0 } else { 1 }),
        PState::Hitstun => (Anim::Hurt, 0),
        PState::Climb => (Anim::Climb, ((p.climb_t * 6.0) as u8) % 4),
        PState::Attack => {
            let a = p.attack.as_ref().unwrap();
            let m = charsprite::motion_for(p.class, a.kind);
            let d = &a.def;
            let (phase, prog) = match a.phase() {
                AttackPhase::Startup => (AttackPhase::Startup, a.t / d.startup.max(1e-3)),
                AttackPhase::Active => (AttackPhase::Active, (a.t - d.startup) / d.active.max(1e-3)),
                AttackPhase::Recovery => (AttackPhase::Recovery, (a.t - d.startup - d.active) / d.recovery.max(1e-3)),
            };
            // Hundred Fists and other rapid multi-hit jabs: a new punch on
            // every hit, alternating front and back fist.
            let hits = match a.skill.map(|s| &s.effect) {
                Some(galebound_core::skills::SkillEffect::Strike { hits, .. }) => *hits,
                _ => 1,
            };
            if hits > 1 && m == charsprite::Motion::Jab {
                let frame = match phase {
                    AttackPhase::Startup => 0,
                    AttackPhase::Recovery => 3,
                    AttackPhase::Active => {
                        let per = prog.clamp(0.0, 0.999) * hits as f32;
                        let n = per as u32;
                        if per.fract() < 0.6 { 1 + (n % 2) as u8 } else { 0 }
                    }
                };
                return (Anim::Attack(charsprite::Motion::Flurry), frame);
            }
            (Anim::Attack(m), charsprite::attack_frame(phase, prog))
        }
        PState::Normal => {
            if !p.body.on_ground {
                if p.body.vel.y < 0.0 {
                    (Anim::JumpUp, if p.body.vel.y < -500.0 { 0 } else { 1 })
                } else {
                    (Anim::Fall, if p.body.vel.y < 400.0 { 0 } else { 1 })
                }
            } else if p.body.vel.x.abs() > 40.0 {
                (Anim::Run, charsprite::frame_for_loop(Anim::Run, p.anim_t))
            } else if p.since_combat < crate::poses::BATTLE_STANCE_SECONDS {
                (Anim::Battle, charsprite::frame_for_loop(Anim::Battle, p.anim_t))
            } else {
                (Anim::Idle, charsprite::frame_for_loop(Anim::Idle, p.anim_t))
            }
        }
    }
}

pub fn player(sim: &Sim, cam: &Camera, look: &Appearance) {
    gfx::set_cam(cam.x, cam.y);
    let p = &sim.player;
    // Defeated: after the fall animation, a tombstone drops where you lie.
    if p.state == PState::Dead && p.state_t > galebound_core::sim::TOMB_DELAY {
        let f = p.body.feet();
        let t = ((p.state_t - galebound_core::sim::TOMB_DELAY) / 0.28).min(1.0);
        // falls in from above with a small bounce
        let drop = if t < 1.0 { (1.0 - t * t) * u(70.0) } else { 0.0 };
        let land = p.state_t - galebound_core::sim::TOMB_DELAY - 0.28;
        let bounce = if (0.0..0.15).contains(&land) { (land / 0.15 * std::f32::consts::PI).sin() * u(3.0) } else { 0.0 };
        let tex = sprites::get("tombstone", crate::lootart::tombstone);
        char_shadow(f.x, f.y, 1.0);
        sprite_at(&tex, f.x, f.y - drop - bounce, false, WHITE);
        name_tag(f.x, f.y, &sim.save.name, None, rgb(200, 200, 210));
        return;
    }
    let flick = p.invuln > 0.0 && p.state != PState::Dodge && ((sim.time * 20.0) as i32 % 2 == 0);
    let a = if flick { 0.35 } else { 1.0 };
    let f = p.body.feet();
    let (anim, frame) = player_anim(p);
    // Weapons are stowed while climbing.
    let weapon = if p.state == PState::Climb { None } else { sim.save.inventory.equipped(galebound_core::items::EquipSlot::Weapon).and_then(|d| galebound_core::items::weapon_art(d.id)) };
    let tex = sprites::character(look, weapon, anim, frame);
    let flip = p.facing < 0.0;
    // shadow stays on the ground below the player and shrinks with height
    let ground = galebound_core::physics::ground_below(&p.body, &sim.level_ref());
    let height = (ground - f.y).max(0.0);
    char_shadow(f.x, ground, (1.0 - height / 220.0).clamp(0.25, 1.0));
    if p.state == PState::Dodge {
        for i in 1..4 {
            let off = -p.body.vel.x * 0.014 * i as f32;
            let offy = -p.body.vel.y * 0.014 * i as f32;
            draw_char(&tex, f.x + off, f.y + offy, flip, Color::new(0.6, 0.85, 1.0, 0.35 / i as f32));
        }
    }
    draw_char(&tex, f.x, f.y, flip, Color::new(1.0, 1.0, 1.0, a));
    let tag = sim.guild.as_ref().map(|g| format!("<{}>", g.name));
    if sim.am_admin() {
        admin_tag(f.x, f.y, &sim.save.name, tag.as_deref());
    } else {
        name_tag(f.x, f.y, &sim.save.name, tag.as_deref(), rgb(255, 255, 255));
    }
}

// ---------------------------------------------------------------- enemies

pub fn enemies(sim: &Sim, cam: &Camera) {
    gfx::set_cam(cam.x, cam.y);
    for e in &sim.enemies {
        let b = &e.body;
        if b.pos.x + b.w < cam.x - 120.0 || b.pos.x > cam.x + gfx::view_w() + 120.0 {
            continue;
        }
        let a = if e.state == EState::Dying { (1.0 - e.state_t / 0.6).max(0.0) } else { 1.0 };
        let flash = e.flash > 0.0;
        let f = b.feet();
        let flip = e.facing < 0.0;
        let tc = e.kind.tint();
        let tint = Color::new(tc[0], tc[1], tc[2], a);
        let base = e.kind.art_base();
        let walk_frame = ((sim.time * 8.0) as i32).rem_euclid(4);
        let t = match base {
            EnemyKind::TrainingDummy => {
                let w = ((e.wobble * 18.0).sin() * e.wobble * 3.0).round() as i32;
                sprites::get(&format!("dummy{}{}", w, flash as u8), || {
                    let mut c = art::dummy(w);
                    if flash {
                        c.tint_all([255, 255, 255, 255]);
                    }
                    c
                })
            }
            EnemyKind::Mossling => {
                let fr = if b.on_ground { walk_frame } else if b.vel.y < 0.0 { 3 } else { 1 };
                sprites::get(&format!("moss{}{}", fr, flash as u8), || art::mossling(fr, 0.0, flash))
            }
            EnemyKind::Burrowbeak => {
                let st = match e.state {
                    EState::Telegraph => 1,
                    EState::Lunge => 2,
                    _ => 0,
                };
                let fr = if st == 0 && e.state == EState::Walk { walk_frame } else { 0 };
                sprites::get(&format!("beak{}{}{}", fr, st, flash as u8), || art::burrowbeak(fr, st, flash))
            }
            EnemyKind::Bramblehorn => {
                let st = match e.state {
                    EState::Telegraph => 1,
                    EState::Charge => 2,
                    EState::Stunned => 3,
                    _ => 0,
                };
                let fr = walk_frame;
                let en = e.boss_phase2;
                sprites::get(&format!("boar{}{}{}{}", fr, st, flash as u8, en as u8), || art::bramblehorn(fr, st, flash, en))
            }
            EnemyKind::Thunderhorn => {
                let st = match e.state {
                    EState::Telegraph => 1,
                    EState::Charge => 2,
                    EState::Stunned => 3,
                    _ => 0,
                };
                let fr = walk_frame;
                let en = e.boss_phase2;
                sprites::get(&format!("thorn{}{}{}{}", fr, st, flash as u8, en as u8), || art::thunderhorn(fr, st, flash, en))
            }
            EnemyKind::Puffwool => {
                let fr = if b.on_ground { walk_frame } else if b.vel.y < 0.0 { 3 } else { 1 };
                sprites::get(&format!("puff{}{}", fr, flash as u8), || art::puffwool(fr, flash))
            }
            EnemyKind::Gravecap => {
                let fr = if b.on_ground { walk_frame } else if b.vel.y < 0.0 { 3 } else { 1 };
                sprites::get(&format!("gcap{}{}", fr, flash as u8), || art::gravecap(fr, flash))
            }
            EnemyKind::Gustling | EnemyKind::Lanternwisp | EnemyKind::Galehawk => {
                let st = match e.state {
                    EState::Telegraph => 1,
                    EState::Lunge => 2,
                    _ => 0,
                };
                let fr = ((sim.time * 6.0) as i32).rem_euclid(4);
                let key = format!("{}{}{}{}", base.key(), fr, st, flash as u8);
                let kind = base;
                sprites::get(&key, move || match kind {
                    EnemyKind::Gustling => art::gustling(fr, st.min(1), flash),
                    EnemyKind::Lanternwisp => art::lanternwisp(fr, st.min(1), flash),
                    _ => art::galehawk(fr, st, flash),
                })
            }
            EnemyKind::Rotstump | EnemyKind::CragCrab => {
                let st = match e.state {
                    EState::Telegraph => 1,
                    EState::Lunge => 2,
                    _ => 0,
                };
                let fr = if e.state == EState::Walk || st == 1 { walk_frame } else { 0 };
                let key = format!("{}{}{}{}", base.key(), fr, st, flash as u8);
                let kind = base;
                sprites::get(&key, move || if kind == EnemyKind::Rotstump { art::rotstump(fr, st, flash) } else { art::crag_crab(fr, st, flash) })
            }
            // 0.15.0 monsters
            EnemyKind::Cactling => {
                let fr = if b.on_ground { walk_frame } else if b.vel.y < 0.0 { 3 } else { 1 };
                sprites::get(&format!("cact{}{}", fr, flash as u8), || crate::art2::cactling(fr, flash))
            }
            EnemyKind::DuneScuttler | EnemyKind::Frostling | EnemyKind::CinderSalamander | EnemyKind::FrostBat | EnemyKind::RimeGolem | EnemyKind::RimeColossus => {
                let st = match e.state {
                    EState::Telegraph | EState::Spit => 1,
                    EState::Lunge | EState::Charge => 2,
                    _ if e.state != EState::Walk && e.state != EState::Idle && base == EnemyKind::RimeColossus => 1,
                    _ => 0,
                };
                let fr = if matches!(base, EnemyKind::FrostBat) { ((sim.time * 8.0) as i32).rem_euclid(4) } else if e.state == EState::Walk || st == 1 { walk_frame } else { 0 };
                let en = e.boss_phase2;
                let key = format!("{}{}{}{}{}", base.key(), fr, st, flash as u8, en as u8);
                let kind = base;
                sprites::get(&key, move || match kind {
                    EnemyKind::DuneScuttler => crate::art2::scorpion(fr, st, flash),
                    EnemyKind::Frostling => crate::art2::yeti(fr, st, flash),
                    EnemyKind::CinderSalamander => crate::art2::salamander(fr, st, flash),
                    EnemyKind::FrostBat => crate::art2::bat(fr, st, flash),
                    EnemyKind::RimeColossus => crate::art2::golem(fr, st, flash, true, en),
                    _ => crate::art2::golem(fr, st, flash, false, false),
                })
            }
            EnemyKind::Glacia | EnemyKind::Sol => {
                let fr = ((sim.time * 6.0) as i32).rem_euclid(4);
                let st = match e.state {
                    EState::Spit | EState::Breath | EState::Flare | EState::Telegraph => 1,
                    EState::Charge | EState::Inferno | EState::Icicles | EState::LunarRain => 2,
                    _ => 0,
                };
                let en = e.boss_phase2;
                let key = format!("{}{}{}{}{}", base.key(), fr, st, flash as u8, en as u8);
                let kind = base;
                sprites::get(&key, move || if kind == EnemyKind::Glacia { crate::art2::wyrm(fr, st, flash, en) } else { crate::art2::seraph(fr, st, flash, en) })
            }
            EnemyKind::FrostCrystal => {
                let fr = ((sim.time * 4.0) as i32).rem_euclid(4);
                sprites::get(&format!("fcrys{}{}", fr, flash as u8), || crate::art2::frost_crystal(fr, flash))
            }
            // Variants always map to a base kind via art_base().
            _ => sprites::get("dummy00", || art::dummy(0)),
        };
        // Shadow on the ground (floaters cast it below them), then the sprite
        let gy = if e.is_floater() { galebound_core::physics::ground_below(b, &sim.level_ref()) } else { f.y };
        let sa = if e.is_floater() { 0.5 } else { 1.0 };
        gfx::ellipse(f.x, gy, b.w * 0.55, u(3.0), 0.0, rgba(0, 0, 0, (60.0 * a * sa) as u8));
        gfx::tex(&t, f.x - u(t.width() * 0.5), f.y - u(t.height() - 2.0), 1.0, flip, tint);
        if e.state == EState::Telegraph {
            let mark = if e.kind.is_boss() { "!!" } else { "!" };
            gfx::text_centered_outlined(mark, f.x, b.pos.y - u(if e.kind.is_boss() { 24.0 } else { 16.0 }), 32.0, rgb(255, 80, 60));
        }
        // HP bar & name once hit (the boss has a screen-wide bar)
        if e.kind != EnemyKind::TrainingDummy && !e.kind.is_boss() && e.state != EState::Dying && e.hp < e.max_hp {
            let w = u(30.0);
            let y = b.pos.y - u(10.0);
            gfx::rect(f.x - w * 0.5 - u(1.0), y - u(1.0), w + u(2.0), u(5.0), rgb(26, 18, 30));
            gfx::rect(f.x - w * 0.5, y, w * (e.hp / e.max_hp).max(0.0), u(3.0), rgb(230, 70, 70));
            let label = format!("Lv.{} {}", e.level, e.kind.name());
            let lw = gfx::text_w(&label, 16.0) + u(6.0);
            gfx::rrect(f.x - lw * 0.5, f.y + u(2.0), lw, u(11.0), u(2.0), rgba(150, 40, 50, 210));
            gfx::text_centered(&label, f.x, f.y + u(4.0), 16.0, rgb(255, 240, 240));
        }
        vault_boss_fx(sim, e);
    }
}

/// The 0.15.0 dungeon bosses' mechanics: what's coming, and what to do.
fn dungeon_boss_fx(sim: &Sim, e: &galebound_core::enemies::Enemy) {
    use galebound_core::enemies::boss_ai::{ECLIPSE_SECS, INFERNO_WARN, OVERHEAT_SECS};
    if !e.kind.is_boss() {
        return;
    }
    let b = &e.body;
    let (f, c, t) = (b.feet(), b.center(), sim.time as f32);
    let label = |text: &str, col: Color| gfx::text_centered_outlined(text, f.x, b.pos.y - u(44.0), 26.0, col);
    match e.state {
        EState::Blizzard => {
            for i in 0..40 {
                let x = c.x - 900.0 + ((i * 97) as f32 + t * 260.0) % 1800.0;
                let y = c.y - 360.0 + ((i * 53) as f32 + t * 180.0) % 520.0;
                gfx::line(x, y, x - 18.0, y + 8.0, 2.0, rgba(240, 248, 255, 170));
            }
            label("BLIZZARD - keep moving or freeze!", rgb(200, 230, 255));
        }
        EState::Avalanche => label("Avalanche - jump the snowballs!", rgb(220, 236, 255)),
        EState::Icicles => label("Icicles fall where you stand - move!", rgb(190, 230, 255)),
        EState::Encased => {
            gfx::ellipse(c.x, c.y, b.w * 0.75, b.h * 0.85, 0.0, rgba(180, 230, 255, 90));
            gfx::ellipse(c.x, c.y, b.w * 0.75, b.h * 0.85, 0.0, rgba(255, 255, 255, 40));
            label("ICE SHIELD - smash the Frost Crystals!", rgb(170, 230, 255));
        }
        EState::Overheat => {
            let k = (e.state_t / OVERHEAT_SECS).clamp(0.0, 1.0);
            gfx::circle(c.x, c.y, b.w * (0.7 + 0.2 * (t * 10.0).sin()), rgba(255, 220, 140, (60.0 + 120.0 * k) as u8));
            let (bw, bx, by) = (160.0, f.x - 80.0, b.pos.y - u(30.0));
            gfx::rect(bx, by, bw, 10.0, rgba(0, 0, 0, 160));
            gfx::rect(bx, by, bw * k, 10.0, rgb(255, 120 - (80.0 * k) as u8, 40));
            label("OVERHEATING - hit it hard to cool it down!", rgb(255, 200, 120));
        }
        EState::Eruption => label("Magma! Stay out of the pools", rgb(255, 170, 90)),
        EState::Breath => label("Flame breath!", rgb(255, 160, 90)),
        EState::Inferno => {
            let arena = galebound_core::world::highlands::final_arena_x(sim.map.key);
            let live = e.state_t > INFERNO_WARN;
            let (x0, x1) = if e.facing > 0.0 { (arena, arena + 760.0) } else { (arena - 760.0, arena) };
            let a = if live { 120.0 + 40.0 * (t * 12.0).sin() } else { 40.0 + 40.0 * (t * 8.0).sin().abs() };
            gfx::rect(x0, f.y - 140.0, x1 - x0, 140.0, rgba(255, 90, 30, a as u8));
            for i in 0..18 {
                let x = x0 + (x1 - x0) * (i as f32 + 0.5) / 18.0;
                let hgt = 30.0 + ((t * 9.0 + i as f32).sin().abs()) * if live { 70.0 } else { 20.0 };
                gfx::tri(x - 14.0, f.y, x + 14.0, f.y, x, f.y - hgt, rgba(255, 180, 60, (a + 40.0).min(255.0) as u8));
            }
            label(if live { "INFERNO - get to the other side!" } else { "The ground is igniting..." }, rgb(255, 170, 90));
        }
        EState::WingGust => label("Wing gust - brace yourself!", rgb(255, 210, 160)),
        EState::Flare => label("Solar Flare!", rgb(255, 236, 150)),
        EState::LunarRain => label("Lunar Rain!", rgb(200, 210, 255)),
        EState::Eclipse => {
            let k = (e.state_t / ECLIPSE_SECS).clamp(0.0, 1.0);
            gfx::circle(c.x, c.y, b.w * 0.9, rgba(30, 20, 60, 120));
            gfx::circle(c.x, c.y, b.w * 0.9 * (1.0 - k) + 6.0, rgba(255, 240, 200, 80));
            label(&format!("ECLIPSE - {:.0}s to finish it!", (ECLIPSE_SECS - e.state_t).max(0.0)), rgb(255, 230, 200));
        }
        _ => {}
    }
}

/// Turrets, the spirit hawk and the Oracle's orbiting stars (0.15.0).
fn summons(sim: &Sim) {
    use galebound_core::skills::SummonKind;
    let ink = rgb(26, 18, 30);
    let t = sim.time as f32;
    for s in &sim.summons {
        if !sim.settings.fx_shown(FxCat::Companion, s.owner.is_none()) {
            continue;
        }
        let fade = (s.left / 0.6).clamp(0.0, 1.0);
        match s.kind {
            SummonKind::Turret | SummonKind::Mender => {
                let (x, y) = (s.pos.x, s.pos.y);
                let mender = s.kind == SummonKind::Mender;
                let body = if mender { rgb(120, 200, 150) } else { rgb(150, 156, 176) };
                // tripod legs
                for dx in [-14.0f32, 0.0, 14.0] {
                    gfx::line(x, y - 18.0, x + dx, y, 3.0, alpha(ink, fade));
                }
                gfx::rrect(x - 14.0, y - 40.0, 28.0, 24.0, 5.0, alpha(ink, fade));
                gfx::rrect(x - 12.0, y - 38.0, 24.0, 20.0, 4.0, alpha(body, fade));
                if mender {
                    gfx::rect(x - 2.0, y - 34.0, 4.0, 12.0, rgb(255, 255, 255));
                    gfx::rect(x - 6.0, y - 30.0, 12.0, 4.0, rgb(255, 255, 255));
                    gfx::circle(x, y - 28.0, 60.0 + (t * 4.0).sin() * 6.0, rgba(140, 255, 180, 26));
                } else {
                    gfx::rect(x + s.facing * 8.0 - 1.0, y - 32.0, s.facing * 18.0, 6.0, alpha(ink, fade));
                    gfx::circle(x, y - 28.0, 4.0, rgb(255, 120, 80));
                }
                // time left
                let k = (s.left / s.total).clamp(0.0, 1.0);
                gfx::rect(x - 14.0, y - 48.0, 28.0, 4.0, rgba(0, 0, 0, 140));
                gfx::rect(x - 14.0, y - 48.0, 28.0 * k, 4.0, if mender { rgb(120, 230, 150) } else { rgb(255, 200, 90) });
            }
            SummonKind::Hawk => {
                let (x, y) = (s.pos.x, s.pos.y);
                let flap = (t * 14.0).sin() * 10.0;
                let col = rgba(170, 240, 200, (210.0 * fade) as u8);
                gfx::tri(x, y, x - 24.0 * s.facing, y - 6.0 - flap, x - 8.0 * s.facing, y + 4.0, col);
                gfx::tri(x, y, x - 22.0 * s.facing, y + 2.0 + flap * 0.5, x - 6.0 * s.facing, y + 6.0, col);
                gfx::circle(x + 6.0 * s.facing, y - 2.0, 6.0, col);
                gfx::tri(x + 11.0 * s.facing, y - 3.0, x + 18.0 * s.facing, y, x + 11.0 * s.facing, y + 1.0, rgb(250, 220, 120));
                gfx::circle(x, y, 22.0, rgba(140, 255, 200, 30));
            }
        }
    }
    // the blizzard: a frost meter, then a block of ice
    let pb = &sim.player.body;
    if sim.frozen > 0.0 {
        gfx::rrect(pb.pos.x - 8.0, pb.pos.y - 8.0, pb.w + 16.0, pb.h + 12.0, 6.0, rgba(180, 225, 255, 140));
        gfx::rrect(pb.pos.x - 4.0, pb.pos.y - 4.0, 8.0, pb.h, 3.0, rgba(255, 255, 255, 90));
    } else if sim.frost > 0.02 {
        let (x, y) = (pb.pos.x + pb.w * 0.5 - 30.0, pb.pos.y - 26.0);
        gfx::rect(x, y, 60.0, 7.0, rgba(0, 0, 0, 150));
        gfx::rect(x, y, 60.0 * sim.frost.min(1.0), 7.0, rgb(170, 220, 255));
        gfx::text_centered_outlined("FROST", x + 30.0, y - 16.0, 14.0, rgb(200, 235, 255));
    }
    let star = |o: galebound_core::math::V2| {
        gfx::circle(o.x, o.y, 13.0, rgba(200, 180, 255, 50));
        gfx::circle(o.x, o.y, 6.0, rgb(255, 240, 200));
        gfx::circle(o.x, o.y, 3.0, rgb(255, 255, 255));
    };
    if sim.settings.fx_shown(FxCat::Companion, true) {
        let n = sim.orbit_count();
        for k in 0..n {
            star(sim.orbit_pos(k, n));
        }
    }
    if sim.settings.fx_shown(FxCat::Companion, false) {
        for o in sim.others.iter().filter(|o| !o.dead) {
            let n = sim.other_orbit_count(&o.info.class, o.info.level);
            let c = galebound_core::math::V2::new(o.pos.x, o.pos.y - galebound_core::sim::PLAYER_H * 0.5);
            for k in 0..n {
                star(sim.orbit_pos_at(c, k, n));
            }
        }
    }
}

/// The Tempest Vault bosses' mechanics: what's coming, and what to do.
fn vault_boss_fx(sim: &Sim, e: &galebound_core::enemies::Enemy) {
    use galebound_core::enemies::{PORTCULLIS_WARN, SURGE_RANGE, SURGE_WARN, VORTEX_RANGE};
    if !matches!(e.kind, EnemyKind::Gatekeeper | EnemyKind::Stormwarden) {
        dungeon_boss_fx(sim, e);
        return;
    }
    let b = &e.body;
    let (f, c, t) = (b.feet(), b.center(), sim.time as f32);
    let label = |text: &str, col: Color| gfx::text_centered_outlined(text, f.x, b.pos.y - u(44.0), 26.0, col);
    match e.state {
        EState::Guard => {
            // a glowing shield wall in front of it
            let sx = c.x + e.facing * (b.w * 0.5 + 14.0);
            let pulse = 0.65 + 0.25 * (t * 8.0).sin();
            gfx::ellipse(sx, c.y - 6.0, 16.0, b.h * 0.75, 0.0, rgba(255, 210, 100, (90.0 * pulse) as u8));
            gfx::rrect(sx - 6.0, b.pos.y - 18.0, 12.0, b.h + 18.0, 5.0, rgba(255, 226, 140, (200.0 * pulse) as u8));
            gfx::line(sx, b.pos.y - 14.0, sx, f.y - 4.0, 2.0, rgba(255, 255, 230, 230));
            label("SHIELD UP - strike from behind!", rgb(255, 214, 110));
        }
        EState::Portcullis if e.state_t < PORTCULLIS_WARN + 0.4 => {
            label("Spikes from above - find a gap!", rgb(220, 226, 240));
        }
        EState::Vortex => {
            // rings rushing inward
            for i in 0..5 {
                let k = 1.0 - ((t * 0.9 + i as f32 / 5.0).fract());
                let r = 40.0 + k * VORTEX_RANGE * 0.6;
                gfx::circle_lines(c.x, c.y, r, 2.0 + k * 2.0, rgba(150, 205, 255, (40.0 + 120.0 * (1.0 - k)) as u8));
            }
            for i in 0..8 {
                let a = t * 5.0 + i as f32 * std::f32::consts::TAU / 8.0;
                let r = 70.0 + (t * 3.0 + i as f32).sin().abs() * 60.0;
                gfx::line(c.x + a.cos() * r, c.y + a.sin() * r, c.x + (a + 0.6).cos() * (r - 30.0), c.y + (a + 0.6).sin() * (r - 30.0), 2.0, rgba(220, 240, 255, 170));
            }
            label("VORTEX - run from it!", rgb(170, 215, 255));
        }
        EState::Surge => {
            let live = e.state_t > SURGE_WARN;
            let (x0, x1) = (c.x - SURGE_RANGE, c.x + SURGE_RANGE);
            let y = f.y;
            if live {
                gfx::rect(x0, y - 10.0, x1 - x0, 10.0, rgba(255, 236, 120, 50));
                let seed = (t * 24.0) as u32;
                let mut x = x0;
                let mut i = 0u32;
                while x < x1 {
                    let nx = (x + 120.0).min(x1);
                    zigzag(x, y - 4.0, nx, y - 4.0, 4, 9.0, seed.wrapping_add(i * 977), 3.0, rgba(255, 230, 90, 150));
                    zigzag(x, y - 4.0, nx, y - 4.0, 4, 9.0, seed.wrapping_add(i * 977), 1.2, rgb(255, 255, 230));
                    x = nx;
                    i += 1;
                }
                label("THE FLOOR IS LIVE - get up on a ledge!", rgb(255, 236, 120));
            } else {
                // sparks crawl along the floor, faster as it charges
                let k = e.state_t / SURGE_WARN;
                let n = (8.0 + 30.0 * k) as u32;
                for i in 0..n {
                    let h = (i.wrapping_mul(2654435761) ^ (t * 10.0) as u32).wrapping_mul(40503) % 10_000;
                    let x = x0 + (h as f32 / 10_000.0) * (x1 - x0);
                    gfx::line(x, y - 2.0, x + 6.0, y - 10.0 - 8.0 * k, 2.0, rgba(255, 240, 140, (90.0 + 140.0 * k) as u8));
                }
                label("The floor is charging...", rgb(255, 236, 160));
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------- projectiles & effects

/// Hitbox view (F3): your body in green, monsters in blue, and the active
/// part of your attack in red (yellow while winding up or recovering).
/// Handy for checking that a pose in poses.rs lines up with its hitbox.
pub fn hitboxes(sim: &Sim, cam: &Camera) {
    gfx::set_cam(cam.x, cam.y);
    let outline = |b: Aabb, c: macroquad::prelude::Color| gfx::rect_lines(b.x, b.y, b.w, b.h, 1.0, c);
    let p = &sim.player;
    outline(p.body.aabb(), rgba(80, 255, 120, 220));
    for e in &sim.enemies {
        outline(e.body.aabb(), rgba(110, 180, 255, 200));
    }
    if let Some(a) = &p.attack {
        if a.def.hw > 0.0 && a.def.hh > 0.0 {
            let b = galebound_core::sim::attack_hitbox(p, &a.def);
            let live = a.phase() == AttackPhase::Active;
            let c = if live { rgba(255, 60, 60, 230) } else { rgba(255, 220, 80, 140) };
            if live {
                gfx::rect(b.x, b.y, b.w, b.h, rgba(255, 60, 60, 50));
            }
            outline(b, c);
        }
    }
}

/// The sweep of melee attacks and abilities (yours and other players').
pub fn swings(sim: &Sim, cam: &Camera) {
    use galebound_core::classes::AttackKind as K;
    gfx::set_cam(cam.x, cam.y);
    for s in &sim.swings {
        let cat = if s.fx.is_some() { FxCat::Skill } else { FxCat::Weapon };
        if !sim.settings.fx_shown(cat, s.owner.is_none()) {
            continue;
        }
        let k = (s.t / s.dur).clamp(0.0, 1.0);
        let fade = (1.0 - k).powf(0.7);
        let col = match s.fx {
            Some(fx) => px_color(crate::icons::fx_color(fx)),
            None if s.heavy => rgb(255, 232, 170),
            None => rgb(240, 246, 255),
        };
        let core = mix(col, rgb(255, 255, 255), 0.55);
        // centred on the real hitbox (see `attack_hitbox` in core/src/sim.rs)
        let cx = s.pos.x + if s.both { s.hx * s.facing } else { (s.hx + s.hw * 0.25) * s.facing };
        let cy = s.pos.y + s.hy;
        let (rx, ry) = (s.hw * 0.5, s.hh * 0.5);
        if s.both && s.hits > 1 {
            // a whirlwind: a soft funnel, then stacked spinning rings
            for i in 0..6 {
                let f = i as f32 / 5.0;
                gfx::ellipse(cx, cy + ry * 0.9 - f * ry * 1.9, rx * (0.45 + f * 0.6), ry * 0.22, 0.0, alpha(col, 0.13 * fade));
            }
            let spin = s.t * 22.0;
            for i in 0..5 {
                let f = i as f32 / 4.0;
                let y = cy + ry * 0.9 - f * ry * 1.9;
                let r = rx * (0.45 + f * 0.6);
                let a = fade * (0.95 - f * 0.3);
                for j in 0..10 {
                    let t0 = spin + j as f32 * 0.6 + f * 2.0;
                    let t1 = t0 + 0.42;
                    let (x0, y0) = (cx + t0.cos() * r, y + t0.sin() * r * 0.22);
                    let (x1, y1) = (cx + t1.cos() * r, y + t1.sin() * r * 0.22);
                    let front = t0.sin() > 0.0;
                    gfx::line(x0, y0, x1, y1, u(if front { 3.0 } else { 1.5 }), alpha(if front { core } else { col }, a * if front { 1.0 } else { 0.55 }));
                }
            }
            gfx::ellipse(cx, cy + ry * 0.95, rx * 0.8, u(4.0), 0.0, alpha(col, 0.25 * fade));
            continue;
        }
        if s.both {
            // a burst all around: an expanding ring
            let r = 0.55 + k * 0.6;
            gfx::ellipse(cx, cy, rx * r, ry * r, 0.0, alpha(col, 0.18 * fade));
            for j in 0..28 {
                let t0 = j as f32 / 28.0 * std::f32::consts::TAU;
                let t1 = t0 + 0.18;
                gfx::line(cx + t0.cos() * rx * r, cy + t0.sin() * ry * r, cx + t1.cos() * rx * r, cy + t1.sin() * ry * r, u(2.0), alpha(core, fade));
            }
            continue;
        }
        // a crescent sweeping through the hitbox, in the attack's direction
        let (a0, a1) = match s.kind {
            K::LightDown | K::HeavyDown | K::AirDown | K::AirHeavyDown => (-0.4, 1.9),
            K::AirNeutral | K::AirHeavyNeutral => (-2.4, 0.4),
            K::LightNeutral | K::HeavyNeutral => (-1.7, 0.9),
            _ => (-1.25, 1.25),
        };
        let reveal = (k * 2.2).min(1.0);
        let n = 14;
        let (ox, oy) = (cx - s.hx * s.facing * 0.35, cy);
        for j in 0..n {
            let f0 = j as f32 / n as f32;
            if f0 > reveal {
                break;
            }
            let f1 = (j + 1) as f32 / n as f32;
            let (t0, t1) = (a0 + (a1 - a0) * f0, a0 + (a1 - a0) * f1);
            let thick = (f0 * std::f32::consts::PI).sin() * if s.heavy { 6.0 } else { 4.0 } + 1.0;
            for (rr, w, c, a) in [(1.0, thick + 2.0, col, 0.45), (0.94, thick, core, 0.9)] {
                let p0 = (ox + t0.cos() * rx * rr * s.facing, oy + t0.sin() * ry * rr);
                let p1 = (ox + t1.cos() * rx * rr * s.facing, oy + t1.sin() * ry * rr);
                gfx::line(p0.0, p0.1, p1.0, p1.1, u(w * 0.5), alpha(c, a * fade));
            }
        }
    }
}

pub fn projectiles(sim: &Sim, cam: &Camera) {
    gfx::set_cam(cam.x, cam.y);
    let ink = rgb(26, 18, 30);
    for p in &sim.projectiles {
        if p.team != Team::Enemy && !sim.settings.fx_shown(p.fx, p.team == Team::Player) {
            continue;
        }
        let (x, y) = (p.pos.x, p.pos.y);
        let ang = p.vel.y.atan2(p.vel.x);
        let (dx, dy) = (ang.cos(), ang.sin());
        let fade = (p.life / p.max_life).clamp(0.0, 1.0);
        match p.style {
            ProjStyle::Arrow => {
                let l = 16.0;
                gfx::line(x - dx * (l + 18.0), y - dy * (l + 18.0), x - dx * l, y - dy * l, u(1.0), rgba(255, 255, 255, 90));
                gfx::line(x - dx * l, y - dy * l, x + dx * l, y + dy * l, u(3.0), ink);
                gfx::line(x - dx * l, y - dy * l, x + dx * l, y + dy * l, u(1.0), rgb(170, 116, 70));
                gfx::tri(x + dx * l + dy * 5.0, y + dy * l - dx * 5.0, x + dx * l - dy * 5.0, y + dy * l + dx * 5.0, x + dx * (l + 11.0), y + dy * (l + 11.0), rgb(220, 228, 240));
            }
            ProjStyle::Knife => {
                gfx::line(x - dx * 9.0, y - dy * 9.0, x + dx * 9.0, y + dy * 9.0, u(3.0), ink);
                gfx::line(x - dx * 8.0, y - dy * 8.0, x + dx * 8.0, y + dy * 8.0, u(1.0), rgb(210, 220, 236));
                gfx::line(x - dx * 22.0, y - dy * 22.0, x - dx * 9.0, y - dy * 9.0, u(1.0), rgba(170, 120, 230, 130));
            }
            ProjStyle::Orb => {
                let r = p.w * 0.5;
                gfx::circle(x, y, r * 1.3, alpha(rgb(140, 220, 255), 0.25 * fade));
                gfx::circle(x, y, r + u(1.0), alpha(ink, fade));
                gfx::circle(x, y, r, alpha(rgb(150, 226, 255), fade));
                gfx::circle(x - r * 0.3, y - r * 0.3, r * 0.35, alpha(rgb(255, 255, 255), fade));
            }
            ProjStyle::Bolt => {
                gfx::line(x - dx * p.w, y - dy * p.w, x, y, p.h * 0.6, rgba(150, 210, 255, 130));
                gfx::circle(x, y, p.h * 0.5 + u(1.0), ink);
                gfx::circle(x, y, p.h * 0.5, rgb(200, 238, 255));
                gfx::circle(x, y, p.h * 0.22, rgb(255, 255, 255));
            }
            ProjStyle::Caltrop => {
                let c = if p.team == Team::Enemy { rgb(110, 176, 84) } else { rgb(170, 172, 190) };
                let s = p.w * 0.6;
                gfx::tri(x - s - u(1.0), y + s * 0.6 + u(1.0), x + s + u(1.0), y + s * 0.6 + u(1.0), x, y - s - u(1.0), ink);
                gfx::tri(x - s, y + s * 0.6, x + s, y + s * 0.6, x, y - s, c);
                gfx::tri(x - s * 0.7, y - s * 0.2, x + s * 0.7, y - s * 0.2, x, y + s * 0.8, shade(c, 0.8));
            }
            ProjStyle::Shockwave => {
                gfx::ellipse(x, y + 6.0, p.w * 0.75, p.h * 0.45, 0.0, alpha(rgb(214, 186, 130), 0.85 * fade));
                for i in 0..3 {
                    let bx = x - 14.0 + i as f32 * 12.0;
                    let hgt = 14.0 + ((sim.time * 30.0 + i as f32).sin().abs() * 8.0);
                    gfx::tri(bx - 6.0, y + 14.0, bx + 6.0, y + 14.0, bx, y + 14.0 - hgt, alpha(ink, fade));
                    gfx::tri(bx - 4.0, y + 13.0, bx + 4.0, y + 13.0, bx, y + 16.0 - hgt, alpha(rgb(176, 136, 90), fade));
                }
            }
            ProjStyle::Bullet => {
                // a hot tracer round
                gfx::line(x - dx * 26.0, y - dy * 26.0, x, y, u(1.2), rgba(255, 230, 150, 120));
                gfx::line(x - dx * 6.0, y - dy * 6.0, x + dx * 3.0, y + dy * 3.0, u(2.6), ink);
                gfx::line(x - dx * 5.0, y - dy * 5.0, x + dx * 2.0, y + dy * 2.0, u(1.4), rgb(255, 214, 110));
            }
            ProjStyle::Card => {
                // a spinning playing card
                let a = sim.time * 18.0;
                let (cx, cy) = (a.cos() * 9.0, a.sin() * 9.0);
                let (px, py) = (-a.sin() * 6.0, a.cos() * 6.0);
                let pts = [(x - cx - px, y - cy - py), (x + cx - px, y + cy - py), (x + cx + px, y + cy + py), (x - cx + px, y - cy + py)];
                gfx::tri(pts[0].0, pts[0].1, pts[1].0, pts[1].1, pts[2].0, pts[2].1, rgb(250, 248, 240));
                gfx::tri(pts[0].0, pts[0].1, pts[2].0, pts[2].1, pts[3].0, pts[3].1, rgb(250, 248, 240));
                gfx::circle(x, y, 2.5, rgb(200, 40, 50));
                gfx::circle(x, y, 14.0, rgba(200, 160, 255, 50));
            }
            ProjStyle::Cannonball => {
                let r = p.w * 0.45;
                gfx::line(x - dx * 30.0, y - dy * 30.0, x, y, r, rgba(200, 200, 210, 60));
                gfx::circle(x, y, r + u(1.0), ink);
                gfx::circle(x, y, r, rgb(60, 60, 70));
                gfx::circle(x - r * 0.3, y - r * 0.3, r * 0.35, rgb(150, 150, 166));
                gfx::circle(x + dx * r, y + dy * r, 3.0, rgb(255, 160, 60));
            }
            ProjStyle::Snowball => {
                let r = p.w * 0.5;
                gfx::circle(x, y, r + u(1.0), alpha(ink, fade));
                gfx::circle(x, y, r, alpha(rgb(240, 246, 255), fade));
                let spin = sim.time * 8.0;
                gfx::circle(x + spin.cos() * r * 0.5, y + spin.sin() * r * 0.5, r * 0.25, rgb(200, 216, 240));
                gfx::ellipse(x, y + r, r * 1.2, 4.0, 0.0, rgba(255, 255, 255, 90));
            }
            ProjStyle::Icicle => {
                // a falling shard of ice, point down
                let (hw, hh) = (p.w * 0.3, p.w * 1.5);
                gfx::tri(x - hw - u(1.0), y - hh - u(1.0), x + hw + u(1.0), y - hh - u(1.0), x, y + hh * 0.5 + u(2.0), ink);
                gfx::tri(x - hw, y - hh, x + hw, y - hh, x, y + hh * 0.5, rgb(190, 230, 255));
                gfx::tri(x - hw * 0.3, y - hh, x + hw * 0.4, y - hh, x, y + hh * 0.2, rgb(255, 255, 255));
            }
            ProjStyle::Fireball => {
                let r = p.w * 0.45;
                gfx::line(x - dx * 26.0, y - dy * 26.0, x, y, r * 1.4, rgba(255, 120, 40, 90));
                gfx::circle(x, y, r * 1.5, rgba(255, 160, 60, 70));
                gfx::circle(x, y, r + u(1.0), ink);
                gfx::circle(x, y, r, rgb(255, 120, 40));
                gfx::circle(x, y, r * 0.55, rgb(255, 230, 140));
            }
            ProjStyle::LavaPool => {
                // a bubbling pool of lava on the floor
                let w = p.w;
                let a = (p.life / p.max_life).min(1.0).max(0.0).sqrt();
                gfx::ellipse(x, y + 6.0, w * 0.75, 9.0, 0.0, rgba(255, 90, 30, (200.0 * a) as u8));
                gfx::ellipse(x, y + 6.0, w * 0.5, 5.0, 0.0, rgba(255, 210, 90, (220.0 * a) as u8));
                for i in 0..3 {
                    let bx = x - w * 0.4 + i as f32 * w * 0.4;
                    let by = y + 2.0 - ((sim.time * 2.0 + i as f32).sin().abs() * 8.0);
                    gfx::circle(bx, by, 3.0, rgba(255, 180, 70, (200.0 * a) as u8));
                }
            }
            ProjStyle::Flame => {
                let r = p.w * 0.5;
                let t = 1.0 - (p.life / p.max_life).clamp(0.0, 1.0);
                gfx::circle(x, y, r * (1.0 + t), rgba(255, 110, 30, (150.0 * (1.0 - t)) as u8));
                gfx::circle(x, y, r * 0.6 * (1.0 + t), rgba(255, 220, 120, (200.0 * (1.0 - t)) as u8));
            }
            ProjStyle::Spike => {
                // an iron portcullis spike on a chain, point down
                let (hw, hh) = (p.w * 0.32, p.w * 1.6);
                gfx::line(x, y - hh - 120.0, x, y - hh, u(1.5), rgba(120, 120, 140, 160));
                gfx::rect(x - hw - u(1.0), y - hh - u(1.0), hw * 2.0 + u(2.0), hh + u(2.0), ink);
                gfx::rect(x - hw, y - hh, hw * 2.0, hh, rgb(96, 100, 118));
                gfx::rect(x - hw, y - hh, hw * 0.6, hh, rgb(140, 146, 166));
                gfx::tri(x - hw - u(1.0), y - u(1.0), x + hw + u(1.0), y - u(1.0), x, y + hw * 3.0 + u(2.0), ink);
                gfx::tri(x - hw, y, x + hw, y, x, y + hw * 3.0, rgb(186, 190, 206));
            }
            ProjStyle::Lightning => {
                // a jagged white-blue bolt trailing back up to the sky
                let top = y - p.travelled.min(430.0) - 40.0;
                zigzag(x, top, x, y, 7, 18.0, (p.pos.x as i32 as u32).wrapping_mul(2654435761) ^ (sim.time * 30.0) as u32, u(3.0), rgba(120, 190, 255, 120));
                zigzag(x, top, x, y, 7, 18.0, (p.pos.x as i32 as u32).wrapping_mul(2654435761) ^ (sim.time * 30.0) as u32, u(1.4), rgb(240, 250, 255));
                gfx::circle(x, y, 16.0, rgba(180, 220, 255, 90));
            }
            ProjStyle::Meteor => {
                let r = p.w * 0.5;
                // fiery tail
                for k in 1..7 {
                    let t = k as f32 / 7.0;
                    gfx::circle(x - dx * r * 1.6 * k as f32 * 0.5, y - dy * r * 1.6 * k as f32 * 0.5, r * (1.0 - t * 0.7), rgba(255, (170.0 - t * 110.0) as u8, 40, (200.0 * (1.0 - t)) as u8));
                }
                gfx::circle(x, y, r * 1.5, rgba(255, 150, 40, 60));
                gfx::circle(x, y, r + u(1.5), ink);
                gfx::circle(x, y, r, rgb(150, 70, 40));
                gfx::circle(x + r * 0.25, y + r * 0.2, r * 0.55, rgb(110, 48, 30));
                gfx::circle(x - r * 0.3, y - r * 0.3, r * 0.3, rgb(255, 190, 90));
                gfx::circle(x - r * 0.1, y + r * 0.35, r * 0.16, rgb(255, 230, 140));
            }
            ProjStyle::Star => {
                gfx::circle(x, y, 14.0, alpha(rgb(255, 230, 140), 0.3));
                gfx::line(x, y, x - dx * 30.0, y - dy * 30.0, u(2.0), rgba(255, 230, 140, 120));
                gfx::tri(x - 9.0, y + 5.0, x + 9.0, y + 5.0, x, y - 11.0, rgb(255, 236, 140));
                gfx::tri(x - 9.0, y - 5.0, x + 9.0, y - 5.0, x, y + 11.0, rgb(255, 236, 140));
                gfx::rect(x - u(1.0), y - u(1.0), u(2.0), u(2.0), rgb(255, 255, 255));
            }
        }
    }
}

/// A jagged line from (x1, y1) to (x2, y2) with `n` kinks, the same shape
/// for the same `seed`.
#[allow(clippy::too_many_arguments)]
fn zigzag(x1: f32, y1: f32, x2: f32, y2: f32, n: u32, jitter: f32, seed: u32, th: f32, c: Color) {
    let (dx, dy) = (x2 - x1, y2 - y1);
    let len = (dx * dx + dy * dy).sqrt().max(1.0);
    let (nx, ny) = (-dy / len, dx / len);
    let mut h = seed | 1;
    let mut rnd = || {
        h ^= h << 13;
        h ^= h >> 17;
        h ^= h << 5;
        (h % 2000) as f32 / 1000.0 - 1.0
    };
    let mut px = x1;
    let mut py = y1;
    for k in 1..=n {
        let t = k as f32 / n as f32;
        let off = if k == n { 0.0 } else { rnd() * jitter };
        let qx = x1 + dx * t + nx * off;
        let qy = y1 + dy * t + ny * off;
        gfx::line(px, py, qx, qy, th, c);
        px = qx;
        py = qy;
    }
}

/// Lightning arcs and explosion rings (Arcane Bolt, Meteor, Thunder Palm).
pub fn magic(sim: &Sim, cam: &Camera) {
    use galebound_core::sim::{BoomKind, ZapKind};
    gfx::set_cam(cam.x, cam.y);
    summons(sim);
    for b in sim.booms.iter().filter(|b| sim.settings.fx_shown(b.fx, b.mine)) {
        let t = 1.0 - (b.life / b.max_life).clamp(0.0, 1.0);
        let r = b.radius * (0.35 + 0.65 * t.sqrt());
        let a = 1.0 - t;
        match b.kind {
            BoomKind::Meteor => {
                gfx::circle(b.pos.x, b.pos.y, r, rgba(255, 150, 50, (110.0 * a) as u8));
                gfx::circle(b.pos.x, b.pos.y, r * 0.6, rgba(255, 230, 150, (170.0 * a) as u8));
                gfx::circle_lines(b.pos.x, b.pos.y, r, u(2.5), rgba(255, 110, 40, (230.0 * a) as u8));
                // scorch on the ground
                gfx::ellipse(b.pos.x, b.pos.y + b.radius * 0.35, b.radius * 0.9, b.radius * 0.18, 0.0, rgba(40, 20, 14, (110.0 * a) as u8));
            }
            BoomKind::Thunder => {
                gfx::circle(b.pos.x, b.pos.y, r, rgba(150, 210, 255, (70.0 * a) as u8));
                gfx::circle_lines(b.pos.x, b.pos.y, r, u(1.5), rgba(220, 240, 255, (220.0 * a) as u8));
            }
        }
    }
    for z in sim.zaps.iter().filter(|z| sim.settings.fx_shown(z.fx, z.mine)) {
        let a = (z.life / z.max_life).clamp(0.0, 1.0);
        // re-roll the shape a few times a second so it flickers
        let seed = z.seed ^ ((sim.time * 24.0) as u32);
        let (n, jit, th) = match z.kind {
            ZapKind::Chain => (8, 16.0, u(2.2)),
            ZapKind::Sky => (9, 22.0, u(2.6)),
            ZapKind::Spark => (4, 7.0, u(1.2)),
        };
        zigzag(z.a.x, z.a.y, z.b.x, z.b.y, n, jit, seed, th * 2.2, rgba(110, 180, 255, (120.0 * a) as u8));
        zigzag(z.a.x, z.a.y, z.b.x, z.b.y, n, jit, seed, th, alpha(rgb(240, 250, 255), a));
        if z.kind != ZapKind::Spark {
            gfx::circle(z.b.x, z.b.y, 12.0 * a + 4.0, rgba(200, 230, 255, (150.0 * a) as u8));
        }
    }
}

pub fn particles(sim: &Sim, cam: &Camera) {
    gfx::set_cam(cam.x, cam.y);
    for p in &sim.particles {
        let t = (p.life / p.max_life).clamp(0.0, 1.0);
        let c: Color = match p.kind {
            FxKind::Hit => rgb(255, 255, 255),
            FxKind::Crit => rgb(255, 210, 80),
            FxKind::Dust => rgb(226, 214, 190),
            FxKind::Heal => rgb(120, 255, 150),
            FxKind::LevelUp => rgb(255, 230, 100),
            FxKind::Spark => rgb(170, 220, 255),
        };
        // square pixel particles
        let s = u((p.size * 0.5 * (0.4 + t * 0.6)).round().max(1.0));
        gfx::rect(p.pos.x - s * 0.5, p.pos.y - s * 0.5, s, s, alpha(c, t.min(1.0)));
    }
    // Damage numbers: chunky, MMO-style.
    for f in &sim.floats {
        let (size, c) = match f.kind {
            FloatKind::Damage => (32.0, rgb(255, 255, 255)),
            FloatKind::Crit => (48.0, rgb(255, 196, 60)),
            FloatKind::PlayerHurt => (32.0, rgb(255, 92, 92)),
            FloatKind::Heal => (32.0, rgb(120, 255, 150)),
            FloatKind::Info => (16.0, rgb(240, 240, 255)),
            FloatKind::Exp => (16.0, rgb(214, 170, 255)),
            FloatKind::Loot => (14.0, rgb(140, 230, 255)),
        };
        gfx::text_centered_outlined(&f.text, f.pos.x, f.pos.y, size, alpha(c, f.life.min(1.0)));
    }
}

/// Loot and gold lying on the ground, with a pick-up hint on the nearest one.
pub fn drops(sim: &Sim, cam: &Camera, time: f32) {
    use galebound_core::sim::{DropKind, COLLECT_FLY, DROP_LIFE};
    gfx::set_cam(cam.x, cam.y);
    for d in &sim.drops {
        let f = d.body.feet();
        if f.x < cam.x - 60.0 || f.x > cam.x + gfx::view_w() + 60.0 {
            continue;
        }
        // Blink during the last 10 seconds before vanishing.
        let left = DROP_LIFE - d.age;
        if d.taken.is_none() && left < 10.0 && ((time * 8.0) as i32) % 2 == 0 {
            continue;
        }
        let (a, k) = match d.taken {
            Some((_, _, t)) => (1.0 - t / COLLECT_FLY, 1.0 - 0.5 * t / COLLECT_FLY),
            None => (1.0, 1.0),
        };
        let bob = if d.body.on_ground && d.taken.is_none() { ((time * 3.0 + d.id as f32).sin() * 1.5).round() * u(1.0) } else { 0.0 };
        let (tex, glow) = match d.kind {
            DropKind::Gold => {
                let size = if d.count >= 100 { 2 } else if d.count >= 20 { 1 } else { 0 };
                (sprites::get(&format!("coins{}", size), || crate::lootart::coin_pile(size)), rgb(255, 220, 110))
            }
            DropKind::Item(id) => {
                let Some(def) = galebound_core::items::item(id) else { continue };
                let t = crate::icons::tier_color(def.tier.max(1));
                (sprites::get(&format!("icon_i_{}", def.id), || crate::icons::item(def)), rgb(t[0], t[1], t[2]))
            }
        };
        if d.body.on_ground && d.taken.is_none() {
            let pulse = 0.18 + 0.08 * (time * 4.0 + d.id as f32).sin();
            gfx::ellipse(f.x, f.y - u(1.0), u(9.0), u(2.5), 0.0, alpha(glow, pulse));
        }
        let w = u(tex.width()) * k;
        let h = u(tex.height()) * k;
        gfx::tex(&tex, f.x - w * 0.5, f.y - h - u(1.0) + bob, k, false, Color::new(1.0, 1.0, 1.0, a));
    }
}

/// The pick-up hint over the nearest drop (drawn after everything else in the world).
pub fn drop_labels(sim: &Sim, cam: &Camera) {
    use galebound_core::sim::PICKUP_REACH;
    if sim.player.state == PState::Dead || sim.dialog.is_some() {
        return;
    }
    gfx::set_cam(cam.x, cam.y);
    let Some(i) = sim.drop_near(sim.player.body.center(), PICKUP_REACH) else { return };
    let d = &sim.drops[i];
    let f = d.body.feet();
    let key = sim.settings.layout.key_label(galebound_core::input::Action::Pickup);
    let label = format!("[{}] {}", key, d.name());
    let w = gfx::text_w(&label, 16.0) + u(6.0);
    let y = f.y - u(36.0);
    gfx::rrect(f.x - w * 0.5, y - u(2.0), w, u(11.0), u(2.0), rgba(24, 22, 40, 200));
    gfx::text_centered(&label, f.x, y, 16.0, rgb(255, 255, 255));
}

/// Your pet: trots (or flutters) along beside you and fetches loot.
pub fn pet(sim: &Sim, cam: &Camera, time: f32) {
    let (Some(a), Some(p)) = (&sim.pet, sim.active_pet()) else { return };
    gfx::set_cam(cam.x, cam.y);
    let flies = p.species.flies();
    let starving = p.starving();
    let frame: u8 = if starving {
        0
    } else if flies {
        ((time * 9.0) as u8) % 2
    } else if a.moving {
        ((a.anim_t * 10.0) as u8) % 2
    } else {
        0
    };
    let tex = sprites::get(&format!("pet_{}_{}", p.species.key(), frame), || crate::petart::pet(p.species, frame));
    let lift = if flies && !starving {
        u(16.0) + ((time * 3.0).sin() * 2.0).round() * u(1.0)
    } else if a.moving {
        ((a.anim_t * 12.0).sin().abs() * 2.0).round() * u(1.0)
    } else {
        0.0
    };
    gfx::ellipse(a.pos.x, a.pos.y - u(1.0), u(7.0), u(2.0), 0.0, rgba(0, 0, 0, if flies { 40 } else { 70 }));
    let tint = if starving { Color::new(0.75, 0.75, 0.8, 1.0) } else { WHITE };
    sprite_at(&tex, a.pos.x, a.pos.y - lift, a.facing < 0.0, tint);
}

/// The pet's name (and a "Hungry!" bubble), drawn on top of the world.
pub fn pet_label(sim: &Sim, cam: &Camera, time: f32) {
    let (Some(a), Some(p)) = (&sim.pet, sim.active_pet()) else { return };
    gfx::set_cam(cam.x, cam.y);
    let lift = if p.species.flies() && !p.starving() { u(16.0) } else { 0.0 };
    let top = a.pos.y - lift - u(crate::petart::H as f32) - u(10.0);
    gfx::text_centered_outlined(&format!("{} Lv.{}", p.name, p.level), a.pos.x, top, 16.0, rgb(190, 230, 255));
    if p.starving() {
        let pulse = 0.6 + 0.4 * (time * 4.0).sin();
        gfx::text_centered_outlined("Hungry!", a.pos.x, top - u(11.0), 16.0, alpha(rgb(255, 170, 120), pulse));
    }
}

/// Other players on this map: their character, tombstone and pet.
pub fn others(sim: &Sim, cam: &Camera, time: f32) {
    gfx::set_cam(cam.x, cam.y);
    for o in &sim.others {
        if o.since > 10.0 || o.pos.x < cam.x - 120.0 || o.pos.x > cam.x + gfx::view_w() + 120.0 {
            continue;
        }
        let f = o.pos;
        // their pet first, so it's behind them
        if o.st.pet_on {
            if let Some(sp) = galebound_core::pets::PetSpecies::from_key(&o.info.pet) {
                let frame = (o.st.pet_frame % 2) as u8;
                let tex = sprites::get(&format!("pet_{}_{}", sp.key(), frame), || crate::petart::pet(sp, frame));
                let lift = if sp.flies() { u(16.0) + ((time * 3.0).sin() * 2.0).round() * u(1.0) } else { 0.0 };
                gfx::ellipse(o.pet_pos.x, o.pet_pos.y - u(1.0), u(7.0), u(2.0), 0.0, rgba(0, 0, 0, 60));
                sprite_at(&tex, o.pet_pos.x, o.pet_pos.y - lift, o.st.pet_facing < 0, WHITE);
            }
        }
        if o.dead {
            let tex = sprites::get("tombstone", crate::lootart::tombstone);
            char_shadow(f.x, f.y, 1.0);
            sprite_at(&tex, f.x, f.y, false, WHITE);
            continue;
        }
        let look = Appearance::decode(&o.info.look).unwrap_or_default();
        let weapon = if o.info.weapon.is_empty() { None } else { galebound_core::items::weapon_art(&o.info.weapon) };
        let anim = crate::online::anim_from_code(o.st.anim);
        let frame = (o.st.frame as u8).min(anim.frames().saturating_sub(1));
        let weapon = if anim == Anim::Climb { None } else { weapon };
        let tex = sprites::character(&look, weapon, anim, frame);
        char_shadow(f.x, f.y, 1.0);
        draw_char(&tex, f.x, f.y, o.st.facing < 0, WHITE);
    }
}

/// Names, HP bars and the revive prompt for other players (drawn on top).
pub fn other_labels(sim: &Sim, cam: &Camera) {
    gfx::set_cam(cam.x, cam.y);
    let revive = sim.revivable();
    for (i, o) in sim.others.iter().enumerate() {
        if o.since > 10.0 || o.pos.x < cam.x - 120.0 || o.pos.x > cam.x + gfx::view_w() + 120.0 {
            continue;
        }
        let f = o.pos;
        let tag = (!o.info.guild.is_empty()).then(|| format!("<{}>", o.info.guild));
        let label = format!("{}  Lv.{}", o.name, o.info.level);
        let foe = sim.duel.as_ref().map(|d| d.foe == o.id).unwrap_or(false);
        if sim.is_admin_name(&o.name) {
            admin_tag(f.x, f.y, &label, tag.as_deref());
        } else {
            name_tag(f.x, f.y, &label, tag.as_deref(), if foe { rgb(255, 120, 110) } else { rgb(150, 210, 255) });
        }
        if foe {
            // crossed swords over your duel opponent
            let y = f.y - u(84.0);
            gfx::line(f.x - u(5.0), y - u(5.0), f.x + u(5.0), y + u(5.0), u(1.5), rgb(255, 120, 110));
            gfx::line(f.x + u(5.0), y - u(5.0), f.x - u(5.0), y + u(5.0), u(1.5), rgb(255, 120, 110));
        }
        if !o.dead && o.st.hp < 0.999 {
            let (w, y) = (u(28.0), f.y - u(70.0));
            gfx::rect(f.x - w * 0.5, y, w, u(3.0), rgba(20, 16, 30, 200));
            gfx::rect(f.x - w * 0.5, y, w * o.st.hp.clamp(0.0, 1.0), u(3.0), rgb(226, 72, 80));
        }
        if revive == Some(i) && sim.dialog.is_none() {
            let key = sim.settings.layout.key_label(galebound_core::input::Action::Interact);
            gfx::text_centered_outlined(&format!("[{}] Revive {}", key, o.name), f.x, f.y - u(46.0), 16.0, rgb(140, 255, 170));
        }
    }
}
