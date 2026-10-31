//! Class abilities. Abilities are learned automatically as you level (Novice
//! abilities stay available after a class change) and are used from the
//! hotbar (keys 1-8). Each has a Mana cost and a cooldown.

use crate::classes::{self, AttackDef, AttackKind, ClassId, ProjStyle, ProjectileDef};
use crate::stats::StatMods;

pub const HOTBAR_SLOTS: usize = 8;

/// Visual flavour (particles, flash and aura colour) for the client.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SkillFx {
    Impact,
    Steel,
    Shadow,
    Storm,
    Wind,
    Arcane,
    Nature,
    Holy,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SkillEffect {
    /// An empowered attack built on one of the class's moves (`anim`).
    /// With `proj`, `damage` goes to the projectiles instead of a melee hitbox.
    Strike {
        anim: AttackKind,
        damage: f32,
        hits: u8,
        /// Multiplies the melee hitbox size.
        reach: f32,
        both_sides: bool,
        lunge: Option<(f32, f32)>,
        kb: Option<(f32, f32)>,
        /// Extra crit chance (percentage points) for this skill.
        crit_bonus: f32,
        invuln: bool,
        proj: Option<ProjectileDef>,
    },
    Buff { mods: StatMods, secs: f32 },
    Heal { hp: f32, mp: f32 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct SkillDef {
    pub id: &'static str,
    pub name: &'static str,
    pub class: ClassId,
    pub level: u32,
    pub mp: f32,
    pub cooldown: f32,
    pub desc: &'static str,
    pub effect: SkillEffect,
    pub fx: SkillFx,
}

impl SkillDef {
    pub fn kind_label(&self) -> &'static str {
        match self.effect {
            SkillEffect::Strike { proj: Some(_), .. } => "Ranged",
            SkillEffect::Strike { .. } => "Melee",
            SkillEffect::Buff { .. } => "Buff",
            SkillEffect::Heal { .. } => "Heal",
        }
    }
}

const fn z() -> StatMods {
    StatMods {
        main: crate::stats::MainStats { str: 0, dex: 0, int: 0, luk: 0 },
        hp: 0.0,
        mp: 0.0,
        patk: 0.0,
        matk: 0.0,
        pdef: 0.0,
        mdef: 0.0,
        crit: 0.0,
        crit_dmg: 0.0,
        atk_pct: 0.0,
        pdef_pct: 0.0,
        mdef_pct: 0.0,
        speed_pct: 0.0,
    }
}

const fn strike(anim: AttackKind, damage: f32, hits: u8, reach: f32) -> SkillEffect {
    SkillEffect::Strike { anim, damage, hits, reach, both_sides: false, lunge: None, kb: None, crit_bonus: 0.0, invuln: false, proj: None }
}

const fn shot(anim: AttackKind, damage: f32, p: ProjectileDef, kb: (f32, f32)) -> SkillEffect {
    SkillEffect::Strike { anim, damage, hits: 1, reach: 1.0, both_sides: false, lunge: None, kb: Some(kb), crit_bonus: 0.0, invuln: false, proj: Some(p) }
}

const fn pr(vx: f32, vy: f32, w: f32, h: f32, life: f32, pierce: bool, count: u8, spread: f32, gravity: f32, style: ProjStyle) -> ProjectileDef {
    ProjectileDef { vx, vy, w, h, gravity, life, pierce, count, spread, style }
}

use AttackKind::*;
use ClassId::*;

pub const SKILLS: &[SkillDef] = &[
    // ---------------------------------------------------------------- Novice (everyone keeps these)
    SkillDef {
        id: "power_strike",
        name: "Power Strike",
        class: Novice,
        level: 1,
        mp: 6.0,
        cooldown: 4.0,
        desc: "A wind-up swing that hits much harder than a normal attack.",
        effect: SkillEffect::Strike { anim: HeavySide, damage: 24.0, hits: 1, reach: 1.15, both_sides: false, lunge: Some((360.0, 0.0)), kb: Some((560.0, -320.0)), crit_bonus: 0.0, invuln: false, proj: None },
        fx: SkillFx::Impact,
    },
    SkillDef {
        id: "second_wind",
        name: "Second Wind",
        class: Novice,
        level: 3,
        mp: 12.0,
        cooldown: 25.0,
        desc: "Catch your breath and recover 25% of your HP.",
        effect: SkillEffect::Heal { hp: 0.25, mp: 0.0 },
        fx: SkillFx::Holy,
    },
    SkillDef {
        id: "brace",
        name: "Brace",
        class: Novice,
        level: 6,
        mp: 10.0,
        cooldown: 30.0,
        desc: "Plant your feet: +40% P.DEF for 15 seconds.",
        effect: SkillEffect::Buff { mods: StatMods { pdef_pct: 0.4, ..z() }, secs: 15.0 },
        fx: SkillFx::Steel,
    },
    // ---------------------------------------------------------------- Vanguard
    SkillDef {
        id: "shield_slam",
        name: "Shield Slam",
        class: Vanguard,
        level: 10,
        mp: 10.0,
        cooldown: 5.0,
        desc: "Charge shield-first, launching enemies away.",
        effect: SkillEffect::Strike { anim: HeavySide, damage: 30.0, hits: 1, reach: 1.25, both_sides: false, lunge: Some((480.0, 0.0)), kb: Some((720.0, -380.0)), crit_bonus: 0.0, invuln: false, proj: None },
        fx: SkillFx::Steel,
    },
    SkillDef {
        id: "war_cry",
        name: "War Cry",
        class: Vanguard,
        level: 12,
        mp: 20.0,
        cooldown: 45.0,
        desc: "A rallying roar: +20% attack and +10% P.DEF for 30 seconds.",
        effect: SkillEffect::Buff { mods: StatMods { atk_pct: 0.2, pdef_pct: 0.1, ..z() }, secs: 30.0 },
        fx: SkillFx::Impact,
    },
    SkillDef {
        id: "whirlwind",
        name: "Whirlwind",
        class: Vanguard,
        level: 15,
        mp: 18.0,
        cooldown: 8.0,
        desc: "Spin with your blade out, hitting everything around you 4 times. You can't be hurt mid-spin.",
        effect: SkillEffect::Strike { anim: HeavyDown, damage: 11.0, hits: 4, reach: 1.3, both_sides: true, lunge: None, kb: Some((180.0, -160.0)), crit_bonus: 0.0, invuln: true, proj: None },
        fx: SkillFx::Wind,
    },
    SkillDef {
        id: "iron_bastion",
        name: "Iron Bastion",
        class: Vanguard,
        level: 20,
        mp: 25.0,
        cooldown: 60.0,
        desc: "Become a fortress: +60% P.DEF and +40% M.DEF for 20 seconds.",
        effect: SkillEffect::Buff { mods: StatMods { pdef_pct: 0.6, mdef_pct: 0.4, ..z() }, secs: 20.0 },
        fx: SkillFx::Steel,
    },
    // ---------------------------------------------------------------- Duskblade
    SkillDef {
        id: "shadow_flurry",
        name: "Shadow Flurry",
        class: Duskblade,
        level: 10,
        mp: 8.0,
        cooldown: 4.0,
        desc: "Dash forward slashing 4 times.",
        effect: SkillEffect::Strike { anim: LightSide, damage: 8.0, hits: 4, reach: 1.2, both_sides: false, lunge: Some((420.0, 0.0)), kb: Some((160.0, -120.0)), crit_bonus: 5.0, invuln: false, proj: None },
        fx: SkillFx::Shadow,
    },
    SkillDef {
        id: "fan_of_knives",
        name: "Fan of Knives",
        class: Duskblade,
        level: 12,
        mp: 12.0,
        cooldown: 6.0,
        desc: "Throw a spread of 5 knives.",
        effect: shot(HeavyNeutral, 11.0, pr(900.0, 0.0, 16.0, 6.0, 0.6, false, 5, 280.0, 0.0, ProjStyle::Knife), (240.0, -180.0)),
        fx: SkillFx::Shadow,
    },
    SkillDef {
        id: "nightveil",
        name: "Nightveil",
        class: Duskblade,
        level: 15,
        mp: 18.0,
        cooldown: 40.0,
        desc: "Wrap yourself in shadow: +25% Crit Chance and +15% move speed for 20 seconds.",
        effect: SkillEffect::Buff { mods: StatMods { crit: 25.0, speed_pct: 0.15, ..z() }, secs: 20.0 },
        fx: SkillFx::Shadow,
    },
    SkillDef {
        id: "assassinate",
        name: "Assassinate",
        class: Duskblade,
        level: 20,
        mp: 22.0,
        cooldown: 12.0,
        desc: "Vanish and reappear with a devastating stab. +50% Crit Chance.",
        effect: SkillEffect::Strike { anim: HeavySide, damage: 48.0, hits: 1, reach: 1.1, both_sides: false, lunge: Some((620.0, 0.0)), kb: Some((520.0, -300.0)), crit_bonus: 50.0, invuln: true, proj: None },
        fx: SkillFx::Shadow,
    },
    // ---------------------------------------------------------------- Stormfist
    SkillDef {
        id: "thunder_palm",
        name: "Thunder Palm",
        class: Stormfist,
        level: 10,
        mp: 8.0,
        cooldown: 4.0,
        desc: "A crackling palm strike that sends foes flying.",
        effect: SkillEffect::Strike { anim: HeavyNeutral, damage: 26.0, hits: 1, reach: 1.25, both_sides: false, lunge: Some((200.0, 0.0)), kb: Some((620.0, -460.0)), crit_bonus: 0.0, invuln: false, proj: None },
        fx: SkillFx::Storm,
    },
    SkillDef {
        id: "hundred_fists",
        name: "Hundred Fists",
        class: Stormfist,
        level: 12,
        mp: 14.0,
        cooldown: 7.0,
        desc: "A blur of 6 rapid punches.",
        effect: strike(LightNeutral, 6.5, 6, 1.35),
        fx: SkillFx::Impact,
    },
    SkillDef {
        id: "focus_chi",
        name: "Focus Chi",
        class: Stormfist,
        level: 15,
        mp: 16.0,
        cooldown: 40.0,
        desc: "Centre yourself: +15% attack and +40% Crit Damage for 25 seconds.",
        effect: SkillEffect::Buff { mods: StatMods { atk_pct: 0.15, crit_dmg: 40.0, ..z() }, secs: 25.0 },
        fx: SkillFx::Storm,
    },
    SkillDef {
        id: "typhoon_kick",
        name: "Typhoon Kick",
        class: Stormfist,
        level: 20,
        mp: 22.0,
        cooldown: 10.0,
        desc: "A spinning kick that hits all around you 3 times, then a lightning shockwave.",
        effect: SkillEffect::Strike { anim: HeavyDown, damage: 16.0, hits: 3, reach: 1.5, both_sides: true, lunge: None, kb: Some((380.0, -420.0)), crit_bonus: 0.0, invuln: true, proj: None },
        fx: SkillFx::Storm,
    },
    // ---------------------------------------------------------------- Windrunner
    SkillDef {
        id: "piercing_arrow",
        name: "Piercing Arrow",
        class: Windrunner,
        level: 10,
        mp: 8.0,
        cooldown: 4.0,
        desc: "A heavy arrow that passes through every enemy in a line.",
        effect: shot(HeavySide, 26.0, pr(1250.0, 0.0, 28.0, 6.0, 0.8, true, 1, 0.0, 0.0, ProjStyle::Arrow), (420.0, -200.0)),
        fx: SkillFx::Wind,
    },
    SkillDef {
        id: "multishot",
        name: "Multishot",
        class: Windrunner,
        level: 12,
        mp: 12.0,
        cooldown: 6.0,
        desc: "Loose 5 arrows at once in a fan.",
        effect: shot(LightSide, 12.0, pr(1050.0, 0.0, 22.0, 4.0, 0.7, false, 5, 300.0, 0.0, ProjStyle::Arrow), (220.0, -140.0)),
        fx: SkillFx::Wind,
    },
    SkillDef {
        id: "hawkeye",
        name: "Hawkeye",
        class: Windrunner,
        level: 15,
        mp: 18.0,
        cooldown: 45.0,
        desc: "Sharpen your aim: +20% Crit Chance and +10% attack for 30 seconds.",
        effect: SkillEffect::Buff { mods: StatMods { crit: 20.0, atk_pct: 0.1, ..z() }, secs: 30.0 },
        fx: SkillFx::Nature,
    },
    SkillDef {
        id: "storm_arrow",
        name: "Storm Arrow",
        class: Windrunner,
        level: 20,
        mp: 24.0,
        cooldown: 12.0,
        desc: "A huge arrow wrapped in gale-force wind. Pierces.",
        effect: shot(HeavySide, 50.0, pr(900.0, 0.0, 48.0, 22.0, 1.1, true, 1, 0.0, 0.0, ProjStyle::Bolt), (640.0, -320.0)),
        fx: SkillFx::Storm,
    },
    // ---------------------------------------------------------------- Aethermancer (magic damage)
    SkillDef {
        id: "arcane_bolt",
        name: "Arcane Bolt",
        class: Aethermancer,
        level: 10,
        mp: 10.0,
        cooldown: 2.5,
        desc: "A fast bolt of pure aether.",
        effect: shot(LightSide, 24.0, pr(980.0, 0.0, 18.0, 18.0, 0.8, false, 1, 0.0, 0.0, ProjStyle::Bolt), (300.0, -200.0)),
        fx: SkillFx::Arcane,
    },
    SkillDef {
        id: "cyclone",
        name: "Cyclone",
        class: Aethermancer,
        level: 12,
        mp: 18.0,
        cooldown: 7.0,
        desc: "Summon a whirlwind around yourself that hits 4 times.",
        effect: SkillEffect::Strike { anim: HeavyDown, damage: 12.0, hits: 4, reach: 1.6, both_sides: true, lunge: None, kb: Some((160.0, -260.0)), crit_bonus: 0.0, invuln: false, proj: None },
        fx: SkillFx::Wind,
    },
    SkillDef {
        id: "aether_ward",
        name: "Aether Ward",
        class: Aethermancer,
        level: 15,
        mp: 20.0,
        cooldown: 45.0,
        desc: "A shimmering barrier: +60% M.DEF and +30% P.DEF for 25 seconds.",
        effect: SkillEffect::Buff { mods: StatMods { mdef_pct: 0.6, pdef_pct: 0.3, ..z() }, secs: 25.0 },
        fx: SkillFx::Arcane,
    },
    SkillDef {
        id: "starfall",
        name: "Starfall",
        class: Aethermancer,
        level: 20,
        mp: 30.0,
        cooldown: 12.0,
        desc: "Call down a spray of 5 falling stars.",
        effect: shot(HeavyNeutral, 22.0, pr(520.0, -420.0, 22.0, 22.0, 1.6, true, 5, 520.0, 900.0, ProjStyle::Star), (260.0, -360.0)),
        fx: SkillFx::Arcane,
    },
];

pub fn skill(id: &str) -> Option<&'static SkillDef> {
    SKILLS.iter().find(|s| s.id == id)
}

/// Every skill a class can ever use (Novice skills included), in level order.
pub fn class_skills(class: ClassId) -> Vec<&'static SkillDef> {
    let mut v: Vec<_> = SKILLS.iter().filter(|s| s.class == ClassId::Novice || s.class == class).collect();
    v.sort_by_key(|s| (s.class != ClassId::Novice, s.level));
    v
}

pub fn knows(class: ClassId, level: u32, s: &SkillDef) -> bool {
    (s.class == ClassId::Novice || s.class == class) && level >= s.level
}

/// The attack a Strike skill performs for this class.
pub fn skill_attack(class: ClassId, s: &SkillDef) -> Option<AttackDef> {
    let SkillEffect::Strike { anim, damage, hits, reach, both_sides, lunge, kb, invuln, proj, .. } = s.effect else {
        return None;
    };
    let mut d = classes::attack_def(class, anim);
    d.name = s.name;
    d.mp_cost = 0.0;
    d.ends_on_land = false;
    d.hw *= reach;
    d.hh *= reach.sqrt();
    d.hx *= reach;
    if both_sides {
        d.both_sides = true;
        d.hx = 0.0;
        d.hw = d.hw.max(90.0 * reach);
    }
    if let Some((x, y)) = lunge {
        d.lunge = Some((x * crate::physics::PACE_H, y * crate::physics::PACE_V));
    }
    if let Some((x, y)) = kb {
        d.kbx = x * crate::physics::PACE_H;
        d.kby = y * crate::physics::PACE_V;
    }
    d.invuln = invuln;
    let hits = hits.max(1) as f32;
    d.active = d.active.max(0.07 * hits);
    if let Some(p) = proj {
        d.damage = 0.0;
        let mut p = p;
        p.vx *= 0.85;
        p.vy *= 0.85;
        p.gravity *= 0.7;
        p.life /= 0.85;
        d.projectile = Some(p);
    } else {
        d.damage = damage;
        d.projectile = None;
    }
    Some(d)
}

// -------------------------------------------------------------------- hotbar

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum HotbarSlot {
    #[default]
    Empty,
    Skill(String),
    Item(String),
}

impl HotbarSlot {
    pub fn encode(&self) -> String {
        match self {
            HotbarSlot::Empty => "-".into(),
            HotbarSlot::Skill(s) => format!("s:{}", s),
            HotbarSlot::Item(s) => format!("i:{}", s),
        }
    }
    pub fn decode(s: &str) -> HotbarSlot {
        if let Some(r) = s.strip_prefix("s:") {
            if skill(r).is_some() {
                return HotbarSlot::Skill(r.into());
            }
        } else if let Some(r) = s.strip_prefix("i:") {
            if crate::items::item(r).is_some() {
                return HotbarSlot::Item(r.into());
            }
        }
        HotbarSlot::Empty
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_class_has_four_skills_and_strikes_work() {
        for &c in crate::classes::ALL_CLASSES {
            let own: Vec<_> = SKILLS.iter().filter(|s| s.class == c).collect();
            assert_eq!(own.len(), 4, "{:?}", c);
            assert_eq!(own.iter().map(|s| s.level).min(), Some(crate::classes::CLASS_CHANGE_LEVEL));
            for s in class_skills(c) {
                if let Some(d) = skill_attack(c, s) {
                    assert!(d.damage > 0.0 || d.projectile.is_some(), "{} does nothing", s.id);
                    assert!(d.total() < 2.0, "{} too long", s.id);
                }
                assert!(s.mp > 0.0 && s.cooldown > 0.0);
            }
        }
        for (i, a) in SKILLS.iter().enumerate() {
            for b in &SKILLS[i + 1..] {
                assert_ne!(a.id, b.id);
            }
        }
    }

    #[test]
    fn hotbar_round_trips() {
        for h in [HotbarSlot::Empty, HotbarSlot::Skill("power_strike".into()), HotbarSlot::Item("hp_potion".into())] {
            assert_eq!(HotbarSlot::decode(&h.encode()), h);
        }
        assert_eq!(HotbarSlot::decode("s:nope"), HotbarSlot::Empty);
    }
}
