//! The 0.15.0 dungeon bosses' mechanics (a child module of `enemies`).
//!
//! - Rime Colossus (Frozen Throne gate): Blizzard - keep moving or freeze
//!   (the freezing is felt by each player, see `Sim::boss_hazards`);
//!   Avalanche - snowballs roll in from both walls, jump them.
//! - Glacia, the Frost Wyrm (Frozen Throne, flying): Icicle Barrage aimed
//!   wherever you stand; Ice Shield - she's untouchable until the Frost
//!   Crystals she calls up are smashed, then she's stunned.
//! - Magmaw (Cinder Sanctum gate): Overheat - a damage race: hurt it enough
//!   while it glows and it cools down stunned, fail and it goes supernova;
//!   Eruption - lobs magma that leaves burning pools.
//! - Ignaroth, the Cinder Dragon (Cinder Sanctum): Flame Breath sweeps;
//!   Inferno sets the half of the arena it faces on fire - cross over;
//!   Wing Gust blows everyone back.
//! - The Aether Twins, Sol (ground) and Luna (flying) (Celestial Spire):
//!   Solar Flare and Lunar Rain; kill one and the other goes into Eclipse,
//!   then brings its twin back unless you finish it within the eclipse.

use super::*;

/// Blizzard: how long it blows; the frost you build standing still (per
/// second) and lose moving.
pub const BLIZZARD_SECS: f32 = 5.0;
pub const FROST_RATE: f32 = 0.55;
/// Seconds a full frost meter freezes you.
pub const FREEZE_SECS: f32 = 1.6;
/// Glacia gives up her shield (and heals) if the crystals live this long.
pub const ENCASE_SECS: f32 = 18.0;
/// Overheat: how long you have, and how much of its max HP you must deal.
pub const OVERHEAT_SECS: f32 = 6.0;
pub const OVERHEAT_COOL: f32 = 0.08;
/// Inferno: warning, then burning.
pub const INFERNO_WARN: f32 = 1.5;
pub const INFERNO_LIVE: f32 = 4.0;
/// Wing Gust push (px/s, away from the dragon).
pub const GUST_SECS: f32 = 2.4;
pub const GUST_PUSH: f32 = 280.0;
/// Eclipse: how long you have to kill the surviving twin.
pub const ECLIPSE_SECS: f32 = 10.0;
/// The revived twin comes back with this much HP.
pub const REVIVE_HP: f32 = 0.4;

/// A monster a boss calls into the world.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spawn {
    pub kind: EnemyKind,
    pub x: f32,
    /// Feet position.
    pub y: f32,
    pub hp_frac: f32,
}

fn shot(pos: V2, vel: V2, size: f32, gravity: f32, life: f32, pierce: bool, damage: f32, kb: V2, style: ProjStyle, grounded: bool) -> Shot {
    Shot { pos, vel, size, gravity, life, pierce, damage, kb, style, grounded }
}

fn ring(out: &mut Out, c: V2, n: u32, speed: f32, size: f32, damage: f32, style: ProjStyle) {
    for i in 0..n {
        let a = i as f32 / n as f32 * std::f32::consts::TAU;
        let d = V2::new(a.cos(), a.sin());
        out.shots.push(shot(c + d * 40.0, d * speed, size, 0.0, 2.0, false, damage, V2::new(220.0, -220.0), style, false));
    }
}

/// The next move for a 0.15.0 ground boss (`basic` is a charge, stomp or volley).
pub(super) fn next_move(e: &Enemy, basic: EState, enraged: bool) -> Option<EState> {
    use EnemyKind::*;
    let m = e.moves % 4;
    Some(match e.kind {
        RimeColossus => match m {
            0 => basic,
            1 => EState::Blizzard,
            2 => EState::Avalanche,
            _ if enraged => EState::Blizzard,
            _ => basic,
        },
        Magmaw => match m {
            0 => basic,
            1 => EState::Eruption,
            2 => EState::Overheat,
            _ if enraged => EState::Eruption,
            _ => basic,
        },
        Ignaroth => match m {
            0 => EState::Breath,
            1 => EState::Inferno,
            2 => basic,
            _ if enraged => EState::WingGust,
            _ => EState::Breath,
        },
        Sol => match m {
            0 | 2 => basic,
            1 => EState::Flare,
            _ if enraged => EState::Flare,
            _ => basic,
        },
        _ => return None,
    })
}

/// A 0.15.0 boss state. Returns false if `e.state` isn't one of these.
pub(super) fn special(e: &mut Enemy, d: &EnemyDef, ctx: &Ctx, out: &mut Out, enraged: bool) -> bool {
    let dt = ctx.dt;
    let c = e.body.center();
    let feet = e.body.pos.y + e.body.h;
    match e.state {
        EState::Blizzard => {
            e.body.vel.x = 0.0;
            // a few drifting ice shards, the rest is the cold itself
            e.cooldown -= dt;
            if e.cooldown <= 0.0 {
                e.cooldown = if enraged { 0.5 } else { 0.8 };
                let x = e.home.x + ((e.state_t * 977.0) as i32 % 1000) as f32 - 500.0;
                out.shots.push(shot(V2::new(x, e.home.y - 520.0), V2::new(-120.0, 220.0), 16.0, 120.0, 3.0, false, d.proj_damage * 0.6, V2::new(120.0, -160.0), ProjStyle::Icicle, false));
            }
            if e.state_t > BLIZZARD_SECS {
                e.state = EState::Recover;
                e.state_t = 0.0;
            }
        }
        EState::Avalanche => {
            e.body.vel.x = 0.0;
            // three waves of snowballs from both walls, rolling inwards
            let wave = (e.state_t / 0.9) as u32;
            if wave < 3 && e.adds != wave + 1 {
                e.adds = wave + 1;
                for s in [-1.0f32, 1.0] {
                    let x = e.home.x + s * (ARENA_HALF + 20.0);
                    out.shots.push(shot(V2::new(x, feet - 26.0), V2::new(-s * if enraged { 520.0 } else { 430.0 }, 0.0), 46.0, 0.0, 3.4, true, d.attack * 0.7, V2::new(360.0, -420.0), ProjStyle::Snowball, true));
                }
                out.shake = out.shake.max(5.0);
            }
            if e.state_t > 3.2 {
                e.adds = 0;
                e.state = EState::Recover;
                e.state_t = 0.0;
            }
        }
        EState::Eruption => {
            e.body.vel.x = 0.0;
            if e.state_t < dt * 1.5 {
                e.aim = V2::new(ctx.player.x, feet);
            }
            let xs = [-260.0f32, -90.0, 90.0, 260.0];
            if e.state_t > 0.6 && !e.attack_hit {
                e.attack_hit = true;
                for ox in xs {
                    let tx = e.aim.x + ox;
                    let t = 1.0;
                    let g = 900.0;
                    let vx = (tx - c.x) / t;
                    let vy = ((feet - 20.0 - c.y) - 0.5 * g * t * t) / t;
                    out.shots.push(shot(c, V2::new(vx, vy), 26.0, g, t + 0.05, false, d.proj_damage, V2::new(200.0, -300.0), ProjStyle::Fireball, false));
                }
                out.shake = out.shake.max(5.0);
            }
            if e.state_t > 1.6 && e.adds == 0 {
                e.adds = 1;
                for ox in xs {
                    out.shots.push(shot(V2::new(e.aim.x + ox, feet - 10.0), V2::ZERO, 70.0, 0.0, if enraged { 9.0 } else { 7.0 }, true, d.proj_damage * 0.45, V2::new(80.0, -380.0), ProjStyle::LavaPool, true));
                }
            }
            if e.state_t > 2.2 {
                e.adds = 0;
                e.state = EState::Recover;
                e.state_t = 0.0;
            }
        }
        EState::Overheat => {
            e.body.vel.x = 0.0;
            if e.state_t < dt * 1.5 {
                e.mark_hp = e.hp;
            }
            if e.mark_hp - e.hp >= e.max_hp * OVERHEAT_COOL {
                // cooled down in time
                e.state = EState::Stunned;
                e.state_t = -1.5; // a longer stun than a wall charge
                out.shake = out.shake.max(8.0);
            } else if e.state_t > OVERHEAT_SECS {
                // supernova
                ring(out, c, 28, 460.0, 34.0, d.attack * 1.4, ProjStyle::Fireball);
                ring(out, c, 14, 300.0, 34.0, d.attack * 1.1, ProjStyle::Fireball);
                out.shake = out.shake.max(16.0);
                e.state = EState::Recover;
                e.state_t = 0.0;
            }
        }
        EState::Breath => {
            // a sweeping gout of fire from the jaws
            e.body.vel.x = 0.0;
            if e.state_t > 0.5 && e.state_t < 2.0 {
                e.cooldown -= dt;
                if e.cooldown <= 0.0 {
                    e.cooldown = 0.08;
                    let k = (e.state_t - 0.5) / 1.5;
                    let a = -0.9 + k * 1.2; // from up high down to the floor
                    let dir = V2::new(a.cos() * e.facing, a.sin().abs() * if a < 0.0 { -1.0 } else { 1.0 });
                    let mouth = V2::new(c.x + e.facing * e.body.w * 0.45, c.y - 10.0);
                    out.shots.push(shot(mouth, dir * 520.0, 30.0, 0.0, 1.0, true, d.proj_damage * 0.55, V2::new(200.0, -200.0), ProjStyle::Flame, false));
                }
            }
            if e.state_t > 2.4 {
                e.state = EState::Recover;
                e.state_t = 0.0;
            }
        }
        EState::Inferno => {
            // the side it faces burns (each player feels it, see Sim::boss_hazards)
            e.body.vel.x = 0.0;
            if e.state_t > INFERNO_WARN + INFERNO_LIVE {
                e.state = EState::Recover;
                e.state_t = 0.0;
            }
        }
        EState::WingGust => {
            e.body.vel.x = 0.0;
            e.cooldown -= dt;
            if e.cooldown <= 0.0 {
                e.cooldown = 0.6;
                out.shots.push(shot(V2::new(c.x + e.facing * 60.0, c.y + 10.0), V2::new(e.facing * 380.0, 0.0), 20.0, 0.0, 1.6, false, d.proj_damage * 0.5, V2::new(300.0, -200.0), ProjStyle::Fireball, false));
            }
            if e.state_t > GUST_SECS {
                e.state = EState::Recover;
                e.state_t = 0.0;
            }
        }
        EState::Flare => {
            e.body.vel.x = 0.0;
            if e.state_t > 0.7 && !e.attack_hit {
                e.attack_hit = true;
                ring(out, c, if enraged { 18 } else { 14 }, 380.0, 22.0, d.proj_damage, ProjStyle::Star);
                out.shake = out.shake.max(9.0);
            }
            if e.state_t > 1.4 {
                e.state = EState::Recover;
                e.state_t = 0.0;
            }
        }
        EState::Eclipse => eclipse(e, out),
        _ => return false,
    }
    true
}

/// The surviving twin waits out the eclipse, then brings the other back.
fn eclipse(e: &mut Enemy, out: &mut Out) {
    e.body.vel = e.body.vel * 0.9;
    if e.state_t > ECLIPSE_SECS {
        let twin = if e.kind == EnemyKind::Sol { EnemyKind::Luna } else { EnemyKind::Sol };
        out.spawns.push(Spawn { kind: twin, x: e.home.x + if twin == EnemyKind::Luna { 160.0 } else { -160.0 }, y: e.home.y, hp_frac: REVIVE_HP });
        out.shake = out.shake.max(10.0);
        e.state = EState::Walk;
        e.state_t = 0.0;
        e.think = 0.6;
    }
}

/// Flying bosses: Glacia and Luna hover over their arena.
pub(super) fn sky_boss(e: &mut Enemy, d: &EnemyDef, ctx: &Ctx, out: &mut Out) {
    let (_, dir, _) = to_player(e, ctx);
    let enraged = e.hp < e.max_hp * 0.5;
    if enraged {
        e.boss_phase2 = true;
    }
    let dt = ctx.dt;
    let c = e.body.center();
    let hover_y = e.home.y - d.hover - e.body.h * 0.5;
    let bob = (e.state_t * 1.8).sin() * 26.0;
    let steer = |e: &mut Enemy, tx: f32, speed: f32| {
        let c = e.body.center();
        let ty = hover_y + bob;
        let to = V2::new((tx - c.x).clamp(-speed, speed), (ty - c.y).clamp(-speed, speed) * 2.0);
        e.body.vel = e.body.vel + (to - e.body.vel) * (3.0 * dt).min(1.0);
    };
    match e.state {
        EState::Spit => {
            steer(e, c.x, 40.0);
            if e.state_t > 0.5 && !e.attack_hit {
                e.attack_hit = true;
                let to = ctx.player - c;
                let base = to.y.atan2(to.x);
                for i in -2..=2 {
                    let a = base + i as f32 * 0.16;
                    let v = V2::new(a.cos(), a.sin());
                    out.shots.push(shot(c + v * 40.0, v * d.proj_speed, 20.0, 0.0, 2.4, false, d.proj_damage, V2::new(200.0, -200.0), d.proj_style, false));
                }
            }
            if e.state_t > 1.2 {
                e.state = EState::Walk;
                e.state_t = 0.0;
                e.think = 0.8;
            }
        }
        EState::Icicles => {
            steer(e, e.home.x, 60.0);
            let n = (e.state_t / 0.55) as u32;
            if n < 6 && e.adds_fired != n + 1 {
                e.adds_fired = n + 1;
                out.shots.push(shot(V2::new(ctx.player.x, e.home.y - 580.0), V2::new(0.0, 60.0), 22.0, 1100.0, 1.6, true, d.attack * 0.8, V2::new(200.0, -300.0), ProjStyle::Icicle, false));
            }
            if e.state_t > 3.8 {
                e.adds_fired = 0;
                e.state = EState::Walk;
                e.state_t = 0.0;
                e.think = 0.6;
            }
        }
        EState::Encased => {
            // untouchable until the crystals are smashed (each death counts
            // down `adds`, see `on_death`)
            steer(e, e.home.x, 60.0);
            e.cooldown -= dt;
            if e.cooldown <= 0.0 {
                e.cooldown = 3.0;
                ring(out, c, 10, 300.0, 18.0, d.proj_damage * 0.8, ProjStyle::Icicle);
            }
            if e.state_t < dt * 1.5 {
                e.adds = 3;
                for (k, ox) in [-420.0f32, 0.0, 420.0].iter().enumerate() {
                    out.spawns.push(Spawn { kind: EnemyKind::FrostCrystal, x: e.home.x + ox, y: e.home.y - if k == 1 { 230.0 } else { 120.0 }, hp_frac: 1.0 });
                }
            }
            if e.adds == 0 && e.state_t > 0.5 {
                e.state = EState::Stunned;
                e.state_t = -2.0;
                out.shake = out.shake.max(10.0);
            } else if e.state_t > ENCASE_SECS {
                e.hp = (e.hp + e.max_hp * 0.08).min(e.max_hp);
                e.adds = 0;
                e.state = EState::Walk;
                e.state_t = 0.0;
            }
        }
        EState::LunarRain => {
            steer(e, e.home.x, 60.0);
            e.cooldown -= dt;
            if e.cooldown <= 0.0 && e.state_t < 2.6 {
                e.cooldown = if enraged { 0.18 } else { 0.26 };
                let r = ((e.state_t * 7919.0) as i32).rem_euclid(1200) as f32 - 600.0;
                out.shots.push(shot(V2::new(e.home.x + r, e.home.y - 560.0), V2::new(-40.0, 380.0), 22.0, 0.0, 2.0, false, d.proj_damage, V2::new(160.0, -200.0), ProjStyle::Star, false));
            }
            if e.state_t > 3.0 {
                e.state = EState::Walk;
                e.state_t = 0.0;
                e.think = 0.6;
            }
        }
        EState::Stunned => {
            // knocked out of the sky for a moment
            e.body.vel = e.body.vel * (1.0 - 3.0 * dt).max(0.0);
            e.body.vel.y += 200.0 * dt;
            if e.state_t > 2.2 {
                e.state = EState::Walk;
                e.state_t = 0.0;
                e.think = 0.6;
            }
        }
        EState::Recover => {
            steer(e, c.x, 40.0);
            if e.state_t > 0.8 {
                e.state = EState::Walk;
                e.state_t = 0.0;
            }
        }
        EState::Eclipse => {
            steer(e, e.home.x, 30.0);
            eclipse(e, out);
        }
        _ => {
            // drift over the player, staying inside the arena
            let tx = ctx.player.x.clamp(e.home.x - ARENA_HALF + 80.0, e.home.x + ARENA_HALF - 80.0);
            steer(e, tx, d.speed);
            e.facing = dir;
            e.think -= dt;
            if e.think <= 0.0 && ctx.player_alive {
                e.state = match (e.kind, e.moves % 4) {
                    (EnemyKind::Glacia, 1) => EState::Icicles,
                    (EnemyKind::Glacia, 2) if e.hp < e.max_hp * 0.85 => EState::Encased,
                    (EnemyKind::Glacia, 3) if enraged => EState::Icicles,
                    (EnemyKind::Luna, 1) | (EnemyKind::Luna, 3) => EState::LunarRain,
                    _ => EState::Spit,
                };
                e.moves += 1;
                e.attack_hit = false;
                e.cooldown = 0.3;
                e.state_t = 0.0;
            }
        }
    }
}

/// A monster died: the bosses that care react (Glacia's crystals, the
/// Twins). Shared by the offline game and the server.
pub fn on_death(dead: EnemyKind, others: &mut [Enemy]) {
    match dead {
        EnemyKind::FrostCrystal => {
            for e in others.iter_mut().filter(|e| e.kind == EnemyKind::Glacia && e.state == EState::Encased) {
                e.adds = e.adds.saturating_sub(1);
            }
        }
        EnemyKind::Sol | EnemyKind::Luna => {
            let twin = if dead == EnemyKind::Sol { EnemyKind::Luna } else { EnemyKind::Sol };
            for e in others.iter_mut().filter(|e| e.kind == twin && e.hp > 0.0 && e.state != EState::Dying) {
                e.state = EState::Eclipse;
                e.state_t = 0.0;
            }
        }
        _ => {}
    }
}

/// Is the other Aether Twin still standing? (Victory needs both down.)
pub fn twin_alive(dead: EnemyKind, others: &[Enemy]) -> bool {
    let twin = match dead {
        EnemyKind::Sol => EnemyKind::Luna,
        EnemyKind::Luna => EnemyKind::Sol,
        _ => return false,
    };
    others.iter().any(|e| e.kind == twin && e.hp > 0.0 && e.state != EState::Dying)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::{Aabb, Rng};

    fn run(e: &mut Enemy, secs: f32, mut each: impl FnMut(&Enemy, &Out)) {
        let floor = [Aabb::new(0.0, 640.0, 8000.0, 300.0)];
        let lvl = Level { solids: &floor, platforms: &[], bounds: Aabb::new(0.0, -600.0, 8000.0, 1500.0) };
        let dt = 1.0 / 120.0;
        for i in 0..(secs / dt) as usize {
            let player = V2::new(e.home.x - 250.0 + (i as f32 * 0.02).sin() * 120.0, 600.0);
            let mut out = Out::default();
            update(e, &Ctx { level: &lvl, player, player_alive: true, dt }, &mut out);
            each(e, &out);
            let cx = e.body.center().x;
            assert!((cx - e.home.x).abs() <= ARENA_HALF + 1.0, "{:?} left its arena", e.kind);
        }
    }

    #[test]
    fn every_new_boss_uses_its_mechanics() {
        let mut rng = Rng::new(5);
        let cases: [(EnemyKind, &[EState]); 6] = [
            (EnemyKind::RimeColossus, &[EState::Blizzard, EState::Avalanche]),
            (EnemyKind::Glacia, &[EState::Icicles, EState::Encased]),
            (EnemyKind::Magmaw, &[EState::Eruption, EState::Overheat]),
            (EnemyKind::Ignaroth, &[EState::Breath, EState::Inferno]),
            (EnemyKind::Sol, &[EState::Flare]),
            (EnemyKind::Luna, &[EState::LunarRain]),
        ];
        for (kind, states) in cases {
            let mut e = Enemy::new(1, kind, 4000.0, 640.0, None, &mut rng);
            e.hp = e.max_hp * 0.8; // Glacia shields up below 85%
            let mut seen = vec![];
            let mut spawns = 0;
            let mut styles = vec![];
            run(&mut e, 90.0, |e, out| {
                if !seen.contains(&e.state) {
                    seen.push(e.state);
                }
                spawns += out.spawns.len();
                for s in &out.shots {
                    if !styles.contains(&s.style) {
                        styles.push(s.style);
                    }
                }
            });
            for st in states {
                assert!(seen.contains(st), "{:?} never used {:?} (saw {:?})", kind, st, seen);
            }
            if kind == EnemyKind::Glacia {
                assert!(spawns >= 3, "Glacia calls up her crystals");
            }
            if kind == EnemyKind::Magmaw {
                assert!(styles.contains(&ProjStyle::LavaPool));
            }
            if kind == EnemyKind::RimeColossus {
                assert!(styles.contains(&ProjStyle::Snowball));
            }
        }
    }

    #[test]
    fn glacias_shield_breaks_when_the_crystals_die() {
        let mut rng = Rng::new(2);
        let mut g = Enemy::new(1, EnemyKind::Glacia, 4000.0, 640.0, None, &mut rng);
        g.state = EState::Encased;
        g.state_t = 0.0;
        assert_eq!(damage_mult(&g, 0.0), 0.0, "untouchable while encased");
        let mut out = Out::default();
        let floor = [Aabb::new(0.0, 640.0, 8000.0, 300.0)];
        let lvl = Level { solids: &floor, platforms: &[], bounds: Aabb::new(0.0, -600.0, 8000.0, 1500.0) };
        update(&mut g, &Ctx { level: &lvl, player: V2::new(3800.0, 600.0), player_alive: true, dt: 1.0 / 120.0 }, &mut out);
        assert_eq!(out.spawns.iter().filter(|s| s.kind == EnemyKind::FrostCrystal).count(), 3);
        let mut all = vec![g];
        for _ in 0..3 {
            on_death(EnemyKind::FrostCrystal, &mut all);
        }
        for _ in 0..120 {
            let mut out = Out::default();
            update(&mut all[0], &Ctx { level: &lvl, player: V2::new(3800.0, 600.0), player_alive: true, dt: 1.0 / 120.0 }, &mut out);
        }
        assert_eq!(all[0].state, EState::Stunned, "the shield shatters");
    }

    #[test]
    fn overheat_is_a_damage_race() {
        let mut rng = Rng::new(2);
        let floor = [Aabb::new(0.0, 640.0, 8000.0, 300.0)];
        let lvl = Level { solids: &floor, platforms: &[], bounds: Aabb::new(0.0, -600.0, 8000.0, 1500.0) };
        let ctx = |p| Ctx { level: &lvl, player: p, player_alive: true, dt: 1.0 / 120.0 };
        // hit it hard enough: stunned
        let mut m = Enemy::new(1, EnemyKind::Magmaw, 4000.0, 640.0, None, &mut rng);
        m.state = EState::Overheat;
        m.state_t = 0.0;
        update(&mut m, &ctx(V2::new(3800.0, 600.0)), &mut Out::default());
        m.hp -= m.max_hp * 0.1;
        update(&mut m, &ctx(V2::new(3800.0, 600.0)), &mut Out::default());
        assert_eq!(m.state, EState::Stunned);
        // don't: supernova
        let mut m = Enemy::new(1, EnemyKind::Magmaw, 4000.0, 640.0, None, &mut rng);
        m.state = EState::Overheat;
        m.state_t = 0.0;
        let mut boom = 0;
        for _ in 0..(120.0 * (OVERHEAT_SECS + 0.2)) as usize {
            let mut out = Out::default();
            update(&mut m, &ctx(V2::new(3800.0, 600.0)), &mut out);
            boom += out.shots.len();
        }
        assert!(boom >= 28, "supernova");
    }

    #[test]
    fn the_twins_revive_each_other_unless_both_fall() {
        let mut rng = Rng::new(2);
        let floor = [Aabb::new(0.0, 640.0, 8000.0, 300.0)];
        let lvl = Level { solids: &floor, platforms: &[], bounds: Aabb::new(0.0, -600.0, 8000.0, 1500.0) };
        let sol = Enemy::new(1, EnemyKind::Sol, 4000.0, 640.0, None, &mut rng);
        let luna = Enemy::new(2, EnemyKind::Luna, 4200.0, 640.0, None, &mut rng);
        let mut all = vec![sol, luna];
        // Sol dies: Luna eclipses
        all[0].hp = 0.0;
        all[0].state = EState::Dying;
        on_death(EnemyKind::Sol, &mut all);
        assert_eq!(all[1].state, EState::Eclipse);
        assert!(twin_alive(EnemyKind::Sol, &all), "Luna still stands");
        assert_eq!(damage_mult(&all[1], 0.0), 0.5);
        // ...and brings Sol back if you're too slow
        let mut revived = vec![];
        for _ in 0..(120.0 * (ECLIPSE_SECS + 0.5)) as usize {
            let mut out = Out::default();
            update(&mut all[1], &Ctx { level: &lvl, player: V2::new(3900.0, 600.0), player_alive: true, dt: 1.0 / 120.0 }, &mut out);
            revived.extend(out.spawns);
        }
        assert_eq!(revived.len(), 1);
        assert_eq!(revived[0].kind, EnemyKind::Sol);
        assert!((revived[0].hp_frac - REVIVE_HP).abs() < 1e-6);
        // both down: no twin left
        all[1].hp = 0.0;
        all[1].state = EState::Dying;
        assert!(!twin_alive(EnemyKind::Luna, &all));
    }
}
