//! Pixel art for pets (facing right; flip for left). Two frames each: a
//! step / wing-beat pair that alternates while the pet moves.

use crate::pixel::{light, rgb, shadow, Canvas, Px, INK};
use galebound_core::pets::PetSpecies;

pub const W: i32 = 22;
pub const H: i32 = 18;

pub fn pet(species: PetSpecies, frame: u8) -> Canvas {
    let mut c = Canvas::new(W, H);
    let f = (frame % 2) as i32;
    match species {
        PetSpecies::BreezeFox => fox(&mut c, f),
        PetSpecies::Puffling => puffling(&mut c, f),
        PetSpecies::Sparkwing => sparkwing(&mut c, f),
    }
    let mut out = c.outlined(INK);
    // eyes go on after the outline so they stay crisp
    let eye: Px = rgb(26, 18, 30);
    match species {
        PetSpecies::BreezeFox => {
            out.set(17, 7, eye);
            out.set(17, 6, rgb(255, 255, 255));
        }
        PetSpecies::Puffling => {
            out.set(16, 8, eye);
            out.set(13, 8, eye);
        }
        PetSpecies::Sparkwing => {
            out.set(14, 6, eye);
            out.set(14, 5, rgb(255, 255, 255));
        }
    }
    out
}

fn fox(c: &mut Canvas, f: i32) {
    let fur: Px = rgb(236, 132, 52);
    let cream: Px = rgb(252, 236, 210);
    // big fluffy tail with a white tip
    c.ellipse(5.0, 7.0 + f as f32 * 0.5, 4.0, 3.0, fur);
    c.disc(2.5, 5.5 + f as f32 * 0.5, 2.0, cream);
    // body
    c.ellipse(10.5, 10.5, 5.0, 3.0, fur);
    c.rect(8, 12, 5, 1, cream);
    // head and snout
    c.disc(15.5, 7.5, 3.4, fur);
    c.rect(17, 8, 4, 2, cream);
    c.set(21, 8, rgb(40, 30, 30));
    // ears
    c.poly(&[(13.0, 5.0), (14.0, 1.0), (16.0, 4.5)], fur);
    c.poly(&[(16.0, 4.5), (18.0, 1.0), (19.0, 5.0)], fur);
    c.set(14, 3, shadow(fur, 0.4));
    c.set(18, 3, shadow(fur, 0.4));
    // legs (alternate while trotting)
    let leg = shadow(fur, 0.35);
    let (a, b) = if f == 0 { (0, 1) } else { (1, 0) };
    c.rect(7 + a, 13, 2, 4, leg);
    c.rect(13 - b, 13, 2, 4, leg);
    c.rect(8, 16, 1, 1, rgb(60, 40, 40));
    c.rect(13, 16, 1, 1, rgb(60, 40, 40));
    c.rect(6, 9, 8, 1, light(fur, 0.25));
}

fn puffling(c: &mut Canvas, f: i32) {
    let wool: Px = rgb(246, 246, 252);
    let face: Px = rgb(92, 84, 110);
    let bob = f as f32 * 0.5;
    // cloud body made of puffs
    for (x, y, r) in [(6.0, 9.0, 3.5), (10.0, 7.0, 4.0), (9.0, 11.0, 3.6), (13.0, 10.0, 3.5), (4.5, 11.5, 2.5)] {
        c.disc(x, y + bob, r, wool);
    }
    // shading on the underside
    for x in 4..16 {
        c.set_on(x, 14, shadow(wool, 0.18));
    }
    // face and little horns
    c.ellipse(15.0, 8.5 + bob, 3.0, 2.6, face);
    c.set(13, 5, rgb(214, 190, 140));
    c.set(17, 5, rgb(214, 190, 140));
    c.set(14, 10, light(face, 0.3));
    // stubby legs
    let (a, b) = if f == 0 { (0, 1) } else { (1, 0) };
    c.rect(6, 14 + a, 2, 3 - a, face);
    c.rect(12, 14 + b, 2, 3 - b, face);
    // a sky-blue tuft
    c.disc(9.0, 4.0 + bob, 1.5, rgb(170, 220, 255));
}

fn sparkwing(c: &mut Canvas, f: i32) {
    let blue: Px = rgb(70, 140, 236);
    let belly: Px = rgb(220, 236, 255);
    let gold: Px = rgb(250, 210, 70);
    // round body and head
    c.disc(10.0, 10.0, 5.0, blue);
    c.disc(13.0, 6.5, 3.6, blue);
    c.ellipse(11.0, 12.0, 3.0, 2.0, belly);
    // beak
    c.poly(&[(16.0, 6.0), (19.0, 7.0), (16.0, 8.0)], gold);
    // lightning crest
    c.set(12, 2, gold);
    c.set(13, 1, gold);
    c.set(13, 3, gold);
    c.set(14, 2, gold);
    // tail feathers
    c.poly(&[(5.0, 10.0), (1.0, 8.0), (2.0, 12.0)], shadow(blue, 0.3));
    // wing: up or down
    if f == 0 {
        c.poly(&[(7.0, 8.0), (11.0, 8.0), (6.0, 2.0)], light(blue, 0.3));
    } else {
        c.poly(&[(7.0, 9.0), (11.0, 9.0), (6.0, 15.0)], light(blue, 0.3));
    }
    // feet tucked under
    c.set(9, 15, gold);
    c.set(11, 15, gold);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_pet_has_two_frames() {
        for &s in galebound_core::pets::ALL_SPECIES {
            for f in 0..2 {
                assert!(pet(s, f).bounds().is_some());
            }
            assert_ne!(pet(s, 0).px, pet(s, 1).px, "{:?} frames differ", s);
        }
    }
}
