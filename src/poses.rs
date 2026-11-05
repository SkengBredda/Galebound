//! # The animation book
//!
//! Every pose a character can strike lives in this file: standing around,
//! the battle stance, running, jumping, and every attack. The drawing code
//! (`charsprite.rs`) reads these poses and draws the body, clothes, hair
//! and weapon on top, so **you can change how characters move without
//! touching any drawing code.**
//!
//! ## How to read a pose
//!
//! Characters are drawn facing **right**. All angles are in degrees.
//!
//! ```text
//!                 180 (straight up)
//!                    |
//!    behind  -90 ----+---- 90  in front (the way they face)
//!                    |
//!                    0 (straight down)
//! ```
//!
//! * `lean`: tilts the whole upper body. Positive leans forward, negative
//!   leans back.
//! * `front_leg`, `back_leg`, `front_arm`, `back_arm` are each a
//!   `limb(swing, bend)`:
//!   * `swing` is where the limb points, using the dial above. 0 hangs
//!     straight down, 90 points straight ahead, 180 points up. Arms are
//!     measured from the body's lean, legs from straight down.
//!   * `bend` is how much the knee or elbow folds (0 = straight). Knees
//!     always fold backward and elbows forward, like real joints.
//! * `weapon`: angle of the weapon compared with the forearm. 0 means the
//!   weapon carries straight on from the arm (good for stabs and swings).
//! * `aim`: (only for strikes) makes the front arm and weapon point at a
//!   spot, measured from the character's middle in the **same units as
//!   the hitboxes in `core/src/classes.rs`** (x forward, y down). Use it to
//!   make a swing visibly land where the hit actually happens. The test
//!   `strikes_land_in_their_hitboxes` checks this for every class.
//!
//! ## Trying out an edit
//!
//! 1. Change a number, e.g. make the idle lean `6.0` instead of `2.0`.
//! 2. Double-click `preview-poses.bat`. It draws every animation into
//!    `pose_preview.png` (no need to start the game or a server).
//! 3. When you're happy, build the game as usual.
//!
//! Each animation has a fixed number of frames (`Anim::frames` in
//! `charsprite.rs`). Attacks always have 4: 0 = wind-up, 1 = the hit,
//! 2 = follow-through, 3 = settling back.

use crate::charsprite::{Anim, Motion};
use galebound_core::classes::Weapon;

/// How long (seconds) a character stays in the battle stance after
/// attacking or being hit, before relaxing into the normal idle pose.
pub const BATTLE_STANCE_SECONDS: f32 = 5.0;

// =================================================================== the pieces

/// One bendy limb: where it points (`swing`) and how much the middle joint
/// folds (`bend`). See the dial at the top of the file.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Limb {
    pub swing: f32,
    pub bend: f32,
}

/// Shorthand for writing a limb: `limb(swing, bend)`.
pub const fn limb(swing: f32, bend: f32) -> Limb {
    Limb { swing, bend }
}

/// What holds the body up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Plant {
    /// The lowest foot touches the ground (standing, running).
    Lowest,
    /// The back foot is the standing leg (kicks, sweeps).
    Back,
    /// In the air: the hips float at standing height.
    Air,
}

/// Where a motion streak (the white "whoosh" on the sprite) is centred.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SwooshAt {
    Shoulder,
    Hip,
}

/// A white motion streak sweeping from angle `from` to angle `to`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Swoosh {
    pub from: f32,
    pub to: f32,
    pub at: SwooshAt,
}

const fn swoosh(from: f32, to: f32, at: SwooshAt) -> Option<Swoosh> {
    Some(Swoosh { from, to, at })
}

/// A complete body pose. Most poses only change a few fields and copy the
/// rest from another pose with `..STAND`.
#[derive(Clone, Copy, Debug)]
pub struct Pose {
    /// Upper-body tilt (positive = forward).
    pub lean: f32,
    /// Raise the whole body this many pixels (little hops).
    pub lift: f32,
    pub plant: Plant,
    pub front_leg: Limb,
    pub back_leg: Limb,
    pub front_arm: Limb,
    pub back_arm: Limb,
    /// Front-hand weapon angle compared with the forearm.
    pub weapon: f32,
    /// Back-hand weapon angle (second dagger) compared with the back forearm.
    pub back_weapon: f32,
    /// Strike target (see the top of the file), or None.
    pub aim: Option<(f32, f32)>,
    /// Hair blowing backward (0 = still, 1 = running).
    pub hair_trail: f32,
    /// Hair lifting up (positive) or pressed down (negative).
    pub hair_lift: f32,
    pub blink: bool,
    /// Mouth open (shouting, talking).
    pub mouth: bool,
    /// Pained face.
    pub hurt: bool,
    pub swoosh: Option<Swoosh>,
    /// Force the front foot to this angle (kicks), instead of automatic.
    pub front_foot: Option<f32>,
}

/// Plain standing, arms hanging, weapon pointing down. Every other pose
/// starts from this one.
pub const STAND: Pose = Pose {
    lean: 2.0,
    lift: 0.0,
    plant: Plant::Lowest,
    front_leg: limb(5.0, 6.0),
    back_leg: limb(-5.0, 5.0),
    front_arm: limb(4.0, 12.0),
    back_arm: limb(-6.0, 10.0),
    weapon: 10.0,
    back_weapon: 10.0,
    aim: None,
    hair_trail: 0.0,
    hair_lift: 0.0,
    blink: false,
    mouth: false,
    hurt: false,
    swoosh: None,
    front_foot: None,
};

/// Fists-up fighting guard (brawlers and dagger users).
pub const GUARD: Pose = Pose {
    lean: 5.0,
    front_leg: limb(14.0, 10.0),
    back_leg: limb(-14.0, 12.0),
    front_arm: limb(28.0, 132.0),
    back_arm: limb(42.0, 128.0),
    weapon: 60.0,
    back_weapon: 60.0,
    ..STAND
};

/// Ready stance for weapon users: feet apart, knees soft.
pub const READY: Pose = Pose {
    lean: 4.0,
    front_leg: limb(10.0, 9.0),
    back_leg: limb(-9.0, 10.0),
    front_arm: limb(18.0, 112.0),
    back_arm: limb(30.0, 104.0),
    weapon: 60.0,
    back_weapon: 60.0,
    ..STAND
};

fn uses_fists(weapon: Option<Weapon>) -> bool {
    matches!(weapon, Some(Weapon::Gauntlets) | Some(Weapon::TwinDaggers))
}

// =================================================================== everyday poses

/// Relaxed standing (not fighting): arms down, weapon lowered.
/// Frames 1 and 2 breathe in slightly.
fn idle(frame: usize, weapon: Option<Weapon>) -> Pose {
    let breathe = if frame == 1 || frame == 2 { 1.0 } else { 0.0 };
    let mut p = Pose {
        front_leg: limb(5.0, 6.0 + breathe * 3.0),
        back_leg: limb(-5.0, 5.0 + breathe * 3.0),
        front_arm: limb(4.0 - breathe, 12.0 + breathe * 3.0),
        back_arm: limb(-6.0 + breathe, 10.0 + breathe * 3.0),
        ..STAND
    };
    match weapon {
        // blade or club held low, tip toward the ground in front
        Some(Weapon::SwordShield) | Some(Weapon::WoodSword) | Some(Weapon::Club) | Some(Weapon::Stick) => {
            p.front_arm = limb(8.0, 14.0 + breathe * 3.0);
            p.weapon = 6.0;
        }
        // both daggers point down
        Some(Weapon::TwinDaggers) => {
            p.weapon = 2.0;
            p.back_weapon = 2.0;
        }
        // bow held upright at the side
        Some(Weapon::Bow) => {
            p.front_arm = limb(10.0, 16.0 + breathe * 3.0);
            p.weapon = 62.0;
        }
        // staff held upright like a walking stick
        Some(Weapon::Staff) => {
            p.front_arm = limb(14.0, 34.0 + breathe * 3.0);
            p.weapon = 128.0;
        }
        Some(Weapon::Gauntlets) | None => {}
    }
    p
}

/// Battle stance (shown for a few seconds after attacking or being hit).
/// The knees dip and the arms rise on frames 1 and 2.
fn battle(frame: usize, weapon: Option<Weapon>) -> Pose {
    let dip = if frame == 1 || frame == 2 { 6.0 } else { 0.0 };
    let breathe = if frame == 1 || frame == 2 { 4.0 } else { 0.0 };
    let mut p = if uses_fists(weapon) { GUARD } else { READY };
    p.front_leg.bend += dip;
    p.back_leg.bend += dip;
    match weapon {
        Some(Weapon::Gauntlets) | Some(Weapon::TwinDaggers) => p.front_arm.bend -= breathe,
        Some(Weapon::SwordShield) | Some(Weapon::WoodSword) => {
            p.front_arm = limb(12.0, 38.0 + breathe);
            p.back_arm = limb(18.0, 60.0);
        }
        Some(Weapon::Bow) => {
            p.front_arm = limb(8.0, 24.0 + breathe);
            p.weapon = 0.0;
        }
        Some(Weapon::Staff) => {
            p.front_arm = limb(14.0, 62.0 + breathe);
            p.weapon = 100.0;
        }
        _ => {
            p.front_arm.bend += breathe;
            p.back_arm.bend += breathe;
            p.front_arm.swing -= breathe * 0.5;
        }
    }
    p
}

/// Talking to someone: one hand gestures, mouth opens and closes.
fn talk(frame: usize) -> Pose {
    Pose { front_arm: limb(24.0, 55.0 + (frame % 2) as f32 * 25.0), mouth: frame % 2 == 0, ..STAND }
}

/// Run cycle: deep knee lift, straight push-off leg, fists up, forward lean.
fn run(frame: usize) -> Pose {
    // Front leg swing and knee bend for each of the 8 frames.
    // The back leg uses the same table, half a cycle later.
    const SWING: [f32; 8] = [35.0, 22.0, 5.0, -15.0, -32.0, -18.0, 15.0, 40.0];
    const KNEE: [f32; 8] = [12.0, 22.0, 25.0, 35.0, 55.0, 105.0, 100.0, 45.0];
    let f = frame;
    let b = (f + 4) % 8;
    Pose {
        lean: 14.0,
        lift: if f == 3 || f == 7 { 1.5 } else { 0.0 },
        front_leg: limb(SWING[f], KNEE[f]),
        back_leg: limb(SWING[b], KNEE[b]),
        // arms swing opposite to the legs
        front_arm: limb(-SWING[f] * 0.9 + 12.0, 100.0),
        back_arm: limb(-SWING[b] * 0.9 + 12.0, 100.0),
        weapon: 40.0,
        back_weapon: 60.0,
        hair_trail: 1.0,
        hair_lift: if f % 4 == 3 { 0.25 } else { 0.1 },
        ..STAND
    }
}

fn jump_up(frame: usize) -> Pose {
    let base = Pose { plant: Plant::Air, hair_trail: 0.2, weapon: 60.0, back_weapon: 60.0, ..STAND };
    if frame == 0 {
        // take-off: front knee tucked, arms thrown up
        Pose {
            lean: 6.0,
            front_leg: limb(55.0, 95.0),
            back_leg: limb(-5.0, 50.0),
            front_arm: limb(60.0, 80.0),
            back_arm: limb(-35.0, 60.0),
            hair_lift: -0.4,
            ..base
        }
    } else {
        // rising: both knees tucked
        Pose {
            lean: 8.0,
            front_leg: limb(70.0, 115.0),
            back_leg: limb(35.0, 105.0),
            front_arm: limb(40.0, 115.0),
            back_arm: limb(20.0, 110.0),
            hair_lift: 0.3,
            ..base
        }
    }
}

fn fall(frame: usize) -> Pose {
    let base = Pose { plant: Plant::Air, hair_trail: 0.3, weapon: 60.0, back_weapon: 60.0, ..STAND };
    if frame == 0 {
        Pose {
            lean: 4.0,
            front_leg: limb(25.0, 45.0),
            back_leg: limb(-15.0, 35.0),
            front_arm: limb(80.0, 30.0),
            back_arm: limb(-70.0, 25.0),
            hair_lift: 0.8,
            ..base
        }
    } else {
        // arms out for balance before landing
        Pose {
            lean: 2.0,
            front_leg: limb(12.0, 22.0),
            back_leg: limb(-10.0, 15.0),
            front_arm: limb(100.0, 20.0),
            back_arm: limb(-90.0, 20.0),
            hair_lift: 1.0,
            ..base
        }
    }
}

fn dodge(frame: usize) -> Pose {
    let base = Pose { hair_trail: 1.2, weapon: 60.0, back_weapon: 60.0, ..STAND };
    if frame == 0 {
        Pose {
            lean: 28.0,
            front_leg: limb(55.0, 85.0),
            back_leg: limb(-35.0, 40.0),
            front_arm: limb(-50.0, 40.0),
            back_arm: limb(-70.0, 30.0),
            ..base
        }
    } else {
        Pose {
            lean: 22.0,
            front_leg: limb(40.0, 70.0),
            back_leg: limb(-45.0, 30.0),
            front_arm: limb(-40.0, 50.0),
            back_arm: limb(-60.0, 40.0),
            blink: true,
            ..base
        }
    }
}

fn hurt() -> Pose {
    Pose {
        lean: -14.0,
        front_leg: limb(10.0, 18.0),
        back_leg: limb(-14.0, 10.0),
        front_arm: limb(-50.0, 40.0),
        back_arm: limb(50.0, 30.0),
        weapon: 60.0,
        back_weapon: 60.0,
        hurt: true,
        hair_lift: 0.4,
        hair_trail: -0.3,
        ..STAND
    }
}

/// On a ladder, rope or vine: hands reach up in turn, knees step.
fn climb(frame: usize) -> Pose {
    let up = frame % 2 == 0;
    let step = if frame >= 2 { 10.0 } else { 0.0 };
    let (high, low) = (limb(138.0, 12.0), limb(100.0, 45.0));
    Pose {
        lean: 4.0,
        plant: Plant::Air,
        front_arm: if up { high } else { low },
        back_arm: if up { low } else { high },
        front_leg: if up { limb(55.0, 95.0 - step) } else { limb(20.0, 40.0) },
        back_leg: if up { limb(15.0, 35.0) } else { limb(50.0, 90.0 - step) },
        weapon: 120.0,
        back_weapon: 120.0,
        hair_lift: 0.1,
        ..STAND
    }
}

// =================================================================== attacks

/// Attacks: frame 0 = wind-up, 1 = the hit, 2 = follow-through, 3 = settle.
fn attack(m: Motion, f: usize, weapon: Option<Weapon>) -> Pose {
    use SwooshAt::*;
    let hit = f == 1 || f == 2;
    // Every attack starts from the guard, shouts on the hit, and lines the
    // weapon up with the arm while striking.
    let base = Pose { mouth: f == 1, weapon: if hit { 0.0 } else { 50.0 }, back_weapon: 30.0, ..GUARD };
    let ff = f as f32;
    let mut p = match m {
        Motion::Jab | Motion::Throw => {
            let wind = if m == Motion::Throw { limb(-50.0, 80.0) } else { limb(20.0, 140.0) };
            match f {
                0 => Pose { lean: -2.0, front_arm: wind, back_arm: limb(42.0, 128.0), ..base },
                1 | 2 => Pose {
                    lean: 10.0,
                    front_leg: limb(20.0, 8.0),
                    back_leg: limb(-20.0, 10.0),
                    front_arm: if f == 1 { limb(84.0, 4.0) } else { limb(80.0, 10.0) },
                    back_arm: limb(40.0, 130.0),
                    aim: if m == Motion::Jab { Some((40.0, -8.0)) } else { None },
                    swoosh: if f == 1 { swoosh(60.0, 100.0, Shoulder) } else { None },
                    ..base
                },
                _ => Pose { lean: 6.0, front_arm: limb(45.0, 90.0), back_arm: limb(42.0, 128.0), ..base },
            }
        }
        Motion::Thrust => match f {
            0 => Pose {
                lean: -2.0,
                front_leg: limb(10.0, 12.0),
                back_leg: limb(-10.0, 10.0),
                front_arm: limb(10.0, 100.0),
                back_arm: limb(40.0, 120.0),
                ..base
            },
            1 | 2 => Pose {
                lean: 16.0,
                front_leg: limb(42.0, 40.0),
                back_leg: limb(-38.0, 4.0),
                front_arm: limb(88.0 - ff * 2.0, 2.0 + ff * 2.0),
                back_arm: limb(-30.0, 30.0),
                aim: Some((48.0, -2.0)),
                swoosh: if f == 1 { swoosh(60.0, 110.0, Shoulder) } else { None },
                ..base
            },
            _ => Pose {
                lean: 8.0,
                front_leg: limb(25.0, 25.0),
                back_leg: limb(-22.0, 8.0),
                front_arm: limb(55.0, 60.0),
                back_arm: limb(20.0, 90.0),
                ..base
            },
        },
        Motion::Sweep => match f {
            0 => Pose {
                lean: 10.0,
                front_leg: limb(35.0, 70.0),
                back_leg: limb(-15.0, 60.0),
                front_arm: limb(50.0, 70.0),
                back_arm: limb(20.0, 90.0),
                ..base
            },
            // low kick along the ground, standing on the bent back leg
            1 | 2 => Pose {
                lean: 20.0,
                plant: Plant::Back,
                front_leg: limb(82.0 - ff * 3.0, 0.0),
                back_leg: limb(-5.0, 115.0),
                front_arm: limb(40.0, 60.0),
                back_arm: limb(-50.0, 40.0),
                swoosh: if f == 1 { swoosh(40.0, 100.0, Hip) } else { None },
                ..base
            },
            _ => Pose {
                lean: 10.0,
                front_leg: limb(40.0, 60.0),
                back_leg: limb(-10.0, 70.0),
                front_arm: limb(45.0, 80.0),
                back_arm: limb(30.0, 100.0),
                ..base
            },
        },
        Motion::Uppercut => match f {
            0 => Pose {
                lean: 12.0,
                front_leg: limb(30.0, 55.0),
                back_leg: limb(-20.0, 55.0),
                front_arm: limb(-10.0, 120.0),
                back_arm: limb(40.0, 120.0),
                ..base
            },
            1 | 2 => Pose {
                lean: -6.0,
                lift: 3.0,
                front_leg: limb(15.0, 10.0),
                back_leg: limb(-20.0, 30.0),
                front_arm: limb(140.0, 10.0),
                back_arm: limb(30.0, 100.0),
                aim: Some((30.0, -28.0)),
                hair_lift: 0.5,
                swoosh: if f == 1 { swoosh(90.0, 165.0, Shoulder) } else { None },
                ..base
            },
            _ => Pose { lean: 2.0, front_arm: limb(100.0, 60.0), back_arm: limb(40.0, 120.0), ..base },
        },
        Motion::Spin => {
            // arms sweep round in a circle (front arm, back arm) per frame
            let (a, b) = [(90.0, -90.0), (120.0, -60.0), (150.0, -30.0), (60.0, -40.0)][f.min(3)];
            let bend = if f == 3 { 40.0 } else { 0.0 };
            Pose {
                plant: Plant::Air,
                front_leg: limb(60.0, 100.0),
                back_leg: limb(20.0, 100.0),
                front_arm: limb(a, bend),
                back_arm: limb(b, bend),
                aim: if f == 1 { Some((32.0, -14.0)) } else { None },
                swoosh: if f == 1 { swoosh(30.0, 170.0, Shoulder) } else { None },
                ..base
            }
        }
        Motion::Spike => {
            // downward strike in the air, at whatever is below
            let air = Pose { plant: Plant::Air, front_leg: limb(55.0, 100.0), back_leg: limb(25.0, 90.0), ..base };
            match f {
                0 => Pose { lean: -6.0, front_arm: limb(170.0, 10.0), back_arm: limb(150.0, 20.0), ..air },
                1 | 2 => Pose {
                    lean: 14.0,
                    front_arm: limb(50.0 - ff * 7.0, 0.0),
                    back_arm: limb(30.0, 10.0),
                    aim: Some((12.0, 32.0)),
                    swoosh: if f == 1 { swoosh(40.0, 170.0, Shoulder) } else { None },
                    ..air
                },
                _ => Pose { lean: 6.0, front_arm: limb(60.0, 60.0), back_arm: limb(40.0, 60.0), ..air },
            }
        }
        Motion::Stomp => {
            // heel drop: knee up, then the front leg stamps straight down
            let air = Pose { plant: Plant::Air, back_leg: limb(55.0, 110.0), ..base };
            match f {
                0 => Pose { lean: -6.0, front_leg: limb(95.0, 120.0), front_arm: limb(120.0, 40.0), back_arm: limb(-60.0, 40.0), ..air },
                1 | 2 => Pose {
                    lean: 8.0,
                    front_leg: limb(18.0 - ff * 4.0, 0.0),
                    front_foot: Some(40.0),
                    front_arm: limb(110.0, 50.0),
                    back_arm: limb(-80.0, 30.0),
                    hair_lift: 0.6,
                    swoosh: if f == 1 { swoosh(110.0, 20.0, Hip) } else { None },
                    ..air
                },
                _ => Pose { lean: 2.0, front_leg: limb(40.0, 70.0), front_arm: limb(60.0, 80.0), back_arm: limb(-30.0, 60.0), ..air },
            }
        }
        Motion::Slam => match f {
            0 => Pose {
                lean: -10.0,
                front_leg: limb(12.0, 10.0),
                back_leg: limb(-12.0, 8.0),
                front_arm: limb(175.0, 15.0),
                back_arm: limb(160.0, 20.0),
                ..base
            },
            1 | 2 => Pose {
                lean: 18.0 + ff,
                front_leg: limb(30.0, 35.0),
                back_leg: limb(-30.0, 10.0),
                front_arm: limb(60.0 - ff * 7.0, 0.0),
                back_arm: limb(40.0, 10.0),
                swoosh: if f == 1 { swoosh(40.0, 175.0, Shoulder) } else { None },
                ..base
            },
            _ => Pose { lean: 8.0, front_arm: limb(70.0, 50.0), back_arm: limb(40.0, 80.0), ..base },
        },
        Motion::Kick => {
            // high kick: chamber, extend, hold, re-chamber
            let kick = Pose { plant: Plant::Back, weapon: 50.0, ..base };
            match f {
                0 => Pose { lean: -2.0, front_leg: limb(95.0, 130.0), back_leg: limb(-6.0, 6.0), ..kick },
                1 | 2 => Pose {
                    lean: -20.0 + ff,
                    front_leg: limb(128.0 - ff * 5.0, 0.0),
                    back_leg: limb(-12.0, 4.0),
                    front_arm: limb(20.0, 130.0),
                    back_arm: limb(50.0, 120.0),
                    front_foot: Some(-15.0),
                    swoosh: if f == 1 { swoosh(100.0, 140.0, Hip) } else { None },
                    ..kick
                },
                _ => Pose { lean: -2.0, front_leg: limb(80.0, 115.0), back_leg: limb(-6.0, 6.0), ..kick },
            }
        }
        Motion::Cast => {
            let cast = Pose { front_leg: limb(12.0, 8.0), back_leg: limb(-12.0, 8.0), weapon: 0.0, ..base };
            match f {
                0 => Pose { lean: -4.0, front_arm: limb(150.0, 30.0), back_arm: limb(-20.0, 30.0), ..cast },
                1 | 2 => Pose { lean: 8.0, front_arm: limb(88.0, 0.0), back_arm: limb(-40.0, 20.0), ..cast },
                _ => Pose { lean: 4.0, front_arm: limb(60.0, 30.0), back_arm: limb(-20.0, 30.0), ..cast },
            }
        }
        Motion::Shoot => {
            // bow arm straight out, string hand draws back then releases
            let draw = [limb(70.0, 60.0), limb(80.0, 150.0), limb(75.0, 155.0), limb(-20.0, 30.0)][f.min(3)];
            Pose {
                lean: 2.0,
                front_leg: limb(16.0, 8.0),
                back_leg: limb(-16.0, 8.0),
                front_arm: limb(90.0, 0.0),
                back_arm: draw,
                weapon: 0.0,
                ..base
            }
        }
        Motion::Dive => Pose {
            lean: 30.0,
            plant: Plant::Air,
            front_leg: limb(75.0, 0.0),
            back_leg: limb(-20.0, 100.0),
            front_arm: limb(-40.0, 30.0),
            back_arm: limb(-60.0, 30.0),
            front_foot: Some(10.0),
            hair_lift: 1.0,
            swoosh: if f == 1 { swoosh(40.0, 90.0, Hip) } else { None },
            ..base
        },
        Motion::Rise => Pose {
            lean: -4.0,
            plant: Plant::Air,
            front_leg: limb(60.0, 110.0),
            back_leg: limb(20.0, 100.0),
            front_arm: limb(175.0, 5.0),
            back_arm: limb(130.0, 30.0),
            weapon: 0.0,
            hair_lift: -0.6,
            swoosh: if f <= 1 { swoosh(100.0, 180.0, Shoulder) } else { None },
            ..base
        },
    };
    if weapon == Some(Weapon::Bow) && m != Motion::Shoot {
        p.weapon = 0.0;
    }
    p
}

// =================================================================== lookup

/// The pose for an animation frame. This is what the drawing code calls.
pub fn pose_for(anim: Anim, frame: u8, weapon: Option<Weapon>) -> Pose {
    let f = frame as usize;
    match anim {
        Anim::Idle => idle(f, weapon),
        Anim::Battle => battle(f, weapon),
        Anim::Talk => talk(f),
        Anim::Run => run(f),
        Anim::JumpUp => jump_up(f),
        Anim::Fall => fall(f),
        Anim::Dodge => dodge(f),
        Anim::Hurt => hurt(),
        Anim::Dead => STAND,
        Anim::Climb => climb(f),
        Anim::Attack(m) => attack(m, f, weapon),
    }
}
