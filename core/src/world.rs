//! Map data: terrain, NPCs, enemy spawns, portals, signs and decorations.

use crate::appearance::Appearance;
use crate::math::Aabb;

pub const START_MAP: &str = "dawnwind_glade";
pub const GROUND: f32 = 640.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    Glade,
    Town,
    /// Sunny rolling farmland.
    Fields,
    /// Haunted forest at night (moon, graves, glowing wisps).
    Woods,
    /// Rocky cliffs at sunset.
    Cliffs,
    /// Starlit violet heights with floating crystals (Starfall Spire).
    Arcane,
    /// Inside a building (shops).
    Interior,
}

pub use crate::enemies::EnemyKind;

#[derive(Clone, Debug)]
pub struct NpcDef {
    pub id: &'static str,
    pub name: &'static str,
    pub title: &'static str,
    pub x: f32,
    pub y: f32, // feet
    pub look: Appearance,
    pub faces_left: bool,
}

#[derive(Clone, Debug)]
pub struct SpawnZone {
    pub kind: EnemyKind,
    pub x0: f32,
    pub x1: f32,
    pub y: f32, // feet height to spawn at
    pub max: usize,
    pub respawn: f32,
}

#[derive(Clone, Debug)]
pub struct Portal {
    pub area: Aabb,
    pub target_map: &'static str,
    pub target_x: f32,
    pub label: &'static str,
    /// Quest/story flag required to use it; None = always open.
    pub requires_flag: Option<&'static str>,
    pub locked_text: &'static str,
    /// Minimum character level to pass (0 = none).
    pub min_level: u32,
}

/// A boss that appears while a quest is active.
#[derive(Clone, Debug)]
pub struct BossSpawn {
    pub kind: EnemyKind,
    pub x: f32,
    pub y: f32,
    pub quest: &'static str,
}

#[derive(Clone, Debug)]
pub struct Gate {
    pub solid: Aabb,
    pub open_flag: &'static str,
}

#[derive(Clone, Debug)]
pub struct Sign {
    pub x: f32,
    pub y: f32, // feet
    /// May contain {Action} placeholders the renderer replaces with key names.
    pub text: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecoKind {
    TreeTall,
    TreeRound,
    Bush,
    Rock,
    Flowers,
    Fence,
    Tent,
    Campfire,
    House,
    Shop,
    Lamp,
    Well,
    Banner,
    Windmill,
    Crate,
    Haystack,
    Scarecrow,
    DeadTree,
    Gravestone,
    Cross,
    Mushrooms,
    Pine,
    LanternPost,
    Crystal,
    // Class towns
    StoreGeneral,
    StoreWeapons,
    StoreArmor,
    Inn,
    ClassHall,
    Signpost,
    // Shop interiors
    Counter,
    Shelf,
    WeaponRack,
    ArmorStand,
    Rug,
    Fireplace,
    WallTorch,
    Window,
    Barrel,
}

#[derive(Clone, Debug)]
pub struct Deco {
    pub kind: DecoKind,
    pub x: f32,
    pub y: f32, // base (feet)
    pub scale: f32,
    /// 0 = behind everything (far), 1 = behind actors, 2 = in front of actors
    pub layer: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClimbKind {
    Ladder,
    Rope,
    Vine,
}

/// Something you can climb with Up/Down: a vertical strip at `x` from `top`
/// (where you step off onto a ledge or platform) down to `bottom`.
#[derive(Clone, Debug)]
pub struct Climb {
    pub x: f32,
    pub top: f32,
    pub bottom: f32,
    pub kind: ClimbKind,
}

pub fn climb(kind: ClimbKind, x: f32, top: f32, bottom: f32) -> Climb {
    Climb { x, top, bottom, kind }
}

#[derive(Clone, Debug)]
pub struct Region {
    pub id: &'static str,
    pub area: Aabb,
}

#[derive(Clone, Debug)]
pub struct Map {
    pub key: &'static str,
    pub name: &'static str,
    pub theme: Theme,
    pub width: f32,
    pub height: f32,
    pub spawn: (f32, f32),
    pub solids: Vec<Aabb>,
    pub platforms: Vec<Aabb>,
    pub gates: Vec<Gate>,
    pub npcs: Vec<NpcDef>,
    pub spawns: Vec<SpawnZone>,
    pub fixed_enemies: Vec<(EnemyKind, f32, f32)>,
    pub portals: Vec<Portal>,
    pub signs: Vec<Sign>,
    pub decos: Vec<Deco>,
    pub regions: Vec<Region>,
    pub bosses: Vec<BossSpawn>,
    /// X positions you respawn at after being defeated (furthest one behind you).
    pub checkpoints: Vec<f32>,
    /// Ladders, ropes and vines.
    pub climbs: Vec<Climb>,
    /// A fall below this Y counts as falling out of the map.
    pub kill_y: f32,
}

impl Map {
    pub fn bounds(&self) -> Aabb {
        Aabb::new(0.0, -600.0, self.width, self.height + 600.0)
    }
}

/// NPC look. `top`/`bottom` use the clothing list *without* the "None" entry
/// (0 = first real item), so NPCs are always dressed.
#[allow(clippy::too_many_arguments)]
fn look(body: u8, hair: u8, hc: u8, face: u8, top: u8, td: u8, bottom: u8, bd: u8, shoes: u8, skin: u8) -> Appearance {
    Appearance { body, hair, hair_color: hc, face, top: top + 1, top_dye: td, bottom: bottom + 1, bottom_dye: bd, shoes, skin }
}

fn deco(kind: DecoKind, x: f32, y: f32, scale: f32, layer: u8) -> Deco {
    Deco { kind, x, y, scale, layer }
}

/// Solid ground block from `x0` to `x1` with its top at `top`.
fn ground(x0: f32, x1: f32, top: f32) -> Aabb {
    Aabb::new(x0, top, x1 - x0, 900.0 - top)
}
fn plat(x: f32, top: f32, w: f32) -> Aabb {
    Aabb::new(x, top, w, 14.0)
}

pub fn load_map(key: &str) -> Option<Map> {
    match key {
        "dawnwind_glade" => Some(dawnwind_glade()),
        "breezehollow" => Some(breezehollow()),
        "whistling_fields" => Some(vertical(whistling_fields())),
        "thornvale_woods" => Some(vertical(thornvale_woods())),
        "windcrag_cliffs" => Some(vertical(windcrag_cliffs())),
        "wayfarers_crossroads" => Some(crossroads()),
        other => {
            if let Some(w) = wild(other) {
                return Some(wild_map(w));
            }
            if let Some(t) = TOWNS.iter().find(|t| t.key == other) {
                return Some(class_town(t));
            }
            for t in TOWNS {
                for (i, shop) in t.shops.iter().enumerate() {
                    if shop.map == other {
                        return Some(shop_interior(t, i));
                    }
                }
            }
            None
        }
    }
}

/// Every map key, for tests and tools.
pub fn all_map_keys() -> Vec<&'static str> {
    let mut v = vec!["dawnwind_glade", "breezehollow", "whistling_fields", "thornvale_woods", "windcrag_cliffs", "wayfarers_crossroads"];
    v.extend(WILDS.iter().map(|w| w.key));
    for t in TOWNS {
        v.push(t.key);
        for s in &t.shops {
            v.push(s.map);
        }
    }
    v
}

/// The tutorial zone: a windy forest glade that teaches movement, dodging,
/// light/heavy attacks, then ends in a mini-boss before the first town.
fn dawnwind_glade() -> Map {
    use DecoKind::*;
    let g = GROUND;
    let solids = vec![
        // A: camp
        ground(0.0, 1300.0, g),
        // B: Windstep Rise (staircase + cliff that needs a double jump)
        ground(1300.0, 1480.0, g - 80.0),
        ground(1480.0, 1940.0, g - 190.0),
        ground(1940.0, 2240.0, g - 370.0), // the Lookout: needs a double jump
        // C: training ground
        ground(2240.0, 3400.0, g),
        // D: Mossy Hollow (a gentle dip)
        ground(3400.0, 3600.0, g),
        ground(3600.0, 4700.0, g + 60.0),
        ground(4700.0, 6800.0, g),
        // far wall
        Aabb::new(6780.0, 0.0, 40.0, 900.0),
    ];
    let platforms = vec![
        plat(2240.0, g - 200.0, 120.0), // way back down
        plat(3750.0, g - 90.0, 180.0),
        plat(4050.0, g - 150.0, 200.0),
        plat(4400.0, g - 90.0, 180.0),
        plat(5600.0, g - 140.0, 160.0),
        plat(5900.0, g - 140.0, 160.0),
    ];
    let npcs = vec![
        NpcDef { id: "elder_wynn", name: "Elder Wynn", title: "Glade Keeper", x: 470.0, y: g, look: look(1, 3, 4, 0, 3, 5, 4, 4, 4, 2), faces_left: true },
        NpcDef { id: "scout_tamsin", name: "Tamsin", title: "Wind Scout", x: 2420.0, y: g, look: look(1, 2, 3, 2, 5, 3, 1, 0, 1, 4), faces_left: true },
        NpcDef { id: "sergeant_brask", name: "Sergeant Brask", title: "Drill Master", x: 2780.0, y: g, look: look(0, 4, 0, 4, 4, 4, 3, 4, 3, 6), faces_left: false },
        NpcDef { id: "ranger_pell", name: "Pell", title: "Glade Ranger", x: 3480.0, y: g, look: look(0, 0, 6, 1, 1, 0, 0, 0, 5, 3), faces_left: false },
        NpcDef { id: "warden_ilsa", name: "Warden Ilsa", title: "Keeper of the Old Gate", x: 5050.0, y: g, look: look(1, 7, 7, 1, 4, 1, 5, 1, 3, 1), faces_left: false },
    ];
    let spawns = vec![
        SpawnZone { kind: EnemyKind::Mossling, x0: 3650.0, x1: 4650.0, y: g + 60.0, max: 7, respawn: 3.5 },
        SpawnZone { kind: EnemyKind::Burrowbeak, x0: 3800.0, x1: 4600.0, y: g + 60.0, max: 3, respawn: 6.0 },
    ];
    let fixed_enemies = vec![
        (EnemyKind::TrainingDummy, 2950.0, g),
        (EnemyKind::TrainingDummy, 3080.0, g),
        (EnemyKind::TrainingDummy, 3210.0, g),
    ];
    let portals = vec![Portal {
        area: Aabb::new(6560.0, g - 120.0, 90.0, 120.0),
        target_map: "breezehollow",
        target_x: 220.0,
        label: "To Breezehollow",
        requires_flag: Some("gate_open"),
        locked_text: "The Old Gate is sealed. Speak with Warden Ilsa.",
        min_level: 0,
    }];
    let gates = vec![Gate { solid: Aabb::new(6440.0, g - 260.0, 40.0, 260.0), open_flag: "gate_open" }];
    let signs = vec![
        Sign { x: 260.0, y: g, text: "Move with {Left} {Right}. Jump with {Jump} - press it again in the air to double jump!" },
        Sign { x: 1180.0, y: g, text: "Windstep Rise: climb to the Lookout. Too high? Jump, then jump again at the top of your arc!" },
        Sign { x: 2290.0, y: g, text: "Hold {Down} and press {Jump} on a wooden platform to drop through." },
        Sign { x: 2600.0, y: g, text: "{Dodge} dodges. Dodge through attacks - you're invulnerable for a moment." },
        Sign { x: 2860.0, y: g, text: "{Light} light attack, {Heavy} heavy. Hold a direction ({Left}/{Right} or {Down}) to change the move - they differ in the air too!" },
        Sign { x: 3560.0, y: g, text: "Mossy Hollow - beware of Mosslings. Drink a potion with {Potion}." },
        Sign { x: 5250.0, y: g, text: "The Old Gate. Beyond lies Breezehollow." },
    ];
    let regions = vec![Region { id: "lookout", area: Aabb::new(1950.0, g - 490.0, 290.0, 125.0) }];
    let mut decos = vec![
        deco(Tent, 120.0, g, 1.0, 1),
        deco(Campfire, 300.0, g, 1.0, 1),
        deco(TreeTall, 40.0, g, 1.3, 0),
        deco(TreeRound, 640.0, g, 1.1, 0),
        deco(Bush, 760.0, g, 1.0, 2),
        deco(Flowers, 860.0, g, 1.0, 1),
        deco(TreeTall, 980.0, g, 1.2, 0),
        deco(Rock, 1380.0, g - 80.0, 0.8, 1),
        deco(TreeRound, 1560.0, g - 190.0, 1.0, 0),
        deco(Banner, 2080.0, g - 370.0, 1.2, 1),
        deco(Flowers, 2160.0, g - 370.0, 1.0, 1),
        deco(Fence, 2700.0, g, 1.0, 1),
        deco(Crate, 2560.0, g, 1.0, 1),
        deco(Crate, 2590.0, g - 34.0, 0.8, 1),
        deco(Fence, 3300.0, g, 1.0, 1),
        deco(TreeTall, 3700.0, g + 60.0, 1.4, 0),
        deco(Bush, 3900.0, g + 60.0, 1.2, 2),
        deco(TreeRound, 4200.0, g + 60.0, 1.3, 0),
        deco(Rock, 4500.0, g + 60.0, 1.0, 1),
        deco(Bush, 4620.0, g + 60.0, 1.0, 2),
        deco(TreeTall, 4900.0, g, 1.3, 0),
        deco(Lamp, 5180.0, g, 1.0, 1),
        deco(Rock, 6000.0, g, 1.2, 1),
        deco(TreeRound, 6300.0, g, 1.2, 0),
        deco(Lamp, 6520.0, g, 1.0, 1),
        deco(Lamp, 6680.0, g, 1.0, 1),
    ];
    for i in 0..10 {
        decos.push(deco(Flowers, 5400.0 + i as f32 * 110.0, g, 0.8, 1));
    }
    Map {
        key: "dawnwind_glade",
        name: "Dawnwind Glade",
        theme: Theme::Glade,
        width: 6780.0,
        height: 900.0,
        spawn: (200.0, g),
        solids,
        platforms,
        gates,
        npcs,
        spawns,
        fixed_enemies,
        portals,
        signs,
        decos,
        regions,
        bosses: vec![BossSpawn { kind: EnemyKind::Bramblehorn, x: 6100.0, y: g, quest: "the_bramblehorn" }],
        checkpoints: vec![200.0, 2420.0, 3480.0, 5050.0],
        climbs: vec![],
        kill_y: 1100.0,
    }
}

/// The first town.
fn breezehollow() -> Map {
    use DecoKind::*;
    let g = GROUND;
    let solids = vec![ground(0.0, 4200.0, g), Aabb::new(-40.0, 0.0, 40.0, 900.0)];
    let platforms = vec![
        // rooftops you can hop onto
        plat(720.0, g - 170.0, 240.0),
        plat(1480.0, g - 190.0, 260.0),
        plat(2560.0, g - 170.0, 240.0),
        plat(3100.0, g - 120.0, 160.0),
    ];
    let npcs = vec![
        NpcDef { id: "mayor_odessa", name: "Mayor Odessa", title: "Mayor of Breezehollow", x: 1980.0, y: g, look: look(1, 7, 1, 0, 0, 2, 2, 2, 5, 5), faces_left: true },
        NpcDef { id: "mira", name: "Mira", title: "Potion Brewer", x: 1150.0, y: g, look: look(1, 5, 7, 2, 3, 6, 2, 6, 4, 0), faces_left: false },
        NpcDef { id: "garrick", name: "Garrick", title: "Blacksmith", x: 2860.0, y: g, look: look(0, 6, 3, 4, 1, 4, 3, 4, 3, 5), faces_left: true },
        NpcDef { id: "mentor_aurel", name: "Aurel", title: "Class Mentor", x: 3420.0, y: g, look: look(0, 3, 4, 1, 3, 1, 4, 1, 4, 3), faces_left: true },
        NpcDef { id: "kid_bo", name: "Bo", title: "Town Kid", x: 2300.0, y: g, look: look(0, 6, 2, 2, 2, 3, 1, 3, 1, 2), faces_left: false },
    ];
    let portals = vec![
        Portal {
            area: Aabb::new(30.0, g - 120.0, 90.0, 120.0),
            target_map: "dawnwind_glade",
            target_x: 6380.0,
            label: "To Dawnwind Glade",
            requires_flag: None,
            locked_text: "",
            min_level: 0,
        },
        Portal {
            area: Aabb::new(4060.0, g - 120.0, 90.0, 120.0),
            target_map: "whistling_fields",
            target_x: 220.0,
            label: "To Whistling Fields",
            requires_flag: Some("fields_open"),
            locked_text: "Mayor Odessa wants a word before you head east.",
            min_level: 0,
        },
    ];
    let mut portals = portals;
    portals.push(portal(3700.0, g, "wayfarers_crossroads", 220.0, "North Road: Wayfarer's Crossroads", None, ""));
    let mut npcs = npcs;
    npcs.push(npc(waystone_id("breezehollow"), "Waystone", "Fast travel", 1520.0, g, Appearance::default(), false));
    let signs = vec![
        Sign { x: 3620.0, y: g, text: "North Road: Wayfarer's Crossroads and the five class towns. Press {Up} at the road to travel." },
        Sign { x: 360.0, y: g, text: "Welcome to Breezehollow! Speak with Mayor Odessa in the town square." },
        Sign { x: 3900.0, y: g, text: "East: Whistling Fields. Farmland - mind the Puffwools." },
    ];
    let mut decos = vec![
        deco(House, 840.0, g, 1.0, 0),
        deco(Shop, 1180.0, g, 1.0, 0),
        deco(House, 1610.0, g, 1.1, 0),
        deco(Well, 1880.0, g, 1.0, 1),
        deco(Banner, 2060.0, g, 1.3, 1),
        deco(Windmill, 2400.0, g, 1.2, 0),
        deco(House, 2680.0, g, 1.0, 0),
        deco(Shop, 2960.0, g, 1.0, 0),
        deco(House, 3200.0, g, 0.9, 0),
        deco(TreeRound, 3600.0, g, 1.2, 0),
        deco(Fence, 3800.0, g, 1.0, 1),
        deco(Crate, 3000.0, g, 1.0, 1),
        deco(TreeTall, 500.0, g, 1.2, 0),
    ];
    for i in 0..9 {
        decos.push(deco(Lamp, 600.0 + i as f32 * 400.0, g, 1.0, 1));
    }
    for i in 0..6 {
        decos.push(deco(Flowers, 1700.0 + i as f32 * 90.0, g, 0.7, 2));
    }
    Map {
        key: "breezehollow",
        name: "Breezehollow",
        theme: Theme::Town,
        width: 4200.0,
        height: 900.0,
        spawn: (220.0, g),
        solids,
        platforms,
        gates: vec![],
        npcs,
        spawns: vec![],
        fixed_enemies: vec![],
        portals,
        signs,
        decos,
        regions: vec![],
        bosses: vec![],
        checkpoints: vec![],
        climbs: vec![],
        kill_y: 1100.0,
    }
}

fn portal(x: f32, floor: f32, target_map: &'static str, target_x: f32, label: &'static str, flag: Option<&'static str>, locked: &'static str) -> Portal {
    Portal { area: Aabb::new(x, floor - 120.0, 90.0, 120.0), target_map, target_x, label, requires_flag: flag, locked_text: locked, min_level: 0 }
}

fn npc(id: &'static str, name: &'static str, title: &'static str, x: f32, y: f32, look: Appearance, faces_left: bool) -> NpcDef {
    NpcDef { id, name, title, x, y, look, faces_left }
}

fn zone(kind: EnemyKind, x0: f32, x1: f32, y: f32, max: usize, respawn: f32) -> SpawnZone {
    SpawnZone { kind, x0, x1, y, max, respawn }
}

/// Zone 1 (Lv 5-8): sunny farmland terraces east of Breezehollow.
fn whistling_fields() -> Map {
    use DecoKind::*;
    use EnemyKind::*;
    let g = GROUND;
    let w = 5800.0;
    let solids = vec![
        ground(0.0, 1100.0, g),
        ground(1100.0, 1700.0, g - 60.0),
        ground(1700.0, 2600.0, g - 120.0),
        ground(2600.0, 3300.0, g - 60.0),
        ground(3300.0, 4400.0, g),
        ground(4400.0, w, g - 90.0),
        Aabb::new(-40.0, 0.0, 40.0, 900.0),
        Aabb::new(w - 20.0, 0.0, 40.0, 900.0),
    ];
    let platforms = vec![plat(1900.0, g - 240.0, 180.0), plat(2250.0, g - 300.0, 200.0), plat(3500.0, g - 110.0, 160.0), plat(3900.0, g - 170.0, 180.0)];
    let npcs = vec![
        npc("farmer_tilly", "Tilly", "Farmer", 430.0, g, look(1, 3, 2, 2, 0, 3, 1, 9, 0, 2), false),
        npc("windcaller_oren", "Oren", "Windcaller", 5300.0, g - 90.0, look(0, 6, 4, 0, 3, 7, 4, 1, 5, 4), true),
    ];
    let spawns = vec![
        zone(Puffwool, 1150.0, 1650.0, g - 60.0, 3, 4.0),
        zone(Puffwool, 1750.0, 2550.0, g - 120.0, 5, 4.0),
        zone(Puffwool, 3350.0, 4350.0, g, 5, 4.0),
        zone(Gustling, 2650.0, 3250.0, g - 60.0, 3, 6.0),
        zone(Gustling, 3400.0, 4350.0, g, 3, 6.0),
        zone(Gustling, 4500.0, 5000.0, g - 90.0, 2, 7.0),
    ];
    let portals = vec![
        portal(30.0, g, "breezehollow", 3900.0, "To Breezehollow", None, ""),
        portal(5640.0, g - 90.0, "thornvale_woods", 220.0, "To Thornvale Woods", Some("woods_open"), "Windcaller Oren is watching this road. Speak with him first."),
    ];
    let signs = vec![
        Sign { x: 300.0, y: g, text: "Whistling Fields. Puffwools are harmless... until you poke one." },
        Sign { x: 3380.0, y: g, text: "Gustlings spit wind orbs. Dodge through them with {Dodge}, then strike!" },
    ];
    let mut decos = vec![
        deco(Windmill, 700.0, g, 1.1, 0),
        deco(Haystack, 980.0, g, 1.0, 1),
        deco(Fence, 1300.0, g - 60.0, 1.0, 1),
        deco(Scarecrow, 2100.0, g - 120.0, 1.0, 1),
        deco(Haystack, 2420.0, g - 120.0, 1.1, 1),
        deco(TreeRound, 2900.0, g - 60.0, 1.2, 0),
        deco(Fence, 3600.0, g, 1.0, 1),
        deco(Haystack, 4100.0, g, 0.9, 1),
        deco(Scarecrow, 4700.0, g - 90.0, 1.0, 1),
        deco(Windmill, 5000.0, g - 90.0, 1.2, 0),
        deco(Bush, 1600.0, g - 60.0, 1.0, 2),
        deco(Bush, 3200.0, g - 60.0, 1.0, 2),
        deco(Banner, 5500.0, g - 90.0, 1.0, 1),
    ];
    for i in 0..14 {
        let x = 200.0 + i as f32 * 400.0;
        let y = if x < 1100.0 { g } else if x < 1700.0 { g - 60.0 } else if x < 2600.0 { g - 120.0 } else if x < 3300.0 { g - 60.0 } else if x < 4400.0 { g } else { g - 90.0 };
        decos.push(deco(Flowers, x, y, 0.9, 1));
    }
    Map {
        key: "whistling_fields",
        name: "Whistling Fields",
        theme: Theme::Fields,
        width: w,
        height: 900.0,
        spawn: (220.0, g),
        solids,
        platforms,
        gates: vec![],
        npcs,
        spawns,
        fixed_enemies: vec![],
        portals,
        signs,
        decos,
        regions: vec![],
        bosses: vec![],
        checkpoints: vec![220.0, 2000.0, 3400.0, 5100.0],
        climbs: vec![],
        kill_y: 1100.0,
    }
}

/// Zone 2 (Lv 9-12): a haunted forest at night, layered stone ledges and graves.
fn thornvale_woods() -> Map {
    use DecoKind::*;
    use EnemyKind::*;
    let g = GROUND;
    let w = 6200.0;
    let solids = vec![
        ground(0.0, 900.0, g),
        ground(900.0, 1500.0, g - 100.0),
        ground(1500.0, 2300.0, g + 40.0),
        ground(2300.0, 2800.0, g - 60.0),
        ground(2800.0, 3800.0, g - 160.0),
        ground(3800.0, 4500.0, g - 40.0),
        ground(4500.0, 5200.0, g - 130.0),
        ground(5200.0, w, g - 30.0),
        Aabb::new(-40.0, 0.0, 40.0, 900.0),
        Aabb::new(w - 20.0, 0.0, 40.0, 900.0),
    ];
    let platforms = vec![
        plat(1600.0, g - 80.0, 180.0),
        plat(1950.0, g - 150.0, 200.0),
        plat(3000.0, g - 300.0, 220.0),
        plat(3420.0, g - 290.0, 180.0),
        plat(4700.0, g - 270.0, 200.0),
    ];
    let npcs = vec![
        npc("gravekeeper_morrow", "Morrow", "Gravekeeper", 380.0, g, look(0, 6, 0, 3, 3, 4, 0, 8, 5, 1), false),
        npc("hermit_sable", "Sable", "Hermit of the Woods", 5520.0, g - 30.0, look(1, 8, 9, 0, 3, 8, 6, 8, 6, 7), true),
    ];
    let spawns = vec![
        zone(Gravecap, 1550.0, 2250.0, g + 40.0, 5, 4.5),
        zone(Lanternwisp, 1600.0, 2250.0, g + 40.0, 3, 6.0),
        zone(Rotstump, 2850.0, 3750.0, g - 160.0, 3, 7.0),
        zone(Lanternwisp, 2850.0, 3750.0, g - 160.0, 2, 7.0),
        zone(Gravecap, 3850.0, 4450.0, g - 40.0, 4, 4.5),
        zone(Rotstump, 4550.0, 5150.0, g - 130.0, 3, 7.0),
    ];
    let portals = vec![
        portal(30.0, g, "whistling_fields", 5450.0, "To Whistling Fields", None, ""),
        portal(6060.0, g - 30.0, "windcrag_cliffs", 220.0, "To Windcrag Cliffs", Some("cliffs_open"), "The cliff path is hidden. Hermit Sable may know the way."),
    ];
    let signs = vec![
        Sign { x: 260.0, y: g, text: "Thornvale Woods. The lanterns here move on their own. Do not follow them." },
        Sign { x: 2860.0, y: g - 160.0, text: "Rotstumps are slow but hit hard. Watch for the rumble before they lunge." },
    ];
    let mut decos = vec![
        deco(DeadTree, 150.0, g, 1.2, 0),
        deco(LanternPost, 600.0, g, 1.0, 1),
        deco(Gravestone, 1000.0, g - 100.0, 1.0, 1),
        deco(Cross, 1150.0, g - 100.0, 1.0, 1),
        deco(DeadTree, 1300.0, g - 100.0, 1.4, 0),
        deco(Mushrooms, 1700.0, g + 40.0, 1.0, 2),
        deco(Gravestone, 2050.0, g + 40.0, 1.1, 1),
        deco(DeadTree, 2200.0, g + 40.0, 1.3, 0),
        deco(Cross, 2450.0, g - 60.0, 1.0, 1),
        deco(LanternPost, 2700.0, g - 60.0, 1.0, 1),
        deco(DeadTree, 3100.0, g - 160.0, 1.5, 0),
        deco(Gravestone, 3300.0, g - 160.0, 1.0, 1),
        deco(Mushrooms, 3600.0, g - 160.0, 1.2, 2),
        deco(Cross, 3950.0, g - 40.0, 1.0, 1),
        deco(Gravestone, 4200.0, g - 40.0, 1.2, 1),
        deco(DeadTree, 4350.0, g - 40.0, 1.2, 0),
        deco(Mushrooms, 4800.0, g - 130.0, 1.0, 2),
        deco(DeadTree, 5000.0, g - 130.0, 1.4, 0),
        deco(LanternPost, 5350.0, g - 30.0, 1.0, 1),
        deco(Gravestone, 5800.0, g - 30.0, 1.0, 1),
        deco(LanternPost, 5980.0, g - 30.0, 1.0, 1),
    ];
    decos.push(deco(Rock, 2600.0, g - 60.0, 1.0, 1));
    Map {
        key: "thornvale_woods",
        name: "Thornvale Woods",
        theme: Theme::Woods,
        width: w,
        height: 900.0,
        spawn: (220.0, g),
        solids,
        platforms,
        gates: vec![],
        npcs,
        spawns,
        fixed_enemies: vec![],
        portals,
        signs,
        decos,
        regions: vec![],
        bosses: vec![],
        checkpoints: vec![220.0, 2350.0, 3850.0, 5300.0],
        climbs: vec![],
        kill_y: 1100.0,
    }
}

/// Zone 3 (Lv 12-15): a sunset climb up rocky cliffs to the storm-swept summit.
fn windcrag_cliffs() -> Map {
    use DecoKind::*;
    use EnemyKind::*;
    let g = GROUND;
    let w = 4800.0;
    let top = g - 400.0;
    let solids = vec![
        ground(0.0, 800.0, g),
        ground(800.0, 1200.0, g - 100.0),
        ground(1200.0, 1600.0, g - 200.0),
        ground(1600.0, 2200.0, g - 290.0),
        ground(2200.0, 2700.0, g - 200.0),
        ground(2700.0, 3100.0, g - 310.0),
        ground(3100.0, w, top),
        Aabb::new(-40.0, -600.0, 40.0, 1500.0),
        Aabb::new(w - 20.0, -600.0, 40.0, 1500.0),
    ];
    let platforms = vec![
        plat(900.0, g - 260.0, 160.0),
        plat(1300.0, g - 380.0, 180.0),
        plat(2300.0, g - 380.0, 200.0),
        plat(3400.0, top - 160.0, 200.0),
        plat(3950.0, top - 160.0, 200.0),
    ];
    let npcs = vec![
        npc("ranger_sela", "Sela", "Cliff Ranger", 360.0, g, look(1, 2, 3, 1, 5, 0, 3, 4, 0, 5), false),
        npc("stormwatcher_kael", "Kael", "Stormwatcher", 3260.0, top, look(0, 0, 4, 4, 4, 1, 4, 1, 3, 3), false),
    ];
    let spawns = vec![
        zone(CragCrab, 850.0, 1180.0, g - 100.0, 2, 6.0),
        zone(CragCrab, 1650.0, 2180.0, g - 290.0, 3, 6.0),
        zone(Galehawk, 1650.0, 2150.0, g - 290.0, 3, 7.0),
        zone(CragCrab, 2250.0, 2680.0, g - 200.0, 2, 6.0),
        zone(Galehawk, 2700.0, 3100.0, g - 310.0, 2, 7.0),
    ];
    let portals = vec![
        portal(30.0, g, "thornvale_woods", 5900.0, "To Thornvale Woods", None, ""),
        portal(4650.0, top, "windcrag_cliffs", 4400.0, "The Skyroad", Some("skyroad_open"), "The Skyroad beyond the summit isn't charted yet. (Next update!)"),
    ];
    let signs = vec![
        Sign { x: 260.0, y: g, text: "Windcrag Cliffs. Galehawks dive from above - dodge when they screech!" },
        Sign { x: 3180.0, y: top, text: "The Summit. Storms gather here." },
    ];
    let decos = vec![
        deco(Pine, 120.0, g, 1.2, 0),
        deco(Rock, 600.0, g, 1.0, 1),
        deco(Pine, 1000.0, g - 100.0, 1.0, 0),
        deco(Crystal, 1450.0, g - 200.0, 1.0, 1),
        deco(Pine, 1800.0, g - 290.0, 1.3, 0),
        deco(Rock, 2050.0, g - 290.0, 1.2, 1),
        deco(Crystal, 2500.0, g - 200.0, 1.2, 1),
        deco(Pine, 2900.0, g - 310.0, 1.1, 0),
        deco(Banner, 3300.0, top, 1.2, 1),
        deco(Crystal, 3700.0, top, 1.4, 1),
        deco(Rock, 4100.0, top, 1.3, 1),
        deco(Pine, 4500.0, top, 1.2, 0),
        deco(Banner, 4600.0, top, 1.0, 1),
    ];
    Map {
        key: "windcrag_cliffs",
        name: "Windcrag Cliffs",
        theme: Theme::Cliffs,
        width: w,
        height: 900.0,
        spawn: (220.0, g),
        solids,
        platforms,
        gates: vec![],
        npcs,
        spawns,
        fixed_enemies: vec![],
        portals,
        signs,
        decos,
        regions: vec![],
        bosses: vec![BossSpawn { kind: Thunderhorn, x: 4200.0, y: top, quest: "storm_summit" }],
        checkpoints: vec![220.0, 1650.0, 3250.0],
        climbs: vec![],
        kill_y: 1100.0,
    }
}

// ------------------------------------------------------------------ class towns

pub struct ShopSpec {
    /// Shopkeeper NPC id == shop id in items::SHOPS.
    pub keeper: &'static str,
    pub map: &'static str,
    pub title: &'static str,
    pub front: DecoKind,
    pub look: Appearance,
}

pub struct TownSpec {
    pub key: &'static str,
    pub name: &'static str,
    pub theme: Theme,
    /// Where this town's road leaves Wayfarer's Crossroads.
    pub crossroads_x: f32,
    pub instructor: &'static str,
    pub instructor_title: &'static str,
    pub instructor_look: Appearance,
    pub inn: &'static str,
    pub inn_look: Appearance,
    pub welcome: &'static str,
    pub shops: [ShopSpec; 3],
    pub flavour: &'static [DecoKind],
}

const fn lk(body: u8, hair: u8, hc: u8, face: u8, top: u8, td: u8, bottom: u8, bd: u8, shoes: u8, skin: u8) -> Appearance {
    Appearance { body, hair, hair_color: hc, face, top: top + 1, top_dye: td, bottom: bottom + 1, bottom_dye: bd, shoes, skin }
}

const fn shops(prefix: [&'static str; 3], maps: [&'static str; 3], titles: [&'static str; 3], looks: [Appearance; 3]) -> [ShopSpec; 3] {
    [
        ShopSpec { keeper: prefix[0], map: maps[0], title: titles[0], front: DecoKind::StoreGeneral, look: looks[0] },
        ShopSpec { keeper: prefix[1], map: maps[1], title: titles[1], front: DecoKind::StoreWeapons, look: looks[1] },
        ShopSpec { keeper: prefix[2], map: maps[2], title: titles[2], front: DecoKind::StoreArmor, look: looks[2] },
    ]
}

pub const TOWNS: &[TownSpec] = &[
    TownSpec {
        key: "ironhold",
        name: "Ironhold",
        theme: Theme::Town,
        crossroads_x: 900.0,
        instructor: "instructor_ironhold",
        instructor_title: "Vanguard Instructor",
        instructor_look: lk(0, 4, 0, 4, 4, 1, 4, 1, 3, 4),
        inn: "inn_ironhold",
        inn_look: lk(1, 7, 2, 0, 0, 2, 2, 2, 5, 1),
        welcome: "Ironhold - the fortress town. Vanguards train in the Barracks Hall at the east end.",
        shops: shops(
            ["ironhold_general", "ironhold_weapons", "ironhold_armor"],
            ["ironhold_general_in", "ironhold_weapons_in", "ironhold_armor_in"],
            ["Grocer", "Weaponsmith", "Armourer"],
            [lk(1, 5, 4, 2, 0, 5, 2, 5, 4, 2), lk(0, 6, 3, 4, 4, 4, 3, 4, 3, 6), lk(1, 1, 0, 1, 4, 8, 3, 8, 3, 3)],
        ),
        flavour: &[DecoKind::Banner, DecoKind::Crate, DecoKind::Barrel, DecoKind::Lamp],
    },
    TownSpec {
        key: "shadowmere",
        name: "Shadowmere",
        theme: Theme::Woods,
        crossroads_x: 1650.0,
        instructor: "instructor_shadowmere",
        instructor_title: "Duskblade Instructor",
        instructor_look: lk(1, 6, 0, 3, 5, 9, 5, 9, 5, 6),
        inn: "inn_shadowmere",
        inn_look: lk(1, 3, 6, 1, 3, 2, 2, 2, 4, 0),
        welcome: "Shadowmere, the town that never sees the sun. Duskblades gather in the Hall of Whispers.",
        shops: shops(
            ["shadowmere_general", "shadowmere_weapons", "shadowmere_armor"],
            ["shadowmere_general_in", "shadowmere_weapons_in", "shadowmere_armor_in"],
            ["Apothecary", "Blade Dealer", "Tailor"],
            [lk(0, 7, 8, 0, 3, 7, 4, 7, 4, 4), lk(0, 0, 0, 3, 5, 9, 5, 9, 5, 7), lk(1, 2, 5, 2, 6, 6, 2, 6, 4, 1)],
        ),
        flavour: &[DecoKind::LanternPost, DecoKind::DeadTree, DecoKind::Gravestone, DecoKind::Mushrooms],
    },
    TownSpec {
        key: "tempest_monastery",
        name: "Tempest Monastery",
        theme: Theme::Cliffs,
        crossroads_x: 2400.0,
        instructor: "instructor_tempest",
        instructor_title: "Stormfist Instructor",
        instructor_look: lk(0, 8, 9, 4, 4, 2, 5, 2, 4, 5),
        inn: "inn_tempest",
        inn_look: lk(0, 8, 9, 0, 4, 6, 5, 6, 4, 3),
        welcome: "Tempest Monastery, high above the clouds. Stormfists train in the Hall of Thunder.",
        shops: shops(
            ["tempest_general", "tempest_weapons", "tempest_armor"],
            ["tempest_general_in", "tempest_weapons_in", "tempest_armor_in"],
            ["Pantry Monk", "Gauntlet Keeper", "Robe Keeper"],
            [lk(0, 8, 9, 0, 4, 3, 5, 3, 4, 2), lk(1, 2, 0, 2, 3, 2, 5, 2, 1, 5), lk(0, 8, 9, 1, 4, 5, 5, 5, 4, 7)],
        ),
        flavour: &[DecoKind::Pine, DecoKind::Rock, DecoKind::Banner, DecoKind::Crystal],
    },
    TownSpec {
        key: "leafwhisper",
        name: "Leafwhisper",
        theme: Theme::Glade,
        crossroads_x: 3150.0,
        instructor: "instructor_leafwhisper",
        instructor_title: "Windrunner Instructor",
        instructor_look: lk(1, 0, 3, 1, 1, 0, 0, 0, 5, 2),
        inn: "inn_leafwhisper",
        inn_look: lk(1, 9, 5, 0, 3, 0, 3, 0, 2, 6),
        welcome: "Leafwhisper, the canopy village. Windrunners train at the Archery Lodge.",
        shops: shops(
            ["leafwhisper_general", "leafwhisper_weapons", "leafwhisper_armor"],
            ["leafwhisper_general_in", "leafwhisper_weapons_in", "leafwhisper_armor_in"],
            ["Forager", "Fletcher", "Outfitter"],
            [lk(0, 9, 3, 1, 0, 0, 1, 0, 2, 4), lk(1, 4, 6, 2, 1, 1, 0, 1, 5, 3), lk(0, 3, 7, 0, 2, 0, 1, 0, 0, 5)],
        ),
        flavour: &[DecoKind::TreeTall, DecoKind::TreeRound, DecoKind::Flowers, DecoKind::Bush],
    },
    TownSpec {
        key: "starfall_spire",
        name: "Starfall Spire",
        theme: Theme::Arcane,
        crossroads_x: 3900.0,
        instructor: "instructor_starfall",
        instructor_title: "Aethermancer Instructor",
        instructor_look: lk(1, 1, 8, 1, 3, 4, 3, 4, 4, 1),
        inn: "inn_starfall",
        inn_look: lk(1, 7, 7, 0, 3, 7, 3, 7, 4, 4),
        welcome: "Starfall Spire, where the sky comes down to meet you. Aethermancers study in the Observatory.",
        shops: shops(
            ["starfall_general", "starfall_weapons", "starfall_armor"],
            ["starfall_general_in", "starfall_weapons_in", "starfall_armor_in"],
            ["Alchemist", "Staff Curator", "Glyph Weaver"],
            [lk(1, 5, 9, 2, 3, 6, 3, 6, 4, 2), lk(0, 3, 8, 1, 3, 1, 3, 1, 4, 6), lk(1, 8, 6, 2, 3, 7, 2, 7, 4, 0)],
        ),
        flavour: &[DecoKind::Crystal, DecoKind::LanternPost, DecoKind::Crystal, DecoKind::Flowers],
    },
];

const TOWN_W: f32 = 3800.0;
const SHOP_X: [f32; 3] = [760.0, 1420.0, 2080.0];
const INN_X: f32 = 2720.0;
const HALL_X: f32 = 3360.0;
const INTERIOR_W: f32 = 1100.0;

fn class_town(t: &'static TownSpec) -> Map {
    use DecoKind::*;
    let g = GROUND;
    let w = TOWN_W;
    let solids = vec![ground(0.0, w, g), Aabb::new(-40.0, 0.0, 40.0, 900.0), Aabb::new(w, 0.0, 40.0, 900.0)];
    let platforms = vec![plat(SHOP_X[0] - 120.0, g - 180.0, 240.0), plat(SHOP_X[2] - 120.0, g - 180.0, 240.0), plat(HALL_X - 140.0, g - 220.0, 280.0)];
    let npcs = vec![
        npc(t.instructor, crate::quests::npc_display_name(t.instructor), t.instructor_title, HALL_X - 170.0, g, t.instructor_look, true),
        npc(t.inn, crate::quests::npc_display_name(t.inn), "Innkeeper - rest & heal", INN_X - 150.0, g, t.inn_look, false),
    ];
    let mut npcs = npcs;
    npcs.push(npc(waystone_id(t.key), "Waystone", "Fast travel", 480.0, g, Appearance::default(), false));
    let road = wild(town_road(t.key)).expect("road");
    let hunt = wild(town_hunt(t.key)).expect("hunting ground");
    let mut portals = vec![
        portal(40.0, g, road.key, road.width - 200.0, road_back_label(t.key), None, ""),
        portal(TOWN_W - 130.0, g, hunt.key, 220.0, hunt.label, None, ""),
    ];
    let mut decos = vec![];
    let mut signs = vec![
        Sign { x: 260.0, y: g, text: t.welcome },
        Sign { x: TOWN_W - 260.0, y: g, text: hunt.sign },
    ];
    for (i, sp) in t.shops.iter().enumerate() {
        let x = SHOP_X[i];
        decos.push(deco(sp.front, x, g, 1.0, 0));
        portals.push(portal(x - 45.0, g, sp.map, 200.0, store_label(sp.front), None, ""));
        signs.push(Sign { x: x + 150.0, y: g, text: store_sign(sp.front) });
    }
    decos.push(deco(Inn, INN_X, g, 1.0, 0));
    decos.push(deco(ClassHall, HALL_X, g, 1.0, 0));
    signs.push(Sign { x: HALL_X + 200.0, y: g, text: "The class hall. Novices of level 10 and above may change class here - talk to the instructor." });
    // Flavour props between buildings.
    let gaps = [420.0, 1090.0, 1750.0, 2400.0, 3040.0, 3640.0];
    for (i, &x) in gaps.iter().enumerate() {
        let k = t.flavour[i % t.flavour.len()];
        decos.push(deco(k, x, g, 1.0, if matches!(k, Flowers | Bush | Mushrooms) { 2 } else { 1 }));
    }
    for i in 0..8 {
        decos.push(deco(if t.theme == Theme::Woods { LanternPost } else { Lamp }, 560.0 + i as f32 * 430.0, g, 1.0, 1));
    }
    Map {
        key: t.key,
        name: t.name,
        theme: t.theme,
        width: w,
        height: 900.0,
        spawn: (220.0, g),
        solids,
        platforms,
        gates: vec![],
        npcs,
        spawns: vec![],
        fixed_enemies: vec![],
        portals,
        signs,
        decos,
        regions: vec![],
        bosses: vec![],
        checkpoints: vec![],
        climbs: vec![],
        kill_y: 1100.0,
    }
}

fn store_label(k: DecoKind) -> &'static str {
    match k {
        DecoKind::StoreGeneral => "Enter: General Store",
        DecoKind::StoreWeapons => "Enter: Weapon Shop",
        _ => "Enter: Armor Shop",
    }
}

fn store_sign(k: DecoKind) -> &'static str {
    match k {
        DecoKind::StoreGeneral => "General Store: potions, ethers and tonics. Press {Up} at the door to go inside.",
        DecoKind::StoreWeapons => "Weapon Shop: class weapons. Press {Up} at the door to go inside.",
        _ => "Armor Shop: armour, boots and accessories. Press {Up} at the door to go inside.",
    }
}

fn shop_interior(t: &'static TownSpec, i: usize) -> Map {
    use DecoKind::*;
    let sp = &t.shops[i];
    let g = GROUND;
    let w = INTERIOR_W;
    let solids = vec![ground(0.0, w, g), Aabb::new(-40.0, 0.0, 40.0, 900.0), Aabb::new(w, 0.0, 40.0, 900.0)];
    let keeper_x = 800.0;
    let shop = crate::items::shop(sp.keeper);
    let npcs = vec![npc(sp.keeper, crate::quests::npc_display_name(sp.keeper), sp.title, keeper_x, g, sp.look, true)];
    let portals = vec![portal(40.0, g, t.key, SHOP_X[i], "Exit", None, "")];
    let goods = match sp.front {
        StoreGeneral => [Shelf, Barrel, Shelf],
        StoreWeapons => [WeaponRack, Barrel, WeaponRack],
        _ => [ArmorStand, Shelf, ArmorStand],
    };
    let decos = vec![
        deco(Window, 300.0, g - 150.0, 1.0, 0),
        deco(Window, 620.0, g - 150.0, 1.0, 0),
        deco(WallTorch, 460.0, g - 170.0, 1.0, 0),
        deco(WallTorch, 960.0, g - 170.0, 1.0, 0),
        deco(Fireplace, 1010.0, g, 1.0, 0),
        deco(goods[0], 240.0, g, 1.0, 0),
        deco(goods[1], 470.0, g, 1.0, 1),
        deco(goods[2], 640.0, g, 1.0, 0),
        deco(Shelf, keeper_x + 60.0, g, 1.1, 0),
        deco(Rug, 520.0, g, 1.0, 1),
        deco(Counter, keeper_x, g, 1.0, 2),
    ];
    let signs = vec![Sign { x: 600.0, y: g, text: "Talk to the shopkeeper with {Interact} to browse. You can sell your own items too." }];
    Map {
        key: sp.map,
        name: shop.map(|s| s.name).unwrap_or("Shop"),
        theme: Theme::Interior,
        width: w,
        height: 900.0,
        spawn: (200.0, g),
        solids,
        platforms: vec![],
        gates: vec![],
        npcs,
        spawns: vec![],
        fixed_enemies: vec![],
        portals,
        signs,
        decos,
        regions: vec![],
        bosses: vec![],
        checkpoints: vec![],
        climbs: vec![],
        kill_y: 1100.0,
    }
}

/// The hub between Breezehollow and the five class towns.
fn crossroads() -> Map {
    use DecoKind::*;
    let g = GROUND;
    let w = 4600.0;
    let solids = vec![ground(0.0, w, g), Aabb::new(-40.0, 0.0, 40.0, 900.0), Aabb::new(w, 0.0, 40.0, 900.0)];
    let platforms = vec![plat(1250.0, g - 130.0, 180.0), plat(2780.0, g - 130.0, 180.0)];
    let npcs = vec![npc("wayfarer_jory", "Jory", "Wayfarer", 480.0, g, look(0, 1, 5, 1, 0, 3, 0, 3, 5, 3), false)];
    let mut portals = vec![portal(40.0, g, "breezehollow", 3745.0, "To Breezehollow", None, "")];
    let mut signs = vec![Sign { x: 300.0, y: g, text: "Wayfarer's Crossroads. Five roads lead to the five class towns - each has shops, an inn and a class instructor." }];
    let mut decos = vec![deco(Campfire, 640.0, g, 1.0, 1), deco(Tent, 760.0, g, 1.0, 0)];
    for t in TOWNS {
        portals.push(portal(t.crossroads_x, g, town_road(t.key), 220.0, road_label(t.key), None, ""));
        decos.push(deco(Signpost, t.crossroads_x - 40.0, g, 1.0, 1));
        signs.push(Sign { x: t.crossroads_x + 160.0, y: g, text: road_sign(t.key) });
    }
    let props = [(TreeRound, 1100.0), (Haystack, 1450.0), (Rock, 2000.0), (Pine, 2650.0), (Flowers, 2900.0), (TreeTall, 3500.0), (Crystal, 4250.0), (Bush, 4450.0)];
    for (k, x) in props {
        decos.push(deco(k, x, g, 1.0, if matches!(k, Flowers | Bush) { 2 } else { 0 }));
    }
    Map {
        key: "wayfarers_crossroads",
        name: "Wayfarer's Crossroads",
        theme: Theme::Fields,
        width: w,
        height: 900.0,
        spawn: (220.0, g),
        solids,
        platforms,
        gates: vec![],
        npcs,
        spawns: vec![],
        fixed_enemies: vec![],
        portals,
        signs,
        decos,
        regions: vec![],
        bosses: vec![],
        checkpoints: vec![],
        climbs: vec![],
        kill_y: 1100.0,
    }
}

fn road_label(town: &str) -> &'static str {
    match town {
        "ironhold" => "Road to Ironhold (Vanguard)",
        "shadowmere" => "Road to Shadowmere (Duskblade)",
        "tempest_monastery" => "Road to Tempest Monastery (Stormfist)",
        "leafwhisper" => "Road to Leafwhisper (Windrunner)",
        _ => "Road to Starfall Spire (Aethermancer)",
    }
}

fn road_sign(town: &str) -> &'static str {
    match town {
        "ironhold" => "Ironhold: home of the Vanguard. Sword, shield and stone walls.",
        "shadowmere" => "Shadowmere: home of the Duskblade. Mind your purse.",
        "tempest_monastery" => "Tempest Monastery: home of the Stormfist. Mind the stairs.",
        "leafwhisper" => "Leafwhisper: home of the Windrunner. Up in the trees.",
        _ => "Starfall Spire: home of the Aethermancer. Where stars fall.",
    }
}

/// Extra climbable shelves added to the original zones.
fn vertical(mut m: Map) -> Map {
    let g = GROUND;
    let (shelf, c, z) = match m.key {
        "whistling_fields" => (Aabb::new(1720.0, g - 360.0, 420.0, 40.0), climb(ClimbKind::Rope, 1760.0, g - 360.0, g - 120.0), zone(EnemyKind::Puffwool, 1820.0, 2080.0, g - 360.0, 2, 8.0)),
        "thornvale_woods" => (Aabb::new(2350.0, g - 300.0, 380.0, 40.0), climb(ClimbKind::Vine, 2390.0, g - 300.0, g - 60.0), zone(EnemyKind::Gravecap, 2440.0, 2680.0, g - 300.0, 2, 8.0)),
        "windcrag_cliffs" => (Aabb::new(380.0, g - 230.0, 360.0, 40.0), climb(ClimbKind::Ladder, 420.0, g - 230.0, g), zone(EnemyKind::CragCrab, 480.0, 700.0, g - 230.0, 1, 9.0)),
        _ => return m,
    };
    m.solids.push(shelf);
    m.climbs.push(c);
    m.spawns.push(z);
    m
}

// ------------------------------------------------------------------ roads & hunting grounds

/// A wild area generated from a few parameters: stepped ground, rock shelves
/// reached by ladders/ropes/vines, and a high shelf with the toughest monsters.
pub struct WildSpec {
    pub key: &'static str,
    pub name: &'static str,
    pub theme: Theme,
    pub width: f32,
    /// West exit (map, x on arrival, label) and optional east exit.
    pub west: (&'static str, f32, &'static str),
    pub east: Option<(&'static str, f32, &'static str)>,
    pub ground: [EnemyKind; 2],
    pub shelf: EnemyKind,
    pub high: EnemyKind,
    pub levels: (u32, u32),
    pub seed: u32,
    /// Label for the portal leading *into* this area from its town.
    pub label: &'static str,
    pub sign: &'static str,
}

use EnemyKind as E;

pub const WILDS: &[WildSpec] = &[
    WildSpec { key: "ironhold_road", name: "Ironhold Road", theme: Theme::Fields, width: 4200.0, west: ("wayfarers_crossroads", 945.0, "To Wayfarer's Crossroads"), east: Some(("ironhold", 220.0, "To Ironhold")), ground: [E::Rockmoss, E::Ironbeak], shelf: E::Ironbeak, high: E::Rockmoss, levels: (10, 12), seed: 1, label: "", sign: "" },
    WildSpec { key: "shadowmere_road", name: "Duskwood Path", theme: Theme::Woods, width: 4200.0, west: ("wayfarers_crossroads", 1695.0, "To Wayfarer's Crossroads"), east: Some(("shadowmere", 220.0, "To Shadowmere")), ground: [E::Shadecap, E::Shadecap], shelf: E::Gloomwisp, high: E::Gloomwisp, levels: (11, 13), seed: 2, label: "", sign: "" },
    WildSpec { key: "tempest_road", name: "Monastery Steps", theme: Theme::Cliffs, width: 4200.0, west: ("wayfarers_crossroads", 2445.0, "To Wayfarer's Crossroads"), east: Some(("tempest_monastery", 220.0, "To Tempest Monastery")), ground: [E::Stormwool, E::Stormwool], shelf: E::Squallsprite, high: E::Squallsprite, levels: (10, 12), seed: 3, label: "", sign: "" },
    WildSpec { key: "leafwhisper_road", name: "Greenway Trail", theme: Theme::Glade, width: 4200.0, west: ("wayfarers_crossroads", 3195.0, "To Wayfarer's Crossroads"), east: Some(("leafwhisper", 220.0, "To Leafwhisper")), ground: [E::Thornling, E::Briarbeak], shelf: E::Briarbeak, high: E::Thornling, levels: (10, 12), seed: 4, label: "", sign: "" },
    WildSpec { key: "starfall_road", name: "Starlit Causeway", theme: Theme::Arcane, width: 4200.0, west: ("wayfarers_crossroads", 3945.0, "To Wayfarer's Crossroads"), east: Some(("starfall_spire", 220.0, "To Starfall Spire")), ground: [E::Starwool, E::Starwool], shelf: E::Gloomwisp, high: E::Starwool, levels: (12, 13), seed: 5, label: "", sign: "" },
    WildSpec { key: "old_quarry", name: "Old Quarry", theme: Theme::Cliffs, width: 5000.0, west: ("ironhold", 3700.0, "To Ironhold"), east: None, ground: [E::QuarryCrab, E::Ironbeak], shelf: E::QuarryCrab, high: E::QuarryCrab, levels: (17, 20), seed: 6, label: "Hunting Ground: Old Quarry (Lv 17-20)", sign: "East: the Old Quarry hunting ground, Lv 17-20. Crabs the size of carts." },
    WildSpec { key: "mournful_marsh", name: "Mournful Marsh", theme: Theme::Woods, width: 5000.0, west: ("shadowmere", 3700.0, "To Shadowmere"), east: None, ground: [E::MireStump, E::Shadecap], shelf: E::Gloomwisp, high: E::MireStump, levels: (18, 21), seed: 7, label: "Hunting Ground: Mournful Marsh (Lv 18-21)", sign: "East: the Mournful Marsh hunting ground, Lv 18-21. Don't follow the lights." },
    WildSpec { key: "thunder_peaks", name: "Thunder Peaks", theme: Theme::Cliffs, width: 5000.0, west: ("tempest_monastery", 3700.0, "To Tempest Monastery"), east: None, ground: [E::Stormwool, E::Thunderhawk], shelf: E::Thunderhawk, high: E::Thunderhawk, levels: (19, 22), seed: 8, label: "Hunting Ground: Thunder Peaks (Lv 19-22)", sign: "East: Thunder Peaks hunting ground, Lv 19-22. Thunderhawks dive from the clouds." },
    WildSpec { key: "canopy_heights", name: "Canopy Heights", theme: Theme::Glade, width: 5000.0, west: ("leafwhisper", 3700.0, "To Leafwhisper"), east: None, ground: [E::Barkstump, E::Thornling], shelf: E::Briarbeak, high: E::Barkstump, levels: (16, 19), seed: 9, label: "Hunting Ground: Canopy Heights (Lv 16-19)", sign: "East: Canopy Heights hunting ground, Lv 16-19. Climb the vines to the treetops." },
    WildSpec { key: "shattered_observatory", name: "Shattered Observatory", theme: Theme::Arcane, width: 5000.0, west: ("starfall_spire", 3700.0, "To Starfall Spire"), east: None, ground: [E::AstralCrab, E::Starwool], shelf: E::Voidgust, high: E::AstralCrab, levels: (21, 24), seed: 10, label: "Hunting Ground: Shattered Observatory (Lv 21-24)", sign: "East: the Shattered Observatory hunting ground, Lv 21-24. Something fell from the stars." },
];

pub fn wild(key: &str) -> Option<&'static WildSpec> {
    WILDS.iter().find(|w| w.key == key)
}

pub fn town_road(town: &str) -> &'static str {
    match town {
        "ironhold" => "ironhold_road",
        "shadowmere" => "shadowmere_road",
        "tempest_monastery" => "tempest_road",
        "leafwhisper" => "leafwhisper_road",
        _ => "starfall_road",
    }
}

pub fn town_hunt(town: &str) -> &'static str {
    match town {
        "ironhold" => "old_quarry",
        "shadowmere" => "mournful_marsh",
        "tempest_monastery" => "thunder_peaks",
        "leafwhisper" => "canopy_heights",
        _ => "shattered_observatory",
    }
}

fn road_back_label(town: &str) -> &'static str {
    match town {
        "ironhold" => "West: Ironhold Road",
        "shadowmere" => "West: Duskwood Path",
        "tempest_monastery" => "West: Monastery Steps",
        "leafwhisper" => "West: Greenway Trail",
        _ => "West: Starlit Causeway",
    }
}

/// NPC id of a town's waystone (fast travel).
pub fn waystone_id(map: &str) -> &'static str {
    match map {
        "breezehollow" => "waystone_breezehollow",
        "ironhold" => "waystone_ironhold",
        "shadowmere" => "waystone_shadowmere",
        "tempest_monastery" => "waystone_tempest_monastery",
        "leafwhisper" => "waystone_leafwhisper",
        _ => "waystone_starfall_spire",
    }
}

/// All maps with a waystone, in travel-menu order.
pub const WAYSTONES: &[&str] = &["breezehollow", "ironhold", "shadowmere", "tempest_monastery", "leafwhisper", "starfall_spire"];

fn climb_kind(theme: Theme) -> ClimbKind {
    match theme {
        Theme::Glade | Theme::Woods => ClimbKind::Vine,
        Theme::Fields | Theme::Arcane => ClimbKind::Rope,
        _ => ClimbKind::Ladder,
    }
}

fn wild_map(w: &'static WildSpec) -> Map {
    use DecoKind::*;
    let g = GROUND;
    let width = w.width;
    let n = 6usize;
    let sec = width / n as f32;
    // Stepped ground (small steps you can walk/jump up).
    let patterns: [[f32; 6]; 3] = [[0.0, -50.0, -50.0, 0.0, -40.0, 0.0], [0.0, 0.0, -60.0, -60.0, -20.0, 0.0], [0.0, -40.0, 0.0, -50.0, -50.0, 0.0]];
    let pat = patterns[(w.seed % 3) as usize];
    let tops: Vec<f32> = pat.iter().map(|o| g + o).collect();
    let mut solids: Vec<Aabb> = (0..n).map(|i| ground(i as f32 * sec, (i + 1) as f32 * sec, tops[i])).collect();
    solids.push(Aabb::new(-40.0, 0.0, 40.0, 900.0));
    solids.push(Aabb::new(width, 0.0, 40.0, 900.0));
    let kind = climb_kind(w.theme);
    let mut climbs = vec![];
    let mut spawns = vec![];
    let mut platforms = vec![];
    // Ground spawns in the middle sections.
    for i in 1..n - 1 {
        let x0 = i as f32 * sec + 60.0;
        let x1 = (i + 1) as f32 * sec - 60.0;
        spawns.push(zone(w.ground[i % 2], x0, x1, tops[i], 3, 6.0));
    }
    // Rock shelves above sections 1, 3 and 4, each with a climb up from the ground.
    let shelf_secs = [1usize, 3, 4];
    let mut shelves = vec![];
    for (k, &i) in shelf_secs.iter().enumerate() {
        let x0 = i as f32 * sec + 80.0;
        let sw = sec - 160.0;
        let top = tops[i] - 220.0;
        let shelf = Aabb::new(x0, top, sw, 40.0);
        solids.push(shelf.clone());
        let cx = if k % 2 == 0 { x0 + 60.0 } else { x0 + sw - 60.0 };
        climbs.push(climb(kind, cx, top, tops[i]));
        spawns.push(zone(w.shelf, x0 + 120.0, x0 + sw - 120.0, top, 2, 7.0));
        shelves.push((x0, sw, top));
    }
    // A high shelf above the middle one, reached by a long climb from it.
    let (mx0, msw, mtop) = shelves[1];
    let hx0 = mx0 + msw * 0.25;
    let hw = msw * 0.6;
    let htop = mtop - 210.0;
    solids.push(Aabb::new(hx0, htop, hw, 36.0));
    climbs.push(climb(if kind == ClimbKind::Ladder { ClimbKind::Rope } else { kind }, hx0 + 40.0, htop, mtop));
    spawns.push(zone(w.high, hx0 + 80.0, hx0 + hw - 60.0, htop, 2, 9.0));
    // Wooden platforms to hop between the shelves.
    platforms.push(plat(2.0 * sec + sec * 0.35, tops[2] - 120.0, 160.0));
    platforms.push(plat(5.0 * sec + sec * 0.2, tops[5] - 130.0, 180.0));

    let mut portals = vec![portal(40.0, tops[0], w.west.0, w.west.1, w.west.2, None, "")];
    if let Some((m, x, l)) = w.east {
        portals.push(portal(width - 130.0, tops[n - 1], m, x, l, None, ""));
    }
    let signs = vec![Sign {
        x: 300.0,
        y: tops[0],
        text: match w.east {
            Some(_) => "Road between the Crossroads and town. Climb up with {Up} at a ladder, rope or vine - the shelves hold tougher monsters.",
            None => "Hunting ground. Climb with {Up} / {Down}. The highest shelf holds the strongest monsters - and the best loot.",
        },
    }];
    let flora: &[DecoKind] = match w.theme {
        Theme::Woods => &[DeadTree, Gravestone, Mushrooms, LanternPost],
        Theme::Cliffs => &[Pine, Rock, Crystal, Pine],
        Theme::Glade => &[TreeTall, Bush, Flowers, TreeRound],
        Theme::Arcane => &[Crystal, Crystal, LanternPost, Rock],
        _ => &[TreeRound, Haystack, Fence, Rock],
    };
    let mut decos = vec![];
    for i in 0..n {
        for k in 0..2 {
            let kind = flora[(i * 2 + k + w.seed as usize) % flora.len()];
            let x = i as f32 * sec + 120.0 + k as f32 * sec * 0.45;
            let layer = if matches!(kind, Flowers | Bush | Mushrooms) { 2 } else if k == 0 { 0 } else { 1 };
            decos.push(deco(kind, x, tops[i], 1.0, layer));
        }
    }
    Map {
        key: w.key,
        name: w.name,
        theme: w.theme,
        width,
        height: 900.0,
        spawn: (220.0, tops[0]),
        solids,
        platforms,
        gates: vec![],
        npcs: vec![],
        spawns,
        fixed_enemies: vec![],
        portals,
        signs,
        decos,
        regions: vec![],
        bosses: vec![],
        checkpoints: vec![220.0, sec * 3.0],
        climbs,
        kill_y: 1100.0,
    }
}

// ------------------------------------------------------------------ world map

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    Town,
    Field,
    Hunting,
}

/// A location on the world map. `pos` is in a 960x560 map space.
pub struct WorldNode {
    pub key: &'static str,
    pub pos: (f32, f32),
    pub levels: Option<(u32, u32)>,
    pub kind: NodeKind,
}

pub const WORLD: &[WorldNode] = &[
    WorldNode { key: "dawnwind_glade", pos: (110.0, 500.0), levels: Some((1, 7)), kind: NodeKind::Field },
    WorldNode { key: "breezehollow", pos: (310.0, 500.0), levels: None, kind: NodeKind::Town },
    WorldNode { key: "whistling_fields", pos: (510.0, 500.0), levels: Some((5, 8)), kind: NodeKind::Field },
    WorldNode { key: "thornvale_woods", pos: (700.0, 500.0), levels: Some((9, 12)), kind: NodeKind::Field },
    WorldNode { key: "windcrag_cliffs", pos: (880.0, 500.0), levels: Some((12, 16)), kind: NodeKind::Field },
    WorldNode { key: "wayfarers_crossroads", pos: (500.0, 390.0), levels: None, kind: NodeKind::Town },
    WorldNode { key: "ironhold_road", pos: (130.0, 290.0), levels: Some((10, 12)), kind: NodeKind::Field },
    WorldNode { key: "ironhold", pos: (110.0, 180.0), levels: None, kind: NodeKind::Town },
    WorldNode { key: "old_quarry", pos: (110.0, 70.0), levels: Some((17, 20)), kind: NodeKind::Hunting },
    WorldNode { key: "shadowmere_road", pos: (310.0, 290.0), levels: Some((11, 13)), kind: NodeKind::Field },
    WorldNode { key: "shadowmere", pos: (300.0, 180.0), levels: None, kind: NodeKind::Town },
    WorldNode { key: "mournful_marsh", pos: (300.0, 70.0), levels: Some((18, 21)), kind: NodeKind::Hunting },
    WorldNode { key: "tempest_road", pos: (500.0, 290.0), levels: Some((10, 12)), kind: NodeKind::Field },
    WorldNode { key: "tempest_monastery", pos: (500.0, 180.0), levels: None, kind: NodeKind::Town },
    WorldNode { key: "thunder_peaks", pos: (500.0, 70.0), levels: Some((19, 22)), kind: NodeKind::Hunting },
    WorldNode { key: "leafwhisper_road", pos: (690.0, 290.0), levels: Some((10, 12)), kind: NodeKind::Field },
    WorldNode { key: "leafwhisper", pos: (700.0, 180.0), levels: None, kind: NodeKind::Town },
    WorldNode { key: "canopy_heights", pos: (700.0, 70.0), levels: Some((16, 19)), kind: NodeKind::Hunting },
    WorldNode { key: "starfall_road", pos: (870.0, 290.0), levels: Some((12, 13)), kind: NodeKind::Field },
    WorldNode { key: "starfall_spire", pos: (880.0, 180.0), levels: None, kind: NodeKind::Town },
    WorldNode { key: "shattered_observatory", pos: (870.0, 70.0), levels: Some((21, 24)), kind: NodeKind::Hunting },
];

/// The world-map node for a map (shop interiors belong to their town).
pub fn world_node_for(map: &str) -> Option<&'static WorldNode> {
    if let Some(n) = WORLD.iter().find(|n| n.key == map) {
        return Some(n);
    }
    let town = TOWNS.iter().find(|t| t.shops.iter().any(|s| s.map == map))?;
    WORLD.iter().find(|n| n.key == town.key)
}

/// Monsters found on a map (spawns, fixed and quest bosses), without duplicates.
pub fn map_monsters(m: &Map) -> Vec<EnemyKind> {
    let mut v: Vec<EnemyKind> = vec![];
    let all = m.spawns.iter().map(|z| z.kind).chain(m.fixed_enemies.iter().map(|f| f.0)).chain(m.bosses.iter().map(|b| b.kind));
    for k in all {
        if !v.contains(&k) && k != EnemyKind::TrainingDummy {
            v.push(k);
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_load_and_link() {
        for k in all_map_keys() {
            let m = load_map(k).expect(k);
            for p in &m.portals {
                let t = load_map(p.target_map).unwrap_or_else(|| panic!("{} portal to unknown map {}", k, p.target_map));
                assert!(p.target_x > 0.0 && p.target_x < t.width, "{} -> {} lands outside", k, p.target_map);
            }
            for n in &m.npcs {
                let on_ground = m.solids.iter().any(|s| n.x >= s.left() && n.x <= s.right() && (s.top() - n.y).abs() < 0.5);
                assert!(on_ground, "{} NPC {} floats", k, n.id);
            }
            for z in &m.spawns {
                let ok = m.solids.iter().any(|s| z.x0 >= s.left() - 1.0 && z.x1 <= s.right() + 1.0 && (s.top() - z.y).abs() < 0.5);
                assert!(ok, "{} spawn zone {:?} {}..{} not on one ground block", k, z.kind, z.x0, z.x1);
            }
            for p in &m.portals {
                let floor = p.area.bottom();
                let on_ground = m.solids.iter().any(|s| p.area.center().x >= s.left() && p.area.center().x <= s.right() && (s.top() - floor).abs() < 0.5);
                assert!(on_ground, "{} portal {} floats", k, p.label);
            }
            for c in &m.climbs {
                let top_ok = m.solids.iter().chain(m.platforms.iter()).any(|s| c.x >= s.left() && c.x <= s.right() && (s.top() - c.top).abs() < 0.5);
                let bottom_ok = m.solids.iter().chain(m.platforms.iter()).any(|s| c.x >= s.left() && c.x <= s.right() && (s.top() - c.bottom).abs() < 0.5);
                assert!(top_ok && bottom_ok && c.bottom > c.top + 60.0, "{} climb at {} doesn't connect two surfaces", k, c.x);
            }
            assert_eq!(m.key, k);
        }
    }

    #[test]
    fn world_map_covers_every_outdoor_map() {
        for k in all_map_keys() {
            let m = load_map(k).unwrap();
            assert!(world_node_for(k).is_some(), "{} missing from the world map", k);
            if m.theme != Theme::Interior {
                assert!(WORLD.iter().any(|n| n.key == k), "{} should have its own node", k);
            }
        }
        for w in WAYSTONES {
            let m = load_map(w).unwrap();
            assert!(m.npcs.iter().any(|n| n.id == waystone_id(w)), "{} has no waystone", w);
        }
    }

    #[test]
    fn every_class_town_has_its_services() {
        use crate::classes::ALL_CLASSES;
        for &c in ALL_CLASSES {
            let town = load_map(c.info().town).unwrap_or_else(|| panic!("no town for {:?}", c));
            let instructor = town.npcs.iter().find(|n| crate::quests::instructor_class(n.id) == Some(c));
            assert!(instructor.is_some(), "{} has no {:?} instructor", town.key, c);
            assert!(town.npcs.iter().any(|n| n.id.starts_with("inn_")), "{} has no inn", town.key);
            let mut kinds = Vec::new();
            for p in &town.portals {
                let inner = load_map(p.target_map).unwrap();
                if inner.theme == Theme::Interior {
                    let keeper = inner.npcs.iter().find_map(|n| crate::quests::npc_shop(n.id)).expect("interior without shopkeeper");
                    kinds.push(crate::items::shop(keeper).unwrap().kind);
                    assert!(inner.portals.iter().any(|q| q.target_map == town.key), "can't leave {}", inner.key);
                }
            }
            for k in [crate::items::ShopKind::General, crate::items::ShopKind::Weapons, crate::items::ShopKind::Armor] {
                assert!(kinds.contains(&k), "{} is missing a {:?} store", town.key, k);
            }
        }
    }
}
