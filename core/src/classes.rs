//! Playable classes and their movesets.
//!
//! Five original classes fill the classic side-scroller MMO roles (sword
//! fighter, rogue, martial artist, archer, mage). Combat follows a
//! platform-fighter layout: Light and Heavy attacks change with direction
//! (neutral / side / down) and with whether you're on the ground or airborne.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ClassId {
    /// Everyone starts here; change class at level 10 in a class town.
    Novice,
    Vanguard,
    Duskblade,
    Stormfist,
    Windrunner,
    Aethermancer,
}

/// The five advanced classes you can change into at level 10.
pub const ALL_CLASSES: &[ClassId] = &[
    ClassId::Vanguard,
    ClassId::Duskblade,
    ClassId::Stormfist,
    ClassId::Windrunner,
    ClassId::Aethermancer,
];

/// Every class including Novice.
pub const EVERY_CLASS: &[ClassId] = &[
    ClassId::Novice,
    ClassId::Vanguard,
    ClassId::Duskblade,
    ClassId::Stormfist,
    ClassId::Windrunner,
    ClassId::Aethermancer,
];

/// Level at which a Novice may change class.
pub const CLASS_CHANGE_LEVEL: u32 = 10;

/// The four main stats players invest points into.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MainStat {
    Str,
    Dex,
    Int,
    Luk,
}

pub const MAIN_STATS: &[MainStat] = &[MainStat::Str, MainStat::Dex, MainStat::Int, MainStat::Luk];

impl MainStat {
    pub fn name(self) -> &'static str {
        match self {
            MainStat::Str => "STR",
            MainStat::Dex => "DEX",
            MainStat::Int => "INT",
            MainStat::Luk => "LUK",
        }
    }
    pub fn long_name(self) -> &'static str {
        match self {
            MainStat::Str => "Strength",
            MainStat::Dex => "Dexterity",
            MainStat::Int => "Intelligence",
            MainStat::Luk => "Luck",
        }
    }
    pub fn help(self) -> &'static str {
        match self {
            MainStat::Str => "+Max HP, +Physical DEF. Main attack stat for Vanguards and Novices.",
            MainStat::Dex => "+Crit Chance. Main attack stat for Windrunners and Stormfists.",
            MainStat::Int => "+Max Mana, +Magical DEF, +MP regen. Main attack stat for Aethermancers.",
            MainStat::Luk => "+Crit Chance, +Crit Damage. Main attack stat for Duskblades.",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Weapon {
    /// Novice weapons: a wooden club, a gnarled stick and a wooden sword.
    Club,
    Stick,
    WoodSword,
    SwordShield,
    TwinDaggers,
    Gauntlets,
    Bow,
    Staff,
}

#[derive(Clone, Debug)]
pub struct ClassInfo {
    pub name: &'static str,
    pub role: &'static str,
    pub blurb: &'static str,
    pub weapon: Weapon,
    pub weapon_name: &'static str,
    /// Later class advancement paths (level 10), shown in the creator.
    pub advancements: [&'static str; 2],
    pub base_hp: f32,
    pub base_mp: f32,
    pub hp_per_level: f32,
    pub mp_per_level: f32,
    pub run_speed: f32,
    pub jump_speed: f32,
    pub air_jumps: u8,
    /// Higher weight = less knockback taken.
    pub weight: f32,
    pub power: f32,
    /// Ratings 1-5 for the creator UI: [Power, Defence, Speed, Range, Difficulty]
    pub ratings: [u8; 5],
    /// Main stats that power this class's attacks (primary counts most).
    pub primary: MainStat,
    pub secondary: MainStat,
    /// Attacks deal magic damage (scales with Magic ATK instead of Physical ATK).
    pub magic: bool,
    /// Town where this class is taught.
    pub town: &'static str,
}

impl ClassId {
    pub fn key(self) -> &'static str {
        match self {
            ClassId::Novice => "novice",
            ClassId::Vanguard => "vanguard",
            ClassId::Duskblade => "duskblade",
            ClassId::Stormfist => "stormfist",
            ClassId::Windrunner => "windrunner",
            ClassId::Aethermancer => "aethermancer",
        }
    }
    pub fn from_key(s: &str) -> Option<ClassId> {
        EVERY_CLASS.iter().copied().find(|c| c.key() == s)
    }

    pub fn info(self) -> ClassInfo {
        match self {
            ClassId::Novice => ClassInfo {
                name: "Novice",
                role: "Adventurer in Training",
                blurb: "Every hero starts somewhere. Learn the basics, then choose your path at level 10.",
                weapon: Weapon::Stick,
                weapon_name: "Gnarled Stick",
                advancements: ["Any class", "at level 10"],
                base_hp: 110.0,
                base_mp: 30.0,
                hp_per_level: 14.0,
                mp_per_level: 4.0,
                run_speed: 270.0,
                jump_speed: 714.0,
                air_jumps: 1,
                weight: 1.0,
                power: 1.0,
                ratings: [2, 2, 3, 1, 1],
                primary: MainStat::Str,
                secondary: MainStat::Dex,
                magic: false,
                town: "breezehollow",
            },
            ClassId::Vanguard => ClassInfo {
                name: "Vanguard",
                role: "Frontline Warrior",
                blurb: "A sword-and-shield bruiser who soaks hits and answers with crushing swings.",
                weapon: Weapon::SwordShield,
                weapon_name: "Sword & Shield",
                advancements: ["Ravager", "Bulwark"],
                base_hp: 180.0,
                base_mp: 40.0,
                hp_per_level: 22.0,
                mp_per_level: 4.0,
                run_speed: 258.0,
                jump_speed: 706.0,
                air_jumps: 1,
                weight: 1.35,
                power: 1.15,
                ratings: [4, 5, 2, 2, 1],
                primary: MainStat::Str,
                secondary: MainStat::Dex,
                magic: false,
                town: "ironhold",
            },
            ClassId::Duskblade => ClassInfo {
                name: "Duskblade",
                role: "Shadow Rogue",
                blurb: "Twin daggers, triple jumps and blink-fast strikes. Hits soft, hits often.",
                weapon: Weapon::TwinDaggers,
                weapon_name: "Twin Daggers",
                advancements: ["Nightstalker", "Snarewright"],
                base_hp: 120.0,
                base_mp: 60.0,
                hp_per_level: 14.0,
                mp_per_level: 6.0,
                run_speed: 312.0,
                jump_speed: 722.0,
                air_jumps: 2,
                weight: 0.85,
                power: 0.85,
                ratings: [3, 2, 5, 2, 4],
                primary: MainStat::Luk,
                secondary: MainStat::Dex,
                magic: false,
                town: "shadowmere",
            },
            ClassId::Stormfist => ClassInfo {
                name: "Stormfist",
                role: "Martial Artist",
                blurb: "Gauntleted brawler of the wind temples. Combos, launchers and thunderous palms.",
                weapon: Weapon::Gauntlets,
                weapon_name: "Storm Gauntlets",
                advancements: ["Riposte Sect", "Tempest Sect"],
                base_hp: 150.0,
                base_mp: 50.0,
                hp_per_level: 18.0,
                mp_per_level: 5.0,
                run_speed: 290.0,
                jump_speed: 738.0,
                air_jumps: 1,
                weight: 1.0,
                power: 1.0,
                ratings: [4, 3, 4, 1, 3],
                primary: MainStat::Dex,
                secondary: MainStat::Str,
                magic: false,
                town: "tempest_monastery",
            },
            ClassId::Windrunner => ClassInfo {
                name: "Windrunner",
                role: "Ranged Archer",
                blurb: "A nimble bow-wielder who controls space with piercing and arcing arrows.",
                weapon: Weapon::Bow,
                weapon_name: "Longbow",
                advancements: ["Skyhunter", "Beastwarden"],
                base_hp: 125.0,
                base_mp: 70.0,
                hp_per_level: 15.0,
                mp_per_level: 7.0,
                run_speed: 282.0,
                jump_speed: 722.0,
                air_jumps: 1,
                weight: 0.9,
                power: 0.95,
                ratings: [3, 2, 4, 5, 2],
                primary: MainStat::Dex,
                secondary: MainStat::Str,
                magic: false,
                town: "leafwhisper",
            },
            ClassId::Aethermancer => ClassInfo {
                name: "Aethermancer",
                role: "Elemental Mage",
                blurb: "Bends wind and starlight through a runed staff. Slow, fragile, devastating.",
                weapon: Weapon::Staff,
                weapon_name: "Runed Staff",
                advancements: ["Galecaller", "Runesage"],
                base_hp: 110.0,
                base_mp: 120.0,
                hp_per_level: 12.0,
                mp_per_level: 12.0,
                run_speed: 258.0,
                jump_speed: 706.0,
                air_jumps: 1,
                weight: 0.85,
                power: 1.25,
                ratings: [5, 1, 2, 4, 3],
                primary: MainStat::Int,
                secondary: MainStat::Luk,
                magic: true,
                town: "starfall_spire",
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AttackKind {
    LightNeutral,
    LightSide,
    LightDown,
    AirNeutral,
    AirSide,
    AirDown,
    HeavyNeutral,
    HeavySide,
    HeavyDown,
    AirHeavyNeutral,
    AirHeavyDown,
}

pub const ALL_ATTACK_KINDS: [AttackKind; 11] = [
    AttackKind::LightNeutral,
    AttackKind::LightSide,
    AttackKind::LightDown,
    AttackKind::AirNeutral,
    AttackKind::AirSide,
    AttackKind::AirDown,
    AttackKind::HeavyNeutral,
    AttackKind::HeavySide,
    AttackKind::HeavyDown,
    AttackKind::AirHeavyNeutral,
    AttackKind::AirHeavyDown,
];

impl AttackKind {
    pub fn index(self) -> u32 {
        ALL_ATTACK_KINDS.iter().position(|k| *k == self).unwrap_or(0) as u32
    }
    pub fn from_index(i: u32) -> Option<AttackKind> {
        ALL_ATTACK_KINDS.get(i as usize).copied()
    }
    pub fn is_heavy(self) -> bool {
        matches!(
            self,
            AttackKind::HeavyNeutral
                | AttackKind::HeavySide
                | AttackKind::HeavyDown
                | AttackKind::AirHeavyNeutral
                | AttackKind::AirHeavyDown
        )
    }
    /// Pick the attack from the button pressed, the direction held and grounded state.
    /// `side` = left/right held, `down` = down held. Air-side heavy reuses the neutral air heavy.
    pub fn select(heavy: bool, grounded: bool, side: bool, down: bool) -> AttackKind {
        use AttackKind::*;
        match (heavy, grounded) {
            (false, true) => {
                if down {
                    LightDown
                } else if side {
                    LightSide
                } else {
                    LightNeutral
                }
            }
            (false, false) => {
                if down {
                    AirDown
                } else if side {
                    AirSide
                } else {
                    AirNeutral
                }
            }
            (true, true) => {
                if down {
                    HeavyDown
                } else if side {
                    HeavySide
                } else {
                    HeavyNeutral
                }
            }
            (true, false) => {
                if down {
                    AirHeavyDown
                } else {
                    AirHeavyNeutral
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectileDef {
    /// Velocity; x is multiplied by facing.
    pub vx: f32,
    pub vy: f32,
    pub w: f32,
    pub h: f32,
    pub gravity: f32,
    pub life: f32,
    pub pierce: bool,
    pub count: u8,
    /// Vertical spread between multiple projectiles (px/s of vy).
    pub spread: f32,
    pub style: ProjStyle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjStyle {
    Arrow,
    Knife,
    Orb,
    Bolt,
    Caltrop,
    Shockwave,
    Star,
}

impl ProjStyle {
    pub fn from_u32(i: u32) -> ProjStyle {
        use ProjStyle::*;
        [Arrow, Knife, Orb, Bolt, Caltrop, Shockwave, Star].get(i as usize).copied().unwrap_or(Orb)
    }
}

#[derive(Clone, Debug)]
pub struct AttackDef {
    pub name: &'static str,
    pub startup: f32,
    pub active: f32,
    pub recovery: f32,
    /// Hitbox relative to the attacker's centre; x is mirrored by facing.
    pub hx: f32,
    pub hy: f32,
    pub hw: f32,
    pub hh: f32,
    /// Hit both sides (centred on attacker) instead of in front.
    pub both_sides: bool,
    pub damage: f32,
    /// Knockback (x along facing, y negative = up).
    pub kbx: f32,
    pub kby: f32,
    pub hitstun: f32,
    /// Velocity applied to the attacker when the active window starts (x along facing).
    /// `None` keeps current velocity (with friction).
    pub lunge: Option<(f32, f32)>,
    /// Invulnerable during the startup+active window.
    pub invuln: bool,
    pub projectile: Option<ProjectileDef>,
    pub mp_cost: f32,
    /// Ends the attack as soon as the attacker lands (dives / ground pounds).
    pub ends_on_land: bool,
}

impl AttackDef {
    pub fn total(&self) -> f32 {
        self.startup + self.active + self.recovery
    }
}

#[allow(clippy::too_many_arguments)]
fn melee(
    name: &'static str,
    startup: f32,
    active: f32,
    recovery: f32,
    hit: (f32, f32, f32, f32),
    damage: f32,
    kb: (f32, f32),
    hitstun: f32,
) -> AttackDef {
    AttackDef {
        name,
        startup,
        active,
        recovery,
        hx: hit.0,
        hy: hit.1,
        hw: hit.2,
        hh: hit.3,
        both_sides: false,
        damage,
        kbx: kb.0,
        kby: kb.1,
        hitstun,
        lunge: None,
        invuln: false,
        projectile: None,
        mp_cost: 0.0,
        ends_on_land: false,
    }
}

fn proj(vx: f32, vy: f32, w: f32, h: f32, style: ProjStyle) -> ProjectileDef {
    ProjectileDef { vx, vy, w, h, gravity: 0.0, life: 1.2, pierce: false, count: 1, spread: 0.0, style }
}

trait With {
    fn lunge(self, x: f32, y: f32) -> Self;
    fn both(self) -> Self;
    fn invuln(self) -> Self;
    fn mp(self, c: f32) -> Self;
    fn shoot(self, p: ProjectileDef) -> Self;
    fn land_cancel(self) -> Self;
}
impl With for AttackDef {
    fn lunge(mut self, x: f32, y: f32) -> Self {
        self.lunge = Some((x, y));
        self
    }
    fn both(mut self) -> Self {
        self.both_sides = true;
        self
    }
    fn invuln(mut self) -> Self {
        self.invuln = true;
        self
    }
    fn mp(mut self, c: f32) -> Self {
        self.mp_cost = c;
        self
    }
    fn shoot(mut self, p: ProjectileDef) -> Self {
        self.projectile = Some(p);
        self
    }
    fn land_cancel(mut self) -> Self {
        self.ends_on_land = true;
        self
    }
}

/// Shared light-attack skeleton for melee classes, scaled by reach and speed.
fn melee_light(kind: AttackKind, reach: f32, speed: f32, dmg: f32) -> AttackDef {
    use AttackKind::*;
    let t = |v: f32| v / speed;
    match kind {
        LightNeutral => melee("Jab", t(0.07), t(0.08), t(0.14), (22.0 * reach, -6.0, 40.0 * reach, 30.0), 8.0 * dmg, (160.0, -140.0), 0.22),
        LightSide => melee("Thrust", t(0.09), t(0.09), t(0.18), (30.0 * reach, 0.0, 52.0 * reach, 24.0), 10.0 * dmg, (320.0, -170.0), 0.26).lunge(260.0, 0.0),
        LightDown => melee("Low Sweep", t(0.08), t(0.09), t(0.18), (24.0 * reach, 20.0, 50.0 * reach, 18.0), 9.0 * dmg, (120.0, -420.0), 0.3),
        AirNeutral => melee("Air Slash", t(0.07), t(0.10), t(0.14), (8.0 * reach, -6.0, 64.0 * reach, 54.0), 9.0 * dmg, (200.0, -220.0), 0.24),
        AirSide => melee("Air Lunge", t(0.09), t(0.10), t(0.16), (30.0 * reach, 0.0, 50.0 * reach, 28.0), 10.0 * dmg, (380.0, -160.0), 0.26).lunge(320.0, -80.0),
        AirDown => melee("Spike", t(0.10), t(0.12), t(0.16), (4.0, 34.0, 34.0, 34.0), 11.0 * dmg, (60.0, 520.0), 0.3),
        _ => unreachable!(),
    }
}

/// Attack windows are stretched by this factor for a calmer pace.
pub const ATTACK_TIME: f32 = 1.15;

/// The moveset for a class, with pacing applied (slower windows, lunges and
/// projectiles scaled to the floatier physics).
pub fn attack_def(class: ClassId, kind: AttackKind) -> AttackDef {
    use crate::physics::{PACE_H, PACE_V};
    let mut d = attack_def_raw(class, kind);
    d.startup *= ATTACK_TIME;
    d.active *= ATTACK_TIME;
    d.recovery *= ATTACK_TIME;
    d.hitstun *= 1.1;
    d.kbx *= PACE_H * 1.05;
    d.kby *= PACE_V;
    if let Some((x, y)) = d.lunge {
        d.lunge = Some((x * PACE_H, y * PACE_V));
    }
    if let Some(p) = &mut d.projectile {
        p.vx *= 0.85;
        p.vy *= 0.85;
        p.gravity *= 0.7;
        p.life /= 0.85;
    }
    d
}

/// Projectile hit values with pacing applied.
pub fn projectile_damage(class: ClassId, kind: AttackKind) -> (f32, f32, f32, f32) {
    use crate::physics::{PACE_H, PACE_V};
    let (d, kx, ky, hs) = projectile_damage_raw(class, kind);
    (d, kx * PACE_H * 1.05, ky * PACE_V, hs * 1.1)
}

fn attack_def_raw(class: ClassId, kind: AttackKind) -> AttackDef {
    use AttackKind::*;
    use ClassId::*;
    match class {
        Novice => match kind {
            LightNeutral | LightSide | LightDown | AirNeutral | AirSide | AirDown => melee_light(kind, 1.05, 0.95, 1.0),
            HeavyNeutral => melee("Overhead Bonk", 0.16, 0.10, 0.28, (24.0, -14.0, 40.0, 50.0), 15.0, (200.0, -520.0), 0.4),
            HeavySide => melee("Shoulder Bash", 0.16, 0.14, 0.28, (26.0, -4.0, 44.0, 40.0), 16.0, (520.0, -260.0), 0.4).lunge(420.0, 0.0),
            HeavyDown => melee("Ground Pound", 0.22, 0.12, 0.30, (0.0, 18.0, 120.0, 24.0), 15.0, (240.0, -440.0), 0.4).both(),
            AirHeavyNeutral => melee("Leap Swing", 0.08, 0.18, 0.26, (10.0, -28.0, 50.0, 60.0), 12.0, (140.0, -600.0), 0.36).lunge(100.0, -760.0),
            AirHeavyDown => melee("Dive Bonk", 0.12, 0.55, 0.22, (0.0, 28.0, 56.0, 40.0), 16.0, (300.0, -320.0), 0.4).lunge(0.0, 1100.0).both().land_cancel(),
        },
        Vanguard => match kind {
            LightNeutral | LightSide | LightDown | AirNeutral | AirSide | AirDown => melee_light(kind, 1.15, 0.9, 1.1),
            HeavyNeutral => melee("Shield Bash", 0.16, 0.10, 0.28, (26.0, -10.0, 44.0, 50.0), 20.0, (260.0, -620.0), 0.42),
            HeavySide => melee("Charging Cleave", 0.20, 0.16, 0.30, (34.0, -4.0, 62.0, 44.0), 24.0, (620.0, -300.0), 0.45).lunge(560.0, 0.0),
            HeavyDown => melee("Earthsplitter", 0.24, 0.12, 0.32, (0.0, 18.0, 150.0, 26.0), 22.0, (300.0, -480.0), 0.45).both(),
            AirHeavyNeutral => melee("Rising Edge", 0.10, 0.20, 0.26, (14.0, -30.0, 54.0, 64.0), 16.0, (140.0, -700.0), 0.4).lunge(120.0, -760.0),
            AirHeavyDown => melee("Meteor Drop", 0.14, 0.60, 0.22, (0.0, 30.0, 70.0, 40.0), 26.0, (380.0, -380.0), 0.5).lunge(0.0, 1300.0).both().land_cancel(),
        },
        Duskblade => match kind {
            LightNeutral | LightSide | LightDown | AirNeutral | AirSide | AirDown => melee_light(kind, 0.9, 1.3, 0.9),
            HeavyNeutral => melee("Fan of Knives", 0.12, 0.06, 0.26, (0.0, 0.0, 0.0, 0.0), 0.0, (0.0, 0.0), 0.0)
                .shoot(ProjectileDef { count: 3, spread: 160.0, life: 0.55, ..proj(900.0, 0.0, 16.0, 6.0, ProjStyle::Knife) })
                .mp(6.0),
            HeavySide => melee("Shadow Step", 0.06, 0.18, 0.24, (0.0, -4.0, 70.0, 44.0), 18.0, (360.0, -380.0), 0.4).lunge(900.0, 0.0).invuln().mp(8.0),
            HeavyDown => melee("Caltrops", 0.14, 0.05, 0.25, (0.0, 0.0, 0.0, 0.0), 0.0, (0.0, 0.0), 0.0)
                .shoot(ProjectileDef { count: 3, spread: 0.0, life: 3.0, pierce: true, ..proj(160.0, -260.0, 12.0, 10.0, ProjStyle::Caltrop) })
                .mp(8.0),
            AirHeavyNeutral => melee("Twin Ascent", 0.06, 0.20, 0.22, (0.0, -24.0, 60.0, 60.0), 14.0, (120.0, -640.0), 0.35).lunge(80.0, -820.0).both(),
            AirHeavyDown => melee("Nightfall Dive", 0.10, 0.50, 0.20, (16.0, 24.0, 44.0, 40.0), 18.0, (300.0, -260.0), 0.4).lunge(520.0, 1000.0).land_cancel(),
        },
        Stormfist => match kind {
            LightNeutral | LightSide | LightDown | AirNeutral | AirSide | AirDown => melee_light(kind, 0.95, 1.15, 1.0),
            HeavyNeutral => melee("Rising Uppercut", 0.12, 0.14, 0.28, (20.0, -24.0, 40.0, 70.0), 18.0, (100.0, -820.0), 0.5).lunge(80.0, -420.0),
            HeavySide => melee("Thunder Palm", 0.22, 0.10, 0.30, (38.0, -4.0, 56.0, 46.0), 24.0, (780.0, -320.0), 0.5).lunge(380.0, 0.0).mp(6.0),
            HeavyDown => melee("Quake Stomp", 0.18, 0.12, 0.30, (0.0, 20.0, 130.0, 26.0), 20.0, (240.0, -560.0), 0.45).both(),
            AirHeavyNeutral => melee("Dragon Ascent", 0.08, 0.24, 0.24, (12.0, -28.0, 48.0, 70.0), 16.0, (160.0, -760.0), 0.42).lunge(160.0, -880.0),
            AirHeavyDown => melee("Comet Kick", 0.12, 0.50, 0.22, (22.0, 26.0, 46.0, 40.0), 22.0, (420.0, -300.0), 0.45).lunge(620.0, 900.0).land_cancel(),
        },
        Windrunner => match kind {
            LightNeutral => melee("Bow Swipe", 0.07, 0.08, 0.14, (20.0, -6.0, 40.0, 34.0), 7.0, (180.0, -140.0), 0.22),
            LightSide => melee("Quick Shot", 0.10, 0.04, 0.16, (0.0, 0.0, 0.0, 0.0), 0.0, (0.0, 0.0), 0.0)
                .shoot(ProjectileDef { life: 0.7, ..proj(1100.0, 0.0, 22.0, 4.0, ProjStyle::Arrow) }),
            LightDown => melee("Low Kick", 0.08, 0.09, 0.18, (22.0, 20.0, 42.0, 18.0), 8.0, (140.0, -400.0), 0.28),
            AirNeutral => melee("Spinning Bow", 0.07, 0.12, 0.14, (0.0, -4.0, 70.0, 60.0), 8.0, (200.0, -220.0), 0.24).both(),
            AirSide => melee("Air Shot", 0.09, 0.04, 0.18, (0.0, 0.0, 0.0, 0.0), 0.0, (0.0, 0.0), 0.0)
                .shoot(ProjectileDef { life: 0.7, ..proj(1100.0, 60.0, 22.0, 4.0, ProjStyle::Arrow) }),
            AirDown => melee("Heel Drop", 0.10, 0.12, 0.16, (4.0, 34.0, 34.0, 34.0), 10.0, (60.0, 500.0), 0.3),
            HeavyNeutral => melee("Skyward Volley", 0.16, 0.05, 0.28, (0.0, 0.0, 0.0, 0.0), 0.0, (0.0, 0.0), 0.0)
                .shoot(ProjectileDef { count: 3, spread: 180.0, gravity: 1200.0, life: 1.4, ..proj(620.0, -620.0, 20.0, 4.0, ProjStyle::Arrow) })
                .mp(6.0),
            HeavySide => melee("Piercing Arrow", 0.26, 0.05, 0.26, (0.0, 0.0, 0.0, 0.0), 0.0, (0.0, 0.0), 0.0)
                .shoot(ProjectileDef { pierce: true, life: 0.9, ..proj(1500.0, 0.0, 34.0, 6.0, ProjStyle::Arrow) })
                .mp(8.0),
            HeavyDown => melee("Backflip Shot", 0.10, 0.05, 0.24, (0.0, 0.0, 0.0, 0.0), 0.0, (0.0, 0.0), 0.0)
                .lunge(-420.0, -560.0)
                .shoot(ProjectileDef { count: 2, spread: 120.0, life: 0.8, ..proj(1000.0, 80.0, 22.0, 4.0, ProjStyle::Arrow) })
                .mp(6.0),
            AirHeavyNeutral => melee("Gale Leap", 0.06, 0.10, 0.24, (0.0, 30.0, 70.0, 40.0), 10.0, (200.0, 300.0), 0.3).lunge(140.0, -860.0).both(),
            AirHeavyDown => melee("Rain of Arrows", 0.14, 0.05, 0.30, (0.0, 0.0, 0.0, 0.0), 0.0, (0.0, 0.0), 0.0)
                .shoot(ProjectileDef { count: 4, spread: 220.0, life: 1.0, ..proj(260.0, 900.0, 4.0, 20.0, ProjStyle::Arrow) })
                .mp(8.0),
        },
        Aethermancer => match kind {
            LightNeutral => melee("Staff Twirl", 0.08, 0.10, 0.16, (14.0, -6.0, 54.0, 46.0), 8.0, (180.0, -180.0), 0.24),
            LightSide => melee("Spark", 0.11, 0.04, 0.18, (0.0, 0.0, 0.0, 0.0), 0.0, (0.0, 0.0), 0.0)
                .shoot(ProjectileDef { life: 0.38, ..proj(820.0, 0.0, 16.0, 16.0, ProjStyle::Bolt) }),
            LightDown => melee("Rune Sweep", 0.10, 0.10, 0.20, (26.0, 20.0, 56.0, 18.0), 9.0, (120.0, -420.0), 0.3),
            AirNeutral => melee("Gust Ring", 0.08, 0.14, 0.16, (0.0, 0.0, 84.0, 74.0), 8.0, (240.0, -240.0), 0.24).both(),
            AirSide => melee("Air Spark", 0.10, 0.04, 0.18, (0.0, 0.0, 0.0, 0.0), 0.0, (0.0, 0.0), 0.0)
                .shoot(ProjectileDef { life: 0.4, ..proj(820.0, 120.0, 16.0, 16.0, ProjStyle::Bolt) }),
            AirDown => melee("Staff Plunge", 0.12, 0.12, 0.18, (4.0, 36.0, 30.0, 36.0), 11.0, (60.0, 520.0), 0.3),
            HeavyNeutral => melee("Orb of Gales", 0.26, 0.05, 0.30, (0.0, 0.0, 0.0, 0.0), 0.0, (0.0, 0.0), 0.0)
                .shoot(ProjectileDef { pierce: true, life: 1.6, ..proj(300.0, 0.0, 44.0, 44.0, ProjStyle::Orb) })
                .mp(12.0),
            HeavySide => melee("Arc Bolt", 0.18, 0.05, 0.26, (0.0, 0.0, 0.0, 0.0), 0.0, (0.0, 0.0), 0.0)
                .shoot(ProjectileDef { life: 0.6, ..proj(1300.0, 0.0, 30.0, 14.0, ProjStyle::Bolt) })
                .mp(9.0),
            HeavyDown => melee("Rune Burst", 0.30, 0.18, 0.30, (0.0, 0.0, 170.0, 90.0), 26.0, (420.0, -560.0), 0.5).both().mp(14.0),
            AirHeavyNeutral => melee("Levitate Burst", 0.08, 0.16, 0.30, (0.0, 24.0, 90.0, 50.0), 12.0, (220.0, 260.0), 0.3).lunge(0.0, -760.0).both().mp(6.0),
            AirHeavyDown => melee("Starfall", 0.18, 0.05, 0.30, (0.0, 0.0, 0.0, 0.0), 0.0, (0.0, 0.0), 0.0)
                .shoot(ProjectileDef { count: 3, spread: 260.0, life: 0.9, ..proj(200.0, 1000.0, 22.0, 22.0, ProjStyle::Star) })
                .mp(12.0),
        },
    }
}

/// Damage dealt by each projectile of an attack (unpaced values).
fn projectile_damage_raw(class: ClassId, kind: AttackKind) -> (f32, f32, f32, f32) {
    // (damage, kbx, kby, hitstun)
    use AttackKind::*;
    match (class, kind) {
        (ClassId::Duskblade, HeavyNeutral) => (8.0, 220.0, -180.0, 0.25),
        (ClassId::Duskblade, HeavyDown) => (6.0, 80.0, -420.0, 0.35),
        (ClassId::Windrunner, LightSide) | (ClassId::Windrunner, AirSide) => (9.0, 220.0, -120.0, 0.22),
        (ClassId::Windrunner, HeavyNeutral) => (10.0, 160.0, -260.0, 0.3),
        (ClassId::Windrunner, HeavySide) => (22.0, 520.0, -240.0, 0.4),
        (ClassId::Windrunner, HeavyDown) => (11.0, 300.0, -200.0, 0.3),
        (ClassId::Windrunner, AirHeavyDown) => (10.0, 80.0, 220.0, 0.3),
        (ClassId::Aethermancer, LightSide) | (ClassId::Aethermancer, AirSide) => (10.0, 200.0, -160.0, 0.24),
        (ClassId::Aethermancer, HeavyNeutral) => (14.0, 260.0, -420.0, 0.35),
        (ClassId::Aethermancer, HeavySide) => (24.0, 560.0, -260.0, 0.42),
        (ClassId::Aethermancer, AirHeavyDown) => (16.0, 120.0, 260.0, 0.35),
        _ => (8.0, 200.0, -200.0, 0.25),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_class_has_every_move() {
        use AttackKind::*;
        for &c in EVERY_CLASS {
            for k in [LightNeutral, LightSide, LightDown, AirNeutral, AirSide, AirDown, HeavyNeutral, HeavySide, HeavyDown, AirHeavyNeutral, AirHeavyDown] {
                let d = attack_def(c, k);
                assert!(d.total() > 0.1 && d.total() < 1.7, "{:?} {:?}", c, k);
                assert!(d.projectile.is_some() || d.damage > 0.0, "{:?} {:?} does nothing", c, k);
            }
            assert_eq!(ClassId::from_key(c.key()), Some(c));
        }
    }
}
