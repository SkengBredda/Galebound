//! Advanced classes in play (0.15.0): advancing at level 30, passive
//! abilities, turrets and companions, party-wide support abilities and the
//! Gambler's dice. A child module of `sim`, so it extends `Sim` directly.

use super::*;
use crate::jobs::{self, Job, PassiveFx};
use crate::skills::SummonKind;

/// Party members this close to a support ability (or a Mender turret) get its effect.
pub const AID_RANGE: f32 = 1000.0;
/// Seconds a combo stack lasts without another hit.
const COMBO_SECS: f32 = 2.5;
/// How far turrets and hawks look for monsters.
const SUMMON_RANGE: f32 = 620.0;
/// Radius the Oracle's orbiting stars circle at.
pub const ORBIT_RADIUS: f32 = 62.0;

/// A turret or companion (visible to its owner; online, its hits are
/// reported to the server like any other).
#[derive(Clone, Debug)]
pub struct Summon {
    /// None = yours; Some(id) = another player's (just for show: it
    /// doesn't hurt monsters or heal anyone on your side).
    pub owner: Option<u32>,
    pub kind: SummonKind,
    pub pos: V2,
    pub left: f32,
    pub total: f32,
    pub timer: f32,
    pub damage: f32,
    pub period: f32,
    pub facing: f32,
    /// Hawk: the monster it's diving at, and how far through the dive.
    pub target: Option<u32>,
    pub dive: f32,
}

/// Running state for the passive abilities.
#[derive(Clone, Debug, Default)]
pub struct PassiveState {
    pub combo: u32,
    pub combo_t: f32,
    pub dodge_t: f32,
    pub still_t: f32,
    pub last_stand_cd: f32,
    /// Monsters already hit once (First Blood).
    pub struck: Vec<u32>,
    /// Ranged attacks fired (Feral Volley).
    pub shots: u32,
    /// Per-monster cooldowns for the orbiting stars.
    pub orbit_cd: Vec<(u32, f32)>,
    /// Set while a lightning arc is resolving (no arcs from arcs).
    pub chaining: bool,
}

/// The Gambler's Jackpot: (name, buff).
fn jackpot(i: usize) -> (&'static str, StatMods) {
    match i {
        0 => ("Lucky Sevens", StatMods { atk_pct: 0.4, ..Default::default() }),
        1 => ("Royal Flush", StatMods { crit: 30.0, crit_dmg: 40.0, ..Default::default() }),
        2 => ("Fast Hands", StatMods { speed_pct: 0.25, atk_pct: 0.1, ..Default::default() }),
        _ => ("Full House", StatMods { pdef_pct: 0.6, mdef_pct: 0.6, ..Default::default() }),
    }
}

impl Sim {
    fn pfx(&self, f: impl Fn(&PassiveFx) -> bool) -> Option<PassiveFx> {
        jobs::find_fx(self.save.job, self.save.level, f)
    }

    /// Advance into an advanced class at level 30. Hands over its weapon.
    pub fn advance_class(&mut self, job: Job) -> Result<(), String> {
        if self.save.job.is_some() {
            return Err("You have already advanced.".into());
        }
        if self.save.class != job.base() {
            return Err(format!("Only a {} can become a {}.", job.base().info().name, job.name()));
        }
        if self.save.level < jobs::JOB_LEVEL {
            return Err(format!("Reach level {} first.", jobs::JOB_LEVEL));
        }
        let info = job.info();
        if !self.save.inventory.room_for(info.starter, 1) && self.save.inventory.equip[EquipSlot::Weapon.index()].is_some() {
            return Err("Make room in your bag for your old weapon first.".into());
        }
        self.save.job = Some(job);
        let slot = EquipSlot::Weapon.index();
        if let Some(old) = self.save.inventory.equip[slot].take() {
            self.save.inventory.add(&old, 1);
        }
        self.save.inventory.equip[slot] = Some(info.starter.to_string());
        self.player.attack = None;
        self.player.state = PState::Normal;
        self.recompute_stats();
        self.player.hp = self.player.max_hp;
        self.player.mp = self.player.max_mp;
        let c = self.player.body.center();
        self.burst(c, FxKind::LevelUp, 80, 380.0);
        self.float(V2::new(c.x, c.y - 70.0), format!("You are now a {}!", info.name), FloatKind::Heal);
        self.toast(&format!("Advanced: {}", info.name), &format!("You wield the {} now. New abilities at Lv 30 and 40, passives at Lv 30, 35 and 45.", info.weapon_name));
        self.learn_new_skills(0);
        for p in jobs::passives(Some(job), self.save.level) {
            self.toast("New passive!", &format!("{} - {}", p.name, p.desc));
        }
        self.quest_event(QuestEvent::Advanced);
        self.look_changed = true;
        self.wants_save = true;
        Ok(())
    }

    /// Passives that change a hit before it's rolled: (damage multiplier,
    /// extra crit chance, guaranteed crit).
    pub(super) fn passive_attack(&mut self, i: usize) -> (f32, f32, bool) {
        let (job, level) = (self.save.job, self.save.level);
        if job.is_none() {
            return (1.0, 0.0, false);
        }
        let e = &self.enemies[i];
        let (id, boss, hp_frac) = (e.id, e.def().boss, e.hp / e.max_hp.max(1.0));
        let dist = (e.body.center().x - self.player.body.center().x).abs();
        let mut mult = 1.0;
        let mut crit = 0.0;
        let mut force = false;
        let p = &self.player;
        for ps in jobs::passives(job, level) {
            match ps.fx {
                PassiveFx::Rage { below, atk } if p.hp < p.max_hp * below => mult *= 1.0 + atk,
                PassiveFx::Combo { per, .. } => mult *= 1.0 + per * self.pstate.combo as f32,
                PassiveFx::DodgeWindow { dmg, .. } if self.pstate.dodge_t > 0.0 => mult *= 1.0 + dmg,
                PassiveFx::BossBane(b) if boss => mult *= 1.0 + b,
                PassiveFx::Execute { below, dmg } if hp_frac < below => mult *= 1.0 + dmg,
                PassiveFx::LongShot { max } => mult *= 1.0 + max * ((dist - 150.0) / 550.0).clamp(0.0, 1.0),
                PassiveFx::Stillness { secs, crit: c } if self.pstate.still_t >= secs => crit += c,
                PassiveFx::FirstStrike if !self.pstate.struck.contains(&id) => force = true,
                _ => {}
            }
        }
        if let Some(PassiveFx::DoubleChance(c)) = self.pfx(|f| matches!(f, PassiveFx::DoubleChance(_))) {
            if self.rng.f() < c {
                mult *= 2.0;
                let ec = self.enemies[i].body.center();
                self.float(V2::new(ec.x, ec.y - 80.0), "Double!".into(), FloatKind::Crit);
            }
        }
        (mult, crit, force)
    }

    /// Passives that follow a hit you dealt.
    pub(super) fn passive_after_hit(&mut self, i: usize, dmg: f32, crit: bool, kb: V2) {
        if self.save.job.is_none() {
            return;
        }
        let id = self.enemies[i].id;
        if !self.pstate.struck.contains(&id) {
            self.pstate.struck.push(id);
            if self.pstate.struck.len() > 64 {
                self.pstate.struck.remove(0);
            }
        }
        let (job, level) = (self.save.job, self.save.level);
        let mut chain = false;
        for ps in jobs::passives(job, level) {
            match ps.fx {
                PassiveFx::Lifesteal(f) => {
                    let p = &mut self.player;
                    p.hp = (p.hp + dmg * f).min(p.max_hp);
                }
                PassiveFx::CritMana(m) if crit => {
                    let p = &mut self.player;
                    p.mp = (p.mp + m).min(p.max_mp);
                }
                PassiveFx::Combo { max, .. } => {
                    self.pstate.combo = (self.pstate.combo + 1).min(max);
                    self.pstate.combo_t = COMBO_SECS;
                }
                PassiveFx::CritChain if crit && !self.pstate.chaining => chain = true,
                _ => {}
            }
        }
        if chain {
            self.pstate.chaining = true;
            self.chain_lightning(i, dmg * 0.6, kb, 0.0);
            self.pstate.chaining = false;
        }
    }

    /// Passives that change a hit you take: the new damage, or None if evaded.
    pub(super) fn passive_hurt(&mut self, dmg: f32) -> Option<f32> {
        if self.save.job.is_none() {
            return Some(dmg);
        }
        let mut d = dmg;
        if let Some(PassiveFx::Evade(c)) = self.pfx(|f| matches!(f, PassiveFx::Evade(_))) {
            if self.rng.f() < c {
                let c = self.player.body.center();
                self.float(V2::new(c.x, c.y - 50.0), "Evaded!".into(), FloatKind::Info);
                self.player.invuln = self.player.invuln.max(0.3);
                return None;
            }
        }
        if let Some(PassiveFx::Guard(g)) = self.pfx(|f| matches!(f, PassiveFx::Guard(_))) {
            d *= g;
        }
        Some(d)
    }

    /// A monster you helped kill died.
    pub(super) fn passive_kill(&mut self) {
        if let Some(PassiveFx::KillCdr(s)) = self.pfx(|f| matches!(f, PassiveFx::KillCdr(_))) {
            for c in &mut self.player.cooldowns {
                c.1 -= s;
            }
        }
    }

    /// Timers and always-on passives (Last Stand, orbiting stars).
    pub(super) fn passive_tick(&mut self, dt: f32) {
        if self.save.job.is_none() {
            return;
        }
        let real = dt / physics::GAME_SPEED;
        let ps = &mut self.pstate;
        ps.combo_t -= real;
        if ps.combo_t <= 0.0 {
            ps.combo = 0;
        }
        ps.dodge_t = (ps.dodge_t - real).max(0.0);
        ps.last_stand_cd = (ps.last_stand_cd - real).max(0.0);
        for c in &mut ps.orbit_cd {
            c.1 -= real;
        }
        ps.orbit_cd.retain(|c| c.1 > 0.0);
        let p = &self.player;
        if p.state == PState::Dead {
            return;
        }
        if p.state == PState::Dodge {
            if let Some(PassiveFx::DodgeWindow { secs, .. }) = self.pfx(|f| matches!(f, PassiveFx::DodgeWindow { .. })) {
                self.pstate.dodge_t = secs;
            }
        }
        let still = p.body.on_ground && p.body.vel.x.abs() < 5.0 && matches!(p.state, PState::Normal | PState::Attack);
        self.pstate.still_t = if still { self.pstate.still_t + real } else { 0.0 };
        if let Some(PassiveFx::LastStand { below, heal, cd }) = self.pfx(|f| matches!(f, PassiveFx::LastStand { .. })) {
            let p = &mut self.player;
            if p.hp > 0.0 && p.hp < p.max_hp * below && self.pstate.last_stand_cd <= 0.0 {
                let h = (p.max_hp * heal).round();
                p.hp = (p.hp + h).min(p.max_hp);
                self.pstate.last_stand_cd = cd;
                let c = self.player.body.center();
                self.burst(c, FxKind::Heal, 30, 220.0);
                self.float(V2::new(c.x, c.y - 70.0), format!("Warlord's Resolve! +{}", h as i32), FloatKind::Heal);
            }
        }
        if let Some(PassiveFx::OrbitOrbs { count, dmg }) = self.pfx(|f| matches!(f, PassiveFx::OrbitOrbs { .. })) {
            let mult = self.player.info.power * self.player.stats.attack_mult(self.player.info.magic);
            for k in 0..count {
                let o = self.orbit_pos(k, count);
                let hb = Aabb::new(o.x - 12.0, o.y - 12.0, 24.0, 24.0);
                let hit = self.enemies.iter().position(|e| e.state != EState::Dying && e.kind != EnemyKind::TrainingDummy && e.body.aabb().overlaps(&hb) && !self.pstate.orbit_cd.iter().any(|c| c.0 == e.id));
                if let Some(i) = hit {
                    let id = self.enemies[i].id;
                    self.pstate.orbit_cd.push((id, 0.6));
                    let dir = (self.enemies[i].body.center().x - o.x).signum();
                    self.damage_enemy_crit(i, 40.0 * dmg * mult, V2::new(120.0 * dir, -120.0), 0.15, false, 0.0);
                }
            }
        }
    }

    /// Where orbiting star `k` of `n` is now.
    pub fn orbit_pos(&self, k: u32, n: u32) -> V2 {
        let c = self.player.body.center();
        let a = self.time * 2.6 + k as f32 / n.max(1) as f32 * std::f32::consts::TAU;
        V2::new(c.x + a.cos() * ORBIT_RADIUS, c.y + a.sin() * ORBIT_RADIUS * 0.7)
    }

    /// Orbiting stars around another player (Oracles of level 30+).
    pub fn other_orbit_count(&self, class_key: &str, level: u32) -> u32 {
        let (_, job) = jobs::class_and_job(class_key);
        match jobs::find_fx(job, level, |f| matches!(f, PassiveFx::OrbitOrbs { .. })) {
            Some(PassiveFx::OrbitOrbs { count, .. }) => count,
            _ => 0,
        }
    }

    /// Where orbiting star `k` of `n` is around a point.
    pub fn orbit_pos_at(&self, c: V2, k: u32, n: u32) -> V2 {
        let a = self.time * 2.6 + k as f32 / n.max(1) as f32 * std::f32::consts::TAU;
        V2::new(c.x + a.cos() * ORBIT_RADIUS, c.y + a.sin() * ORBIT_RADIUS * 0.7)
    }

    /// How many orbiting stars to draw (0 = none).
    pub fn orbit_count(&self) -> u32 {
        match self.pfx(|f| matches!(f, PassiveFx::OrbitOrbs { .. })) {
            Some(PassiveFx::OrbitOrbs { count, .. }) => count,
            _ => 0,
        }
    }

    /// Feral Volley: is this ranged attack the extra one?
    pub(super) fn extra_shot(&mut self) -> bool {
        if let Some(PassiveFx::ExtraShot(n)) = self.pfx(|f| matches!(f, PassiveFx::ExtraShot(_))) {
            self.pstate.shots += 1;
            return self.pstate.shots % n.max(1) == 0;
        }
        false
    }

    /// Heavy Ordnance: blast radius for your projectiles.
    pub(super) fn shot_blast(&self) -> f32 {
        match self.pfx(|f| matches!(f, PassiveFx::Blast(_))) {
            Some(PassiveFx::Blast(r)) => r,
            _ => 0.0,
        }
    }

    // ------------------------------------------------------------------ abilities

    /// A support ability: heal and buff yourself, and (online) your party nearby.
    pub(super) fn party_skill(&mut self, s: &'static SkillDef, mods: StatMods, secs: f32, hp: f32) {
        self.apply_aid(s, None);
        if self.online && self.party.is_some() {
            let c = self.player.body.center();
            self.outbox.push(ClientMsg::Aid { skill: s.id.to_string(), x: c.x, y: c.y });
        }
        let _ = (mods, secs, hp);
    }

    /// Apply a support ability's effect to yourself (`from` = who cast it, if not you).
    fn apply_aid(&mut self, s: &'static SkillDef, from: Option<&str>) {
        let c = self.player.body.center();
        match s.effect {
            SkillEffect::Party { mods, secs, hp } => {
                if mods != StatMods::default() {
                    let p = &mut self.player;
                    p.buffs.retain(|b| b.id != s.id);
                    p.buffs.push(Buff { id: s.id, name: s.name, mods, left: secs, total: secs, fx: s.fx });
                    self.recompute_stats();
                }
                if hp > 0.0 {
                    self.heal_frac(hp);
                }
                self.burst(c, FxKind::Heal, 30, 240.0);
            }
            SkillEffect::Summon { kind: SummonKind::Mender, damage, .. } => self.heal_frac(damage),
            _ => {}
        }
        if let Some(name) = from {
            self.float(V2::new(c.x, c.y - 84.0), format!("{} from {}", s.name, name), FloatKind::Heal);
        }
    }

    fn heal_frac(&mut self, f: f32) {
        let p = &mut self.player;
        if p.state == PState::Dead {
            return;
        }
        let h = (p.max_hp * f).round();
        let before = p.hp;
        p.hp = (p.hp + h).min(p.max_hp);
        let got = p.hp - before;
        if got >= 1.0 {
            let c = self.player.body.center();
            self.float(V2::new(c.x, c.y - 50.0), format!("+{}", got as i32), FloatKind::Heal);
        }
    }

    /// A party member's support ability reached you (online).
    pub(super) fn receive_aid(&mut self, from: &str, skill: &str) {
        if let Some(s) = skills::skill(skill) {
            self.apply_aid(s, Some(from));
        }
    }

    /// Jackpot: one random buff.
    pub(super) fn gamble(&mut self, s: &'static SkillDef) {
        let roll = (self.rng.f() * 4.0) as usize % 4;
        let (name, mods) = jackpot(roll);
        let p = &mut self.player;
        p.buffs.retain(|b| b.id != s.id);
        p.buffs.push(Buff { id: s.id, name, mods, left: 30.0, total: 30.0, fx: s.fx });
        self.recompute_stats();
        let c = self.player.body.center();
        self.burst(c, FxKind::Crit, 30, 260.0);
        self.float(V2::new(c.x, c.y - 84.0), format!("Jackpot: {}!", name), FloatKind::Crit);
    }

    /// Call up a turret or a companion (one of each kind at a time).
    pub(super) fn summon(&mut self, kind: SummonKind, secs: f32, damage: f32, period: f32) {
        let boost = match self.pfx(|f| matches!(f, PassiveFx::TurretBoost(_))) {
            Some(PassiveFx::TurretBoost(b)) if kind != SummonKind::Hawk => b,
            _ => 1.0,
        };
        self.summons.retain(|s| s.kind != kind || s.owner.is_some());
        let p = &self.player;
        let c = p.body.center();
        let pos = match kind {
            SummonKind::Hawk => V2::new(c.x - p.facing * 30.0, c.y - 70.0),
            _ => V2::new(c.x + p.facing * 50.0, p.body.pos.y + p.body.h),
        };
        self.summons.push(Summon { owner: None, kind, pos, left: secs * boost, total: secs * boost, timer: 0.4, damage, period: period / boost, facing: p.facing, target: None, dive: 0.0 });
        self.burst(pos, FxKind::LevelUp, 16, 160.0);
    }

    /// Another player called up a turret or hawk: show it here too.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn ghost_summon(&mut self, owner: u32, c: V2, facing: f32, kind: SummonKind, secs: f32, damage: f32, period: f32, job: Option<Job>) {
        let boost = if job == Some(Job::Engineer) && kind != SummonKind::Hawk { 1.35 } else { 1.0 };
        self.summons.retain(|s| !(s.owner == Some(owner) && s.kind == kind));
        let pos = match kind {
            SummonKind::Hawk => V2::new(c.x - facing * 30.0, c.y - 70.0),
            _ => V2::new(c.x + facing * 50.0, c.y + PLAYER_H * 0.5),
        };
        self.summons.push(Summon { owner: Some(owner), kind, pos, left: secs * boost, total: secs * boost, timer: 0.4, damage, period: period / boost, facing, target: None, dive: 0.0 });
    }

    pub(super) fn update_summons(&mut self, dt: f32) {
        if self.summons.is_empty() {
            return;
        }
        let real = dt / physics::GAME_SPEED;
        // other players' summons go when they leave; yours go when you fall
        let others: Vec<(u32, V2, f32)> = self.others.iter().map(|o| (o.id, V2::new(o.pos.x, o.pos.y - PLAYER_H * 0.5), if o.st.facing < 0 { -1.0 } else { 1.0 })).collect();
        let dead = self.player.state == PState::Dead;
        self.summons.retain(|s| match s.owner {
            None => !dead,
            Some(id) => others.iter().any(|o| o.0 == id),
        });
        let mult = self.player.info.power * self.player.stats.attack_mult(self.player.info.magic);
        let my_pos = self.player.body.center();
        let my_facing = self.player.facing;
        let mut i = 0;
        while i < self.summons.len() {
            let mut s = self.summons[i].clone();
            let mine = s.owner.is_none();
            let (me, owner_facing) = match s.owner {
                None => (my_pos, my_facing),
                Some(id) => others.iter().find(|o| o.0 == id).map(|o| (o.1, o.2)).unwrap_or((my_pos, my_facing)),
            };
            s.left -= real;
            s.timer -= real;
            if s.left <= 0.0 {
                self.burst(s.pos, FxKind::Spark, 10, 140.0);
                self.summons.remove(i);
                continue;
            }
            match s.kind {
                SummonKind::Turret => {
                    let eye = V2::new(s.pos.x, s.pos.y - 30.0);
                    let target = self.nearest_enemy(eye, SUMMON_RANGE);
                    if let Some(t) = target {
                        s.facing = if t.x < eye.x { -1.0 } else { 1.0 };
                        if s.timer <= 0.0 {
                            s.timer = s.period;
                            let d = (t - eye) * (1.0 / (t - eye).len().max(1.0));
                            self.projectiles.push(Projectile {
                                fx: crate::save::FxCat::Companion,
                                team: if mine { Team::Player } else { Team::Ally },
                                pos: eye,
                                vel: d * 1300.0,
                                w: 14.0,
                                h: 5.0,
                                gravity: 0.0,
                                life: 0.7,
                                max_life: 0.7,
                                pierce: false,
                                damage: if mine { s.damage * mult } else { 0.0 },
                                kb: V2::new(160.0, -120.0),
                                hitstun: 0.18,
                                style: ProjStyle::Bullet,
                                hit_ids: Vec::new(),
                                grounded: false,
                                heavy: false,
                                crit_bonus: 0.0,
                                magic: false,
                                travelled: 0.0,
                                blast: 0.0,
                                range: SUMMON_RANGE + 80.0,
                            });
                        }
                    }
                }
                SummonKind::Mender => {
                    if s.timer <= 0.0 {
                        s.timer = s.period;
                        self.zaps.push(Zap { fx: crate::save::FxCat::Companion, mine, a: V2::new(s.pos.x, s.pos.y - 40.0), b: me, life: 0.3, max_life: 0.3, kind: ZapKind::Spark, seed: (self.time * 100.0) as u32 });
                        if mine && (me - s.pos).len() < 500.0 {
                            self.heal_frac(s.damage);
                        }
                        if mine && self.online && self.party.is_some() {
                            self.outbox.push(ClientMsg::Aid { skill: "mender_turret".into(), x: s.pos.x, y: s.pos.y });
                        }
                    }
                }
                SummonKind::Hawk => {
                    // circle above you; dive at a monster nearby, then come back
                    let home = V2::new(me.x - owner_facing * 40.0, me.y - 80.0 + (self.time * 3.0).sin() * 8.0);
                    let tgt = s.target.and_then(|id| self.enemies.iter().position(|e| e.id == id && e.state != EState::Dying));
                    match tgt {
                        Some(ti) => {
                            let tc = self.enemies[ti].body.center();
                            let to = tc - s.pos;
                            s.facing = if to.x < 0.0 { -1.0 } else { 1.0 };
                            let step = 900.0 * dt;
                            if to.len() <= step + 20.0 {
                                let dir = s.facing;
                                if mine {
                                    self.damage_enemy_crit(ti, s.damage * mult, V2::new(260.0 * dir, -260.0), 0.25, true, 0.0);
                                } else {
                                    self.burst(tc, FxKind::Spark, 8, 180.0);
                                }
                                s.target = None;
                                s.timer = s.period;
                            } else {
                                s.pos = s.pos + to * (step / to.len().max(1.0));
                            }
                        }
                        None => {
                            s.target = None;
                            let to = home - s.pos;
                            s.pos = s.pos + to * (6.0 * dt).min(1.0);
                            s.facing = owner_facing;
                            if s.timer <= 0.0 {
                                if let Some(id) = self.nearest_enemy_id(me, SUMMON_RANGE) {
                                    s.target = Some(id);
                                }
                            }
                        }
                    }
                }
            }
            self.summons[i] = s;
            i += 1;
        }
    }

    fn nearest_enemy_id(&self, from: V2, range: f32) -> Option<u32> {
        self.enemies
            .iter()
            .filter(|e| e.state != EState::Dying && e.kind != EnemyKind::TrainingDummy && e.hp > 0.0)
            .map(|e| (e.id, (e.body.center() - from).len()))
            .filter(|(_, d)| *d < range)
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(id, _)| id)
    }

    fn nearest_enemy(&self, from: V2, range: f32) -> Option<V2> {
        let id = self.nearest_enemy_id(from, range)?;
        self.enemies.iter().find(|e| e.id == id).map(|e| e.body.center())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::InputState;

    fn sim_at(class: ClassId, level: u32) -> Sim {
        let mut save = CharacterSave::new("Advancer", Default::default());
        save.class = class;
        save.level = level;
        save.inventory.equip[EquipSlot::Weapon.index()] = Some(items::starter_weapon(class).to_string());
        save.map = "whistling_fields".into();
        let mut s = Sim::new(save, Settings::default(), 3);
        s.tick(&InputState::default());
        s.enemies.clear();
        s
    }

    #[test]
    fn advancing_needs_level_30_and_the_right_class() {
        let mut s = sim_at(ClassId::Vanguard, 29);
        assert!(s.advance_class(Job::Berserker).is_err());
        s.save.level = 30;
        assert!(s.advance_class(Job::Oracle).is_err(), "wrong base class");
        s.advance_class(Job::Berserker).unwrap();
        assert_eq!(s.save.job, Some(Job::Berserker));
        assert_eq!(s.save.inventory.equip[EquipSlot::Weapon.index()].as_deref(), Some("berserker_greatsword"));
        assert!(s.save.inventory.count("iron_sword") == 1, "old weapon goes to the bag");
        assert!(s.known_skills().iter().any(|k| k.id == "cataclysm_cleave"));
        assert!(!s.known_skills().iter().any(|k| k.id == "blood_frenzy"), "Lv 40 ability");
        assert!(s.advance_class(Job::Paladin).is_err(), "only once");
        // the greatsword hits harder than the sword
        let base = classes::attack_def(ClassId::Vanguard, AttackKind::HeavySide);
        let big = classes::attack_for(ClassId::Vanguard, s.save.job, AttackKind::HeavySide);
        assert!(big.damage > base.damage && big.hw > base.hw);
    }

    #[test]
    fn passives_change_combat() {
        // Paladin's Aegis: 15% less damage
        let mut s = sim_at(ClassId::Vanguard, 40);
        s.advance_class(Job::Paladin).unwrap();
        assert_eq!(s.passive_hurt(100.0), Some(85.0));
        // Berserker's Bloodlust at low HP
        let mut s = sim_at(ClassId::Vanguard, 40);
        s.advance_class(Job::Berserker).unwrap();
        s.enemies.clear();
        s.enemies.push(Enemy::new(900, EnemyKind::Mossling, s.player.body.center().x + 60.0, s.player.body.pos.y + s.player.body.h, None, &mut Rng::new(1)));
        let (m, _, _) = s.passive_attack(0);
        assert_eq!(m, 1.0);
        s.player.hp = s.player.max_hp * 0.3;
        let (m, _, _) = s.passive_attack(0);
        assert!((m - 1.25).abs() < 1e-4);
        // Ninja: first hit on each monster always crits
        let mut s = sim_at(ClassId::Duskblade, 30);
        s.advance_class(Job::UmbralNinja).unwrap();
        s.enemies.clear();
        s.enemies.push(Enemy::new(901, EnemyKind::Mossling, 0.0, 0.0, None, &mut Rng::new(1)));
        assert!(s.passive_attack(0).2);
        s.passive_after_hit(0, 10.0, true, V2::ZERO);
        assert!(!s.passive_attack(0).2);
    }

    #[test]
    fn turrets_shoot_and_menders_heal() {
        let mut s = sim_at(ClassId::Gunslinger, 40);
        s.advance_class(Job::Engineer).unwrap();
        s.enemies.clear();
        let c = s.player.body.center();
        let floor = s.player.body.pos.y + s.player.body.h;
        s.enemies.push(Enemy::new(902, EnemyKind::TrainingDummy, c.x + 2000.0, floor, None, &mut Rng::new(1)));
        s.enemies.push(Enemy::new(903, EnemyKind::Mossling, c.x + 260.0, floor, None, &mut Rng::new(1)));
        let turret = skills::skill("sentry_turret").unwrap();
        s.player.mp = s.player.max_mp;
        assert!(s.use_skill(turret));
        assert_eq!(s.summons.len(), 1);
        let shots_before = s.projectiles.len();
        for _ in 0..60 {
            s.update_summons(TICK);
        }
        assert!(s.projectiles.len() > shots_before, "the turret fires");
        let mender = skills::skill("mender_turret").unwrap();
        s.player.cooldowns.clear();
        s.player.mp = s.player.max_mp;
        s.player.state = PState::Normal;
        s.player.attack = None;
        assert!(s.use_skill(mender));
        s.player.hp = s.player.max_hp * 0.5;
        let hp = s.player.hp;
        for _ in 0..(60.0 * 2.5) as usize {
            s.update_summons(TICK);
        }
        assert!(s.player.hp > hp, "the mender heals");
        // turrets don't outlive their timer
        for _ in 0..(60 * 80) {
            s.update_summons(TICK);
        }
        assert!(s.summons.is_empty());
        let _ = InputState::default();
    }

    #[test]
    fn jackpot_and_party_skills_buff_you() {
        let mut s = sim_at(ClassId::Aethermancer, 40);
        s.advance_class(Job::Gambler).unwrap();
        s.player.mp = s.player.max_mp;
        assert!(s.use_skill(skills::skill("jackpot").unwrap()));
        assert!(s.player.buffs.iter().any(|b| b.id == "jackpot"));
        let mut s = sim_at(ClassId::Vanguard, 30);
        s.advance_class(Job::Paladin).unwrap();
        s.player.hp = s.player.max_hp * 0.5;
        s.player.mp = s.player.max_mp;
        assert!(s.use_skill(skills::skill("sanctuary").unwrap()));
        assert!(s.player.hp > s.player.max_hp * 0.8);
        assert!(s.player.buffs.iter().any(|b| b.id == "sanctuary"));
    }

    /// 0.15.1: other players' turrets and hawks show up (just for show),
    /// and the graphics options can hide them.
    #[test]
    fn other_players_summons_show_but_do_nothing() {
        use crate::net::{ActNet, PlayerInfo, ServerMsg, ACT_SKILL};
        use crate::save::{FxCat, FX_MINE, FX_OFF};
        let mut s = sim_at(ClassId::Vanguard, 30);
        let c = s.player.body.center();
        let floor = s.player.body.pos.y + s.player.body.h;
        s.enemies.push(Enemy::new(904, EnemyKind::Mossling, c.x + 200.0, floor, None, &mut Rng::new(1)));
        let hp = s.enemies[0].hp;
        s.net_apply(ServerMsg::PlayerInfo(PlayerInfo { id: 77, name: "Gizmo".into(), class: "engineer".into(), level: 40, ..Default::default() }));
        s.others[0].pos = V2::new(c.x - 100.0, floor);
        s.net_apply(ServerMsg::Act(ActNet { id: 77, kind: ACT_SKILL, skill: "sentry_turret".into(), x: c.x - 100.0, y: c.y, facing: 1 }));
        assert_eq!(s.summons.len(), 1);
        assert_eq!(s.summons[0].owner, Some(77));
        for _ in 0..240 {
            s.update_summons(TICK);
            s.update_projectiles(TICK);
        }
        assert!(s.projectiles.iter().all(|p| p.team != Team::Player), "their turret's shots aren't yours");
        assert_eq!(s.enemies[0].hp, hp, "and they don't hurt anything on your side");
        // they leave: their turret goes
        s.others.clear();
        s.update_summons(TICK);
        assert!(s.summons.is_empty());
        // the options
        assert_eq!(s.other_orbit_count("oracle", 30), 3);
        assert_eq!(s.other_orbit_count("gambler", 60), 0);
        s.settings.fx_companions = FX_MINE;
        assert!(s.settings.fx_shown(FxCat::Companion, true) && !s.settings.fx_shown(FxCat::Companion, false));
        s.settings.fx_skills = FX_OFF;
        assert!(!s.settings.fx_shown(FxCat::Skill, true));
        assert!(s.settings.fx_shown(FxCat::Weapon, false));
    }
}
