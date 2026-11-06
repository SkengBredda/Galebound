//! Pets: bought at any town's Pet Shop, they follow you, pick up loot near
//! you, and grow stronger as you feed them. Each level unlocks passive skills
//! that help their owner. Hunger drains while a pet is out; a starving pet
//! won't loot and its passives switch off until it eats.

use crate::stats::{MainStats, StatMods};

/// Price of a pet at the Pet Shop.
pub const PET_PRICE: u32 = 10_000;
/// What a pet shop charges for a pet (0.14.0: a flat 15,000 gold, not
/// scaled by `items::PRICE_MULT`). Shops buy pets back at `PET_PRICE`-based rates.
pub const PET_SHOP_PRICE: u32 = 15_000;
/// Price of one Pet Food.
pub const FOOD_PRICE: u32 = 100;
pub const MAX_LEVEL: u32 = 20;
pub const HUNGER_MAX: f32 = 100.0;
/// Hunger lost per second while the pet is out: full to empty in 30 minutes.
pub const HUNGER_PER_SEC: f32 = HUNGER_MAX / 1800.0;
/// Below this the pet warns you it's getting hungry.
pub const HUNGER_LOW: f32 = 20.0;
/// What one Pet Food gives.
pub const FOOD_HUNGER: f32 = 30.0;
pub const FOOD_EXP: u32 = 50;
/// How far from you (world units) a pet will go to pick up loot.
pub const LOOT_RADIUS: f32 = 260.0;
/// How many pets one character can own.
pub const MAX_PETS: usize = 6;

/// Pet EXP needed to go from `level` to the next.
pub fn exp_to_next(level: u32) -> u32 {
    50 + 30 * level
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PetSpecies {
    /// Agile scavenger: finds more loot and gold.
    BreezeFox,
    /// Fluffy cloud lamb: defence and healing.
    Puffling,
    /// Tiny storm bird: offence and Mana.
    Sparkwing,
}

pub const ALL_SPECIES: &[PetSpecies] = &[PetSpecies::BreezeFox, PetSpecies::Puffling, PetSpecies::Sparkwing];

impl PetSpecies {
    pub fn key(self) -> &'static str {
        match self {
            PetSpecies::BreezeFox => "breeze_fox",
            PetSpecies::Puffling => "puffling",
            PetSpecies::Sparkwing => "sparkwing",
        }
    }
    pub fn from_key(s: &str) -> Option<PetSpecies> {
        ALL_SPECIES.iter().copied().find(|p| p.key() == s)
    }
    pub fn name(self) -> &'static str {
        match self {
            PetSpecies::BreezeFox => "Breeze Fox",
            PetSpecies::Puffling => "Puffling",
            PetSpecies::Sparkwing => "Sparkwing",
        }
    }
    /// The Pet Shop item that becomes this pet when used.
    pub fn item_id(self) -> &'static str {
        match self {
            PetSpecies::BreezeFox => "pet_breeze_fox",
            PetSpecies::Puffling => "pet_puffling",
            PetSpecies::Sparkwing => "pet_sparkwing",
        }
    }
    /// Flies beside you instead of trotting along the ground.
    pub fn flies(self) -> bool {
        self == PetSpecies::Sparkwing
    }
    pub fn passives(self) -> &'static [PetPassive] {
        match self {
            PetSpecies::BreezeFox => FOX,
            PetSpecies::Puffling => PUFFLING,
            PetSpecies::Sparkwing => SPARKWING,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PassiveEffect {
    /// Added to your stats like gear.
    Stats(StatMods),
    /// Extra pick-up range (world units).
    LootRadius(f32),
    /// Extra gold from gold you pick up (0.1 = +10%).
    GoldFind(f32),
    /// Share of max HP / Mana restored per second.
    Regen { hp: f32, mp: f32 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PetPassive {
    pub name: &'static str,
    pub desc: &'static str,
    /// Pet level that unlocks it.
    pub level: u32,
    pub effect: PassiveEffect,
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

const fn p(name: &'static str, desc: &'static str, level: u32, effect: PassiveEffect) -> PetPassive {
    PetPassive { name, desc, level, effect }
}

const FOX: &[PetPassive] = &[
    p("Keen Nose", "+120 loot pick-up range.", 1, PassiveEffect::LootRadius(120.0)),
    p("Gold Sense", "+10% gold from gold piles.", 5, PassiveEffect::GoldFind(0.10)),
    p("Fleet Paws", "+5% movement speed.", 10, PassiveEffect::Stats(StatMods { speed_pct: 0.05, ..Z })),
    p("Lucky Tail", "+5% Critical Chance.", 15, PassiveEffect::Stats(StatMods { crit: 5.0, ..Z })),
    p("Treasure Hunter", "+15% more gold from piles.", 20, PassiveEffect::GoldFind(0.15)),
];

const PUFFLING: &[PetPassive] = &[
    p("Woolly Guard", "+8% Physical and Magical DEF.", 1, PassiveEffect::Stats(StatMods { pdef_pct: 0.08, mdef_pct: 0.08, ..Z })),
    p("Cosy Fleece", "Heals 0.5% max HP per second.", 5, PassiveEffect::Regen { hp: 0.005, mp: 0.0 }),
    p("Fluffy Bulk", "+60 max HP.", 10, PassiveEffect::Stats(StatMods { hp: 60.0, ..Z })),
    p("Cloud Cushion", "+12% Physical and Magical DEF.", 15, PassiveEffect::Stats(StatMods { pdef_pct: 0.12, mdef_pct: 0.12, ..Z })),
    p("Silver Lining", "+120 max HP.", 20, PassiveEffect::Stats(StatMods { hp: 120.0, ..Z })),
];

const SPARKWING: &[PetPassive] = &[
    p("Static Charge", "+3% Critical Chance.", 1, PassiveEffect::Stats(StatMods { crit: 3.0, ..Z })),
    p("Battle Song", "+5% attack.", 5, PassiveEffect::Stats(StatMods { atk_pct: 0.05, ..Z })),
    p("Mana Spark", "1% max Mana back per second.", 10, PassiveEffect::Regen { hp: 0.0, mp: 0.01 }),
    p("Storm Call", "+20% Critical Damage.", 15, PassiveEffect::Stats(StatMods { crit_dmg: 20.0, ..Z })),
    p("Thunderheart", "+8% attack.", 20, PassiveEffect::Stats(StatMods { atk_pct: 0.08, ..Z })),
];

/// A pet the character owns.
#[derive(Clone, Debug, PartialEq)]
pub struct PetSave {
    pub species: PetSpecies,
    pub name: String,
    pub level: u32,
    pub exp: u32,
    pub hunger: f32,
}

impl PetSave {
    /// A newly adopted pet: level 1 and full.
    pub fn new(species: PetSpecies) -> PetSave {
        PetSave { species, name: species.name().to_string(), level: 1, exp: 0, hunger: HUNGER_MAX }
    }
    pub fn starving(&self) -> bool {
        self.hunger <= 0.0
    }
    /// Passives unlocked at the pet's level (whether or not it's starving).
    pub fn unlocked(&self) -> impl Iterator<Item = &'static PetPassive> + '_ {
        self.species.passives().iter().filter(move |p| p.level <= self.level)
    }
    /// Passives that are working right now (none while starving).
    fn working(&self) -> impl Iterator<Item = &'static PetPassive> + '_ {
        self.unlocked().filter(move |_| !self.starving())
    }
    pub fn stat_mods(&self) -> StatMods {
        let mut m = StatMods::default();
        for p in self.working() {
            if let PassiveEffect::Stats(s) = p.effect {
                m = m + s;
            }
        }
        m
    }
    /// How far from its owner this pet picks up loot.
    pub fn loot_radius(&self) -> f32 {
        LOOT_RADIUS + self.working().map(|p| if let PassiveEffect::LootRadius(r) = p.effect { r } else { 0.0 }).sum::<f32>()
    }
    pub fn gold_find(&self) -> f32 {
        self.working().map(|p| if let PassiveEffect::GoldFind(g) = p.effect { g } else { 0.0 }).sum()
    }
    /// (HP, Mana) share of max restored per second.
    pub fn regen(&self) -> (f32, f32) {
        self.working().fold((0.0, 0.0), |a, p| if let PassiveEffect::Regen { hp, mp } = p.effect { (a.0 + hp, a.1 + mp) } else { a })
    }
    /// Feed one Pet Food: restores hunger and gives EXP. Returns levels gained.
    pub fn feed(&mut self) -> u32 {
        self.hunger = (self.hunger + FOOD_HUNGER).min(HUNGER_MAX);
        if self.level >= MAX_LEVEL {
            self.exp = 0;
            return 0;
        }
        self.exp += FOOD_EXP;
        let mut gained = 0;
        while self.level < MAX_LEVEL && self.exp >= exp_to_next(self.level) {
            self.exp -= exp_to_next(self.level);
            self.level += 1;
            gained += 1;
        }
        if self.level >= MAX_LEVEL {
            self.exp = 0;
        }
        gained
    }
    /// Passive unlocked at exactly this level, if any.
    pub fn passive_at(&self, level: u32) -> Option<&'static PetPassive> {
        self.species.passives().iter().find(|p| p.level == level)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feeding_levels_up_and_unlocks_passives() {
        let mut p = PetSave::new(PetSpecies::BreezeFox);
        assert_eq!(p.loot_radius(), LOOT_RADIUS + 120.0, "level 1 passive works straight away");
        assert_eq!(p.gold_find(), 0.0);
        let mut foods = 0;
        while p.level < 5 {
            p.feed();
            foods += 1;
        }
        assert_eq!(foods, (1..5).map(exp_to_next).sum::<u32>().div_ceil(FOOD_EXP));
        assert!((p.gold_find() - 0.10).abs() < 1e-6, "Gold Sense at level 5");
        for _ in 0..500 {
            p.feed();
        }
        assert_eq!(p.level, MAX_LEVEL);
        assert_eq!(p.unlocked().count(), 5);
        assert_eq!(p.exp, 0);
    }

    #[test]
    fn hunger_switches_passives_off() {
        let mut p = PetSave::new(PetSpecies::Puffling);
        assert!(p.stat_mods().pdef_pct > 0.0);
        p.hunger = 0.0;
        assert!(p.starving());
        assert_eq!(p.stat_mods().pdef_pct, 0.0);
        assert_eq!(p.loot_radius(), LOOT_RADIUS);
        p.feed();
        assert_eq!(p.hunger, FOOD_HUNGER);
        assert!(p.stat_mods().pdef_pct > 0.0);
        p.hunger = 95.0;
        p.feed();
        assert_eq!(p.hunger, HUNGER_MAX, "capped");
    }

    #[test]
    fn every_species_has_a_passive_at_level_1_and_keys_roundtrip() {
        for &s in ALL_SPECIES {
            assert_eq!(s.passives()[0].level, 1);
            assert_eq!(PetSpecies::from_key(s.key()), Some(s));
            let mut levels: Vec<u32> = s.passives().iter().map(|p| p.level).collect();
            levels.dedup();
            assert_eq!(levels.len(), s.passives().len(), "one passive per unlock level");
        }
    }
}
