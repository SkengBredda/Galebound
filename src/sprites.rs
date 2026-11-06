//! Texture cache: pixel art is generated on first use and kept on the GPU.

use crate::art;
use crate::charsprite::{self, Anim};
use crate::gfx;
use crate::pixel::Canvas;
use galebound_core::appearance::Appearance;
use galebound_core::classes::Weapon;
use galebound_core::world::Theme;
use macroquad::prelude::Texture2D;
use std::cell::RefCell;
use std::collections::HashMap;

type CharKey = (Appearance, Option<Weapon>, Anim, u8);

#[derive(Default)]
struct Cache {
    chars: HashMap<CharKey, Texture2D>,
    misc: HashMap<String, Texture2D>,
}

thread_local! {
    static CACHE: RefCell<Cache> = RefCell::new(Cache::default());
}

const MAX_CHAR_FRAMES: usize = 1500;

/// A character frame (96x96, feet at 48,88, facing right).
pub fn character(look: &Appearance, weapon: Option<Weapon>, anim: Anim, frame: u8) -> Texture2D {
    let key = (*look, weapon, anim, frame % anim.frames());
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        if let Some(t) = c.chars.get(&key) {
            return t.clone();
        }
        if c.chars.len() > MAX_CHAR_FRAMES {
            c.chars.clear();
        }
        let canvas = charsprite::render(look, weapon, anim, key.3);
        let t = gfx::texture_from(&canvas);
        c.chars.insert(key, t.clone());
        t
    })
}

/// Any other generated sprite, keyed by name.
pub fn get(key: &str, make: impl FnOnce() -> Canvas) -> Texture2D {
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        if let Some(t) = c.misc.get(key) {
            return t.clone();
        }
        let t = gfx::texture_from(&make());
        c.misc.insert(key.to_string(), t.clone());
        t
    })
}

// Convenience constructors for common art (cached by key).
pub fn cobble(theme: Theme) -> Texture2D {
    get(&format!("cobble_{:?}", theme), || art::cobble_tile(3, theme))
}
pub fn grass(theme: Theme) -> Texture2D {
    get(&format!("grass_{:?}", theme), || art::grass_top(1, theme))
}
pub fn road() -> Texture2D {
    get("road_top", art::town_road_top)
}
