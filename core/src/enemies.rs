//! Monsters: data-driven definitions and behaviour.
//!
//! Every monster uses one of a few behaviours (AI archetypes) with its own
//! numbers, so new zones add monsters by adding data, not code:
//! - Hopper: bounces around; hops at you when you get close.
//! - Lunger: patrols; puffs up (telegraph) then lunges.
//! - Floater: hovers; either shoots aimed projectiles or dive-bombs you.
//! - Boss: charges (telegraphed), stomps shockwaves, spits volleys; enrages.

use crate::classes::ProjStyle;
use crate::math::{approach, Rng, V2};
use crate::physics::{self, Body, Level};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EnemyKind {
    TrainingDummy,
    // Dawnwind Glade
    Mossling,
    Burrowbeak,
    Bramblehorn,
    // Whistling Fields
    Puffwool,
    Gustling,
    // Thornvale Woods
    Gravecap,
    Lanternwisp,
    Rotstump,
    // Windcrag Cliffs
    CragCrab,
    Galehawk,
    Thunderhorn,
    // Roads to the class towns (recoloured cousins)
    Rockmoss,
    Ironbeak,
    QuarryCrab,
    Shadecap,
    Gloomwisp,
    MireStump,
    Stormwool,
    Squallsprite,
    Thunderhawk,
    Thornling,
    Briarbeak,
    Barkstump,
    Starwool,
    Voidgust,
    AstralCrab,
}

pub const ALL_KINDS: &[EnemyKind] = &[
    EnemyKind::TrainingDummy,
    EnemyKind::Mossling,
    EnemyKind::Burrowbeak,
    EnemyKind::Bramblehorn,
    EnemyKind::Puffwool,
    EnemyKind::Gustling,
    EnemyKind::Gravecap,
    EnemyKind::Lanternwisp,
    EnemyKind::Rotstump,
    EnemyKind::CragCrab,
    EnemyKind::Galehawk,
    EnemyKind::Thunderhorn,
    EnemyKind::Rockmoss,
    EnemyKind::Ironbeak,
    EnemyKind::QuarryCrab,
    EnemyKind::Shadecap,
    EnemyKind::Gloomwisp,
    EnemyKind::MireStump,
    EnemyKind::Stormwool,
    EnemyKind::Squallsprite,
    EnemyKind::Thunderhawk,
    EnemyKind::Thornling,
    EnemyKind::Briarbeak,
    EnemyKind::Barkstump,
    EnemyKind::Starwool,
    EnemyKind::Voidgust,
    EnemyKind::AstralCrab,
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ai {
    Dummy,
    Hopper,
    Lunger,
    /// `diver` = dive-bombs instead of shooting.
    Floater { diver: bool },
    Boss,
}

impl Ai {
    pub fn is_floater(&self) -> bool {
        matches!(self, Ai::Floater { .. })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct EnemyDef {
    pub name: &'static str,
    pub w: f32,
    pub h: f32,
    pub hp: f32,
    /// Damage when touching it.
    pub contact: f32,
    /// Damage of its lunge / charge / dive.
    pub attack: f32,
    pub exp: u32,
    pub gold: u32,
    pub level: u32,
    /// Higher = less knockback.
    pub weight: f32,
    pub ai: Ai,
    /// Walk / hop / fly speed.
    pub speed: f32,
    /// Hop jump speed (Hopper) or charge speed (Boss) or dive speed (Floater).
    pub burst: f32,
    pub aggro: f32,
    pub telegraph: f32,
    /// Seconds between ranged attacks.
    pub fire_every: f32,
    pub proj_speed: f32,
    pub proj_damage: f32,
    pub proj_style: ProjStyle,
    /// Hover height above its spawn point (floaters).
    pub hover: f32,
    pub boss: bool,
}

const BASE: EnemyDef = EnemyDef {
    name: "",
    w: 34.0,
    h: 34.0,
    hp: 50.0,
    contact: 8.0,
    attack: 10.0,
    exp: 10,
    gold: 2,
    level: 1,
    weight: 1.0,
    ai: Ai::Hopper,
    speed: 160.0,
    burst: 460.0,
    aggro: 420.0,
    telegraph: 0.55,
    fire_every: 2.6,
    proj_speed: 300.0,
    proj_damage: 10.0,
    proj_style: ProjStyle::Orb,
    hover: 110.0,
    boss: false,
};

impl EnemyKind {
    pub fn def(self) -> EnemyDef {
        use EnemyKind::*;
        match self {
            TrainingDummy => EnemyDef { name: "Training Dummy", w: 34.0, h: 64.0, hp: 9999.0, contact: 0.0, exp: 0, gold: 0, weight: 99.0, ai: Ai::Dummy, ..BASE },
            Mossling => EnemyDef { name: "Mossling", w: 36.0, h: 28.0, hp: 40.0, contact: 6.0, exp: 8, gold: 2, level: 2, weight: 0.8, speed: 170.0, burst: 460.0, aggro: 440.0, ..BASE },
            Burrowbeak => EnemyDef {
                name: "Burrowbeak", w: 34.0, h: 44.0, hp: 70.0, contact: 9.0, attack: 14.0, exp: 15, gold: 4, level: 4, ai: Ai::Lunger, speed: 120.0, burst: 520.0, aggro: 500.0, telegraph: 0.55, ..BASE
            },
            Bramblehorn => EnemyDef {
                name: "Bramblehorn", w: 96.0, h: 76.0, hp: 600.0, contact: 16.0, attack: 26.0, exp: 120, gold: 40, level: 7, weight: 3.0, ai: Ai::Boss, speed: 90.0, burst: 610.0, aggro: 900.0,
                telegraph: 0.8, proj_damage: 12.0, proj_style: ProjStyle::Caltrop, boss: true, ..BASE
            },
            Puffwool => EnemyDef { name: "Puffwool", w: 40.0, h: 34.0, hp: 75.0, contact: 10.0, exp: 22, gold: 5, level: 5, weight: 1.2, speed: 130.0, burst: 380.0, aggro: 360.0, ..BASE },
            Gustling => EnemyDef {
                name: "Gustling", w: 34.0, h: 34.0, hp: 60.0, contact: 8.0, exp: 28, gold: 6, level: 7, weight: 0.7, ai: Ai::Floater { diver: false }, speed: 90.0, aggro: 520.0,
                telegraph: 0.6, fire_every: 2.8, proj_speed: 260.0, proj_damage: 14.0, proj_style: ProjStyle::Orb, hover: 120.0, ..BASE
            },
            Gravecap => EnemyDef { name: "Gravecap", w: 38.0, h: 38.0, hp: 140.0, contact: 18.0, exp: 40, gold: 8, level: 9, weight: 1.1, speed: 150.0, burst: 430.0, aggro: 420.0, ..BASE },
            Lanternwisp => EnemyDef {
                name: "Lanternwisp", w: 30.0, h: 40.0, hp: 110.0, contact: 14.0, exp: 48, gold: 9, level: 10, weight: 0.7, ai: Ai::Floater { diver: false }, speed: 80.0, aggro: 560.0,
                telegraph: 0.7, fire_every: 2.4, proj_speed: 240.0, proj_damage: 20.0, proj_style: ProjStyle::Star, hover: 130.0, ..BASE
            },
            Rotstump => EnemyDef {
                name: "Rotstump", w: 46.0, h: 54.0, hp: 220.0, contact: 22.0, attack: 30.0, exp: 60, gold: 12, level: 11, weight: 1.8, ai: Ai::Lunger, speed: 80.0, burst: 430.0, aggro: 460.0, telegraph: 0.75, ..BASE
            },
            CragCrab => EnemyDef {
                name: "Crag Crab", w: 50.0, h: 36.0, hp: 300.0, contact: 26.0, attack: 34.0, exp: 75, gold: 15, level: 13, weight: 2.0, ai: Ai::Lunger, speed: 110.0, burst: 560.0, aggro: 480.0, telegraph: 0.6, ..BASE
            },
            Galehawk => EnemyDef {
                name: "Galehawk", w: 44.0, h: 30.0, hp: 240.0, contact: 20.0, attack: 36.0, exp: 85, gold: 16, level: 14, weight: 0.9, ai: Ai::Floater { diver: true }, speed: 130.0, burst: 560.0,
                aggro: 560.0, telegraph: 0.55, fire_every: 2.2, hover: 150.0, ..BASE
            },
            Thunderhorn => EnemyDef {
                name: "Thunderhorn", w: 104.0, h: 82.0, hp: 2000.0, contact: 32.0, attack: 44.0, exp: 600, gold: 150, level: 16, weight: 3.5, ai: Ai::Boss, speed: 105.0, burst: 680.0,
                aggro: 1000.0, telegraph: 0.7, proj_damage: 26.0, proj_style: ProjStyle::Bolt, boss: true, ..BASE
            },
            Rockmoss => EnemyDef { name: "Rockmoss", w: 36.0, h: 28.0, hp: 260.0, contact: 24.0, exp: 70, gold: 12, level: 10, weight: 1.2, speed: 170.0, burst: 470.0, aggro: 440.0, ..BASE },
            Ironbeak => EnemyDef { name: "Ironbeak", w: 34.0, h: 44.0, hp: 330.0, contact: 26.0, attack: 36.0, exp: 85, gold: 14, level: 12, ai: Ai::Lunger, speed: 120.0, burst: 560.0, aggro: 500.0, telegraph: 0.55, ..BASE },
            QuarryCrab => EnemyDef { name: "Quarry Crab", w: 50.0, h: 36.0, hp: 620.0, contact: 36.0, attack: 48.0, exp: 150, gold: 26, level: 19, weight: 2.2, ai: Ai::Lunger, speed: 115.0, burst: 580.0, aggro: 480.0, telegraph: 0.6, ..BASE },
            Shadecap => EnemyDef { name: "Shadecap", w: 38.0, h: 38.0, hp: 280.0, contact: 26.0, exp: 72, gold: 12, level: 11, weight: 1.1, speed: 160.0, burst: 450.0, aggro: 440.0, ..BASE },
            Gloomwisp => EnemyDef { name: "Gloomwisp", w: 30.0, h: 40.0, hp: 300.0, contact: 24.0, exp: 90, gold: 14, level: 13, weight: 0.7, ai: Ai::Floater { diver: false }, speed: 85.0, aggro: 560.0, telegraph: 0.65, fire_every: 2.2, proj_speed: 260.0, proj_damage: 32.0, proj_style: ProjStyle::Star, hover: 130.0, ..BASE },
            MireStump => EnemyDef { name: "Mire Stump", w: 46.0, h: 54.0, hp: 700.0, contact: 38.0, attack: 52.0, exp: 165, gold: 28, level: 20, weight: 2.0, ai: Ai::Lunger, speed: 85.0, burst: 450.0, aggro: 460.0, telegraph: 0.7, ..BASE },
            Stormwool => EnemyDef { name: "Stormwool", w: 40.0, h: 34.0, hp: 270.0, contact: 24.0, exp: 68, gold: 12, level: 10, weight: 1.2, speed: 140.0, burst: 400.0, aggro: 380.0, ..BASE },
            Squallsprite => EnemyDef { name: "Squallsprite", w: 34.0, h: 34.0, hp: 260.0, contact: 22.0, exp: 82, gold: 13, level: 12, weight: 0.7, ai: Ai::Floater { diver: false }, speed: 95.0, aggro: 520.0, telegraph: 0.55, fire_every: 2.4, proj_speed: 300.0, proj_damage: 28.0, proj_style: ProjStyle::Orb, hover: 120.0, ..BASE },
            Thunderhawk => EnemyDef { name: "Thunderhawk", w: 44.0, h: 30.0, hp: 600.0, contact: 34.0, attack: 54.0, exp: 175, gold: 30, level: 21, weight: 1.0, ai: Ai::Floater { diver: true }, speed: 140.0, burst: 600.0, aggro: 580.0, telegraph: 0.5, fire_every: 2.0, hover: 150.0, ..BASE },
            Thornling => EnemyDef { name: "Thornling", w: 36.0, h: 28.0, hp: 250.0, contact: 26.0, exp: 68, gold: 12, level: 10, weight: 0.9, speed: 185.0, burst: 500.0, aggro: 460.0, ..BASE },
            Briarbeak => EnemyDef { name: "Briarbeak", w: 34.0, h: 44.0, hp: 320.0, contact: 26.0, attack: 36.0, exp: 84, gold: 14, level: 12, ai: Ai::Lunger, speed: 125.0, burst: 560.0, aggro: 500.0, telegraph: 0.5, ..BASE },
            Barkstump => EnemyDef { name: "Barkstump", w: 46.0, h: 54.0, hp: 640.0, contact: 36.0, attack: 50.0, exp: 150, gold: 26, level: 18, weight: 2.0, ai: Ai::Lunger, speed: 85.0, burst: 450.0, aggro: 460.0, telegraph: 0.7, ..BASE },
            Starwool => EnemyDef { name: "Starwool", w: 40.0, h: 34.0, hp: 330.0, contact: 28.0, exp: 90, gold: 15, level: 13, weight: 1.2, speed: 140.0, burst: 420.0, aggro: 380.0, ..BASE },
            Voidgust => EnemyDef { name: "Voidgust", w: 34.0, h: 34.0, hp: 560.0, contact: 34.0, exp: 180, gold: 30, level: 22, weight: 0.8, ai: Ai::Floater { diver: false }, speed: 100.0, aggro: 560.0, telegraph: 0.5, fire_every: 2.0, proj_speed: 320.0, proj_damage: 46.0, proj_style: ProjStyle::Bolt, hover: 125.0, ..BASE },
            AstralCrab => EnemyDef { name: "Astral Crab", w: 50.0, h: 36.0, hp: 820.0, contact: 42.0, attack: 58.0, exp: 210, gold: 34, level: 23, weight: 2.4, ai: Ai::Lunger, speed: 120.0, burst: 600.0, aggro: 500.0, telegraph: 0.55, ..BASE },
        }
    }
    pub fn name(self) -> &'static str {
        self.def().name
    }
    pub fn key(self) -> &'static str {
        use EnemyKind::*;
        match self {
            TrainingDummy => "dummy",
            Mossling => "mossling",
            Burrowbeak => "burrowbeak",
            Bramblehorn => "bramblehorn",
            Puffwool => "puffwool",
            Gustling => "gustling",
            Gravecap => "gravecap",
            Lanternwisp => "lanternwisp",
            Rotstump => "rotstump",
            CragCrab => "cragcrab",
            Galehawk => "galehawk",
            Thunderhorn => "thunderhorn",
            Rockmoss => "rockmoss",
            Ironbeak => "ironbeak",
            QuarryCrab => "quarrycrab",
            Shadecap => "shadecap",
            Gloomwisp => "gloomwisp",
            MireStump => "mirestump",
            Stormwool => "stormwool",
            Squallsprite => "squallsprite",
            Thunderhawk => "thunderhawk",
            Thornling => "thornling",
            Briarbeak => "briarbeak",
            Barkstump => "barkstump",
            Starwool => "starwool",
            Voidgust => "voidgust",
            AstralCrab => "astralcrab",
        }
    }
    /// Which monster's art (and behaviour family) this one is a variant of.
    pub fn art_base(self) -> EnemyKind {
        use EnemyKind::*;
        match self {
            Rockmoss => Mossling,
            Ironbeak => Burrowbeak,
            QuarryCrab => CragCrab,
            Shadecap => Gravecap,
            Gloomwisp => Lanternwisp,
            MireStump => Rotstump,
            Stormwool => Puffwool,
            Squallsprite => Gustling,
            Thunderhawk => Galehawk,
            Thornling => Mossling,
            Briarbeak => Burrowbeak,
            Barkstump => Rotstump,
            Starwool => Puffwool,
            Voidgust => Gustling,
            AstralCrab => CragCrab,
            other => other,
        }
    }
    /// Colour multiplier for variants drawn with another monster's art.
    pub fn tint(self) -> [f32; 3] {
        use EnemyKind::*;
        match self {
            Rockmoss => [0.72, 0.74, 0.8],
            Ironbeak => [0.7, 0.78, 0.95],
            QuarryCrab => [0.62, 0.7, 0.86],
            Shadecap => [0.7, 0.52, 1.0],
            Gloomwisp => [0.62, 0.7, 1.0],
            MireStump => [0.6, 0.78, 0.6],
            Stormwool => [1.0, 0.92, 0.55],
            Squallsprite => [0.7, 1.0, 0.85],
            Thunderhawk => [1.0, 0.85, 0.45],
            Thornling => [1.0, 0.62, 0.52],
            Briarbeak => [0.62, 0.95, 0.55],
            Barkstump => [0.85, 0.75, 0.55],
            Starwool => [0.78, 0.66, 1.0],
            Voidgust => [0.58, 0.45, 0.95],
            AstralCrab => [0.72, 0.58, 1.0],
            _ => [1.0, 1.0, 1.0],
        }
    }
    /// What it can drop: (item id, chance 0-1). Rolled independently.
    pub fn drops(self) -> &'static [(&'static str, f32)] {
        use EnemyKind::*;
        match self {
            TrainingDummy => &[],
            Mossling => &[("moss_clump", 0.5), ("hp_potion", 0.06)],
            Burrowbeak => &[("beak_shard", 0.45), ("hp_potion", 0.08), ("sandals", 0.02), ("leather_cap", 0.01)],
            Bramblehorn => &[("bramble_thorn", 1.0), ("wood_sword", 0.5), ("copper_ring", 0.5)],
            Puffwool => &[("wool_tuft", 0.5), ("mp_potion", 0.08), ("cloth_tunic", 0.02), ("leather_breeches", 0.015)],
            Gustling => &[("gust_wisp", 0.45), ("mp_potion", 0.1), ("travel_cloak", 0.01)],
            Gravecap => &[("cap_spore", 0.45), ("hp_potion", 0.08), ("leather_boots", 0.015), ("fang_necklace", 0.008)],
            Lanternwisp => &[("wisp_ember", 0.45), ("mp_potion", 0.1), ("sage_pendant", 0.01), ("brass_hoops", 0.008)],
            Rotstump => &[("rotten_bark", 0.45), ("hp_potion_big", 0.05), ("leather_vest", 0.015), ("silver_ring", 0.008)],
            CragCrab => &[("crab_shell", 0.45), ("hp_potion_big", 0.06), ("power_band", 0.01), ("iron_helm", 0.006)],
            Galehawk => &[("hawk_feather", 0.45), ("mp_potion_big", 0.05), ("windstep_boots", 0.01), ("gale_cape", 0.006)],
            Thunderhorn => &[("storm_horn", 1.0), ("storm_amulet", 0.25), ("elixir", 1.0), ("gale_ring", 0.15), ("gale_circlet", 0.1)],
            Rockmoss | Thornling => &[("moss_clump", 0.6), ("iron_ore", 0.15), ("hp_potion", 0.1)],
            Ironbeak | Briarbeak => &[("beak_shard", 0.6), ("iron_ore", 0.2), ("hp_potion_big", 0.04), ("chain_shirt", 0.008)],
            Shadecap => &[("cap_spore", 0.6), ("shadow_silk", 0.15), ("hp_potion", 0.1), ("ranger_hood", 0.006)],
            Gloomwisp => &[("wisp_ember", 0.6), ("shadow_silk", 0.2), ("mp_potion_big", 0.05), ("shadow_garb", 0.008), ("shadow_leggings", 0.008)],
            Stormwool | Starwool => &[("wool_tuft", 0.6), ("star_dust", 0.12), ("mp_potion", 0.1), ("sage_robe", 0.006), ("sage_hat", 0.006)],
            Squallsprite => &[("gust_wisp", 0.6), ("thunder_core", 0.1), ("mp_potion_big", 0.05), ("sage_ring", 0.006)],
            QuarryCrab => &[("crab_shell", 0.6), ("iron_ore", 0.4), ("hp_potion_big", 0.06), ("iron_greaves", 0.01), ("plate_armor", 0.004)],
            MireStump | Barkstump => &[("rotten_bark", 0.6), ("shadow_silk", 0.15), ("elixir", 0.02), ("monk_gi", 0.004)],
            Thunderhawk => &[("hawk_feather", 0.6), ("thunder_core", 0.3), ("mp_potion_big", 0.06), ("storm_amulet", 0.004), ("feather_wings", 0.003)],
            Voidgust | AstralCrab => &[("star_dust", 0.5), ("thunder_core", 0.15), ("elixir", 0.03), ("rogue_coin", 0.01), ("star_drop", 0.006), ("gale_greaves", 0.004)],
        }
    }
    pub fn is_boss(self) -> bool {
        self.def().boss
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EState {
    Idle,
    Walk,
    Telegraph,
    Lunge,
    Recover,
    Hitstun,
    Dying,
    // boss
    Charge,
    Stunned,
    Stomp,
    Spit,
}

/// Every state, indexed by `EState as u32` (for the network).
pub const ALL_ESTATES: [EState; 11] = [
    EState::Idle,
    EState::Walk,
    EState::Telegraph,
    EState::Lunge,
    EState::Recover,
    EState::Hitstun,
    EState::Dying,
    EState::Charge,
    EState::Stunned,
    EState::Stomp,
    EState::Spit,
];

impl EState {
    pub fn from_u32(i: u32) -> Option<EState> {
        ALL_ESTATES.get(i as usize).copied()
    }
}

impl EnemyKind {
    /// Index in `ALL_KINDS` (for the network).
    pub fn index(self) -> u32 {
        ALL_KINDS.iter().position(|k| *k == self).unwrap_or(0) as u32
    }
    pub fn from_index(i: u32) -> Option<EnemyKind> {
        ALL_KINDS.get(i as usize).copied()
    }
}

/// A hit landed on a monster: lose HP, flash, and get knocked back (bosses
/// shrug off knockback mid-charge). Shared by the game and the server.
pub fn apply_hit(e: &mut Enemy, dmg: f32, kb: V2, hitstun: f32) {
    let def = e.def();
    e.hp -= dmg;
    e.flash = 0.12;
    let frac = (e.hp / e.max_hp).clamp(0.0, 1.0);
    let scale = (1.0 + 0.6 * (1.0 - frac)) / def.weight;
    let boss_armor = def.boss && matches!(e.state, EState::Charge | EState::Stomp);
    if !boss_armor {
        e.body.vel = V2::new(kb.x * scale, kb.y * scale);
        if e.body.vel.y < 0.0 {
            e.body.on_ground = false;
        }
        e.state = EState::Hitstun;
        e.state_t = 0.0;
        e.think = hitstun * if def.boss { 0.4 } else { 1.0 };
    }
    e.facing = -kb.x.signum();
}

#[derive(Clone, Debug)]
pub struct Enemy {
    pub id: u32,
    pub kind: EnemyKind,
    pub body: Body,
    pub hp: f32,
    pub max_hp: f32,
    pub facing: f32,
    pub state: EState,
    pub state_t: f32,
    pub think: f32,
    pub zone: Option<usize>,
    pub flash: f32,
    pub wobble: f32,
    pub level: u32,
    pub charge_dist: f32,
    pub attack_hit: bool,
    pub boss_phase2: bool,
    /// Where it spawned (feet); floaters hover relative to this.
    pub home: V2,
    pub cooldown: f32,
    /// Aim direction stored at the end of a telegraph (dives / shots).
    pub aim: V2,
    /// Online: where the server last said it is (the client glides there).
    pub net: Option<V2>,
}

impl Enemy {
    pub fn new(id: u32, kind: EnemyKind, x: f32, feet_y: f32, zone: Option<usize>, rng: &mut Rng) -> Enemy {
        let d = kind.def();
        let floater = matches!(d.ai, Ai::Floater { .. });
        let y = if floater { feet_y - d.hover } else { feet_y };
        Enemy {
            id,
            kind,
            body: Body::new(x - d.w * 0.5, y - d.h, d.w, d.h),
            hp: d.hp,
            max_hp: d.hp,
            facing: if rng.f() < 0.5 { -1.0 } else { 1.0 },
            state: EState::Idle,
            state_t: 0.0,
            think: rng.range(0.2, 1.2),
            zone,
            flash: 0.0,
            wobble: 0.0,
            level: d.level,
            charge_dist: 0.0,
            attack_hit: false,
            boss_phase2: false,
            home: V2::new(x, feet_y),
            cooldown: rng.range(0.8, d.fire_every),
            aim: V2::ZERO,
            net: None,
        }
    }
    pub fn def(&self) -> EnemyDef {
        self.kind.def()
    }
    /// Is it currently doing a damaging attack (lunge, charge, dive)?
    pub fn attacking(&self) -> bool {
        matches!(self.state, EState::Lunge | EState::Charge)
    }
    pub fn is_floater(&self) -> bool {
        matches!(self.def().ai, Ai::Floater { .. })
    }
}

/// A projectile requested by a monster; the simulation turns it into a real one.
#[derive(Clone, Debug)]
pub struct Shot {
    pub pos: V2,
    pub vel: V2,
    pub size: f32,
    pub gravity: f32,
    pub life: f32,
    pub pierce: bool,
    pub damage: f32,
    pub kb: V2,
    pub style: ProjStyle,
    pub grounded: bool,
}

pub struct Ctx<'a> {
    pub level: &'a Level<'a>,
    pub player: V2,
    pub player_alive: bool,
    pub dt: f32,
}

#[derive(Default)]
pub struct Out {
    pub shots: Vec<Shot>,
    pub shake: f32,
}

/// Advance one monster by one tick.
pub fn update(e: &mut Enemy, ctx: &Ctx, out: &mut Out) {
    let dt = ctx.dt;
    e.state_t += dt;
    e.flash = (e.flash - dt).max(0.0);
    e.wobble = (e.wobble - dt * 2.5).max(0.0);
    let d = e.def();
    if d.ai == Ai::Dummy {
        e.body.vel = V2::ZERO;
        return;
    }
    let floater = e.is_floater();
    match e.state {
        EState::Dying => {
            e.body.vel.x *= 0.9;
            if floater {
                e.body.vel.y += 600.0 * dt; // drop out of the sky
            }
        }
        EState::Hitstun => {
            e.think -= dt;
            if floater {
                e.body.vel = e.body.vel * (1.0 - 5.0 * dt).max(0.0);
                if e.think <= 0.0 {
                    e.state = EState::Recover;
                    e.state_t = 0.0;
                }
            } else {
                if e.body.on_ground {
                    e.body.vel.x = approach(e.body.vel.x, 0.0, 1300.0 * dt);
                }
                if e.think <= 0.0 && e.body.on_ground {
                    e.state = EState::Idle;
                    e.state_t = 0.0;
                    e.think = 0.3;
                }
            }
        }
        _ => match d.ai {
            Ai::Hopper => hopper(e, &d, ctx),
            Ai::Lunger => lunger(e, &d, ctx),
            Ai::Floater { diver } => float_ai(e, &d, ctx, diver, out),
            Ai::Boss => boss(e, &d, ctx, out),
            Ai::Dummy => {}
        },
    }
    if !floater || e.state == EState::Dying {
        physics::apply_gravity(&mut e.body, dt, false, 1.0);
    }
    physics::move_and_collide(&mut e.body, ctx.level, dt);
}

fn to_player(e: &Enemy, ctx: &Ctx) -> (f32, f32, bool) {
    let c = e.body.center();
    let dx = ctx.player.x - c.x;
    let same_level = (ctx.player.y - c.y).abs() < 130.0;
    (dx.abs(), if dx >= 0.0 { 1.0 } else { -1.0 }, same_level)
}

fn hopper(e: &mut Enemy, d: &EnemyDef, ctx: &Ctx) {
    let (dist, dir, same) = to_player(e, ctx);
    e.think -= ctx.dt;
    if e.body.on_ground {
        e.body.vel.x = approach(e.body.vel.x, 0.0, 1100.0 * ctx.dt);
        if e.think <= 0.0 {
            let aggro = ctx.player_alive && dist < d.aggro && same;
            let hop_dir = if aggro {
                dir
            } else if e.state_t > 2.0 || !physics::ground_ahead(&e.body, e.facing, ctx.level) {
                -e.facing
            } else {
                e.facing
            };
            if !aggro {
                e.state_t = 0.0;
            }
            e.facing = hop_dir;
            e.body.vel = V2::new(hop_dir * if aggro { d.speed } else { d.speed * 0.5 }, -(if aggro { d.burst } else { d.burst * 0.7 }));
            e.body.on_ground = false;
            e.think = if aggro { 0.7 } else { 1.4 };
        }
    }
}

fn lunger(e: &mut Enemy, d: &EnemyDef, ctx: &Ctx) {
    let (dist, dir, same) = to_player(e, ctx);
    let lunge_time = 0.45;
    match e.state {
        EState::Telegraph => {
            e.body.vel.x = 0.0;
            if e.state_t > d.telegraph {
                e.state = EState::Lunge;
                e.state_t = 0.0;
                e.body.vel.x = e.facing * d.burst;
            }
        }
        EState::Lunge => {
            e.body.vel.x = e.facing * d.burst * (1.0 - e.state_t / lunge_time).max(0.0);
            if e.state_t > lunge_time {
                e.state = EState::Recover;
                e.state_t = 0.0;
            }
        }
        EState::Recover => {
            e.body.vel.x = approach(e.body.vel.x, 0.0, 1500.0 * ctx.dt);
            if e.state_t > 0.8 {
                e.state = EState::Walk;
                e.state_t = 0.0;
            }
        }
        _ => {
            let aggro = ctx.player_alive && dist < d.aggro && same;
            if aggro {
                e.facing = dir;
                if dist < 210.0 && e.body.on_ground {
                    e.state = EState::Telegraph;
                    e.state_t = 0.0;
                } else {
                    e.body.vel.x = e.facing * d.speed;
                    e.state = EState::Walk;
                }
            } else {
                if !physics::ground_ahead(&e.body, e.facing, ctx.level) || e.body.hit_wall || e.state_t > 3.0 {
                    e.facing = -e.facing;
                    e.state_t = 0.0;
                }
                e.body.vel.x = e.facing * d.speed * 0.5;
                e.state = EState::Walk;
            }
        }
    }
}

fn float_ai(e: &mut Enemy, d: &EnemyDef, ctx: &Ctx, diver: bool, out: &mut Out) {
    let c = e.body.center();
    let (dist, dir, _) = to_player(e, ctx);
    let hover_y = e.home.y - d.hover - e.body.h * 0.5;
    let aggro = ctx.player_alive && dist < d.aggro && (ctx.player.y - c.y).abs() < 360.0;
    let bob = (e.state_t * 2.4 + e.id as f32).sin() * 22.0;
    let leash = 420.0;
    match e.state {
        EState::Telegraph => {
            e.body.vel = e.body.vel * (1.0 - 6.0 * ctx.dt).max(0.0);
            if e.state_t > d.telegraph {
                let to = ctx.player - c;
                let l = to.len().max(1.0);
                e.aim = to * (1.0 / l);
                e.state_t = 0.0;
                if diver {
                    e.state = EState::Lunge;
                } else {
                    let s = 16.0;
                    out.shots.push(Shot {
                        pos: c + e.aim * 18.0,
                        vel: e.aim * d.proj_speed,
                        size: s,
                        gravity: 0.0,
                        life: 2.6,
                        pierce: false,
                        damage: d.proj_damage,
                        // flyers' shots sting but don't knock you off ledges
                        kb: V2::ZERO,
                        style: d.proj_style,
                        grounded: false,
                    });
                    e.state = EState::Recover;
                    e.cooldown = d.fire_every;
                }
            }
        }
        EState::Lunge => {
            // dive along the aimed line
            e.body.vel = e.aim * d.burst;
            if e.state_t > 0.55 || e.body.hit_wall || e.body.on_ground {
                e.state = EState::Recover;
                e.state_t = 0.0;
                e.cooldown = d.fire_every;
            }
        }
        _ => {
            // hover: drift toward the player (or patrol home), bob up and down
            e.cooldown -= ctx.dt;
            let target_x = if aggro {
                let keep = if diver { 140.0 } else { 230.0 };
                (ctx.player.x - dir * keep).clamp(e.home.x - leash, e.home.x + leash)
            } else {
                e.home.x + (e.state_t * 0.5 + e.id as f32).sin() * 160.0
            };
            let ty = if aggro && !diver { (ctx.player.y - d.hover).min(hover_y + 40.0) } else { hover_y } + bob;
            let dx = target_x - c.x;
            let dy = ty - c.y;
            e.body.vel.x = approach(e.body.vel.x, (dx * 2.0).clamp(-d.speed, d.speed), 500.0 * ctx.dt);
            e.body.vel.y = approach(e.body.vel.y, (dy * 2.0).clamp(-d.speed, d.speed), 500.0 * ctx.dt);
            e.facing = if aggro { dir } else if e.body.vel.x.abs() > 5.0 { e.body.vel.x.signum() } else { e.facing };
            if e.state == EState::Recover && e.state_t > 0.5 {
                e.state = EState::Idle;
            }
            if aggro && e.cooldown <= 0.0 && e.state != EState::Recover {
                e.state = EState::Telegraph;
                e.state_t = 0.0;
            }
        }
    }
}

fn boss(e: &mut Enemy, d: &EnemyDef, ctx: &Ctx, out: &mut Out) {
    let (dist, dir, _) = to_player(e, ctx);
    let enraged = e.hp < e.max_hp * 0.5;
    if enraged {
        e.boss_phase2 = true;
    }
    let spd = if enraged { 1.25 } else { 1.0 };
    let dt = ctx.dt;
    let c = e.body.center();
    match e.state {
        EState::Telegraph => {
            e.body.vel.x = 0.0;
            if e.state_t > d.telegraph / spd {
                e.state = EState::Charge;
                e.state_t = 0.0;
                e.charge_dist = 0.0;
            }
        }
        EState::Charge => {
            e.body.vel.x = e.facing * d.burst * spd;
            e.charge_dist += (e.body.vel.x * dt).abs();
            if e.body.hit_wall {
                e.state = EState::Stunned;
                e.state_t = 0.0;
                out.shake = out.shake.max(9.0);
            } else if e.charge_dist > 900.0 {
                e.state = EState::Recover;
                e.state_t = 0.0;
            }
        }
        EState::Stunned => {
            e.body.vel.x = approach(e.body.vel.x, 0.0, 2200.0 * dt);
            if e.state_t > 1.8 {
                e.state = EState::Walk;
                e.state_t = 0.0;
                e.think = 0.6;
            }
        }
        EState::Recover => {
            e.body.vel.x = approach(e.body.vel.x, 0.0, 1600.0 * dt);
            if e.state_t > 0.9 / spd {
                e.state = EState::Walk;
                e.state_t = 0.0;
                e.think = 0.6;
            }
        }
        EState::Stomp => {
            if e.state_t < 0.05 && e.body.on_ground {
                e.body.vel.y = -640.0;
                e.body.on_ground = false;
                e.body.vel.x = dir * 130.0;
            } else if e.body.on_ground && e.state_t > 0.25 {
                for s in [-1.0f32, 1.0] {
                    out.shots.push(Shot {
                        pos: V2::new(c.x + s * 50.0, e.body.pos.y + e.body.h - 14.0),
                        vel: V2::new(s * 430.0, 0.0),
                        size: 30.0,
                        gravity: 0.0,
                        life: 1.5,
                        pierce: true,
                        damage: d.attack * 0.7,
                        kb: V2::new(320.0, -430.0),
                        style: ProjStyle::Shockwave,
                        grounded: true,
                    });
                }
                out.shake = out.shake.max(10.0);
                e.state = EState::Recover;
                e.state_t = 0.0;
                e.body.vel.x = 0.0;
            }
        }
        EState::Spit => {
            e.body.vel.x = 0.0;
            if e.state_t > 0.55 && !e.attack_hit {
                e.attack_hit = true;
                let n = if enraged { 5 } else { 3 };
                for i in 0..n {
                    let f = i as f32 / (n - 1) as f32;
                    out.shots.push(Shot {
                        pos: V2::new(c.x + e.facing * 50.0, e.body.pos.y + 20.0),
                        vel: V2::new(e.facing * (220.0 + f * 340.0), -510.0 + f * 100.0),
                        size: 18.0,
                        gravity: 980.0,
                        life: 2.4,
                        pierce: false,
                        damage: d.proj_damage,
                        kb: V2::new(220.0, -250.0),
                        style: d.proj_style,
                        grounded: false,
                    });
                }
            }
            if e.state_t > 1.1 {
                e.state = EState::Walk;
                e.state_t = 0.0;
                e.think = 0.8;
            }
        }
        _ => {
            e.think -= dt * spd;
            let aggro = ctx.player_alive && dist < d.aggro;
            if aggro {
                e.facing = dir;
                e.body.vel.x = if dist > 160.0 { dir * d.speed * spd } else { 0.0 };
                if e.think <= 0.0 && e.body.on_ground {
                    let roll = (e.state_t * 1000.0) as u32 % 3;
                    e.state = match (roll, dist < 260.0) {
                        (_, true) if roll != 2 => EState::Stomp,
                        (2, _) => EState::Spit,
                        _ => EState::Telegraph,
                    };
                    e.attack_hit = false;
                    e.state_t = 0.0;
                }
            } else {
                e.body.vel.x = approach(e.body.vel.x, 0.0, 600.0 * dt);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Aabb;

    /// Every monster can be spawned and simulated for a while next to a
    /// player without errors, and stays inside the world.
    #[test]
    fn all_monsters_behave() {
        let floor = [Aabb::new(0.0, 640.0, 3000.0, 300.0)];
        let lvl = Level { solids: &floor, platforms: &[], bounds: Aabb::new(0.0, -600.0, 3000.0, 1500.0) };
        let mut rng = Rng::new(3);
        for &k in ALL_KINDS {
            let mut e = Enemy::new(1, k, 1500.0, 640.0, None, &mut rng);
            let mut shots = 0;
            for i in 0..1200 {
                let player = V2::new(1300.0 + (i as f32 * 0.01).sin() * 200.0, 600.0);
                let ctx = Ctx { level: &lvl, player, player_alive: true, dt: 1.0 / 120.0 };
                let mut out = Out::default();
                update(&mut e, &ctx, &mut out);
                shots += out.shots.len();
            }
            assert!(e.body.pos.y < 700.0 && e.body.pos.x > -10.0, "{:?} escaped: {:?}", k, e.body.pos);
            let d = k.def();
            if matches!(d.ai, Ai::Floater { diver: false }) || d.ai == Ai::Boss {
                assert!(shots > 0, "{:?} never attacked at range", k);
            }
            if e.is_floater() {
                assert!(e.body.pos.y + e.body.h < 640.0 - 20.0 || e.state == EState::Lunge, "{:?} should hover", k);
            }
        }
    }
}
