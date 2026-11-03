# Galebound Online – Patch Notes

Newest first. Version numbers use `major.minor.patch`: each round of new
features raises the minor number (0.5 → 0.6), and fix-only releases raise the
last number (0.6.0 → 0.6.1). Versions before 0.6.0 were numbered afterwards
so the whole history is in one place.

---

## 0.10.2 – Android controller fix
*3 November 2026*

**Fixed**
- **Controllers on Android:** the first Windows APK build used the standard
  miniquad library instead of Galebound's patched copy, so gamepad buttons
  and sticks would not have worked on phones. macroquad is now pinned to the
  version the patch is made for (0.4.14), so every build uses the patch.
- Removed two harmless compiler warnings from the Android build.

---

## 0.10.1 – Build the APK on Windows
*2 November 2026*

**Added**
- `build-android.bat`: builds `Galebound.apk` on Windows 10/11 with Docker
  Desktop (double-click it in the source folder). No game changes, so 0.10.1
  plays with 0.9.x and 0.10.0.

---

## 0.10.0 – Android, touch & controllers
*2 November 2026*

**Added**
- **Android version** (phones and tablets, Android 5.0+, landscape). It
  plays on the same servers as the PC version: Android and PC players see
  each other, party, trade and chat as normal. Build the .apk on Linux with
  `./build-android.sh` (needs Docker; see `android/README.md`).
- **Touch controls**, shown automatically on Android (and on a PC touch
  screen once you touch it):
  - a floating joystick (put your thumb anywhere bottom-left);
  - Jump, Attack, Heavy, Dodge, Talk and Loot buttons;
  - small Menu, Bag, Chat and Quest buttons.
  Windows, the hotbar and menus work by tapping; tap another player for
  their menu (profile, whisper, party, trade...). The controls hide while a
  window is open. Text boxes and chat bring up the phone's keyboard, and the
  phone's Back button works like Esc.
- **Controller support**:
  - Xbox-style pads on Windows (XInput) and Linux (`/dev/input/js*`), and
    Bluetooth / USB pads on Android.
  - Layout: left stick / D-pad move, A jump, X attack, Y heavy, B dodge,
    RB talk, LB pick up, L3 potion, R3 quest log, Start menu, Back / Select
    bag.
  - Hold LT + A / B / X / Y for hotbar 1-4, or RT + A / B / X / Y for 5-8.
  - The pause menu, dialogs and the defeat screen work with the pad
    (D-pad to move, A to choose, B / Start to go back).
- The pause menu now has Friends & Party, Guild and Achievements (handy on
  touch screens and controllers).
- **Linux builds** of the game and server (x86-64; Zorin OS 17 / Ubuntu
  22.04 or newer).

**Technical**
- The project now includes a lightly patched copy of miniquad (the window and
  input library) in `third_party/`. Android drops controller buttons and
  sticks by default; the patch passes them through.
- Android saves and settings go in the app's private storage.

**Note:** no protocol change: 0.9.x and 0.10.0 clients and servers all work
together. The Android build hasn't been run on a phone yet (it can't be built
here); please report anything odd.

---

## 0.9.1 – 1080p rendering
*2 November 2026*

**Added**
- **Resolution** option in Esc > Graphics: 1280 x 720, 1600 x 900 or
  1920 x 1080. The game now draws at the chosen resolution instead of a
  640 x 360 canvas scaled up. You see the same amount of the world with the
  same layout, but edges, glows, shapes, text and scrolling are much finer,
  and the pixel art stays crisp.
- In a window, the window resizes to the chosen resolution. In Windowed
  Fullscreen or Fullscreen, the picture is scaled to fit your screen.
- Your choice is saved on your computer (`resolution.cfg` next to
  `display.cfg`), and the game opens at that size next time.

**Changed**
- Even at the default 1280 x 720 the game now renders at full window
  resolution (it used to render at 640 x 360 and double it), so movement and
  shapes are smoother than in 0.9.0.

**Note:** 1280 x 720 and 1920 x 1080 are exact multiples of the pixel-art
grid. At 1600 x 900 a few font and sprite pixels are slightly uneven in size.
Clients and servers from 0.9.0 and 0.9.1 work together.

---

## 0.9.0 – Friends, parties & guilds
*1 November 2026*

**Added: social systems** (all kept on the server)
- **Friends list** (`U`): send requests by name (the other player gets a
  popup), accept or decline, see who's online, their level, class and map,
  and get a notice when a friend logs in. Up to 50 friends.
- **Block / ignore list**: blocked players' chat, whispers and invitations
  never reach you, and blocking ends any friendship.
- **Parties** of up to 6: invite, accept, leave, promote a new leader,
  remove members, party chat. The party list shows each member's level,
  class, map and HP. Members on the same map share EXP from every kill.
- **Guilds** (`G`): found one for 5,000 gold; invite players and friends;
  Leader / Officer / Member ranks (promote, demote, hand over leadership,
  remove members, disband); guild chat; an editable guild notice.
  - **Daily guild quest**: a rotating goal the whole guild works on together
    (defeat 120 monsters, 60 on a particular map, 2 quest bosses...).
    Everyone can claim a reward when it's done.
  - **Daily guild chest**: gold and potions for every member, once a day.
  - **Guild levels and buffs**: guild EXP from members' kills, chests and
    quests unlocks five guild-wide buffs (more EXP, attack, Max HP and
    defence, move speed and crit).
  - Guild names show under members' names, WindSlayer style.
- **Right-click a player** for a menu: Whisper, View profile, Invite to
  party, Trade, Invite to guild, Add / Remove friend, Block (plus party and
  guild management for leaders and officers).
- **Player profiles**: level, class, EXP to next level (%), guild,
  achievement score, time played and all 11 equipped items with their stats.
- **Trading** between players: add items and gold to the trade window,
  lock in, then both confirm. Changing an offer unlocks both sides; bound
  items can't be traded; the game checks your bag has room first.
- **Chat tabs**: All, Party, Guild, Whisper and System. System shows EXP
  gained, items and gold looted, quests, achievements and server notices.
  Commands: `/w Name message`, `/r` (reply), `/p`, `/g`, `/a`.
- **Achievements** (`H`): 33 of them, each worth score points and Gale Marks.
- **Exclusive cosmetics**, bought only with Gale Marks: Flower Crown, Cat
  Ears, Starry Halo, Crown of Gales, Angel Wings, Bat Wings and Star Mantle.
  They show over your gear.
- **Daily attendance reward**: a 7-day cycle of gold, potions, scrolls and
  Gale Marks, using the server's date (changing your PC clock won't help).
  A banner reminds you when today's reward is waiting.

**Added: equipment**
- 11 equipment slots: Weapon, Helmet, Torso, Legs, Boots, Necklace, 2 Rings,
  2 Earrings and a Back item.
- 21 new pieces of gear, including helmets (cap, sage hat, iron sallet, hood,
  circlet...), leg armour, rings, earrings, necklaces and back items
  (cloak, pack, cape, feather wings). Helmets, legs and back items show on
  your character.
- Vendors, monster drops and quest rewards now cover the new slots:
  Quartermaster Hesper sells a Lv 8 piece for every slot, and the tutorial
  quests hand out a novice bandana, trousers, earrings and a ring.
- Old characters keep their gear: armour moves to Torso, and old accessories
  move to the Necklace, Ring or Earring slot that fits them.

**Changed: characters**
- Character sprites restyled for more charm, closer to the reference sheets:
  heads and hair about a third bigger, big anime eyes with highlights and
  new expressions, chunkier arms, legs and torso, bigger fists and shoes, and
  a ready "guard" idle stance that breathes.

**Changed: gameplay**
- The whole game runs about 12% slower (players, monsters, projectiles,
  attacks). Ability cooldowns and buffs still count real seconds.
- Jumps are a little lower but hang in the air about 20% longer, for more
  air-combo time. Upward attack launches keep their height.
- Dodge (A/D + L) now keeps you invincible for its whole length.
- Projectiles have maximum ranges (about a screen for long bows and bolts,
  less for thrown knives and orbs; monster shots reach about two-thirds of a
  screen) instead of flying until they hit a wall.
- At most 4 flying monsters per map.
- Flying monsters' shots still hurt but no longer knock you back or stagger you.
- Bosses: talk to a boss's quest NPC to summon it into its arena for 100 gold
  (any time after taking the quest, so you can fight it again).

**Fixed**
- The Thunderhorn didn't appear when you took Kael's quest on the summit.
  Now any boss quest calls its boss straight away. The request also names
  the map, so the server never looks for the boss on the map you just left.
- Other players' attacks, abilities and projectiles were invisible. You
  now see their swings, arrows, bolts, knives, orbs and stars, and buff and
  heal effects.
- Melee abilities had no visual effect. Every melee swing and ability now
  draws a sweep in its ability colour, multi-hit spins draw a whirlwind, and
  all-round bursts draw a ring.
- Some abilities did no damage: melee abilities used by Windrunners and
  Aethermancers (e.g. Power Strike) borrowed a ranged attack's empty hitbox.
  The Aethermancer's Lv 12 Cyclone now shows a whirlwind.

**Tests**
- 112 in total, including a new end-to-end server test (three players:
  friends, whispers, blocking, party EXP sharing, guild founding, invites
  and chest, trading, profiles) and tests for guild quests, attendance,
  achievements, cosmetics, trades, the guild buff, projectile ranges, flyer
  caps, dodge invincibility, boss summoning and the new save format.

**Note:** the protocol changed, so the host and every friend need 0.9.0.
Existing characters and accounts carry over.

---

## 0.8.3 – Staying connected through playit.gg
*1 November 2026*

**Fixed**
- Players were disconnected after about 20 seconds on the login, character
  select or character creation screens. The game only sent its "still here"
  signal during play, so the server took a quiet player for a dropped one.
  The connection now keeps itself alive in the background every 4 seconds on
  every screen, including when the window stalls while being dragged or
  loading.
- The server now waits 45 seconds (was 20) before treating a silent player
  as gone, which gives internet hiccups more slack.

**Changed**
- Hosting now uses a playit.gg tunnel instead of Tailscale. The server's
  start-up banner, the login screen's Server address hint, and the README's
  hosting and troubleshooting sections explain the playit.gg setup.

**Tests**
- New test: a client sits idle on the menus past the old limit and is still
  connected (90 tests in total).

**Note:** the host and every friend need 0.8.3. The server is compatible with
0.8.2 clients, but those clients still drop on the menus.

---

## 0.8.2 – "localhost" fix
*1 November 2026*

- **Fixed: "No server answered at localhost:7777" on Windows while the server
  was running.**
  - Windows tries "localhost" as the IPv6 address `::1` first, but the
    server only listened on IPv4, and the game only tried the first address.
  - Now the game tries every address a name points to (IPv4 first), and the
    server listens on IPv6 as well as IPv4.
- Before this fix, typing `127.0.0.1` instead of `localhost` was the
  workaround.
- Gameplay and the network protocol are unchanged.
- New test: connecting with `localhost` and with the IPv6 loopback address.
- **Added afterwards: `allow-firewall.bat`** (no change to the game):
  - Run it on the host PC to let friends through Windows Firewall. It opens
    TCP port 7777 on every network type, because Windows often treats
    Tailscale as a *public* network.
  - `build-windows.bat` now copies it into the `GaleBound_Windows` folder.
  - The README has a new *If a friend can't connect* checklist.
- **`allow-firewall.bat` updated:**
  - It now also removes the **block** rules Windows creates for the server
    when only "Private networks" was allowed. Block rules beat allow rules,
    so they kept the port closed on networks Windows treats as public.
  - It ends with a short report: network type, Galebound firewall rules,
    the PC's addresses, and whether the server is listening.

---

## 0.8.1 – Windows .exe
*1 November 2026*

- New **`build-windows.bat`**: double-click it to build the game and the
  server into a ready-to-share **`GaleBound_Windows`** folder containing:
  - `Galebound.exe` (the game)
  - `GaleboundServer.exe` (the server)
  - `Start Server.bat`
  - the README and these patch notes
- `Galebound.exe` runs on its own. It needs no Rust and no extra runtimes
  (the C runtime is built in), so friends only need the .exe and Tailscale.
- The game now opens **without a black console window** behind it. The
  server keeps its window, since that's where you type commands.
- Gameplay and the network protocol are unchanged, so 0.8.0 and 0.8.1
  players can play together.

---

## 0.8.0 – Multiplayer
*31 October 2026*

### Host your own server
- New **dedicated server** program (`galebound-server`) for up to **16
  players** (change it with `--max-players`).
  - Start it with `start-server.bat`, or `cargo run --release -p
    galebound-server`.
  - It prints the address to give friends, and remembers everything between
    restarts.
- **Built for Tailscale:** friends connect to your PC's Tailscale address, so
  you need no port forwarding and nothing is exposed to the internet. Step-by-
  step setup is in the README under *Hosting for friends*.
- **Server console commands:** `list` (who's online and where), `say
  <message>` (message everyone) and `stop` (shut down).
- **Server options:** `--port` (default 7777), `--max-players`, `--data`
  (where accounts are saved) and `--motd` (a welcome message shown when
  players join).
- If the server is full, newcomers are told so ("16 / 16 players") and can
  try again later.
- Client and server check they're the same version, and say so clearly if
  not.

### Accounts live on the server
- The login screen has a **Server address** box: `localhost` for your own PC,
  or a friend's Tailscale address. It remembers the last one you used.
- Accounts, characters, progress and settings are stored **on the server**.
  Passwords are salted and hashed there and never sent back to players.
- **Remember me** now works per server: the server gives your computer a
  token, never your password.
- Logging into an account from a second computer signs the first one out.
- **Character names are unique across the whole server.**
- New characters are always created fresh on the server.
- As agreed, everyone starts fresh: characters from single-player saves stay
  on your PC but aren't used online.

### Shared world
- **See each other:** other players on your map appear with their own look,
  gear, animations, name and level, HP bar and pet. They also show on your
  minimap as blue dots.
- **Shared monsters:** the server runs every map's monsters (movement,
  attacks, deaths and respawns), so everyone fights the same ones. A monster
  goes after the nearest player who's still standing.
- **Shared kills:** everyone who hits a monster gets EXP and quest credit when
  it dies. EXP is split by damage dealt, and everyone who helped gets at least
  20% of it.
- **Quest bosses** (the Bramblehorn and the Thunderhorn) appear when someone
  on the map has the quest. Everyone there can fight them, and everyone who
  hits them gets the kill.
- **Shared loot:** gold and items fall for everyone and go to whoever picks
  them up first, you or your pet. Drop an item from your bag to hand it to a
  friend (bound items still can't be dropped).
- **Revive your friends:** walk up to a friend's tombstone and press Talk
  (E / V / Space). They get up with 30% HP. The EXP they lost stays lost;
  only a Revive Scroll gives it back.
- **Chat:** press **Enter** to talk to everyone on the server, then Enter to
  send or Esc to cancel. Joins, leaves and server messages show in yellow, and
  chat fades after a few seconds.

### How it works
- One TCP connection per player carrying short text messages. The server and
  the protocol use only the Rust standard library, so there's nothing extra
  to download.
- **The server decides:** accounts and saves, monsters, loot, who gets each
  drop, chat and revives. It checks every hit for range and damage, and every
  pick-up for distance.
- **Your game decides:** your own movement and hit detection, so combat
  feels instant even with some lag. This is how classic side-scrolling MMOs
  work.
- Your progress is sent to the server every 30 seconds, after loot and
  quests (at most every 3 seconds), and when you leave.

### Changes to controls
- **Enter is now Chat** in every layout. In the Arrow Keys and Classic MMO
  layouts, Talk is now only **V** and **Space** (they used to include Enter).
- Respawn is now a button click only (Enter used to work).

### Tests
- 89 automated tests (up from 79). The new tests cover:
  - every network message
  - online monsters, kills, loot and revives in the game
  - three end-to-end tests that start a real server on a real socket and
    connect real clients. These check sign-in, unique names, shared
    monsters, EXP splits, loot races, item drops, chat, revives, quest
    bosses, saving, remember-me tokens, signing in elsewhere, a full server
    and version checks.
- The game was also tested with two game windows connected to one server: the
  players saw each other, chatted, and one revived the other.

---

## 0.7.0 – Pets
*31 October 2026*

### Pet Shops
- Every main town now has a **Pet Shop**: Pip in Breezehollow (beside the
  windmill), and a pet keeper between the armourer and the inn in Ironhold,
  Shadowmere, Tempest Monastery, Leafwhisper and Starfall Spire. Each keeper
  has a small pen with a fence and a haystack.
- **Pets cost 10,000 gold**. **Pet Food costs 100 gold**.
- Prices of 1,000 gold or more now show thousands separators in shops
  (10,000g).

### Adopting and summoning
- Buy a pet, then **use it from your bag** to adopt it. If no pet is out, it
  comes out with you straight away.
- **Pets window (`O`, or `Esc` → Pets)** lists your pets and shows the chosen
  pet's level, EXP, hunger, status and passive skills. It has Summon / Send
  home and Feed buttons.
- You can own up to **6 pets**, with one out at a time. Summoning a pet sends
  the other one home.

### Three pets to choose from
| Pet | Style | Lv 1 | Lv 5 | Lv 10 | Lv 15 | Lv 20 |
|---|---|---|---|---|---|---|
| **Breeze Fox** | loot and gold | Keen Nose: +120 loot range | Gold Sense: +10% gold | Fleet Paws: +5% move speed | Lucky Tail: +5% crit chance | Treasure Hunter: +15% gold |
| **Puffling** (cloud lamb) | defence and healing | Woolly Guard: +8% P/M DEF | Cosy Fleece: heals 0.5% max HP per second | Fluffy Bulk: +60 HP | Cloud Cushion: +12% P/M DEF | Silver Lining: +120 HP |
| **Sparkwing** (storm bird, flies) | attack and Mana | Static Charge: +3% crit chance | Battle Song: +5% attack | Mana Spark: 1% max Mana per second | Storm Call: +20% crit damage | Thunderheart: +8% attack |

- Passive skills apply while the pet is out and fed. They work like gear
  bonuses and show up in your Character window.
- A toast tells you when your pet learns a new passive.

### Automatic looting
- Your pet **picks up loot on the ground for you**: anything within its range
  of you (260 units, plus the Breeze Fox's Keen Nose). It runs to each drop and
  the loot goes into your bag.
- Gold the pet picks up also gets the Gold Sense bonus. So does gold you pick
  up yourself with `F`.
- If your bag is full, the pet says "Bag full!" and tries again a few seconds
  later. Anything outside its range is left for you.

### Levels, Pet Food and hunger
- Pets go from **level 1 to 20**, and the only way to level them is to feed
  them **Pet Food**: +50 pet EXP and +30 hunger each. Levelling 1→20 takes
  about 130 Pet Food.
- Feed your pet from the Pets window, by clicking Pet Food in your bag, or
  from the hotbar.
- **Hunger** drains while the pet is out (full to empty in 30 minutes) and
  stops while it's at home.
  - Below 20, you're warned the pet is getting hungry.
  - At 0 the pet is **starving**: it greys out, stops looting, plods slowly
    behind you with a "Hungry!" bubble, and its passives switch off until you
    feed it.

### HUD and world
- A **pet panel** under your status bars shows the pet out with you, its
  level and its hunger bar.
- Pets have pixel-art sprites with walking and wing-beat animation, a name
  tag and a shadow. The Breeze Fox and Puffling trot along the ground; the
  Sparkwing flies at your shoulder.
- The Pets window and Pet Shop items have their own icons. The world map lists
  pet shops in each town's services.

### Other changes
- New key in every layout: **`O` Pets**. The `Esc` menu has a Pets entry.
- Saves store your pets (species, name, level, EXP and hunger) and which one
  is out. Older saves load as before.
- 79 automated tests (up from 70). The new tests cover:
  - a pet shop in every main town and the shop prices
  - buying, adopting and summoning pets
  - the pet looting within range and ignoring loot outside it
  - hunger draining and starving switching passives and looting off
  - feeding, levelling and passive unlocks (including healing)
  - sending pets home and swapping between them
  - Gold Sense
  - pets in the save file
  - pet sprites

---

## 0.6.0 – Loot, Vendors & Second Chances
*31 October 2026*

### Loot on the ground
- Monsters no longer put loot straight into your bag. **Gold and items now
  spill onto the ground** with a little bounce and a coloured glow, so you can
  see what dropped.
- **Press `F` to pick up** the nearest drop. **Hold `F`** to keep picking up
  everything in reach. The drop you'd pick up next shows its name and key.
- Picked-up loot flies to you. Gold drops as coin piles, and bigger amounts
  make bigger piles.
- Loot stays on the ground for **2 minutes**, blinks for the last 10 seconds,
  then disappears.
- If your bag is full, the item stays on the ground. If only part of a stack
  fits, you take what fits and the rest stays.
- **Drop items yourself**: drag an item out of the bag to drop the whole
  stack, or Shift-drag to drop one.
- **Pet-ready**: pets will use the same pick-up code (`Sim::pet_collect`), so
  loot a pet collects goes into your bag as if you'd picked it up.

### Tutorial vendor: Quartermaster Hesper
- New NPC **Hesper** with a supply wagon, right next to the training dummies
  in Dawnwind Glade.
- Sells **Trainee's Red Potion** (HP) and **Trainee's Blue Potion** (Mana).
  Your **first 10 are 40% off** (9g and 11g instead of 15g and 18g). The shop
  shows how many discounted potions you have left and the original price
  struck through.
- Trainee potions are **bound**: they can't be sold, traded or dropped. The
  Potion key drinks them like normal Red Potions.
- Sells **Level 8 gear**: Ironwood Blade, Ironbound Cudgel, Padded Gambeson
  and Trailblazer Boots. Revive Scrolls too.
- **Buys monster loot**: right-click items in your bag to sell, the same as at
  any shop.

### Tutorial quest rewards
- Tutorial quest NPCs now hand out **level 1 Novice gear**:

  | Quest | NPC | Reward |
  |---|---|---|
  | First Breeze | Elder Wynn (given by Tamsin) | Novice's Wind Charm (+1 DEX, +1 LUK) |
  | Gust Dodger | Tamsin | Novice's Boots |
  | Sparring Basics | Sergeant Brask | Novice's Practice Sword |
  | Mossling Trouble | Ranger Pell | Novice's Tunic |
  | The Bramblehorn | Warden Ilsa | Revive Scroll |

- **Quest rewards are shown before you accept.** The quest offer window now
  has a Rewards strip with EXP, gold, potions and item icons, above the
  Accept / Not now buttons.
- Every quest can now give items, not just EXP, gold and potions.

### Class advancement
- Changing class now also gives you a **Skill Reset Coupon** (bound).
  **Click it in your bag** at any time to refund every ability point you've
  spent, then rebuild your stats. It's used up when you click it.

### Defeat, tombstones and revives
- When you're defeated you no longer respawn automatically. You **turn into a
  tombstone** where you fell, and it stays there until:
  - you click **Respawn** (or press Enter), which takes you back to the last
    checkpoint with full HP and Mana,
  - you read a **Revive Scroll**, which gets you up on the spot with half your
    HP and Mana, or
  - **another player revives you**. This is ready in the code
    (`Sim::revive_by_player`) for when multiplayer arrives.
- **EXP penalty:** above level 10 (from level 11), a defeat costs **3% of your
  current EXP**. The defeat panel shows how much you lost. You never drop a
  level.
- **Revive Scrolls give back the lost EXP.** Reviving with Respawn or by
  another player doesn't.
- Revive Scrolls work from the defeat panel, from your bag or from the
  hotbar.
- Fall off the map and your tombstone appears where you last stood.
- Logging out as a tombstone keeps you as one, so quitting doesn't dodge the
  penalty.
- Monsters ignore tombstones.

### Minimap
- A new **minimap** in the top-right corner shows terrain, platforms, ladders
  and ropes, portals (greyed out while locked), NPCs, waystones, other players
  and you (with an arrow showing which way you face).
- NPCs with a new quest show as gold and turn-ins show as blue, so the minimap
  doubles as a quest finder.
- **Adjustable**: the header buttons make it smaller or bigger, switch between
  the area around you and the whole map, or hide it. **`N`** cycles through
  small, medium, large and hidden. Your choice is saved.
- The minimap shrinks to fit flat maps, and the quest tracker moves to sit
  just below it.

### Graphics options
- New **Graphics** entry in the `Esc` menu: **Windowed**, **Windowed
  Fullscreen** or **Fullscreen**. The game remembers your choice on this
  computer and starts in that mode next time.
- The graphics library (miniquad) can only make a borderless window that
  covers the screen; it can't switch the monitor into exclusive fullscreen.
  Fullscreen and Windowed Fullscreen therefore currently look the same. The
  option is kept separate so true exclusive fullscreen can be added later.

### Other changes
- New keys in every layout: **`F` Pick up loot** and **`N` Minimap size**.
  Both appear in Controls & Settings and the on-screen hints.
- Revive Scrolls and Skill Reset Coupons have their own icons and can go on
  the hotbar.
- Item tooltips mark bound items: "Bound: can't be sold, traded or dropped".
- The shop window is wider so long item names no longer overlap the prices,
  and descriptions are shortened to fit (the tooltip shows the full text).
- The `Esc` menu has a Graphics entry, and its rows are slightly tighter so
  everything fits.
- The login screen shows the version number (v0.6.0).
- Saves are now **version 4** and include display mode, minimap settings,
  trainee discounts used and tombstone state. Older saves load as before.

### Tests
- 70 automated tests (up from 59). New tests cover:
  - ground loot: dropping, picking up, bag full, despawning and pets
  - the vendor's discounts, bound items and selling loot
  - quest item rewards
  - the Skill Reset Coupon
  - the EXP penalty, Revive Scrolls and tombstones after logging out
  - the minimap buttons

---

## 0.5.0 – Roads, Hunting Grounds & the World Map
*31 October 2026*

- **Novice look**: new characters start in their own Adventurer's Garb (linen
  shirt, rope belt and satchel) with a Gnarled Stick. Garrick sells a Wooden
  Sword and Wooden Club. Take the garb off to show the outfit you designed.
- The weapon in your character's hand matches what you have equipped,
  including on character select.
- **Climbing**: ladders, ropes and vines. Up or Down to grab, hold to climb,
  Jump to let go. Your weapon is put away while climbing.
- **Roads and hunting grounds**: 10 new areas with 15 new monsters. Each class
  town has a road from Wayfarer's Crossroads (Lv 10–13) and a hunting ground
  past its east gate (Lv 16–24), built in layers with tougher monsters on the
  higher shelves.
- **Loot**: monsters drop sellable materials, potions and, rarely, gear.
- **World map (`M`)**: every area and the routes between them, where you are,
  attuned waystones, and each area's recommended level, monsters and possible
  loot.
- **Waystones** in Breezehollow and every class town. Visiting a town attunes
  its waystone, and any waystone can take you there for 25 gold.
- **Esc menu** is now a slim panel that slides in from the right. The game no
  longer pauses while it's open.
- Doors and portals always put you on the ground, never on a rooftop.
- The Controls & Settings screen uses three columns.

## 0.4.0 – Classes, Stats, Items & Class Towns
*31 October 2026*

- **Everyone starts as a Novice** and changes class at **level 10**. Take the
  North Road to Wayfarer's Crossroads, pick one of five class towns and talk
  to its instructor. Changing class refunds your ability points and gives you
  the class weapon and abilities.
- **Stats**: 5 ability points per level for Strength, Dexterity, Intelligence
  and Luck, with an Auto button. These feed HP, Mana, Physical and Magic
  Attack, Physical and Magical DEF, Critical Chance and Critical Damage.
- **Abilities and hotbar**: 8 slots on keys 1–8. The Novice has 3 abilities
  every class keeps, and each class learns 4 more at levels 10, 12, 15 and 20.
- **Items**: a 24-slot inventory, four equipment slots, and class- and
  level-restricted weapons and armour. Armour and boots change how your
  character looks.
- **Five class towns**: Ironhold, Shadowmere, Tempest Monastery, Leafwhisper
  and Starfall Spire. Each has an innkeeper, a class hall and three shops you
  can walk into (general goods, weapons, armour).
- Old saves keep their class, get their points auto-spent and receive the
  class weapon.

## 0.3.0 – Rebuilt Characters & Three New Zones
*31 October 2026*

- **Characters rebuilt** on a proper skeleton, based on your sprite sheets:
  - knees and elbows bend the right way and feet stay planted
  - fists-up guard, 8-frame run, jab, high kick and tucked jump
  - clothes are drawn on the limbs they cover
- Underwear base bodies, a Body Type option in the creator, and "None" for
  tops and bottoms. New options: long locs, short curls, a sport crop top,
  baggy shorts and high-top sneakers.
- **Calmer pacing**: about a third less gravity, about 20% slower running and
  about 15% slower attacks, with monsters slowed to match.
- **Three new zones and 14 quests**:
  - Whistling Fields (Lv 5–8)
  - Thornvale Woods (Lv 9–12, at night)
  - Windcrag Cliffs (Lv 12–16), ending at the Thunderhorn summit boss

## 0.2.0 – Anime Pixel Art
*29 October 2026*

- The whole game is redrawn in **anime pixel art** on a 640×360 canvas,
  scaled up with hard pixel edges.
- Characters have outlines, big anime eyes and hair drawn in strands, plus
  full animation frames: run, idle, jump, fall, dodge, hurt, defeat and 13
  attack motions.
- Pixel-art world: cobblestone cliffs, grass tufts, parallax mountains and
  forest, redrawn houses, windmill, gate and enemies. The Bramblehorn has
  charge, stun and enraged frames.
- A hand-made pixel font for menus, the HUD and damage numbers.

## 0.1.0 – First Playable
*29 October 2026*

- Written in Rust with very light physics and AABB collision.
- **Login screen** with e-mail and password (salted PBKDF2 hashes), and a
  Remember Me option with a warning for shared or public computers.
- **Character creation**: hairstyles, faces, tops, bottoms, shoes and skin
  tones, with 3 character slots.
- **Five original classes** inspired by side-scroller MMO roles (Vanguard,
  Duskblade, Stormfist, Windrunner and Aethermancer), with Brawlhalla-style
  combat: directional light and heavy attacks, dodges, double jumps, air
  recoveries, knockback and hit-stop.
- **Tutorial**: Dawnwind Glade with quest NPCs and the Bramblehorn mini-boss,
  leading to the first town, Breezehollow.
- Three control layouts: WASD, Arrow Keys and Classic MMO.
