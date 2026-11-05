//! Crafting: three crafts (Weaponsmithing, Armoursmithing and Cooking), each
//! with its own crafting level (1-30), and the recipes they unlock.
//!
//! You craft at a station: a town's weaponsmith or armourer, Garrick's forge
//! in Breezehollow (both), a cook or innkeeper (Cooking), or the Workbench and
//! Cooking Stove in your own house. Every craft costs its materials plus a
//! small fee, and earns crafting EXP for that craft.

/// One of the three crafts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Craft {
    Weaponsmithing,
    Armoursmithing,
    Cooking,
}

pub const CRAFTS: &[Craft] = &[Craft::Weaponsmithing, Craft::Armoursmithing, Craft::Cooking];

/// The highest crafting level.
pub const MAX_CRAFT_LEVEL: u32 = 30;

impl Craft {
    pub fn index(self) -> usize {
        CRAFTS.iter().position(|c| *c == self).unwrap()
    }
    pub fn key(self) -> &'static str {
        match self {
            Craft::Weaponsmithing => "weaponsmithing",
            Craft::Armoursmithing => "armoursmithing",
            Craft::Cooking => "cooking",
        }
    }
    pub fn from_key(s: &str) -> Option<Craft> {
        CRAFTS.iter().copied().find(|c| c.key() == s)
    }
    pub fn name(self) -> &'static str {
        match self {
            Craft::Weaponsmithing => "Weaponsmithing",
            Craft::Armoursmithing => "Armoursmithing",
            Craft::Cooking => "Cooking",
        }
    }
}

/// Crafting EXP needed to go from `level` to the next.
pub fn craft_exp_to_next(level: u32) -> u32 {
    30 + level * 25 + level * level * 3
}

/// A character's level and EXP in one craft.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CraftSkill {
    pub level: u32,
    pub exp: u32,
}

impl Default for CraftSkill {
    fn default() -> Self {
        CraftSkill { level: 1, exp: 0 }
    }
}

impl CraftSkill {
    /// Add EXP; returns how many levels were gained.
    pub fn gain(&mut self, exp: u32) -> u32 {
        if self.level >= MAX_CRAFT_LEVEL {
            return 0;
        }
        self.exp += exp;
        let mut gained = 0;
        while self.level < MAX_CRAFT_LEVEL && self.exp >= craft_exp_to_next(self.level) {
            self.exp -= craft_exp_to_next(self.level);
            self.level += 1;
            gained += 1;
        }
        if self.level >= MAX_CRAFT_LEVEL {
            self.exp = 0;
        }
        gained
    }
}

/// Where you can craft.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Station {
    /// Garrick's forge, and your house Workbench: weapons and armour.
    Forge,
    Weaponsmith,
    Armourer,
    /// Cooks, innkeepers and your house Cooking Stove.
    Kitchen,
}

impl Station {
    pub fn crafts(self) -> &'static [Craft] {
        match self {
            Station::Forge => &[Craft::Weaponsmithing, Craft::Armoursmithing],
            Station::Weaponsmith => &[Craft::Weaponsmithing],
            Station::Armourer => &[Craft::Armoursmithing],
            Station::Kitchen => &[Craft::Cooking],
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Station::Forge => "Forge",
            Station::Weaponsmith => "Weaponsmith",
            Station::Armourer => "Armourer",
            Station::Kitchen => "Kitchen",
        }
    }
}

/// The crafting station an NPC runs, if any.
pub fn station_for_npc(npc_id: &str) -> Option<Station> {
    match npc_id {
        "garrick" => Some(Station::Forge),
        "cook_bramwell" => Some(Station::Kitchen),
        id if id.starts_with("inn_") => Some(Station::Kitchen),
        id if id.ends_with("_weapons") => Some(Station::Weaponsmith),
        id if id.ends_with("_armor") => Some(Station::Armourer),
        _ => None,
    }
}

#[derive(Clone, Debug)]
pub struct Recipe {
    /// The item made (recipes are looked up by their output).
    pub output: &'static str,
    pub count: u32,
    pub craft: Craft,
    /// Crafting level needed.
    pub level: u32,
    pub inputs: &'static [(&'static str, u32)],
    /// Gold paid to the station.
    pub fee: u32,
    /// Crafting EXP for one craft (less once the recipe is far below your level).
    pub exp: u32,
}

const fn r(output: &'static str, count: u32, craft: Craft, level: u32, inputs: &'static [(&'static str, u32)], fee: u32, exp: u32) -> Recipe {
    Recipe { output, count, craft, level, inputs, fee, exp }
}

use Craft::*;

pub const RECIPES: &[Recipe] = &[
    // ---------------------------------------------------------------- weaponsmithing
    r("iron_ingot", 1, Weaponsmithing, 1, &[("iron_ore", 3)], 5, 8),
    r("seasoned_plank", 1, Weaponsmithing, 1, &[("sturdy_branch", 4)], 3, 6),
    r("wood_sword", 1, Weaponsmithing, 1, &[("seasoned_plank", 2)], 10, 15),
    r("furn_table", 1, Weaponsmithing, 2, &[("seasoned_plank", 4)], 15, 18),
    r("wood_club", 1, Weaponsmithing, 3, &[("seasoned_plank", 2), ("beak_shard", 2)], 10, 18),
    r("furn_barrel", 1, Weaponsmithing, 3, &[("seasoned_plank", 3)], 10, 16),
    r("furn_workbench", 1, Weaponsmithing, 4, &[("seasoned_plank", 6), ("beak_shard", 4)], 30, 30),
    r("oak_club", 1, Weaponsmithing, 5, &[("seasoned_plank", 3), ("beak_shard", 4)], 20, 25),
    r("furn_stove", 1, Weaponsmithing, 6, &[("iron_ingot", 4), ("seasoned_plank", 2)], 40, 40),
    r("furn_weaponrack", 1, Weaponsmithing, 7, &[("seasoned_plank", 4), ("iron_ingot", 2)], 30, 36),
    r("ironwood_blade", 1, Weaponsmithing, 8, &[("seasoned_plank", 4), ("bramble_thorn", 1), ("wisp_ember", 2)], 40, 45),
    r("ironbound_cudgel", 1, Weaponsmithing, 8, &[("seasoned_plank", 4), ("iron_ingot", 2)], 40, 45),
    r("furn_chest", 1, Weaponsmithing, 9, &[("seasoned_plank", 6), ("iron_ingot", 2)], 50, 50),
    r("storm_alloy", 1, Weaponsmithing, 10, &[("iron_ingot", 2), ("thunder_core", 1)], 20, 30),
    r("steel_sword", 1, Weaponsmithing, 12, &[("iron_ingot", 5), ("crab_shell", 3)], 80, 70),
    r("viper_daggers", 1, Weaponsmithing, 12, &[("iron_ingot", 4), ("shadow_silk", 3)], 80, 70),
    r("thunder_knuckles", 1, Weaponsmithing, 12, &[("iron_ingot", 4), ("thunder_core", 2)], 80, 70),
    r("yew_bow", 1, Weaponsmithing, 12, &[("seasoned_plank", 5), ("hawk_feather", 4)], 80, 70),
    r("runed_staff", 1, Weaponsmithing, 12, &[("seasoned_plank", 4), ("star_dust", 3)], 80, 70),
    r("stormguard_saber", 1, Weaponsmithing, 16, &[("storm_alloy", 3), ("crab_shell", 6), ("iron_ingot", 4)], 200, 130),
    r("nightglass_kris", 1, Weaponsmithing, 16, &[("storm_alloy", 3), ("silk_bolt", 2), ("star_dust", 2)], 200, 130),
    r("thunderclap_cestus", 1, Weaponsmithing, 16, &[("storm_alloy", 4), ("thunder_core", 3)], 200, 130),
    r("hawkwing_recurve", 1, Weaponsmithing, 16, &[("storm_alloy", 2), ("seasoned_plank", 6), ("hawk_feather", 8)], 200, 130),
    r("wisplight_rod", 1, Weaponsmithing, 16, &[("storm_alloy", 2), ("seasoned_plank", 4), ("wisp_ember", 6), ("star_dust", 4)], 200, 130),
    r("galeheart_blade", 1, Weaponsmithing, 22, &[("storm_alloy", 6), ("gale_shard", 8), ("storm_horn", 1)], 600, 320),
    r("galeheart_fangs", 1, Weaponsmithing, 22, &[("storm_alloy", 6), ("gale_shard", 8), ("silk_bolt", 3)], 600, 320),
    r("galeheart_knuckles", 1, Weaponsmithing, 22, &[("storm_alloy", 6), ("gale_shard", 8), ("thunder_core", 4)], 600, 320),
    r("galeheart_longbow", 1, Weaponsmithing, 22, &[("storm_alloy", 4), ("gale_shard", 8), ("hawk_feather", 10)], 600, 320),
    r("galeheart_staff", 1, Weaponsmithing, 22, &[("storm_alloy", 4), ("gale_shard", 8), ("star_dust", 8)], 600, 320),
    // ---------------------------------------------------------------- armoursmithing
    r("wool_cloth", 1, Armoursmithing, 1, &[("wool_tuft", 3)], 3, 6),
    r("cloth_tunic", 1, Armoursmithing, 1, &[("wool_cloth", 3)], 10, 15),
    r("sandals", 1, Armoursmithing, 2, &[("wool_cloth", 2), ("sturdy_branch", 2)], 10, 16),
    r("furn_rug", 1, Armoursmithing, 2, &[("wool_cloth", 4)], 10, 16),
    r("leather_cap", 1, Armoursmithing, 3, &[("beak_shard", 4), ("wool_cloth", 1)], 15, 20),
    r("furn_bed", 1, Armoursmithing, 4, &[("wool_cloth", 6), ("seasoned_plank", 4)], 40, 34),
    r("travel_cloak", 1, Armoursmithing, 5, &[("wool_cloth", 4), ("gust_wisp", 3)], 25, 28),
    r("leather_breeches", 1, Armoursmithing, 5, &[("wool_cloth", 3), ("beak_shard", 4)], 25, 28),
    r("leather_vest", 1, Armoursmithing, 6, &[("wool_cloth", 3), ("beak_shard", 6)], 30, 32),
    r("furn_banner", 1, Armoursmithing, 6, &[("wool_cloth", 3), ("gust_wisp", 2)], 20, 28),
    r("trail_boots", 1, Armoursmithing, 8, &[("wool_cloth", 2), ("beak_shard", 4), ("cap_spore", 3)], 40, 45),
    r("silk_bolt", 1, Armoursmithing, 10, &[("shadow_silk", 3)], 15, 30),
    r("chain_shirt", 1, Armoursmithing, 12, &[("iron_ingot", 5), ("wool_cloth", 3)], 80, 70),
    r("chain_leggings", 1, Armoursmithing, 12, &[("iron_ingot", 4), ("wool_cloth", 2)], 70, 65),
    r("iron_helm", 1, Armoursmithing, 12, &[("iron_ingot", 3), ("wool_cloth", 1)], 60, 60),
    r("shadow_garb", 1, Armoursmithing, 12, &[("silk_bolt", 3), ("wool_cloth", 2)], 80, 70),
    r("sage_robe", 1, Armoursmithing, 12, &[("wool_cloth", 5), ("star_dust", 3)], 80, 70),
    r("iron_greaves", 1, Armoursmithing, 14, &[("iron_ingot", 3), ("crab_shell", 3)], 80, 80),
    r("windstep_boots", 1, Armoursmithing, 14, &[("wool_cloth", 3), ("hawk_feather", 4)], 80, 80),
    r("stormglass_pendant", 1, Armoursmithing, 16, &[("storm_alloy", 2), ("star_dust", 4)], 160, 120),
    r("crabshell_band", 1, Armoursmithing, 16, &[("crab_shell", 6), ("iron_ingot", 2)], 160, 120),
    r("plate_armor", 1, Armoursmithing, 18, &[("storm_alloy", 4), ("iron_ingot", 6), ("crab_shell", 4)], 220, 160),
    r("monk_gi", 1, Armoursmithing, 18, &[("silk_bolt", 4), ("wool_cloth", 4), ("thunder_core", 2)], 220, 160),
    r("galeheart_helm", 1, Armoursmithing, 22, &[("storm_alloy", 3), ("gale_shard", 6)], 500, 280),
    r("galeheart_cuirass", 1, Armoursmithing, 22, &[("storm_alloy", 5), ("gale_shard", 8), ("crab_shell", 6)], 500, 300),
    r("galeheart_tassets", 1, Armoursmithing, 22, &[("silk_bolt", 4), ("gale_shard", 6), ("storm_alloy", 2)], 500, 280),
    r("galeheart_treads", 1, Armoursmithing, 22, &[("storm_alloy", 3), ("gale_shard", 6), ("hawk_feather", 6)], 500, 280),
    r("galeheart_mantle", 1, Armoursmithing, 22, &[("silk_bolt", 4), ("gale_shard", 6), ("gust_wisp", 6)], 500, 280),
    r("galeheart_ring", 1, Armoursmithing, 22, &[("storm_alloy", 2), ("gale_shard", 6), ("star_dust", 4)], 500, 280),
    r("galeheart_earring", 1, Armoursmithing, 22, &[("storm_alloy", 2), ("gale_shard", 6), ("wisp_ember", 6)], 500, 280),
    r("galeheart_amulet", 1, Armoursmithing, 22, &[("storm_alloy", 3), ("gale_shard", 8), ("thunder_core", 3)], 500, 300),
    // ---------------------------------------------------------------- cooking
    r("herb_flatbread", 1, Cooking, 1, &[("flour", 1), ("wild_herbs", 2)], 2, 8),
    r("glade_omelette", 1, Cooking, 3, &[("speckled_egg", 2), ("fresh_milk", 1), ("wild_herbs", 1)], 3, 14),
    r("hearty_stew", 1, Cooking, 5, &[("game_meat", 2), ("wild_herbs", 1), ("spice_pouch", 1)], 4, 20),
    r("glowcap_skewers", 1, Cooking, 8, &[("glowcap", 3), ("spice_pouch", 1)], 5, 28),
    r("honey_roast", 1, Cooking, 10, &[("game_meat", 2), ("sky_honey", 1), ("spice_pouch", 1)], 6, 34),
    r("crab_cakes", 1, Cooking, 12, &[("crab_meat", 2), ("flour", 1), ("speckled_egg", 1)], 8, 42),
    r("wayfarers_pie", 1, Cooking, 15, &[("game_meat", 2), ("glowcap", 1), ("flour", 2), ("fresh_milk", 1)], 10, 55),
    r("aether_tea", 1, Cooking, 15, &[("sky_honey", 1), ("wild_herbs", 3), ("star_dust", 1)], 10, 55),
    r("stormberry_tart", 1, Cooking, 18, &[("sky_honey", 2), ("flour", 1), ("fresh_milk", 1), ("thunder_core", 1)], 12, 70),
    r("gale_feast", 1, Cooking, 22, &[("crab_meat", 2), ("game_meat", 2), ("sky_honey", 1), ("gale_shard", 2)], 20, 110),
];

pub fn recipe(output: &str) -> Option<&'static Recipe> {
    RECIPES.iter().find(|r| r.output == output)
}

/// Every recipe of a craft, lowest level first.
pub fn recipes_for(craft: Craft) -> Vec<&'static Recipe> {
    let mut v: Vec<_> = RECIPES.iter().filter(|r| r.craft == craft).collect();
    v.sort_by_key(|r| r.level);
    v
}

/// Crafting EXP a recipe gives at this crafting level: full EXP near the
/// recipe's level, tapering to a fifth once it's 10+ levels below you.
pub fn exp_for(r: &Recipe, level: u32) -> u32 {
    let gap = level.saturating_sub(r.level) as f32;
    let f = (1.0 - gap * 0.08).max(0.2);
    ((r.exp as f32) * f).round().max(1.0) as u32
}

/// How many times this recipe can be made from `have` (count of an item)
/// and `gold`.
pub fn max_crafts(r: &Recipe, have: impl Fn(&str) -> u32, gold: u32) -> u32 {
    let mut n = u32::MAX;
    for &(id, k) in r.inputs {
        n = n.min(have(id) / k.max(1));
    }
    if r.fee > 0 {
        n = n.min(gold / r.fee);
    }
    if n == u32::MAX {
        0
    } else {
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::{item, ItemKind};

    #[test]
    fn recipes_use_real_items() {
        for (i, r) in RECIPES.iter().enumerate() {
            assert!(item(r.output).is_some(), "recipe makes unknown item {}", r.output);
            for (id, n) in r.inputs {
                assert!(item(id).is_some(), "{} needs unknown item {}", r.output, id);
                assert!(*n > 0);
            }
            assert!(r.level >= 1 && r.level <= MAX_CRAFT_LEVEL);
            for o in &RECIPES[i + 1..] {
                assert_ne!(r.output, o.output, "two recipes make {}", r.output);
            }
            if r.craft == Craft::Cooking {
                assert!(matches!(item(r.output).unwrap().kind, ItemKind::Consumable(_) | ItemKind::Material), "{} isn't food", r.output);
            }
        }
        // every craft has something to make at level 1
        for c in CRAFTS {
            assert!(RECIPES.iter().any(|r| r.craft == *c && r.level == 1));
        }
    }

    #[test]
    fn crafting_levels_go_up_and_stop_at_the_cap() {
        let mut s = CraftSkill::default();
        assert_eq!(s.gain(craft_exp_to_next(1) - 1), 0);
        assert_eq!(s.gain(1), 1);
        assert_eq!(s.level, 2);
        s.gain(10_000_000);
        assert_eq!(s.level, MAX_CRAFT_LEVEL);
        assert_eq!(s.gain(500), 0);
    }

    #[test]
    fn low_recipes_give_less_exp() {
        let r = recipe("herb_flatbread").unwrap();
        assert_eq!(exp_for(r, 1), r.exp);
        assert!(exp_for(r, 20) < r.exp);
        assert!(exp_for(r, 30) >= 1);
    }

    #[test]
    fn stations() {
        assert_eq!(station_for_npc("garrick"), Some(Station::Forge));
        assert_eq!(station_for_npc("ironhold_weapons"), Some(Station::Weaponsmith));
        assert_eq!(station_for_npc("starfall_armor"), Some(Station::Armourer));
        assert_eq!(station_for_npc("inn_tempest"), Some(Station::Kitchen));
        assert_eq!(station_for_npc("ironhold_general"), None);
        assert!(Station::Forge.crafts().contains(&Craft::Armoursmithing));
    }
}
