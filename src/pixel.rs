//! CPU pixel canvas used to author all sprites in code: exact pixel shapes,
//! per-part dark outlines, and palette helpers. Pure Rust (no engine types),
//! so sprites can be generated, cached and unit-previewed anywhere.

pub type Px = [u8; 4];

pub const CLEAR: Px = [0, 0, 0, 0];
/// The near-black, slightly purple outline used everywhere (anime pixel style).
pub const INK: Px = [26, 18, 30, 255];

pub fn rgb(r: u8, g: u8, b: u8) -> Px {
    [r, g, b, 255]
}
pub fn from3(c: [u8; 3]) -> Px {
    [c[0], c[1], c[2], 255]
}

/// Darken with a slight hue shift toward purple/blue (pixel-art shadow).
pub fn shadow(c: Px, k: f32) -> Px {
    let f = |v: u8, bias: f32| ((v as f32) * (1.0 - k) + bias * k).clamp(0.0, 255.0) as u8;
    [f(c[0], 30.0), f(c[1], 20.0), f(c[2], 60.0), c[3]]
}
/// Lighten with a slight warm shift (pixel-art highlight).
pub fn light(c: Px, k: f32) -> Px {
    let f = |v: u8, t: f32| ((v as f32) + (t - v as f32) * k).clamp(0.0, 255.0) as u8;
    [f(c[0], 255.0), f(c[1], 250.0), f(c[2], 225.0), c[3]]
}
pub fn mix(a: Px, b: Px, t: f32) -> Px {
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    [f(a[0], b[0]), f(a[1], b[1]), f(a[2], b[2]), f(a[3], b[3])]
}

#[derive(Clone)]
pub struct Canvas {
    pub w: i32,
    pub h: i32,
    pub px: Vec<Px>,
}

impl Canvas {
    pub fn new(w: i32, h: i32) -> Canvas {
        Canvas { w, h, px: vec![CLEAR; (w * h) as usize] }
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32) -> Px {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            CLEAR
        } else {
            self.px[(y * self.w + x) as usize]
        }
    }
    #[inline]
    pub fn set(&mut self, x: i32, y: i32, c: Px) {
        if x < 0 || y < 0 || x >= self.w || y >= self.h || c[3] == 0 {
            return;
        }
        let i = (y * self.w + x) as usize;
        if c[3] == 255 {
            self.px[i] = c;
        } else {
            let d = self.px[i];
            let a = c[3] as f32 / 255.0;
            let da = d[3] as f32 / 255.0;
            let oa = a + da * (1.0 - a);
            let ch = |s: u8, t: u8| ((s as f32 * a + t as f32 * da * (1.0 - a)) / oa.max(1e-4)) as u8;
            self.px[i] = [ch(c[0], d[0]), ch(c[1], d[1]), ch(c[2], d[2]), (oa * 255.0) as u8];
        }
    }
    /// Only paints over pixels that are already opaque (for shading inside a shape).
    pub fn set_on(&mut self, x: i32, y: i32, c: Px) {
        if self.get(x, y)[3] > 0 {
            self.set(x, y, c);
        }
    }
    pub fn opaque(&self, x: i32, y: i32) -> bool {
        self.get(x, y)[3] > 0
    }

    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: Px) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.set(xx, yy, c);
            }
        }
    }
    /// Filled disc; radius in pixels (half-pixel centres for nice small circles).
    pub fn disc(&mut self, cx: f32, cy: f32, r: f32, c: Px) {
        self.ellipse(cx, cy, r, r, c);
    }
    pub fn ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, c: Px) {
        let x0 = (cx - rx - 1.0).floor() as i32;
        let x1 = (cx + rx + 1.0).ceil() as i32;
        let y0 = (cy - ry - 1.0).floor() as i32;
        let y1 = (cy + ry + 1.0).ceil() as i32;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let dx = (x as f32 + 0.5 - cx) / rx.max(0.3);
                let dy = (y as f32 + 0.5 - cy) / ry.max(0.3);
                if dx * dx + dy * dy <= 1.0 {
                    self.set(x, y, c);
                }
            }
        }
    }
    /// Filled polygon (even-odd, sampled at pixel centres).
    pub fn poly(&mut self, pts: &[(f32, f32)], c: Px) {
        if pts.len() < 3 {
            return;
        }
        let y0 = pts.iter().map(|p| p.1).fold(f32::MAX, f32::min).floor() as i32;
        let y1 = pts.iter().map(|p| p.1).fold(f32::MIN, f32::max).ceil() as i32;
        let n = pts.len();
        for y in y0..=y1 {
            let sy = y as f32 + 0.5;
            let mut xs: Vec<f32> = Vec::new();
            for i in 0..n {
                let (a, b) = (pts[i], pts[(i + 1) % n]);
                if (a.1 <= sy && b.1 > sy) || (b.1 <= sy && a.1 > sy) {
                    xs.push(a.0 + (sy - a.1) / (b.1 - a.1) * (b.0 - a.0));
                }
            }
            xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
            for pair in xs.chunks(2) {
                if pair.len() == 2 {
                    let xa = (pair[0] - 0.5).ceil() as i32;
                    let xb = (pair[1] - 0.5).floor() as i32;
                    for x in xa..=xb {
                        self.set(x, y, c);
                    }
                }
            }
        }
    }
    /// 1px line (Bresenham).
    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: Px) {
        let (mut x, mut y) = (x0, y0);
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            self.set(x, y, c);
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }
    pub fn linef(&mut self, a: (f32, f32), b: (f32, f32), c: Px) {
        self.line(a.0.round() as i32, a.1.round() as i32, b.0.round() as i32, b.1.round() as i32, c);
    }
    /// Thick segment with round ends (limbs). `w` = width in pixels.
    pub fn capsule(&mut self, a: (f32, f32), b: (f32, f32), w: f32, c: Px) {
        let r = w * 0.5;
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len2 = (dx * dx + dy * dy).max(1e-4);
        let x0 = (a.0.min(b.0) - r - 1.0).floor() as i32;
        let x1 = (a.0.max(b.0) + r + 1.0).ceil() as i32;
        let y0 = (a.1.min(b.1) - r - 1.0).floor() as i32;
        let y1 = (a.1.max(b.1) + r + 1.0).ceil() as i32;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                let t = (((px - a.0) * dx + (py - a.1) * dy) / len2).clamp(0.0, 1.0);
                let (qx, qy) = (a.0 + dx * t - px, a.1 + dy * t - py);
                if qx * qx + qy * qy <= r * r {
                    self.set(x, y, c);
                }
            }
        }
    }
    /// Tapered segment: width wa at a, wb at b (hair strands, tails, blades).
    pub fn taper(&mut self, a: (f32, f32), b: (f32, f32), wa: f32, wb: f32, c: Px) {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let l = (dx * dx + dy * dy).sqrt().max(1e-4);
        let (nx, ny) = (-dy / l, dx / l);
        let (ha, hb) = (wa * 0.5, wb * 0.5);
        self.poly(&[(a.0 + nx * ha, a.1 + ny * ha), (b.0 + nx * hb, b.1 + ny * hb), (b.0 - nx * hb, b.1 - ny * hb), (a.0 - nx * ha, a.1 - ny * ha)], c);
    }

    /// Composite another canvas on top at an offset.
    pub fn draw(&mut self, o: &Canvas, ox: i32, oy: i32) {
        for y in 0..o.h {
            for x in 0..o.w {
                let c = o.px[(y * o.w + x) as usize];
                if c[3] > 0 {
                    self.set(x + ox, y + oy, c);
                }
            }
        }
    }

    /// Add a 1px outline around every opaque pixel (4-neighbourhood).
    pub fn outlined(&self, c: Px) -> Canvas {
        let mut out = self.clone();
        for y in 0..self.h {
            for x in 0..self.w {
                if !self.opaque(x, y) && (self.opaque(x - 1, y) || self.opaque(x + 1, y) || self.opaque(x, y - 1) || self.opaque(x, y + 1)) {
                    out.px[(y * self.w + x) as usize] = c;
                }
            }
        }
        out
    }

    /// Shade the lower/back side of a shape: any opaque pixel whose neighbour
    /// in direction (dx, dy) is transparent gets the shadow colour.
    pub fn rim_shade(&mut self, dx: i32, dy: i32, depth: i32, c: Px) {
        let src = self.clone();
        for y in 0..self.h {
            for x in 0..self.w {
                if !src.opaque(x, y) {
                    continue;
                }
                for d in 1..=depth {
                    if !src.opaque(x + dx * d, y + dy * d) {
                        self.px[(y * self.w + x) as usize] = c;
                        break;
                    }
                }
            }
        }
    }

    /// Recolour every opaque pixel (keeps alpha).
    pub fn tint_all(&mut self, c: Px) {
        for p in self.px.iter_mut() {
            if p[3] > 0 {
                *p = [c[0], c[1], c[2], p[3]];
            }
        }
    }

    #[allow(dead_code)]
    pub fn flipped_x(&self) -> Canvas {
        let mut out = Canvas::new(self.w, self.h);
        for y in 0..self.h {
            for x in 0..self.w {
                out.px[(y * self.w + (self.w - 1 - x)) as usize] = self.px[(y * self.w + x) as usize];
            }
        }
        out
    }

    pub fn bytes(&self) -> Vec<u8> {
        self.px.iter().flat_map(|p| p.iter().copied()).collect()
    }

    /// Tight bounds of opaque pixels: (x0, y0, x1, y1) inclusive.
    #[allow(dead_code)]
    pub fn bounds(&self) -> Option<(i32, i32, i32, i32)> {
        let mut b: Option<(i32, i32, i32, i32)> = None;
        for y in 0..self.h {
            for x in 0..self.w {
                if self.opaque(x, y) {
                    b = Some(match b {
                        None => (x, y, x, y),
                        Some((a, c, d, e)) => (a.min(x), c.min(y), d.max(x), e.max(y)),
                    });
                }
            }
        }
        b
    }
}

/// A "part" drawn on its own layer then outlined, so it reads clearly against
/// whatever is behind it (the core trick of the anime pixel look).
pub struct Layer {
    pub c: Canvas,
}

impl Layer {
    pub fn new(w: i32, h: i32) -> Layer {
        Layer { c: Canvas::new(w, h) }
    }
    /// Outline it and stamp onto `dst`.
    pub fn stamp(self, dst: &mut Canvas, outline: Px) {
        let o = self.c.outlined(outline);
        dst.draw(&o, 0, 0);
    }
}

// ---------------------------------------------------------------- tiny PNG writer (debug previews)

fn crc32(d: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in d {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
    }
    !c
}

/// Encode RGBA as an (uncompressed) PNG. Used by debug/preview tooling only.
#[allow(dead_code)]
pub fn png_rgba(w: u32, h: u32, rgba: &[u8]) -> Vec<u8> {
    let mut raw = Vec::with_capacity((w * 4 + 1) as usize * h as usize);
    for y in 0..h as usize {
        raw.push(0u8);
        raw.extend_from_slice(&rgba[y * w as usize * 4..(y + 1) * w as usize * 4]);
    }
    let mut z = vec![0x78, 0x01];
    let chunks: Vec<&[u8]> = raw.chunks(65535).collect();
    for (i, ch) in chunks.iter().enumerate() {
        z.push((i + 1 == chunks.len()) as u8);
        z.extend_from_slice(&(ch.len() as u16).to_le_bytes());
        z.extend_from_slice(&(!(ch.len() as u16)).to_le_bytes());
        z.extend_from_slice(ch);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &x in &raw {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut chunk = |ty: &[u8], data: &[u8]| {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut c = ty.to_vec();
        c.extend_from_slice(data);
        out.extend_from_slice(&c);
        out.extend_from_slice(&crc32(&c).to_be_bytes());
    };
    let mut ih = Vec::new();
    ih.extend_from_slice(&w.to_be_bytes());
    ih.extend_from_slice(&h.to_be_bytes());
    ih.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(b"IHDR", &ih);
    chunk(b"IDAT", &z);
    chunk(b"IEND", &[]);
    out
}
