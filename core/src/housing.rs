//! Player housing: a house in Breezehollow you buy with a House Deed,
//! furnish with furniture and use for storage, cooking, crafting and rest.
//!
//! The house has [`SPOTS`] places for furniture along the floor. Functional
//! furniture does something when you press Talk next to it:
//! - Feather Bed: a full heal and Well Rested (+10% EXP from monsters, 30 min);
//! - Storage Chest: opens your house storage ([`STORAGE_SLOTS`] slots);
//! - Cooking Stove: cook meals;
//! - Workbench: craft weapons and armour.
//! Houses are private: online, each player's house is their own instance.

use crate::items::Furniture;
use crate::world::DecoKind;

/// The house map.
pub const HOUSE_MAP: &str = "player_house";
/// Where the house door is in Breezehollow.
pub const HOUSE_DOOR_X: f32 = 760.0;
/// Extra item slots in your house storage.
pub const STORAGE_SLOTS: usize = 48;
/// Seconds the Well Rested bonus lasts.
pub const WELL_RESTED_SECS: f32 = 30.0 * 60.0;
/// Extra EXP from monsters while Well Rested.
pub const WELL_RESTED_EXP: f32 = 0.10;

/// Furniture spots: x position on the house floor.
pub const SPOTS: &[f32] = &[300.0, 470.0, 640.0, 810.0, 980.0, 1150.0, 1320.0, 1490.0];

/// Default spot names, shown in the furnishing window.
pub fn spot_name(i: usize) -> String {
    format!("Spot {}", i + 1)
}

impl Furniture {
    /// How the furniture is drawn.
    pub fn deco(self) -> DecoKind {
        match self {
            Furniture::Bed => DecoKind::Bed,
            Furniture::Chest => DecoKind::Chest,
            Furniture::Stove => DecoKind::Stove,
            Furniture::Workbench => DecoKind::Workbench,
            Furniture::Table => DecoKind::Table,
            Furniture::Bookshelf => DecoKind::Bookshelf,
            Furniture::Plant => DecoKind::Plant,
            Furniture::Rug => DecoKind::Rug,
            Furniture::Fireplace => DecoKind::Fireplace,
            Furniture::WeaponRack => DecoKind::WeaponRack,
            Furniture::ArmorStand => DecoKind::ArmorStand,
            Furniture::Barrel => DecoKind::Barrel,
            Furniture::Banner => DecoKind::Banner,
            Furniture::Lamp => DecoKind::Lamp,
            Furniture::Trophy => DecoKind::Trophy,
        }
    }
    /// What pressing Talk next to it does, for the on-screen hint.
    pub fn use_hint(self) -> Option<&'static str> {
        match self {
            Furniture::Bed => Some("Sleep"),
            Furniture::Chest => Some("Open storage"),
            Furniture::Stove => Some("Cook"),
            Furniture::Workbench => Some("Craft"),
            _ => None,
        }
    }
}

/// The furniture item in a spot, from the save's list.
pub fn placed(house: &[(usize, String)], spot: usize) -> Option<&str> {
    house.iter().find(|h| h.0 == spot).map(|h| h.1.as_str())
}

/// The kind of furniture an item is.
pub fn furniture_of(id: &str) -> Option<Furniture> {
    match crate::items::item(id)?.kind {
        crate::items::ItemKind::Furniture(f) => Some(f),
        _ => None,
    }
}

/// Put items into storage; returns how many didn't fit.
pub fn storage_add(storage: &mut Vec<crate::items::Stack>, id: &str, mut n: u32) -> u32 {
    use crate::items::{item, Stack, STACK_MAX};
    let Some(def) = item(id) else { return n };
    if def.stackable() {
        for s in storage.iter_mut().filter(|s| s.id == id) {
            let take = (STACK_MAX - s.count.min(STACK_MAX)).min(n);
            s.count += take;
            n -= take;
        }
        while n > 0 && storage.len() < STORAGE_SLOTS {
            let take = n.min(STACK_MAX);
            storage.push(Stack { id: id.to_string(), count: take });
            n -= take;
        }
    } else {
        while n > 0 && storage.len() < STORAGE_SLOTS {
            storage.push(Stack { id: id.to_string(), count: 1 });
            n -= 1;
        }
    }
    n
}
