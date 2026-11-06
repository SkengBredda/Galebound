//! Monsters: data-driven definitions and behaviour.
//!
//! Every monster uses one of a few behaviours (AI archetypes) with its own
//! numbers, so new zones add monsters by adding data, not code:
//! - Hopper: bounces around; hops at you when you get close.
//! - Lunger: patrols; puffs up (telegraph) then lunges.
//! - Floater: hovers; either shoots aimed projectiles or dive-bombs you.
//! - Spitter: walks along the ground and fires straight shots (0.14.0: the
//!   field's old flyers - flying monsters now live only in dungeons).
//! - Boss: charges (telegraphed), stomps shockwaves, spits volleys; enrages.
//!   The Tempest Vault bosses add their own arena mechanics on top (0.14.0):
//!   the Gatekeeper's shield and falling portcullis spikes, the Stormwarden's
//!   vortex and electrified floor.

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
    // The Tempest Vault (co-op dungeon, 0.13.0)
    Cryptling,
    Vaultbeak,
    Galewraith,
    Ruinstump,
    Vaultcrab,
    Gatekeeper,
    Stormwarden,
    // Tempest Vault flyers (0.14.0)
    Stormkite,
    VaultWisp,
    // 0.15.0
    Cactling,
    DuneScuttler,
    Sandwhirl,
    MesaScorpion,
    Thornback,
    DustDevil,
    Snowpuff,
    Rimeclaw,
    IceSprite,
    Frostling,
    GlacialYeti,
    RimeGolem,
    FrostWisp,
    CinderSalamander,
    MagmaCrab,
    AshenStump,
    EmberGolem,
    LavaSalamander,
    FlameScorpion,
    ObsidianGolem,
    Stormbeak,
    CloudRam,
    ZephyrWyrmling,
    ThunderGolem,
    StarWyrmling,
    VoidScorpion,
    AstralGolem,
    FrostBat,
    IceWisp,
    FireBat,
    EmberWisp,
    StarBat,
    SeraphWisp,
    FrostCrystal,
    RimeColossus,
    Glacia,
    Magmaw,
    Ignaroth,
    Sol,
    Luna,
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
    EnemyKind::Cryptling,
    EnemyKind::Vaultbeak,
    EnemyKind::Galewraith,
    EnemyKind::Ruinstump,
    EnemyKind::Vaultcrab,
    EnemyKind::Gatekeeper,
    EnemyKind::Stormwarden,
    EnemyKind::Stormkite,
    EnemyKind::VaultWisp,
    EnemyKind::Cactling,
    EnemyKind::DuneScuttler,
    EnemyKind::Sandwhirl,
    EnemyKind::MesaScorpion,
    EnemyKind::Thornback,
    EnemyKind::DustDevil,
    EnemyKind::Snowpuff,
    EnemyKind::Rimeclaw,
    EnemyKind::IceSprite,
    EnemyKind::Frostling,
    EnemyKind::GlacialYeti,
    EnemyKind::RimeGolem,
    EnemyKind::FrostWisp,
    EnemyKind::CinderSalamander,
    EnemyKind::MagmaCrab,
    EnemyKind::AshenStump,
    EnemyKind::EmberGolem,
    EnemyKind::LavaSalamander,
    EnemyKind::FlameScorpion,
    EnemyKind::ObsidianGolem,
    EnemyKind::Stormbeak,
    EnemyKind::CloudRam,
    EnemyKind::ZephyrWyrmling,
    EnemyKind::ThunderGolem,
    EnemyKind::StarWyrmling,
    EnemyKind::VoidScorpion,
    EnemyKind::AstralGolem,
    EnemyKind::FrostBat,
    EnemyKind::IceWisp,
    EnemyKind::FireBat,
    EnemyKind::EmberWisp,
    EnemyKind::StarBat,
    EnemyKind::SeraphWisp,
    EnemyKind::FrostCrystal,
    EnemyKind::RimeColossus,
    EnemyKind::Glacia,
    EnemyKind::Magmaw,
    EnemyKind::Ignaroth,
    EnemyKind::Sol,
    EnemyKind::Luna,
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ai {
    Dummy,
    Hopper,
    Lunger,
    /// `diver` = dive-bombs instead of shooting.
    Floater { diver: bool },
    /// Walks the ground and fires straight shots at you.
    Spitter,
    Boss,
    /// A flying boss (Glacia, Luna; see `boss_ai`).
    SkyBoss,
}

impl Ai {
    pub fn is_floater(&self) -> bool {
        matches!(self, Ai::Floater { .. } | Ai::SkyBoss)
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
                name: "Gustling", w: 34.0, h: 34.0, hp: 60.0, contact: 8.0, exp: 28, gold: 6, level: 7, weight: 0.7, ai: Ai::Spitter, speed: 90.0, aggro: 520.0,
                telegraph: 0.6, fire_every: 2.8, proj_speed: 260.0, proj_damage: 14.0, proj_style: ProjStyle::Orb, hover: 120.0, ..BASE
            },
            Gravecap => EnemyDef { name: "Gravecap", w: 38.0, h: 38.0, hp: 140.0, contact: 18.0, exp: 40, gold: 8, level: 9, weight: 1.1, speed: 150.0, burst: 430.0, aggro: 420.0, ..BASE },
            Lanternwisp => EnemyDef {
                name: "Lanternwisp", w: 30.0, h: 40.0, hp: 110.0, contact: 14.0, exp: 48, gold: 9, level: 10, weight: 0.7, ai: Ai::Spitter, speed: 80.0, aggro: 560.0,
                telegraph: 0.7, fire_every: 2.4, proj_speed: 240.0, proj_damage: 20.0, proj_style: ProjStyle::Star, hover: 130.0, ..BASE
            },
            Rotstump => EnemyDef {
                name: "Rotstump", w: 46.0, h: 54.0, hp: 220.0, contact: 22.0, attack: 30.0, exp: 60, gold: 12, level: 11, weight: 1.8, ai: Ai::Lunger, speed: 80.0, burst: 430.0, aggro: 460.0, telegraph: 0.75, ..BASE
            },
            CragCrab => EnemyDef {
                name: "Crag Crab", w: 50.0, h: 36.0, hp: 300.0, contact: 26.0, attack: 34.0, exp: 75, gold: 15, level: 13, weight: 2.0, ai: Ai::Lunger, speed: 110.0, burst: 560.0, aggro: 480.0, telegraph: 0.6, ..BASE
            },
            Galehawk => EnemyDef {
                name: "Galehawk", w: 44.0, h: 30.0, hp: 240.0, contact: 20.0, attack: 36.0, exp: 85, gold: 16, level: 14, weight: 0.9, ai: Ai::Lunger, speed: 130.0, burst: 560.0,
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
            Gloomwisp => EnemyDef { name: "Gloomwisp", w: 30.0, h: 40.0, hp: 300.0, contact: 24.0, exp: 90, gold: 14, level: 13, weight: 0.7, ai: Ai::Spitter, speed: 85.0, aggro: 560.0, telegraph: 0.65, fire_every: 2.2, proj_speed: 260.0, proj_damage: 32.0, proj_style: ProjStyle::Star, hover: 130.0, ..BASE },
            MireStump => EnemyDef { name: "Mire Stump", w: 46.0, h: 54.0, hp: 700.0, contact: 38.0, attack: 52.0, exp: 165, gold: 28, level: 20, weight: 2.0, ai: Ai::Lunger, speed: 85.0, burst: 450.0, aggro: 460.0, telegraph: 0.7, ..BASE },
            Stormwool => EnemyDef { name: "Stormwool", w: 40.0, h: 34.0, hp: 270.0, contact: 24.0, exp: 68, gold: 12, level: 10, weight: 1.2, speed: 140.0, burst: 400.0, aggro: 380.0, ..BASE },
            Squallsprite => EnemyDef { name: "Squallsprite", w: 34.0, h: 34.0, hp: 260.0, contact: 22.0, exp: 82, gold: 13, level: 12, weight: 0.7, ai: Ai::Spitter, speed: 95.0, aggro: 520.0, telegraph: 0.55, fire_every: 2.4, proj_speed: 300.0, proj_damage: 28.0, proj_style: ProjStyle::Orb, hover: 120.0, ..BASE },
            Thunderhawk => EnemyDef { name: "Thunderhawk", w: 44.0, h: 30.0, hp: 600.0, contact: 34.0, attack: 54.0, exp: 175, gold: 30, level: 21, weight: 1.0, ai: Ai::Lunger, speed: 140.0, burst: 600.0, aggro: 580.0, telegraph: 0.5, fire_every: 2.0, hover: 150.0, ..BASE },
            Thornling => EnemyDef { name: "Thornling", w: 36.0, h: 28.0, hp: 250.0, contact: 26.0, exp: 68, gold: 12, level: 10, weight: 0.9, speed: 185.0, burst: 500.0, aggro: 460.0, ..BASE },
            Briarbeak => EnemyDef { name: "Briarbeak", w: 34.0, h: 44.0, hp: 320.0, contact: 26.0, attack: 36.0, exp: 84, gold: 14, level: 12, ai: Ai::Lunger, speed: 125.0, burst: 560.0, aggro: 500.0, telegraph: 0.5, ..BASE },
            Barkstump => EnemyDef { name: "Barkstump", w: 46.0, h: 54.0, hp: 640.0, contact: 36.0, attack: 50.0, exp: 150, gold: 26, level: 18, weight: 2.0, ai: Ai::Lunger, speed: 85.0, burst: 450.0, aggro: 460.0, telegraph: 0.7, ..BASE },
            Starwool => EnemyDef { name: "Starwool", w: 40.0, h: 34.0, hp: 330.0, contact: 28.0, exp: 90, gold: 15, level: 13, weight: 1.2, speed: 140.0, burst: 420.0, aggro: 380.0, ..BASE },
            Voidgust => EnemyDef { name: "Voidgust", w: 34.0, h: 34.0, hp: 560.0, contact: 34.0, exp: 180, gold: 30, level: 22, weight: 0.8, ai: Ai::Spitter, speed: 100.0, aggro: 560.0, telegraph: 0.5, fire_every: 2.0, proj_speed: 320.0, proj_damage: 46.0, proj_style: ProjStyle::Bolt, hover: 125.0, ..BASE },
            Cryptling => EnemyDef { name: "Cryptling", w: 38.0, h: 30.0, hp: 1100.0, contact: 44.0, exp: 260, gold: 30, level: 22, weight: 1.3, speed: 190.0, burst: 500.0, aggro: 520.0, ..BASE },
            Vaultbeak => EnemyDef { name: "Vaultbeak", w: 34.0, h: 44.0, hp: 1400.0, contact: 46.0, attack: 66.0, exp: 300, gold: 34, level: 23, ai: Ai::Lunger, speed: 130.0, burst: 600.0, aggro: 560.0, telegraph: 0.5, ..BASE },
            Galewraith => EnemyDef {
                name: "Galewraith", w: 32.0, h: 42.0, hp: 1000.0, contact: 40.0, exp: 290, gold: 32, level: 23, weight: 0.8, ai: Ai::Floater { diver: false }, speed: 95.0, aggro: 600.0,
                telegraph: 0.5, fire_every: 1.9, proj_speed: 330.0, proj_damage: 52.0, proj_style: ProjStyle::Bolt, hover: 120.0, ..BASE
            },
            Ruinstump => EnemyDef { name: "Ruinstump", w: 48.0, h: 56.0, hp: 2000.0, contact: 52.0, attack: 72.0, exp: 360, gold: 40, level: 24, weight: 2.4, ai: Ai::Lunger, speed: 90.0, burst: 470.0, aggro: 500.0, telegraph: 0.65, ..BASE },
            Vaultcrab => EnemyDef { name: "Vaultcrab", w: 52.0, h: 38.0, hp: 2200.0, contact: 54.0, attack: 74.0, exp: 380, gold: 42, level: 24, weight: 2.6, ai: Ai::Lunger, speed: 125.0, burst: 620.0, aggro: 520.0, telegraph: 0.55, ..BASE },
            Gatekeeper => EnemyDef {
                name: "The Gatekeeper", w: 100.0, h: 80.0, hp: 9000.0, contact: 50.0, attack: 70.0, exp: 1800, gold: 300, level: 24, weight: 3.6, ai: Ai::Boss, speed: 100.0, burst: 650.0,
                aggro: 900.0, telegraph: 0.7, proj_damage: 40.0, proj_style: ProjStyle::Caltrop, boss: true, ..BASE
            },
            Stormwarden => EnemyDef {
                name: "The Stormwarden", w: 112.0, h: 88.0, hp: 20000.0, contact: 58.0, attack: 82.0, exp: 5000, gold: 800, level: 25, weight: 4.0, ai: Ai::Boss, speed: 115.0, burst: 720.0,
                aggro: 1100.0, telegraph: 0.6, proj_damage: 50.0, proj_style: ProjStyle::Bolt, boss: true, ..BASE
            },
            Stormkite => EnemyDef {
                name: "Stormkite", w: 44.0, h: 30.0, hp: 1200.0, contact: 44.0, attack: 70.0, exp: 300, gold: 34, level: 23, weight: 0.9, ai: Ai::Floater { diver: true }, speed: 140.0, burst: 640.0,
                aggro: 600.0, telegraph: 0.5, fire_every: 2.0, hover: 150.0, ..BASE
            },
            VaultWisp => EnemyDef {
                name: "Vault Wisp", w: 30.0, h: 40.0, hp: 950.0, contact: 40.0, exp: 280, gold: 30, level: 22, weight: 0.7, ai: Ai::Floater { diver: false }, speed: 85.0, aggro: 600.0,
                telegraph: 0.55, fire_every: 2.1, proj_speed: 300.0, proj_damage: 48.0, proj_style: ProjStyle::Star, hover: 130.0, ..BASE
            },
            Cactling => EnemyDef { name: "Cactling", w: 36.0, h: 38.0, hp: 290.0, contact: 26.0, attack: 30.0, exp: 76, gold: 13, level: 11, weight: 1.1, ai: Ai::Hopper, speed: 150.0, burst: 470.0, aggro: 420.0, ..BASE },
            DuneScuttler => EnemyDef { name: "Dune Scuttler", w: 44.0, h: 28.0, hp: 330.0, contact: 26.0, attack: 36.0, exp: 86, gold: 14, level: 12, weight: 1.4, ai: Ai::Lunger, speed: 140.0, burst: 560.0, aggro: 480.0, telegraph: 0.55, ..BASE },
            Sandwhirl => EnemyDef { name: "Sandwhirl", w: 34.0, h: 34.0, hp: 280.0, contact: 22.0, attack: 24.0, exp: 84, gold: 14, level: 12, weight: 0.7, ai: Ai::Spitter, speed: 95.0, aggro: 520.0, telegraph: 0.55, fire_every: 2.4, proj_speed: 300.0, proj_damage: 30.0, proj_style: ProjStyle::Orb, hover: 120.0, ..BASE },
            MesaScorpion => EnemyDef { name: "Mesa Scorpion", w: 52.0, h: 32.0, hp: 660.0, contact: 36.0, attack: 50.0, exp: 158, gold: 27, level: 19, weight: 2.0, ai: Ai::Lunger, speed: 140.0, burst: 600.0, aggro: 500.0, telegraph: 0.55, ..BASE },
            Thornback => EnemyDef { name: "Thornback Cactling", w: 40.0, h: 42.0, hp: 580.0, contact: 34.0, attack: 40.0, exp: 145, gold: 25, level: 18, weight: 1.4, ai: Ai::Hopper, speed: 150.0, burst: 500.0, aggro: 440.0, ..BASE },
            DustDevil => EnemyDef { name: "Dust Devil", w: 36.0, h: 36.0, hp: 600.0, contact: 32.0, attack: 30.0, exp: 170, gold: 29, level: 20, weight: 0.8, ai: Ai::Spitter, speed: 100.0, aggro: 560.0, telegraph: 0.5, fire_every: 2.1, proj_speed: 330.0, proj_damage: 46.0, proj_style: ProjStyle::Orb, hover: 120.0, ..BASE },
            Snowpuff => EnemyDef { name: "Snowpuff", w: 42.0, h: 36.0, hp: 1350.0, contact: 52.0, attack: 60.0, exp: 290, gold: 48, level: 30, weight: 1.3, ai: Ai::Hopper, speed: 140.0, burst: 440.0, aggro: 400.0, ..BASE },
            Rimeclaw => EnemyDef { name: "Rimeclaw", w: 50.0, h: 36.0, hp: 1500.0, contact: 54.0, attack: 82.0, exp: 310, gold: 50, level: 31, weight: 2.2, ai: Ai::Lunger, speed: 120.0, burst: 600.0, aggro: 500.0, telegraph: 0.55, ..BASE },
            IceSprite => EnemyDef { name: "Ice Sprite", w: 34.0, h: 34.0, hp: 1350.0, contact: 48.0, attack: 50.0, exp: 315, gold: 51, level: 32, weight: 0.7, ai: Ai::Spitter, speed: 95.0, aggro: 560.0, telegraph: 0.5, fire_every: 2.2, proj_speed: 330.0, proj_damage: 70.0, proj_style: ProjStyle::Bolt, hover: 120.0, ..BASE },
            Frostling => EnemyDef { name: "Frostling", w: 44.0, h: 52.0, hp: 1700.0, contact: 56.0, attack: 88.0, exp: 330, gold: 54, level: 33, weight: 1.8, ai: Ai::Lunger, speed: 130.0, burst: 580.0, aggro: 500.0, telegraph: 0.6, ..BASE },
            GlacialYeti => EnemyDef { name: "Glacial Yeti", w: 54.0, h: 64.0, hp: 2100.0, contact: 62.0, attack: 98.0, exp: 380, gold: 60, level: 37, weight: 2.6, ai: Ai::Lunger, speed: 125.0, burst: 600.0, aggro: 520.0, telegraph: 0.6, ..BASE },
            RimeGolem => EnemyDef { name: "Rime Golem", w: 56.0, h: 66.0, hp: 2400.0, contact: 64.0, attack: 102.0, exp: 395, gold: 62, level: 38, weight: 3.0, ai: Ai::Lunger, speed: 90.0, burst: 520.0, aggro: 520.0, telegraph: 0.7, ..BASE },
            FrostWisp => EnemyDef { name: "Frost Wisp", w: 30.0, h: 40.0, hp: 1900.0, contact: 56.0, attack: 60.0, exp: 370, gold: 58, level: 36, weight: 0.7, ai: Ai::Spitter, speed: 90.0, aggro: 580.0, telegraph: 0.5, fire_every: 2.0, proj_speed: 340.0, proj_damage: 82.0, proj_style: ProjStyle::Star, hover: 120.0, ..BASE },
            CinderSalamander => EnemyDef { name: "Cinder Salamander", w: 50.0, h: 30.0, hp: 2300.0, contact: 66.0, attack: 80.0, exp: 425, gold: 68, level: 41, weight: 1.4, ai: Ai::Spitter, speed: 110.0, aggro: 580.0, telegraph: 0.5, fire_every: 2.0, proj_speed: 360.0, proj_damage: 95.0, proj_style: ProjStyle::Fireball, hover: 120.0, ..BASE },
            MagmaCrab => EnemyDef { name: "Magma Crab", w: 52.0, h: 38.0, hp: 2600.0, contact: 70.0, attack: 110.0, exp: 440, gold: 70, level: 42, weight: 2.6, ai: Ai::Lunger, speed: 120.0, burst: 620.0, aggro: 520.0, telegraph: 0.55, ..BASE },
            AshenStump => EnemyDef { name: "Ashen Stump", w: 48.0, h: 56.0, hp: 2800.0, contact: 72.0, attack: 112.0, exp: 450, gold: 72, level: 43, weight: 2.4, ai: Ai::Lunger, speed: 90.0, burst: 480.0, aggro: 500.0, telegraph: 0.65, ..BASE },
            EmberGolem => EnemyDef { name: "Ember Golem", w: 56.0, h: 66.0, hp: 3000.0, contact: 74.0, attack: 116.0, exp: 465, gold: 74, level: 44, weight: 3.0, ai: Ai::Lunger, speed: 90.0, burst: 520.0, aggro: 520.0, telegraph: 0.7, ..BASE },
            LavaSalamander => EnemyDef { name: "Lava Salamander", w: 54.0, h: 32.0, hp: 3100.0, contact: 78.0, attack: 90.0, exp: 505, gold: 80, level: 47, weight: 1.5, ai: Ai::Spitter, speed: 110.0, aggro: 600.0, telegraph: 0.45, fire_every: 1.9, proj_speed: 380.0, proj_damage: 112.0, proj_style: ProjStyle::Fireball, hover: 120.0, ..BASE },
            FlameScorpion => EnemyDef { name: "Flame Scorpion", w: 54.0, h: 34.0, hp: 3300.0, contact: 80.0, attack: 128.0, exp: 515, gold: 82, level: 48, weight: 2.2, ai: Ai::Lunger, speed: 150.0, burst: 640.0, aggro: 520.0, telegraph: 0.5, ..BASE },
            ObsidianGolem => EnemyDef { name: "Obsidian Golem", w: 58.0, h: 68.0, hp: 3600.0, contact: 84.0, attack: 132.0, exp: 530, gold: 84, level: 49, weight: 3.2, ai: Ai::Lunger, speed: 90.0, burst: 520.0, aggro: 520.0, telegraph: 0.7, ..BASE },
            Stormbeak => EnemyDef { name: "Stormbeak", w: 36.0, h: 46.0, hp: 3500.0, contact: 84.0, attack: 135.0, exp: 545, gold: 86, level: 51, weight: 1.4, ai: Ai::Lunger, speed: 140.0, burst: 620.0, aggro: 540.0, telegraph: 0.5, ..BASE },
            CloudRam => EnemyDef { name: "Cloud Ram", w: 46.0, h: 40.0, hp: 3600.0, contact: 95.0, attack: 110.0, exp: 555, gold: 88, level: 52, weight: 1.8, ai: Ai::Hopper, speed: 150.0, burst: 520.0, aggro: 440.0, ..BASE },
            ZephyrWyrmling => EnemyDef { name: "Zephyr Wyrmling", w: 54.0, h: 32.0, hp: 3500.0, contact: 86.0, attack: 90.0, exp: 565, gold: 90, level: 53, weight: 1.4, ai: Ai::Spitter, speed: 120.0, aggro: 620.0, telegraph: 0.45, fire_every: 1.8, proj_speed: 400.0, proj_damage: 125.0, proj_style: ProjStyle::Bolt, hover: 120.0, ..BASE },
            ThunderGolem => EnemyDef { name: "Thunder Golem", w: 58.0, h: 68.0, hp: 4100.0, contact: 92.0, attack: 145.0, exp: 580, gold: 92, level: 54, weight: 3.2, ai: Ai::Lunger, speed: 95.0, burst: 540.0, aggro: 540.0, telegraph: 0.7, ..BASE },
            StarWyrmling => EnemyDef { name: "Star Wyrmling", w: 54.0, h: 32.0, hp: 4100.0, contact: 94.0, attack: 96.0, exp: 610, gold: 96, level: 56, weight: 1.4, ai: Ai::Spitter, speed: 120.0, aggro: 640.0, telegraph: 0.45, fire_every: 1.7, proj_speed: 420.0, proj_damage: 138.0, proj_style: ProjStyle::Star, hover: 120.0, ..BASE },
            VoidScorpion => EnemyDef { name: "Void Scorpion", w: 54.0, h: 34.0, hp: 4400.0, contact: 98.0, attack: 155.0, exp: 625, gold: 98, level: 57, weight: 2.4, ai: Ai::Lunger, speed: 155.0, burst: 660.0, aggro: 540.0, telegraph: 0.5, ..BASE },
            AstralGolem => EnemyDef { name: "Astral Golem", w: 58.0, h: 68.0, hp: 4800.0, contact: 102.0, attack: 160.0, exp: 640, gold: 100, level: 58, weight: 3.4, ai: Ai::Lunger, speed: 95.0, burst: 540.0, aggro: 540.0, telegraph: 0.7, ..BASE },
            FrostBat => EnemyDef { name: "Frost Bat", w: 44.0, h: 30.0, hp: 2000.0, contact: 58.0, attack: 96.0, exp: 380, gold: 60, level: 36, weight: 0.9, ai: Ai::Floater { diver: true }, speed: 140.0, burst: 640.0, aggro: 600.0, telegraph: 0.5, fire_every: 2.0, proj_speed: 0.0, proj_damage: 0.0, proj_style: ProjStyle::Orb, hover: 150.0, ..BASE },
            IceWisp => EnemyDef { name: "Ice Wisp", w: 30.0, h: 40.0, hp: 1900.0, contact: 56.0, attack: 60.0, exp: 375, gold: 60, level: 36, weight: 0.7, ai: Ai::Floater { diver: false }, speed: 85.0, burst: 0.0, aggro: 620.0, telegraph: 0.5, fire_every: 2.0, proj_speed: 340.0, proj_damage: 80.0, proj_style: ProjStyle::Bolt, hover: 130.0, ..BASE },
            FireBat => EnemyDef { name: "Fire Bat", w: 44.0, h: 30.0, hp: 3100.0, contact: 78.0, attack: 124.0, exp: 500, gold: 80, level: 46, weight: 0.9, ai: Ai::Floater { diver: true }, speed: 150.0, burst: 660.0, aggro: 620.0, telegraph: 0.5, fire_every: 1.9, proj_speed: 0.0, proj_damage: 0.0, proj_style: ProjStyle::Orb, hover: 150.0, ..BASE },
            EmberWisp => EnemyDef { name: "Ember Wisp", w: 30.0, h: 40.0, hp: 2900.0, contact: 76.0, attack: 80.0, exp: 490, gold: 78, level: 46, weight: 0.7, ai: Ai::Floater { diver: false }, speed: 90.0, burst: 0.0, aggro: 620.0, telegraph: 0.45, fire_every: 1.9, proj_speed: 360.0, proj_damage: 104.0, proj_style: ProjStyle::Fireball, hover: 130.0, ..BASE },
            StarBat => EnemyDef { name: "Star Bat", w: 44.0, h: 30.0, hp: 4300.0, contact: 98.0, attack: 150.0, exp: 620, gold: 98, level: 56, weight: 0.9, ai: Ai::Floater { diver: true }, speed: 160.0, burst: 680.0, aggro: 640.0, telegraph: 0.45, fire_every: 1.8, proj_speed: 0.0, proj_damage: 0.0, proj_style: ProjStyle::Orb, hover: 150.0, ..BASE },
            SeraphWisp => EnemyDef { name: "Seraph Wisp", w: 30.0, h: 40.0, hp: 4100.0, contact: 96.0, attack: 96.0, exp: 615, gold: 96, level: 56, weight: 0.7, ai: Ai::Floater { diver: false }, speed: 95.0, burst: 0.0, aggro: 640.0, telegraph: 0.45, fire_every: 1.8, proj_speed: 380.0, proj_damage: 132.0, proj_style: ProjStyle::Star, hover: 130.0, ..BASE },
            FrostCrystal => EnemyDef { name: "Frost Crystal", w: 40.0, h: 60.0, hp: 2600.0, contact: 0.0, attack: 0.0, exp: 0, gold: 0, level: 40, weight: 99.0, ai: Ai::Dummy, ..BASE },
            RimeColossus => EnemyDef { name: "The Rime Colossus", w: 110.0, h: 104.0, hp: 26000.0, contact: 70.0, attack: 120.0, exp: 6000, gold: 900, level: 38, weight: 4.2, ai: Ai::Boss, speed: 95.0, burst: 650.0, aggro: 1000.0, telegraph: 0.7, proj_damage: 80.0, proj_speed: 320.0, proj_style: ProjStyle::Icicle, hover: 110.0, boss: true, ..BASE },
            Glacia => EnemyDef { name: "Glacia, the Frost Wyrm", w: 130.0, h: 70.0, hp: 52000.0, contact: 74.0, attack: 130.0, exp: 14000, gold: 2000, level: 40, weight: 4.0, ai: Ai::SkyBoss, speed: 120.0, burst: 0.0, aggro: 1200.0, telegraph: 0.6, proj_damage: 92.0, proj_speed: 380.0, proj_style: ProjStyle::Icicle, hover: 230.0, boss: true, ..BASE },
            Magmaw => EnemyDef { name: "Magmaw", w: 116.0, h: 104.0, hp: 44000.0, contact: 90.0, attack: 150.0, exp: 10000, gold: 1500, level: 48, weight: 4.4, ai: Ai::Boss, speed: 100.0, burst: 680.0, aggro: 1000.0, telegraph: 0.65, proj_damage: 110.0, proj_speed: 340.0, proj_style: ProjStyle::Fireball, hover: 110.0, boss: true, ..BASE },
            Ignaroth => EnemyDef { name: "Ignaroth, the Cinder Dragon", w: 150.0, h: 84.0, hp: 82000.0, contact: 96.0, attack: 165.0, exp: 24000, gold: 3500, level: 50, weight: 4.8, ai: Ai::Boss, speed: 110.0, burst: 720.0, aggro: 1200.0, telegraph: 0.6, proj_damage: 120.0, proj_speed: 380.0, proj_style: ProjStyle::Flame, hover: 110.0, boss: true, ..BASE },
            Sol => EnemyDef { name: "Sol, the Dawn Twin", w: 80.0, h: 96.0, hp: 70000.0, contact: 104.0, attack: 175.0, exp: 20000, gold: 3000, level: 58, weight: 4.0, ai: Ai::Boss, speed: 120.0, burst: 760.0, aggro: 1200.0, telegraph: 0.6, proj_damage: 140.0, proj_speed: 400.0, proj_style: ProjStyle::Star, hover: 110.0, boss: true, ..BASE },
            Luna => EnemyDef { name: "Luna, the Dusk Twin", w: 80.0, h: 96.0, hp: 60000.0, contact: 98.0, attack: 160.0, exp: 20000, gold: 3000, level: 58, weight: 3.6, ai: Ai::SkyBoss, speed: 130.0, burst: 0.0, aggro: 1200.0, telegraph: 0.6, proj_damage: 132.0, proj_speed: 400.0, proj_style: ProjStyle::Star, hover: 210.0, boss: true, ..BASE },
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
            Cryptling => "cryptling",
            Vaultbeak => "vaultbeak",
            Galewraith => "galewraith",
            Ruinstump => "ruinstump",
            Vaultcrab => "vaultcrab",
            Gatekeeper => "gatekeeper",
            Stormwarden => "stormwarden",
            Stormkite => "stormkite",
            VaultWisp => "vaultwisp",
            Cactling => "cactling",
            DuneScuttler => "dunescuttler",
            Sandwhirl => "sandwhirl",
            MesaScorpion => "mesascorpion",
            Thornback => "thornback",
            DustDevil => "dustdevil",
            Snowpuff => "snowpuff",
            Rimeclaw => "rimeclaw",
            IceSprite => "icesprite",
            Frostling => "frostling",
            GlacialYeti => "glacialyeti",
            RimeGolem => "rimegolem",
            FrostWisp => "frostwisp",
            CinderSalamander => "cindersalamander",
            MagmaCrab => "magmacrab",
            AshenStump => "ashenstump",
            EmberGolem => "embergolem",
            LavaSalamander => "lavasalamander",
            FlameScorpion => "flamescorpion",
            ObsidianGolem => "obsidiangolem",
            Stormbeak => "stormbeak",
            CloudRam => "cloudram",
            ZephyrWyrmling => "zephyrwyrmling",
            ThunderGolem => "thundergolem",
            StarWyrmling => "starwyrmling",
            VoidScorpion => "voidscorpion",
            AstralGolem => "astralgolem",
            FrostBat => "frostbat",
            IceWisp => "icewisp",
            FireBat => "firebat",
            EmberWisp => "emberwisp",
            StarBat => "starbat",
            SeraphWisp => "seraphwisp",
            FrostCrystal => "frostcrystal",
            RimeColossus => "rimecolossus",
            Glacia => "glacia",
            Magmaw => "magmaw",
            Ignaroth => "ignaroth",
            Sol => "sol",
            Luna => "luna",
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
            Cryptling => Mossling,
            Vaultbeak => Burrowbeak,
            Galewraith => Lanternwisp,
            Ruinstump => Rotstump,
            Vaultcrab => CragCrab,
            Gatekeeper => Bramblehorn,
            Stormwarden => Thunderhorn,
            Stormkite => Galehawk,
            VaultWisp => Lanternwisp,
            Sandwhirl => Gustling,
            MesaScorpion => DuneScuttler,
            Thornback => Cactling,
            DustDevil => Gustling,
            Snowpuff => Puffwool,
            Rimeclaw => CragCrab,
            IceSprite => Gustling,
            GlacialYeti => Frostling,
            FrostWisp => Lanternwisp,
            MagmaCrab => CragCrab,
            AshenStump => Rotstump,
            EmberGolem => RimeGolem,
            LavaSalamander => CinderSalamander,
            FlameScorpion => DuneScuttler,
            ObsidianGolem => RimeGolem,
            Stormbeak => Burrowbeak,
            CloudRam => Puffwool,
            ZephyrWyrmling => CinderSalamander,
            ThunderGolem => RimeGolem,
            StarWyrmling => CinderSalamander,
            VoidScorpion => DuneScuttler,
            AstralGolem => RimeGolem,
            IceWisp => Lanternwisp,
            FireBat => FrostBat,
            EmberWisp => Lanternwisp,
            StarBat => FrostBat,
            SeraphWisp => Lanternwisp,
            Magmaw => RimeColossus,
            Ignaroth => Glacia,
            Luna => Sol,
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
            Cryptling => [0.55, 0.85, 1.0],
            Vaultbeak => [0.75, 0.7, 1.0],
            Galewraith => [0.5, 0.95, 1.0],
            Ruinstump => [0.6, 0.6, 0.85],
            Vaultcrab => [0.55, 0.8, 1.0],
            Gatekeeper => [1.0, 0.85, 0.4],
            Stormwarden => [1.0, 0.45, 0.42],
            Stormkite => [0.6, 0.78, 1.0],
            VaultWisp => [0.82, 0.6, 1.0],
            Sandwhirl => [1.0, 0.86, 0.6],
            MesaScorpion => [1.0, 0.7, 0.5],
            Thornback => [0.7, 0.85, 0.6],
            DustDevil => [0.95, 0.75, 0.5],
            Snowpuff => [0.85, 0.95, 1.1],
            Rimeclaw => [0.7, 0.9, 1.15],
            IceSprite => [0.75, 0.95, 1.2],
            GlacialYeti => [0.8, 0.9, 1.15],
            FrostWisp => [0.7, 0.9, 1.2],
            MagmaCrab => [1.2, 0.6, 0.4],
            AshenStump => [0.6, 0.55, 0.55],
            EmberGolem => [1.25, 0.6, 0.4],
            LavaSalamander => [1.2, 0.75, 0.5],
            FlameScorpion => [1.3, 0.55, 0.35],
            ObsidianGolem => [0.45, 0.4, 0.55],
            Stormbeak => [0.7, 0.85, 1.25],
            CloudRam => [1.05, 1.05, 1.15],
            ZephyrWyrmling => [0.6, 1.0, 1.2],
            ThunderGolem => [1.2, 1.1, 0.6],
            StarWyrmling => [0.85, 0.7, 1.3],
            VoidScorpion => [0.55, 0.45, 0.9],
            AstralGolem => [0.8, 0.75, 1.3],
            IceWisp => [0.65, 0.95, 1.25],
            FireBat => [1.3, 0.6, 0.4],
            EmberWisp => [1.3, 0.7, 0.45],
            StarBat => [0.9, 0.75, 1.35],
            SeraphWisp => [1.2, 1.1, 0.7],
            Magmaw => [1.3, 0.55, 0.35],
            Ignaroth => [1.35, 0.55, 0.35],
            Luna => [0.7, 0.75, 1.25],

            _ => [1.0, 1.0, 1.0],
        }
    }
    /// What it can drop: (item id, chance 0-1). Rolled independently.
    pub fn drops(self) -> &'static [(&'static str, f32)] {
        use EnemyKind::*;
        match self {
            TrainingDummy => &[],
            Mossling => &[("moss_clump", 0.5), ("sturdy_branch", 0.35), ("wild_herbs", 0.25), ("hp_potion", 0.06)],
            Burrowbeak => &[("beak_shard", 0.45), ("speckled_egg", 0.3), ("game_meat", 0.15), ("hp_potion", 0.08), ("sandals", 0.02), ("leather_cap", 0.01)],
            Bramblehorn => &[("bramble_thorn", 1.0), ("iron_ore", 1.0), ("wood_sword", 0.5), ("copper_ring", 0.5)],
            Puffwool => &[("wool_tuft", 0.5), ("game_meat", 0.35), ("mp_potion", 0.08), ("cloth_tunic", 0.02), ("leather_breeches", 0.015)],
            Gustling => &[("gust_wisp", 0.45), ("sky_honey", 0.15), ("mp_potion", 0.1), ("travel_cloak", 0.01)],
            Gravecap => &[("cap_spore", 0.45), ("glowcap", 0.3), ("hp_potion", 0.08), ("leather_boots", 0.015), ("fang_necklace", 0.008)],
            Lanternwisp => &[("wisp_ember", 0.45), ("mp_potion", 0.1), ("sage_pendant", 0.01), ("brass_hoops", 0.008)],
            Rotstump => &[("rotten_bark", 0.45), ("sturdy_branch", 0.5), ("hp_potion_big", 0.05), ("leather_vest", 0.015), ("silver_ring", 0.008)],
            CragCrab => &[("crab_shell", 0.45), ("crab_meat", 0.35), ("iron_ore", 0.15), ("hp_potion_big", 0.06), ("power_band", 0.01), ("iron_helm", 0.006)],
            Galehawk => &[("hawk_feather", 0.45), ("speckled_egg", 0.25), ("mp_potion_big", 0.05), ("windstep_boots", 0.01), ("gale_cape", 0.006)],
            Thunderhorn => &[("storm_horn", 1.0), ("storm_amulet", 0.25), ("elixir", 1.0), ("gale_ring", 0.15), ("gale_circlet", 0.1)],
            Rockmoss | Thornling => &[("moss_clump", 0.6), ("sturdy_branch", 0.3), ("wild_herbs", 0.3), ("iron_ore", 0.15), ("hp_potion", 0.1)],
            Ironbeak | Briarbeak => &[("beak_shard", 0.6), ("speckled_egg", 0.3), ("game_meat", 0.15), ("iron_ore", 0.2), ("hp_potion_big", 0.04), ("chain_shirt", 0.008)],
            Shadecap => &[("cap_spore", 0.6), ("glowcap", 0.35), ("shadow_silk", 0.15), ("hp_potion", 0.1), ("ranger_hood", 0.006)],
            Gloomwisp => &[("wisp_ember", 0.6), ("sky_honey", 0.1), ("shadow_silk", 0.2), ("mp_potion_big", 0.05), ("shadow_garb", 0.008), ("shadow_leggings", 0.008)],
            Stormwool | Starwool => &[("wool_tuft", 0.6), ("game_meat", 0.35), ("star_dust", 0.12), ("mp_potion", 0.1), ("sage_robe", 0.006), ("sage_hat", 0.006)],
            Squallsprite => &[("gust_wisp", 0.6), ("sky_honey", 0.25), ("thunder_core", 0.1), ("mp_potion_big", 0.05), ("sage_ring", 0.006)],
            QuarryCrab => &[("crab_shell", 0.6), ("crab_meat", 0.35), ("iron_ore", 0.4), ("hp_potion_big", 0.06), ("iron_greaves", 0.01), ("plate_armor", 0.004)],
            MireStump | Barkstump => &[("rotten_bark", 0.6), ("sturdy_branch", 0.5), ("glowcap", 0.15), ("shadow_silk", 0.15), ("elixir", 0.02), ("monk_gi", 0.004)],
            Thunderhawk => &[("hawk_feather", 0.6), ("speckled_egg", 0.2), ("sky_honey", 0.15), ("thunder_core", 0.3), ("mp_potion_big", 0.06), ("storm_amulet", 0.004), ("feather_wings", 0.003)],
            Voidgust | AstralCrab => &[("star_dust", 0.5), ("crab_meat", 0.2), ("sky_honey", 0.15), ("thunder_core", 0.15), ("elixir", 0.03), ("rogue_coin", 0.01), ("star_drop", 0.006), ("gale_greaves", 0.004)],
            Cryptling | Vaultbeak | Galewraith | Stormkite | VaultWisp => &[("gale_shard", 0.25), ("thunder_core", 0.2), ("elixir", 0.05), ("galeheart_ring", 0.004), ("galeheart_earring", 0.004)],
            Ruinstump | Vaultcrab => &[("gale_shard", 0.35), ("iron_ore", 0.5), ("crab_meat", 0.3), ("elixir", 0.06), ("galeheart_treads", 0.004), ("galeheart_helm", 0.004)],
            Gatekeeper => &[("gale_shard", 1.0), ("gale_shard", 1.0), ("gale_shard", 0.5), ("storm_horn", 0.5), ("elixir", 1.0)],
            Stormwarden => &[("gale_shard", 1.0), ("gale_shard", 1.0), ("gale_shard", 1.0), ("crimson_core", 0.35), ("storm_horn", 1.0), ("elixir", 1.0)],
            Cactling => &[("cactus_spine", 0.5), ("scorpion_stinger", 0.2), ("game_meat", 0.2), ("hp_potion", 0.08)],
            DuneScuttler => &[("cactus_spine", 0.5), ("scorpion_stinger", 0.2), ("game_meat", 0.2), ("hp_potion", 0.08)],
            Sandwhirl => &[("dust_devil_core", 0.3), ("gust_wisp", 0.4), ("mp_potion", 0.1)],
            MesaScorpion => &[("scorpion_stinger", 0.5), ("dust_devil_core", 0.25), ("cactus_spine", 0.3), ("hp_potion_big", 0.06), ("sidewinder_pistols", 0.003), ("windstep_boots", 0.006)],
            Thornback => &[("scorpion_stinger", 0.5), ("dust_devil_core", 0.25), ("cactus_spine", 0.3), ("hp_potion_big", 0.06), ("sidewinder_pistols", 0.003), ("windstep_boots", 0.006)],
            DustDevil => &[("scorpion_stinger", 0.5), ("dust_devil_core", 0.25), ("cactus_spine", 0.3), ("hp_potion_big", 0.06), ("sidewinder_pistols", 0.003), ("windstep_boots", 0.006)],
            Snowpuff => &[("frost_pelt", 0.45), ("rime_crystal", 0.25), ("game_meat", 0.25), ("hp_potion_grand", 0.04), ("frostguard_treads", 0.003)],
            Rimeclaw => &[("frost_pelt", 0.45), ("rime_crystal", 0.25), ("game_meat", 0.25), ("hp_potion_grand", 0.04), ("frostguard_treads", 0.003)],
            IceSprite => &[("ice_sprite_dust", 0.45), ("rime_crystal", 0.2), ("mp_potion_grand", 0.04), ("frostguard_hat", 0.003)],
            Frostling => &[("frost_pelt", 0.45), ("rime_crystal", 0.25), ("game_meat", 0.25), ("hp_potion_grand", 0.04), ("frostguard_treads", 0.003)],
            GlacialYeti => &[("yeti_horn", 0.35), ("rime_crystal", 0.35), ("golem_core", 0.15), ("frost_pelt", 0.3), ("elixir_grand", 0.03), ("frostguard_plate", 0.002), ("frostguard_jacket", 0.002), ("frostguard_robe", 0.002), ("frostguard_ring", 0.003)],
            RimeGolem => &[("yeti_horn", 0.35), ("rime_crystal", 0.35), ("golem_core", 0.15), ("frost_pelt", 0.3), ("elixir_grand", 0.03), ("frostguard_plate", 0.002), ("frostguard_jacket", 0.002), ("frostguard_robe", 0.002), ("frostguard_ring", 0.003)],
            FrostWisp => &[("yeti_horn", 0.35), ("rime_crystal", 0.35), ("golem_core", 0.15), ("frost_pelt", 0.3), ("elixir_grand", 0.03), ("frostguard_plate", 0.002), ("frostguard_jacket", 0.002), ("frostguard_robe", 0.002), ("frostguard_ring", 0.003)],
            CinderSalamander => &[("salamander_scale", 0.4), ("magma_shell", 0.3), ("golem_core", 0.15), ("hp_potion_grand", 0.05), ("emberweave_treads", 0.003), ("emberweave_ring", 0.002)],
            MagmaCrab => &[("salamander_scale", 0.4), ("magma_shell", 0.3), ("golem_core", 0.15), ("hp_potion_grand", 0.05), ("emberweave_treads", 0.003), ("emberweave_ring", 0.002)],
            AshenStump => &[("salamander_scale", 0.4), ("magma_shell", 0.3), ("golem_core", 0.15), ("hp_potion_grand", 0.05), ("emberweave_treads", 0.003), ("emberweave_ring", 0.002)],
            EmberGolem => &[("salamander_scale", 0.4), ("magma_shell", 0.3), ("golem_core", 0.15), ("hp_potion_grand", 0.05), ("emberweave_treads", 0.003), ("emberweave_ring", 0.002)],
            LavaSalamander => &[("obsidian_shard", 0.4), ("magma_shell", 0.3), ("salamander_scale", 0.3), ("elixir_grand", 0.04), ("emberweave_plate", 0.002), ("emberweave_jacket", 0.002), ("emberweave_robe", 0.002), ("emberweave_amulet", 0.003)],
            FlameScorpion => &[("obsidian_shard", 0.4), ("magma_shell", 0.3), ("salamander_scale", 0.3), ("elixir_grand", 0.04), ("emberweave_plate", 0.002), ("emberweave_jacket", 0.002), ("emberweave_robe", 0.002), ("emberweave_amulet", 0.003)],
            ObsidianGolem => &[("obsidian_shard", 0.4), ("magma_shell", 0.3), ("salamander_scale", 0.3), ("elixir_grand", 0.04), ("emberweave_plate", 0.002), ("emberweave_jacket", 0.002), ("emberweave_robe", 0.002), ("emberweave_amulet", 0.003)],
            Stormbeak => &[("storm_feather", 0.4), ("wyrm_scale", 0.25), ("golem_core", 0.15), ("elixir_grand", 0.04), ("skyreach_mantle", 0.003)],
            CloudRam => &[("storm_feather", 0.4), ("wyrm_scale", 0.25), ("golem_core", 0.15), ("elixir_grand", 0.04), ("skyreach_mantle", 0.003)],
            ZephyrWyrmling => &[("storm_feather", 0.4), ("wyrm_scale", 0.25), ("golem_core", 0.15), ("elixir_grand", 0.04), ("skyreach_mantle", 0.003)],
            ThunderGolem => &[("storm_feather", 0.4), ("wyrm_scale", 0.25), ("golem_core", 0.15), ("elixir_grand", 0.04), ("skyreach_mantle", 0.003)],
            StarWyrmling => &[("void_essence", 0.4), ("astral_core", 0.3), ("wyrm_scale", 0.25), ("elixir_grand", 0.05), ("emberweave_earring", 0.003), ("celestial_ring", 0.0015)],
            VoidScorpion => &[("void_essence", 0.4), ("astral_core", 0.3), ("wyrm_scale", 0.25), ("elixir_grand", 0.05), ("emberweave_earring", 0.003), ("celestial_ring", 0.0015)],
            AstralGolem => &[("void_essence", 0.4), ("astral_core", 0.3), ("wyrm_scale", 0.25), ("elixir_grand", 0.05), ("emberweave_earring", 0.003), ("celestial_ring", 0.0015)],
            FrostBat => &[("rime_shard", 0.2), ("rime_crystal", 0.4), ("elixir_grand", 0.05), ("rimeheart_ring", 0.003), ("rimeheart_earring", 0.003)],
            IceWisp => &[("rime_shard", 0.2), ("rime_crystal", 0.4), ("elixir_grand", 0.05), ("rimeheart_ring", 0.003), ("rimeheart_earring", 0.003)],
            FireBat => &[("ember_shard", 0.2), ("obsidian_shard", 0.4), ("elixir_grand", 0.05), ("cinderforged_ring", 0.002), ("cinderforged_earring", 0.002)],
            EmberWisp => &[("ember_shard", 0.2), ("obsidian_shard", 0.4), ("elixir_grand", 0.05), ("cinderforged_ring", 0.002), ("cinderforged_earring", 0.002)],
            StarBat => &[("star_shard", 0.2), ("astral_core", 0.4), ("elixir_grand", 0.06), ("celestial_ring", 0.002), ("celestial_earring", 0.002)],
            SeraphWisp => &[("star_shard", 0.2), ("astral_core", 0.4), ("elixir_grand", 0.06), ("celestial_ring", 0.002), ("celestial_earring", 0.002)],
            FrostCrystal => &[],
            RimeColossus => &[("rime_shard", 1.0), ("rime_shard", 1.0), ("rime_crystal", 1.0), ("elixir_grand", 1.0)],
            Glacia => &[("rime_shard", 1.0), ("rime_shard", 1.0), ("rime_shard", 1.0), ("yeti_horn", 1.0), ("elixir_grand", 1.0)],
            Magmaw => &[("ember_shard", 1.0), ("ember_shard", 1.0), ("magma_shell", 1.0), ("elixir_grand", 1.0)],
            Ignaroth => &[("ember_shard", 1.0), ("ember_shard", 1.0), ("ember_shard", 1.0), ("obsidian_shard", 1.0), ("elixir_grand", 1.0)],
            Sol => &[("star_shard", 1.0), ("star_shard", 1.0), ("astral_core", 1.0), ("elixir_grand", 1.0)],
            Luna => &[("star_shard", 1.0), ("star_shard", 1.0), ("void_essence", 1.0), ("elixir_grand", 1.0)],

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
    // Tempest Vault boss mechanics (0.14.0)
    /// Gatekeeper: shield raised in front - only hits from behind do full damage.
    Guard,
    /// Gatekeeper: brings iron spikes down from the ceiling.
    Portcullis,
    /// Stormwarden: a whirlwind that drags players in, then bursts.
    Vortex,
    /// Stormwarden: electrifies the floor - only platforms are safe.
    Surge,
    // 0.15.0 dungeon bosses (see `boss_ai`)
    Blizzard,
    Avalanche,
    Icicles,
    Encased,
    Overheat,
    Eruption,
    Breath,
    Inferno,
    WingGust,
    Flare,
    LunarRain,
    Eclipse,
}

/// Every state, indexed by `EState as u32` (for the network).
pub const ALL_ESTATES: [EState; 27] = [
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
    EState::Guard,
    EState::Portcullis,
    EState::Vortex,
    EState::Surge,
    EState::Blizzard,
    EState::Avalanche,
    EState::Icicles,
    EState::Encased,
    EState::Overheat,
    EState::Eruption,
    EState::Breath,
    EState::Inferno,
    EState::WingGust,
    EState::Flare,
    EState::LunarRain,
    EState::Eclipse,
];

// ------------------------------------------------------------------ vault boss mechanics

/// How long the Gatekeeper holds its shield up.
pub const GUARD_SECS: f32 = 3.4;
/// Damage the Gatekeeper's shield lets through from the front.
pub const GUARD_BLOCK: f32 = 0.1;
/// Seconds before the portcullis spikes are released (they then fall ~1s).
pub const PORTCULLIS_WARN: f32 = 0.75;
/// Spacing of the spikes across the arena.
pub const SPIKE_GAP: f32 = 130.0;
/// How long the Stormwarden's vortex pulls.
pub const VORTEX_SECS: f32 = 3.0;
/// How far the vortex reaches and how hard it pulls (px/s).
pub const VORTEX_RANGE: f32 = 780.0;
pub const VORTEX_PULL: f32 = 230.0;
/// The floor sparks for this long before it becomes live...
pub const SURGE_WARN: f32 = 1.4;
/// ...and is live for this long.
pub const SURGE_LIVE: f32 = 2.8;
/// How far the live floor reaches from the Stormwarden.
pub const SURGE_RANGE: f32 = 760.0;
/// Vault bosses can't leave their arena: this far either side of where they stand.
pub const ARENA_HALF: f32 = 640.0;

/// Damage multiplier for a hit from `from_x`: the Gatekeeper's shield
/// blocks most hits from the front, Glacia's ice shield blocks everything,
/// and an eclipsing Twin takes half.
pub fn damage_mult(e: &Enemy, from_x: f32) -> f32 {
    match e.state {
        EState::Encased => 0.0,
        EState::Eclipse => 0.5,
        _ => guard_mult(e, from_x),
    }
}

/// The Gatekeeper's raised shield blocks most of a hit from the front.
pub fn guard_mult(e: &Enemy, from_x: f32) -> f32 {
    if e.state == EState::Guard && (from_x - e.body.center().x) * e.facing > 0.0 {
        GUARD_BLOCK
    } else {
        1.0
    }
}

/// Is the floor around this Stormwarden live (players standing on it get shocked)?
pub fn surge_live(e: &Enemy) -> bool {
    e.state == EState::Surge && e.state_t > SURGE_WARN
}

fn arena_half(k: EnemyKind) -> Option<f32> {
    use EnemyKind::*;
    matches!(k, Gatekeeper | Stormwarden | RimeColossus | Glacia | Magmaw | Ignaroth | Sol | Luna).then_some(ARENA_HALF)
}

#[path = "boss_ai.rs"]
pub mod boss_ai;
pub use boss_ai::{on_death, twin_alive, Spawn};

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
    let boss_armor = def.boss && !matches!(e.state, EState::Idle | EState::Walk | EState::Recover | EState::Hitstun | EState::Telegraph | EState::Spit);
    if !boss_armor {
        e.body.vel = V2::new(kb.x * scale, kb.y * scale);
        if e.body.vel.y < 0.0 {
            e.body.on_ground = false;
        }
        e.state = EState::Hitstun;
        e.state_t = 0.0;
        e.think = hitstun * if def.boss { 0.4 } else { 1.0 };
    }
    // a shielded Gatekeeper doesn't turn round to face you
    if e.state != EState::Guard && kb.x != 0.0 {
        e.facing = -kb.x.signum();
    }
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
    /// Moves made so far (vault bosses cycle through their mechanics).
    pub moves: u32,
    /// Boss bookkeeping: crystals left / waves fired, and HP at the start
    /// of an Overheat (0.15.0).
    pub adds: u32,
    pub adds_fired: u32,
    pub mark_hp: f32,
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
            moves: 0,
            adds: 0,
            adds_fired: 0,
            mark_hp: 0.0,
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
    /// Monsters a boss calls up (Glacia's crystals, a revived Twin).
    pub spawns: Vec<Spawn>,
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
            Ai::Spitter => spitter(e, &d, ctx, out),
            Ai::Boss => boss(e, &d, ctx, out),
            Ai::SkyBoss => boss_ai::sky_boss(e, &d, ctx, out),
            Ai::Dummy => {}
        },
    }
    if !floater || e.state == EState::Dying {
        physics::apply_gravity(&mut e.body, dt, false, 1.0);
    }
    physics::move_and_collide(&mut e.body, ctx.level, dt);
    // vault bosses stay in their arena; running into its edge counts as a wall
    if let Some(h) = arena_half(e.kind) {
        let cx = e.body.center().x;
        let (lo, hi) = (e.home.x - h, e.home.x + h);
        if cx < lo || cx > hi {
            e.body.pos.x = cx.clamp(lo, hi) - e.body.w * 0.5;
            e.body.vel.x = 0.0;
            e.body.hit_wall = true;
        }
    }
}

/// Ground shooter: keeps its distance, plants itself, and fires a straight shot.
fn spitter(e: &mut Enemy, d: &EnemyDef, ctx: &Ctx, out: &mut Out) {
    let (dist, dir, same) = to_player(e, ctx);
    let c = e.body.center();
    match e.state {
        EState::Telegraph => {
            e.body.vel.x = 0.0;
            if e.state_t > d.telegraph {
                let mut to = ctx.player - c;
                to.y = to.y.clamp(-60.0, 60.0);
                let l = to.len().max(1.0);
                e.aim = to * (1.0 / l);
                out.shots.push(Shot {
                    pos: c + e.aim * 18.0,
                    vel: e.aim * d.proj_speed,
                    size: 16.0,
                    gravity: 0.0,
                    life: 2.4,
                    pierce: false,
                    damage: d.proj_damage,
                    kb: V2::ZERO,
                    style: d.proj_style,
                    grounded: false,
                });
                e.state = EState::Recover;
                e.state_t = 0.0;
                e.cooldown = d.fire_every;
            }
        }
        EState::Recover => {
            e.body.vel.x = approach(e.body.vel.x, 0.0, 1500.0 * ctx.dt);
            if e.state_t > 0.6 {
                e.state = EState::Walk;
                e.state_t = 0.0;
            }
        }
        _ => {
            e.cooldown -= ctx.dt;
            let aggro = ctx.player_alive && dist < d.aggro && same;
            if aggro {
                e.facing = dir;
                e.body.vel.x = if dist > 320.0 {
                    dir * d.speed
                } else if dist < 170.0 && physics::ground_ahead(&e.body, -dir, ctx.level) {
                    -dir * d.speed * 0.6
                } else {
                    approach(e.body.vel.x, 0.0, 900.0 * ctx.dt)
                };
                e.state = EState::Walk;
                if e.cooldown <= 0.0 && e.body.on_ground {
                    e.state = EState::Telegraph;
                    e.state_t = 0.0;
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
    if boss_ai::special(e, d, ctx, out, enraged) {
        return;
    }
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
        EState::Guard => {
            // shield up: plant, and send shockwaves along the floor in front
            e.body.vel.x = 0.0;
            e.cooldown -= dt;
            if e.cooldown <= 0.0 {
                e.cooldown = if enraged { 0.7 } else { 0.95 };
                out.shots.push(Shot {
                    pos: V2::new(c.x + e.facing * 60.0, e.body.pos.y + e.body.h - 14.0),
                    vel: V2::new(e.facing * 470.0, 0.0),
                    size: 26.0,
                    gravity: 0.0,
                    life: 1.6,
                    pierce: true,
                    damage: d.attack * 0.6,
                    kb: V2::new(300.0, -420.0),
                    style: ProjStyle::Shockwave,
                    grounded: true,
                });
                out.shake = out.shake.max(4.0);
            }
            if e.state_t > GUARD_SECS {
                e.state = EState::Recover;
                e.state_t = 0.0;
            }
        }
        EState::Portcullis => {
            // iron spikes drop from the ceiling across the arena, with gaps
            e.body.vel.x = 0.0;
            if e.state_t > PORTCULLIS_WARN && !e.attack_hit {
                e.attack_hit = true;
                let gap = (e.moves + (e.hp as u32 / 97)) % 3;
                for k in -5i32..=5 {
                    if (k + 5 + gap as i32) % 3 == 0 {
                        continue; // a safe spot
                    }
                    out.shots.push(Shot {
                        pos: V2::new(e.home.x + k as f32 * SPIKE_GAP, e.home.y - 560.0),
                        vel: V2::new(0.0, 60.0),
                        size: 22.0,
                        gravity: 1000.0,
                        life: 1.7,
                        pierce: true,
                        damage: d.attack * 0.9,
                        kb: V2::new(260.0, -360.0),
                        style: ProjStyle::Spike,
                        grounded: false,
                    });
                }
                out.shake = out.shake.max(6.0);
            }
            if e.state_t > PORTCULLIS_WARN + 1.0 {
                e.state = EState::Walk;
                e.state_t = 0.0;
                e.think = 0.5;
            }
        }
        EState::Vortex => {
            // the pull itself is felt by each player (Sim::boss_hazards); at the
            // end the storm bursts outwards
            e.body.vel.x = 0.0;
            if e.state_t > VORTEX_SECS - 0.3 && !e.attack_hit {
                e.attack_hit = true;
                let n = if enraged { 12 } else { 10 };
                for i in 0..n {
                    let a = i as f32 / n as f32 * std::f32::consts::TAU;
                    let dirv = V2::new(a.cos(), a.sin());
                    out.shots.push(Shot {
                        pos: c + dirv * 40.0,
                        vel: dirv * 360.0,
                        size: 18.0,
                        gravity: 0.0,
                        life: 2.0,
                        pierce: false,
                        damage: d.proj_damage,
                        kb: V2::new(220.0, -220.0),
                        style: d.proj_style,
                        grounded: false,
                    });
                }
                out.shake = out.shake.max(9.0);
            }
            if e.state_t > VORTEX_SECS {
                e.state = EState::Recover;
                e.state_t = 0.0;
            }
        }
        EState::Surge => {
            // the floor charges up, then shocks anyone standing on it
            // (Sim::boss_hazards); meanwhile it picks people off the platforms
            e.body.vel.x = 0.0;
            if e.state_t > SURGE_WARN + 0.9 && !e.attack_hit {
                e.attack_hit = true;
                let to = ctx.player - c;
                let base = to.y.atan2(to.x);
                for i in -1..=1 {
                    let a = base + i as f32 * 0.18;
                    let dirv = V2::new(a.cos(), a.sin());
                    out.shots.push(Shot {
                        pos: c + dirv * 40.0,
                        vel: dirv * 380.0,
                        size: 18.0,
                        gravity: 0.0,
                        life: 2.2,
                        pierce: false,
                        damage: d.proj_damage,
                        kb: V2::new(200.0, -200.0),
                        style: d.proj_style,
                        grounded: false,
                    });
                }
            }
            if e.state_t > SURGE_WARN + SURGE_LIVE {
                e.state = EState::Recover;
                e.state_t = 0.0;
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
                    let near = dist < 260.0;
                    let basic = match (roll, near) {
                        (_, true) if roll != 2 => EState::Stomp,
                        (2, _) => EState::Spit,
                        _ => EState::Telegraph,
                    };
                    // the vault bosses work through their own mechanics in turn
                    e.state = match e.kind {
                        EnemyKind::Gatekeeper => match e.moves % 4 {
                            0 => basic,
                            1 => EState::Portcullis,
                            2 => EState::Guard,
                            _ if enraged => EState::Portcullis,
                            _ => basic,
                        },
                        EnemyKind::Stormwarden => match e.moves % 4 {
                            0 => basic,
                            1 => EState::Vortex,
                            2 if enraged => EState::Surge,
                            2 => basic,
                            _ => EState::Surge,
                        },
                        _ => boss_ai::next_move(e, basic, enraged).unwrap_or(basic),
                    };
                    e.moves += 1;
                    e.cooldown = 0.45;
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
            if matches!(d.ai, Ai::Floater { diver: false } | Ai::Spitter | Ai::SkyBoss) || d.ai == Ai::Boss {
                assert!(shots > 0, "{:?} never attacked at range", k);
            }
            if e.is_floater() {
                assert!(e.body.pos.y + e.body.h < 640.0 - 20.0 || e.state == EState::Lunge, "{:?} should hover", k);
            }
        }
    }
}
