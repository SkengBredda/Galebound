//! Very light platformer physics: gravity, velocity integration and AABB
//! collision against solid blocks and one-way platforms. Movement is resolved
//! one axis at a time (X then Y), which is simple, stable and fast.

use crate::math::{Aabb, V2};

// Tuned for a floatier, calmer MMO feel (about 2/3 of a platform fighter's
// gravity): jumps hang in the air longer and knocked-back monsters arc
// instead of dropping like rocks.
pub const GRAVITY: f32 = 1750.0; // px/s^2
pub const FAST_FALL_GRAVITY: f32 = 2900.0;
pub const MAX_FALL: f32 = 780.0;
pub const FAST_FALL_SPEED: f32 = 1080.0;

/// Everything in the world (players, monsters, projectiles, attacks) runs at
/// this fraction of real time, for a calmer pace. Timers that players read
/// as real time (pet hunger, play time, toasts) are not scaled.
pub const GAME_SPEED: f32 = 0.88;
/// Players fall more gently than monsters: lower jumps that hang longer,
/// for more air-combo time.
pub const PLAYER_GRAVITY: f32 = 0.70;
/// Jump launch speed scale, paired with [`PLAYER_GRAVITY`]
/// (about 8% lower apex, about 20% more time in the air).
pub const PLAYER_JUMP: f32 = 0.80;
/// Scale for upward attack lunges so they keep their height under the
/// lighter player gravity (sqrt of PLAYER_GRAVITY).
pub const PLAYER_LIFT: f32 = 0.837;

/// Scale for horizontal speeds tuned under the old, faster pacing.
pub const PACE_H: f32 = 0.8;
/// Scale for vertical launch speeds (sqrt of the gravity ratio keeps arc heights).
pub const PACE_V: f32 = 0.82;

#[derive(Clone, Debug, Default)]
pub struct Body {
    pub pos: V2, // top-left
    pub vel: V2,
    pub w: f32,
    pub h: f32,
    pub on_ground: bool,
    pub hit_wall: bool,
    pub hit_ceiling: bool,
    /// Seconds left during which one-way platforms are ignored (drop-through).
    pub drop_timer: f32,
}

impl Body {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Body { pos: V2::new(x, y), w, h, ..Default::default() }
    }
    pub fn aabb(&self) -> Aabb {
        Aabb::new(self.pos.x, self.pos.y, self.w, self.h)
    }
    pub fn feet(&self) -> V2 {
        V2::new(self.pos.x + self.w * 0.5, self.pos.y + self.h)
    }
    pub fn center(&self) -> V2 {
        V2::new(self.pos.x + self.w * 0.5, self.pos.y + self.h * 0.5)
    }
}

pub struct Level<'a> {
    pub solids: &'a [Aabb],
    pub platforms: &'a [Aabb], // one-way: only collide from above
    pub bounds: Aabb,
}

/// Apply gravity to a body. `fast_fall` uses heavier gravity (down held in air).
pub fn apply_gravity(b: &mut Body, dt: f32, fast_fall: bool, gravity_scale: f32) {
    if b.on_ground {
        return;
    }
    let (g, cap) = if fast_fall { (FAST_FALL_GRAVITY, FAST_FALL_SPEED) } else { (GRAVITY, MAX_FALL) };
    b.vel.y = (b.vel.y + g * gravity_scale * dt).min(cap);
}

/// Integrate velocity and resolve collisions.
pub fn move_and_collide(b: &mut Body, lvl: &Level, dt: f32) {
    if b.drop_timer > 0.0 {
        b.drop_timer -= dt;
    }
    b.hit_wall = false;
    b.hit_ceiling = false;

    // ---- X axis
    let dx = b.vel.x * dt;
    b.pos.x += dx;
    let mut r = b.aabb();
    for s in lvl.solids {
        if r.overlaps(s) {
            if dx > 0.0 {
                b.pos.x = s.left() - b.w;
            } else if dx < 0.0 {
                b.pos.x = s.right();
            } else {
                // Spawned inside: push out the shortest way horizontally.
                let c = r.center().x;
                if c < s.center().x { b.pos.x = s.left() - b.w } else { b.pos.x = s.right() }
            }
            b.vel.x = 0.0;
            b.hit_wall = true;
            r = b.aabb();
        }
    }
    // World bounds (left/right)
    if b.pos.x < lvl.bounds.left() {
        b.pos.x = lvl.bounds.left();
        b.vel.x = 0.0;
        b.hit_wall = true;
    }
    if b.pos.x + b.w > lvl.bounds.right() {
        b.pos.x = lvl.bounds.right() - b.w;
        b.vel.x = 0.0;
        b.hit_wall = true;
    }

    // ---- Y axis
    let prev_bottom = b.pos.y + b.h;
    let dy = b.vel.y * dt;
    b.pos.y += dy;
    let was_on_ground = b.on_ground;
    b.on_ground = false;
    r = b.aabb();
    for s in lvl.solids {
        if r.overlaps(s) {
            if dy > 0.0 {
                b.pos.y = s.top() - b.h;
                b.on_ground = true;
            } else if dy < 0.0 {
                b.pos.y = s.bottom();
                b.hit_ceiling = true;
            }
            b.vel.y = 0.0;
            r = b.aabb();
        }
    }
    if dy >= 0.0 && b.drop_timer <= 0.0 {
        for p in lvl.platforms {
            let new_bottom = b.pos.y + b.h;
            let horiz = b.pos.x + b.w > p.left() && b.pos.x < p.right();
            if horiz && prev_bottom <= p.top() + 0.5 && new_bottom >= p.top() {
                b.pos.y = p.top() - b.h;
                b.vel.y = 0.0;
                b.on_ground = true;
            }
        }
    }
    // Ground probe: keep "on_ground" stable when standing still (vel.y == 0).
    if !b.on_ground && dy == 0.0 && was_on_ground {
        b.on_ground = standing_on(b, lvl);
    }
    if b.pos.y < lvl.bounds.top() {
        b.pos.y = lvl.bounds.top();
        b.vel.y = 0.0;
    }
}

/// True when there is solid or platform directly beneath the body's feet.
pub fn standing_on(b: &Body, lvl: &Level) -> bool {
    let probe = Aabb::new(b.pos.x + 1.0, b.pos.y + b.h, b.w - 2.0, 1.5);
    lvl.solids.iter().any(|s| probe.overlaps(s))
        || (b.drop_timer <= 0.0
            && lvl.platforms.iter().any(|p| probe.overlaps(p) && (b.pos.y + b.h - p.top()).abs() < 1.0))
}

/// Y of the first solid/platform surface below the body's feet (for shadows).
pub fn ground_below(b: &Body, lvl: &Level) -> f32 {
    let (x0, x1) = (b.pos.x + 2.0, b.pos.x + b.w - 2.0);
    let feet = b.pos.y + b.h;
    lvl.solids
        .iter()
        .chain(lvl.platforms.iter())
        .filter(|s| s.right() > x0 && s.left() < x1 && s.top() >= feet - 1.0)
        .map(|s| s.top())
        .fold(feet + 2000.0, f32::min)
}

/// Is there ground ahead of a walking body? (Used by enemy AI to avoid ledges.)
pub fn ground_ahead(b: &Body, dir: f32, lvl: &Level) -> bool {
    let x = if dir > 0.0 { b.pos.x + b.w + 2.0 } else { b.pos.x - 4.0 };
    let probe = Aabb::new(x, b.pos.y + b.h, 2.0, 12.0);
    lvl.solids.iter().chain(lvl.platforms.iter()).any(|s| probe.overlaps(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lvl<'a>(solids: &'a [Aabb], plats: &'a [Aabb]) -> Level<'a> {
        Level { solids, platforms: plats, bounds: Aabb::new(0.0, -1000.0, 2000.0, 3000.0) }
    }

    #[test]
    fn lands_on_floor() {
        let floor = [Aabb::new(0.0, 500.0, 2000.0, 50.0)];
        let l = lvl(&floor, &[]);
        let mut b = Body::new(100.0, 300.0, 20.0, 40.0);
        for _ in 0..240 {
            apply_gravity(&mut b, 1.0 / 120.0, false, 1.0);
            move_and_collide(&mut b, &l, 1.0 / 120.0);
        }
        assert!(b.on_ground);
        assert!((b.pos.y + b.h - 500.0).abs() < 0.01);
    }

    #[test]
    fn one_way_platform() {
        let floor = [Aabb::new(0.0, 500.0, 2000.0, 50.0)];
        let plat = [Aabb::new(50.0, 400.0, 200.0, 10.0)];
        let l = lvl(&floor, &plat);
        // Jumping up through the platform from below works.
        let mut b = Body::new(100.0, 460.0, 20.0, 40.0);
        b.vel.y = -900.0;
        let mut min_y: f32 = 1e9;
        for _ in 0..120 {
            apply_gravity(&mut b, 1.0 / 120.0, false, 1.0);
            move_and_collide(&mut b, &l, 1.0 / 120.0);
            min_y = min_y.min(b.pos.y);
        }
        assert!(min_y < 360.0, "passed through from below");
        assert!(b.on_ground && (b.pos.y + b.h - 400.0).abs() < 0.01, "landed on top: {:?}", b.pos);
        // Drop through
        b.drop_timer = 0.25;
        for _ in 0..120 {
            apply_gravity(&mut b, 1.0 / 120.0, false, 1.0);
            move_and_collide(&mut b, &l, 1.0 / 120.0);
        }
        assert!((b.pos.y + b.h - 500.0).abs() < 0.01, "dropped to floor");
    }

    #[test]
    fn wall_blocks() {
        let s = [Aabb::new(0.0, 500.0, 2000.0, 50.0), Aabb::new(300.0, 300.0, 50.0, 200.0)];
        let l = lvl(&s, &[]);
        let mut b = Body::new(200.0, 460.0, 20.0, 40.0);
        b.on_ground = true;
        for _ in 0..120 {
            b.vel.x = 400.0;
            apply_gravity(&mut b, 1.0 / 120.0, false, 1.0);
            move_and_collide(&mut b, &l, 1.0 / 120.0);
        }
        assert!((b.pos.x + b.w - 300.0).abs() < 0.01);
    }
}
