//! Class abilities. Abilities are learned automatically as you level (Novice
//! abilities stay available after a class change) and are used from the
//! hotbar (keys 1-8). Each has a Mana cost and a cooldown.

use crate::classes::{self, AttackDef, AttackKind, ClassId, ProjStyle, ProjectileDef};
use crate::jobs::Job;
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
    /// Heals (fraction of max HP) and buffs you and every party member near
    /// you (0.15.0).
    Party { mods: StatMods, secs: f32, hp: f32 },
    /// Calls up a turret or a companion for `secs` seconds. `damage` is per
    /// hit (a Mender turret heals that fraction of max HP instead), every
    /// `period` seconds.
    Summon { kind: SummonKind, secs: f32, damage: f32, period: f32 },
    /// Roll the dice: one random buff out of four (the Gambler's Jackpot).
    Gamble,
}

/// What a Summon ability calls up.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SummonKind {
    /// Shoots the nearest monster.
    Turret,
    /// Heals you and your party nearby.
    Mender,
    /// A spirit hawk that dives at monsters near you.
    Hawk,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SkillDef {
    pub id: &'static str,
    pub name: &'static str,
    pub class: ClassId,
    /// Only this advanced class may use it.
    pub job: Option<Job>,
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
            SkillEffect::Party { .. } => "Party",
            SkillEffect::Summon { .. } => "Summon",
            SkillEffect::Gamble => "Gamble",
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
        job: None,
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
        job: None,
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
        job: None,
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
        job: None,
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
        job: None,
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
        job: None,
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
        job: None,
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
        job: None,
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
        job: None,
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
        job: None,
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
        job: None,
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
        job: None,
        level: 10,
        mp: 8.0,
        cooldown: 4.0,
        desc: "A palm strike charged with lightning: it sends foes flying, and the lightning leaps to 3 more nearby (60% damage).",
        effect: SkillEffect::Strike { anim: LightSide, damage: 28.0, hits: 1, reach: 1.3, both_sides: false, lunge: Some((260.0, 0.0)), kb: Some((620.0, -460.0)), crit_bonus: 0.0, invuln: false, proj: None },
        fx: SkillFx::Storm,
    },
    SkillDef {
        id: "hundred_fists",
        name: "Hundred Fists",
        class: Stormfist,
        job: None,
        level: 12,
        mp: 14.0,
        cooldown: 7.0,
        desc: "A blur of 6 rapid punches, left and right.",
        effect: strike(LightNeutral, 6.5, 6, 1.35),
        fx: SkillFx::Impact,
    },
    SkillDef {
        id: "focus_chi",
        name: "Focus Chi",
        class: Stormfist,
        job: None,
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
        job: None,
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
        job: None,
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
        job: None,
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
        job: None,
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
        job: None,
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
        job: None,
        level: 10,
        mp: 10.0,
        cooldown: 2.5,
        desc: "Call lightning down from the sky onto up to 4 foes in front of you. Each bolt bursts on impact.",
        // Lightning: vy = fall speed, spread = burst radius (see sim_combat)
        effect: shot(HeavyNeutral, 22.0, pr(0.0, 2400.0, 26.0, 70.0, 0.6, true, 1, 70.0, 0.0, ProjStyle::Lightning), (220.0, -260.0)),
        fx: SkillFx::Arcane,
    },
    SkillDef {
        id: "cyclone",
        name: "Cyclone",
        class: Aethermancer,
        job: None,
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
        job: None,
        level: 15,
        mp: 20.0,
        cooldown: 45.0,
        desc: "A shimmering barrier: +60% M.DEF and +30% P.DEF for 25 seconds.",
        effect: SkillEffect::Buff { mods: StatMods { mdef_pct: 0.6, pdef_pct: 0.3, ..z() }, secs: 25.0 },
        fx: SkillFx::Arcane,
    },
    SkillDef {
        id: "meteor",
        name: "Meteor",
        class: Aethermancer,
        job: None,
        level: 20,
        mp: 32.0,
        cooldown: 12.0,
        desc: "Call down a blazing meteor onto the nearest foe in front of you. It explodes, hitting everything nearby.",
        // Meteor: vx = fall speed, spread = blast radius (see sim_combat)
        effect: shot(HeavyNeutral, 62.0, pr(1050.0, 0.0, 56.0, 56.0, 1.4, true, 1, 170.0, 0.0, ProjStyle::Meteor), (520.0, -560.0)),
        fx: SkillFx::Arcane,
    },

    // ---------------------------------------------------------------- Gunslinger (0.15.0)
    SkillDef {
        id: "ricochet",
        name: "Ricochet",
        class: Gunslinger,
        job: None,
        level: 10,
        mp: 10.0,
        cooldown: 5.0,
        desc: "A trick shot that punches through every monster in a line.",
        effect: shot(LightSide, 26.0, pr(1500.0, 0.0, 16.0, 6.0, 0.7, true, 1, 0.0, 0.0, ProjStyle::Bullet), (300.0, -180.0)),
        fx: SkillFx::Impact,
    },
    SkillDef {
        id: "acrobats_flair",
        name: "Acrobat's Flair",
        class: Gunslinger,
        job: None,
        level: 12,
        mp: 18.0,
        cooldown: 40.0,
        desc: "Showboat: +15% move speed and +8% crit chance for 30 seconds.",
        effect: SkillEffect::Buff { mods: StatMods { speed_pct: 0.15, crit: 8.0, ..z() }, secs: 30.0 },
        fx: SkillFx::Wind,
    },
    SkillDef {
        id: "bullet_dance",
        name: "Bullet Dance",
        class: Gunslinger,
        job: None,
        level: 15,
        mp: 18.0,
        cooldown: 8.0,
        desc: "Spin on the spot firing both pistols, hitting everything around you 6 times. You can't be hurt mid-spin.",
        effect: SkillEffect::Strike { anim: AirNeutral, damage: 10.0, hits: 6, reach: 1.4, both_sides: true, lunge: None, kb: Some((160.0, -140.0)), crit_bonus: 0.0, invuln: true, proj: None },
        fx: SkillFx::Impact,
    },
    SkillDef {
        id: "dead_eye",
        name: "Dead Eye",
        class: Gunslinger,
        job: None,
        level: 20,
        mp: 30.0,
        cooldown: 12.0,
        desc: "Take a breath and fire one perfect shot: huge damage and +40% crit chance.",
        effect: SkillEffect::Strike { anim: HeavySide, damage: 70.0, hits: 1, reach: 1.0, both_sides: false, lunge: None, kb: Some((520.0, -260.0)), crit_bonus: 40.0, invuln: false, proj: Some(pr(1800.0, 0.0, 22.0, 7.0, 0.8, true, 1, 0.0, 0.0, ProjStyle::Bullet)) },
        fx: SkillFx::Impact,
    },
    // ---------------------------------------------------------------- advanced classes (0.15.0)
    SkillDef {
        id: "cataclysm_cleave",
        name: "Cataclysm Cleave",
        class: Vanguard,
        job: Some(Job::Berserker),
        level: 30,
        mp: 26.0,
        cooldown: 9.0,
        desc: "Heave the greatsword through everything in front of you for massive damage.",
        effect: SkillEffect::Strike { anim: HeavySide, damage: 80.0, hits: 1, reach: 1.6, both_sides: false, lunge: Some((420.0, 0.0)), kb: Some((700.0, -420.0)), crit_bonus: 10.0, invuln: false, proj: None },
        fx: SkillFx::Impact,
    },
    SkillDef {
        id: "blood_frenzy",
        name: "Blood Frenzy",
        class: Vanguard,
        job: Some(Job::Berserker),
        level: 40,
        mp: 30.0,
        cooldown: 50.0,
        desc: "Lose yourself: +35% attack and +10% move speed, but -20% P.DEF, for 20 seconds.",
        effect: SkillEffect::Buff { mods: StatMods { atk_pct: 0.35, speed_pct: 0.1, pdef_pct: -0.2, ..z() }, secs: 20.0 },
        fx: SkillFx::Impact,
    },
    SkillDef {
        id: "sanctuary",
        name: "Sanctuary",
        class: Vanguard,
        job: Some(Job::Paladin),
        level: 30,
        mp: 35.0,
        cooldown: 45.0,
        desc: "Raise your greatshield and bless everyone nearby: heals you and your party 35% and gives +25% P.DEF and M.DEF for 15 seconds.",
        effect: SkillEffect::Party { mods: StatMods { pdef_pct: 0.25, mdef_pct: 0.25, ..z() }, secs: 15.0, hp: 0.35 },
        fx: SkillFx::Holy,
    },
    SkillDef {
        id: "judgement_slam",
        name: "Judgement Slam",
        class: Vanguard,
        job: Some(Job::Paladin),
        level: 40,
        mp: 28.0,
        cooldown: 10.0,
        desc: "Slam the greatshield down, hurling back everything around you. You can't be hurt mid-slam.",
        effect: SkillEffect::Strike { anim: HeavyDown, damage: 62.0, hits: 1, reach: 1.6, both_sides: true, lunge: None, kb: Some((500.0, -560.0)), crit_bonus: 0.0, invuln: true, proj: None },
        fx: SkillFx::Holy,
    },
    SkillDef {
        id: "chain_surge",
        name: "Chain Surge",
        class: Stormfist,
        job: Some(Job::SurgeMonk),
        level: 30,
        mp: 22.0,
        cooldown: 7.0,
        desc: "Dash forward whirling the nunchucks: 8 lightning-fast hits.",
        effect: SkillEffect::Strike { anim: LightSide, damage: 9.0, hits: 8, reach: 1.25, both_sides: false, lunge: Some((380.0, 0.0)), kb: Some((120.0, -100.0)), crit_bonus: 0.0, invuln: false, proj: None },
        fx: SkillFx::Storm,
    },
    SkillDef {
        id: "thunder_rush",
        name: "Thunder Rush",
        class: Stormfist,
        job: Some(Job::SurgeMonk),
        level: 40,
        mp: 30.0,
        cooldown: 10.0,
        desc: "Become lightning and tear through a line of monsters 5 times. You can't be hurt during the rush.",
        effect: SkillEffect::Strike { anim: HeavySide, damage: 18.0, hits: 5, reach: 1.3, both_sides: false, lunge: Some((900.0, 0.0)), kb: Some((300.0, -260.0)), crit_bonus: 0.0, invuln: true, proj: None },
        fx: SkillFx::Storm,
    },
    SkillDef {
        id: "battle_standard",
        name: "Battle Standard",
        class: Stormfist,
        job: Some(Job::Warlord),
        level: 30,
        mp: 35.0,
        cooldown: 50.0,
        desc: "Plant your war banner: you and your party nearby get +20% attack and +10% P.DEF for 25 seconds.",
        effect: SkillEffect::Party { mods: StatMods { atk_pct: 0.2, pdef_pct: 0.1, ..z() }, secs: 25.0, hp: 0.0 },
        fx: SkillFx::Steel,
    },
    SkillDef {
        id: "earthshaker",
        name: "Earthshaker Sweep",
        class: Stormfist,
        job: Some(Job::Warlord),
        level: 40,
        mp: 30.0,
        cooldown: 10.0,
        desc: "Sweep the war staff in a full circle, hitting everything around you.",
        effect: SkillEffect::Strike { anim: HeavyDown, damage: 64.0, hits: 1, reach: 1.8, both_sides: true, lunge: None, kb: Some((520.0, -600.0)), crit_bonus: 0.0, invuln: false, proj: None },
        fx: SkillFx::Impact,
    },
    SkillDef {
        id: "iaido",
        name: "Iaido",
        class: Duskblade,
        job: Some(Job::UmbralNinja),
        level: 30,
        mp: 26.0,
        cooldown: 9.0,
        desc: "Draw and cut in one motion: a dash slash with +30% crit chance. You can't be hurt during the draw.",
        effect: SkillEffect::Strike { anim: HeavySide, damage: 92.0, hits: 1, reach: 1.5, both_sides: false, lunge: Some((760.0, 0.0)), kb: Some((460.0, -300.0)), crit_bonus: 30.0, invuln: true, proj: None },
        fx: SkillFx::Shadow,
    },
    SkillDef {
        id: "shadow_clone",
        name: "Shadow Clone",
        class: Duskblade,
        job: Some(Job::UmbralNinja),
        level: 40,
        mp: 32.0,
        cooldown: 12.0,
        desc: "Split into shadows that cut through a line of monsters 6 times.",
        effect: SkillEffect::Strike { anim: LightSide, damage: 14.0, hits: 6, reach: 1.4, both_sides: false, lunge: Some((900.0, 0.0)), kb: Some((160.0, -120.0)), crit_bonus: 10.0, invuln: true, proj: None },
        fx: SkillFx::Shadow,
    },
    SkillDef {
        id: "chain_reap",
        name: "Chain Reap",
        class: Duskblade,
        job: Some(Job::SoulReaper),
        level: 30,
        mp: 22.0,
        cooldown: 7.0,
        desc: "Fling the sickle out on its chain and drag monsters back to you, hitting 3 times.",
        effect: SkillEffect::Strike { anim: LightSide, damage: 20.0, hits: 3, reach: 2.2, both_sides: false, lunge: None, kb: Some((-460.0, -160.0)), crit_bonus: 0.0, invuln: false, proj: None },
        fx: SkillFx::Shadow,
    },
    SkillDef {
        id: "death_spiral",
        name: "Death Spiral",
        class: Duskblade,
        job: Some(Job::SoulReaper),
        level: 40,
        mp: 32.0,
        cooldown: 11.0,
        desc: "Whirl the kusarigama around you, hitting everything nearby 6 times. You can't be hurt mid-spiral.",
        effect: SkillEffect::Strike { anim: AirNeutral, damage: 15.0, hits: 6, reach: 1.6, both_sides: true, lunge: Some((0.0, -300.0)), kb: Some((160.0, -160.0)), crit_bonus: 0.0, invuln: true, proj: None },
        fx: SkillFx::Shadow,
    },
    SkillDef {
        id: "silver_volley",
        name: "Silver Volley",
        class: Windrunner,
        job: Some(Job::WitchHunter),
        level: 30,
        mp: 24.0,
        cooldown: 7.0,
        desc: "Fire five silver bolts in a fan. Each one pierces.",
        effect: SkillEffect::Strike { anim: HeavySide, damage: 22.0, hits: 1, reach: 1.0, both_sides: false, lunge: None, kb: Some((300.0, -200.0)), crit_bonus: 0.0, invuln: false, proj: Some(pr(1700.0, 0.0, 30.0, 6.0, 0.8, true, 5, 200.0, 0.0, ProjStyle::Arrow)) },
        fx: SkillFx::Holy,
    },
    SkillDef {
        id: "hex_trap",
        name: "Hex Trap",
        class: Windrunner,
        job: Some(Job::WitchHunter),
        level: 40,
        mp: 26.0,
        cooldown: 12.0,
        desc: "Toss three hex traps that hurt and launch any monster that walks over them.",
        effect: SkillEffect::Strike { anim: HeavyDown, damage: 30.0, hits: 1, reach: 1.0, both_sides: false, lunge: None, kb: Some((200.0, -460.0)), crit_bonus: 0.0, invuln: false, proj: Some(pr(160.0, -260.0, 16.0, 12.0, 4.0, true, 3, 0.0, 0.0, ProjStyle::Caltrop)) },
        fx: SkillFx::Arcane,
    },
    SkillDef {
        id: "spirit_hawk",
        name: "Spirit Hawk",
        class: Windrunner,
        job: Some(Job::BeastMaster),
        level: 30,
        mp: 30.0,
        cooldown: 30.0,
        desc: "Call a spirit hawk for 25 seconds. It dives at monsters near you.",
        effect: SkillEffect::Summon { kind: SummonKind::Hawk, secs: 25.0, damage: 40.0, period: 1.2 },
        fx: SkillFx::Nature,
    },
    SkillDef {
        id: "stampede_arrow",
        name: "Stampede Arrow",
        class: Windrunner,
        job: Some(Job::BeastMaster),
        level: 40,
        mp: 30.0,
        cooldown: 10.0,
        desc: "A giant arrow wrapped in a stampede of spirit beasts. Pierces everything.",
        effect: SkillEffect::Strike { anim: HeavySide, damage: 78.0, hits: 1, reach: 1.0, both_sides: false, lunge: None, kb: Some((700.0, -320.0)), crit_bonus: 0.0, invuln: false, proj: Some(pr(1500.0, 0.0, 60.0, 20.0, 1.0, true, 1, 0.0, 0.0, ProjStyle::Arrow)) },
        fx: SkillFx::Nature,
    },
    SkillDef {
        id: "starlight_blessing",
        name: "Starlight Blessing",
        class: Aethermancer,
        job: Some(Job::Oracle),
        level: 30,
        mp: 40.0,
        cooldown: 40.0,
        desc: "Call down starlight: heals you and your party nearby 40% and gives +15% attack for 20 seconds.",
        effect: SkillEffect::Party { mods: StatMods { atk_pct: 0.15, ..z() }, secs: 20.0, hp: 0.4 },
        fx: SkillFx::Holy,
    },
    SkillDef {
        id: "cosmic_convergence",
        name: "Cosmic Convergence",
        class: Aethermancer,
        job: Some(Job::Oracle),
        level: 40,
        mp: 36.0,
        cooldown: 10.0,
        desc: "Hurl three cosmic orbs that drift forward, burning through everything.",
        effect: SkillEffect::Strike { anim: HeavyNeutral, damage: 40.0, hits: 1, reach: 1.0, both_sides: false, lunge: None, kb: Some((320.0, -380.0)), crit_bonus: 0.0, invuln: false, proj: Some(pr(420.0, 0.0, 50.0, 50.0, 1.6, true, 3, 240.0, 0.0, ProjStyle::Orb)) },
        fx: SkillFx::Arcane,
    },
    SkillDef {
        id: "wild_card",
        name: "Wild Card",
        class: Aethermancer,
        job: Some(Job::Gambler),
        level: 30,
        mp: 20.0,
        cooldown: 5.0,
        desc: "Throw a random card: anywhere from half to four times the damage. Pierces.",
        effect: SkillEffect::Strike { anim: LightSide, damage: 34.0, hits: 1, reach: 1.0, both_sides: false, lunge: None, kb: Some((320.0, -200.0)), crit_bonus: 0.0, invuln: false, proj: Some(pr(1400.0, 0.0, 20.0, 14.0, 0.7, true, 1, 0.0, 0.0, ProjStyle::Card)) },
        fx: SkillFx::Arcane,
    },
    SkillDef {
        id: "jackpot",
        name: "Jackpot",
        class: Aethermancer,
        job: Some(Job::Gambler),
        level: 40,
        mp: 30.0,
        cooldown: 35.0,
        desc: "Roll the dice for one of four big buffs (30 seconds): attack, crits, speed or a fortress of defence.",
        effect: SkillEffect::Gamble,
        fx: SkillFx::Arcane,
    },
    SkillDef {
        id: "sentry_turret",
        name: "Sentry Turret",
        class: Gunslinger,
        job: Some(Job::Engineer),
        level: 30,
        mp: 30.0,
        cooldown: 20.0,
        desc: "Deploy a turret for 20 seconds that shoots the nearest monster.",
        effect: SkillEffect::Summon { kind: SummonKind::Turret, secs: 20.0, damage: 22.0, period: 0.7 },
        fx: SkillFx::Steel,
    },
    SkillDef {
        id: "mender_turret",
        name: "Mender Turret",
        class: Gunslinger,
        job: Some(Job::Engineer),
        level: 40,
        mp: 36.0,
        cooldown: 35.0,
        desc: "Deploy a repair turret for 18 seconds: every 2 seconds it heals you and your party nearby 6% HP.",
        effect: SkillEffect::Summon { kind: SummonKind::Mender, secs: 18.0, damage: 0.06, period: 2.0 },
        fx: SkillFx::Holy,
    },
    SkillDef {
        id: "headshot",
        name: "Headshot",
        class: Gunslinger,
        job: Some(Job::Sniper),
        level: 30,
        mp: 26.0,
        cooldown: 9.0,
        desc: "A charged rifle shot with +50% crit chance. Pierces.",
        effect: SkillEffect::Strike { anim: HeavySide, damage: 110.0, hits: 1, reach: 1.0, both_sides: false, lunge: None, kb: Some((520.0, -240.0)), crit_bonus: 50.0, invuln: false, proj: Some(pr(2400.0, 0.0, 20.0, 6.0, 0.9, true, 1, 0.0, 0.0, ProjStyle::Bullet)) },
        fx: SkillFx::Impact,
    },
    SkillDef {
        id: "suppressing_fire",
        name: "Suppressing Fire",
        class: Gunslinger,
        job: Some(Job::Sniper),
        level: 40,
        mp: 30.0,
        cooldown: 10.0,
        desc: "Rake the area with six piercing rounds.",
        effect: SkillEffect::Strike { anim: HeavyNeutral, damage: 24.0, hits: 1, reach: 1.0, both_sides: false, lunge: None, kb: Some((220.0, -120.0)), crit_bonus: 0.0, invuln: false, proj: Some(pr(2000.0, 0.0, 16.0, 6.0, 0.8, true, 6, 180.0, 0.0, ProjStyle::Bullet)) },
        fx: SkillFx::Impact,
    },
];

pub fn skill(id: &str) -> Option<&'static SkillDef> {
    SKILLS.iter().find(|s| s.id == id)
}

/// Every skill a class can ever use (Novice skills included), in level
/// order. Advanced-class skills only show for that advanced class.
pub fn class_skills(class: ClassId) -> Vec<&'static SkillDef> {
    job_skills(class, None)
}

/// Every skill a class and advanced class can use, in level order.
pub fn job_skills(class: ClassId, job: Option<Job>) -> Vec<&'static SkillDef> {
    let mut v: Vec<_> = SKILLS.iter().filter(|s| (s.class == ClassId::Novice || s.class == class) && (s.job.is_none() || s.job == job)).collect();
    v.sort_by_key(|s| (s.class != ClassId::Novice, s.level));
    v
}

pub fn knows(class: ClassId, level: u32, s: &SkillDef) -> bool {
    knows_job(class, None, level, s)
}

pub fn knows_job(class: ClassId, job: Option<Job>, level: u32, s: &SkillDef) -> bool {
    (s.class == ClassId::Novice || s.class == class) && level >= s.level && (s.job.is_none() || s.job == job)
}

/// The attack a Strike skill performs for this class.
pub fn skill_attack(class: ClassId, s: &SkillDef) -> Option<AttackDef> {
    skill_attack_job(class, None, s)
}

/// The attack a Strike skill performs for this class and advanced class.
pub fn skill_attack_job(class: ClassId, job: Option<Job>, s: &SkillDef) -> Option<AttackDef> {
    let SkillEffect::Strike { anim, damage, hits, reach, both_sides, lunge, kb, invuln, proj, .. } = s.effect else {
        return None;
    };
    let mut d = classes::attack_def(class, anim);
    if let Some(j) = job {
        // the advanced class's weapon speed applies to abilities too
        let m = j.info().moves;
        d.startup *= m.time;
        d.active *= m.time;
        d.recovery *= m.time;
    }
    if proj.is_none() && (d.hw <= 0.0 || d.hh <= 0.0) {
        // Ranged classes' animations for this slot only fire projectiles;
        // a melee ability still needs a real hitbox in front of the caster.
        d.hx = 34.0;
        d.hy = -4.0;
        d.hw = 74.0;
        d.hh = 60.0;
        d.hitstun = d.hitstun.max(0.3);
    }
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
            let own: Vec<_> = SKILLS.iter().filter(|s| s.class == c && s.job.is_none()).collect();
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
        for &j in crate::jobs::ALL_JOBS {
            let own: Vec<_> = SKILLS.iter().filter(|s| s.job == Some(j)).collect();
            assert_eq!(own.len(), 2, "{:?} has 2 active abilities", j);
            assert!(own.iter().all(|s| s.class == j.base() && s.level >= crate::jobs::JOB_LEVEL));
            for s in &own {
                if let Some(d) = skill_attack_job(j.base(), Some(j), s) {
                    assert!(d.damage > 0.0 || d.projectile.is_some(), "{} does nothing", s.id);
                    assert!(d.total() < 2.2, "{} too long", s.id);
                }
                assert!(knows_job(j.base(), Some(j), 60, s));
                assert!(!knows_job(j.base(), None, 60, s), "{} needs the advanced class", s.id);
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
