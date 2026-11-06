//! Second class advancement (0.15.0). At level 30 each class can advance
//! into one of two advanced classes ("jobs"). An advanced class keeps its
//! base class's moveset and abilities, wields a new weapon type (which
//! reshapes the moveset a little), learns 2 new active abilities (Lv 30 and
//! Lv 40) and 3 passive abilities (Lv 30, 35 and 45).

use crate::classes::{AttackDef, AttackKind, ClassId, ProjStyle, Weapon};
use crate::stats::{MainStats, StatMods};

/// Level at which a class may advance.
pub const JOB_LEVEL: u32 = 30;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Job {
    Berserker,
    Paladin,
    SurgeMonk,
    Warlord,
    UmbralNinja,
    SoulReaper,
    WitchHunter,
    BeastMaster,
    Oracle,
    Gambler,
    Engineer,
    Sniper,
}

pub const ALL_JOBS: &[Job] = &[
    Job::Berserker,
    Job::Paladin,
    Job::SurgeMonk,
    Job::Warlord,
    Job::UmbralNinja,
    Job::SoulReaper,
    Job::WitchHunter,
    Job::BeastMaster,
    Job::Oracle,
    Job::Gambler,
    Job::Engineer,
    Job::Sniper,
];

/// How an advanced class's weapon reshapes the base moveset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MoveMod {
    /// Damage multiplier for normal attacks.
    pub damage: f32,
    /// Attack windows are multiplied by this (lower = faster).
    pub time: f32,
    /// Melee hitboxes are this much bigger.
    pub reach: f32,
    /// Projectile speed and lifetime multiplier.
    pub proj_speed: f32,
    /// Projectiles take on this look (None keeps the class's own).
    pub proj_style: Option<ProjStyle>,
    /// Projectiles pierce.
    pub pierce: bool,
}

const fn mm(damage: f32, time: f32, reach: f32, proj_speed: f32, proj_style: Option<ProjStyle>, pierce: bool) -> MoveMod {
    MoveMod { damage, time, reach, proj_speed, proj_style, pierce }
}

pub struct JobInfo {
    pub key: &'static str,
    pub name: &'static str,
    pub base: ClassId,
    pub weapon: Weapon,
    pub weapon_name: &'static str,
    pub blurb: &'static str,
    /// Dialogue choice label: "Become a Berserker".
    pub choice: &'static str,
    /// The weapon handed over on advancement.
    pub starter: &'static str,
    pub moves: MoveMod,
}

impl Job {
    pub fn key(self) -> &'static str {
        self.info().key
    }
    pub fn from_key(s: &str) -> Option<Job> {
        ALL_JOBS.iter().copied().find(|j| j.key() == s)
    }
    pub fn name(self) -> &'static str {
        self.info().name
    }
    pub fn base(self) -> ClassId {
        self.info().base
    }
    pub fn info(self) -> JobInfo {
        use Job::*;
        use ProjStyle as P;
        match self {
            Berserker => JobInfo {
                key: "berserker",
                name: "Berserker",
                base: ClassId::Vanguard,
                weapon: Weapon::Greatsword,
                weapon_name: "Two-Handed Greatsword",
                blurb: "Throws the shield away for a greatsword. Huge, slow swings that grow deadlier the more you bleed.",
                choice: "Become a Berserker",
                starter: "berserker_greatsword",
                moves: mm(1.32, 1.12, 1.3, 1.0, None, false),
            },
            Paladin => JobInfo {
                key: "paladin",
                name: "Paladin",
                base: ClassId::Vanguard,
                weapon: Weapon::Greatshield,
                weapon_name: "Greatshield",
                blurb: "A wall of holy steel. Draws monsters onto itself and shields and heals the whole party.",
                choice: "Become a Paladin",
                starter: "paladin_greatshield",
                moves: mm(1.0, 1.0, 1.15, 1.0, None, false),
            },
            SurgeMonk => JobInfo {
                key: "surge_monk",
                name: "Surge Monk",
                base: ClassId::Stormfist,
                weapon: Weapon::Nunchucks,
                weapon_name: "Storm Nunchucks",
                blurb: "Nunchucks that crackle with lightning. Blinding combos that build power with every hit.",
                choice: "Become a Surge Monk",
                starter: "surge_nunchucks",
                moves: mm(0.95, 0.86, 1.1, 1.0, None, false),
            },
            Warlord => JobInfo {
                key: "warlord",
                name: "Warlord",
                base: ClassId::Stormfist,
                weapon: Weapon::WarStaff,
                weapon_name: "War Staff",
                blurb: "Sweeps whole crowds with a heavy war staff and rallies the party behind a battle standard.",
                choice: "Become a Warlord",
                starter: "warlord_staff",
                moves: mm(1.18, 1.06, 1.45, 1.0, None, false),
            },
            UmbralNinja => JobInfo {
                key: "umbral_ninja",
                name: "Umbral Ninja",
                base: ClassId::Duskblade,
                weapon: Weapon::Katana,
                weapon_name: "Katana",
                blurb: "One perfect cut from the shadows. Every first strike is a critical hit.",
                choice: "Become an Umbral Ninja",
                starter: "umbral_katana",
                moves: mm(1.18, 0.96, 1.25, 1.1, None, false),
            },
            SoulReaper => JobInfo {
                key: "soul_reaper",
                name: "Soul Reaper",
                base: ClassId::Duskblade,
                weapon: Weapon::Kusarigama,
                weapon_name: "Kusarigama",
                blurb: "A sickle on a chain. Hooks monsters in, dodges out, and hits hardest in the moments after it moves.",
                choice: "Become a Soul Reaper",
                starter: "reaper_kusarigama",
                moves: mm(1.1, 1.0, 1.55, 1.0, None, false),
            },
            WitchHunter => JobInfo {
                key: "witch_hunter",
                name: "Witch Hunter",
                base: ClassId::Windrunner,
                weapon: Weapon::Crossbow,
                weapon_name: "Repeating Crossbow",
                blurb: "Silver bolts and hex traps. Built to bring down bosses and finish off the wounded.",
                choice: "Become a Witch Hunter",
                starter: "hunter_crossbow",
                moves: mm(1.16, 1.0, 1.0, 1.3, None, true),
            },
            BeastMaster => JobInfo {
                key: "beast_master",
                name: "Beast Master",
                base: ClassId::Windrunner,
                weapon: Weapon::Greatbow,
                weapon_name: "Greatbow",
                blurb: "A towering greatbow and a spirit hawk at your side. Your pet grows stronger too.",
                choice: "Become a Beast Master",
                starter: "beast_greatbow",
                moves: mm(1.24, 1.1, 1.0, 1.15, None, false),
            },
            Oracle => JobInfo {
                key: "oracle",
                name: "Oracle",
                base: ClassId::Aethermancer,
                weapon: Weapon::CosmicOrbs,
                weapon_name: "Cosmic Orbs",
                blurb: "Stars orbit the Oracle and burn anything that comes close. Sees blows coming and heals the party.",
                choice: "Become an Oracle",
                starter: "oracle_orbs",
                moves: mm(1.1, 1.0, 1.0, 1.0, Some(P::Orb), false),
            },
            Gambler => JobInfo {
                key: "gambler",
                name: "Gambler",
                base: ClassId::Aethermancer,
                weapon: Weapon::Cards,
                weapon_name: "Deck of Cards",
                blurb: "Enchanted cards and loaded dice. Every throw is a bet - and the house usually wins.",
                choice: "Become a Gambler",
                starter: "gambler_deck",
                moves: mm(1.05, 0.9, 1.0, 1.2, Some(P::Card), false),
            },
            Engineer => JobInfo {
                key: "engineer",
                name: "Engineer",
                base: ClassId::Gunslinger,
                weapon: Weapon::HandCannon,
                weapon_name: "Hand-Cannon",
                blurb: "Exploding cannon shots and turrets: one that shoots, one that heals the party.",
                choice: "Become an Engineer",
                starter: "engineer_cannon",
                moves: mm(1.22, 1.12, 1.0, 0.85, Some(P::Cannonball), false),
            },
            Sniper => JobInfo {
                key: "sniper",
                name: "Sniper",
                base: ClassId::Gunslinger,
                weapon: Weapon::OmniRifle,
                weapon_name: "Omni-Rifle",
                blurb: "Hits from across the map. Stand still to steady your aim; the farther the target, the harder it hurts.",
                choice: "Become a Sniper",
                starter: "sniper_rifle",
                moves: mm(1.28, 1.12, 1.0, 1.6, Some(P::Bullet), true),
            },
        }
    }
}

/// The two advanced classes a base class can become.
pub fn jobs_for(class: ClassId) -> [Job; 2] {
    match class {
        ClassId::Vanguard => [Job::Berserker, Job::Paladin],
        ClassId::Stormfist => [Job::SurgeMonk, Job::Warlord],
        ClassId::Duskblade => [Job::UmbralNinja, Job::SoulReaper],
        ClassId::Windrunner => [Job::WitchHunter, Job::BeastMaster],
        ClassId::Aethermancer => [Job::Oracle, Job::Gambler],
        ClassId::Gunslinger => [Job::Engineer, Job::Sniper],
        ClassId::Novice => [Job::Berserker, Job::Paladin],
    }
}

/// A player-info class string: a base class key or an advanced class key.
pub fn class_and_job(key: &str) -> (ClassId, Option<Job>) {
    if let Some(j) = Job::from_key(key) {
        return (j.base(), Some(j));
    }
    (ClassId::from_key(key).unwrap_or(ClassId::Novice), None)
}

/// The display name for a player-info class string.
pub fn class_display(key: &str) -> &'static str {
    match class_and_job(key) {
        (_, Some(j)) => j.name(),
        (c, None) => c.info().name,
    }
}

/// Apply an advanced class's weapon to one of its base attacks.
pub fn apply_moves(job: Option<Job>, d: &mut AttackDef) {
    let Some(job) = job else { return };
    let m = job.info().moves;
    d.startup *= m.time;
    d.active *= m.time;
    d.recovery *= m.time;
    d.damage *= m.damage;
    if d.hw > 0.0 {
        d.hw *= m.reach;
        d.hx *= m.reach;
        d.hh *= m.reach.sqrt();
    }
    if let Some(p) = &mut d.projectile {
        p.vx *= m.proj_speed;
        p.vy *= m.proj_speed;
        p.life *= m.proj_speed.max(1.0);
        if m.pierce {
            p.pierce = true;
        }
        if let Some(s) = m.proj_style {
            if !matches!(p.style, ProjStyle::Caltrop | ProjStyle::Lightning | ProjStyle::Meteor) {
                p.style = s;
            }
        }
    }
}

/// Damage multiplier for a job's projectile attacks (their damage table
/// lives with the base class).
pub fn projectile_mult(job: Option<Job>) -> f32 {
    job.map(|j| j.info().moves.damage).unwrap_or(1.0)
}

// -------------------------------------------------------------------- passives

/// What a passive ability does beyond its stat bonus.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PassiveFx {
    /// Stats only.
    None,
    /// +attack while your HP is below a fraction.
    Rage { below: f32, atk: f32 },
    /// Heal this fraction of the damage you deal.
    Lifesteal(f32),
    /// Monsters prefer to attack you (online: the server's monsters target you first).
    Threat,
    /// Damage you take is multiplied by this.
    Guard(f32),
    /// Your first hit on each monster is always a critical hit.
    FirstStrike,
    /// Each hit within 2.5 s adds `per` damage, up to `max` stacks.
    Combo { per: f32, max: u32 },
    /// Critical hits arc lightning to a nearby monster.
    CritChain,
    /// Drop below `below` HP: heal `heal` of max HP (once every `cd` seconds).
    LastStand { below: f32, heal: f32, cd: f32 },
    /// After a dodge, +`dmg` damage for `secs` seconds.
    DodgeWindow { secs: f32, dmg: f32 },
    /// Kills take this many seconds off your ability cooldowns.
    KillCdr(f32),
    /// +damage against bosses.
    BossBane(f32),
    /// +damage against monsters below a fraction of their HP.
    Execute { below: f32, dmg: f32 },
    /// Your pet's bonuses are multiplied by this.
    PetBoost(f32),
    /// Every Nth ranged attack fires twice.
    ExtraShot(u32),
    /// Orbs circle you and burn monsters they touch (damage x your attack).
    OrbitOrbs { count: u32, dmg: f32 },
    /// Chance to evade a hit completely.
    Evade(f32),
    /// Mana restored on every critical hit.
    CritMana(f32),
    /// Chance for a hit to deal double damage.
    DoubleChance(f32),
    /// Turrets fire this much faster and last this much longer.
    TurretBoost(f32),
    /// Standing still for `secs` seconds gives +`crit` crit chance.
    Stillness { secs: f32, crit: f32 },
    /// Up to +`max` damage against far-away targets.
    LongShot { max: f32 },
    /// Ranged attacks explode on impact (blast radius).
    Blast(f32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Passive {
    pub id: &'static str,
    pub name: &'static str,
    pub job: Job,
    pub level: u32,
    pub desc: &'static str,
    pub mods: StatMods,
    pub fx: PassiveFx,
}

const Z: StatMods = StatMods {
    main: MainStats { str: 0, dex: 0, int: 0, luk: 0 },
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
};

const fn p(id: &'static str, name: &'static str, job: Job, level: u32, desc: &'static str, mods: StatMods, fx: PassiveFx) -> Passive {
    Passive { id, name, job, level, desc, mods, fx }
}

use Job::*;
use PassiveFx as F;

pub const PASSIVES: &[Passive] = &[
    // Berserker
    p("bloodlust", "Bloodlust", Berserker, 30, "+25% attack while your HP is below half.", Z, F::Rage { below: 0.5, atk: 0.25 }),
    p("greatsword_mastery", "Greatsword Mastery", Berserker, 35, "+12% attack and +20% crit damage.", StatMods { atk_pct: 0.12, crit_dmg: 20.0, ..Z }, F::None),
    p("undying_fury", "Undying Fury", Berserker, 45, "Heal for 4% of all damage you deal. +80 HP.", StatMods { hp: 80.0, ..Z }, F::Lifesteal(0.04)),
    // Paladin
    p("provoke", "Provoke", Paladin, 30, "Monsters go for you before the rest of your party. +120 HP.", StatMods { hp: 120.0, ..Z }, F::Threat),
    p("aegis", "Aegis", Paladin, 35, "Take 15% less damage from everything.", Z, F::Guard(0.85)),
    p("guardians_oath", "Guardian's Oath", Paladin, 45, "+25% P.DEF and M.DEF.", StatMods { pdef_pct: 0.25, mdef_pct: 0.25, ..Z }, F::None),
    // Surge Monk
    p("flow_state", "Flow State", SurgeMonk, 30, "Each hit within 2.5 seconds of the last adds +2% damage, up to +20%.", Z, F::Combo { per: 0.02, max: 10 }),
    p("lightning_reflexes", "Lightning Reflexes", SurgeMonk, 35, "+8% move speed and +6% crit chance.", StatMods { speed_pct: 0.08, crit: 6.0, ..Z }, F::None),
    p("static_overload", "Static Overload", SurgeMonk, 45, "Critical hits arc lightning to a nearby monster.", Z, F::CritChain),
    // Warlord
    p("commanding_presence", "Commanding Presence", Warlord, 30, "+10% attack and +100 HP.", StatMods { atk_pct: 0.1, hp: 100.0, ..Z }, F::None),
    p("staff_mastery", "Staff Mastery", Warlord, 35, "+8% attack, +5% crit chance and +10% P.DEF.", StatMods { atk_pct: 0.08, crit: 5.0, pdef_pct: 0.1, ..Z }, F::None),
    p("warlords_resolve", "Warlord's Resolve", Warlord, 45, "Drop below 30% HP and instantly heal 30% (once a minute).", Z, F::LastStand { below: 0.3, heal: 0.3, cd: 60.0 }),
    // Umbral Ninja
    p("first_blood", "First Blood", UmbralNinja, 30, "Your first hit on every monster is a critical hit.", Z, F::FirstStrike),
    p("ninjutsu", "Ninjutsu", UmbralNinja, 35, "+8% crit chance and +25% crit damage.", StatMods { crit: 8.0, crit_dmg: 25.0, ..Z }, F::None),
    p("vanish", "Vanish", UmbralNinja, 45, "15% chance to evade a hit completely.", Z, F::Evade(0.15)),
    // Soul Reaper
    p("reapers_window", "Reaper's Window", SoulReaper, 30, "After a dodge, deal +35% damage for 2.5 seconds.", Z, F::DodgeWindow { secs: 2.5, dmg: 0.35 }),
    p("soul_harvest", "Soul Harvest", SoulReaper, 35, "Every kill takes 1.5 seconds off your ability cooldowns.", Z, F::KillCdr(1.5)),
    p("soul_siphon", "Soul Siphon", SoulReaper, 45, "Heal for 5% of all damage you deal. +6% crit chance.", StatMods { crit: 6.0, ..Z }, F::Lifesteal(0.05)),
    // Witch Hunter
    p("hex_bane", "Hex Bane", WitchHunter, 30, "+30% damage against bosses.", Z, F::BossBane(0.3)),
    p("steady_hands", "Steady Hands", WitchHunter, 35, "+10% attack and +6% crit chance.", StatMods { atk_pct: 0.1, crit: 6.0, ..Z }, F::None),
    p("executioner", "Executioner", WitchHunter, 45, "+40% damage against monsters below 30% HP.", Z, F::Execute { below: 0.3, dmg: 0.4 }),
    // Beast Master
    p("wild_instinct", "Wild Instinct", BeastMaster, 30, "+8% move speed, +6% crit chance and +60 HP.", StatMods { speed_pct: 0.08, crit: 6.0, hp: 60.0, ..Z }, F::None),
    p("alpha_bond", "Alpha Bond", BeastMaster, 35, "Your pet's bonuses are doubled.", Z, F::PetBoost(2.0)),
    p("feral_volley", "Feral Volley", BeastMaster, 45, "Every 3rd ranged attack fires a second volley.", Z, F::ExtraShot(3)),
    // Oracle
    p("orbiting_stars", "Orbiting Stars", Oracle, 30, "Three stars circle you, burning monsters they touch.", Z, F::OrbitOrbs { count: 3, dmg: 0.35 }),
    p("foresight", "Foresight", Oracle, 35, "12% chance to evade a hit completely. +40 Mana.", StatMods { mp: 40.0, ..Z }, F::Evade(0.12)),
    p("astral_mind", "Astral Mind", Oracle, 45, "+15% attack and +80 Mana.", StatMods { atk_pct: 0.15, mp: 80.0, ..Z }, F::None),
    // Gambler
    p("lucky_streak", "Lucky Streak", Gambler, 30, "Critical hits restore 3 Mana.", StatMods { crit: 4.0, ..Z }, F::CritMana(3.0)),
    p("loaded_dice", "Loaded Dice", Gambler, 35, "+10% crit chance and +20% crit damage.", StatMods { crit: 10.0, crit_dmg: 20.0, ..Z }, F::None),
    p("house_edge", "House Edge", Gambler, 45, "12% chance for any hit to deal double damage.", Z, F::DoubleChance(0.12)),
    // Engineer
    p("heavy_ordnance", "Heavy Ordnance", Engineer, 30, "Cannon shots explode, hitting everything nearby.", Z, F::Blast(70.0)),
    p("overclock", "Overclock", Engineer, 35, "Turrets fire 35% faster and last 35% longer.", Z, F::TurretBoost(1.35)),
    p("field_plating", "Field Plating", Engineer, 45, "+15% P.DEF and M.DEF and +100 HP.", StatMods { pdef_pct: 0.15, mdef_pct: 0.15, hp: 100.0, ..Z }, F::None),
    // Sniper
    p("steady_aim", "Steady Aim", Sniper, 30, "Stand still for a second: +25% crit chance until you move.", Z, F::Stillness { secs: 1.0, crit: 25.0 }),
    p("long_shot", "Long Shot", Sniper, 35, "Up to +40% damage against far-away targets.", Z, F::LongShot { max: 0.4 }),
    p("marksman", "Marksman", Sniper, 45, "+12% attack and +20% crit damage.", StatMods { atk_pct: 0.12, crit_dmg: 20.0, ..Z }, F::None),
];

/// Passives this character has learned.
pub fn passives(job: Option<Job>, level: u32) -> impl Iterator<Item = &'static Passive> {
    PASSIVES.iter().filter(move |p| Some(p.job) == job && level >= p.level)
}

/// Every passive of an advanced class (learned or not), in level order.
pub fn job_passives(job: Job) -> Vec<&'static Passive> {
    PASSIVES.iter().filter(|p| p.job == job).collect()
}

/// Stat bonuses from learned passives.
pub fn passive_mods(job: Option<Job>, level: u32) -> StatMods {
    let mut m = StatMods::default();
    for p in passives(job, level) {
        m = m + p.mods;
    }
    m
}

/// The first learned passive with an effect matching `f`.
pub fn find_fx(job: Option<Job>, level: u32, f: impl Fn(&PassiveFx) -> bool) -> Option<PassiveFx> {
    passives(job, level).map(|p| p.fx).find(|x| f(x))
}

/// Does this attack kind fire projectiles for the class (for ExtraShot etc.)?
pub fn is_ranged(class: ClassId, kind: AttackKind) -> bool {
    crate::classes::attack_def(class, kind).projectile.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_class_has_two_jobs_with_three_passives() {
        for &c in crate::classes::ALL_CLASSES {
            let [a, b] = jobs_for(c);
            assert_ne!(a, b);
            for j in [a, b] {
                assert_eq!(j.base(), c);
                assert_eq!(Job::from_key(j.key()), Some(j));
                let ps = job_passives(j);
                assert_eq!(ps.len(), 3, "{:?}", j);
                assert_eq!(ps[0].level, JOB_LEVEL);
                assert!(crate::items::item(j.info().starter).is_some(), "{:?} starter weapon", j);
                assert_eq!(class_and_job(j.key()), (c, Some(j)));
            }
        }
        for (i, a) in PASSIVES.iter().enumerate() {
            for b in &PASSIVES[i + 1..] {
                assert_ne!(a.id, b.id);
            }
        }
        assert_eq!(class_and_job("vanguard"), (ClassId::Vanguard, None));
    }
}
