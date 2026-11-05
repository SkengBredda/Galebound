//! Items: consumables, weapons, armour, boots and accessories, plus the
//! inventory/equipment model and shop catalogues.

use crate::classes::ClassId;
use crate::stats::{MainStats, StatMods};

pub const INVENTORY_SLOTS: usize = 24;
pub const STACK_MAX: u32 = 99;
/// Shops buy items back for this fraction of their base price.
pub const SELL_RATE: f32 = 0.4;
/// Everything costs this many times its base price to buy from a shop
/// (0.13.0: prices went up by 800%). What shops pay you is unchanged.
pub const PRICE_MULT: u32 = 5;
/// The tutorial outfitter's potions: this many purchases are discounted...
pub const TRAINEE_DISCOUNT_BUYS: u32 = 10;
/// ...by this fraction (40% off).
pub const TRAINEE_DISCOUNT: f32 = 0.4;

/// What kind of gear an item is. Rings and earrings fit either of their
/// two slots.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GearKind {
    Weapon,
    Helmet,
    Torso,
    Legs,
    Boots,
    Necklace,
    Ring,
    Earring,
    Back,
}

impl GearKind {
    pub fn name(self) -> &'static str {
        match self {
            GearKind::Weapon => "Weapon",
            GearKind::Helmet => "Helmet",
            GearKind::Torso => "Torso",
            GearKind::Legs => "Legs",
            GearKind::Boots => "Boots",
            GearKind::Necklace => "Necklace",
            GearKind::Ring => "Ring",
            GearKind::Earring => "Earring",
            GearKind::Back => "Back",
        }
    }
    /// The equipment slots this kind of gear can go in.
    pub fn slots(self) -> &'static [EquipSlot] {
        use EquipSlot as E;
        match self {
            GearKind::Weapon => &[E::Weapon],
            GearKind::Helmet => &[E::Helmet],
            GearKind::Torso => &[E::Torso],
            GearKind::Legs => &[E::Legs],
            GearKind::Boots => &[E::Boots],
            GearKind::Necklace => &[E::Necklace],
            GearKind::Ring => &[E::Ring1, E::Ring2],
            GearKind::Earring => &[E::Earring1, E::Earring2],
            GearKind::Back => &[E::Back],
        }
    }
}

/// Where gear is worn: eleven slots, two each for rings and earrings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EquipSlot {
    Weapon,
    Helmet,
    Torso,
    Legs,
    Boots,
    Necklace,
    Ring1,
    Ring2,
    Earring1,
    Earring2,
    Back,
}

pub const EQUIP_SLOTS: &[EquipSlot] = &[
    EquipSlot::Weapon,
    EquipSlot::Helmet,
    EquipSlot::Torso,
    EquipSlot::Legs,
    EquipSlot::Boots,
    EquipSlot::Necklace,
    EquipSlot::Ring1,
    EquipSlot::Ring2,
    EquipSlot::Earring1,
    EquipSlot::Earring2,
    EquipSlot::Back,
];

/// Number of equipment slots.
pub const EQUIP_COUNT: usize = 11;

impl EquipSlot {
    pub fn index(self) -> usize {
        EQUIP_SLOTS.iter().position(|s| *s == self).unwrap()
    }
    pub fn kind(self) -> GearKind {
        match self {
            EquipSlot::Weapon => GearKind::Weapon,
            EquipSlot::Helmet => GearKind::Helmet,
            EquipSlot::Torso => GearKind::Torso,
            EquipSlot::Legs => GearKind::Legs,
            EquipSlot::Boots => GearKind::Boots,
            EquipSlot::Necklace => GearKind::Necklace,
            EquipSlot::Ring1 | EquipSlot::Ring2 => GearKind::Ring,
            EquipSlot::Earring1 | EquipSlot::Earring2 => GearKind::Earring,
            EquipSlot::Back => GearKind::Back,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            EquipSlot::Ring1 => "Ring I",
            EquipSlot::Ring2 => "Ring II",
            EquipSlot::Earring1 => "Earring I",
            EquipSlot::Earring2 => "Earring II",
            s => s.kind().name(),
        }
    }
    /// A short label for tight slot grids.
    pub fn short(self) -> &'static str {
        match self {
            EquipSlot::Weapon => "Weapon",
            EquipSlot::Helmet => "Head",
            EquipSlot::Torso => "Torso",
            EquipSlot::Legs => "Legs",
            EquipSlot::Boots => "Boots",
            EquipSlot::Necklace => "Neck",
            EquipSlot::Ring1 | EquipSlot::Ring2 => "Ring",
            EquipSlot::Earring1 | EquipSlot::Earring2 => "Ear",
            EquipSlot::Back => "Back",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            EquipSlot::Weapon => "weapon",
            EquipSlot::Helmet => "helmet",
            EquipSlot::Torso => "torso",
            EquipSlot::Legs => "legs",
            EquipSlot::Boots => "boots",
            EquipSlot::Necklace => "necklace",
            EquipSlot::Ring1 => "ring1",
            EquipSlot::Ring2 => "ring2",
            EquipSlot::Earring1 => "earring1",
            EquipSlot::Earring2 => "earring2",
            EquipSlot::Back => "back",
        }
    }
    pub fn from_key(s: &str) -> Option<EquipSlot> {
        EQUIP_SLOTS.iter().copied().find(|e| e.key() == s)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Effect {
    /// Restore a fraction of max HP / MP.
    Restore { hp: f32, mp: f32 },
    /// Temporary stat boost (seconds).
    Buff { mods: StatMods, secs: f32 },
    /// A meal: a long food buff (one at a time; a new meal replaces the
    /// old one). `hp_regen` / `mp_regen` are per second.
    Food { mods: StatMods, secs: f32, hp_regen: f32, mp_regen: f32 },
}

/// Items with a one-off use that isn't a potion or a buff.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Special {
    /// Refunds every spent ability point.
    SkillReset,
    /// Get back up where you fell, with no EXP loss.
    Revive,
    /// Adopt this pet.
    Pet(crate::pets::PetSpecies),
    /// Feeds your active pet (hunger and pet EXP).
    PetFood,
    /// Unlocks your own house in Breezehollow.
    HouseDeed,
    /// A dungeon reward cache: open it for gear and materials.
    Cache,
}

/// Furniture for your house (see `housing`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Furniture {
    /// Sleep for a full heal and the Well Rested bonus.
    Bed,
    /// Opens your house storage.
    Chest,
    /// Cook at home.
    Stove,
    /// Forge weapons and armour at home.
    Workbench,
    Table,
    Bookshelf,
    Plant,
    Rug,
    Fireplace,
    WeaponRack,
    ArmorStand,
    Barrel,
    Banner,
    Lamp,
    Trophy,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ItemKind {
    Consumable(Effect),
    Equip(GearKind),
    /// Monster loot: stacks, sells to any shop.
    Material,
    Special(Special),
    /// Furniture: place it in your house.
    Furniture(Furniture),
}

/// What a piece of gear looks like on the character (art hint, used by the client).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Look {
    None,
    /// Overrides the outfit top with (top style, dye).
    Top(u8, u8),
    /// Overrides shoes with (style).
    Shoes(u8),
    /// Overrides the outfit bottom with (bottom style, dye).
    Bottom(u8, u8),
    /// A hat / helmet style (see `appearance::HATS`).
    Hat(u8),
    /// Something worn on the back (see `appearance::BACKS`).
    Back(u8),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ItemDef {
    pub id: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    pub kind: ItemKind,
    pub price: u32,
    pub level: u32,
    /// Only this class may equip it (weapons).
    pub class: Option<ClassId>,
    pub mods: StatMods,
    pub look: Look,
    /// Rarity: 0 common, 1 uncommon, 2 rare, 3 epic, 4 Gold, 5 Red
    /// (Gold and Red come from co-op dungeons).
    pub tier: u8,
    /// Bound to the character: can't be sold, traded or dropped.
    pub bound: bool,
}

impl ItemDef {
    pub fn stackable(&self) -> bool {
        matches!(self.kind, ItemKind::Consumable(_) | ItemKind::Material | ItemKind::Special(_) | ItemKind::Furniture(_))
    }
    pub fn can_sell(&self) -> bool {
        self.price > 0 && !self.bound
    }
    /// What a shop charges for one (see [`PRICE_MULT`]).
    pub fn buy_price(&self) -> u32 {
        self.price.saturating_mul(PRICE_MULT)
    }
    /// The rarity's name.
    pub fn tier_name(&self) -> &'static str {
        tier_name(self.tier)
    }
    /// Trading between players will check this.
    pub fn can_trade(&self) -> bool {
        !self.bound
    }
    pub fn can_drop(&self) -> bool {
        !self.bound
    }
    /// What kind of gear this is (None for anything you can't wear).
    pub fn slot(&self) -> Option<GearKind> {
        match self.kind {
            ItemKind::Equip(s) => Some(s),
            _ => None,
        }
    }
    pub fn sell_price(&self) -> u32 {
        ((self.price as f32) * SELL_RATE).floor() as u32
    }
    /// Short stat lines for tooltips, e.g. ["+12 P.ATK", "+4 LUK"].
    pub fn stat_lines(&self) -> Vec<String> {
        let food = match self.kind {
            ItemKind::Consumable(Effect::Food { mods, .. }) => Some(mods),
            _ => None,
        };
        let m = food.as_ref().unwrap_or(&self.mods);
        let flat = [
            (m.patk, "P.ATK"),
            (m.matk, "M.ATK"),
            (m.pdef, "P.DEF"),
            (m.mdef, "M.DEF"),
            (m.hp, "HP"),
            (m.mp, "Mana"),
            (m.main.str as f32, "STR"),
            (m.main.dex as f32, "DEX"),
            (m.main.int as f32, "INT"),
            (m.main.luk as f32, "LUK"),
        ];
        let mut v: Vec<String> = flat.iter().filter(|(x, _)| *x != 0.0).map(|(x, l)| format!("{:+} {}", x.round() as i32, l)).collect();
        if m.crit != 0.0 {
            v.push(format!("{:+}% Crit Chance", m.crit.round() as i32));
        }
        if m.crit_dmg != 0.0 {
            v.push(format!("{:+}% Crit Damage", m.crit_dmg.round() as i32));
        }
        for (x, l) in [(m.atk_pct, "ATK"), (m.pdef_pct, "P.DEF"), (m.mdef_pct, "M.DEF"), (m.speed_pct, "Move Speed")] {
            if x != 0.0 {
                v.push(format!("{:+}% {}", (x * 100.0).round() as i32, l));
            }
        }
        if let ItemKind::Consumable(Effect::Food { secs, hp_regen, mp_regen, .. }) = self.kind {
            if hp_regen > 0.0 {
                v.push(format!("+{} HP per second", hp_regen));
            }
            if mp_regen > 0.0 {
                v.push(format!("+{} Mana per second", mp_regen));
            }
            v.push(format!("Lasts {} minutes", (secs / 60.0).round() as i32));
        }
        v
    }
}

const fn ms(str: i32, dex: i32, int: i32, luk: i32) -> MainStats {
    MainStats { str, dex, int, luk }
}

const Z: StatMods = StatMods {
    main: ms(0, 0, 0, 0),
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

const fn material(id: &'static str, name: &'static str, desc: &'static str, price: u32, tier: u8) -> ItemDef {
    ItemDef { id, name, desc, kind: ItemKind::Material, price, level: 1, class: None, mods: Z, look: Look::None, tier, bound: false }
}

const fn consumable(id: &'static str, name: &'static str, desc: &'static str, price: u32, level: u32, effect: Effect, tier: u8) -> ItemDef {
    ItemDef { id, name, desc, kind: ItemKind::Consumable(effect), price, level, class: None, mods: Z, look: Look::None, tier, bound: false }
}

#[allow(clippy::too_many_arguments)]
const fn weapon(id: &'static str, name: &'static str, desc: &'static str, class: ClassId, price: u32, level: u32, mods: StatMods, tier: u8) -> ItemDef {
    ItemDef { id, name, desc, kind: ItemKind::Equip(GearKind::Weapon), price, level, class: Some(class), mods, look: Look::None, tier, bound: false }
}

#[allow(clippy::too_many_arguments)]
const fn gear(id: &'static str, name: &'static str, desc: &'static str, slot: GearKind, price: u32, level: u32, mods: StatMods, look: Look, tier: u8) -> ItemDef {
    ItemDef { id, name, desc, kind: ItemKind::Equip(slot), price, level, class: None, mods, look, tier, bound: false }
}

const fn special(id: &'static str, name: &'static str, desc: &'static str, price: u32, kind: Special, tier: u8) -> ItemDef {
    ItemDef { id, name, desc, kind: ItemKind::Special(kind), price, level: 1, class: None, mods: Z, look: Look::None, tier, bound: false }
}

/// Mark an item as bound (no selling, trading or dropping).
const fn bound(mut d: ItemDef) -> ItemDef {
    d.bound = true;
    d
}

const fn patk(v: f32, crit: f32) -> StatMods {
    StatMods { patk: v, crit, ..Z }
}
const fn matk(v: f32, mp: f32) -> StatMods {
    StatMods { matk: v, mp, ..Z }
}
const fn def(p: f32, m: f32, hp: f32) -> StatMods {
    StatMods { pdef: p, mdef: m, hp, ..Z }
}


#[allow(clippy::too_many_arguments)]
const fn food(id: &'static str, name: &'static str, desc: &'static str, price: u32, mods: StatMods, mins: f32, hp_regen: f32, mp_regen: f32, tier: u8) -> ItemDef {
    ItemDef { id, name, desc, kind: ItemKind::Consumable(Effect::Food { mods, secs: mins * 60.0, hp_regen, mp_regen }), price, level: 1, class: None, mods: Z, look: Look::None, tier, bound: false }
}

const fn furn(id: &'static str, name: &'static str, desc: &'static str, price: u32, f: Furniture, tier: u8) -> ItemDef {
    ItemDef { id, name, desc, kind: ItemKind::Furniture(f), price, level: 1, class: None, mods: Z, look: Look::None, tier, bound: false }
}

/// The name of a rarity tier.
pub fn tier_name(tier: u8) -> &'static str {
    match tier {
        0 => "Common",
        1 => "Uncommon",
        2 => "Rare",
        3 => "Epic",
        4 => "Gold",
        _ => "Red",
    }
}

/// Gold-tier gear a dungeon cache can hold.
pub const GOLD_LOOT: &[&str] = &[
    "galeheart_blade", "galeheart_fangs", "galeheart_knuckles", "galeheart_longbow", "galeheart_staff",
    "galeheart_helm", "galeheart_cuirass", "galeheart_tassets", "galeheart_treads", "galeheart_mantle",
    "galeheart_ring", "galeheart_earring", "galeheart_amulet",
];
/// Red-tier gear: only from the Tempest Vault's caches.
pub const RED_LOOT: &[&str] = &[
    "crimson_tempest_blade", "crimson_tempest_fangs", "crimson_tempest_knuckles", "crimson_tempest_bow", "crimson_tempest_staff",
    "crimson_tempest_helm", "crimson_tempest_cuirass", "crimson_tempest_greaves", "crimson_tempest_sabatons", "crimson_tempest_cloak",
    "crimson_tempest_ring", "crimson_tempest_earring", "crimson_tempest_amulet",
];

use ClassId::*;
use GearKind::*;

pub const ITEMS: &[ItemDef] = &[
    // ---------------------------------------------------------------- consumables
    consumable("hp_potion", "Red Potion", "Restores 40% of your HP.", 15, 1, Effect::Restore { hp: 0.40, mp: 0.0 }, 0),
    consumable("hp_potion_big", "Crimson Draught", "Restores 70% of your HP.", 45, 8, Effect::Restore { hp: 0.70, mp: 0.0 }, 1),
    consumable("mp_potion", "Blue Potion", "Restores 40% of your Mana.", 18, 1, Effect::Restore { hp: 0.0, mp: 0.40 }, 0),
    consumable("mp_potion_big", "Azure Ether", "Restores 70% of your Mana.", 55, 8, Effect::Restore { hp: 0.0, mp: 0.70 }, 1),
    consumable("elixir", "Gale Elixir", "Restores 60% of HP and Mana.", 140, 12, Effect::Restore { hp: 0.60, mp: 0.60 }, 2),
    consumable("might_tonic", "Might Tonic", "+15% attack for 60 seconds.", 70, 5, Effect::Buff { mods: StatMods { atk_pct: 0.15, ..Z }, secs: 60.0 }, 1),
    consumable("iron_tonic", "Ironbark Tonic", "+30% P.DEF and M.DEF for 60 seconds.", 70, 5, Effect::Buff { mods: StatMods { pdef_pct: 0.3, mdef_pct: 0.3, ..Z }, secs: 60.0 }, 1),
    bound(consumable("trainee_hp_potion", "Trainee's Red Potion", "Restores 40% of your HP. A gift for new adventurers.", 15, 1, Effect::Restore { hp: 0.40, mp: 0.0 }, 0)),
    bound(consumable("trainee_mp_potion", "Trainee's Blue Potion", "Restores 40% of your Mana. A gift for new adventurers.", 18, 1, Effect::Restore { hp: 0.0, mp: 0.40 }, 0)),
    consumable("lucky_clover", "Lucky Clover Tea", "+10% Crit Chance for 60 seconds.", 90, 8, Effect::Buff { mods: StatMods { crit: 10.0, ..Z }, secs: 60.0 }, 2),
    // ---------------------------------------------------------------- special
    special("revive_scroll", "Revive Scroll", "Use while defeated to get back up where you fell, with no EXP lost.", 60, Special::Revive, 2),
    special("pet_breeze_fox", "Breeze Fox", "A quick, curious fox kit. Use it to adopt your pet. It loots for you and grows on Pet Food: finds more loot and gold.", crate::pets::PET_PRICE, Special::Pet(crate::pets::PetSpecies::BreezeFox), 3),
    special("pet_puffling", "Puffling", "A lamb made of cloud. Use it to adopt your pet. It loots for you and grows on Pet Food: defence and healing.", crate::pets::PET_PRICE, Special::Pet(crate::pets::PetSpecies::Puffling), 3),
    special("pet_sparkwing", "Sparkwing", "A tiny bird that crackles with static. Use it to adopt your pet. It loots for you and grows on Pet Food: attack and Mana.", crate::pets::PET_PRICE, Special::Pet(crate::pets::PetSpecies::Sparkwing), 3),
    special("pet_food", "Pet Food", "Feeds your pet: +30 hunger and pet EXP. Use it from the bag, the hotbar or the Pets window.", crate::pets::FOOD_PRICE, Special::PetFood, 1),
    bound(special("skill_reset_coupon", "Skill Reset Coupon", "Refunds every ability point you've spent so you can rebuild your stats.", 0, Special::SkillReset, 3)),
    // ---------------------------------------------------------------- monster loot (sell it)
    material("moss_clump", "Moss Clump", "Soft and damp. Herbalists buy these.", 6, 0),
    material("beak_shard", "Beak Shard", "A chipped piece of a Burrowbeak's beak.", 9, 0),
    material("bramble_thorn", "Bramblehorn Thorn", "A trophy from the Old Gate's guardian.", 80, 2),
    material("wool_tuft", "Wool Tuft", "Fluffy. Smells faintly of thunder.", 10, 0),
    material("gust_wisp", "Gust Wisp", "A little breeze in a bottle. Don't open it.", 14, 0),
    material("cap_spore", "Grave Spores", "Glows faintly in the dark.", 16, 1),
    material("wisp_ember", "Wisp Ember", "Still warm.", 18, 1),
    material("rotten_bark", "Rotten Bark", "Crumbles if you squeeze it.", 20, 1),
    material("crab_shell", "Crab Shell", "Hard as stone. Armourers love it.", 26, 1),
    material("hawk_feather", "Galehawk Feather", "Fletchers pay well for these.", 28, 1),
    material("storm_horn", "Storm Horn", "Crackles with trapped lightning.", 300, 3),
    material("iron_ore", "Iron Ore", "Raw ore from the roadside rocks.", 34, 1),
    material("shadow_silk", "Shadow Silk", "Thread spun from twilight.", 40, 2),
    material("star_dust", "Star Dust", "Glitters even in your pocket.", 48, 2),
    material("thunder_core", "Thunder Core", "It hums. It bites.", 55, 2),
    // ---------------------------------------------------------------- weapons
    weapon("gnarled_stick", "Gnarled Stick", "Every hero's first weapon. It's... a stick.", Novice, 0, 1, patk(3.0, 0.0), 0),
    weapon("wood_sword", "Wooden Sword", "A carved practice sword. Light and quick.", Novice, 30, 1, patk(5.0, 1.0), 0),
    weapon("wood_club", "Wooden Club", "A sturdy branch with a grip wrapped in cloth.", Novice, 25, 1, patk(6.0, 0.0), 0),
    weapon("novice_sword", "Novice's Practice Sword", "Sergeant Brask's spare. Balanced for beginners.", Novice, 12, 1, patk(5.0, 2.0), 0),
    weapon("ironwood_blade", "Ironwood Blade", "Carved from ironwood, hard as steel and half the weight.", Novice, 180, 8, patk(12.0, 3.0), 1),
    weapon("ironbound_cudgel", "Ironbound Cudgel", "An oak cudgel wrapped in iron bands.", Novice, 170, 8, patk(14.0, 0.0), 1),
    weapon("oak_club", "Studded Oak Club", "Iron studs make every bonk count.", Novice, 60, 5, patk(9.0, 0.0), 1),
    weapon("iron_sword", "Ironhold Arming Sword", "Standard issue of the Ironhold guard, with a kite shield.", Vanguard, 0, 10, StatMods { patk: 14.0, pdef: 4.0, ..Z }, 1),
    weapon("steel_sword", "Steel Longsword", "Balanced steel and a riveted heater shield.", Vanguard, 260, 14, StatMods { patk: 22.0, pdef: 7.0, ..Z }, 2),
    weapon("gale_sword", "Galeforged Blade", "Folded with wind-iron from the Windcrag.", Vanguard, 620, 18, StatMods { patk: 32.0, pdef: 10.0, hp: 40.0, ..Z }, 3),
    weapon("dusk_daggers", "Dusk Fangs", "A pair of blackened knives.", Duskblade, 0, 10, patk(12.0, 3.0), 1),
    weapon("viper_daggers", "Viper Tongues", "Wavy blades that find the gaps in armour.", Duskblade, 260, 14, patk(19.0, 5.0), 2),
    weapon("eclipse_daggers", "Eclipse Edge", "Forged during a moonless night in Shadowmere.", Duskblade, 620, 18, StatMods { patk: 28.0, crit: 7.0, crit_dmg: 20.0, ..Z }, 3),
    weapon("storm_wraps", "Storm Wraps", "Copper-lined hand wraps that tingle with static.", Stormfist, 0, 10, patk(13.0, 1.0), 1),
    weapon("thunder_knuckles", "Thunder Knuckles", "Brass knuckles etched with storm sigils.", Stormfist, 260, 14, patk(21.0, 2.0), 2),
    weapon("tempest_gauntlets", "Tempest Gauntlets", "The monastery's master gauntlets.", Stormfist, 620, 18, StatMods { patk: 30.0, crit: 3.0, speed_pct: 0.05, ..Z }, 3),
    weapon("ash_bow", "Ashwood Shortbow", "Light, quick and forgiving.", Windrunner, 0, 10, patk(12.0, 2.0), 1),
    weapon("yew_bow", "Leafwhisper Yew Bow", "Strung with spider silk from the canopy.", Windrunner, 260, 14, patk(20.0, 3.0), 2),
    weapon("galewing_bow", "Galewing Longbow", "Arrows loosed from it ride the wind.", Windrunner, 620, 18, StatMods { patk: 30.0, crit: 5.0, main: ms(0, 4, 0, 0), ..Z }, 3),
    weapon("apprentice_staff", "Apprentice Staff", "A plain staff with a chip of aether crystal.", Aethermancer, 0, 10, matk(14.0, 10.0), 1),
    weapon("runed_staff", "Runed Staff", "Runes carved along its length glow when you cast.", Aethermancer, 260, 14, matk(23.0, 25.0), 2),
    weapon("starfall_staff", "Starfall Staff", "Topped with a shard that fell from the sky.", Aethermancer, 620, 18, StatMods { matk: 34.0, mp: 40.0, main: ms(0, 0, 4, 0), ..Z }, 3),
    // ---------------------------------------------------------------- torso
    gear("novice_garb", "Adventurer's Garb", "Linen shirt, rope belt and a satchel for the road ahead.", Torso, 0, 1, def(2.0, 2.0, 0.0), Look::Top(8, 5), 0),
    gear("novice_tunic", "Novice's Tunic", "Ranger Pell's old tunic, patched at the elbows.", Torso, 12, 1, def(3.0, 3.0, 10.0), Look::Top(1, 7), 0),
    gear("trail_gambeson", "Padded Gambeson", "Quilted layers that soak up blows.", Torso, 200, 8, def(12.0, 6.0, 22.0), Look::Top(2, 3), 1),
    gear("cloth_tunic", "Traveller's Tunic", "Simple and comfortable.", Torso, 25, 1, def(3.0, 2.0, 0.0), Look::Top(1, 1), 0),
    gear("leather_vest", "Leather Jerkin", "Boiled leather over linen.", Torso, 90, 5, def(8.0, 4.0, 15.0), Look::Top(2, 0), 1),
    gear("mage_robe", "Apprentice Robe", "Threaded with silver to ward off spells.", Torso, 110, 5, def(4.0, 10.0, 0.0), Look::Top(4, 4), 1),
    gear("chain_shirt", "Chain Shirt", "Rings of iron under a surcoat.", Torso, 240, 10, def(16.0, 7.0, 30.0), Look::Top(5, 8), 2),
    gear("shadow_garb", "Shadowsilk Jacket", "Dark, light and quiet.", Torso, 240, 10, StatMods { pdef: 11.0, mdef: 9.0, crit: 2.0, ..Z }, Look::Top(6, 9), 2),
    gear("sage_robe", "Sage's Robe", "Embroidered with protective glyphs.", Torso, 260, 10, StatMods { pdef: 7.0, mdef: 18.0, mp: 30.0, ..Z }, Look::Top(4, 6), 2),
    gear("plate_armor", "Galeplate Cuirass", "Heavy plate with a wind-sigil on the chest.", Torso, 560, 15, def(26.0, 12.0, 60.0), Look::Top(5, 3), 3),
    gear("monk_gi", "Tempest Gi", "Training robes of the monastery.", Torso, 520, 15, StatMods { pdef: 18.0, mdef: 14.0, speed_pct: 0.04, ..Z }, Look::Top(3, 2), 3),
    // ---------------------------------------------------------------- helmets
    gear("novice_bandana", "Novice's Bandana", "A knotted headband. Keeps the hair out of your eyes.", Helmet, 10, 1, def(1.0, 1.0, 5.0), Look::Hat(1), 0),
    gear("leather_cap", "Leather Cap", "A snug cap of boiled leather.", Helmet, 60, 5, def(3.0, 1.0, 10.0), Look::Hat(2), 1),
    gear("trail_hat", "Trailblazer's Hat", "A wide-brimmed hat with a jaunty feather.", Helmet, 140, 8, StatMods { pdef: 5.0, mdef: 3.0, hp: 15.0, ..Z }, Look::Hat(5), 1),
    gear("iron_helm", "Iron Sallet", "Riveted iron, open-faced for a clear view.", Helmet, 220, 10, def(8.0, 3.0, 25.0), Look::Hat(4), 2),
    gear("sage_hat", "Pointed Sage Hat", "Every spell is easier under a proper hat.", Helmet, 220, 10, StatMods { pdef: 3.0, mdef: 9.0, mp: 20.0, ..Z }, Look::Hat(3), 2),
    gear("ranger_hood", "Ranger's Hood", "A deep green hood that hides your eyes.", Helmet, 220, 10, StatMods { pdef: 5.0, mdef: 5.0, crit: 1.0, ..Z }, Look::Hat(6), 2),
    gear("gale_circlet", "Galewind Circlet", "A silver band set with a storm-blue stone.", Helmet, 520, 15, StatMods { pdef: 12.0, mdef: 12.0, hp: 30.0, main: ms(1, 1, 1, 1), ..Z }, Look::Hat(7), 3),
    // ---------------------------------------------------------------- legs
    gear("novice_trousers", "Novice's Trousers", "Sturdy trousers with patched knees.", Legs, 10, 1, def(2.0, 1.0, 5.0), Look::Bottom(1, 4), 0),
    gear("leather_breeches", "Leather Breeches", "Supple leather that moves with you.", Legs, 70, 5, def(5.0, 2.0, 10.0), Look::Bottom(1, 9), 1),
    gear("trail_cargos", "Trailblazer Cargos", "Pockets for everything. Even more pockets.", Legs, 160, 8, StatMods { pdef: 6.0, mdef: 3.0, hp: 15.0, ..Z }, Look::Bottom(4, 0), 1),
    gear("chain_leggings", "Chain Leggings", "Mail from hip to ankle.", Legs, 230, 10, def(10.0, 4.0, 25.0), Look::Bottom(1, 8), 2),
    gear("shadow_leggings", "Shadowsilk Leggings", "Fitted and silent.", Legs, 230, 10, StatMods { pdef: 7.0, mdef: 6.0, crit: 1.0, ..Z }, Look::Bottom(6, 8), 2),
    gear("sage_hakama", "Glyphwoven Hakama", "Wide, pleated and warded.", Legs, 240, 10, StatMods { pdef: 4.0, mdef: 11.0, mp: 15.0, ..Z }, Look::Bottom(5, 6), 2),
    gear("gale_greaves", "Galeplate Tassets", "Plate over padded trousers.", Legs, 500, 15, def(16.0, 8.0, 40.0), Look::Bottom(4, 1), 3),
    // ---------------------------------------------------------------- boots
    gear("novice_boots", "Novice's Boots", "Tamsin's scouting boots. Light and quiet.", Boots, 12, 1, StatMods { pdef: 2.0, mdef: 1.0, speed_pct: 0.02, ..Z }, Look::Shoes(0), 0),
    gear("trail_boots", "Trailblazer Boots", "Sturdy boots for long roads.", Boots, 150, 8, StatMods { pdef: 6.0, mdef: 3.0, speed_pct: 0.03, ..Z }, Look::Shoes(3), 1),
    gear("sandals", "Woven Sandals", "Better than bare feet.", Boots, 15, 1, def(1.0, 1.0, 0.0), Look::Shoes(2), 0),
    gear("leather_boots", "Leather Boots", "Laced, worn-in and dependable.", Boots, 70, 5, StatMods { pdef: 4.0, mdef: 2.0, speed_pct: 0.02, ..Z }, Look::Shoes(0), 1),
    gear("windstep_boots", "Windstep Boots", "Feathers stitched at the heel.", Boots, 220, 10, StatMods { pdef: 6.0, mdef: 4.0, speed_pct: 0.06, ..Z }, Look::Shoes(5), 2),
    gear("iron_greaves", "Iron Greaves", "Heavy, but nothing gets through.", Boots, 260, 12, def(12.0, 5.0, 25.0), Look::Shoes(3), 2),
    // ---------------------------------------------------------------- necklaces
    gear("novice_charm", "Novice's Wind Charm", "Elder Wynn's blessing, carved into a pebble on a cord.", Necklace, 10, 1, StatMods { main: ms(0, 1, 0, 1), ..Z }, Look::None, 0),
    gear("fang_necklace", "Wolf Fang Necklace", "A lucky fang on a leather thong.", Necklace, 130, 8, StatMods { main: ms(2, 1, 0, 0), patk: 2.0, ..Z }, Look::None, 1),
    gear("sage_pendant", "Sage's Pendant", "+3 INT, +20 Mana.", Necklace, 120, 6, StatMods { main: ms(0, 0, 3, 0), mp: 20.0, ..Z }, Look::None, 1),
    gear("storm_amulet", "Storm Amulet", "A sliver of trapped lightning.", Necklace, 480, 14, StatMods { main: ms(2, 2, 2, 2), crit: 3.0, atk_pct: 0.05, ..Z }, Look::None, 3),
    // ---------------------------------------------------------------- rings
    gear("copper_ring", "Copper Ring", "A plain band. Feels lucky.", Ring, 40, 1, StatMods { main: ms(0, 0, 0, 2), ..Z }, Look::None, 0),
    gear("novice_ring", "Novice's Signet", "Stamped with the Glade's wind-sigil.", Ring, 10, 1, StatMods { main: ms(1, 0, 1, 0), ..Z }, Look::None, 0),
    gear("power_band", "Band of Might", "+3 STR.", Ring, 120, 6, StatMods { main: ms(3, 0, 0, 0), hp: 15.0, ..Z }, Look::None, 1),
    gear("silver_ring", "Silver Ring", "Polished to a mirror shine.", Ring, 150, 8, StatMods { main: ms(1, 1, 1, 1), mdef: 2.0, ..Z }, Look::None, 1),
    gear("sage_ring", "Ring of Insight", "+3 INT, +3 M.ATK.", Ring, 200, 10, StatMods { main: ms(0, 0, 3, 0), matk: 3.0, ..Z }, Look::None, 2),
    gear("gale_ring", "Ring of Gales", "The wind wraps around your fingers.", Ring, 460, 14, StatMods { main: ms(2, 2, 0, 0), speed_pct: 0.03, crit: 2.0, ..Z }, Look::None, 3),
    // ---------------------------------------------------------------- earrings
    gear("pebble_studs", "Pebble Studs", "Smooth river pebbles set in tin.", Earring, 10, 1, StatMods { mdef: 1.0, mp: 5.0, ..Z }, Look::None, 0),
    gear("hunter_charm", "Hunter's Feather Earring", "+3 DEX, +2% Crit.", Earring, 120, 6, StatMods { main: ms(0, 3, 0, 0), crit: 2.0, ..Z }, Look::None, 1),
    gear("rogue_coin", "Rogue's Coin Earring", "+4 LUK, +10% Crit Damage.", Earring, 140, 6, StatMods { main: ms(0, 0, 0, 4), crit_dmg: 10.0, ..Z }, Look::None, 1),
    gear("brass_hoops", "Brass Hoops", "Bright and loud, like their owner.", Earring, 130, 8, StatMods { main: ms(0, 1, 0, 2), mdef: 2.0, ..Z }, Look::None, 1),
    gear("star_drop", "Stardrop Earring", "A falling star caught in glass.", Earring, 440, 14, StatMods { main: ms(0, 0, 2, 2), mp: 25.0, crit_dmg: 8.0, ..Z }, Look::None, 3),
    // ---------------------------------------------------------------- back
    gear("travel_cloak", "Traveller's Cloak", "Keeps the rain off and the wind at your back.", Back, 80, 5, StatMods { mdef: 3.0, hp: 10.0, speed_pct: 0.01, ..Z }, Look::Back(1), 1),
    gear("trail_pack", "Trailblazer's Pack", "Bedroll, rope, snacks. Mostly snacks.", Back, 140, 8, StatMods { pdef: 3.0, hp: 25.0, ..Z }, Look::Back(4), 1),
    gear("gale_cape", "Galeborne Cape", "Billows even when there's no wind.", Back, 260, 10, StatMods { pdef: 4.0, mdef: 6.0, speed_pct: 0.03, ..Z }, Look::Back(2), 2),
    gear("feather_wings", "Featherdown Wings", "Too small to fly. Not too small to look amazing.", Back, 560, 15, StatMods { mdef: 8.0, speed_pct: 0.05, main: ms(0, 2, 2, 0), ..Z }, Look::Back(3), 3),
    // ---------------------------------------------------------------- 0.13.0: crafting materials
    material("sturdy_branch", "Sturdy Branch", "Straight and seasoned by the wind. Woodworkers turn these into planks.", 4, 0),
    material("seasoned_plank", "Seasoned Plank", "Crafted from Sturdy Branches. Used for wooden weapons and furniture.", 30, 1),
    material("iron_ingot", "Iron Ingot", "Smelted from Iron Ore at any forge.", 40, 1),
    material("storm_alloy", "Storm Alloy", "Iron folded around a Thunder Core. Hums faintly.", 120, 2),
    material("wool_cloth", "Wool Cloth", "Woven from Wool Tufts. The base of most clothing.", 14, 0),
    material("silk_bolt", "Shadowsilk Bolt", "A bolt of cloth spun from Shadow Silk.", 60, 2),
    material("gale_shard", "Gale Shard", "A splinter of bottled storm from the Tempest Vault. Forges Gold-tier gear.", 90, 4),
    material("crimson_core", "Crimson Core", "The Stormwarden's still-beating heart of lightning.", 400, 5),
    // ---------------------------------------------------------------- 0.13.0: cooking ingredients
    material("wild_herbs", "Wild Herbs", "Fragrant leaves from the roadside. A cook's best friend.", 3, 0),
    material("game_meat", "Wild Game", "Fresh meat from the fields. Cook it before eating!", 6, 0),
    material("speckled_egg", "Speckled Egg", "Laid by a very annoyed bird.", 5, 0),
    material("glowcap", "Glowcap Mushroom", "Glows in the pot as well as in the woods.", 8, 1),
    material("crab_meat", "Crab Meat", "Sweet and tender, once you get past the shell.", 12, 1),
    material("sky_honey", "Sky Honey", "Honey from hives that drift on the wind.", 14, 1),
    material("flour", "Flour", "A sack of milled wheat. Sold by cooks and general stores.", 4, 0),
    material("fresh_milk", "Fresh Milk", "From Breezehollow's dairy. Sold by cooks and general stores.", 4, 0),
    material("spice_pouch", "Spice Pouch", "Salt, pepper and a secret. Sold by cooks and general stores.", 6, 0),
    // ---------------------------------------------------------------- 0.13.0: meals (food buffs, one at a time)
    food("herb_flatbread", "Herb Flatbread", "Warm bread folded around wild herbs.", 12, StatMods { hp: 40.0, ..Z }, 15.0, 2.0, 0.0, 0),
    food("glade_omelette", "Glade Omelette", "Fluffy eggs with milk and herbs.", 18, StatMods { main: ms(3, 3, 0, 0), ..Z }, 15.0, 0.0, 0.0, 0),
    food("hearty_stew", "Hearty Stew", "Slow-cooked game with herbs and spice.", 30, StatMods { hp: 80.0, pdef: 6.0, ..Z }, 20.0, 3.0, 0.0, 1),
    food("glowcap_skewers", "Glowcap Skewers", "Grilled mushrooms that tingle on the tongue.", 34, StatMods { matk: 10.0, mp: 30.0, ..Z }, 20.0, 0.0, 0.6, 1),
    food("honey_roast", "Honey-Glazed Roast", "Sticky, sweet and very filling.", 45, StatMods { atk_pct: 0.06, ..Z }, 20.0, 0.0, 0.0, 1),
    food("crab_cakes", "Crab Cakes", "Golden-fried and crunchy.", 50, StatMods { pdef: 12.0, mdef: 12.0, hp: 40.0, ..Z }, 25.0, 0.0, 0.0, 2),
    food("wayfarers_pie", "Wayfarer's Pie", "A whole meal in a crust. Keeps you going for miles.", 60, StatMods { hp: 120.0, ..Z }, 25.0, 4.0, 0.0, 2),
    food("aether_tea", "Aether Tea", "Herbal tea sweetened with Sky Honey and a pinch of Star Dust.", 70, StatMods { main: ms(0, 0, 4, 0), mp: 40.0, ..Z }, 30.0, 0.0, 1.0, 2),
    food("stormberry_tart", "Stormberry Tart", "Crackles when you bite it.", 70, StatMods { crit: 5.0, crit_dmg: 10.0, ..Z }, 30.0, 0.0, 0.0, 2),
    food("gale_feast", "Gale Feast", "A banquet seasoned with a Gale Shard. Fit for vault delvers.", 160, StatMods { atk_pct: 0.08, hp: 80.0, speed_pct: 0.03, ..Z }, 30.0, 2.0, 0.0, 4),
    // ---------------------------------------------------------------- 0.13.0: crafted weapons (Lv 16, Epic)
    weapon("stormguard_saber", "Stormguard Saber", "Forged from Storm Alloy, with a crab-shell guard.", Vanguard, 520, 16, StatMods { patk: 27.0, pdef: 9.0, hp: 30.0, ..Z }, 3),
    weapon("nightglass_kris", "Nightglass Kris", "Twin wavy blades wrapped in shadowsilk.", Duskblade, 520, 16, StatMods { patk: 24.0, crit: 6.0, crit_dmg: 12.0, ..Z }, 3),
    weapon("thunderclap_cestus", "Thunderclap Cestus", "Storm Alloy knuckles that clap like thunder.", Stormfist, 520, 16, StatMods { patk: 26.0, crit: 3.0, speed_pct: 0.03, ..Z }, 3),
    weapon("hawkwing_recurve", "Hawkwing Recurve", "Fletched with Galehawk feathers.", Windrunner, 520, 16, StatMods { patk: 26.0, crit: 4.0, main: ms(0, 3, 0, 0), ..Z }, 3),
    weapon("wisplight_rod", "Wisplight Rod", "A wisp ember burns in its tip.", Aethermancer, 520, 16, StatMods { matk: 30.0, mp: 35.0, main: ms(0, 0, 3, 0), ..Z }, 3),
    // ---------------------------------------------------------------- 0.13.0: Gold tier (Lv 22; dungeon caches or crafted with Gale Shards)
    weapon("galeheart_blade", "Galeheart Blade", "Gold tier. A storm sings inside the steel.", Vanguard, 1400, 22, StatMods { patk: 44.0, pdef: 14.0, hp: 70.0, ..Z }, 4),
    weapon("galeheart_fangs", "Galeheart Fangs", "Gold tier. They strike twice before you hear the first.", Duskblade, 1400, 22, StatMods { patk: 40.0, crit: 9.0, crit_dmg: 28.0, ..Z }, 4),
    weapon("galeheart_knuckles", "Galeheart Knuckles", "Gold tier. Every punch carries a gale.", Stormfist, 1400, 22, StatMods { patk: 42.0, crit: 5.0, speed_pct: 0.07, ..Z }, 4),
    weapon("galeheart_longbow", "Galeheart Longbow", "Gold tier. The string is a captured wind.", Windrunner, 1400, 22, StatMods { patk: 42.0, crit: 7.0, main: ms(0, 6, 0, 0), ..Z }, 4),
    weapon("galeheart_staff", "Galeheart Staff", "Gold tier. A Gale Shard crowns its head.", Aethermancer, 1400, 22, StatMods { matk: 48.0, mp: 60.0, main: ms(0, 0, 6, 0), ..Z }, 4),
    gear("galeheart_helm", "Galeheart Circlet", "Gold tier. Set with a Gale Shard.", Helmet, 1200, 22, StatMods { pdef: 16.0, mdef: 14.0, hp: 50.0, main: ms(2, 2, 2, 2), ..Z }, Look::Hat(7), 4),
    gear("galeheart_cuirass", "Galeheart Cuirass", "Gold tier. Plate that never weighs you down.", Torso, 1200, 22, StatMods { pdef: 32.0, mdef: 24.0, hp: 90.0, ..Z }, Look::Top(5, 3), 4),
    gear("galeheart_tassets", "Galeheart Hakama", "Gold tier. Pleated cloth woven with gold thread.", Legs, 1200, 22, StatMods { pdef: 20.0, mdef: 14.0, hp: 55.0, ..Z }, Look::Bottom(5, 3), 4),
    gear("galeheart_treads", "Galeheart Treads", "Gold tier. Your feet barely touch the ground.", Boots, 1200, 22, StatMods { pdef: 12.0, mdef: 8.0, speed_pct: 0.07, ..Z }, Look::Shoes(3), 4),
    gear("galeheart_mantle", "Galeheart Mantle", "Gold tier. Billows in a wind only you can feel.", Back, 1200, 22, StatMods { pdef: 6.0, mdef: 12.0, speed_pct: 0.05, main: ms(0, 3, 3, 0), ..Z }, Look::Back(2), 4),
    gear("galeheart_ring", "Galeheart Ring", "Gold tier.", Ring, 1200, 22, StatMods { main: ms(3, 3, 3, 3), crit: 3.0, ..Z }, Look::None, 4),
    gear("galeheart_earring", "Galeheart Earring", "Gold tier.", Earring, 1200, 22, StatMods { main: ms(0, 2, 2, 4), crit_dmg: 15.0, mp: 30.0, ..Z }, Look::None, 4),
    gear("galeheart_amulet", "Galeheart Amulet", "Gold tier.", Necklace, 1200, 22, StatMods { main: ms(3, 3, 3, 3), atk_pct: 0.06, crit: 3.0, ..Z }, Look::None, 4),
    // ---------------------------------------------------------------- 0.13.0: Red tier (Lv 24; Tempest Vault caches only)
    weapon("crimson_tempest_blade", "Crimson Tempest Blade", "Red tier. Forged in the Stormwarden's own lightning.", Vanguard, 3000, 24, StatMods { patk: 58.0, pdef: 20.0, hp: 110.0, atk_pct: 0.05, ..Z }, 5),
    weapon("crimson_tempest_fangs", "Crimson Tempest Fangs", "Red tier. Red lightning trails every cut.", Duskblade, 3000, 24, StatMods { patk: 52.0, crit: 12.0, crit_dmg: 40.0, ..Z }, 5),
    weapon("crimson_tempest_knuckles", "Crimson Tempest Knuckles", "Red tier. The storm answers every blow.", Stormfist, 3000, 24, StatMods { patk: 55.0, crit: 7.0, speed_pct: 0.1, ..Z }, 5),
    weapon("crimson_tempest_bow", "Crimson Tempest Bow", "Red tier. Its arrows are bolts of red lightning.", Windrunner, 3000, 24, StatMods { patk: 55.0, crit: 10.0, main: ms(0, 9, 0, 0), ..Z }, 5),
    weapon("crimson_tempest_staff", "Crimson Tempest Staff", "Red tier. A Crimson Core pulses at its tip.", Aethermancer, 3000, 24, StatMods { matk: 62.0, mp: 90.0, main: ms(0, 0, 9, 0), ..Z }, 5),
    gear("crimson_tempest_helm", "Crimson Tempest Helm", "Red tier.", Helmet, 2600, 24, StatMods { pdef: 22.0, mdef: 20.0, hp: 80.0, main: ms(3, 3, 3, 3), ..Z }, Look::Hat(4), 5),
    gear("crimson_tempest_cuirass", "Crimson Tempest Cuirass", "Red tier. Scorched red by the Stormwarden's breath.", Torso, 2600, 24, StatMods { pdef: 44.0, mdef: 32.0, hp: 140.0, atk_pct: 0.03, ..Z }, Look::Top(5, 2), 5),
    gear("crimson_tempest_greaves", "Crimson Tempest Legguards", "Red tier.", Legs, 2600, 24, StatMods { pdef: 28.0, mdef: 20.0, hp: 90.0, ..Z }, Look::Bottom(1, 2), 5),
    gear("crimson_tempest_sabatons", "Crimson Tempest Sabatons", "Red tier.", Boots, 2600, 24, StatMods { pdef: 16.0, mdef: 12.0, speed_pct: 0.09, ..Z }, Look::Shoes(3), 5),
    gear("crimson_tempest_cloak", "Crimson Tempest Cloak", "Red tier. It crackles in the dark.", Back, 2600, 24, StatMods { pdef: 10.0, mdef: 16.0, speed_pct: 0.06, main: ms(0, 4, 4, 0), ..Z }, Look::Back(2), 5),
    gear("crimson_tempest_ring", "Crimson Tempest Ring", "Red tier.", Ring, 2600, 24, StatMods { main: ms(4, 4, 4, 4), crit: 5.0, ..Z }, Look::None, 5),
    gear("crimson_tempest_earring", "Crimson Tempest Earring", "Red tier.", Earring, 2600, 24, StatMods { main: ms(0, 3, 3, 5), crit_dmg: 22.0, mp: 40.0, ..Z }, Look::None, 5),
    gear("crimson_tempest_amulet", "Crimson Tempest Amulet", "Red tier.", Necklace, 2600, 24, StatMods { main: ms(4, 4, 4, 4), atk_pct: 0.08, crit: 4.0, ..Z }, Look::None, 5),
    // ---------------------------------------------------------------- 0.13.0: crafted accessories (Lv 16)
    gear("stormglass_pendant", "Stormglass Pendant", "Storm Alloy set around a sphere of star dust.", Necklace, 420, 16, StatMods { main: ms(1, 1, 1, 1), mp: 20.0, crit: 2.0, ..Z }, Look::None, 3),
    gear("crabshell_band", "Crabshell Band", "A ring carved from a Quarry Crab's shell.", Ring, 420, 16, StatMods { main: ms(2, 0, 0, 0), pdef: 6.0, hp: 40.0, ..Z }, Look::None, 3),
    // ---------------------------------------------------------------- 0.13.0: housing and dungeons
    special("house_deed", "Breezehollow House Deed", "Use it to become the owner of a cosy house in Breezehollow. Enter it from the door by the estate office.", 6000, Special::HouseDeed, 3),
    special("vault_cache", "Tempest Vault Cache", "Your share of the Stormwarden's hoard. Use it to open: Gale Shards and a chance of Gold or Red gear.", 0, Special::Cache, 4),
    furn("furn_bed", "Feather Bed", "Furniture. Sleep in it for a full heal and the Well Rested bonus (+10% EXP from monsters for 30 minutes).", 220, Furniture::Bed, 2),
    furn("furn_chest", "Storage Chest", "Furniture. Opens your house storage (48 extra slots).", 260, Furniture::Chest, 2),
    furn("furn_stove", "Cooking Stove", "Furniture. Cook meals at home.", 300, Furniture::Stove, 2),
    furn("furn_workbench", "Workbench", "Furniture. Craft weapons and armour at home.", 300, Furniture::Workbench, 2),
    furn("furn_table", "Oak Table", "Furniture.", 90, Furniture::Table, 1),
    furn("furn_bookshelf", "Bookshelf", "Furniture. Full of adventure stories.", 140, Furniture::Bookshelf, 1),
    furn("furn_plant", "Potted Fern", "Furniture.", 40, Furniture::Plant, 0),
    furn("furn_rug", "Woven Rug", "Furniture.", 60, Furniture::Rug, 0),
    furn("furn_fireplace", "Stone Fireplace", "Furniture. Crackles cosily.", 280, Furniture::Fireplace, 2),
    furn("furn_weaponrack", "Weapon Rack", "Furniture. Show off your arsenal.", 150, Furniture::WeaponRack, 1),
    furn("furn_armorstand", "Armour Stand", "Furniture.", 150, Furniture::ArmorStand, 1),
    furn("furn_barrel", "Barrel", "Furniture.", 30, Furniture::Barrel, 0),
    furn("furn_banner", "Gale Banner", "Furniture. Flaps in a breeze that isn't there.", 80, Furniture::Banner, 1),
    furn("furn_lamp", "Standing Lamp", "Furniture.", 50, Furniture::Lamp, 0),
    furn("furn_trophy", "Stormwarden Trophy", "Furniture. Proof you beat the Stormwarden. Rarely found in Tempest Vault caches.", 500, Furniture::Trophy, 5),
];

/// What a weapon looks like in the character's hand.
pub fn weapon_art(id: &str) -> Option<crate::classes::Weapon> {
    use crate::classes::Weapon;
    Some(match id {
        "gnarled_stick" => Weapon::Stick,
        "wood_sword" | "novice_sword" | "ironwood_blade" => Weapon::WoodSword,
        "wood_club" | "oak_club" | "ironbound_cudgel" => Weapon::Club,
        other => item(other)?.class?.info().weapon,
    })
}

pub fn item(id: &str) -> Option<&'static ItemDef> {
    ITEMS.iter().find(|i| i.id == id)
}

/// The free weapon you're handed when you join a class.
pub fn starter_weapon(class: ClassId) -> &'static str {
    match class {
        Novice => "gnarled_stick",
        Vanguard => "iron_sword",
        Duskblade => "dusk_daggers",
        Stormfist => "storm_wraps",
        Windrunner => "ash_bow",
        Aethermancer => "apprentice_staff",
    }
}

// -------------------------------------------------------------------- inventory

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stack {
    pub id: String,
    pub count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Inventory {
    /// Up to INVENTORY_SLOTS stacks, in display order.
    pub slots: Vec<Stack>,
    /// Equipped item ids, indexed by EquipSlot::index().
    pub equip: [Option<String>; EQUIP_COUNT],
}

impl Inventory {
    pub fn count(&self, id: &str) -> u32 {
        self.slots.iter().filter(|s| s.id == id).map(|s| s.count).sum()
    }
    /// Can `n` of this item be added?
    pub fn room_for(&self, id: &str, n: u32) -> bool {
        let Some(def) = item(id) else { return false };
        let free = INVENTORY_SLOTS.saturating_sub(self.slots.len()) as u32;
        if def.stackable() {
            let partial: u32 = self.slots.iter().filter(|s| s.id == id).map(|s| STACK_MAX - s.count.min(STACK_MAX)).sum();
            partial + free * STACK_MAX >= n
        } else {
            free >= n
        }
    }
    /// Add items; returns how many didn't fit.
    pub fn add(&mut self, id: &str, mut n: u32) -> u32 {
        let Some(def) = item(id) else { return n };
        if def.stackable() {
            for s in self.slots.iter_mut().filter(|s| s.id == id) {
                let take = (STACK_MAX - s.count.min(STACK_MAX)).min(n);
                s.count += take;
                n -= take;
            }
            while n > 0 && self.slots.len() < INVENTORY_SLOTS {
                let take = n.min(STACK_MAX);
                self.slots.push(Stack { id: id.to_string(), count: take });
                n -= take;
            }
        } else {
            while n > 0 && self.slots.len() < INVENTORY_SLOTS {
                self.slots.push(Stack { id: id.to_string(), count: 1 });
                n -= 1;
            }
        }
        n
    }
    /// Remove up to `n` of an item (from the last stacks first); returns how many were removed.
    pub fn remove(&mut self, id: &str, n: u32) -> u32 {
        let mut left = n;
        let mut i = self.slots.len();
        while i > 0 && left > 0 {
            i -= 1;
            if self.slots[i].id == id {
                let take = self.slots[i].count.min(left);
                self.slots[i].count -= take;
                left -= take;
                if self.slots[i].count == 0 {
                    self.slots.remove(i);
                }
            }
        }
        n - left
    }
    pub fn take_slot(&mut self, idx: usize, n: u32) -> Option<(String, u32)> {
        let s = self.slots.get_mut(idx)?;
        let take = s.count.min(n);
        s.count -= take;
        let id = s.id.clone();
        if s.count == 0 {
            self.slots.remove(idx);
        }
        Some((id, take))
    }
    /// Where a piece of gear of this kind would go: the first empty slot it
    /// fits, else the first one (which it would replace).
    pub fn slot_for(&self, kind: GearKind) -> EquipSlot {
        let slots = kind.slots();
        slots.iter().copied().find(|s| self.equip[s.index()].is_none()).unwrap_or(slots[0])
    }
    pub fn equipped(&self, slot: EquipSlot) -> Option<&'static ItemDef> {
        self.equip[slot.index()].as_deref().and_then(item)
    }
    pub fn gear_mods(&self) -> StatMods {
        let mut m = StatMods::default();
        for e in self.equip.iter().flatten() {
            if let Some(d) = item(e) {
                m = m + d.mods;
            }
        }
        m
    }
}

// -------------------------------------------------------------------- shops

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShopKind {
    General,
    Weapons,
    Armor,
    Pets,
    Housing,
    Pantry,
}

impl ShopKind {
    pub fn name(self) -> &'static str {
        match self {
            ShopKind::General => "General Goods",
            ShopKind::Weapons => "Weapons",
            ShopKind::Armor => "Armor",
            ShopKind::Pets => "Pets & pet food",
            ShopKind::Housing => "Houses & furniture",
            ShopKind::Pantry => "Cooking ingredients",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ShopDef {
    pub id: &'static str,
    pub name: &'static str,
    pub kind: ShopKind,
    pub stock: &'static [&'static str],
}

const GENERAL_STOCK: &[&str] = &["hp_potion", "hp_potion_big", "mp_potion", "mp_potion_big", "elixir", "might_tonic", "iron_tonic", "lucky_clover", "revive_scroll", "flour", "fresh_milk", "spice_pouch"];
/// Breezehollow's estate office: the house deed and furniture.
pub const ESTATE_SHOP: &str = "breezehollow_estate";
/// The cook's pantry in Breezehollow.
pub const PANTRY_SHOP: &str = "breezehollow_pantry";
/// Every main town's pet shop.
pub const PET_SHOP: &str = "pet_shop";
/// The tutorial outfitter by the training dummies.
pub const GLADE_SHOP: &str = "glade_outfitter";
/// Bound potions that are 40% off for a character's first purchases.
pub const TRAINEE_POTIONS: &[&str] = &["trainee_hp_potion", "trainee_mp_potion"];
const ARMOR_STOCK: &[&str] = &[
    "leather_cap", "iron_helm", "sage_hat", "ranger_hood", "gale_circlet",
    "cloth_tunic", "leather_vest", "mage_robe", "chain_shirt", "shadow_garb", "sage_robe", "plate_armor", "monk_gi",
    "leather_breeches", "chain_leggings", "shadow_leggings", "sage_hakama", "gale_greaves",
    "sandals", "leather_boots", "windstep_boots", "iron_greaves",
    "sage_pendant", "storm_amulet", "copper_ring", "power_band", "sage_ring", "gale_ring",
    "hunter_charm", "rogue_coin", "star_drop", "travel_cloak", "gale_cape", "feather_wings",
];

pub const SHOPS: &[ShopDef] = &[
    ShopDef {
        id: GLADE_SHOP,
        name: "Quartermaster Hesper's Wagon",
        kind: ShopKind::General,
        stock: &[
            "trainee_hp_potion", "trainee_mp_potion", "revive_scroll", "ironwood_blade", "ironbound_cudgel",
            "trail_hat", "trail_gambeson", "trail_cargos", "trail_boots", "fang_necklace", "silver_ring", "brass_hoops", "trail_pack",
        ],
    },
    ShopDef { id: PET_SHOP, name: "Whisker & Wing Pet Shop", kind: ShopKind::Pets, stock: &["pet_breeze_fox", "pet_puffling", "pet_sparkwing", "pet_food"] },
    ShopDef {
        id: ESTATE_SHOP,
        name: "Hollowhome Estates",
        kind: ShopKind::Housing,
        stock: &[
            "house_deed", "furn_bed", "furn_chest", "furn_stove", "furn_workbench", "furn_table", "furn_bookshelf", "furn_plant", "furn_rug", "furn_fireplace",
            "furn_weaponrack", "furn_armorstand", "furn_barrel", "furn_banner", "furn_lamp",
        ],
    },
    ShopDef { id: PANTRY_SHOP, name: "Bramwell's Pantry", kind: ShopKind::Pantry, stock: &["flour", "fresh_milk", "spice_pouch", "wild_herbs", "herb_flatbread", "hearty_stew"] },
    ShopDef { id: "breezehollow_general", name: "Mira's Remedies", kind: ShopKind::General, stock: &["hp_potion", "mp_potion", "might_tonic", "iron_tonic", "revive_scroll"] },
    ShopDef { id: "breezehollow_smith", name: "Garrick's Forge", kind: ShopKind::Weapons, stock: &["wood_sword", "wood_club", "oak_club", "leather_cap", "cloth_tunic", "leather_vest", "leather_breeches", "sandals", "leather_boots", "copper_ring", "travel_cloak"] },
    ShopDef { id: "ironhold_general", name: "The Iron Kettle", kind: ShopKind::General, stock: GENERAL_STOCK },
    ShopDef { id: "ironhold_weapons", name: "Ironhold Armoury", kind: ShopKind::Weapons, stock: &["steel_sword", "gale_sword", "oak_club"] },
    ShopDef { id: "ironhold_armor", name: "Plate & Rivet", kind: ShopKind::Armor, stock: ARMOR_STOCK },
    ShopDef { id: "shadowmere_general", name: "The Lantern Apothecary", kind: ShopKind::General, stock: GENERAL_STOCK },
    ShopDef { id: "shadowmere_weapons", name: "Whisperblade", kind: ShopKind::Weapons, stock: &["viper_daggers", "eclipse_daggers", "oak_club"] },
    ShopDef { id: "shadowmere_armor", name: "Silk & Shade", kind: ShopKind::Armor, stock: ARMOR_STOCK },
    ShopDef { id: "tempest_general", name: "Monastery Pantry", kind: ShopKind::General, stock: GENERAL_STOCK },
    ShopDef { id: "tempest_weapons", name: "Hall of Fists", kind: ShopKind::Weapons, stock: &["thunder_knuckles", "tempest_gauntlets", "oak_club"] },
    ShopDef { id: "tempest_armor", name: "Robes of the Storm", kind: ShopKind::Armor, stock: ARMOR_STOCK },
    ShopDef { id: "leafwhisper_general", name: "Canopy Provisions", kind: ShopKind::General, stock: GENERAL_STOCK },
    ShopDef { id: "leafwhisper_weapons", name: "Fletcher's Nook", kind: ShopKind::Weapons, stock: &["yew_bow", "galewing_bow", "oak_club"] },
    ShopDef { id: "leafwhisper_armor", name: "Barkweave Outfitters", kind: ShopKind::Armor, stock: ARMOR_STOCK },
    ShopDef { id: "starfall_general", name: "Aether Alchemy", kind: ShopKind::General, stock: GENERAL_STOCK },
    ShopDef { id: "starfall_weapons", name: "The Staff Gallery", kind: ShopKind::Weapons, stock: &["runed_staff", "starfall_staff", "oak_club"] },
    ShopDef { id: "starfall_armor", name: "Glyphwoven Garments", kind: ShopKind::Armor, stock: ARMOR_STOCK },
];

/// Price of buying `n` of an item, given how many discounted trainee
/// purchases the character has left. Returns (gold, discounted units used).
pub fn purchase_cost(id: &str, n: u32, discounts_left: u32) -> Option<(u32, u32)> {
    let def = item(id)?;
    if TRAINEE_POTIONS.contains(&id) {
        let k = n.min(discounts_left);
        Some((k * discounted_price(def) + (n - k) * def.buy_price(), k))
    } else {
        Some((n.saturating_mul(def.buy_price()), 0))
    }
}

/// A trainee potion's price while the 40% discount lasts.
pub fn discounted_price(def: &ItemDef) -> u32 {
    ((def.buy_price() as f32) * (1.0 - TRAINEE_DISCOUNT)).round() as u32
}

pub fn shop(id: &str) -> Option<&'static ShopDef> {
    SHOPS.iter().find(|s| s.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogue_is_consistent() {
        for (i, a) in ITEMS.iter().enumerate() {
            for b in &ITEMS[i + 1..] {
                assert_ne!(a.id, b.id, "duplicate item id");
            }
        }
        for s in SHOPS {
            for id in s.stock {
                let d = item(id).unwrap_or_else(|| panic!("shop {} sells unknown item {}", s.id, id));
                assert!(d.price > 0, "{} must have a price", id);
            }
        }
        for &c in crate::classes::EVERY_CLASS {
            assert_eq!(item(starter_weapon(c)).unwrap().class, Some(c));
        }
        // The tutorial outfitter sells bound potions and Lv 8 Novice gear.
        let glade = shop(GLADE_SHOP).unwrap();
        for id in TRAINEE_POTIONS {
            assert!(glade.stock.contains(id));
            let d = item(id).unwrap();
            assert!(d.bound && !d.can_sell() && !d.can_trade() && !d.can_drop());
        }
        let gear: Vec<_> = glade.stock.iter().filter_map(|id| item(id)).filter(|d| d.slot().is_some()).collect();
        for k in [GearKind::Weapon, GearKind::Helmet, GearKind::Torso, GearKind::Legs, GearKind::Boots, GearKind::Necklace, GearKind::Ring, GearKind::Earring, GearKind::Back] {
            assert!(gear.iter().any(|d| d.slot() == Some(k)), "the outfitter sells no {:?}", k);
        }
        for d in gear {
            assert_eq!(d.level, 8, "{}", d.id);
            assert!(d.class.is_none() || d.class == Some(ClassId::Novice), "{} must be usable by Novices", d.id);
        }
    }

    #[test]
    fn trainee_discount() {
        let red = item("trainee_hp_potion").unwrap();
        // 15g base, x9 = 135g, 40% off = 81g
        assert_eq!(red.buy_price(), 135);
        assert_eq!(discounted_price(red), 81);
        assert_eq!(purchase_cost("trainee_hp_potion", 3, 10), Some((243, 3)));
        // 2 discounted + 3 full price
        assert_eq!(purchase_cost("trainee_hp_potion", 5, 2), Some((2 * 81 + 3 * 135, 2)));
        assert_eq!(purchase_cost("hp_potion", 5, 10), Some((675, 0)));
        // selling back still pays 40% of the base price
        assert_eq!(item("hp_potion").unwrap().sell_price(), 6);
    }

    #[test]
    fn inventory_stacks_and_limits() {
        let mut inv = Inventory::default();
        assert_eq!(inv.add("hp_potion", 150), 0);
        assert_eq!(inv.slots.len(), 2);
        assert_eq!(inv.count("hp_potion"), 150);
        assert_eq!(inv.remove("hp_potion", 60), 60);
        assert_eq!(inv.count("hp_potion"), 90);
        for _ in 0..30 {
            inv.add("copper_ring", 1);
        }
        assert_eq!(inv.slots.len(), INVENTORY_SLOTS);
        assert!(!inv.room_for("sandals", 1));
        assert!(inv.room_for("hp_potion", 9));
        assert_eq!(inv.add("unknown", 1), 1);
    }
}
