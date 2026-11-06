//! Life skills and home: crafting, meals, the player's house and the
//! Tempest Vault co-op dungeon (0.13.0). A child module of `sim`, so it
//! extends `Sim` directly.

use super::*;
use crate::crafting::{self, Craft, Station};
use crate::housing::{self, HOUSE_MAP};
use crate::items::Furniture;
#[allow(unused_imports)]
use crate::world::{VAULT_GATE_FLAG, VAULT_LEVEL, VAULT_MAP};
#[allow(unused_imports)]
use crate::math::Rng;

/// Buff id used for the Well Rested bonus.
pub const RESTED_BUFF: &str = "well_rested";

/// HP and Mana per second from the meal you've eaten, if any.
pub fn food_regen(buffs: &[Buff]) -> (f32, f32) {
    for b in buffs {
        if let Some(d) = items::item(b.id) {
            if let ItemKind::Consumable(Effect::Food { hp_regen, mp_regen, .. }) = d.kind {
                return (hp_regen, mp_regen);
            }
        }
    }
    (0.0, 0.0)
}

/// The meal buff (id, seconds left), for saving.
pub fn food_left(buffs: &[Buff]) -> Option<(String, f32)> {
    buffs.iter().find(|b| is_food(b.id)).map(|b| (b.id.to_string(), b.left))
}

fn is_food(id: &str) -> bool {
    matches!(items::item(id).map(|d| d.kind), Some(ItemKind::Consumable(Effect::Food { .. })))
}

/// Dungeon bosses (no quest to turn in).
pub fn is_vault_boss(k: EnemyKind) -> bool {
    world::is_dungeon_boss(k)
}

/// What a crafting attempt made.
#[derive(Clone, Debug, PartialEq)]
pub struct Crafted {
    pub item: &'static str,
    pub count: u32,
    pub exp: u32,
    /// Crafting levels gained.
    pub levels: u32,
}

impl Sim {
    /// Set-up after loading a character.
    pub(super) fn on_load(&mut self) {
        // A meal eaten before logging out is still in your belly.
        if let Some((id, left)) = self.save.food.clone() {
            if let Some(def) = items::item(&id) {
                self.eat(def, Some(left));
            }
        }
        if self.save.rested > 0.0 {
            self.push_rested(self.save.rested);
        }
        // Dungeon runs don't survive logging out: back to where you came in.
        if let Some(d) = world::dungeon(&self.save.map) {
            if self.map.key == d.map {
                self.change_map(d.home, d.home_x - 100.0);
            }
        }
        // An old save that already has a class shouldn't be stuck on "Choose Your Path".
        quests::sync_class_quests(&mut self.save);
        self.furnish();
    }

    /// The map key the server knows us by: the map plus any private instance.
    pub fn net_map(&self) -> String {
        format!("{}{}", self.map.key, self.instance)
    }

    /// Houses are private to their owner; a vault run belongs to a party.
    pub(super) fn set_instance(&mut self) {
        if let Some(i) = self.forced_instance.take() {
            self.instance = i;
            return;
        }
        if self.map.key == HOUSE_MAP {
            self.instance = format!("#{}", self.save.name);
        } else if world::is_dungeon(self.map.key) {
            if self.instance.is_empty() {
                self.instance = format!("#{}", self.save.name);
            }
        } else {
            self.instance.clear();
        }
    }

    /// EXP multiplier for monster kills (Well Rested).
    pub fn kill_exp_mult(&self) -> f32 {
        if self.save.rested > 0.0 {
            1.0 + housing::WELL_RESTED_EXP
        } else {
            1.0
        }
    }

    /// A monster you helped kill died (offline or online).
    pub(super) fn on_killed(&mut self, k: EnemyKind) {
        self.passive_kill();
        if let Some(d) = world::highlands::gate_of(k) {
            if let Some((_, flag)) = d.gate {
                self.save.set_flag(flag);
            }
            let next = d.finals.iter().map(|b| b.name()).collect::<Vec<_>>().join(" and ");
            self.toast(&format!("{} falls!", k.name()), &format!("The inner gate grinds open. {} waits beyond.", next));
            self.shake = 10.0;
        }
        if let Some(d) = world::highlands::final_of(k) {
            // the Twins: both must fall (the other may still be eclipsing)
            if enemies::twin_alive(k, &self.enemies) {
                self.toast(&format!("{} falls!", k.name()), "Its twin begins an eclipse - finish it within 10 seconds!");
                return;
            }
            if d.map == VAULT_MAP {
                self.save.vault_clears += 1;
            } else {
                self.save.set_flag(&format!("cleared_{}", d.map));
                self.save.tally.dungeon_clears += 1;
            }
            let cache = d.cache;
            let left = self.save.inventory.add(cache, 1);
            if left > 0 {
                let c = self.player.body.center();
                self.spawn_drop(DropKind::Item(cache), left, c, 40.0);
            }
            let cname = items::item(cache).map(|i| i.name).unwrap_or("cache");
            self.toast(&format!("{} cleared!", d.name), &format!("A {} is yours. Open it from your bag.", cname));
            self.sys_line(format!("You received a {}.", cname));
            self.shake = 12.0;
            self.wants_save = true;
        }
    }

    // ------------------------------------------------------------------ meals

    /// Eat a meal (replacing any other); `left` resumes a saved meal.
    pub(super) fn eat(&mut self, def: &'static items::ItemDef, left: Option<f32>) {
        let ItemKind::Consumable(Effect::Food { mods, secs, .. }) = def.kind else { return };
        let p = &mut self.player;
        p.buffs.retain(|b| !is_food(b.id));
        let l = left.unwrap_or(secs).min(secs);
        p.buffs.push(Buff { id: def.id, name: def.name, mods, left: l, total: secs, fx: skills::SkillFx::Nature });
        self.save.food = Some((def.id.to_string(), l));
        self.recompute_stats();
    }

    /// The meal you've eaten and its seconds left.
    pub fn current_meal(&self) -> Option<(&'static items::ItemDef, f32)> {
        self.player.buffs.iter().find(|b| is_food(b.id)).and_then(|b| items::item(b.id).map(|d| (d, b.left)))
    }

    fn push_rested(&mut self, secs: f32) {
        let p = &mut self.player;
        p.buffs.retain(|b| b.id != RESTED_BUFF);
        p.buffs.push(Buff {
            id: RESTED_BUFF,
            name: "Well Rested",
            mods: StatMods::default(),
            left: secs,
            total: housing::WELL_RESTED_SECS,
            fx: skills::SkillFx::Holy,
        });
    }

    // ------------------------------------------------------------------ crafting

    /// Can this character use this station right now (talking to it, or at home)?
    pub fn station_crafts(&self) -> &'static [Craft] {
        self.craft_open.map(|s| s.crafts()).unwrap_or(&[])
    }

    /// Why this recipe can't be crafted `n` times here, if it can't.
    pub fn craft_problem(&self, output: &str, n: u32) -> Option<String> {
        let r = crafting::recipe(output)?;
        if !self.station_crafts().contains(&r.craft) {
            return Some(format!("You need a {} station", r.craft.name()));
        }
        let lv = self.save.craft_level(r.craft);
        if lv < r.level {
            return Some(format!("Needs {} Lv.{}", r.craft.name(), r.level));
        }
        let max = crafting::max_crafts(r, |id| self.save.inventory.count(id), self.save.gold);
        if max < n.max(1) {
            if r.inputs.iter().any(|(id, k)| self.save.inventory.count(id) < k * n.max(1)) {
                return Some("Not enough materials".into());
            }
            return Some(format!("Not enough gold ({}g each)", r.fee));
        }
        None
    }

    /// Craft a recipe `n` times at the open station.
    pub fn craft(&mut self, output: &str, n: u32) -> Result<Crafted, String> {
        let n = n.max(1);
        let r = crafting::recipe(output).ok_or("Unknown recipe")?;
        if let Some(p) = self.craft_problem(output, n) {
            return Err(p);
        }
        // Room for the results, counting the slots the materials free up.
        let mut bag = self.save.inventory.clone();
        for (id, k) in r.inputs {
            bag.remove(id, k * n);
        }
        if !bag.room_for(r.output, r.count * n) {
            return Err("Your bag is full".into());
        }
        bag.add(r.output, r.count * n);
        self.save.inventory = bag;
        self.save.gold -= r.fee * n;
        let skill = &mut self.save.crafts[r.craft.index()];
        let mut exp = 0;
        let mut levels = 0;
        for _ in 0..n {
            let e = crafting::exp_for(r, skill.level);
            exp += e;
            levels += skill.gain(e);
        }
        self.save.tally.crafts += n;
        let def = items::item(r.output).unwrap();
        let c = self.player.body.center();
        self.burst(c, FxKind::LevelUp, 12, 160.0);
        self.float(V2::new(c.x, c.y - 60.0), format!("Crafted {} x{}", def.name, r.count * n), FloatKind::Heal);
        self.sys_line(format!("Crafted {}x {} (+{} {} EXP)", r.count * n, def.name, exp, r.craft.name()));
        if levels > 0 {
            let lv = self.save.crafts[r.craft.index()].level;
            self.toast(&format!("{} Lv.{}!", r.craft.name(), lv), "New recipes may be ready in the crafting window.");
        }
        self.wants_save = true;
        Ok(Crafted { item: r.output, count: r.count * n, exp, levels })
    }

    // ------------------------------------------------------------------ housing

    pub(super) fn use_house_deed(&mut self) -> Result<(), String> {
        if self.save.has_flag(world::HOUSE_FLAG) {
            return Err("You already own a house".into());
        }
        self.save.inventory.remove("house_deed", 1);
        self.save.set_flag(world::HOUSE_FLAG);
        self.save.tally.houses += 1;
        self.toast("You own a house!", "Your door is by Rosalind's estate office in Breezehollow. Press Up at the door to go in.");
        self.wants_save = true;
        Ok(())
    }

    /// Rebuild the house's furniture decorations from the save.
    pub(super) fn furnish(&mut self) {
        if self.map.key != HOUSE_MAP {
            return;
        }
        // the house's own walls and windows are layer 0; furniture is layer 1
        self.map.decos.retain(|d| d.layer == 0);
        for (spot, id) in self.save.house.clone() {
            let (Some(f), Some(&x)) = (housing::furniture_of(&id), housing::SPOTS.get(spot)) else { continue };
            self.map.decos.push(world::Deco { kind: f.deco(), x, y: world::GROUND, scale: 1.0, layer: 1 });
        }
    }

    /// The furniture spot you're standing at, with what's in it.
    pub fn nearby_spot(&self) -> Option<(usize, Option<Furniture>)> {
        if self.map.key != HOUSE_MAP {
            return None;
        }
        let x = self.player.body.feet().x;
        let (i, &sx) = housing::SPOTS.iter().enumerate().min_by(|a, b| (a.1 - x).abs().total_cmp(&(b.1 - x).abs()))?;
        if (sx - x).abs() > 70.0 {
            return None;
        }
        Some((i, housing::placed(&self.save.house, i).and_then(housing::furniture_of)))
    }

    /// Talk next to furniture: sleep, open storage, cook or craft.
    pub(super) fn use_furniture(&mut self) -> bool {
        let Some((_, Some(f))) = self.nearby_spot() else { return false };
        match f {
            Furniture::Bed => self.sleep(),
            Furniture::Chest => {
                self.storage_open = true;
                self.wants_save = true;
            }
            Furniture::Stove => self.craft_open = Some(Station::Kitchen),
            Furniture::Workbench => self.craft_open = Some(Station::Forge),
            _ => return false,
        }
        true
    }

    /// Sleep in your bed: full heal and Well Rested.
    pub fn sleep(&mut self) {
        self.heal_full();
        self.save.rested = housing::WELL_RESTED_SECS;
        self.push_rested(housing::WELL_RESTED_SECS);
        self.toast("Well Rested", "+10% EXP from monsters for 30 minutes.");
        self.wants_save = true;
    }

    /// Put a piece of furniture from your bag into a spot.
    pub fn place_furniture(&mut self, spot: usize, id: &str) -> Result<(), String> {
        if !self.save.has_flag(world::HOUSE_FLAG) {
            return Err("You don't own a house yet".into());
        }
        if spot >= housing::SPOTS.len() {
            return Err("No such spot".into());
        }
        housing::furniture_of(id).ok_or("That isn't furniture")?;
        if self.save.inventory.count(id) == 0 {
            return Err("It's not in your bag".into());
        }
        if housing::placed(&self.save.house, spot).is_some() {
            self.take_furniture(spot)?;
        }
        self.save.inventory.remove(id, 1);
        self.save.house.push((spot, id.to_string()));
        self.furnish();
        self.wants_save = true;
        Ok(())
    }

    /// Put the furniture in a spot back in your bag.
    pub fn take_furniture(&mut self, spot: usize) -> Result<(), String> {
        let id = housing::placed(&self.save.house, spot).ok_or("Nothing there")?.to_string();
        if !self.save.inventory.room_for(&id, 1) {
            return Err("Your bag is full".into());
        }
        self.save.house.retain(|h| h.0 != spot);
        self.save.inventory.add(&id, 1);
        self.furnish();
        self.wants_save = true;
        Ok(())
    }

    /// Move a bag slot (all of it, or `n`) into house storage.
    pub fn store(&mut self, idx: usize, n: u32) -> Result<(), String> {
        let st = self.save.inventory.slots.get(idx).ok_or("No item")?.clone();
        let n = n.min(st.count).max(1);
        let left = housing::storage_add(&mut self.save.storage, &st.id, n);
        let moved = n - left;
        if moved == 0 {
            return Err("Storage is full".into());
        }
        self.save.inventory.take_slot(idx, moved);
        self.wants_save = true;
        Ok(())
    }

    /// Move a storage slot (all of it, or `n`) into your bag.
    pub fn withdraw(&mut self, idx: usize, n: u32) -> Result<(), String> {
        let st = self.save.storage.get(idx).ok_or("No item")?.clone();
        let n = n.min(st.count).max(1);
        let left = self.save.inventory.add(&st.id, n);
        let moved = n - left;
        if moved == 0 {
            return Err("Your bag is full".into());
        }
        let s = &mut self.save.storage[idx];
        s.count -= moved;
        if s.count == 0 {
            self.save.storage.remove(idx);
        }
        self.wants_save = true;
        Ok(())
    }

    // ------------------------------------------------------------------ the Tempest Vault

    /// Go into the vault: your party leader's copy if you're in a party.
    pub fn enter_vault(&mut self) -> Result<(), String> {
        self.enter_dungeon(VAULT_MAP)
    }

    /// Go into a co-op dungeon: your party leader's copy if you're in a party.
    pub fn enter_dungeon(&mut self, map: &str) -> Result<(), String> {
        let d = world::dungeon(map).ok_or_else(|| "No such dungeon".to_string())?;
        if self.save.level < d.level {
            return Err(format!("You must be level {} to enter.", d.level));
        }
        let owner = self.party.as_ref().map(|p| p.leader.clone()).filter(|l| !l.is_empty()).unwrap_or_else(|| self.save.name.clone());
        self.instance = format!("#{}", owner);
        // every run starts with the inner gate shut
        if let Some((_, flag)) = d.gate {
            self.save.flags.retain(|f| f != flag);
        }
        self.forced_instance = Some(self.instance.clone());
        self.change_map(d.map, 200.0);
        if self.party.is_some() {
            self.sys_line(format!("Entered {}'s {}. Party members who enter join you.", owner, d.name));
        }
        Ok(())
    }

    /// Open a dungeon cache: shards, maybe dungeon gear.
    pub(super) fn open_cache(&mut self, id: &str) -> Result<(), String> {
        if !self.save.inventory.room_for("galeheart_ring", 2) {
            return Err("Make room in your bag first (2 slots)".into());
        }
        let pick = |rng: &mut Rng, list: &'static [&'static str]| list[(rng.f() * list.len() as f32) as usize % list.len()];
        let mut got: Vec<(&'static str, u32)> = vec![];
        let roll = self.rng.f();
        match id {
            "frozen_cache" | "cinder_cache" | "celestial_cache" => {
                let (shard, loot) = match id {
                    "frozen_cache" => ("rime_shard", items::RIME_LOOT),
                    "cinder_cache" => ("ember_shard", items::CINDER_LOOT),
                    _ => ("star_shard", items::CELESTIAL_LOOT),
                };
                got.push((shard, 3 + (self.rng.f() * 4.0) as u32));
                if roll < CACHE_HIGH {
                    got.push((pick(&mut self.rng, loot), 1));
                }
            }
            _ => {
                got.push(("gale_shard", 3 + (self.rng.f() * 4.0) as u32));
                if roll < CACHE_RED {
                    got.push((pick(&mut self.rng, items::RED_LOOT), 1));
                } else if roll < CACHE_RED + CACHE_GOLD {
                    got.push((pick(&mut self.rng, items::GOLD_LOOT), 1));
                }
            }
        }
        self.save.inventory.remove(id, 1);
        if self.rng.f() < CACHE_TROPHY {
            got.push(("furn_trophy", 1));
        }
        let mut names = vec![];
        let c = self.player.body.center();
        for (id, n) in got {
            let left = self.save.inventory.add(id, n);
            if left > 0 {
                self.spawn_drop(DropKind::Item(id), left, c, 60.0);
            }
            let d = items::item(id).unwrap();
            names.push(format!("{}x {}", n, d.name));
            if d.tier >= 4 && d.slot().is_some() {
                self.toast(&format!("{} item!", d.tier_name()), d.name);
            }
        }
        self.burst(c, FxKind::LevelUp, 30, 260.0);
        self.sys_line(format!("{}: {}", items::item(id).map(|i| i.name).unwrap_or("Cache"), names.join(", ")));
        self.wants_save = true;
        Ok(())
    }

    // ------------------------------------------------------------------ quest log

    /// Show or hide a quest in the on-screen tracker.
    pub fn set_tracked(&mut self, quest: &str, on: bool) {
        self.save.untracked.retain(|q| q != quest);
        if !on {
            self.save.untracked.push(quest.to_string());
        }
        self.wants_save = true;
    }
}

/// Chances when opening a Vault Cache.
pub const CACHE_RED: f32 = 0.06;
pub const CACHE_GOLD: f32 = 0.45;
pub const CACHE_TROPHY: f32 = 0.03;
/// Chance a 0.15.0 dungeon cache holds a piece of that dungeon's gear.
pub const CACHE_HIGH: f32 = 0.35;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::appearance::Appearance;
    use crate::classes::ClassId;

    fn sim() -> Sim {
        let mut save = CharacterSave::new("Crafter", Appearance::default());
        save.map = "breezehollow".into();
        let mut s = Sim::new(save, Settings::default(), 7);
        s.tick(&InputState::default());
        s
    }

    #[test]
    fn crafting_uses_materials_and_levels_up() {
        let mut s = sim();
        s.save.gold = 1000;
        s.save.inventory.add("sturdy_branch", 40);
        assert!(s.craft("seasoned_plank", 1).is_err(), "no station open");
        s.craft_open = Some(Station::Kitchen);
        assert!(s.craft("seasoned_plank", 1).unwrap_err().contains("station"));
        s.craft_open = Some(Station::Forge);
        let r = s.craft("seasoned_plank", 5).unwrap();
        assert_eq!(r.count, 5);
        assert_eq!(s.save.inventory.count("sturdy_branch"), 20);
        assert_eq!(s.save.inventory.count("seasoned_plank"), 5);
        assert_eq!(s.save.gold, 1000 - 15);
        assert!(s.save.crafts[Craft::Weaponsmithing.index()].exp > 0);
        // level-gated recipe
        assert!(s.craft("oak_club", 1).unwrap_err().contains("Lv.5"));
        // grind levels and make a weapon
        s.save.crafts[Craft::Weaponsmithing.index()].level = 5;
        s.save.inventory.add("beak_shard", 4);
        s.craft("oak_club", 1).unwrap();
        assert_eq!(s.save.inventory.count("oak_club"), 1);
        assert!(s.craft("oak_club", 1).unwrap_err().contains("materials"));
        assert!(s.save.tally.crafts >= 6);
    }

    #[test]
    fn meals_buff_you_replace_each_other_and_survive_a_relog() {
        let mut s = sim();
        s.player.hp = 10.0;
        let hp0 = s.player.max_hp;
        s.save.inventory.add("hearty_stew", 1);
        s.save.inventory.add("honey_roast", 1);
        assert!(s.use_item("hearty_stew"));
        assert!(s.player.max_hp >= hp0 + 79.0);
        assert_eq!(s.current_meal().unwrap().0.id, "hearty_stew");
        assert!(s.use_item("honey_roast"));
        assert_eq!(s.player.buffs.iter().filter(|b| is_food(b.id)).count(), 1, "one meal at a time");
        assert!((s.player.max_hp - hp0).abs() < 1.0);
        let save = s.snapshot();
        assert_eq!(save.food.as_ref().unwrap().0, "honey_roast");
        let text = crate::save::encode_character(&save);
        let back = crate::save::decode_character(&text).unwrap();
        let s2 = Sim::new(back, Settings::default(), 1);
        assert_eq!(s2.current_meal().unwrap().0.id, "honey_roast");
    }

    #[test]
    fn mana_regen_slows_in_combat() {
        let mut s = sim();
        s.player.mp = 0.0;
        s.player.since_combat = 99.0;
        for _ in 0..120 {
            s.tick(&InputState::default());
        }
        let calm = s.player.mp;
        s.player.mp = 0.0;
        for _ in 0..120 {
            s.player.since_combat = 0.0;
            s.tick(&InputState::default());
        }
        let fighting = s.player.mp;
        assert!(calm > 0.5);
        assert!(fighting < calm * 0.4, "in combat {} vs out {}", fighting, calm);
    }

    #[test]
    fn house_deed_furniture_storage_and_bed() {
        let mut s = sim();
        s.save.inventory.add("house_deed", 1);
        s.save.inventory.add("furn_bed", 1);
        s.save.inventory.add("furn_chest", 1);
        s.save.inventory.add("hp_potion", 20);
        assert!(s.place_furniture(0, "furn_bed").is_err(), "no house yet");
        assert!(s.use_item("house_deed"));
        assert!(s.save.has_flag(world::HOUSE_FLAG));
        s.change_map(HOUSE_MAP, 160.0);
        assert_eq!(s.map.key, HOUSE_MAP);
        s.place_furniture(0, "furn_bed").unwrap();
        s.place_furniture(1, "furn_chest").unwrap();
        assert_eq!(s.save.inventory.count("furn_bed"), 0);
        assert_eq!(s.map.decos.iter().filter(|d| d.layer == 1).count(), 2);
        // sleep in the bed
        s.player.body.pos.x = housing::SPOTS[0] - PLAYER_W * 0.5;
        assert!(s.use_furniture());
        assert!(s.save.rested > 0.0);
        assert!(s.kill_exp_mult() > 1.0);
        // open the chest and store potions
        s.player.body.pos.x = housing::SPOTS[1] - PLAYER_W * 0.5;
        assert!(s.use_furniture());
        assert!(s.storage_open);
        let idx = s.save.inventory.slots.iter().position(|x| x.id == "hp_potion").unwrap();
        s.store(idx, 999).unwrap();
        assert_eq!(s.save.inventory.count("hp_potion"), 0);
        assert_eq!(s.save.storage[0].count, 23, "20 + the 3 every character starts with");
        s.withdraw(0, 5).unwrap();
        assert_eq!(s.save.inventory.count("hp_potion"), 5);
        // take the bed back
        s.take_furniture(0).unwrap();
        assert_eq!(s.save.inventory.count("furn_bed"), 1);
        // everything survives a save
        let back = crate::save::decode_character(&crate::save::encode_character(&s.snapshot())).unwrap();
        assert_eq!(back.house, vec![(1, "furn_chest".to_string())]);
        assert_eq!(back.storage[0].count, 18);
        assert!(back.rested > 0.0);
        // online, the house is private
        s.online = true;
        s.set_instance();
        assert_eq!(s.net_map(), "player_house#Crafter");
    }

    #[test]
    fn vault_gate_bosses_and_caches() {
        let mut s = sim();
        s.save.level = 22;
        s.save.class = ClassId::Vanguard;
        s.enter_vault().unwrap();
        assert_eq!(s.map.key, VAULT_MAP);
        assert_eq!(s.net_map(), "tempest_vault#Crafter");
        assert!(s.enemies.iter().any(|e| e.kind == EnemyKind::Stormwarden));
        // the inner gate is shut until the Gatekeeper falls
        assert!(s.solids_with_gates().len() > s.map.solids.len());
        s.on_killed(EnemyKind::Gatekeeper);
        assert_eq!(s.solids_with_gates().len(), s.map.solids.len());
        s.on_killed(EnemyKind::Stormwarden);
        assert_eq!(s.save.inventory.count("vault_cache"), 1);
        assert_eq!(s.save.vault_clears, 1);
        // caches always hold shards; over many opens both Gold and Red turn up
        let (mut gold, mut red) = (0, 0);
        for _ in 0..200 {
            s.save.inventory = Default::default();
            s.save.inventory.add("vault_cache", 1);
            assert!(s.use_item("vault_cache"));
            assert!(s.save.inventory.count("gale_shard") >= 3);
            gold += items::GOLD_LOOT.iter().filter(|id| s.save.inventory.count(id) > 0).count();
            red += items::RED_LOOT.iter().filter(|id| s.save.inventory.count(id) > 0).count();
        }
        assert!(gold > 40 && red > 0, "gold {} red {}", gold, red);
        // a party shares one vault: the leader's
        s.change_map("wayfarers_crossroads", 4200.0);
        assert!(s.instance.is_empty());
        s.party = Some(crate::social::PartyNet { leader: "Leader".into(), members: vec![] });
        s.enter_vault().unwrap();
        assert_eq!(s.net_map(), "tempest_vault#Leader");
        assert!(!s.save.has_flag(VAULT_GATE_FLAG), "each run starts with the gate shut");
        // logging out inside the vault puts you back at the crossroads
        let save = s.snapshot();
        let s2 = Sim::new(save, Settings::default(), 3);
        assert_eq!(s2.map.key, "wayfarers_crossroads");
    }

    #[test]
    fn low_level_characters_cant_enter_the_vault() {
        let mut s = sim();
        assert!(s.enter_vault().is_err());
        assert_eq!(s.map.key, "breezehollow");
    }

    #[test]
    fn station_shopkeepers_offer_crafting() {
        let mut s = sim();
        let mut d = quests::talk("garrick", "Garrick", &mut s.save);
        assert_eq!(d.action, DialogAction::Shop("breezehollow_smith"));
        d.lines.truncate(1);
        let ad = ActiveDialog { dialog: d, line: 0, choice: 0, message: None };
        assert_eq!(ad.choices().unwrap()[1], "Craft");
        let d = quests::talk("inn_ironhold", "Bess", &mut s.save);
        assert_eq!(d.action, DialogAction::Craft(Station::Kitchen));
    }
}

#[cfg(test)]
mod ladders {
    use super::*;
    use crate::input::Action;
    use crate::classes::ClassId;
    use crate::appearance::Appearance;

    fn run(s: &mut Sim, secs: f32, i: InputState) -> f32 {
        let mut maxjump: f32 = 0.0;
        let n = (secs * 60.0) as i32;
        for k in 0..n {
            let x0 = s.player.body.pos.x;
            let inp = if k == 0 { i.clone() } else { let mut j = InputState::default(); for a in [Action::Left, Action::Right, Action::Up, Action::Jump] { if i.held(a) { j.set(a, true, false); } } j };
            s.tick(&inp);
            maxjump = maxjump.max((s.player.body.pos.x - x0).abs());
        }
        maxjump
    }

    /// Jumping left or right off any ladder, at any height, never moves you
    /// more than a normal step in one frame (0.14.0: it used to throw you
    /// through the ledge next to the ladder).
    #[test]
    fn jumping_off_ladders_never_teleports() {
        let mut bad = vec![];
        for w in world::all_map_keys() {
            let Some(m) = world::load_map(w) else { continue };
            for (ci, c) in m.climbs.iter().enumerate() {
                for dir in [Action::Left, Action::Right] {
                    for frac in [0.05f32, 0.2, 0.5, 0.8, 0.95] {
                        let mut save = CharacterSave::new("P", Appearance::default());
                        save.level = 30; save.class = ClassId::Windrunner;
                        save.map = w.to_string();
                        let mut s = Sim::new(save, Settings::default(), 1);
                        s.tick(&InputState::default());
                        s.player.state = PState::Climb; s.player.climb = ci;
                        s.player.body.pos.x = c.x - PLAYER_W * 0.5;
                        s.player.body.pos.y = c.top + (c.bottom - c.top) * frac - PLAYER_H;
                        s.player.body.vel = V2::ZERO;
                        s.player.invuln = 99.0;
                        let mut i = InputState::default();
                        i.set(Action::Jump, true, true); i.set(dir, true, true);
                        let j = run(&mut s, 0.8, i);
                        if j > 30.0 {
                            bad.push(format!("{} ladder {} {:?} at {:.0}%: moved {:.0} in one frame", w, ci, dir, frac * 100.0, j));
                        }
                    }
                }
            }
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }
}
