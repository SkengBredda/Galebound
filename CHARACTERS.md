# Editing characters and animations

This guide is for changing how characters stand, run and fight. You don't
need to know much Rust: a pose is a list of angles, and you change numbers.

## Where things are

| What you want to change | File |
|---|---|
| Any pose or animation (idle, battle stance, run, jump, every attack) | `src/poses.rs` |
| How long the battle stance lasts after a fight | `BATTLE_STANCE_SECONDS` at the top of `src/poses.rs` |
| Where an attack hits, how hard, how fast | `core/src/classes.rs` (`attack_def_raw`) |
| Which pose each class uses for each attack | `motion_for` near the top of `src/charsprite.rs` |
| How a body, hair, clothes or weapon is drawn | `src/charsprite.rs` (more advanced) |

## Reading a pose

Characters always face **right**. Angles are in degrees on this dial:

```
                180 (straight up)
                   |
   behind  -90 ----+---- 90  in front
                   |
                   0 (straight down)
```

A pose looks like this (the relaxed standing pose):

```rust
pub const STAND: Pose = Pose {
    lean: 2.0,                     // body tilt: + forward, - back
    front_leg: limb(5.0, 6.0),     // limb(where it points, how much the knee bends)
    back_leg: limb(-5.0, 5.0),
    front_arm: limb(4.0, 12.0),    // limb(where it points, how much the elbow bends)
    back_arm: limb(-6.0, 10.0),
    weapon: 10.0,                  // weapon angle compared with the forearm
    ...
};
```

- `limb(swing, bend)`: **swing** is where the whole limb points on the dial;
  **bend** folds the knee or elbow (0 = straight, up to about 160).
- Other poses copy everything they don't mention: `..STAND` at the end means
  "the rest is the same as STAND".
- The front arm and leg are the ones nearer the camera.

### Small examples

- Arms hang wider when idle: in `idle()` change `front_arm: limb(4.0 - breathe, ...)`
  to `limb(10.0 - breathe, ...)`.
- A more aggressive battle stance: in `GUARD`, change `lean: 5.0` to `lean: 10.0`.
- Battle stance lasts longer: set `BATTLE_STANCE_SECONDS` to `8.0`.

## Frames

Each animation has a set number of frames (`Anim::frames` in
`charsprite.rs`): idle and battle 4, run 8, attacks 4. Attack frames are:

| Frame | Meaning |
|---|---|
| 0 | wind-up |
| 1 | **the hit** (this is when the hitbox is live) |
| 2 | follow-through |
| 3 | settling back |

## Lining strikes up with hitboxes

Attacks can have an `aim`: a spot the front arm and weapon point at,
measured from the character's middle in the same units as the hitboxes in
`core/src/classes.rs` (x forward, y down). For example the jab uses
`aim: Some((40.0, -8.0))`: 40 in front, 8 above the middle. Kicks and
sweeps don't use `aim`; their legs are posed directly.

If you change an attack's hitbox in `classes.rs`, move its `aim` (or the
leg angles) so the weapon still lands inside it.

## Checking your changes

1. **Preview without playing:** double-click `preview-poses.bat`. It first
   checks every melee attack lands inside its hitbox (and tells you which
   one doesn't), then opens `pose_preview.png`:
   - one row per class (Novice, Vanguard, Duskblade, Stormfist, Windrunner,
     Aethermancer);
   - columns: idle, idle breathing, battle, battle breathing, run x2, jump,
     fall, dodge, hurt, climb, then the hit frame of each attack (light
     neutral, light side, light down, air neutral, air side, air down,
     heavy neutral, heavy side, heavy down, air heavy neutral, air heavy
     down);
   - attack cells show the **real hitbox in red**. A good pose puts the
     weapon, fist or foot inside the red box.
2. **In the game:** press **F3** to show hitboxes: your body in green,
   monsters in blue, your attack in red while it can hit (yellow while
   winding up or recovering).
3. If something won't build, copy the red `error` lines and send them to
   Claude.

## Adding a new attack pose

1. Add a name to `enum Motion` in `src/charsprite.rs`.
2. Add it to the **end** of `MOTIONS` in `src/online.rs` (so other players
   see it; the end keeps older games working).
3. Add its 4 frames in `attack()` in `src/poses.rs` (copy a similar one).
4. Use it for a class in `motion_for` in `src/charsprite.rs`.
5. Run `preview-poses.bat`.
