//! Pixel-art rendering pipeline.
//!
//! Everything is drawn into a 640x360 render target with no smoothing, then
//! scaled to the window with hard pixel edges (integer scale when possible).
//! Callers keep working in a 1280x720 "virtual" UI space; world drawing uses
//! a camera (`set_cam`) where 1 world unit = 0.85 canvas pixels.
//! Text uses the hand-made pixel font in `font.rs` via a glyph atlas.

use crate::font::Font;
use crate::pixel::Canvas;
use macroquad::prelude::{
    clear_background, draw_circle, draw_circle_lines, draw_ellipse, draw_line, draw_rectangle,
    draw_texture_ex, draw_triangle, mouse_position, render_target,
    screen_height, screen_width, set_camera, set_default_camera, vec2, Camera2D, Color,
    DrawTextureParams, FilterMode, Rect, RenderTarget, Texture2D,
};
use std::cell::{Cell, RefCell};

/// Virtual UI space (all UI code uses these coordinates).
pub const VW: f32 = 1280.0;
pub const VH: f32 = 720.0;
/// The pixel canvas actually rendered.
pub const CW: f32 = 640.0;
pub const CH: f32 = 360.0;
/// Canvas pixels per virtual unit.
const UI_S: f32 = CW / VW;
/// Canvas pixels per world unit.
pub const WORLD_PX: f32 = 0.85;
/// World zoom relative to UI space.
pub const WORLD_ZOOM: f32 = WORLD_PX / UI_S;
/// Size of the visible world area in world units.
pub const VIEW_W: f32 = CW / WORLD_PX;
pub const VIEW_H: f32 = CH / WORLD_PX;

#[derive(Clone, Copy)]
struct Xf {
    cx: f32,
    cy: f32,
    z: f32,
}

struct State {
    rt: RenderTarget,
    cam: Camera2D,
    font: Font,
    atlas: Texture2D,
    /// Where the canvas lands on screen: (x, y, scale).
    blit: (f32, f32, f32),
}

thread_local! {
    static XF: Cell<Xf> = const { Cell::new(Xf { cx: 0.0, cy: 0.0, z: 1.0 }) };
    static ST: RefCell<Option<State>> = const { RefCell::new(None) };
}

/// Create the render target and font atlas. Call once at startup.
pub fn init() {
    let rt = render_target(CW as u32, CH as u32);
    rt.texture.set_filter(FilterMode::Nearest);
    let mut cam = Camera2D::from_display_rect(Rect::new(0.0, 0.0, CW, CH));
    cam.render_target = Some(rt.clone());
    let font = Font::new();
    // Glyph atlas: 128 cells of 6x8, white pixels.
    let mut c = Canvas::new(128 * 6, 8);
    for code in 32u8..127 {
        let ch = code as char;
        if let Some(g) = font.glyph(ch) {
            for (r, bits) in g.rows.iter().enumerate() {
                for col in 0..5 {
                    if bits & (1 << col) != 0 {
                        c.set(code as i32 * 6 + col, r as i32, [255, 255, 255, 255]);
                    }
                }
            }
        }
    }
    let atlas = Texture2D::from_rgba8(c.w as u16, c.h as u16, &c.bytes());
    atlas.set_filter(FilterMode::Nearest);
    ST.with(|s| *s.borrow_mut() = Some(State { rt, cam, font, atlas, blit: (0.0, 0.0, 1.0) }));
}

fn with<R>(f: impl FnOnce(&State) -> R) -> R {
    ST.with(|s| f(s.borrow().as_ref().expect("gfx::init() not called")))
}

/// Start drawing a frame into the pixel canvas.
pub fn begin_frame(bg: Color) {
    with(|s| set_camera(&s.cam));
    XF.with(|x| x.set(Xf { cx: 0.0, cy: 0.0, z: 1.0 }));
    clear_background(bg);
}

/// Present the pixel canvas to the window (call last each frame).
pub fn end_frame() {
    set_default_camera();
    clear_background(Color::new(0.0, 0.0, 0.0, 1.0));
    let (sw, sh) = (screen_width(), screen_height());
    let fit = (sw / CW).min(sh / CH);
    // Integer scaling keeps every pixel the same size; fall back to
    // fractional only when the window is smaller than one canvas.
    let scale = if fit >= 1.0 { fit.floor() } else { fit };
    let (w, h) = (CW * scale, CH * scale);
    let (x, y) = (((sw - w) * 0.5).floor(), ((sh - h) * 0.5).floor());
    ST.with(|s| {
        if let Some(st) = s.borrow_mut().as_mut() {
            st.blit = (x, y, scale);
            draw_texture_ex(
                &st.rt.texture,
                x,
                y,
                Color::new(1.0, 1.0, 1.0, 1.0),
                DrawTextureParams { dest_size: Some(vec2(w, h)), flip_y: true, ..Default::default() },
            );
        }
    });
}

/// Draw in world space: camera top-left at (cx, cy) world units.
/// The camera is snapped to whole canvas pixels so nothing shimmers.
pub fn set_cam(cx: f32, cy: f32) {
    let sx = (cx * WORLD_PX).round() / WORLD_PX;
    let sy = (cy * WORLD_PX).round() / WORLD_PX;
    XF.with(|x| x.set(Xf { cx: sx, cy: sy, z: WORLD_ZOOM }));
}
/// Back to UI space (no camera, no zoom).
pub fn clear_cam() {
    XF.with(|x| x.set(Xf { cx: 0.0, cy: 0.0, z: 1.0 }));
}

#[inline]
fn xf() -> Xf {
    XF.with(|x| x.get())
}
/// Current space -> canvas pixels.
#[inline]
fn p(x: f32, y: f32) -> (f32, f32) {
    let t = xf();
    ((x - t.cx) * t.z * UI_S, (y - t.cy) * t.z * UI_S)
}
#[inline]
fn sc(v: f32) -> f32 {
    v * xf().z * UI_S
}
/// Snap to whole canvas pixels.
#[inline]
fn pr(x: f32, y: f32) -> (f32, f32) {
    let (a, b) = p(x, y);
    (a.round(), b.round())
}

// ---------------------------------------------------------------- colours

pub fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgba(r, g, b, 255)
}
pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color {
    Color::from_rgba(r, g, b, a)
}
#[allow(dead_code)]
pub fn from3(c: [u8; 3]) -> Color {
    rgb(c[0], c[1], c[2])
}
/// Multiply brightness (0.5 = darker, 1.3 = lighter).
pub fn shade(c: Color, f: f32) -> Color {
    if f <= 1.0 {
        Color::new(c.r * f, c.g * f, c.b * f, c.a)
    } else {
        let t = (f - 1.0).min(1.0);
        Color::new(c.r + (1.0 - c.r) * t, c.g + (1.0 - c.g) * t, c.b + (1.0 - c.b) * t, c.a)
    }
}
pub fn alpha(c: Color, a: f32) -> Color {
    Color::new(c.r, c.g, c.b, c.a * a)
}
pub fn mix(a: Color, b: Color, t: f32) -> Color {
    Color::new(a.r + (b.r - a.r) * t, a.g + (b.g - a.g) * t, a.b + (b.b - a.b) * t, a.a + (b.a - a.a) * t)
}

// ---------------------------------------------------------------- shapes (all pixel-snapped)

pub fn rect(x: f32, y: f32, w: f32, h: f32, c: Color) {
    let (x0, y0) = pr(x, y);
    let (x1, y1) = pr(x + w, y + h);
    if x1 > x0 && y1 > y0 {
        draw_rectangle(x0, y0, x1 - x0, y1 - y0, c);
    }
}
pub fn rect_lines(x: f32, y: f32, w: f32, h: f32, th: f32, c: Color) {
    let (x0, y0) = pr(x, y);
    let (x1, y1) = pr(x + w, y + h);
    let t = sc(th).round().max(1.0);
    let (w, h) = (x1 - x0, y1 - y0);
    draw_rectangle(x0, y0, w, t, c);
    draw_rectangle(x0, y1 - t, w, t, c);
    draw_rectangle(x0, y0 + t, t, h - 2.0 * t, c);
    draw_rectangle(x1 - t, y0 + t, t, h - 2.0 * t, c);
}
/// Rounded rectangle with crisp stepped pixel corners.
pub fn rrect(x: f32, y: f32, w: f32, h: f32, r: f32, c: Color) {
    let (x0, y0) = pr(x, y);
    let (x1, y1) = pr(x + w, y + h);
    let (w, h) = (x1 - x0, y1 - y0);
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    let rp = sc(r).round().min((w / 2.0).floor()).min((h / 2.0).floor()).max(0.0) as i32;
    if rp <= 0 {
        draw_rectangle(x0, y0, w, h, c);
        return;
    }
    let rf = rp as f32;
    draw_rectangle(x0, y0 + rf, w, h - 2.0 * rf, c);
    for i in 0..rp {
        let dy = rf - i as f32 - 0.5;
        let inset = (rf - (rf * rf - dy * dy).max(0.0).sqrt()).round();
        draw_rectangle(x0 + inset, y0 + i as f32, w - 2.0 * inset, 1.0, c);
        draw_rectangle(x0 + inset, y1 - 1.0 - i as f32, w - 2.0 * inset, 1.0, c);
    }
}
pub fn circle(x: f32, y: f32, r: f32, c: Color) {
    let (sx, sy) = p(x, y);
    draw_circle(sx, sy, sc(r).max(0.5), c);
}
#[allow(dead_code)]
pub fn circle_lines(x: f32, y: f32, r: f32, th: f32, c: Color) {
    let (sx, sy) = p(x, y);
    draw_circle_lines(sx, sy, sc(r), sc(th).max(1.0), c);
}
/// Ellipse with radii rx, ry; rotation in degrees.
pub fn ellipse(x: f32, y: f32, rx: f32, ry: f32, rot: f32, c: Color) {
    let (sx, sy) = p(x, y);
    draw_ellipse(sx, sy, sc(rx).max(0.5), sc(ry).max(0.5), rot, c);
}
pub fn tri(ax: f32, ay: f32, bx: f32, by: f32, cx: f32, cy: f32, c: Color) {
    let (a, b, d) = (p(ax, ay), p(bx, by), p(cx, cy));
    draw_triangle(vec2(a.0, a.1), vec2(b.0, b.1), vec2(d.0, d.1), c);
}
pub fn line(x1: f32, y1: f32, x2: f32, y2: f32, th: f32, c: Color) {
    let (a, b) = (p(x1, y1), p(x2, y2));
    draw_line(a.0, a.1, b.0, b.1, sc(th).max(1.0), c);
}
/// Thick line with round caps.
#[allow(dead_code)]
pub fn limb(x1: f32, y1: f32, x2: f32, y2: f32, th: f32, c: Color) {
    line(x1, y1, x2, y2, th, c);
    circle(x1, y1, th * 0.5, c);
    circle(x2, y2, th * 0.5, c);
}
/// Vertical gradient using horizontal bands (posterised: pixel-art friendly).
pub fn vgradient(x: f32, y: f32, w: f32, h: f32, top: Color, bottom: Color, bands: u32) {
    let bh = h / bands as f32;
    for i in 0..bands {
        let t = i as f32 / (bands - 1).max(1) as f32;
        rect(x, y + i as f32 * bh, w, bh + 1.0, mix(top, bottom, t));
    }
}

// ---------------------------------------------------------------- sprites / textures

pub fn texture_from(c: &Canvas) -> Texture2D {
    let t = Texture2D::from_rgba8(c.w as u16, c.h as u16, &c.bytes());
    t.set_filter(FilterMode::Nearest);
    t
}

/// Draw a texture with its top-left at (x, y) in the current space, each
/// texture pixel `k` canvas pixels big (k = 1 for world sprites).
pub fn tex(t: &Texture2D, x: f32, y: f32, k: f32, flip_x: bool, tint: Color) {
    let (sx, sy) = pr(x, y);
    draw_texture_ex(
        t,
        sx,
        sy,
        tint,
        DrawTextureParams { dest_size: Some(vec2(t.width() * k, t.height() * k)), flip_x, ..Default::default() },
    );
}

/// Draw part of a texture (source rect in texture pixels) at canvas scale k.
pub fn tex_part(t: &Texture2D, x: f32, y: f32, src: (f32, f32, f32, f32), k: f32, tint: Color) {
    let (sx, sy) = pr(x, y);
    draw_texture_ex(
        t,
        sx,
        sy,
        tint,
        DrawTextureParams {
            dest_size: Some(vec2(src.2 * k, src.3 * k)),
            source: Some(Rect::new(src.0, src.1, src.2, src.3)),
            ..Default::default()
        },
    );
}

/// Convert a length in canvas pixels to the current space's units.
pub fn px_to_units(px: f32) -> f32 {
    px / (xf().z * UI_S)
}

// ---------------------------------------------------------------- text

/// Pixel-font scale for a requested size (in virtual units, as before).
fn font_k(size: f32) -> f32 {
    (size / 16.0).round().max(1.0)
}

/// Width of text in the current space's units.
pub fn text_w(s: &str, size: f32) -> f32 {
    let k = font_k(size);
    let w = with(|st| st.font.width(s)) as f32 * k;
    px_to_units(w)
}

/// Height of one line of text in current units.
#[allow(dead_code)]
pub fn text_h(size: f32) -> f32 {
    px_to_units(7.0 * font_k(size))
}

fn draw_glyphs(s: &str, x: f32, y: f32, size: f32, c: Color) {
    let k = font_k(size);
    let (mut cx, cy) = pr(x, y);
    with(|st| {
        for ch in s.chars() {
            let code = if (ch as u32) < 128 && st.font.glyph(ch).is_some() { ch as u32 } else if ch == ' ' { 32 } else { '?' as u32 };
            if ch != ' ' {
                draw_texture_ex(
                    &st.atlas,
                    cx,
                    cy,
                    c,
                    DrawTextureParams {
                        dest_size: Some(vec2(5.0 * k, 8.0 * k)),
                        source: Some(Rect::new(code as f32 * 6.0, 0.0, 5.0, 8.0)),
                        ..Default::default()
                    },
                );
            }
            cx += st.font.advance(ch) as f32 * k;
        }
    });
}

/// Draw text with its top-left at (x, y).
pub fn text(s: &str, x: f32, y: f32, size: f32, c: Color) {
    draw_glyphs(s, x, y, size, c);
}

/// Text with a dark 1-pixel outline (readable over anything).
pub fn text_outlined(s: &str, x: f32, y: f32, size: f32, c: Color) {
    let k = font_k(size);
    let o = rgba(14, 10, 22, (c.a * 255.0) as u8);
    let d = px_to_units(k);
    for (dx, dy) in [(-d, 0.0), (d, 0.0), (0.0, -d), (0.0, d), (d, d)] {
        draw_glyphs(s, x + dx, y + dy, size, o);
    }
    draw_glyphs(s, x, y, size, c);
}

pub fn text_centered(s: &str, cx: f32, y: f32, size: f32, c: Color) {
    text(s, cx - text_w(s, size) * 0.5, y, size, c);
}
pub fn text_centered_outlined(s: &str, cx: f32, y: f32, size: f32, c: Color) {
    text_outlined(s, cx - text_w(s, size) * 0.5, y, size, c);
}

/// Split text into lines that fit `max_w`.
pub fn wrap(s: &str, max_w: f32, size: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in s.split_whitespace() {
        let candidate = if cur.is_empty() { word.to_string() } else { format!("{} {}", cur, word) };
        if text_w(&candidate, size) > max_w && !cur.is_empty() {
            lines.push(std::mem::take(&mut cur));
            cur = word.to_string();
        } else {
            cur = candidate;
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

/// Line spacing for wrapped text at a size, in current units.
pub fn line_h(size: f32) -> f32 {
    px_to_units(crate::font::LINE_H as f32 * font_k(size))
}

/// Draws wrapped text; returns the height used.
pub fn text_wrapped(s: &str, x: f32, y: f32, max_w: f32, size: f32, c: Color) -> f32 {
    let lh = line_h(size);
    let lines = wrap(s, max_w, size);
    for (i, l) in lines.iter().enumerate() {
        text(l, x, y + i as f32 * lh, size, c);
    }
    lines.len() as f32 * lh
}

// ---------------------------------------------------------------- input

/// Mouse position in virtual (UI) coordinates.
pub fn mouse() -> (f32, f32) {
    let (mx, my) = mouse_position();
    let (bx, by, k) = ST.with(|s| s.borrow().as_ref().map(|st| st.blit).unwrap_or((0.0, 0.0, 1.0)));
    (((mx - bx) / k) / UI_S, ((my - by) / k) / UI_S)
}

/// The mouse position in world units, for a camera at (cx, cy).
pub fn mouse_world(cx: f32, cy: f32) -> (f32, f32) {
    let (mx, my) = mouse();
    (mx / WORLD_ZOOM + cx, my / WORLD_ZOOM + cy)
}

pub fn mouse_in(x: f32, y: f32, w: f32, h: f32) -> bool {
    let (mx, my) = mouse();
    mx >= x && mx <= x + w && my >= y && my <= y + h
}
