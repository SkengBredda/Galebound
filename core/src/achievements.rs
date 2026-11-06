//! Achievements, the Gale Marks they pay out, the exclusive cosmetics those
//! marks buy, and the daily attendance calendar.

use crate::save::CharacterSave;

/// Running totals kept on each character, for achievements and profiles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tally {
    pub kills: u32,
    pub bosses: u32,
    pub quests: u32,
    pub deaths: u32,
    pub revives: u32,
    pub looted: u32,
    pub trades: u32,
    pub parties: u32,
    pub friends: u32,
    pub guild: bool,
    pub class_changed: bool,
    pub most_gold: u32,
    pub best_pet: u32,
    pub waystones: u32,
    /// Items crafted.
    pub crafts: u32,
    /// Houses bought.
    pub houses: u32,
    /// 0.15.0 co-op dungeons cleared.
    pub dungeon_clears: u32,
}

impl Tally {
    pub const KEYS: [&'static str; 17] = [
        "kills", "bosses", "quests", "deaths", "revives", "looted", "trades", "parties", "friends", "guild", "class", "gold", "pet", "waystones", "crafts",
        "houses", "dungeons",
    ];
    fn slot(&mut self, k: &str) -> Option<&mut u32> {
        Some(match k {
            "kills" => &mut self.kills,
            "bosses" => &mut self.bosses,
            "quests" => &mut self.quests,
            "deaths" => &mut self.deaths,
            "revives" => &mut self.revives,
            "looted" => &mut self.looted,
            "trades" => &mut self.trades,
            "parties" => &mut self.parties,
            "friends" => &mut self.friends,
            "gold" => &mut self.most_gold,
            "pet" => &mut self.best_pet,
            "waystones" => &mut self.waystones,
            "crafts" => &mut self.crafts,
            "houses" => &mut self.houses,
            "dungeons" => &mut self.dungeon_clears,
            _ => return None,
        })
    }
    /// "kills=12,bosses=1,..." (only non-zero values).
    pub fn encode(&self) -> String {
        let mut c = *self;
        let mut v = Vec::new();
        for k in Self::KEYS {
            let n = match k {
                "guild" => self.guild as u32,
                "class" => self.class_changed as u32,
                _ => *c.slot(k).unwrap(),
            };
            if n > 0 {
                v.push(format!("{}={}", k, n));
            }
        }
        v.join(",")
    }
    pub fn decode(s: &str) -> Tally {
        let mut t = Tally::default();
        for p in s.split(',') {
            if let Some((k, n)) = p.split_once('=') {
                let n: u32 = n.trim().parse().unwrap_or(0);
                match k.trim() {
                    "guild" => t.guild = n > 0,
                    "class" => t.class_changed = n > 0,
                    k => {
                        if let Some(x) = t.slot(k) {
                            *x = n;
                        }
                    }
                }
            }
        }
        t
    }
}

pub struct AchievementDef {
    pub id: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    /// Score points; you also earn this many Gale Marks.
    pub points: u32,
    pub check: fn(&CharacterSave) -> bool,
}

/// How far along an achievement is (for the progress bar): (have, need).
pub fn progress(a: &AchievementDef, s: &CharacterSave) -> Option<(u32, u32)> {
    let t = &s.tally;
    let gear = s.inventory.equip.iter().filter(|e| e.is_some()).count() as u32;
    Some(match a.id {
        "first_blood" => (t.kills, 1),
        "hunter_100" => (t.kills, 100),
        "hunter_1000" => (t.kills, 1000),
        "hunter_5000" => (t.kills, 5000),
        "giant_slayer" => (t.bosses, 1),
        "boss_breaker" => (t.bosses, 10),
        "level_10" => (s.level, 10),
        "level_15" => (s.level, 15),
        "level_20" => (s.level, 20),
        "level_30" => (s.level, 30),
        "quests_5" => (t.quests, 5),
        "quests_20" => (t.quests, 20),
        "pocket_change" => (t.most_gold, 10_000),
        "treasure_hoard" => (t.most_gold, 100_000),
        "pet_lover" => (t.best_pet, 10),
        "guardian_angel" => (t.revives, 1),
        "lifesaver" => (t.revives, 10),
        "friendly_face" => (t.friends, 1),
        "popular" => (t.friends, 10),
        "fair_deal" => (t.trades, 1),
        "merchant" => (t.trades, 20),
        "regular" => (s.attendance.count, 7),
        "devoted" => (s.attendance.count, 28),
        "fully_kitted" => (gear, crate::items::EQUIP_COUNT as u32),
        "back_for_more" => (t.deaths, 10),
        "hoarder" => (t.looted, 100),
        "magpie" => (t.looted, 1000),
        "wanderer" => ((s.play_seconds / 3600) as u32, 10),
        "wayfarer" => (t.waystones, 5),
        _ => return None,
    })
}

macro_rules! ach {
    ($id:expr, $name:expr, $desc:expr, $pts:expr, |$s:ident| $e:expr) => {
        AchievementDef { id: $id, name: $name, desc: $desc, points: $pts, check: |$s: &CharacterSave| $e }
    };
}

pub const ACHIEVEMENTS: &[AchievementDef] = &[
    ach!("first_blood", "First Steps", "Defeat your first monster.", 5, |s| s.tally.kills >= 1),
    ach!("hunter_100", "Monster Hunter", "Defeat 100 monsters.", 10, |s| s.tally.kills >= 100),
    ach!("hunter_1000", "Monster Slayer", "Defeat 1,000 monsters.", 25, |s| s.tally.kills >= 1000),
    ach!("hunter_5000", "Legend of the Wilds", "Defeat 5,000 monsters.", 50, |s| s.tally.kills >= 5000),
    ach!("giant_slayer", "Giant Slayer", "Defeat a quest boss.", 10, |s| s.tally.bosses >= 1),
    ach!("boss_breaker", "Boss Breaker", "Defeat 10 quest bosses.", 25, |s| s.tally.bosses >= 10),
    ach!("level_10", "Coming of Age", "Reach level 10.", 10, |s| s.level >= 10),
    ach!("level_15", "Seasoned", "Reach level 15.", 15, |s| s.level >= 15),
    ach!("level_20", "Veteran", "Reach level 20.", 25, |s| s.level >= 20),
    ach!("level_30", "Hero of the Gale", "Reach level 30.", 50, |s| s.level >= 30),
    ach!("path_chosen", "A Path Chosen", "Change class.", 10, |s| s.tally.class_changed),
    ach!("level_40", "Highlander", "Reach level 40.", 50, |s| s.level >= 40),
    ach!("level_50", "Paragon", "Reach level 50.", 75, |s| s.level >= 50),
    ach!("level_60", "Legend of Galebound", "Reach level 60, the level cap.", 100, |s| s.level >= 60),
    ach!("advanced", "The Second Path", "Advance into an advanced class.", 25, |s| s.job.is_some()),
    ach!("frozen_throne", "Throne of Ice", "Defeat Glacia in the Frozen Throne.", 40, |s| s.has_flag("cleared_frozen_throne")),
    ach!("cinder_sanctum", "Dragonslayer", "Defeat Ignaroth in the Cinder Sanctum.", 50, |s| s.has_flag("cleared_cinder_sanctum")),
    ach!("celestial_spire", "Eclipse Breaker", "Defeat the Aether Twins in the Celestial Spire.", 75, |s| s.has_flag("cleared_celestial_spire")),
    ach!("first_craft", "Handmade", "Craft your first item.", 5, |s| s.tally.crafts >= 1),
    ach!("crafts_250", "Artisan", "Craft 250 items.", 25, |s| s.tally.crafts >= 250),
    ach!("master_smith", "Master Smith", "Reach Weaponsmithing or Armoursmithing level 20.", 25, |s| s.crafts[0].level >= 20 || s.crafts[1].level >= 20),
    ach!("master_chef", "Master Chef", "Reach Cooking level 20.", 25, |s| s.crafts[2].level >= 20),
    ach!("homeowner", "Home Sweet Home", "Buy a house in Breezehollow.", 15, |s| s.tally.houses >= 1),
    ach!("vault_clear", "Vault Breaker", "Defeat the Stormwarden in the Tempest Vault.", 25, |s| s.vault_clears >= 1),
    ach!("vault_10", "Storm Chaser", "Clear the Tempest Vault 10 times.", 50, |s| s.vault_clears >= 10),
    ach!("quests_5", "Helping Hand", "Complete 5 quests.", 5, |s| s.tally.quests >= 5),
    ach!("quests_20", "Hero for Hire", "Complete 20 quests.", 20, |s| s.tally.quests >= 20),
    ach!("pocket_change", "Pocket Change", "Hold 10,000 gold at once.", 10, |s| s.tally.most_gold >= 10_000),
    ach!("treasure_hoard", "Dragon's Hoard", "Hold 100,000 gold at once.", 30, |s| s.tally.most_gold >= 100_000),
    ach!("best_friend", "Best Friend", "Adopt a pet.", 10, |s| !s.pets.is_empty()),
    ach!("pet_lover", "Pet Whisperer", "Raise a pet to level 10.", 20, |s| s.tally.best_pet >= 10),
    ach!("guardian_angel", "Guardian Angel", "Revive another player.", 10, |s| s.tally.revives >= 1),
    ach!("lifesaver", "Lifesaver", "Revive other players 10 times.", 25, |s| s.tally.revives >= 10),
    ach!("better_together", "Better Together", "Join a party.", 5, |s| s.tally.parties >= 1),
    ach!("sworn_banner", "Sworn to the Banner", "Join or found a guild.", 10, |s| s.tally.guild),
    ach!("friendly_face", "Friendly Face", "Make a friend.", 5, |s| s.tally.friends >= 1),
    ach!("popular", "Popular", "Have 10 friends.", 15, |s| s.tally.friends >= 10),
    ach!("fair_deal", "Fair Deal", "Complete a trade.", 5, |s| s.tally.trades >= 1),
    ach!("merchant", "Merchant Prince", "Complete 20 trades.", 20, |s| s.tally.trades >= 20),
    ach!("regular", "Regular", "Claim 7 daily attendance rewards.", 10, |s| s.attendance.count >= 7),
    ach!("devoted", "Devoted", "Claim 28 daily attendance rewards.", 30, |s| s.attendance.count >= 28),
    ach!("fully_kitted", "Fully Kitted", "Fill all 11 equipment slots.", 15, |s| s.inventory.equip.iter().all(|e| e.is_some())),
    ach!("back_for_more", "Back for More", "Get back up after falling 10 times.", 5, |s| s.tally.deaths >= 10),
    ach!("hoarder", "Hoarder", "Pick up 100 items.", 5, |s| s.tally.looted >= 100),
    ach!("magpie", "Magpie", "Pick up 1,000 items.", 15, |s| s.tally.looted >= 1000),
    ach!("wanderer", "Wanderer", "Play for 10 hours.", 15, |s| s.play_seconds >= 36_000),
    ach!("wayfarer", "Wayfarer", "Attune to 5 waystones.", 10, |s| s.tally.waystones >= 5),
];

pub fn achievement(id: &str) -> Option<&'static AchievementDef> {
    ACHIEVEMENTS.iter().find(|a| a.id == id)
}

/// Achievement score: the points of everything unlocked.
pub fn score(s: &CharacterSave) -> u32 {
    s.achievements.iter().filter_map(|id| achievement(id)).map(|a| a.points).sum()
}

/// Anything newly earned (not yet in `s.achievements`).
pub fn newly_earned(s: &CharacterSave) -> Vec<&'static AchievementDef> {
    ACHIEVEMENTS.iter().filter(|a| !s.achievements.iter().any(|x| x == a.id) && (a.check)(s)).collect()
}

// ------------------------------------------------------------------ cosmetics

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CosmeticKind {
    Hat(u8),
    Back(u8),
}

pub struct CosmeticDef {
    pub id: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    pub kind: CosmeticKind,
    /// Price in Gale Marks.
    pub price: u32,
}

/// Exclusive cosmetics: only sold for Gale Marks, shown over your gear.
pub const COSMETICS: &[CosmeticDef] = &[
    CosmeticDef { id: "cos_flower_crown", name: "Flower Crown", desc: "Fresh every morning. Nobody knows how.", kind: CosmeticKind::Hat(9), price: 30 },
    CosmeticDef { id: "cos_cat_ears", name: "Cat Ears", desc: "They twitch when you're surprised.", kind: CosmeticKind::Hat(8), price: 40 },
    CosmeticDef { id: "cos_halo", name: "Starry Halo", desc: "A ring of tiny stars that follows you around.", kind: CosmeticKind::Hat(10), price: 80 },
    CosmeticDef { id: "cos_crown", name: "Crown of Gales", desc: "Worn by those the wind itself respects.", kind: CosmeticKind::Hat(11), price: 120 },
    CosmeticDef { id: "cos_angel_wings", name: "Angel Wings", desc: "Soft white wings that flutter as you move.", kind: CosmeticKind::Back(5), price: 100 },
    CosmeticDef { id: "cos_bat_wings", name: "Bat Wings", desc: "For the dramatic adventurer.", kind: CosmeticKind::Back(6), price: 100 },
    CosmeticDef { id: "cos_star_mantle", name: "Star Mantle", desc: "A cape stitched from the night sky.", kind: CosmeticKind::Back(7), price: 150 },
];

pub fn cosmetic(id: &str) -> Option<&'static CosmeticDef> {
    COSMETICS.iter().find(|c| c.id == id)
}

// ------------------------------------------------------------------ attendance

/// Daily login rewards (a 7-day cycle that repeats): gold, items, marks.
pub struct DayReward {
    pub gold: u32,
    pub items: &'static [(&'static str, u32)],
    pub marks: u32,
}

pub const ATTENDANCE: [DayReward; 7] = [
    DayReward { gold: 300, items: &[("hp_potion", 3)], marks: 0 },
    DayReward { gold: 0, items: &[("hp_potion_big", 3), ("mp_potion_big", 2)], marks: 0 },
    DayReward { gold: 500, items: &[], marks: 2 },
    DayReward { gold: 0, items: &[("revive_scroll", 1), ("pet_food", 3)], marks: 0 },
    DayReward { gold: 0, items: &[("might_tonic", 2), ("iron_tonic", 2)], marks: 0 },
    DayReward { gold: 800, items: &[("elixir", 1)], marks: 0 },
    DayReward { gold: 1000, items: &[("elixir", 2), ("lucky_clover", 1)], marks: 10 },
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Attendance {
    /// Server day of the last claim (days since 1970).
    pub last_day: u32,
    /// Rewards claimed in total.
    pub count: u32,
}

impl Attendance {
    pub fn can_claim(&self, today: u32) -> bool {
        today > 0 && today > self.last_day
    }
    /// Which reward (0..7) the next claim gives.
    pub fn next_index(&self) -> usize {
        self.count as usize % ATTENDANCE.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::appearance::{Appearance, BACKS, HATS};

    #[test]
    fn tally_roundtrip_and_catalogue() {
        let t = Tally { kills: 12, bosses: 1, guild: true, most_gold: 500, ..Default::default() };
        assert_eq!(Tally::decode(&t.encode()), t);
        for (i, a) in ACHIEVEMENTS.iter().enumerate() {
            assert!(ACHIEVEMENTS[i + 1..].iter().all(|b| b.id != a.id), "duplicate {}", a.id);
            assert!(a.points > 0);
        }
        for c in COSMETICS {
            match c.kind {
                CosmeticKind::Hat(h) => assert!((h as usize) < HATS.len()),
                CosmeticKind::Back(b) => assert!((b as usize) < BACKS.len()),
            }
        }
        for d in &ATTENDANCE {
            for (id, _) in d.items {
                assert!(crate::items::item(id).is_some(), "{}", id);
            }
        }
    }

    #[test]
    fn achievements_unlock_from_the_tally() {
        let mut s = CharacterSave::new("Ach", Appearance::default());
        assert!(newly_earned(&s).is_empty());
        s.tally.kills = 100;
        let got: Vec<&str> = newly_earned(&s).iter().map(|a| a.id).collect();
        assert!(got.contains(&"first_blood") && got.contains(&"hunter_100") && !got.contains(&"hunter_1000"));
        s.achievements = got.iter().map(|x| x.to_string()).collect();
        assert_eq!(score(&s), 15);
        assert!(newly_earned(&s).is_empty());
        assert_eq!(progress(achievement("hunter_1000").unwrap(), &s), Some((100, 1000)));
    }
}
