# Galebound Online

A side-scrolling action MMO prototype in Rust: classic bright side-scroller MMO
look and progression, with platform-fighter style combat (directional light /
heavy attacks, dodges with invulnerability, double jumps, air recoveries,
knockback and hit-stop).

## Running it

1. Install Rust: https://rustup.rs
2. **Linux only** – install windowing/GL headers once:
   `sudo apt install libx11-dev libxi-dev libgl1-mesa-dev libasound2-dev`
3. In this folder:

```
cargo run --release
```

Run the test suite (physics, combat, stats, shops, abilities, class change,
quests, accounts, bots that walk the zones and towns):

```
cargo test --workspace
```

## What's in this build

**Login screen**
- Register / sign in with e-mail + password (validated, minimum 8 characters).
- Passwords are never stored: each account keeps a random salt and a
  PBKDF2-HMAC-SHA256 hash (120k iterations). 5 wrong attempts = 30 s lockout.
- **Remember me**: ticking it opens a *shared / public computer* warning you
  must confirm. It stores a random token (not your password); signing in
  without it, or choosing "Use a different account", revokes the token.
- Saves live in your user data folder
  (Windows `%APPDATA%\Galebound`, macOS `~/Library/Application Support/Galebound`,
  Linux `~/.local/share/galebound`). Override with `GALEBOUND_DATA=<dir>`.

**Novice → class change at level 10.** Every character starts as a
*Novice* in their own Adventurer's Garb (linen shirt, rope belt, satchel) with
a Gnarled Stick; Garrick sells a Wooden Sword and Wooden Club. Take the garb
off in the inventory to show the outfit you designed - you no longer pick a class in the
creator. At level 10, take the North Road from Breezehollow to **Wayfarer's
Crossroads**, walk to a class town and talk to its instructor. Changing class
refunds every ability point so you can rebuild around the new main stats,
gives you the class weapon and unlocks its abilities.

| Class | Role | Weapon | Main stats | Town |
|---|---|---|---|---|
| Vanguard | Frontline warrior / tank | Sword & Shield | STR / DEX | Ironhold |
| Duskblade | Shadow rogue (triple jump) | Twin Daggers | LUK / DEX | Shadowmere |
| Stormfist | Martial artist | Storm Gauntlets | DEX / STR | Tempest Monastery |
| Windrunner | Ranged archer | Longbow | DEX / STR | Leafwhisper |
| Aethermancer | Elemental mage (magic damage) | Runed Staff | INT / LUK | Starfall Spire |

**Stats.** You get 5 ability points (AP) per level to spend on the main stats
**Strength, Dexterity, Intelligence, Luck** (Character window, `P`; "Auto"
spends them the way your class wants). They, your level, gear and buffs give:
**HP, Mana, Physical ATK, Magic ATK, Physical DEF, Magical DEF, Critical
Chance % and Critical Damage %**. Physical DEF only reduces physical hits
(contact, charges); Magical DEF reduces magic (wind orbs, starfire, bolts).
All formulas are in `core/src/stats.rs`.

**Abilities & hotbar.** 8 hotbar slots on keys `1`-`8`. Abilities cost Mana
and have cooldowns; consumables can go on the bar too. Open Abilities (`B`)
and drag an ability onto a slot (or click it to fill the first empty slot);
drag slots to reorder, right-click to clear. New abilities are placed on the
bar automatically when learned.

| Class | Lv | Abilities |
|---|---|---|
| Novice (kept by everyone) | 1 / 3 / 6 | Power Strike, Second Wind (heal), Brace (+P.DEF) |
| Vanguard | 10 / 12 / 15 / 20 | Shield Slam, War Cry, Whirlwind (4 hits, can't be hurt), Iron Bastion |
| Duskblade | 10 / 12 / 15 / 20 | Shadow Flurry (4 hits), Fan of Knives, Nightveil (+crit, +speed), Assassinate |
| Stormfist | 10 / 12 / 15 / 20 | Thunder Palm, Hundred Fists (6 hits), Focus Chi, Typhoon Kick |
| Windrunner | 10 / 12 / 15 / 20 | Piercing Arrow, Multishot, Hawkeye, Storm Arrow |
| Aethermancer | 10 / 12 / 15 / 20 | Arcane Bolt, Cyclone, Aether Ward, Starfall |

**Class towns.** Wayfarer's Crossroads links Breezehollow to five towns, each
in its own style (stone fortress, moonlit Shadowmere, sunset monastery, forest
village, starlit spire). Every town is a safe respite with an innkeeper who
heals you, a class hall with the instructor, and three **enterable shops** -
walk to the door and press Up to go inside, talk to the shopkeeper to trade:

- **General store** - HP/Mana potions, elixirs, tonics (temporary buffs).
- **Weapon shop** - the town class's weapons (Lv 14 and 18 upgrades).
- **Armour shop** - armour, boots and accessories (armour and boots change
  how your character looks).

**Roads and hunting grounds.** Each class town is reached by its own road
from Wayfarer's Crossroads (Lv 10-13), and every town has a hunting ground
past its east gate:

| Town | Road (Lv) | Hunting ground (Lv) |
|---|---|---|
| Ironhold | Ironhold Road (10-12) | Old Quarry (17-20) |
| Shadowmere | Duskwood Path (11-13) | Mournful Marsh (18-21) |
| Tempest Monastery | Monastery Steps (10-12) | Thunder Peaks (19-22) |
| Leafwhisper | Greenway Trail (10-12) | Canopy Heights (16-19) |
| Starfall Spire | Starlit Causeway (12-13) | Shattered Observatory (21-24) |

They have 15 new monsters and are built in layers. **Ladders, ropes and vines**
(`Up`/`Down` to climb, `Jump` to let go) lead to rock shelves with tougher
monsters, and a high shelf holds the strongest ones. The Fields, Woods and
Cliffs have new climbable shelves too.

**Loot.** Monsters drop materials (sell them at any shop), potions and,
rarely, gear. Drops go straight into your bag.

**World map (`M`).** Shows every area, where you are, recommended levels,
monsters and possible loot, and which waystones you've attuned.

**Waystones.** Breezehollow and every class town have a waystone. Walking
into a town attunes it, and from then on any waystone can take you there
for 25 gold.

**Menu (`Esc`).** A slim side menu with shortcuts to the map and windows. The
game is never paused - it's an MMO - so your character just stands still while
the menu is open.

Inventory (`I`): 24 slots plus Weapon / Armor / Boots / Accessory. Click to
use or equip, right-click to sell while a shop is open. Mira and Garrick in
Breezehollow also run small shops.

Every class has 11 moves: light neutral/side/down, air neutral/side/down, heavy
neutral/side/down, air-heavy (recovery) and air-heavy-down (dive / slam).

**Character creation** (everyone starts as a Novice; the left panel previews
the five classes) - body type (masculine / feminine), 10 hairstyles
(including long locs and short curls) x 10 hair colours, 6 faces, 8 tops and
8 bottoms (each with 10 dyes, or "None" to wear just underwear), 7 shoes
including high-top sneakers, 8 skin tones, live animated preview, 3 slots.

**Tutorial: Dawnwind Glade → Breezehollow**
1. *Elder Wynn* – climb Windstep Rise (teaches double jump).
2. *Tamsin* – dodge 3 times.
3. *Sergeant Brask* – light attacks, then heavy/signature attacks on dummies.
4. *Ranger Pell* – clear Mosslings, then Burrowbeaks (telegraphed lunges).
5. *Warden Ilsa* – defeat the **Bramblehorn** mini-boss (charges, stomps,
   thorn volleys, enrages at 50%) to open the Old Gate.
6. *Mayor Odessa* in Breezehollow, then meet the townsfolk (Mira sells potions
   and heals you, Garrick the blacksmith, Aurel the class mentor).

**Zones beyond the town** (23 quests in total, Lv 1 to ~15)

| Zone | Levels | Monsters | Story |
|---|---|---|---|
| Whistling Fields | 5-8 | Puffwool (hopper), Gustling (floats, shoots wind orbs) | Farmer Tilly's flock has gone wild; the wind has soured |
| Thornvale Woods (night) | 9-12 | Gravecap, Lanternwisp (floats, hurls starfire), Rotstump (telegraphed lunge) | Gravekeeper Morrow and Hermit Sable; haunted trees and graves |
| Windcrag Cliffs (sunset) | 12-16 | Crag Crab, Galehawk (screeches, then dive-bombs), **Thunderhorn** boss | Climb to the summit and break the storm |

Monsters use four reusable behaviours (hopper, lunger, floater, boss) defined
as data in `core/src/enemies.rs`, so adding a monster is mostly adding numbers
and art.

Signs along the way teach the controls and always show *your* current keys.

**Control layouts** (change any time: Esc → Controls & Settings)

| Action | WASD | Arrow Keys | Classic MMO |
|---|---|---|---|
| Move / aim | A D W S | Arrows | Arrows |
| Jump | Space | Space | Alt |
| Light attack | J | X | Ctrl |
| Heavy attack | K | C | Shift |
| Dodge | L / Shift | Z | Z |
| Talk / interact | E | V / Enter | Space / Enter |
| Potion | Q | A | Delete / End |
| Quest log | Tab | Tab | Tab |
| Inventory / Character / Abilities | I / P / B | I / P / B | I / P / B |
| World map | M | M | M |
| Climb ladders / ropes / vines | W / S | Up / Down | Up / Down |
| Hotbar | 1-8 | 1-8 | 1-8 |

Options: "Up also jumps", damage numbers, screen shake.

Pacing: gravity is about 2/3 of a typical platform fighter's, so jumps hang
longer and knocked-back monsters arc through the air; run speed and attack
windows are tuned for a calmer MMO feel. All the knobs live in
`core/src/physics.rs` (`GRAVITY`, `PACE_H`, `PACE_V`) and
`core/src/classes.rs` (`ATTACK_TIME`). F1 toggles the
on-screen key hints. Up enters portals. Down + Jump drops through wooden
platforms; holding Down while falling fast-falls.

## Project layout

```
core/        galebound-core: zero-dependency game logic (unit tested)
  physics.rs   AABB collision, gravity, one-way platforms
  classes.rs   classes, all movesets / frame data
  stats.rs     main stats, derived combat stats, ability points
  items.rs     items, inventory/equipment, shop catalogues
  skills.rs    class abilities, hotbar
  sim.rs       120 Hz simulation: player controller, combat, FX events
  enemies.rs   monster definitions and behaviours (hopper, lunger, floater, boss)
  world.rs     maps (incl. class towns + shop interiors), NPCs, spawns, portals
  quests.rs    quest chain, dialogue, progress tracking
  auth.rs      accounts, login, lockout, remember-me tokens
  crypto.rs    SHA-256 / HMAC / PBKDF2
  save.rs      save format (plain text, atomic writes)
src/         the client (macroquad): rendering, UI, input
  gfx.rs       640x360 pixel canvas, pixel-snapped shapes, pixel text
  pixel.rs     CPU pixel canvas: exact shapes, per-part outlines, shading
  charsprite.rs anime pixel characters: rig, poses, animation frames
  art.rs       enemies, props, terrain tiles, parallax backgrounds
  font.rs      hand-made pixel font
  sprites.rs   texture cache
  render.rs    draws the world from the simulation state
  hud.rs       bars, tracker, dialog, quest log, boss bar, buffs
  windows.rs   hotbar, inventory, character, abilities, shop windows
  townart.rs   storefronts, inns, class halls, shop interiors
  icons.rs     item and ability icons
  screens/     login, character select, creation, game
```

## Art style

Everything is anime pixel art, generated in code (no image files needed):

- The game renders to a **640x360 pixel canvas** and scales it up with hard
  pixel edges (2x at 720p, 3x at 1080p, 4x at 1440p).
- **Characters** (`charsprite.rs`) are built on a real skeleton (pelvis,
  spine, head; shoulder, elbow, fist; hip, knee, ankle, foot) with joint
  limits - knees only bend backward, elbows only forward - and feet that stay
  planted on the ground. Poses are keyframed from the reference sprite sheets:
  boxing-guard idle, 8-frame run with a real push-off, punch, high kick
  (chamber, extend, re-chamber), jump tuck, fall, dodge, hurt, defeat and
  more. Bodies wear underwear (boxers, or briefs and a sports bra); every
  piece of clothing is drawn on the bone it covers, so sleeves, trouser legs
  and skirts move with the limbs. Tests check every outfit x weapon x frame.
- **World art** (`art.rs`): cobblestone cliffs, grass lips and tufts, wooden
  platforms, parallax mountains and forests, houses, windmill, props, the
  stone gate, and all enemies with their state frames.
- **Pixel font** (`font.rs`): a hand-made proportional font.

Sprites are generated once and cached as textures (`sprites.rs`). To use an
artist's sprite sheets later, replace the generator for that sprite; the
gameplay code doesn't change.

## Road to the "MMO" part

This build is single-player with local accounts, deliberately structured for
the next step:

- `galebound-core` has no engine or OS dependencies, so the **same simulation
  can run on an authoritative server** (e.g. tokio + WebSockets/QUIC).
- `AccountStore` is the only thing that touches account files; replace its
  methods with HTTPS calls to a login server and the login screen stays as is.
- Next milestones: login server + database, shared maps with other players
  (server-authoritative positions, client-side prediction for your own
  character), chat, parties, then second-tier class advancements.

## A note on the classes

The class lineup follows the generic roles common to side-scrolling MMOs
(warrior, rogue, monk, archer, mage). Names, moves, quests, maps and art are
all original to this project; keep it that way as content grows.
