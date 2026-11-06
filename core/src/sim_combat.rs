//! 0.14.0 combat: aim assist for player projectiles, the Aethermancer's sky
//! lightning (Arcane Bolt) and Meteor, and the Stormfist's chain-lightning
//! Thunder Palm. A child module of `sim`.
//!
//! - **Cone aim.** Straight-flying player projectiles (arrows, bolts, knives,
//!   orbs) turn toward the closest monster inside a cone in front of you
//!   ([`AIM_CONE_DEG`] either side), so ranged classes can hit flyers above
//!   or below them. With nothing in the cone they fly straight as before.
//! - **Blasts.** A projectile with `blast > 0` explodes when it hits a
//!   monster or the ground and damages everything within that radius.
//! - **Zaps** and **booms** are purely visual: lightning arcs and explosion
//!   rings for the renderer.

use super::*;
use crate::classes::ProjectileDef;

/// Half-angle of the aim-assist cone, in degrees.
pub const AIM_CONE_DEG: f32 = 38.0;
/// Arcane Bolt: how far it looks for targets, and how many it strikes.
pub const SKY_RANGE: f32 = 460.0;
pub const SKY_TARGETS: usize = 4;
/// Height lightning and meteors fall from.
const SKY_HEIGHT: f32 = 430.0;
/// Thunder Palm: how far the lightning jumps, how many extra foes, and their share of the damage.
pub const CHAIN_RANGE: f32 = 280.0;
pub const CHAIN_TARGETS: usize = 3;
pub const CHAIN_DAMAGE: f32 = 0.6;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ZapKind {
    /// Lightning jumping between foes (Thunder Palm).
    Chain,
    /// A bolt from the sky (Arcane Bolt).
    Sky,
    /// Little crackles on the Stormfist's palm.
    Spark,
}

/// A lightning arc, for drawing.
#[derive(Clone, Debug)]
pub struct Zap {
    /// Kind of effect, and whether it's yours (the graphics options).
    pub fx: crate::save::FxCat,
    pub mine: bool,
    pub a: V2,
    pub b: V2,
    pub life: f32,
    pub max_life: f32,
    pub seed: u32,
    pub kind: ZapKind,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BoomKind {
    Thunder,
    Meteor,
}

/// An explosion ring, for drawing.
#[derive(Clone, Debug)]
pub struct Boom {
    pub fx: crate::save::FxCat,
    pub mine: bool,
    pub pos: V2,
    pub radius: f32,
    pub life: f32,
    pub max_life: f32,
    pub kind: BoomKind,
}

/// A blast that went off this frame.
pub(super) struct Burst {
    pub pos: V2,
    pub radius: f32,
    pub damage: f32,
    pub kb: V2,
    pub heavy: bool,
    pub crit_bonus: f32,
    pub team: Team,
    pub style: ProjStyle,
}

fn is_straight(style: ProjStyle) -> bool {
    matches!(style, ProjStyle::Arrow | ProjStyle::Bolt | ProjStyle::Knife | ProjStyle::Orb)
}

impl Sim {
    /// The point to aim at in front of `from`, if a target sits in the cone.
    pub fn cone_target(&self, from: V2, facing: f32, range: f32) -> Option<V2> {
        let cone = AIM_CONE_DEG.to_radians();
        let mut best: Option<(f32, V2)> = None;
        let mut consider = |c: V2| {
            let d = c - from;
            let ahead = d.x * facing;
            if ahead <= 4.0 {
                return;
            }
            let dist = d.len();
            if dist > range {
                return;
            }
            let ang = d.y.abs().atan2(ahead);
            if ang > cone {
                return;
            }
            // nearer and straighter ahead wins
            let score = dist * (1.0 + ang);
            if best.map(|b| score < b.0).unwrap_or(true) {
                best = Some((score, c));
            }
        };
        for e in &self.enemies {
            if e.state != EState::Dying {
                consider(e.body.center());
            }
        }
        if let Some((_, b)) = self.duel_foe() {
            consider(b.center());
        }
        best.map(|b| b.1)
    }

    /// Turn a freshly fired projectile toward the cone target.
    pub(super) fn aim_projectile(&self, p: &mut Projectile, facing: f32) {
        if !is_straight(p.style) || p.gravity != 0.0 || p.team == Team::Enemy {
            return;
        }
        let speed = p.vel.len();
        if speed < 1.0 {
            return;
        }
        if let Some(t) = self.cone_target(p.pos, facing, p.range) {
            let d = t - p.pos;
            // keep any spread between multi-shots: rotate by the same amount
            let off = p.vel.y.atan2(p.vel.x) - 0.0f32.atan2(facing);
            let ang = d.y.atan2(d.x) + off;
            p.vel = V2::new(ang.cos() * speed, ang.sin() * speed);
        }
    }

    /// Arcane Bolt / Meteor: instead of flying from your hand, they come
    /// down from the sky onto targets in front of you. Returns true if it
    /// handled the projectile.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn sky_cast(&mut self, team: Team, pd: &ProjectileDef, c: V2, facing: f32, damage: f32, kb: V2, hitstun: f32, crit_bonus: f32, magic: bool) -> bool {
        match pd.style {
            ProjStyle::Lightning => {
                // the nearest few foes in front (or a spot on the ground ahead)
                let mut targets: Vec<(f32, V2)> = self
                    .enemies
                    .iter()
                    .filter(|e| e.state != EState::Dying)
                    .map(|e| e.body.center())
                    .chain(self.duel_foe().map(|f| f.1.center()))
                    .filter(|t| (t.x - c.x) * facing > -60.0 && (*t - c).len() < SKY_RANGE)
                    .map(|t| ((t - c).len(), t))
                    .collect();
                targets.sort_by(|a, b| a.0.total_cmp(&b.0));
                let mut pts: Vec<V2> = targets.into_iter().take(SKY_TARGETS).map(|t| t.1).collect();
                if pts.is_empty() {
                    pts.push(V2::new(c.x + facing * 240.0, c.y));
                }
                for t in pts {
                    let start = V2::new(t.x, t.y - SKY_HEIGHT);
                    self.projectiles.push(Projectile {
                        fx: crate::save::FxCat::Skill,
                        team,
                        pos: start,
                        vel: V2::new(0.0, pd.vy.abs().max(2000.0)),
                        w: pd.w,
                        h: pd.h,
                        gravity: 0.0,
                        life: pd.life,
                        max_life: pd.life,
                        pierce: true,
                        damage,
                        kb,
                        hitstun,
                        style: ProjStyle::Lightning,
                        hit_ids: Vec::new(),
                        grounded: false,
                        heavy: true,
                        crit_bonus,
                        magic,
                        travelled: 0.0,
                        range: SKY_HEIGHT + 600.0,
                        blast: pd.spread.max(60.0),
                    });
                    let seed = self.rng.next_u32();
                    self.zaps.push(Zap { fx: crate::save::FxCat::Skill, mine: team == Team::Player, a: start, b: V2::new(t.x, t.y + 30.0), life: 0.22, max_life: 0.22, seed, kind: ZapKind::Sky });
                }
                // a crackle at the caster's hand
                let hand = V2::new(c.x + facing * 20.0, c.y - 30.0);
                let seed = self.rng.next_u32();
                self.zaps.push(Zap { fx: crate::save::FxCat::Skill, mine: team == Team::Player, a: hand, b: V2::new(hand.x, hand.y - 70.0), life: 0.18, max_life: 0.18, seed, kind: ZapKind::Spark });
                true
            }
            ProjStyle::Meteor => {
                let target = self
                    .enemies
                    .iter()
                    .filter(|e| e.state != EState::Dying)
                    .map(|e| e.body.center())
                    .chain(self.duel_foe().map(|f| f.1.center()))
                    .filter(|t| (t.x - c.x) * facing > 0.0 && (*t - c).len() < 560.0)
                    .min_by(|a, b| (*a - c).len().total_cmp(&(*b - c).len()))
                    .unwrap_or(V2::new(c.x + facing * 300.0, c.y + 30.0));
                let start = V2::new(target.x - facing * 260.0, target.y - SKY_HEIGHT);
                let d = target - start;
                let speed = pd.vx.abs().max(900.0);
                let vel = d * (speed / d.len().max(1.0));
                self.projectiles.push(Projectile {
                    fx: crate::save::FxCat::Skill,
                    team,
                    pos: start,
                    vel,
                    w: pd.w,
                    h: pd.h,
                    gravity: 0.0,
                    life: pd.life,
                    max_life: pd.life,
                    pierce: true,
                    damage,
                    kb,
                    hitstun,
                    style: ProjStyle::Meteor,
                    hit_ids: Vec::new(),
                    grounded: false,
                    heavy: true,
                    crit_bonus,
                    magic,
                    travelled: 0.0,
                    range: d.len() + 400.0,
                    blast: pd.spread.max(120.0),
                });
                true
            }
            _ => false,
        }
    }

    /// Blasts that went off: damage everything in range (our own), and the
    /// explosion for everyone to see.
    pub(super) fn apply_bursts(&mut self, bursts: Vec<Burst>) {
        for b in bursts {
            let kind = if b.style == ProjStyle::Meteor { BoomKind::Meteor } else { BoomKind::Thunder };
            self.booms.push(Boom { fx: if b.style == ProjStyle::Cannonball { crate::save::FxCat::Weapon } else { crate::save::FxCat::Skill }, mine: b.team == Team::Player, pos: b.pos, radius: b.radius, life: 0.45, max_life: 0.45, kind });
            let n = if kind == BoomKind::Meteor { 34 } else { 14 };
            self.burst(b.pos, if kind == BoomKind::Meteor { FxKind::Crit } else { FxKind::Spark }, n, if kind == BoomKind::Meteor { 420.0 } else { 300.0 });
            self.shake = self.shake.max(if kind == BoomKind::Meteor { 9.0 } else { 4.0 });
            if b.team != Team::Player {
                continue;
            }
            let in_range = |c: V2, half: f32| (c - b.pos).len() <= b.radius + half;
            let mut hits: Vec<(usize, f32)> = Vec::new();
            for (i, e) in self.enemies.iter().enumerate() {
                if e.state == EState::Dying {
                    continue;
                }
                let c = e.body.center();
                if in_range(c, e.body.w.max(e.body.h) * 0.5) {
                    hits.push((i, (c.x - b.pos.x).signum()));
                }
            }
            for (i, dir) in hits {
                let dir = if dir == 0.0 { 1.0 } else { dir };
                self.damage_enemy_crit(i, b.damage, V2::new(b.kb.x.abs() * dir, b.kb.y), 0.35, b.heavy, b.crit_bonus);
            }
            if let Some((foe, fb)) = self.duel_foe() {
                if in_range(fb.center(), PLAYER_H * 0.5) {
                    let dir = (fb.center().x - b.pos.x).signum();
                    self.duel_strike(foe, fb.center(), b.damage, V2::new(b.kb.x.abs() * dir, b.kb.y), b.heavy, b.crit_bonus);
                }
            }
        }
    }

    /// Thunder Palm landed on `first`: lightning leaps to the nearest few
    /// other monsters.
    pub(super) fn chain_lightning(&mut self, first: usize, base: f32, kb: V2, crit_bonus: f32) {
        let mut hit: Vec<u32> = vec![self.enemies[first].id];
        let mut from = self.enemies[first].body.center();
        for _ in 0..CHAIN_TARGETS {
            let next = self
                .enemies
                .iter()
                .enumerate()
                .filter(|(_, e)| e.state != EState::Dying && !hit.contains(&e.id) && (e.body.center() - from).len() < CHAIN_RANGE)
                .min_by(|a, b| (a.1.body.center() - from).len().total_cmp(&(b.1.body.center() - from).len()))
                .map(|(i, _)| i);
            let Some(i) = next else { break };
            let to = self.enemies[i].body.center();
            hit.push(self.enemies[i].id);
            let seed = self.rng.next_u32();
            self.zaps.push(Zap { fx: crate::save::FxCat::Skill, mine: true, a: from, b: to, life: 0.3, max_life: 0.3, seed, kind: ZapKind::Chain });
            let dir = (to.x - from.x).signum();
            self.damage_enemy_crit(i, base * CHAIN_DAMAGE, V2::new(kb.x.abs() * 0.5 * if dir == 0.0 { 1.0 } else { dir }, kb.y * 0.6), 0.45, true, crit_bonus);
            from = to;
        }
        let c = self.enemies[first].body.center();
        self.booms.push(Boom { fx: crate::save::FxCat::Skill, mine: true, pos: c, radius: 70.0, life: 0.3, max_life: 0.3, kind: BoomKind::Thunder });
    }

    /// Another player's Thunder Palm: the same leaping lightning, for show.
    pub(super) fn ghost_chain(&mut self, c: V2, facing: f32) {
        let mut from = V2::new(c.x + facing * 40.0, c.y);
        let mut hit: Vec<u32> = vec![];
        for k in 0..=CHAIN_TARGETS {
            let range = if k == 0 { 140.0 } else { CHAIN_RANGE };
            let next = self
                .enemies
                .iter()
                .filter(|e| e.state != EState::Dying && !hit.contains(&e.id) && (e.body.center() - from).len() < range)
                .min_by(|a, b| (a.body.center() - from).len().total_cmp(&(b.body.center() - from).len()))
                .map(|e| (e.id, e.body.center()));
            let Some((id, to)) = next else { break };
            hit.push(id);
            if k > 0 {
                let seed = self.rng.next_u32();
                self.zaps.push(Zap { fx: crate::save::FxCat::Skill, mine: false, a: from, b: to, life: 0.3, max_life: 0.3, seed, kind: ZapKind::Chain });
            } else {
                self.booms.push(Boom { fx: crate::save::FxCat::Skill, mine: false, pos: to, radius: 70.0, life: 0.3, max_life: 0.3, kind: BoomKind::Thunder });
            }
            from = to;
        }
    }

    /// Crackles around the Stormfist's hand while Thunder Palm winds up.
    pub(super) fn palm_sparks(&mut self) {
        let p = &self.player;
        let Some(a) = p.attack.as_ref() else { return };
        if a.skill.map(|s| s.id) != Some("thunder_palm") || a.phase() == AttackPhase::Recovery {
            return;
        }
        if self.rng.f() > 0.45 {
            return;
        }
        let c = p.body.center();
        let hand = V2::new(c.x + p.facing * 26.0, c.y - 8.0);
        let a = V2::new(hand.x + self.rng.range(-14.0, 14.0), hand.y + self.rng.range(-14.0, 14.0));
        let b = V2::new(hand.x + self.rng.range(-20.0, 20.0), hand.y + self.rng.range(-20.0, 20.0));
        let seed = self.rng.next_u32();
        self.zaps.push(Zap { fx: crate::save::FxCat::Skill, mine: true, a, b, life: 0.12, max_life: 0.12, seed, kind: ZapKind::Spark });
        self.particles.push(Particle { pos: hand, vel: V2::new(self.rng.range(-120.0, 120.0), self.rng.range(-160.0, 40.0)), life: 0.25, max_life: 0.25, kind: FxKind::Spark, size: 4.0 });
    }

    pub(super) fn update_zaps(&mut self, dt: f32) {
        for z in &mut self.zaps {
            z.life -= dt;
        }
        self.zaps.retain(|z| z.life > 0.0);
        for b in &mut self.booms {
            b.life -= dt;
        }
        self.booms.retain(|b| b.life > 0.0);
    }
}

impl Sim {
    /// The vault bosses' arena hazards, felt by this player: the
    /// Stormwarden's vortex drags you in, and its live floor shocks anyone
    /// standing on the ground (platforms are safe). Works online too, since
    /// the server sends each monster's state.
    pub(super) fn boss_hazards(&mut self) {
        self.boss_pull = 0.0;
        let real = TICK / physics::GAME_SPEED;
        self.frozen = (self.frozen - TICK).max(0.0);
        if self.player.state == PState::Dead || !crate::world::is_dungeon(self.map.key) {
            self.frost = 0.0;
            return;
        }
        self.dungeon_hazards(real);
        let pc = self.player.body.center();
        let mut zap: Option<(f32, f32)> = None;
        for e in &self.enemies {
            if e.kind != EnemyKind::Stormwarden {
                continue;
            }
            let ec = e.body.center();
            let dx = ec.x - pc.x;
            match e.state {
                EState::Vortex if dx.abs() < enemies::VORTEX_RANGE && dx.abs() > 30.0 => {
                    let strength = if e.boss_phase2 { 1.25 } else { 1.0 };
                    self.boss_pull = dx.signum() * enemies::VORTEX_PULL * strength;
                }
                _ if enemies::surge_live(e) && dx.abs() < enemies::SURGE_RANGE => zap = Some((e.def().attack * 0.45, ec.x)),
                _ => {}
            }
        }
        if let Some((dmg, x)) = zap {
            if self.on_solid_ground() {
                self.hurt_player(dmg, x, V2::new(0.0, -460.0), true);
            }
        }
    }

    /// The 0.15.0 bosses' hazards: the Rime Colossus's blizzard (keep moving
    /// or freeze), Ignaroth's inferno (the half it faces burns) and wing
    /// gust (pushes you away).
    fn dungeon_hazards(&mut self, real: f32) {
        use crate::enemies::boss_ai::{FREEZE_SECS, FROST_RATE, GUST_PUSH, INFERNO_WARN};
        let pc = self.player.body.center();
        let arena = crate::world::highlands::final_arena_x(self.map.key);
        let mut blizzard = false;
        let mut burn: Option<(f32, f32)> = None;
        for e in &self.enemies {
            let ec = e.body.center();
            let near = (ec.x - pc.x).abs() < 1100.0;
            match e.state {
                EState::Blizzard if e.kind == EnemyKind::RimeColossus && near => blizzard = true,
                EState::Inferno if e.kind == EnemyKind::Ignaroth && e.state_t > INFERNO_WARN && near => {
                    // the side it faces (relative to the arena's middle) is on fire
                    if (pc.x - arena) * e.facing > -20.0 {
                        burn = Some((e.def().attack * 0.4, ec.x));
                    }
                }
                EState::WingGust if e.kind == EnemyKind::Ignaroth && near => {
                    self.boss_pull = (pc.x - ec.x).signum() * GUST_PUSH;
                }
                _ => {}
            }
        }
        if blizzard {
            let moving = self.player.body.vel.x.abs() > 60.0 || !self.player.body.on_ground;
            self.frost = if moving { (self.frost - real * FROST_RATE * 1.5).max(0.0) } else { self.frost + real * FROST_RATE };
            if self.frost >= 1.0 && self.frozen <= 0.0 {
                self.frost = 0.0;
                self.frozen = FREEZE_SECS;
                let c = self.player.body.center();
                self.float(V2::new(c.x, c.y - 70.0), "Frozen solid!".into(), FloatKind::PlayerHurt);
                let dmg = self.player.max_hp * 0.12;
                self.hurt_player(dmg, c.x, V2::ZERO, true);
            }
        } else {
            self.frost = (self.frost - real).max(0.0);
        }
        if let Some((dmg, x)) = burn {
            self.hurt_player(dmg, x, V2::new(0.0, -380.0), true);
        }
    }

    /// Standing on the floor itself, not on a platform?
    pub fn on_solid_ground(&self) -> bool {
        let b = &self.player.body;
        if !b.on_ground {
            return false;
        }
        let solids = self.solids_with_gates();
        physics::standing_on(&Body { drop_timer: 0.0, ..b.clone() }, &Level { solids: &solids, platforms: &[], bounds: self.map.bounds() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::appearance::Appearance;
    use crate::input::Action;

    fn sim(class: ClassId) -> Sim {
        let mut save = CharacterSave::new("Aim", Appearance::default());
        save.map = "whistling_fields".into();
        save.level = 22;
        save.class = class;
        let mut s = Sim::new(save, Settings::default(), 9);
        s.tick(&InputState::default());
        s.enemies.clear();
        s.player.invuln = 99.0;
        s
    }

    fn put(s: &mut Sim, kind: EnemyKind, dx: f32, dy: f32) -> u32 {
        let f = s.player.body.feet();
        let id = 9000 + s.enemies.len() as u32;
        let mut e = enemies::Enemy::new(id, kind, f.x + dx, f.y, None, &mut s.rng);
        e.body.vel = V2::ZERO;
        // put its centre exactly dx, dy from the player's centre (flyers hover otherwise)
        let c = s.player.body.center();
        e.body.pos = V2::new(c.x + dx - e.body.w * 0.5, c.y + dy - e.body.h * 0.5);
        s.enemies.push(e);
        id
    }

    #[test]
    fn arrows_curve_toward_a_flyer_in_the_cone() {
        let mut s = sim(ClassId::Windrunner);
        s.player.facing = 1.0;
        put(&mut s, EnemyKind::Stormkite, 300.0, -170.0);
        let c = s.player.body.center();
        let mut p = Projectile {
            fx: crate::save::FxCat::Weapon,
            team: Team::Player, pos: c, vel: V2::new(900.0, 0.0), w: 10.0, h: 4.0, gravity: 0.0, life: 1.0, max_life: 1.0, pierce: false, damage: 1.0, kb: V2::ZERO,
            hitstun: 0.1, style: ProjStyle::Arrow, hit_ids: vec![], grounded: false, heavy: false, crit_bonus: 0.0, magic: false, travelled: 0.0, range: 760.0, blast: 0.0,
        };
        s.aim_projectile(&mut p, 1.0);
        assert!(p.vel.y < -200.0, "arrow should angle up toward the flyer: {:?}", p.vel);
        assert!((p.vel.len() - 900.0).abs() < 1.0, "same speed");
        // something behind you, or far outside the cone, is ignored
        s.enemies.clear();
        put(&mut s, EnemyKind::Stormkite, -300.0, -100.0);
        put(&mut s, EnemyKind::Stormkite, 60.0, -400.0);
        let mut q = p.clone();
        q.vel = V2::new(900.0, 0.0);
        s.aim_projectile(&mut q, 1.0);
        assert_eq!(q.vel, V2::new(900.0, 0.0));
    }

    #[test]
    fn arcane_bolt_strikes_from_the_sky_and_meteor_blasts_an_area() {
        let mut s = sim(ClassId::Aethermancer);
        s.player.facing = 1.0;
        for dx in [160.0, 230.0, 300.0] {
            put(&mut s, EnemyKind::Puffwool, dx, 0.0);
        }
        let hp0: Vec<f32> = s.enemies.iter().map(|e| e.hp).collect();
        assert!(s.use_skill(skills::skill("arcane_bolt").unwrap()));
        let mut struck_from_above = false;
        for _ in 0..90 {
            s.player.invuln = 99.0;
            struck_from_above |= s.projectiles.iter().any(|p| p.style == ProjStyle::Lightning && p.vel.y > 0.0 && p.vel.x == 0.0);
            s.tick(&InputState::default());
        }
        assert!(struck_from_above);
        let hurt = s.enemies.iter().zip(&hp0).filter(|(e, h)| e.hp < **h || e.state == EState::Dying).count();
        assert_eq!(hurt, 3, "every monster in range takes a bolt");
        // Meteor: one big rock, many monsters hurt by the blast
        let mut s = sim(ClassId::Aethermancer);
        s.player.facing = 1.0;
        for dx in [200.0, 260.0, 320.0] {
            put(&mut s, EnemyKind::Puffwool, dx, 0.0);
        }
        let hp0: Vec<f32> = s.enemies.iter().map(|e| e.hp).collect();
        assert_eq!(skills::skill("meteor").unwrap().name, "Meteor");
        assert!(s.use_skill(skills::skill("meteor").unwrap()));
        let mut boomed = false;
        for _ in 0..120 {
            s.player.invuln = 99.0;
            s.tick(&InputState::default());
            boomed |= s.booms.iter().any(|b| b.kind == BoomKind::Meteor);
        }
        assert!(boomed);
        let hurt = s.enemies.iter().zip(&hp0).filter(|(e, h)| e.hp < **h || e.state == EState::Dying).count();
        assert_eq!(hurt, 3, "the blast hits the whole group");
    }

    #[test]
    fn thunder_palm_chains_to_nearby_monsters() {
        let mut s = sim(ClassId::Stormfist);
        s.player.facing = 1.0;
        put(&mut s, EnemyKind::Puffwool, 40.0, 0.0);
        put(&mut s, EnemyKind::Puffwool, 200.0, 0.0);
        put(&mut s, EnemyKind::Puffwool, 360.0, 0.0);
        put(&mut s, EnemyKind::Puffwool, 1200.0, 0.0);
        for e in &mut s.enemies {
            e.max_hp = 99_999.0;
            e.hp = 99_999.0;
        }
        assert!(s.use_skill(skills::skill("thunder_palm").unwrap()));
        let mut zapped = false;
        for _ in 0..60 {
            s.player.invuln = 99.0;
            s.tick(&InputState::default());
            zapped |= s.zaps.iter().any(|z| z.kind == ZapKind::Chain);
        }
        assert!(zapped);
        let hurt: Vec<bool> = s.enemies.iter().map(|e| e.hp < 99_999.0).collect();
        assert_eq!(hurt, vec![true, true, true, false], "the palm, two jumps, and the far one is out of reach");
        let _ = Action::Light;
    }

    #[test]
    fn no_flyers_in_the_field_but_plenty_in_the_vault() {
        for k in world::all_map_keys() {
            let m = world::load_map(k).unwrap();
            let kinds: Vec<EnemyKind> = m.spawns.iter().map(|z| z.kind).chain(m.fixed_enemies.iter().map(|f| f.0)).chain(m.bosses.iter().map(|b| b.kind)).collect();
            let flyers = kinds.iter().filter(|k| k.def().ai.is_floater()).count();
            if world::is_dungeon(k) {
                assert!(flyers >= 8, "{} should be full of flyers, has {}", k, flyers);
            } else {
                assert_eq!(flyers, 0, "{} still has flying monsters", k);
            }
        }
    }

    /// The field's old flyers walk now: every spawn zone puts its monsters
    /// on solid footing, not inside a wall or below the map.
    #[test]
    fn field_monsters_land_on_their_feet() {
        let mut rng = crate::math::Rng::new(4);
        for k in world::all_map_keys() {
            let m = world::load_map(k).unwrap();
            let mut solids = m.solids.clone();
            solids.extend(m.gates.iter().map(|g| g.solid));
            let lvl = Level { solids: &solids, platforms: &m.platforms, bounds: m.bounds() };
            for z in &m.spawns {
                for x in [z.x0 + 20.0, (z.x0 + z.x1) * 0.5, z.x1 - 20.0] {
                    let mut e = enemies::Enemy::new(1, z.kind, x, z.y, None, &mut rng);
                    for _ in 0..360 {
                        let ctx = enemies::Ctx { level: &lvl, player: V2::new(-99999.0, -99999.0), player_alive: false, dt: TICK };
                        enemies::update(&mut e, &ctx, &mut enemies::Out::default());
                    }
                    assert!(e.body.pos.y + e.body.h < m.kill_y, "{:?} at {} on {} fell out of the map", z.kind, x, k);
                    assert!(!solids.iter().any(|s| { let b = e.body.aabb(); s.overlaps(&Aabb::new(b.x + 2.0, b.y + 2.0, b.w - 4.0, b.h - 4.0)) }), "{:?} at {} on {} is stuck in a wall", z.kind, x, k);
                }
            }
        }
    }

    #[test]
    fn gatekeeper_raises_its_shield_and_drops_spikes_in_its_arena() {
        let floor = [Aabb::new(0.0, 640.0, 7000.0, 300.0)];
        let lvl = Level { solids: &floor, platforms: &[], bounds: Aabb::new(0.0, -600.0, 7000.0, 1500.0) };
        let mut rng = crate::math::Rng::new(1);
        let mut e = enemies::Enemy::new(1, EnemyKind::Gatekeeper, 3000.0, 640.0, None, &mut rng);
        let (mut guard, mut spikes, mut waves) = (false, 0, 0);
        for i in 0..120 * 60 {
            // the player keeps running away to the left: the boss must stay home
            let player = V2::new(3000.0 - 300.0 - (i as f32 * 0.4).min(1500.0), 600.0);
            let mut out = enemies::Out::default();
            enemies::update(&mut e, &enemies::Ctx { level: &lvl, player, player_alive: true, dt: TICK }, &mut out);
            guard |= e.state == EState::Guard;
            for sh in &out.shots {
                spikes += (sh.style == ProjStyle::Spike) as u32;
                waves += (sh.style == ProjStyle::Shockwave && e.state == EState::Guard) as u32;
            }
            let cx = e.body.center().x;
            assert!((cx - 3000.0).abs() <= enemies::ARENA_HALF + 1.0, "left its arena: {}", cx);
        }
        assert!(guard && spikes >= 5 && waves > 0, "guard {} spikes {} waves {}", guard, spikes, waves);
        // the shield: hits from the front barely scratch it, hits from behind land
        e.state = EState::Guard;
        e.state_t = 0.0;
        e.facing = 1.0;
        let c = e.body.center();
        assert_eq!(enemies::guard_mult(&e, c.x + 100.0), enemies::GUARD_BLOCK);
        assert_eq!(enemies::guard_mult(&e, c.x - 100.0), 1.0);
        // being hit doesn't turn it round or break the guard
        enemies::apply_hit(&mut e, 10.0, V2::new(-300.0, -100.0), 0.3);
        assert_eq!(e.facing, 1.0);
        assert_eq!(e.state, EState::Guard);
    }

    #[test]
    fn stormwarden_vortex_pulls_and_its_live_floor_spares_the_ledges() {
        let mut s = sim(ClassId::Vanguard);
        s.change_map(world::VAULT_MAP, world::SW_ARENA_X - 450.0);
        for _ in 0..30 {
            s.tick(&InputState::default());
        }
        s.enemies.retain(|e| e.kind == EnemyKind::Stormwarden);
        assert_eq!(s.enemies.len(), 1);
        s.player.invuln = 99.0;
        // the vortex drags you toward it
        let x0 = s.player.body.center().x;
        for _ in 0..60 {
            s.enemies[0].state = EState::Vortex;
            s.enemies[0].state_t = 0.5;
            s.tick(&InputState::default());
        }
        let x1 = s.player.body.center().x;
        assert!(x1 > x0 + 60.0, "pulled from {} to {}", x0, x1);
        // the live floor shocks you on the ground...
        let surge = |s: &mut Sim| {
            s.enemies[0].state = EState::Surge;
            s.enemies[0].state_t = enemies::SURGE_WARN + 0.2;
            s.enemies[0].attack_hit = true;
        };
        surge(&mut s);
        s.tick(&InputState::default());
        assert!(s.on_solid_ground());
        s.player.invuln = 0.0;
        let hp = s.player.hp;
        surge(&mut s);
        s.tick(&InputState::default());
        assert!(s.player.hp < hp, "the floor should shock you");
        // ...but not up on a ledge
        let ledge = *s.map.platforms.iter().find(|p| (p.x - (world::SW_ARENA_X - 580.0)).abs() < 1.0).expect("arena ledge");
        for _ in 0..240 {
            s.player.body.pos = V2::new(ledge.x + 40.0, ledge.y - s.player.body.h);
            s.player.body.vel = V2::ZERO;
            s.player.invuln = 99.0;
            surge(&mut s);
            s.tick(&InputState::default());
        }
        assert!(s.player.body.on_ground && !s.on_solid_ground(), "stands on the ledge");
        s.player.invuln = 0.0;
        let hp = s.player.hp;
        for _ in 0..30 {
            s.player.body.pos.x = ledge.x + 40.0;
            surge(&mut s);
            s.tick(&InputState::default());
        }
        assert_eq!(s.player.hp, hp, "ledges are safe");
    }

    #[test]
    fn blizzards_freeze_the_still_and_infernos_burn_one_side() {
        use crate::enemies::boss_ai::{INFERNO_WARN, FREEZE_SECS};
        let mut s = sim(ClassId::Vanguard);
        s.save.level = 40;
        s.change_map("frozen_throne", world::GK_ARENA_X - 300.0);
        for _ in 0..20 {
            s.tick(&InputState::default());
        }
        s.enemies.retain(|e| e.kind == EnemyKind::RimeColossus);
        s.player.invuln = 99.0;
        for _ in 0..(120.0 * 2.2) as usize {
            s.enemies[0].state = EState::Blizzard;
            s.enemies[0].state_t = 0.5;
            s.tick(&InputState::default());
        }
        assert!(s.frozen > 0.0 && s.frozen <= FREEZE_SECS, "standing still in a blizzard freezes you");
        // the inferno burns the half Ignaroth faces
        let mut s = sim(ClassId::Vanguard);
        s.save.level = 50;
        s.change_map("cinder_sanctum", world::SW_ARENA_X - 400.0);
        for _ in 0..20 {
            s.tick(&InputState::default());
        }
        s.enemies.retain(|e| e.kind == EnemyKind::Ignaroth);
        let burn = |s: &mut Sim, facing: f32| {
            s.player.invuln = 0.0;
            let hp = s.player.hp;
            s.enemies[0].state = EState::Inferno;
            s.enemies[0].state_t = INFERNO_WARN + 0.5;
            s.enemies[0].facing = facing;
            s.enemies[0].body.pos.x = world::SW_ARENA_X + 300.0;
            s.boss_hazards();
            s.player.hp < hp
        };
        assert!(burn(&mut s, -1.0), "facing your side: you burn");
        s.player.hp = s.player.max_hp;
        assert!(!burn(&mut s, 1.0), "the other side is safe");
    }
}
