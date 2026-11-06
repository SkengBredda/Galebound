//! The minimap (top-right): terrain, portals, NPCs, other players and you.
//! Adjustable: the buttons in its header (or the Minimap key) change the
//! size, switch between the whole map and the area around you, or hide it.

use crate::gfx::{self, alpha, rgb, rgba};
use crate::widgets::{self, Ui};
use galebound_core::input::Action;
use galebound_core::quests;
use galebound_core::save::{Settings, MINIMAP_SIZES};
use galebound_core::sim::{PState, Sim};
use macroquad::prelude::Color;

/// Largest map-area size per setting (0 = hidden). Flat maps use less height.
const SIZES: [(f32, f32); MINIMAP_SIZES as usize] = [(0.0, 0.0), (220.0, 96.0), (300.0, 132.0), (400.0, 176.0)];
const HEADER: f32 = 24.0;
const MARGIN: f32 = 14.0;
/// World units shown across the minimap in "around you" mode, per size.
const LOCAL_SPAN: [f32; MINIMAP_SIZES as usize] = [1.0, 1800.0, 2400.0, 3000.0];

/// What a click on the minimap asked for.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MinimapAction {
    None,
    Smaller,
    Bigger,
    ToggleWhole,
    Hide,
    Show,
}

/// Where the minimap sits and which part of the world it shows.
struct View {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    /// World point at the top-left, and minimap pixels per world unit.
    x0: f32,
    y0: f32,
    sx: f32,
    sy: f32,
}

/// Lay out the minimap for this map and player position (None when hidden).
fn view(sim: &Sim) -> Option<View> {
    let s = &sim.settings;
    let (w, max_h) = SIZES[(s.minimap_size as usize).min(SIZES.len() - 1)];
    if w == 0.0 {
        return None;
    }
    let m = &sim.map;
    let pf = sim.player.body.feet();
    let (mut ymin, mut ymax) = (f32::MAX, f32::MIN);
    // (thin boundary walls reach the sky; they don't count as terrain height)
    for t in m.solids.iter().chain(m.platforms.iter()).filter(|r| r.w >= 80.0).map(|r| r.top()).filter(|t| *t > -590.0) {
        ymin = ymin.min(t);
        ymax = ymax.max(t);
    }
    if ymin == f32::MAX {
        ymin = 0.0;
        ymax = galebound_core::world::GROUND;
    }
    // Room above the highest ledge for NPCs, a strip of ground below the lowest.
    let (ymin, ymax) = (ymin - 110.0, ymax + 70.0);
    let span_x = if s.minimap_whole { m.width } else { LOCAL_SPAN[s.minimap_size as usize].min(m.width) };
    let sx = w / span_x;
    // Whole-map view: exaggerate height a little so long, flat maps stay readable.
    let sy = if s.minimap_whole { (max_h / (ymax - ymin)).min(sx * 2.5).max(sx) } else { sx };
    let x0 = if s.minimap_whole { 0.0 } else { (pf.x - span_x * 0.5).clamp(0.0, (m.width - span_x).max(0.0)) };
    // Flat maps get a shorter minimap; tall ones scroll with you.
    let content_h = (ymax - ymin) * sy;
    let (h, y0) = if content_h <= max_h {
        let h = content_h.max(48.0);
        (h, ymin - (h - content_h) / sy)
    } else {
        let span_y = max_h / sy;
        (max_h, (pf.y - 40.0 - span_y * 0.6).clamp(ymin, ymax - span_y))
    };
    Some(View { x: gfx::VW - MARGIN - w, y: MARGIN + HEADER, w, h, x0, y0, sx, sy })
}

/// Y just below the minimap block (the quest tracker starts here).
pub fn bottom(sim: &Sim) -> f32 {
    match view(sim) {
        Some(v) => v.y + v.h + 14.0,
        // hidden: a one-line strip with the map name and a "show" button
        None => MARGIN + HEADER + 6.0,
    }
}

/// Apply a minimap button / key press to the settings. Returns true if they changed.
pub fn apply(s: &mut Settings, a: MinimapAction) -> bool {
    let before = (s.minimap_size, s.minimap_whole);
    match a {
        MinimapAction::None => {}
        MinimapAction::Smaller => s.minimap_size = s.minimap_size.saturating_sub(1).max(1),
        MinimapAction::Bigger => s.minimap_size = (s.minimap_size + 1).min(MINIMAP_SIZES - 1),
        MinimapAction::ToggleWhole => s.minimap_whole = !s.minimap_whole,
        MinimapAction::Hide => s.minimap_size = 0,
        MinimapAction::Show => s.minimap_size = 2,
    }
    before != (s.minimap_size, s.minimap_whole)
}

/// The Minimap key cycles small -> medium -> large -> hidden.
pub fn cycle(s: &mut Settings) {
    s.minimap_size = (s.minimap_size + 1) % MINIMAP_SIZES;
}

fn icon_button(ui: &Ui, x: f32, y: f32, label: &str) -> bool {
    let (w, h) = (22.0, 18.0);
    let hover = ui.enabled && gfx::mouse_in(x, y, w, h);
    gfx::rrect(x, y, w, h, 4.0, if hover { rgba(120, 220, 255, 90) } else { rgba(255, 255, 255, 28) });
    gfx::text_centered(label, x + w * 0.5, y + 1.0, 14.0, if hover { rgb(255, 255, 255) } else { rgb(200, 210, 230) });
    hover && ui.clicked
}

/// Draw the minimap and handle clicks on its buttons.
pub fn frame(sim: &Sim, ui: &Ui) -> MinimapAction {
    gfx::clear_cam();
    let s = &sim.settings;
    let key = s.layout.key_label(Action::Minimap);
    let mut act = MinimapAction::None;
    let Some(v) = view(sim) else {
        // Hidden: just the map name and a button to bring it back.
        let name = sim.map.name;
        let tw = gfx::text_w(name, 16.0);
        let bx = gfx::VW - MARGIN - 26.0;
        widgets::hud_panel(bx - tw - 18.0, MARGIN, tw + 44.0, HEADER + 2.0);
        gfx::text_outlined(name, bx - tw - 8.0, MARGIN + 5.0, 16.0, rgb(255, 255, 255));
        if icon_button(ui, bx, MARGIN + 4.0, "+") {
            act = MinimapAction::Show;
        }
        if gfx::mouse_in(bx, MARGIN + 4.0, 22.0, 18.0) {
            let t = format!("Show minimap [{}]", key);
            gfx::text_outlined(&t, gfx::VW - MARGIN - gfx::text_w(&t, 13.0), MARGIN + HEADER + 8.0, 13.0, rgb(220, 230, 245));
        }
        return act;
    };
    let (x, y, w, h) = (v.x, v.y, v.w, v.h);
    let (x0, y0, sx, sy) = (v.x0, v.y0, v.sx, v.sy);
    let m = &sim.map;
    let pf = sim.player.body.feet();

    // ---- frame and header
    widgets::hud_panel(x - 6.0, MARGIN - 2.0, w + 12.0, h + HEADER + 10.0);
    let name = sim.map.name;
    let name_size = if gfx::text_w(name, 16.0) < w - 104.0 { 16.0 } else { 13.0 };
    gfx::text_outlined(name, x + 2.0, MARGIN + 3.0, name_size, rgb(255, 255, 255));
    let bx = x + w - 4.0 * 24.0;
    let by = MARGIN + 2.0;
    let whole_tip = if s.minimap_whole { "Zoom in around you" } else { "Show the whole map" };
    let buttons = [("-", MinimapAction::Smaller, "Smaller"), ("+", MinimapAction::Bigger, "Bigger"), (if s.minimap_whole { "o" } else { "O" }, MinimapAction::ToggleWhole, whole_tip), ("x", MinimapAction::Hide, "Hide")];
    let mut tip = None;
    for (i, (label, a, help)) in buttons.iter().enumerate() {
        let bxi = bx + i as f32 * 24.0;
        if icon_button(ui, bxi, by, label) {
            act = *a;
        }
        if gfx::mouse_in(bxi, by, 22.0, 18.0) {
            tip = Some(*help);
        }
    }

    let to = |wx: f32, wy: f32| (x + (wx - x0) * sx, y + (wy - y0) * sy);
    let inside = |px: f32, py: f32| px >= x && px <= x + w && py >= y && py <= y + h;
    // Clip a world rect to the minimap and fill it.
    let fill = |l: f32, t: f32, r: f32, b: f32, c: Color| {
        let (ax, ay) = to(l, t);
        let (bx, by) = to(r, b);
        let (ax, ay, bx, by) = (ax.max(x), ay.max(y), bx.min(x + w), by.min(y + h));
        if bx > ax && by > ay {
            gfx::rect(ax, ay, bx - ax, by - ay, c);
        }
    };

    // ---- background and terrain
    gfx::rect(x, y, w, h, rgba(12, 18, 36, 215));
    let ground = rgb(86, 124, 92);
    let edge = rgb(150, 206, 132);
    for r in &m.solids {
        fill(r.left(), r.top(), r.right(), r.bottom(), ground);
        fill(r.left(), r.top(), r.right(), r.top() + 2.0 / sy, edge);
    }
    for r in &m.platforms {
        fill(r.left(), r.top(), r.right(), r.top() + (3.0 / sy).max(r.h), rgb(196, 160, 104));
    }
    for g in &m.gates {
        if !sim.save.has_flag(g.open_flag) {
            fill(g.solid.left(), g.solid.top(), g.solid.right(), g.solid.bottom(), rgb(200, 80, 70));
        }
    }
    for c in &m.climbs {
        fill(c.x - 1.0 / sx, c.top, c.x + 1.0 / sx, c.bottom, rgb(210, 180, 120));
    }

    // ---- portals
    let pulse = 0.6 + 0.4 * (sim.time * 3.0).sin();
    for p in &m.portals {
        let c = p.area.center();
        let (px, py) = to(c.x, p.area.bottom());
        if inside(px, py - 4.0) {
            let open = p.requires_flag.map(|f| sim.save.has_flag(f)).unwrap_or(true) && sim.save.level >= p.min_level;
            let col = if open { rgb(120, 220, 255) } else { rgb(150, 150, 170) };
            gfx::rrect(px - 4.0, py - 12.0, 8.0, 12.0, 3.0, alpha(col, if open { pulse } else { 0.8 }));
            gfx::rect(px - 2.0, py - 9.0, 4.0, 8.0, rgba(12, 18, 36, 200));
        }
    }

    // ---- NPCs (quest markers stand out)
    for n in &m.npcs {
        let (px, py) = to(n.x, n.y);
        if !inside(px, py - 4.0) {
            continue;
        }
        if n.id.starts_with("waystone_") {
            gfx::tri(px, py - 12.0, px - 4.0, py - 6.0, px + 4.0, py - 6.0, rgb(120, 220, 255));
            gfx::tri(px, py, px - 4.0, py - 6.0, px + 4.0, py - 6.0, rgb(120, 220, 255));
            continue;
        }
        match quests::npc_marker(n.id, &sim.save) {
            Some(mk) => {
                let col = if mk == '!' { rgb(255, 210, 60) } else { rgb(120, 220, 255) };
                gfx::rect(px - 4.0, py - 10.0, 8.0, 8.0, rgb(26, 18, 30));
                gfx::rect(px - 3.0, py - 9.0, 6.0, 6.0, col);
            }
            None => {
                gfx::rect(px - 3.0, py - 8.0, 6.0, 6.0, rgb(26, 18, 30));
                gfx::rect(px - 2.0, py - 7.0, 4.0, 4.0, rgb(250, 226, 120));
            }
        }
    }

    // ---- other players
    for o in &sim.others {
        let (px, py) = to(o.pos.x, o.pos.y);
        if inside(px, py - 4.0) {
            let col = if o.dead { rgb(160, 160, 170) } else { rgb(110, 170, 255) };
            gfx::rect(px - 4.0, py - 10.0, 8.0, 8.0, rgb(26, 18, 30));
            gfx::rect(px - 3.0, py - 9.0, 6.0, 6.0, col);
        }
    }

    // ---- you
    let (px, py) = to(pf.x, pf.y);
    let (px, py) = (px.clamp(x + 6.0, x + w - 6.0), py.clamp(y + 12.0, y + h));
    let dead = sim.player.state == PState::Dead;
    let you = if dead { rgb(190, 190, 200) } else { rgb(255, 92, 92) };
    let blink = ((sim.time * 3.0) as i32) % 2 == 0;
    gfx::rect(px - 5.0, py - 12.0, 10.0, 10.0, rgb(255, 255, 255));
    gfx::rect(px - 3.0, py - 10.0, 6.0, 6.0, if blink { you } else { alpha(you, 0.75) });
    if !dead {
        let f = sim.player.facing;
        gfx::tri(px + f * 6.0, py - 11.0, px + f * 6.0, py - 3.0, px + f * 10.0, py - 7.0, rgb(255, 255, 255));
    }

    gfx::rect_lines(x, y, w, h, 1.0, rgba(255, 255, 255, 50));
    if let Some(t) = tip {
        let label = format!("{}  [{}] cycles size", t, key);
        let tw = gfx::text_w(&label, 13.0) + 16.0;
        gfx::rrect(gfx::VW - MARGIN - tw, y + h + 12.0, tw, 22.0, 5.0, rgba(18, 20, 36, 235));
        gfx::text(&label, gfx::VW - MARGIN - tw + 8.0, y + h + 15.0, 13.0, rgb(230, 234, 245));
    }
    act
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn buttons_adjust_settings() {
        let mut s = Settings::default();
        assert_eq!(s.minimap_size, 2);
        assert!(apply(&mut s, MinimapAction::Bigger));
        assert_eq!(s.minimap_size, 3);
        assert!(!apply(&mut s, MinimapAction::Bigger), "already largest");
        apply(&mut s, MinimapAction::Smaller);
        apply(&mut s, MinimapAction::Smaller);
        assert!(!apply(&mut s, MinimapAction::Smaller), "smallest visible size");
        assert_eq!(s.minimap_size, 1);
        apply(&mut s, MinimapAction::Hide);
        assert_eq!(s.minimap_size, 0);
        apply(&mut s, MinimapAction::Show);
        assert_eq!(s.minimap_size, 2);
        apply(&mut s, MinimapAction::ToggleWhole);
        assert!(s.minimap_whole);
        for _ in 0..4 {
            cycle(&mut s);
        }
        assert_eq!(s.minimap_size, 2, "cycling comes back round");
    }
}
