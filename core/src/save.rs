//! Save data model and a tiny human-readable text format.
//!
//! Format: one `key=value` per line, `[character]` starts a character block,
//! `#` lines are comments. Values escape `\` and newlines. This keeps the
//! project free of serialization crates and the files easy to inspect.

use crate::appearance::Appearance;
use crate::classes::ClassId;
use crate::items::{self, EquipSlot, Inventory};
use crate::skills::{HotbarSlot, HOTBAR_SLOTS};
use crate::stats::{self, MainStats};
use crate::input::Layout;
use crate::pets::{PetSave, PetSpecies};
use std::path::PathBuf;

pub const MAX_CHARACTERS: usize = 3;
pub const SAVE_VERSION: u32 = 6;

/// How the game window is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayMode {
    Windowed,
    /// A borderless window covering the whole screen.
    WindowedFullscreen,
    Fullscreen,
}

pub const DISPLAY_MODES: &[DisplayMode] = &[DisplayMode::Windowed, DisplayMode::WindowedFullscreen, DisplayMode::Fullscreen];

impl DisplayMode {
    pub fn key(self) -> &'static str {
        match self {
            DisplayMode::Windowed => "windowed",
            DisplayMode::WindowedFullscreen => "borderless",
            DisplayMode::Fullscreen => "fullscreen",
        }
    }
    pub fn from_key(s: &str) -> Option<DisplayMode> {
        DISPLAY_MODES.iter().copied().find(|m| m.key() == s)
    }
    pub fn name(self) -> &'static str {
        match self {
            DisplayMode::Windowed => "Windowed",
            DisplayMode::WindowedFullscreen => "Windowed Fullscreen",
            DisplayMode::Fullscreen => "Fullscreen",
        }
    }
}

/// Minimap sizes, cycled with the Minimap key. 0 = hidden.
pub const MINIMAP_SIZES: u8 = 4;

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub layout: Layout,
    /// Brawlhalla-style option: pressing Up also jumps.
    pub tap_up_to_jump: bool,
    pub show_damage_numbers: bool,
    pub screen_shake: bool,
    pub display: DisplayMode,
    /// 0 hidden, 1 small, 2 medium, 3 large.
    pub minimap_size: u8,
    /// Show the whole map (true) or just the area around you (false).
    pub minimap_whole: bool,
    /// Who's effects to show (0.15.1), see `FX_*`: companions (turrets,
    /// hawks, orbiting stars), ability effects and weapon effects.
    pub fx_companions: u8,
    pub fx_skills: u8,
    pub fx_weapons: u8,
}

/// Effect visibility: everyone's, only your own, or none.
pub const FX_ALL: u8 = 0;
pub const FX_MINE: u8 = 1;
pub const FX_OFF: u8 = 2;

/// Which kind of effect something is (for the graphics options).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FxCat {
    /// Basic attacks: swings, shots.
    Weapon,
    /// Abilities: their projectiles, lightning, explosions, auras.
    Skill,
    /// Turrets, the spirit hawk, orbiting stars.
    Companion,
}

impl Settings {
    /// Should an effect of this kind be drawn? `mine` = your own.
    pub fn fx_shown(&self, cat: FxCat, mine: bool) -> bool {
        let v = match cat {
            FxCat::Weapon => self.fx_weapons,
            FxCat::Skill => self.fx_skills,
            FxCat::Companion => self.fx_companions,
        };
        v == FX_ALL || (v == FX_MINE && mine)
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            layout: Layout::Wasd,
            tap_up_to_jump: false,
            show_damage_numbers: true,
            screen_shake: true,
            display: DisplayMode::Windowed,
            minimap_size: 2,
            minimap_whole: false,
            fx_companions: FX_ALL,
            fx_skills: FX_ALL,
            fx_weapons: FX_ALL,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuestState {
    Active,
    ReadyToTurnIn,
    Done,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QuestEntry {
    pub id: String,
    pub state: QuestState,
    pub progress: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CharacterSave {
    pub name: String,
    pub class: ClassId,
    /// Advanced class (level 30, 0.15.0).
    pub job: Option<crate::jobs::Job>,
    pub look: Appearance,
    pub level: u32,
    pub exp: u32,
    pub gold: u32,
    /// Main stats bought with ability points.
    pub stats: MainStats,
    /// Unspent ability points.
    pub ap: u32,
    pub inventory: Inventory,
    pub hotbar: Vec<HotbarSlot>,
    pub hp: f32,
    pub mp: f32,
    pub map: String,
    pub x: f32,
    pub y: f32,
    pub quests: Vec<QuestEntry>,
    pub flags: Vec<String>,
    pub play_seconds: u64,
    /// Discounted trainee potions bought so far (the first few are 40% off).
    pub trainee_buys: u32,
    /// Set while the character lies as a tombstone: the EXP lost on death
    /// (a Revive Scroll gives it back).
    pub tomb: Option<u32>,
    /// Pets you own, and which one is out with you.
    pub pets: Vec<PetSave>,
    pub active_pet: Option<usize>,
    /// Running totals for achievements and profiles.
    pub tally: crate::achievements::Tally,
    /// Unlocked achievement ids.
    pub achievements: Vec<String>,
    /// Gale Marks: earned from achievements, spent on exclusive cosmetics.
    pub marks: u32,
    /// Owned cosmetic ids, and which hat / back cosmetic is worn (0 = none).
    pub cosmetics: Vec<String>,
    pub cos_hat: u8,
    pub cos_back: u8,
    pub attendance: crate::achievements::Attendance,
    /// Crafting level and EXP, indexed by `Craft::index()`.
    pub crafts: [crate::crafting::CraftSkill; 3],
    /// The meal you've eaten and its seconds left.
    pub food: Option<(String, f32)>,
    /// Seconds of Well Rested left (from sleeping in your house).
    pub rested: f32,
    /// Furniture placed in your house: (spot, item id).
    pub house: Vec<(usize, String)>,
    /// Your house storage chest.
    pub storage: Vec<items::Stack>,
    /// Quests hidden from the on-screen tracker.
    pub untracked: Vec<String>,
    /// Quest log option: hide completed quests.
    pub hide_done_quests: bool,
    /// Tempest Vault clears.
    pub vault_clears: u32,
}

impl CharacterSave {
    /// A brand-new character: everyone starts as a Novice.
    pub fn new(name: &str, look: Appearance) -> Self {
        let mut inventory = Inventory::default();
        inventory.add("hp_potion", 3);
        inventory.equip[EquipSlot::Weapon.index()] = Some(items::starter_weapon(ClassId::Novice).to_string());
        inventory.equip[EquipSlot::Torso.index()] = Some("novice_garb".to_string());
        let mut hotbar = vec![HotbarSlot::Empty; HOTBAR_SLOTS];
        hotbar[0] = HotbarSlot::Skill("power_strike".into());
        hotbar[HOTBAR_SLOTS - 2] = HotbarSlot::Item("hp_potion".into());
        hotbar[HOTBAR_SLOTS - 1] = HotbarSlot::Item("mp_potion".into());
        CharacterSave {
            name: name.to_string(),
            class: ClassId::Novice,
            job: None,
            look,
            level: 1,
            exp: 0,
            gold: 0,
            stats: stats::START_STATS,
            ap: 0,
            inventory,
            hotbar,
            hp: -1.0, // -1 = full on load
            mp: -1.0,
            map: crate::world::START_MAP.to_string(),
            x: -1.0,
            y: -1.0,
            quests: Vec::new(),
            flags: Vec::new(),
            play_seconds: 0,
            trainee_buys: 0,
            tomb: None,
            pets: Vec::new(),
            active_pet: None,
            tally: Default::default(),
            achievements: Vec::new(),
            marks: 0,
            cosmetics: Vec::new(),
            cos_hat: 0,
            cos_back: 0,
            attendance: Default::default(),
            crafts: Default::default(),
            food: None,
            rested: 0.0,
            house: Vec::new(),
            storage: Vec::new(),
            untracked: Vec::new(),
            hide_done_quests: false,
            vault_clears: 0,
        }
    }
    /// The level of one craft.
    pub fn craft_level(&self, c: crate::crafting::Craft) -> u32 {
        self.crafts[c.index()].level
    }
    /// Is this quest shown in the on-screen tracker?
    pub fn tracked(&self, quest: &str) -> bool {
        !self.untracked.iter().any(|q| q == quest)
    }
    pub fn has_flag(&self, f: &str) -> bool {
        self.flags.iter().any(|x| x == f)
    }
    pub fn set_flag(&mut self, f: &str) {
        if !self.has_flag(f) {
            self.flags.push(f.to_string());
        }
    }
    pub fn quest(&self, id: &str) -> Option<&QuestEntry> {
        self.quests.iter().find(|q| q.id == id)
    }
    /// Bring a save from before stats/items existed up to date: the old
    /// class is kept, its ability points are auto-assigned and it gets the
    /// class's starter weapon.
    fn migrate_pre_stats(&mut self) {
        let ap = (self.level.max(1) - 1) * stats::AP_PER_LEVEL;
        self.stats = stats::auto_assign(self.class, stats::START_STATS, ap);
        self.ap = 0;
        self.inventory.equip[EquipSlot::Weapon.index()] = Some(items::starter_weapon(self.class).to_string());
    }
    /// How the character looks in the world: their outfit with gear on
    /// top, and any exclusive cosmetics over that.
    pub fn worn_look(&self) -> Appearance {
        use crate::items::{EquipSlot, Look};
        let mut l = self.look;
        for slot in crate::items::EQUIP_SLOTS {
            let Some(d) = self.inventory.equipped(*slot) else { continue };
            match (slot, d.look) {
                (EquipSlot::Torso, Look::Top(t, dye)) => {
                    l.top = t;
                    l.top_dye = dye;
                }
                (EquipSlot::Legs, Look::Bottom(b, dye)) => {
                    l.bottom = b;
                    l.bottom_dye = dye;
                }
                (EquipSlot::Boots, Look::Shoes(s)) => l.shoes = s,
                (EquipSlot::Helmet, Look::Hat(h)) => l.hat = h,
                (EquipSlot::Back, Look::Back(b)) => l.back = b,
                _ => {}
            }
        }
        if self.cos_hat != 0 {
            l.hat = self.cos_hat;
        }
        if self.cos_back != 0 {
            l.back = self.cos_back;
        }
        l.sanitized()
    }
    pub fn quest_mut(&mut self, id: &str) -> Option<&mut QuestEntry> {
        self.quests.iter_mut().find(|q| q.id == id)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Account {
    pub email: String,
    pub salt_hex: String,
    pub hash_hex: String,
    pub iterations: u32,
    /// SHA-256 of the "remember me" token, if remembered on this machine.
    pub remember_hash: Option<String>,
    pub settings: Settings,
    pub characters: Vec<CharacterSave>,
    pub created_unix: u64,
}

// ------------------------------------------------------------ encoding

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\n', "\\n").replace('\r', "")
}
fn unesc(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next() {
                Some('n') => out.push('\n'),
                Some(o) => out.push(o),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn qstate_str(s: QuestState) -> &'static str {
    match s {
        QuestState::Active => "active",
        QuestState::ReadyToTurnIn => "ready",
        QuestState::Done => "done",
    }
}

pub fn encode_account(a: &Account) -> String {
    let mut s = String::new();
    s.push_str("# Galebound account save. Passwords are not stored, only a salted PBKDF2 hash.\n");
    let mut kv = |k: &str, v: String| {
        s.push_str(k);
        s.push('=');
        s.push_str(&esc(&v));
        s.push('\n');
    };
    kv("version", SAVE_VERSION.to_string());
    kv("email", a.email.clone());
    kv("salt", a.salt_hex.clone());
    kv("hash", a.hash_hex.clone());
    kv("iterations", a.iterations.to_string());
    kv("remember", a.remember_hash.clone().unwrap_or_default());
    kv("created", a.created_unix.to_string());
    s.push_str(&encode_settings(&a.settings));
    for c in &a.characters {
        s.push_str("[character]\n");
        s.push_str(&encode_character(c));
    }
    s
}

/// The settings lines of an account file.
pub fn encode_settings(st: &Settings) -> String {
    let mut s = String::new();
    let mut kv = |k: &str, v: String| {
        s.push_str(k);
        s.push('=');
        s.push_str(&esc(&v));
        s.push('\n');
    };
    kv("layout", st.layout.key().to_string());
    kv("tap_up_jump", st.tap_up_to_jump.to_string());
    kv("damage_numbers", st.show_damage_numbers.to_string());
    kv("fx_companions", st.fx_companions.to_string());
    kv("fx_skills", st.fx_skills.to_string());
    kv("fx_weapons", st.fx_weapons.to_string());
    kv("screen_shake", st.screen_shake.to_string());
    kv("display", st.display.key().to_string());
    kv("minimap", st.minimap_size.to_string());
    kv("minimap_whole", st.minimap_whole.to_string());
    s
}

/// Read settings written by `encode_settings`.
pub fn decode_settings(text: &str) -> Settings {
    decode_account(&format!("email=x@x.x\nhash=-\n{}", text)).map(|a| a.settings).unwrap_or_default()
}

/// One character's block of an account file (without the `[character]` line).
pub fn encode_character(c: &CharacterSave) -> String {
    let mut s = String::new();
    let mut kv = |k: &str, v: String| {
        s.push_str(k);
        s.push('=');
        s.push_str(&esc(&v));
        s.push('\n');
    };
    kv("name", c.name.clone());
    kv("class", c.class.key().to_string());
    if let Some(j) = c.job {
        kv("job", j.key().to_string());
    }
    kv("look", c.look.encode());
    kv("level", c.level.to_string());
    kv("exp", c.exp.to_string());
    kv("gold", c.gold.to_string());
    kv("stats", format!("{},{},{},{}", c.stats.str, c.stats.dex, c.stats.int, c.stats.luk));
    kv("ap", c.ap.to_string());
    for st in &c.inventory.slots {
        kv("item", format!("{},{}", st.id, st.count));
    }
    for slot in items::EQUIP_SLOTS {
        if let Some(id) = &c.inventory.equip[slot.index()] {
            kv("equip", format!("{},{}", slot.key(), id));
        }
    }
    kv("hotbar", c.hotbar.iter().map(|h| h.encode()).collect::<Vec<_>>().join(" "));
    kv("hp", c.hp.to_string());
    kv("mp", c.mp.to_string());
    kv("map", c.map.clone());
    kv("pos", format!("{},{}", c.x, c.y));
    kv("play_seconds", c.play_seconds.to_string());
    kv("trainee_buys", c.trainee_buys.to_string());
    if let Some(lost) = c.tomb {
        kv("tomb", lost.to_string());
    }
    for p in &c.pets {
        // name last: it may contain commas
        kv("pet", format!("{},{},{},{:.2},{}", p.species.key(), p.level, p.exp, p.hunger, p.name));
    }
    if let Some(i) = c.active_pet {
        kv("active_pet", i.to_string());
    }
    kv("tally", c.tally.encode());
    if !c.achievements.is_empty() {
        kv("achievements", c.achievements.join(","));
    }
    kv("marks", c.marks.to_string());
    if !c.cosmetics.is_empty() {
        kv("cosmetics", c.cosmetics.join(","));
    }
    kv("cos_worn", format!("{},{}", c.cos_hat, c.cos_back));
    kv("attendance", format!("{},{}", c.attendance.last_day, c.attendance.count));
    for cr in crate::crafting::CRAFTS {
        let k = c.crafts[cr.index()];
        kv("craft", format!("{},{},{}", cr.key(), k.level, k.exp));
    }
    if let Some((id, left)) = &c.food {
        kv("food", format!("{},{:.0}", id, left));
    }
    if c.rested > 0.0 {
        kv("rested", format!("{:.0}", c.rested));
    }
    for (spot, id) in &c.house {
        kv("house", format!("{},{}", spot, id));
    }
    for st in &c.storage {
        kv("store", format!("{},{}", st.id, st.count));
    }
    if !c.untracked.is_empty() {
        kv("untracked", c.untracked.join(","));
    }
    if c.hide_done_quests {
        kv("hide_done", "true".into());
    }
    if c.vault_clears > 0 {
        kv("vault_clears", c.vault_clears.to_string());
    }
    for q in &c.quests {
        kv("quest", format!("{},{},{}", q.id, qstate_str(q.state), q.progress));
    }
    for f in &c.flags {
        kv("flag", f.clone());
    }
    s
}

/// Read one character written by `encode_character`.
pub fn decode_character(text: &str) -> Option<CharacterSave> {
    let a = decode_account(&format!("version={}\nemail=x@x.x\nhash=-\n[character]\n{}", SAVE_VERSION, text)).ok()?;
    a.characters.into_iter().next().filter(|c| !c.name.trim().is_empty())
}

/// Character name rules (the same in the game and on the server).
pub fn validate_name(name: &str) -> Result<String, String> {
    let n = name.trim();
    let len = n.chars().count();
    if !(3..=14).contains(&len) {
        return Err("Name must be 3-14 characters.".into());
    }
    if !n.chars().all(|c| c.is_alphanumeric() || c == ' ') {
        return Err("Use letters, numbers and spaces only.".into());
    }
    if n.contains("  ") {
        return Err("No double spaces, please.".into());
    }
    Ok(n.to_string())
}

/// An account as sent to its owner: no password hash, salt or token hash.
pub fn public_account(a: &Account) -> Account {
    Account { salt_hex: "-".into(), hash_hex: "-".into(), remember_hash: None, ..a.clone() }
}

pub fn decode_account(text: &str) -> Result<Account, String> {
    let mut a = Account {
        email: String::new(),
        salt_hex: String::new(),
        hash_hex: String::new(),
        iterations: crate::crypto::PBKDF2_ITERATIONS,
        remember_hash: None,
        settings: Settings::default(),
        characters: Vec::new(),
        created_unix: 0,
    };
    let mut cur: Option<CharacterSave> = None;
    // Did the current character block have stats (v3+)?
    let mut saw_stats = false;
    let finish = |c: &mut CharacterSave, saw_stats: bool| {
        if !saw_stats {
            c.migrate_pre_stats();
        }
    };
    for (ln, raw) in text.lines().enumerate() {
        let line = raw.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "[character]" {
            if let Some(mut c) = cur.take() {
                finish(&mut c, saw_stats);
                a.characters.push(c);
            }
            let mut c = CharacterSave::new("", Appearance::default());
            c.inventory = Inventory::default();
            cur = Some(c);
            saw_stats = false;
            continue;
        }
        let (k, v) = line.split_once('=').ok_or(format!("line {}: expected key=value", ln + 1))?;
        let v = unesc(v);
        let pf = |s: &str| s.parse::<f32>().unwrap_or(0.0);
        let pu = |s: &str| s.parse::<u32>().unwrap_or(0);
        let pb = |s: &str| s == "true";
        match &mut cur {
            None => match k {
                "email" => a.email = v,
                "salt" => a.salt_hex = v,
                "hash" => a.hash_hex = v,
                "iterations" => a.iterations = pu(&v).max(1),
                "remember" => a.remember_hash = if v.is_empty() { None } else { Some(v) },
                "created" => a.created_unix = v.parse().unwrap_or(0),
                "layout" => a.settings.layout = Layout::from_key(&v).unwrap_or(Layout::Wasd),
                "tap_up_jump" => a.settings.tap_up_to_jump = pb(&v),
                "damage_numbers" => a.settings.show_damage_numbers = pb(&v),
                "fx_companions" => a.settings.fx_companions = v.parse::<u8>().unwrap_or(FX_ALL).min(FX_OFF),
                "fx_skills" => a.settings.fx_skills = v.parse::<u8>().unwrap_or(FX_ALL).min(FX_OFF),
                "fx_weapons" => a.settings.fx_weapons = v.parse::<u8>().unwrap_or(FX_ALL).min(FX_OFF),
                "screen_shake" => a.settings.screen_shake = pb(&v),
                "display" => a.settings.display = DisplayMode::from_key(&v).unwrap_or(DisplayMode::Windowed),
                "minimap" => a.settings.minimap_size = (pu(&v) as u8).min(MINIMAP_SIZES - 1),
                "minimap_whole" => a.settings.minimap_whole = pb(&v),
                _ => {}
            },
            Some(c) => match k {
                "name" => c.name = v,
                "class" => c.class = ClassId::from_key(&v).unwrap_or(ClassId::Novice),
                "job" => c.job = crate::jobs::Job::from_key(&v),
                "look" => {
                    if let Some(l) = Appearance::decode(&v) {
                        c.look = l;
                    }
                }
                "level" => c.level = pu(&v).clamp(1, 200),
                "exp" => c.exp = pu(&v),
                "gold" => c.gold = pu(&v),
                "potions" => {
                    // v2 saves counted potions separately.
                    c.inventory.add("hp_potion", pu(&v));
                }
                "stats" => {
                    let p: Vec<i32> = v.split(',').map(|x| x.trim().parse().unwrap_or(0)).collect();
                    if p.len() == 4 {
                        c.stats = MainStats { str: p[0], dex: p[1], int: p[2], luk: p[3] };
                        saw_stats = true;
                    }
                }
                "ap" => c.ap = pu(&v),
                "item" => {
                    if let Some((id, n)) = v.split_once(',') {
                        c.inventory.add(id, pu(n));
                    }
                }
                "equip" => {
                    if let Some((slot, id)) = v.split_once(',') {
                        if let Some(def) = items::item(id) {
                            // v4 and older had armor / accessory slots: put
                            // the item wherever its kind of gear goes now.
                            let slot = EquipSlot::from_key(slot).filter(|s| def.slot() == Some(s.kind()));
                            let slot = match (slot, def.slot()) {
                                (Some(s), _) => Some(s),
                                (None, Some(k)) => Some(c.inventory.slot_for(k)),
                                _ => None,
                            };
                            if let Some(s) = slot {
                                if c.inventory.equip[s.index()].is_none() {
                                    c.inventory.equip[s.index()] = Some(id.to_string());
                                } else {
                                    c.inventory.add(id, 1);
                                }
                            }
                        }
                    }
                }
                "hotbar" => {
                    let mut h: Vec<HotbarSlot> = v.split_whitespace().map(HotbarSlot::decode).collect();
                    h.resize(HOTBAR_SLOTS, HotbarSlot::Empty);
                    // 0.14.0: Starfall became Meteor
                    for s in h.iter_mut() {
                        if *s == HotbarSlot::Skill("starfall".into()) {
                            *s = HotbarSlot::Skill("meteor".into());
                        }
                    }
                    c.hotbar = h;
                }
                "tally" => c.tally = crate::achievements::Tally::decode(&v),
                "achievements" => c.achievements = v.split(',').filter(|x| crate::achievements::achievement(x).is_some()).map(String::from).collect(),
                "marks" => c.marks = pu(&v),
                "cosmetics" => c.cosmetics = v.split(',').filter(|x| crate::achievements::cosmetic(x).is_some()).map(String::from).collect(),
                "cos_worn" => {
                    if let Some((h, b)) = v.split_once(',') {
                        c.cos_hat = pu(h) as u8;
                        c.cos_back = pu(b) as u8;
                    }
                }
                "attendance" => {
                    if let Some((d, n)) = v.split_once(',') {
                        c.attendance = crate::achievements::Attendance { last_day: pu(d), count: pu(n) };
                    }
                }
                "hp" => c.hp = pf(&v),
                "mp" => c.mp = pf(&v),
                "map" => c.map = v,
                "pos" => {
                    if let Some((x, y)) = v.split_once(',') {
                        c.x = pf(x);
                        c.y = pf(y);
                    }
                }
                "play_seconds" => c.play_seconds = v.parse().unwrap_or(0),
                "trainee_buys" => c.trainee_buys = pu(&v),
                "tomb" => c.tomb = Some(pu(&v)),
                "pet" => {
                    let p: Vec<&str> = v.splitn(5, ',').collect();
                    if p.len() == 5 {
                        if let Some(species) = PetSpecies::from_key(p[0]) {
                            c.pets.push(PetSave {
                                species,
                                level: pu(p[1]).clamp(1, crate::pets::MAX_LEVEL),
                                exp: pu(p[2]),
                                hunger: pf(p[3]).clamp(0.0, crate::pets::HUNGER_MAX),
                                name: p[4].to_string(),
                            });
                        }
                    }
                }
                "active_pet" => c.active_pet = Some(pu(&v) as usize),
                "quest" => {
                    let p: Vec<&str> = v.split(',').collect();
                    if p.len() == 3 {
                        let state = match p[1] {
                            "ready" => QuestState::ReadyToTurnIn,
                            "done" => QuestState::Done,
                            _ => QuestState::Active,
                        };
                        c.quests.push(QuestEntry { id: p[0].to_string(), state, progress: pu(p[2]) });
                    }
                }
                "flag" => c.set_flag(&v),
                "craft" => {
                    let p: Vec<&str> = v.split(',').collect();
                    if p.len() == 3 {
                        if let Some(cr) = crate::crafting::Craft::from_key(p[0]) {
                            c.crafts[cr.index()] = crate::crafting::CraftSkill { level: pu(p[1]).clamp(1, crate::crafting::MAX_CRAFT_LEVEL), exp: pu(p[2]) };
                        }
                    }
                }
                "food" => {
                    if let Some((id, left)) = v.split_once(',') {
                        if items::item(id).is_some() && pf(left) > 0.0 {
                            c.food = Some((id.to_string(), pf(left)));
                        }
                    }
                }
                "rested" => c.rested = pf(&v).clamp(0.0, crate::housing::WELL_RESTED_SECS),
                "house" => {
                    if let Some((spot, id)) = v.split_once(',') {
                        let spot = pu(spot) as usize;
                        if spot < crate::housing::SPOTS.len() && crate::housing::furniture_of(id).is_some() && !c.house.iter().any(|h| h.0 == spot) {
                            c.house.push((spot, id.to_string()));
                        }
                    }
                }
                "store" => {
                    if let Some((id, n)) = v.split_once(',') {
                        crate::housing::storage_add(&mut c.storage, id, pu(n));
                    }
                }
                "untracked" => c.untracked = v.split(',').filter(|x| !x.is_empty()).map(String::from).collect(),
                "hide_done" => c.hide_done_quests = pb(&v),
                "vault_clears" => c.vault_clears = pu(&v),
                _ => {}
            },
        }
    }
    if let Some(mut c) = cur.take() {
        finish(&mut c, saw_stats);
        a.characters.push(c);
    }
    for c in &mut a.characters {
        if c.active_pet.map(|i| i >= c.pets.len()).unwrap_or(false) {
            c.active_pet = None;
        }
    }
    if a.email.is_empty() || a.hash_hex.is_empty() {
        return Err("save file is missing account fields".into());
    }
    a.characters.truncate(MAX_CHARACTERS);
    Ok(a)
}

/// Where saves live: per-user app data folder, falling back to ./saves.
pub fn data_dir() -> PathBuf {
    if let Ok(p) = std::env::var("GALEBOUND_DATA") {
        return PathBuf::from(p);
    }
    // Android: the app's private storage (package name from Cargo.toml).
    #[cfg(target_os = "android")]
    {
        return PathBuf::from("/data/data/com.galebound.online/files");
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(p) = std::env::var("APPDATA") {
            return PathBuf::from(p).join("Galebound");
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(h) = std::env::var("HOME") {
            return PathBuf::from(h).join("Library/Application Support/Galebound");
        }
    }
    #[cfg(all(unix, not(target_os = "macos"), not(target_os = "android")))]
    {
        if let Ok(p) = std::env::var("XDG_DATA_HOME") {
            return PathBuf::from(p).join("galebound");
        }
        if let Ok(h) = std::env::var("HOME") {
            return PathBuf::from(h).join(".local/share/galebound");
        }
    }
    PathBuf::from("saves")
}

/// Write atomically: temp file then rename, so a crash never corrupts a save.
pub fn write_atomic(path: &std::path::Path, contents: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, contents)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip() {
        let mut c = CharacterSave::new("Skeng", Appearance { hair: 3, skin: 5, ..Default::default() });
        c.class = ClassId::Stormfist;
        c.stats.dex = 30;
        c.ap = 4;
        c.inventory.add("elixir", 2);
        c.inventory.add("chain_shirt", 1);
        c.inventory.equip[EquipSlot::Ring2.index()] = Some("copper_ring".into());
        c.inventory.equip[EquipSlot::Back.index()] = Some("gale_cape".into());
        c.hotbar[3] = HotbarSlot::Skill("thunder_palm".into());
        c.quests.push(QuestEntry { id: "first_breeze".into(), state: QuestState::Done, progress: 1 });
        c.set_flag("met_elder");
        c.x = 123.5;
        c.trainee_buys = 4;
        c.tomb = Some(17);
        c.tally.kills = 77;
        c.achievements = vec!["first_blood".into()];
        c.marks = 9;
        c.cosmetics = vec!["cos_halo".into()];
        c.cos_hat = 10;
        c.attendance = crate::achievements::Attendance { last_day: 20000, count: 3 };
        let mut pet = PetSave::new(PetSpecies::Sparkwing);
        pet.name = "Zap, the Bolt".into();
        pet.level = 7;
        pet.exp = 33;
        pet.hunger = 41.5;
        c.pets.push(PetSave::new(PetSpecies::BreezeFox));
        c.pets.push(pet);
        c.active_pet = Some(1);
        let a = Account {
            email: "a@b.co".into(),
            salt_hex: "00ff".into(),
            hash_hex: "abcd".into(),
            iterations: 10,
            remember_hash: Some("ee".into()),
            settings: Settings { layout: Layout::Arrows, display: DisplayMode::WindowedFullscreen, minimap_size: 3, minimap_whole: true, ..Default::default() },
            characters: vec![c],
            created_unix: 42,
        };
        let txt = encode_account(&a);
        let b = decode_account(&txt).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn characters_and_settings_travel_on_their_own() {
        let mut c = CharacterSave::new("Skeng", Appearance { hair: 2, ..Default::default() });
        c.level = 9;
        c.inventory.add("pet_food", 3);
        c.set_flag("gate_open");
        c.pets.push(crate::pets::PetSave::new(PetSpecies::Puffling));
        c.active_pet = Some(0);
        let back = decode_character(&encode_character(&c)).unwrap();
        assert_eq!(back, c);
        let st = Settings { layout: Layout::ClassicMmo, minimap_size: 1, fx_companions: FX_MINE, fx_skills: FX_OFF, ..Default::default() };
        assert_eq!(decode_settings(&encode_settings(&st)), st);
        assert!(decode_character("garbage").is_none());
    }

    #[test]
    fn old_saves_migrate() {
        let old = "version=2\nemail=a@b.co\nsalt=00\nhash=ab\n[character]\nname=Old\nclass=windrunner\nlevel=11\npotions=4\n";
        let a = decode_account(old).unwrap();
        let c = &a.characters[0];
        assert_eq!(c.class, ClassId::Windrunner);
        assert_eq!(c.inventory.count("hp_potion"), 4);
        assert_eq!(c.stats.total(), stats::START_STATS.total() + 50);
        assert!(c.stats.dex > c.stats.int, "auto-assigned to the class's main stat");
        assert_eq!(c.inventory.equip[0].as_deref(), Some("ash_bow"));
        let fresh = CharacterSave::new("New", Appearance::default());
        assert_eq!(fresh.class, ClassId::Novice);
    }

    #[test]
    fn old_equipment_slots_move_to_the_new_ones() {
        let old = "version=4\nemail=a@b.co\nsalt=00\nhash=ab\n[character]\nname=Old\nclass=vanguard\nlevel=12\nstats=10,5,5,5\n\
                   equip=weapon,iron_sword\nequip=armor,chain_shirt\nequip=boots,leather_boots\nequip=accessory,rogue_coin\n";
        let a = decode_account(old).unwrap();
        let c = &a.characters[0];
        let e = |s: EquipSlot| c.inventory.equip[s.index()].as_deref();
        assert_eq!(e(EquipSlot::Weapon), Some("iron_sword"));
        assert_eq!(e(EquipSlot::Torso), Some("chain_shirt"));
        assert_eq!(e(EquipSlot::Boots), Some("leather_boots"));
        assert_eq!(e(EquipSlot::Earring1), Some("rogue_coin"), "the old accessory finds its new slot");
        // and the new format round-trips, both ring slots included
        let mut c2 = c.clone();
        c2.inventory.equip[EquipSlot::Ring1.index()] = Some("copper_ring".into());
        c2.inventory.equip[EquipSlot::Ring2.index()] = Some("power_band".into());
        let back = decode_character(&encode_character(&c2)).unwrap();
        assert_eq!(back.inventory.equip, c2.inventory.equip);
    }

}
