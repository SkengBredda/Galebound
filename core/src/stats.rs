//! Character stats.
//!
//! Main stats (STR / DEX / INT / LUK) are bought with ability points (AP,
//! 5 per level). Together with class, level, equipment and buffs they derive
//! the combat stats: HP, Mana, Physical/Magic ATK, Physical/Magical DEF,
//! Critical Chance % and Critical Damage %.

use crate::classes::{ClassId, MainStat};

pub const AP_PER_LEVEL: u32 = 5;
pub const START_STATS: MainStats = MainStats { str: 5, dex: 5, int: 4, luk: 4 };
pub const MAX_CRIT_CHANCE: f32 = 75.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MainStats {
    pub str: i32,
    pub dex: i32,
    pub int: i32,
    pub luk: i32,
}

impl MainStats {
    pub fn get(&self, s: MainStat) -> i32 {
        match s {
            MainStat::Str => self.str,
            MainStat::Dex => self.dex,
            MainStat::Int => self.int,
            MainStat::Luk => self.luk,
        }
    }
    pub fn add(&mut self, s: MainStat, v: i32) {
        match s {
            MainStat::Str => self.str += v,
            MainStat::Dex => self.dex += v,
            MainStat::Int => self.int += v,
            MainStat::Luk => self.luk += v,
        }
    }
    pub fn total(&self) -> i32 {
        self.str + self.dex + self.int + self.luk
    }
}

/// Flat (and a few percentage) stat bonuses from equipment and buffs.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StatMods {
    pub main: MainStats,
    pub hp: f32,
    pub mp: f32,
    pub patk: f32,
    pub matk: f32,
    pub pdef: f32,
    pub mdef: f32,
    /// Percentage points.
    pub crit: f32,
    /// Percentage points (added to the base 150%).
    pub crit_dmg: f32,
    /// Fractions, e.g. 0.2 = +20%.
    pub atk_pct: f32,
    pub pdef_pct: f32,
    pub mdef_pct: f32,
    pub speed_pct: f32,
}

impl std::ops::Add for StatMods {
    type Output = StatMods;
    fn add(self, o: StatMods) -> StatMods {
        StatMods {
            main: MainStats { str: self.main.str + o.main.str, dex: self.main.dex + o.main.dex, int: self.main.int + o.main.int, luk: self.main.luk + o.main.luk },
            hp: self.hp + o.hp,
            mp: self.mp + o.mp,
            patk: self.patk + o.patk,
            matk: self.matk + o.matk,
            pdef: self.pdef + o.pdef,
            mdef: self.mdef + o.mdef,
            crit: self.crit + o.crit,
            crit_dmg: self.crit_dmg + o.crit_dmg,
            atk_pct: self.atk_pct + o.atk_pct,
            pdef_pct: self.pdef_pct + o.pdef_pct,
            mdef_pct: self.mdef_pct + o.mdef_pct,
            speed_pct: self.speed_pct + o.speed_pct,
        }
    }
}

/// Final combat stats.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Derived {
    pub main: MainStats,
    pub max_hp: f32,
    pub max_mp: f32,
    pub patk: f32,
    pub matk: f32,
    pub pdef: f32,
    pub mdef: f32,
    /// Percent, 0-75.
    pub crit_chance: f32,
    /// Percent, e.g. 150 = crits deal 1.5x.
    pub crit_damage: f32,
    pub mp_regen: f32,
    pub speed_mult: f32,
}

impl Derived {
    /// Damage multiplier for an attack (physical or magical).
    pub fn attack_mult(&self, magic: bool) -> f32 {
        let atk = if magic { self.matk } else { self.patk };
        1.0 + atk / 80.0
    }
    /// Fraction of incoming damage that gets through.
    pub fn damage_taken(&self, magic: bool) -> f32 {
        let def = if magic { self.mdef } else { self.pdef };
        100.0 / (100.0 + def.max(0.0) * 1.5)
    }
}

pub fn derive(class: ClassId, level: u32, base: MainStats, mods: &StatMods) -> Derived {
    let info = class.info();
    let l = level.max(1) as f32 - 1.0;
    let m = MainStats {
        str: base.str + mods.main.str,
        dex: base.dex + mods.main.dex,
        int: base.int + mods.main.int,
        luk: base.luk + mods.main.luk,
    };
    let (s, d, i, k) = (m.str as f32, m.dex as f32, m.int as f32, m.luk as f32);
    let prim = m.get(info.primary) as f32;
    let sec = m.get(info.secondary) as f32;
    let (patk, matk) = if info.magic {
        (mods.patk + s * 0.5, mods.matk + prim * 1.6 + sec * 0.5)
    } else {
        (mods.patk + prim * 1.4 + sec * 0.5, mods.matk + i * 0.5)
    };
    let level_def = l + 1.0;
    Derived {
        main: m,
        max_hp: info.base_hp + info.hp_per_level * l + s * 6.0 + mods.hp,
        max_mp: info.base_mp + info.mp_per_level * l + i * 6.0 + mods.mp,
        patk: patk * (1.0 + mods.atk_pct),
        matk: matk * (1.0 + mods.atk_pct),
        pdef: (mods.pdef + s * 0.4 + level_def) * (1.0 + mods.pdef_pct),
        mdef: (mods.mdef + i * 0.5 + level_def) * (1.0 + mods.mdef_pct),
        crit_chance: (5.0 + k * 0.35 + d * 0.1 + mods.crit).min(MAX_CRIT_CHANCE),
        crit_damage: 150.0 + k * 0.5 + mods.crit_dmg,
        mp_regen: 2.0 + i * 0.08,
        speed_mult: 1.0 + mods.speed_pct,
    }
}

/// Spread `ap` points the way a class would want them (used when migrating
/// old saves and for the "Auto" button): 60% primary, 30% secondary, 10% HP.
pub fn auto_assign(class: ClassId, mut stats: MainStats, ap: u32) -> MainStats {
    let info = class.info();
    for n in 0..ap {
        let s = match n % 10 {
            0..=5 => info.primary,
            6..=8 => info.secondary,
            _ => {
                if info.magic {
                    MainStat::Luk
                } else {
                    MainStat::Str
                }
            }
        };
        stats.add(s, 1);
    }
    stats
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classes::EVERY_CLASS;

    #[test]
    fn stats_grow_with_level_and_points() {
        for &c in EVERY_CLASS {
            let a = derive(c, 1, START_STATS, &StatMods::default());
            let b = derive(c, 15, auto_assign(c, START_STATS, 70), &StatMods::default());
            assert!(b.max_hp > a.max_hp && b.max_mp > a.max_mp);
            assert!(b.pdef > a.pdef && b.mdef > a.mdef);
            let info = c.info();
            assert!(b.attack_mult(info.magic) > a.attack_mult(info.magic) * 1.4, "{:?} attack should scale", c);
            assert!(a.crit_chance >= 5.0 && b.crit_damage >= 150.0);
        }
    }

    #[test]
    fn defence_reduces_damage() {
        let weak = derive(ClassId::Novice, 1, START_STATS, &StatMods::default());
        let armored = derive(ClassId::Novice, 1, START_STATS, &StatMods { pdef: 40.0, ..Default::default() });
        assert!(armored.damage_taken(false) < weak.damage_taken(false));
        assert!((armored.damage_taken(true) - weak.damage_taken(true)).abs() < 1e-6, "physical armour doesn't help vs magic");
    }

    #[test]
    fn crit_is_capped() {
        let d = derive(ClassId::Duskblade, 50, MainStats { str: 4, dex: 300, int: 4, luk: 500 }, &StatMods::default());
        assert_eq!(d.crit_chance, MAX_CRIT_CHANCE);
    }
}
