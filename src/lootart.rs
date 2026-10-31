//! Pixel art for things lying in the world: gold piles and the tombstone a
//! defeated character turns into. Drawn on the CPU canvas like every sprite.

use crate::pixel::{light, rgb, shadow, Canvas, Px, INK};

/// A pile of coins; bigger amounts make a bigger pile (`size` 0-2).
pub fn coin_pile(size: u8) -> Canvas {
    let mut c = Canvas::new(16, 12);
    let gold: Px = rgb(240, 196, 70);
    let coins: &[(i32, i32)] = match size {
        0 => &[(6, 7)],
        1 => &[(3, 8), (8, 8), (5, 5)],
        _ => &[(1, 8), (6, 8), (10, 8), (3, 5), (8, 5), (5, 2)],
    };
    for &(x, y) in coins {
        c.rect(x, y + 1, 5, 2, shadow(gold, 0.35));
        c.rect(x, y, 5, 2, gold);
        c.rect(x + 1, y, 2, 1, light(gold, 0.6));
    }
    c.outlined(INK)
}

/// A rounded headstone with a little cross and some grass at its foot.
pub fn tombstone() -> Canvas {
    let mut c = Canvas::new(22, 28);
    let stone: Px = rgb(150, 156, 172);
    // body with a rounded top
    c.rect(4, 8, 14, 17, stone);
    c.ellipse(11.0, 8.5, 7.0, 6.0, stone);
    // base slab
    c.rect(2, 23, 18, 4, shadow(stone, 0.25));
    // shading: right side darker, left rim lighter
    for y in 3..23 {
        c.set_on(16, y, shadow(stone, 0.2));
        c.set_on(17, y, shadow(stone, 0.3));
        c.set_on(5, y, light(stone, 0.25));
    }
    // engraved cross
    let groove = shadow(stone, 0.5);
    c.rect(10, 8, 2, 10, groove);
    c.rect(7, 11, 8, 2, groove);
    // a crack and moss
    c.set_on(14, 19, groove);
    c.set_on(15, 20, groove);
    let moss = rgb(96, 150, 78);
    for (x, y) in [(4, 22), (5, 22), (5, 21), (16, 22), (17, 22)] {
        c.set_on(x, y, moss);
    }
    let mut out = c.outlined(INK);
    // grass tufts in front (drawn after the outline so they overlap it)
    let grass = rgb(110, 176, 80);
    for (x, h) in [(1, 3), (3, 2), (18, 3), (20, 2)] {
        for k in 0..h {
            out.set(x, 27 - k, grass);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn art_has_pixels() {
        for c in [coin_pile(0), coin_pile(1), coin_pile(2), tombstone()] {
            assert!(c.bounds().is_some());
        }
    }
}
