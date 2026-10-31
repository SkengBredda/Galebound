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
use std::path::PathBuf;

pub const MAX_CHARACTERS: usize = 3;
pub const SAVE_VERSION: u32 = 3;

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub layout: Layout,
    /// Brawlhalla-style option: pressing Up also jumps.
    pub tap_up_to_jump: bool,
    pub show_damage_numbers: bool,
    pub screen_shake: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { layout: Layout::Wasd, tap_up_to_jump: false, show_damage_numbers: true, screen_shake: true }
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
}

impl CharacterSave {
    /// A brand-new character: everyone starts as a Novice.
    pub fn new(name: &str, look: Appearance) -> Self {
        let mut inventory = Inventory::default();
        inventory.add("hp_potion", 3);
        inventory.equip[EquipSlot::Weapon.index()] = Some(items::starter_weapon(ClassId::Novice).to_string());
        inventory.equip[EquipSlot::Armor.index()] = Some("novice_garb".to_string());
        let mut hotbar = vec![HotbarSlot::Empty; HOTBAR_SLOTS];
        hotbar[0] = HotbarSlot::Skill("power_strike".into());
        hotbar[HOTBAR_SLOTS - 2] = HotbarSlot::Item("hp_potion".into());
        hotbar[HOTBAR_SLOTS - 1] = HotbarSlot::Item("mp_potion".into());
        CharacterSave {
            name: name.to_string(),
            class: ClassId::Novice,
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
        }
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
    kv("layout", a.settings.layout.key().to_string());
    kv("tap_up_jump", a.settings.tap_up_to_jump.to_string());
    kv("damage_numbers", a.settings.show_damage_numbers.to_string());
    kv("screen_shake", a.settings.screen_shake.to_string());
    for c in &a.characters {
        s.push_str("[character]\n");
        let mut kv = |k: &str, v: String| {
            s.push_str(k);
            s.push('=');
            s.push_str(&esc(&v));
            s.push('\n');
        };
        kv("name", c.name.clone());
        kv("class", c.class.key().to_string());
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
        for q in &c.quests {
            kv("quest", format!("{},{},{}", q.id, qstate_str(q.state), q.progress));
        }
        for f in &c.flags {
            kv("flag", f.clone());
        }
    }
    s
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
                "screen_shake" => a.settings.screen_shake = pb(&v),
                _ => {}
            },
            Some(c) => match k {
                "name" => c.name = v,
                "class" => c.class = ClassId::from_key(&v).unwrap_or(ClassId::Novice),
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
                        if let (Some(slot), Some(def)) = (EquipSlot::from_key(slot), items::item(id)) {
                            if def.slot() == Some(slot) {
                                c.inventory.equip[slot.index()] = Some(id.to_string());
                            }
                        }
                    }
                }
                "hotbar" => {
                    let mut h: Vec<HotbarSlot> = v.split_whitespace().map(HotbarSlot::decode).collect();
                    h.resize(HOTBAR_SLOTS, HotbarSlot::Empty);
                    c.hotbar = h;
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
                _ => {}
            },
        }
    }
    if let Some(mut c) = cur.take() {
        finish(&mut c, saw_stats);
        a.characters.push(c);
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
    #[cfg(all(unix, not(target_os = "macos")))]
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
        c.inventory.equip[EquipSlot::Accessory.index()] = Some("copper_ring".into());
        c.hotbar[3] = HotbarSlot::Skill("thunder_palm".into());
        c.quests.push(QuestEntry { id: "first_breeze".into(), state: QuestState::Done, progress: 1 });
        c.set_flag("met_elder");
        c.x = 123.5;
        let a = Account {
            email: "a@b.co".into(),
            salt_hex: "00ff".into(),
            hash_hex: "abcd".into(),
            iterations: 10,
            remember_hash: Some("ee".into()),
            settings: Settings { layout: Layout::Arrows, ..Default::default() },
            characters: vec![c],
            created_unix: 42,
        };
        let txt = encode_account(&a);
        let b = decode_account(&txt).unwrap();
        assert_eq!(a, b);
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
}
