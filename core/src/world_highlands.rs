//! 0.15.0: the Skyreach Highlands (level 30-60 content) and the co-op
//! dungeon registry. A child module of `world`.
//!
//! Every co-op dungeon shares the Tempest Vault's shape: a hall of monsters
//! (flyers included), a walled arena for the gate boss and the gate it
//! guards, a second hall, then the final boss's arena and the victory hall.
//! Each party gets its own copy; monsters don't respawn.

use super::*;
use EnemyKind as E;

/// A co-op dungeon.
pub struct DungeonSpec {
    pub map: &'static str,
    pub name: &'static str,
    pub theme: Theme,
    /// The NPC who sends you in, and where.
    pub npc: &'static str,
    pub npc_name: &'static str,
    pub home: &'static str,
    pub home_x: f32,
    pub level: u32,
    /// Choice label in the NPC's dialogue.
    pub enter: &'static str,
    /// The gate boss and the flag its death sets (None: no gate).
    pub gate: Option<(EnemyKind, &'static str)>,
    /// Final boss(es): all must fall for the clear.
    pub finals: &'static [EnemyKind],
    /// The cache everyone who's there gets for the clear.
    pub cache: &'static str,
    /// Hall monsters by role: [ground, shooting flyer, diving flyer, ground 2, heavy, crab].
    pub mobs: [EnemyKind; 6],
    pub signs: [&'static str; 4],
    pub levels: (u32, u32),
}

pub const DUNGEONS: &[DungeonSpec] = &[
    DungeonSpec {
        map: VAULT_MAP,
        name: "Tempest Vault",
        theme: Theme::Arcane,
        npc: VAULT_NPC,
        npc_name: "Maren",
        home: "wayfarers_crossroads",
        home_x: 4300.0,
        level: VAULT_LEVEL,
        enter: "Enter the Tempest Vault",
        gate: Some((E::Gatekeeper, VAULT_GATE_FLAG)),
        finals: &[E::Stormwarden],
        cache: "vault_cache",
        mobs: [E::Cryptling, E::VaultWisp, E::Stormkite, E::Vaultbeak, E::Ruinstump, E::Vaultcrab],
        signs: ["", "", "", ""],
        levels: (20, 25),
    },
    DungeonSpec {
        map: "frozen_throne",
        name: "The Frozen Throne",
        theme: Theme::Frost,
        npc: "delver_hrolf",
        npc_name: "Hrolf",
        home: "skyreach_citadel",
        home_x: 3900.0,
        level: 35,
        enter: "Enter the Frozen Throne",
        gate: Some((E::RimeColossus, "ft_gate")),
        finals: &[E::Glacia],
        cache: "frozen_cache",
        mobs: [E::Frostling, E::IceWisp, E::FrostBat, E::Rimeclaw, E::GlacialYeti, E::RimeGolem],
        signs: [
            "The Frozen Throne. Frost Bats nest in the rafters - keep an eye on the air.",
            "The Rime Colossus. When the blizzard howls, KEEP MOVING or freeze solid. Jump the snowballs rolling in from the walls.",
            "The gate thaws when the Colossus falls.",
            "Glacia's lair. Icicles fall wherever you stand - keep moving. When she hides in ice, smash the Frost Crystals to break her shield.",
        ],
        levels: (35, 40),
    },
    DungeonSpec {
        map: "cinder_sanctum",
        name: "The Cinder Sanctum",
        theme: Theme::Ember,
        npc: "delver_ashka",
        npc_name: "Ashka",
        home: "skyreach_citadel",
        home_x: 4150.0,
        level: 45,
        enter: "Enter the Cinder Sanctum",
        gate: Some((E::Magmaw, "cs_gate")),
        finals: &[E::Ignaroth],
        cache: "cinder_cache",
        mobs: [E::AshenStump, E::EmberWisp, E::FireBat, E::FlameScorpion, E::ObsidianGolem, E::MagmaCrab],
        signs: [
            "The Cinder Sanctum. Fire Bats wheel overhead and the floor is hot enough to cook on.",
            "Magmaw. When it glows white-hot, hit it with everything you have to cool it down - or it explodes. Stay out of the lava pools.",
            "The gate cools when Magmaw falls.",
            "Ignaroth, the Cinder Dragon. When half the arena catches fire, run to the other half. Duck its breath and brace for its wings.",
        ],
        levels: (45, 50),
    },
    DungeonSpec {
        map: "celestial_spire",
        name: "The Celestial Spire",
        theme: Theme::Sky,
        npc: "delver_vesper",
        npc_name: "Vesper",
        home: "skyreach_citadel",
        home_x: 4400.0,
        level: 55,
        enter: "Enter the Celestial Spire",
        gate: None,
        finals: &[E::Sol, E::Luna],
        cache: "celestial_cache",
        mobs: [E::VoidScorpion, E::SeraphWisp, E::StarBat, E::StarWyrmling, E::AstralGolem, E::ThunderGolem],
        signs: [
            "The Celestial Spire. The stars themselves guard this place.",
            "The Hall of Constellations. Clear it - the Twins are watching.",
            "Beyond: the Twins' throne.",
            "Sol and Luna, the Aether Twins. Kill one and the other eclipses - finish it within 10 seconds or it brings its twin back!",
        ],
        levels: (55, 60),
    },
];

pub fn dungeon(map: &str) -> Option<&'static DungeonSpec> {
    let map = base_key(map);
    DUNGEONS.iter().find(|d| d.map == map)
}

pub fn is_dungeon(map: &str) -> bool {
    dungeon(map).is_some()
}

pub fn dungeon_for_npc(npc: &str) -> Option<&'static DungeonSpec> {
    DUNGEONS.iter().find(|d| d.npc == npc)
}

/// Is this monster a dungeon boss (they have no quest to turn in)?
pub fn is_dungeon_boss(k: EnemyKind) -> bool {
    DUNGEONS.iter().any(|d| d.finals.contains(&k) || d.gate.map(|g| g.0) == Some(k))
}

/// The dungeon whose gate this boss guards.
pub fn gate_of(k: EnemyKind) -> Option<&'static DungeonSpec> {
    DUNGEONS.iter().find(|d| d.gate.map(|g| g.0) == Some(k))
}

/// The dungeon this boss is (one of) the final boss(es) of.
pub fn final_of(k: EnemyKind) -> Option<&'static DungeonSpec> {
    DUNGEONS.iter().find(|d| d.finals.contains(&k))
}

/// Middle of a dungeon's final arena (the Twins' arena, Ignaroth's inferno).
pub fn final_arena_x(map: &str) -> f32 {
    let _ = map;
    SW_ARENA_X
}

/// A 0.15.0 dungeon, built on the Tempest Vault's layout.
pub(super) fn dungeon_map(d: &'static DungeonSpec) -> Map {
    use DecoKind::*;
    let mut m = tempest_vault();
    let g = GROUND;
    let [ground_a, shooter, diver, ground_b, heavy, crab] = d.mobs;
    let swap = |k: EnemyKind| match k {
        E::Cryptling => ground_a,
        E::VaultWisp => shooter,
        E::Galewraith => shooter,
        E::Stormkite => diver,
        E::Vaultbeak => ground_b,
        E::Ruinstump => heavy,
        E::Vaultcrab => crab,
        other => other,
    };
    let mut fixed: Vec<(EnemyKind, f32, f32)> = m.fixed_enemies.iter().filter(|f| !f.0.is_boss()).map(|&(k, x, y)| (swap(k), x, y)).collect();
    match d.gate {
        Some((boss, flag)) => {
            fixed.push((boss, GK_ARENA_X, g));
            m.gates = vec![Gate { solid: Aabb::new(VAULT_GATE_X, g - 400.0, 40.0, 400.0), open_flag: flag }];
        }
        None => {
            // no gate boss: an ambush of the hall's toughest instead
            m.gates.clear();
            for (k, x) in [(heavy, GK_ARENA_X - 300.0), (crab, GK_ARENA_X), (diver, GK_ARENA_X + 200.0), (heavy, GK_ARENA_X + 400.0)] {
                fixed.push((k, x, g));
            }
        }
    }
    match d.finals {
        [one] => fixed.push((*one, SW_ARENA_X, g)),
        many => {
            for (i, k) in many.iter().enumerate() {
                fixed.push((*k, SW_ARENA_X + (i as f32 - 0.5) * 320.0, g));
            }
        }
    }
    m.fixed_enemies = fixed;
    m.key = d.map;
    m.name = d.name;
    m.theme = d.theme;
    m.portals = vec![
        portal(40.0, g, d.home, d.home_x, "Leave the dungeon", None, ""),
        portal(m.width - 160.0, g, d.home, d.home_x, "Leave the dungeon (victory!)", None, ""),
    ];
    let xs = [260.0, 2360.0, 3880.0, 6000.0];
    m.signs = xs.iter().zip(d.signs.iter()).map(|(&x, &text)| Sign { x, y: g, text }).collect();
    let (lamp, prop) = match d.theme {
        Theme::Frost => (Crystal, Pine),
        Theme::Ember => (LanternPost, Rock),
        _ => (Crystal, LanternPost),
    };
    for dc in m.decos.iter_mut() {
        if dc.kind == Crystal {
            dc.kind = lamp;
        } else if dc.kind == LanternPost {
            dc.kind = prop;
        } else if dc.kind == Gravestone {
            dc.kind = Rock;
        }
    }
    m
}

// ------------------------------------------------------------------ Skyreach Citadel

pub const CITADEL: &str = "skyreach_citadel";
const CITADEL_W: f32 = 4800.0;
/// Where the Skybridge to the Stormcrown Isles leaves the Citadel.
const SKYBRIDGE_X: f32 = 1750.0;
/// The quest givers in the Citadel.
pub const CITADEL_NPCS: &[(&str, &str, &str, f32)] = &[
    ("captain_brannoch", "Captain Brannoch", "Frost Warden", 300.0),
    ("sage_ilyra", "Sage Ilyra", "Ember Scholar", 1130.0),
    ("skywatcher_oriel", "Skywatcher Oriel", "Watcher of the Isles", 3040.0),
];

/// The Citadel: a class town's services plus the Highlands' roads and
/// dungeon delvers.
pub(super) fn citadel(t: &'static TownSpec) -> Map {
    use DecoKind::*;
    let mut m = class_town(t);
    let g = GROUND;
    m.width = CITADEL_W;
    m.solids = vec![ground(0.0, CITADEL_W, g), Aabb::new(-40.0, 0.0, 40.0, 900.0), Aabb::new(CITADEL_W, 0.0, 40.0, 900.0)];
    for (id, name, title, x) in CITADEL_NPCS {
        m.npcs.push(npc(id, name, title, *x, g, look(1, 3, 2, 1, 4, 4, 1, 4, 3, 2), *x > 2000.0));
    }
    for d in DUNGEONS.iter().filter(|d| d.home == CITADEL) {
        m.npcs.push(npc(d.npc, d.npc_name, "Dungeon Delver - co-op", d.home_x, g, look(0, 5, 1, 4, 5, 6, 4, 6, 3, 5), true));
        m.decos.push(deco(VaultDoor, d.home_x + 110.0, g, 0.9, 0));
    }
    let storm = wild("stormcrown_isles").expect("stormcrown");
    let ember = wild("emberfall_caldera").expect("emberfall");
    m.portals.push(portal(SKYBRIDGE_X, g, storm.key, 220.0, storm.label, None, ""));
    m.portals.push(portal(CITADEL_W - 130.0, g, ember.key, 220.0, ember.label, None, ""));
    m.decos.push(deco(Signpost, SKYBRIDGE_X - 40.0, g, 1.0, 1));
    m.decos.push(deco(Signpost, CITADEL_W - 170.0, g, 1.0, 1));
    m.signs.retain(|s| !s.text.starts_with("The class hall"));
    m.signs.push(Sign { x: 3560.0, y: g, text: "Grandmaster Aldric's hall. Level 30 adventurers of every class may choose their advanced class here." });
    m.signs.push(Sign { x: SKYBRIDGE_X + 160.0, y: g, text: storm.sign });
    m.signs.push(Sign { x: CITADEL_W - 330.0, y: g, text: ember.sign });
    m.signs.push(Sign { x: 3780.0, y: g, text: "The Delvers' Rest. Three co-op dungeons: the Frozen Throne (Lv 35), the Cinder Sanctum (Lv 45) and the Celestial Spire (Lv 55)." });
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dungeons_have_bosses_flyers_and_ways_out() {
        for d in DUNGEONS {
            let m = load_map(d.map).expect(d.map);
            assert!(m.fixed_enemies.iter().filter(|f| f.0.def().ai.is_floater()).count() >= 4, "{} needs flyers", d.map);
            for b in d.finals {
                assert!(m.fixed_enemies.iter().any(|f| f.0 == *b), "{} missing {:?}", d.map, b);
            }
            if let Some((g, flag)) = d.gate {
                assert!(m.fixed_enemies.iter().any(|f| f.0 == g));
                assert_eq!(m.gates[0].open_flag, flag);
            }
            assert!(m.portals.iter().all(|p| p.target_map == d.home));
            assert!(crate::items::item(d.cache).is_some());
            let home = load_map(d.home).unwrap();
            assert!(home.npcs.iter().any(|n| n.id == d.npc), "{} has no delver", d.home);
        }
    }
}
