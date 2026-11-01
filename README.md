# Galebound Online

A side-scrolling action MMO prototype in Rust: classic bright side-scroller MMO
look and progression, with platform-fighter style combat (directional light /
heavy attacks, dodges with invulnerability, double jumps, air recoveries,
knockback and hit-stop).

**Version 0.8.1 – multiplayer.** Every update is listed in [PATCH_NOTES.md](PATCH_NOTES.md).

## Running it

1. Install Rust: https://rustup.rs
2. **Linux only** – install windowing/GL headers once:
   `sudo apt install libx11-dev libxi-dev libgl1-mesa-dev libasound2-dev`
3. Start a **server** (see *Hosting for friends* below) - or join a friend's.
4. Start the **game** in this folder:

```
cargo run --release
```

On the login screen, put the server's address in the **Server address** box
(bottom-left): `localhost` if the server runs on your PC, or the host's
Tailscale address. Then create an account *on that server* and play.

## Windows .exe

Double-click **`build-windows.bat`**. It builds everything (the first time
takes a few minutes) and opens a **`GaleBound_Windows`** folder containing
`Galebound.exe`, `GaleboundServer.exe` and `Start Server.bat`. Give friends
`Galebound.exe`; it runs on its own with nothing else to install.

If the build says `link.exe` was not found, install the free *Build Tools for
Visual Studio* (https://visualstudio.microsoft.com/visual-cpp-build-tools/),
tick **Desktop development with C++**, and run the script again.

## Hosting for friends (up to 16 players)

The server is a separate program in this folder. It keeps everyone's accounts
and characters, runs the monsters on every map, and decides who gets the
loot. Your PC must stay on while people play.

**1. Start the server.** Double-click `start-server.bat` (Windows), or run:

```
cargo run --release -p galebound-server
```

The first start builds it (a few minutes). Leave the window open. Windows
Firewall will ask whether to allow it: allow **private networks**. Type
`list` to see who's online, `say <message>` to message everyone, and
`stop` to shut it down. Options: `--port 7777` (default), `--max-players 16`,
`--data <folder>` (where accounts are saved; default
`%APPDATA%\Galebound\server` on Windows), `--motd "Welcome!"`.

**2. Set up Tailscale (once).** You and every friend:
- install Tailscale from https://tailscale.com/download and sign in;
- you invite friends to your tailnet (admin console -> Users -> Invite), or
  share just your PC with them (Machines -> your PC -> Share).

Your PC's Tailscale address is shown in the Tailscale app, or run
`tailscale ip -4`. It looks like `100.101.102.103`.

**3. Friends connect.** They start the game and type your Tailscale address
in **Server address** (port 7777 is added automatically). You type
`localhost`.

No port forwarding is needed, nothing is exposed to the internet, and it
works even if your internet provider uses CGNAT. If you change the port, add it
to the address: `100.101.102.103:7778`.

**What's shared:** other players appear on your map and minimap with their
name, level, gear, HP and pet. Monsters are the same for everyone: everyone
who hits a monster gets EXP when it dies (split by damage, at least 20% each)
and quest credit. Loot falls for everyone and goes to whoever picks it up
first (you or your pet). Drop items to hand them to a friend. Walk up to a
friend's tombstone and press **Talk** (E / V / Space) to revive them. Press
**Enter** to chat with everyone on the server.

Run the test suite (physics, combat, stats, shops, abilities, class change,
quests, accounts, bots that walk the zones and towns):

```
cargo test --workspace
```

The tests include a real server on a real socket with two clients.

## What's in this build

**Login screen**
- **Server address** box: `localhost` or a host's Tailscale address.
- Register / sign in with e-mail + password (validated, minimum 8 characters).
- Passwords are never stored: each account keeps a random salt and a
  PBKDF2-HMAC-SHA256 hash (120k iterations). 5 wrong attempts = 30 s lockout.
- **Remember me**: ticking it opens a *shared / public computer* warning you
  must confirm. It stores a random token (not your password); signing in
  without it, or choosing "Use a different account", revokes the token.
- Accounts and characters are saved **on the server**. Your own computer only
  keeps the last server address, display settings and any "remember me"
  token, in your user data folder (Windows `%APPDATA%\Galebound`, macOS
  `~/Library/Application Support/Galebound`, Linux `~/.local/share/galebound`;
  override with `GALEBOUND_DATA=<dir>`).

**Novice → class change at level 10.** Every character starts as a
*Novice* in their own Adventurer's Garb (linen shirt, rope belt, satchel) with
a Gnarled Stick; Garrick sells a Wooden Sword and Wooden Club. Take the garb
off in the inventory to show the outfit you designed - you no longer pick a class in the
creator. At level 10, take the North Road from Breezehollow to **Wayfarer's
Crossroads**, walk to a class town and talk to its instructor. Changing class
refunds every ability point so you can rebuild around the new main stats,
gives you the class weapon, unlocks its abilities and hands you a **Skill
Reset Coupon** (bound to you) - use it from your bag any time to refund every
spent ability point again.

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

**Loot on the ground.** Monsters spill gold and loot (materials, potions and,
rarely, gear) onto the ground. Walk over and press `F` to pick up the nearest
drop; hold `F` to keep scooping. Loot stays for 2 minutes and blinks before it
vanishes. Sell materials at any shop. Drag an item out of the bag to drop it
(Shift-drag drops one). The pick-up code also works for pets
(`Sim::pet_collect`), ready for when pets arrive.

**Pets** (`O` for the Pets window). Every main town (Breezehollow and the
five class towns) has a **Pet Shop** keeper next to a little pen. Pets cost
**10,000 gold**; use the pet from your bag to adopt it and it comes out with
you. A pet trots (or flutters) beside you and **automatically picks up loot**
on the ground within its range of you (260, more for a Breeze Fox), straight
into your bag.

| Pet | Lv 1 | Lv 5 | Lv 10 | Lv 15 | Lv 20 |
|---|---|---|---|---|---|
| Breeze Fox | +120 loot range | +10% gold | +5% move speed | +5% crit | +15% gold |
| Puffling | +8% P/M DEF | heal 0.5% HP/s | +60 HP | +12% P/M DEF | +120 HP |
| Sparkwing | +3% crit | +5% attack | 1% Mana/s | +20% crit damage | +8% attack |

Pets level up (to 20) only by eating **Pet Food** (100 gold at any Pet Shop):
each one gives +50 pet EXP and +30 hunger. **Hunger** drains while the pet is
out (full to empty in 30 minutes) and pauses while it's at home. A starving pet
stops looting and its passives switch off until it's fed. Feed from the Pets
window, the bag or the hotbar. You can own up to 6 pets; one is out at a time.

**Defeat.** When your HP hits 0 you fall and become a **tombstone** where you
lay. It stays until you click **Respawn** (back to the last checkpoint with
full HP), read a **Revive Scroll** (get up on the spot), or a friend walks
up and revives you (Talk key at your tombstone; you get up with 30% HP).
Above level 10, a defeat costs **3% of your current EXP**; a Revive Scroll
gives it all back. You stay a tombstone even if you log out.

**Minimap** (top-right). Terrain, platforms and ladders, portals, NPCs (gold =
new quest, blue = turn-in), waystones, other players and you. The header
buttons make it smaller / bigger, switch between the area around you and the
whole map, or hide it; `N` cycles sizes.

**Graphics** (`Esc` → Graphics). Windowed, Windowed Fullscreen or Fullscreen.
The choice is remembered on this computer. The graphics library only offers a
borderless fullscreen window, so Fullscreen and Windowed Fullscreen currently
behave the same.

**World map (`M`).** Shows every area, where you are, recommended levels,
monsters and possible loot, and which waystones you've attuned.

**Waystones.** Breezehollow and every class town have a waystone. Walking
into a town attunes it, and from then on any waystone can take you there
for 25 gold.

**Menu (`Esc`).** A slim side menu with shortcuts to the map and windows. The
game is never paused - it's an MMO - so your character just stands still while
the menu is open.

Inventory (`I`): 24 slots plus Weapon / Armor / Boots / Accessory. Click to
use or equip, right-click to sell while a shop is open, drag out to drop.
**Bound** items (trainee potions, Skill Reset Coupons) can't be sold, traded
or dropped. Mira and Garrick in Breezehollow also run small shops.

Every class has 11 moves: light neutral/side/down, air neutral/side/down, heavy
neutral/side/down, air-heavy (recovery) and air-heavy-down (dive / slam).

**Character creation** (everyone starts as a Novice; the left panel previews
the five classes) - body type (masculine / feminine), 10 hairstyles
(including long locs and short curls) x 10 hair colours, 6 faces, 8 tops and
8 bottoms (each with 10 dyes, or "None" to wear just underwear), 7 shoes
including high-top sneakers, 8 skin tones, live animated preview, 3 slots.

**Tutorial: Dawnwind Glade → Breezehollow**
1. *Elder Wynn* – climb Windstep Rise (teaches double jump). Reward: Novice's
   Wind Charm (handed over by Tamsin).
2. *Tamsin* – dodge 3 times. Reward: Novice's Boots.
3. *Sergeant Brask* – light attacks (reward: Novice's Practice Sword), then
   heavy/signature attacks on dummies.
4. *Quartermaster Hesper* – her supply wagon stands next to the dummies.
   Trainee HP & Mana potions (40% off for your first 10, bound to you), Revive
   Scrolls, and Lv 8 weapons and armour (Ironwood Blade, Ironbound Cudgel,
   Padded Gambeson, Trailblazer Boots). She buys monster loot too.
5. *Ranger Pell* – clear Mosslings (reward: Novice's Tunic), then Burrowbeaks
   (telegraphed lunges).
6. *Warden Ilsa* – defeat the **Bramblehorn** mini-boss (charges, stomps,
   thorn volleys, enrages at 50%) to open the Old Gate. Reward includes a
   Revive Scroll.

Every quest offer shows its rewards (EXP, gold, potions and items with icons)
in the dialog window before you accept.
7. *Mayor Odessa* in Breezehollow, then meet the townsfolk (Mira sells potions
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
| Talk / interact / revive a friend | E | V | Space |
| Potion | Q | A | Delete / End |
| Quest log | Tab | Tab | Tab |
| Inventory / Character / Abilities | I / P / B | I / P / B | I / P / B |
| World map | M | M | M |
| Pick up loot (hold to keep picking) | F | F | F |
| Minimap size | N | N | N |
| Pets window | O | O | O |
| Chat | Enter | Enter | Enter |
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
  sim.rs       120 Hz simulation: player controller, combat, ground loot,
               death / revive, FX events
  enemies.rs   monster definitions and behaviours (hopper, lunger, floater, boss)
  world.rs     maps (incl. class towns + shop interiors), NPCs, spawns, portals
  quests.rs    quest chain, dialogue, progress tracking
  pets.rs      pet species, levels, hunger and passive skills
  auth.rs      accounts, login, lockout, remember-me tokens
  crypto.rs    SHA-256 / HMAC / PBKDF2
  save.rs      save format (plain text, atomic writes)
  net.rs       multiplayer protocol and connections
server/      galebound-server: the dedicated server (accounts, monsters, loot, chat)
src/         the client (macroquad): rendering, UI, input
  online.rs    server address, "remember me" token, animation codes
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
  lootart.rs   gold piles and the tombstone
  petart.rs    pet sprites
  minimap.rs   the adjustable minimap
  display.rs   windowed / fullscreen switching
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

## How multiplayer works

- `server/` (`galebound-server`) uses only the Rust standard library. Each
  player has one TCP connection carrying short text messages
  (`core/src/net.rs`). It runs one 60 Hz loop that owns all the state, and
  sends monster snapshots to each map 15 times a second.
- **The server decides** accounts and saves, monsters (AI, HP, deaths,
  respawns, quest bosses), loot on the ground and who gets it, chat and
  revives. Account files live only on the server, and passwords never leave
  it: clients get a copy of their account with the password data removed.
- **Each game client** moves and animates its own character and does its own
  hit detection, then reports hits. Classic side-scroller MMOs work the same
  way, and it keeps combat instant even with some lag. The server checks every
  hit for range and size, and every pick-up for distance. Inventories and
  quest progress are saved from the client, which is fine for a group of
  friends. Moving those checks onto the server would be the next step for a
  public server.

## A note on the classes

The class lineup follows the generic roles common to side-scrolling MMOs
(warrior, rogue, monk, archer, mage). Names, moves, quests, maps and art are
all original to this project; keep it that way as content grows.
