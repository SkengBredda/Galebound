//! Anime pixel-art character sprites, generated in code.
//!
//! A 2D skeleton (pelvis -> spine -> neck -> head, shoulders -> upper arm ->
//! forearm -> fist, hips -> thigh -> shin -> foot) is posed from hand-made
//! keyframes modelled on the reference sprite sheets (idle, guard, run, jump,
//! punch, high kick ...). Joints have real limits: knees only bend backward,
//! elbows only forward. Feet are planted on the ground automatically.
//!
//! Every body part is drawn per segment, and so is clothing: sleeves live on
//! the arm bones, trouser legs on the leg bones and skirts hang from the
//! hips toward the knees, so clothes move with the limbs. The base body wears
//! underwear (boxers, or briefs and a sports bra) under any clothing.
//!
//! Output: 96x96 RGBA, feet at (48, 88), facing right.

use crate::pixel::{from3, light, mix, rgb, shadow, Canvas, Px, INK};
use galebound_core::appearance::{Appearance, DYES, HAIR_COLORS, SKIN_TONES};
use galebound_core::classes::{AttackKind, ClassId, Weapon};

pub const SIZE: i32 = 96;
pub const HEIGHT: i32 = 96;
pub const FEET_X: i32 = 48;
pub const FEET_Y: i32 = 88;

// Bone lengths (sprite pixels).
const THIGH: f32 = 14.0;
const SHIN: f32 = 13.5;
const ANKLE_H: f32 = 2.0; // ankle joint above the sole
const UPPER: f32 = 8.5;
const FORE: f32 = 8.0;
const NECK_Y: f32 = 15.0; // neck base above pelvis
const HEAD_Y: f32 = 24.0; // head centre above pelvis

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Motion {
    Jab,
    Thrust,
    Sweep,
    Uppercut,
    Spin,
    Spike,
    Slam,
    Kick,
    Cast,
    Shoot,
    Dive,
    Rise,
    Throw,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Anim {
    Idle,
    Run,
    JumpUp,
    Fall,
    Attack(Motion),
    Dodge,
    Hurt,
    Dead,
    Talk,
    /// On a ladder / rope / vine (seen from the side, hands reaching up).
    Climb,
}

impl Anim {
    pub fn frames(self) -> u8 {
        match self {
            Anim::Idle => 4,
            Anim::Run => 8,
            Anim::JumpUp | Anim::Fall => 2,
            Anim::Attack(_) => 4,
            Anim::Dodge => 2,
            Anim::Hurt | Anim::Dead => 1,
            Anim::Talk => 4,
            Anim::Climb => 4,
        }
    }
    pub fn fps(self) -> f32 {
        match self {
            Anim::Idle => 4.0,
            Anim::Run => 12.0,
            Anim::Talk => 5.0,
            _ => 8.0,
        }
    }
}

/// Which body motion each attack uses, by class.
pub fn motion_for(class: ClassId, kind: AttackKind) -> Motion {
    use AttackKind::*;
    let w = class.info().weapon;
    let ranged = matches!(w, Weapon::Bow | Weapon::Staff);
    let ranged_motion = if w == Weapon::Bow { Motion::Shoot } else { Motion::Cast };
    match kind {
        LightNeutral => Motion::Jab,
        LightSide => if ranged { ranged_motion } else { Motion::Thrust },
        LightDown => Motion::Sweep,
        AirNeutral => Motion::Spin,
        AirSide => if ranged { ranged_motion } else { Motion::Thrust },
        AirDown => Motion::Spike,
        HeavyNeutral => match class {
            ClassId::Duskblade => Motion::Throw,
            ClassId::Vanguard | ClassId::Stormfist | ClassId::Novice => Motion::Uppercut,
            _ => ranged_motion,
        },
        HeavySide => match class {
            ClassId::Stormfist => Motion::Kick,
            ClassId::Vanguard | ClassId::Duskblade | ClassId::Novice => Motion::Thrust,
            _ => ranged_motion,
        },
        HeavyDown => match class {
            ClassId::Duskblade => Motion::Throw,
            ClassId::Windrunner => Motion::Shoot,
            ClassId::Aethermancer => Motion::Cast,
            _ => Motion::Slam,
        },
        AirHeavyNeutral => Motion::Rise,
        AirHeavyDown => if ranged { Motion::Cast } else { Motion::Dive },
    }
}

// =================================================================== pose data

/// Which foot (or none) holds the body up.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Plant {
    /// Lowest foot touches the ground (standing, running).
    Lowest,
    /// Back foot is the standing leg (kicks, sweeps).
    Back,
    /// In the air: pelvis at a fixed height above the ground line.
    Air,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum SwooshAt {
    Shoulder,
    Hip,
}

/// Angles in degrees. Limbs: 0 = pointing straight down, positive = rotated
/// toward the facing direction. Legs are absolute; arms are relative to the
/// torso lean. `knee`/`elbow` are bend amounts (always >= 0).
#[derive(Clone, Copy, Debug)]
struct Pose {
    lean: f32,
    lift: f32,
    plant: Plant,
    fl: (f32, f32),
    bl: (f32, f32),
    fa: (f32, f32),
    ba: (f32, f32),
    /// Weapon angle relative to the forearm.
    w: f32,
    /// Back-hand weapon angle relative to the back forearm.
    bw: f32,
    hair_trail: f32,
    hair_lift: f32,
    blink: bool,
    mouth: bool,
    hurt: bool,
    swoosh: Option<(f32, f32, SwooshAt)>,
    foot_front: Option<f32>,
}

impl Pose {
    fn new() -> Pose {
        Pose {
            lean: 2.0,
            lift: 0.0,
            plant: Plant::Lowest,
            fl: (6.0, 6.0),
            bl: (-5.0, 4.0),
            fa: (6.0, 14.0),
            ba: (-4.0, 10.0),
            w: 60.0,
            bw: 60.0,
            hair_trail: 0.0,
            hair_lift: 0.0,
            blink: false,
            mouth: false,
            hurt: false,
            swoosh: None,
            foot_front: None,
        }
    }
    fn legs(mut self, fl: (f32, f32), bl: (f32, f32)) -> Pose {
        self.fl = fl;
        self.bl = bl;
        self
    }
    fn arms(mut self, fa: (f32, f32), ba: (f32, f32)) -> Pose {
        self.fa = fa;
        self.ba = ba;
        self
    }
    fn lean(mut self, l: f32) -> Pose {
        self.lean = l;
        self
    }
}

fn guard_stance() -> Pose {
    Pose::new().lean(5.0).legs((14.0, 10.0), (-14.0, 12.0)).arms((28.0, 132.0), (42.0, 128.0))
}

fn uses_guard(weapon: Option<Weapon>) -> bool {
    matches!(weapon, Some(Weapon::Gauntlets) | Some(Weapon::TwinDaggers))
}

fn pose_for(anim: Anim, frame: u8, weapon: Option<Weapon>) -> Pose {
    let f = frame as usize;
    match anim {
        Anim::Idle => {
            let dip = if f == 1 || f == 2 { 6.0 } else { 0.0 };
            let breathe = if f == 1 || f == 2 { 4.0 } else { 0.0 };
            if uses_guard(weapon) {
                let mut p = guard_stance();
                p.fl.1 += dip;
                p.bl.1 += dip;
                p.fa.1 -= breathe;
                p
            } else {
                let mut p = Pose::new();
                p.fl.1 += dip;
                p.bl.1 += dip;
                match weapon {
                    Some(Weapon::SwordShield) | Some(Weapon::WoodSword) => {
                        p.fa = (12.0, 38.0 + breathe);
                        p.ba = (18.0, 60.0);
                        p.w = 60.0;
                    }
                    Some(Weapon::Bow) => {
                        p.fa = (8.0, 24.0 + breathe);
                        p.w = 0.0;
                    }
                    Some(Weapon::Staff) => {
                        p.fa = (14.0, 62.0 + breathe);
                        p.w = 100.0;
                    }
                    _ => {
                        p.fa.1 += breathe;
                        p.ba.1 += breathe;
                    }
                }
                p
            }
        }
        Anim::Talk => {
            let mut p = Pose::new();
            p.fa = (24.0, 55.0 + (f % 2) as f32 * 25.0);
            p.mouth = f % 2 == 0;
            p
        }
        Anim::Run => {
            // Run cycle keyed from the reference sheets: deep knee lift,
            // straight push-off leg, fists up at chest height, forward lean.
            const T: [f32; 8] = [35.0, 22.0, 5.0, -15.0, -32.0, -18.0, 15.0, 40.0];
            const K: [f32; 8] = [12.0, 22.0, 25.0, 35.0, 55.0, 105.0, 100.0, 45.0];
            let b = (f + 4) % 8;
            let mut p = Pose::new().lean(14.0).legs((T[f], K[f]), (T[b], K[b]));
            p.fa = (-T[f] * 0.9 + 12.0, 100.0);
            p.ba = (-T[b] * 0.9 + 12.0, 100.0);
            p.lift = if f == 3 || f == 7 { 1.5 } else { 0.0 };
            p.hair_trail = 1.0;
            p.hair_lift = if f % 4 == 3 { 0.25 } else { 0.1 };
            p.w = 40.0;
            p
        }
        Anim::JumpUp => {
            let mut p = if f == 0 {
                Pose::new().lean(6.0).legs((55.0, 95.0), (-5.0, 50.0)).arms((60.0, 80.0), (-35.0, 60.0))
            } else {
                Pose::new().lean(8.0).legs((70.0, 115.0), (35.0, 105.0)).arms((40.0, 115.0), (20.0, 110.0))
            };
            p.plant = Plant::Air;
            p.hair_trail = 0.2;
            p.hair_lift = if f == 0 { -0.4 } else { 0.3 };
            p
        }
        Anim::Fall => {
            let mut p = if f == 0 {
                Pose::new().lean(4.0).legs((25.0, 45.0), (-15.0, 35.0)).arms((80.0, 30.0), (-70.0, 25.0))
            } else {
                Pose::new().lean(2.0).legs((12.0, 22.0), (-10.0, 15.0)).arms((100.0, 20.0), (-90.0, 20.0))
            };
            p.plant = Plant::Air;
            p.hair_trail = 0.3;
            p.hair_lift = 0.8 + f as f32 * 0.2;
            p
        }
        Anim::Dodge => {
            let mut p = if f == 0 {
                Pose::new().lean(28.0).legs((55.0, 85.0), (-35.0, 40.0)).arms((-50.0, 40.0), (-70.0, 30.0))
            } else {
                Pose::new().lean(22.0).legs((40.0, 70.0), (-45.0, 30.0)).arms((-40.0, 50.0), (-60.0, 40.0))
            };
            p.hair_trail = 1.2;
            p.blink = f == 1;
            p
        }
        Anim::Hurt => {
            let mut p = Pose::new().lean(-14.0).legs((10.0, 18.0), (-14.0, 10.0)).arms((-50.0, 40.0), (50.0, 30.0));
            p.hurt = true;
            p.hair_lift = 0.4;
            p.hair_trail = -0.3;
            p
        }
        Anim::Dead => Pose::new(),
        Anim::Climb => {
            // Alternate hands reaching up and knees stepping, like a ladder climb.
            let up = f % 2 == 0;
            let k = if f >= 2 { 1.0 } else { 0.0 };
            let mut p = Pose::new().lean(4.0);
            p.plant = Plant::Air;
            p.fa = if up { (138.0, 12.0) } else { (100.0, 45.0) };
            p.ba = if up { (100.0, 45.0) } else { (138.0, 12.0) };
            p.fl = if up { (55.0, 95.0 - k * 10.0) } else { (20.0, 40.0) };
            p.bl = if up { (15.0, 35.0) } else { (50.0, 90.0 - k * 10.0) };
            p.w = 120.0;
            p.bw = 120.0;
            p.hair_lift = 0.1;
            p
        }
        Anim::Attack(m) => attack_pose(m, f, weapon),
    }
}

fn attack_pose(m: Motion, f: usize, weapon: Option<Weapon>) -> Pose {
    let mut p = guard_stance();
    p.mouth = f == 1;
    let strike = f == 1 || f == 2;
    // Weapons line up with the forearm when striking, angled when winding up.
    p.w = if strike { 0.0 } else { 50.0 };
    p.bw = 30.0;
    match m {
        Motion::Jab | Motion::Throw => {
            let wind = if m == Motion::Throw { (-50.0, 80.0) } else { (20.0, 140.0) };
            match f {
                0 => p = p.lean(-2.0).arms(wind, (42.0, 128.0)),
                1 => {
                    p = p.lean(10.0).legs((20.0, 8.0), (-20.0, 10.0)).arms((84.0, 4.0), (40.0, 130.0));
                    p.swoosh = Some((60.0, 100.0, SwooshAt::Shoulder));
                }
                2 => p = p.lean(10.0).legs((20.0, 8.0), (-20.0, 10.0)).arms((80.0, 10.0), (40.0, 130.0)),
                _ => p = p.lean(6.0).arms((45.0, 90.0), (42.0, 128.0)),
            }
        }
        Motion::Thrust => match f {
            0 => p = p.lean(-2.0).legs((10.0, 12.0), (-10.0, 10.0)).arms((10.0, 100.0), (40.0, 120.0)),
            1 | 2 => {
                p = p.lean(16.0).legs((42.0, 40.0), (-38.0, 4.0)).arms((88.0 - f as f32 * 2.0, 2.0 + f as f32 * 2.0), (-30.0, 30.0));
                if f == 1 {
                    p.swoosh = Some((60.0, 110.0, SwooshAt::Shoulder));
                }
            }
            _ => p = p.lean(8.0).legs((25.0, 25.0), (-22.0, 8.0)).arms((55.0, 60.0), (20.0, 90.0)),
        },
        Motion::Sweep => match f {
            0 => p = p.lean(10.0).legs((35.0, 70.0), (-15.0, 60.0)).arms((50.0, 70.0), (20.0, 90.0)),
            1 | 2 => {
                p = p.lean(20.0).legs((82.0 - f as f32 * 3.0, 0.0), (-5.0, 115.0)).arms((40.0, 60.0), (-50.0, 40.0));
                p.plant = Plant::Back;
                if f == 1 {
                    p.swoosh = Some((40.0, 100.0, SwooshAt::Hip));
                }
            }
            _ => p = p.lean(10.0).legs((40.0, 60.0), (-10.0, 70.0)).arms((45.0, 80.0), (30.0, 100.0)),
        },
        Motion::Uppercut => match f {
            0 => p = p.lean(12.0).legs((30.0, 55.0), (-20.0, 55.0)).arms((-10.0, 120.0), (40.0, 120.0)),
            1 | 2 => {
                p = p.lean(-6.0).legs((15.0, 10.0), (-20.0, 30.0)).arms((165.0 + f as f32 * 3.0, 10.0), (30.0, 100.0));
                p.lift = 3.0;
                p.hair_lift = 0.5;
                if f == 1 {
                    p.swoosh = Some((90.0, 175.0, SwooshAt::Shoulder));
                }
            }
            _ => p = p.lean(2.0).arms((100.0, 60.0), (40.0, 120.0)),
        },
        Motion::Spin => {
            p.plant = Plant::Air;
            p = p.legs((60.0, 100.0), (20.0, 100.0));
            let (a, b) = [(90.0, -90.0), (120.0, -60.0), (150.0, -30.0), (60.0, -40.0)][f.min(3)];
            p.fa = (a, if f == 3 { 40.0 } else { 0.0 });
            p.ba = (b, if f == 3 { 40.0 } else { 0.0 });
            if f == 1 {
                p.swoosh = Some((30.0, 170.0, SwooshAt::Shoulder));
            }
        }
        Motion::Spike => {
            p.plant = Plant::Air;
            p = p.legs((55.0, 100.0), (25.0, 90.0));
            match f {
                0 => p = p.lean(-6.0).arms((170.0, 10.0), (150.0, 20.0)),
                1 | 2 => {
                    p = p.lean(14.0).arms((50.0 - f as f32 * 7.0, 0.0), (30.0, 10.0));
                    if f == 1 {
                        p.swoosh = Some((40.0, 170.0, SwooshAt::Shoulder));
                    }
                }
                _ => p = p.lean(6.0).arms((60.0, 60.0), (40.0, 60.0)),
            }
        }
        Motion::Slam => match f {
            0 => p = p.lean(-10.0).legs((12.0, 10.0), (-12.0, 8.0)).arms((175.0, 15.0), (160.0, 20.0)),
            1 | 2 => {
                p = p.lean(18.0 + f as f32).legs((30.0, 35.0), (-30.0, 10.0)).arms((60.0 - f as f32 * 7.0, 0.0), (40.0, 10.0));
                if f == 1 {
                    p.swoosh = Some((40.0, 175.0, SwooshAt::Shoulder));
                }
            }
            _ => p = p.lean(8.0).arms((70.0, 50.0), (40.0, 80.0)),
        },
        Motion::Kick => {
            // High kick from the reference sheets: chamber, extend, hold, re-chamber.
            p.plant = Plant::Back;
            p.w = 50.0;
            match f {
                0 => p = p.lean(-2.0).legs((95.0, 130.0), (-6.0, 6.0)),
                1 | 2 => {
                    p = p.lean(-20.0 + f as f32).legs((128.0 - f as f32 * 5.0, 0.0), (-12.0, 4.0)).arms((20.0, 130.0), (50.0, 120.0));
                    p.foot_front = Some(-15.0);
                    if f == 1 {
                        p.swoosh = Some((100.0, 140.0, SwooshAt::Hip));
                    }
                }
                _ => p = p.lean(-2.0).legs((80.0, 115.0), (-6.0, 6.0)),
            }
        }
        Motion::Cast => {
            p = p.legs((12.0, 8.0), (-12.0, 8.0));
            p.w = 0.0;
            match f {
                0 => p = p.lean(-4.0).arms((150.0, 30.0), (-20.0, 30.0)),
                1 | 2 => p = p.lean(8.0).arms((88.0, 0.0), (-40.0, 20.0)),
                _ => p = p.lean(4.0).arms((60.0, 30.0), (-20.0, 30.0)),
            }
        }
        Motion::Shoot => {
            p = p.lean(2.0).legs((16.0, 8.0), (-16.0, 8.0));
            p.w = 0.0;
            let ba = [(70.0, 60.0), (80.0, 150.0), (75.0, 155.0), (-20.0, 30.0)][f.min(3)];
            p = p.arms((90.0, 0.0), ba);
        }
        Motion::Dive => {
            p.plant = Plant::Air;
            p = p.lean(30.0).legs((75.0, 0.0), (-20.0, 100.0)).arms((-40.0, 30.0), (-60.0, 30.0));
            p.foot_front = Some(10.0);
            p.hair_lift = 1.0;
            if f == 1 {
                p.swoosh = Some((40.0, 90.0, SwooshAt::Hip));
            }
        }
        Motion::Rise => {
            p.plant = Plant::Air;
            p = p.lean(-4.0).legs((60.0, 110.0), (20.0, 100.0)).arms((175.0, 5.0), (130.0, 30.0));
            p.hair_lift = -0.6;
            p.w = 0.0;
            if f <= 1 {
                p.swoosh = Some((100.0, 180.0, SwooshAt::Shoulder));
            }
        }
    }
    if weapon == Some(Weapon::Bow) && m != Motion::Shoot {
        p.w = 0.0;
    }
    p
}

// =================================================================== skeleton

type P2 = (f32, f32);

fn dir(deg: f32) -> P2 {
    let r = deg.to_radians();
    (r.sin(), r.cos())
}
fn add(a: P2, d: P2, l: f32) -> P2 {
    (a.0 + d.0 * l, a.1 + d.1 * l)
}
fn lerp(a: P2, b: P2, t: f32) -> P2 {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

#[derive(Clone, Copy)]
struct Leg {
    hip: P2,
    knee: P2,
    ankle: P2,
    thigh_deg: f32,
    shin_deg: f32,
    foot_deg: f32,
}

#[derive(Clone, Copy)]
struct Arm {
    sh: P2,
    elbow: P2,
    hand: P2,
    fore_deg: f32,
}

struct Rig {
    pelvis: P2,
    lean: f32,
    head: P2,
    fl: Leg,
    bl: Leg,
    fa: Arm,
    ba: Arm,
    fem: bool,
}

impl Rig {
    /// Torso-local point (x forward, y up from the pelvis) to sprite pixels.
    fn t(&self, x: f32, y: f32) -> P2 {
        let r = self.lean.to_radians();
        let up = (r.sin(), -r.cos());
        let fwd = (r.cos(), r.sin());
        (self.pelvis.0 + fwd.0 * x + up.0 * y, self.pelvis.1 + fwd.1 * x + up.1 * y)
    }
}

fn build_leg(hip: P2, (thigh, knee): (f32, f32), foot_override: Option<f32>) -> Leg {
    let knee_bend = knee.clamp(0.0, 160.0);
    let k = add(hip, dir(thigh), THIGH);
    let shin_deg = thigh - knee_bend;
    let a = add(k, dir(shin_deg), SHIN);
    let auto = if shin_deg < 0.0 { -shin_deg * 0.75 } else { -shin_deg * 0.25 };
    Leg { hip, knee: k, ankle: a, thigh_deg: thigh, shin_deg, foot_deg: foot_override.unwrap_or(auto.clamp(-25.0, 75.0)) }
}

fn build_arm(sh: P2, lean: f32, (upper, elbow): (f32, f32)) -> Arm {
    let ua = lean + upper;
    let fa = ua + elbow.clamp(0.0, 165.0);
    let e = add(sh, dir(ua), UPPER);
    let h = add(e, dir(fa), FORE);
    Arm { sh, elbow: e, hand: h, fore_deg: fa }
}

fn build_rig(p: &Pose, fem: bool) -> Rig {
    // Legs relative to a pelvis at the origin, then plant on the ground.
    let fl0 = build_leg((1.2, 0.0), p.fl, p.foot_front);
    let bl0 = build_leg((-1.2, 0.0), p.bl, None);
    let ground = FEET_Y as f32;
    // Lowest point of a leg: the sole, or the knee when kneeling.
    let drop_of = |l: &Leg| (l.ankle.1 + ANKLE_H * (l.foot_deg.to_radians().cos()).max(0.3)).max(l.knee.1 + 2.5);
    let py = match p.plant {
        Plant::Lowest => ground - drop_of(&fl0).max(drop_of(&bl0)),
        Plant::Back => ground - drop_of(&bl0),
        Plant::Air => ground - (THIGH + SHIN + ANKLE_H),
    } - p.lift;
    let pelvis = (FEET_X as f32, py);
    let shift = |l: Leg| Leg {
        hip: (l.hip.0 + pelvis.0, l.hip.1 + pelvis.1),
        knee: (l.knee.0 + pelvis.0, l.knee.1 + pelvis.1),
        ankle: (l.ankle.0 + pelvis.0, l.ankle.1 + pelvis.1),
        ..l
    };
    let mut rig = Rig {
        pelvis,
        lean: p.lean,
        head: (0.0, 0.0),
        fl: shift(fl0),
        bl: shift(bl0),
        fa: Arm { sh: (0.0, 0.0), elbow: (0.0, 0.0), hand: (0.0, 0.0), fore_deg: 0.0 },
        ba: Arm { sh: (0.0, 0.0), elbow: (0.0, 0.0), hand: (0.0, 0.0), fore_deg: 0.0 },
        fem,
    };
    let sw = if fem { 0.0 } else { 0.6 };
    let sh_f = rig.t(1.5 + sw * 0.5, 13.0);
    let sh_b = rig.t(-2.6 - sw, 13.4);
    rig.fa = build_arm(sh_f, p.lean, p.fa);
    rig.ba = build_arm(sh_b, p.lean, p.ba);
    rig.head = rig.t(1.4, HEAD_Y);
    rig
}

// =================================================================== palette

struct Pal {
    skin: Px,
    hair: Px,
    top: Px,
    bottom: Px,
    eye: Px,
    under: Px,
    under_trim: Px,
}

fn palette(look: &Appearance) -> Pal {
    let hair = from3(HAIR_COLORS[look.hair_color as usize].1);
    let fem = look.feminine();
    Pal {
        skin: from3(SKIN_TONES[look.skin as usize].1),
        hair,
        top: from3(DYES[look.top_dye as usize].1),
        bottom: from3(DYES[look.bottom_dye as usize].1),
        eye: mix(shadow(hair, 0.3), rgb(74, 46, 92), 0.45),
        under: if fem { rgb(222, 216, 242) } else { rgb(120, 140, 184) },
        under_trim: if fem { rgb(224, 120, 156) } else { rgb(236, 238, 246) },
    }
}

/// Three-tone cel shading of one part layer: dark band on the back/lower
/// edges, a highlight on the top/front edge (light from the upper front).
fn cel(c: &mut Canvas) {
    let src = c.clone();
    for y in 0..c.h {
        for x in 0..c.w {
            let p = src.get(x, y);
            if p[3] == 0 {
                continue;
            }
            let open = |dx: i32, dy: i32| !src.opaque(x + dx, y + dy);
            let i = (y * c.w + x) as usize;
            if open(-1, 0) || open(0, 1) || open(-1, 1) {
                c.px[i] = shadow(p, 0.32);
            } else if open(-2, 0) || open(0, 2) {
                c.px[i] = shadow(p, 0.16);
            } else if open(0, -1) || open(1, -1) {
                c.px[i] = light(p, 0.22);
            }
        }
    }
}

/// Draw `part` with shading and an ink outline onto `out`.
fn stamp(out: &mut Canvas, mut part: Canvas, dim: f32) {
    if dim > 0.0 {
        for p in part.px.iter_mut() {
            if p[3] > 0 {
                *p = shadow(*p, dim);
            }
        }
    }
    cel(&mut part);
    out.draw(&part.outlined(INK), 0, 0);
}

/// A clothing segment: rounded at the joint end, flat at the hem.
fn hem_seg(c: &mut Canvas, a: P2, b: P2, wa: f32, wb: f32, col: Px) {
    c.taper(a, b, wa, wb, col);
    c.disc(a.0, a.1, wa * 0.5, col);
}

/// A tapered limb segment with rounded joints.
fn seg(c: &mut Canvas, a: P2, b: P2, wa: f32, wb: f32, col: Px) {
    c.taper(a, b, wa, wb, col);
    c.disc(a.0, a.1, wa * 0.5, col);
    c.disc(b.0, b.1, wb * 0.5, col);
}

// =================================================================== main entry

pub fn render(look: &Appearance, weapon: Option<Weapon>, anim: Anim, frame: u8) -> Canvas {
    let look = look.sanitized();
    let pal = palette(&look);
    if anim == Anim::Dead {
        return render_dead(&look, &pal);
    }
    let p = pose_for(anim, frame % anim.frames(), weapon);
    let r = build_rig(&p, look.feminine());
    let mut out = Canvas::new(SIZE, HEIGHT);
    let new = || Canvas::new(SIZE, HEIGHT);

    // 1. Hair behind the body
    {
        let mut c = new();
        back_hair(&mut c, &look, &r, &pal, &p);
        if look.top == 3 {
            // hood resting on the back
            let n = r.t(-3.5, 15.5);
            c.ellipse(n.0, n.1, 4.0, 3.0, shadow(pal.top, 0.15));
        }
        if look.top == 8 {
            // satchel on the back hip, strap drawn over the shirt
            let bag = r.t(-6.4, 0.6);
            c.rect((bag.0 - 3.5).round() as i32, (bag.1 - 3.0).round() as i32, 7, 6, rgb(150, 100, 58));
            c.rect((bag.0 - 3.5).round() as i32, (bag.1 - 3.0).round() as i32, 7, 2, rgb(120, 78, 44));
        }
        if look.top == 6 {
            let scarf = scarf_color(&look);
            let n = r.t(-2.0, 15.0);
            let tail = (n.0 - 8.0 - p.hair_trail * 4.0, n.1 + 4.0 - p.hair_lift * 5.0);
            c.taper(n, tail, 4.0, 2.0, scarf);
        }
        stamp(&mut out, c, 0.0);
    }
    // 2. Back arm (far side, dimmed)
    {
        let mut c = new();
        draw_arm(&mut c, &look, &pal, &r.ba, true);
        if weapon == Some(Weapon::TwinDaggers) {
            dagger(&mut c, r.ba.hand, dir(r.ba.fore_deg + p.bw));
        }
        if weapon == Some(Weapon::Gauntlets) {
            gauntlet(&mut c, r.ba.hand);
        }
        stamp(&mut out, c, 0.18);
    }
    // 3. Back leg
    {
        let mut c = new();
        draw_leg(&mut c, &look, &pal, &r.bl);
        stamp(&mut out, c, 0.18);
    }
    // 4. Torso (body, underwear, top, waistband)
    {
        let mut c = new();
        draw_torso(&mut c, &look, &pal, &r);
        stamp(&mut out, c, 0.0);
    }
    // 5. Front leg
    {
        let mut c = new();
        draw_leg(&mut c, &look, &pal, &r.fl);
        stamp(&mut out, c, 0.0);
    }
    // 6. Skirts / robe hems hang over both legs and follow them
    if let Some(c) = draw_skirt(&look, &pal, &r) {
        stamp(&mut out, c, 0.0);
    }
    // 7. Head + hair (one outline), then the face
    {
        let mut c = new();
        draw_head(&mut c, &pal, r.head);
        front_hair(&mut c, &look, &pal, &r, &p);
        stamp(&mut out, c, 0.0);
        draw_face(&mut out, &pal, look.face, r.head, &p);
        hair_shine(&mut out, &look, &pal, r.head);
    }
    // 8. Shield on the back arm sits in front of the body
    if weapon == Some(Weapon::SwordShield) {
        let mut c = new();
        let h = r.ba.hand;
        shield(&mut c, (h.0 + 2.0, h.1 - 1.0));
        stamp(&mut out, c, 0.0);
    }
    // 9. Front arm + weapon
    {
        let wd = dir(r.fa.fore_deg + p.w);
        let mut wc = new();
        if let Some(w) = weapon {
            draw_weapon(&mut wc, w, &r.fa, wd, &r.ba, &p);
        }
        let mut wcs = wc.clone();
        cel(&mut wcs);
        let weapon_layer = wcs.outlined(INK);
        let behind = weapon == Some(Weapon::Bow);
        if behind {
            out.draw(&weapon_layer, 0, 0);
        }
        let mut c = new();
        draw_arm(&mut c, &look, &pal, &r.fa, false);
        if weapon == Some(Weapon::Gauntlets) {
            gauntlet(&mut c, r.fa.hand);
        }
        stamp(&mut out, c, 0.0);
        if !behind {
            out.draw(&weapon_layer, 0, 0);
        }
    }
    // 10. Motion swoosh
    if let Some((a0, a1, at)) = p.swoosh {
        let (center, reach) = match at {
            SwooshAt::Shoulder => (
                r.fa.sh,
                match weapon {
                    Some(Weapon::SwordShield) | Some(Weapon::Staff) => 30.0,
                    Some(Weapon::TwinDaggers) => 24.0,
                    _ => 19.0,
                },
            ),
            SwooshAt::Hip => (r.fl.hip, 27.0),
        };
        let steps = 18;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let a = a0 + (a1 - a0) * t;
            let pnt = add(center, dir(a), reach);
            let w = 0.6 + 1.6 * t;
            out.disc(pnt.0, pnt.1, w, [255, 255, 255, (110.0 + 130.0 * t) as u8]);
        }
    }
    out
}

fn scarf_color(look: &Appearance) -> Px {
    from3(DYES[(look.top_dye as usize + 2) % DYES.len()].1)
}

// =================================================================== limbs

/// Sleeve coverage for each top: (upper arm fraction, forearm fraction, extra width).
fn sleeve(top: u8) -> (f32, f32, f32) {
    match top {
        1 => (0.75, 0.0, 0.8),  // tunic: short sleeves
        2 => (0.55, 0.0, 0.6),  // vest over a tee
        3 => (1.0, 0.92, 1.8),  // hoodie: baggy long sleeves
        4 => (1.0, 1.0, 2.2),   // robe: wide long sleeves
        5 => (1.0, 0.9, 0.6),   // breastplate undershirt
        6 => (1.0, 0.9, 1.0),   // scarf jacket
        8 => (0.6, 0.0, 0.7),   // adventurer's garb: short linen sleeves
        _ => (0.0, 0.0, 0.0),   // none / crop top
    }
}

fn draw_arm(c: &mut Canvas, look: &Appearance, pal: &Pal, a: &Arm, _back: bool) {
    let skin = pal.skin;
    let (w_sh, w_el, w_wr) = if look.feminine() { (3.6, 3.1, 2.6) } else { (4.2, 3.6, 3.0) };
    seg(c, a.sh, a.elbow, w_sh, w_el, skin);
    seg(c, a.elbow, a.hand, w_el, w_wr, skin);
    // sleeves follow the bones
    let (uf, ff, extra) = sleeve(look.top);
    let cloth = if look.top == 2 { rgb(238, 232, 218) } else { pal.top };
    if uf > 0.0 {
        let end = lerp(a.sh, a.elbow, uf);
        if uf >= 1.0 {
            seg(c, a.sh, end, w_sh + extra, w_el + extra, cloth);
        } else {
            hem_seg(c, a.sh, end, w_sh + extra, w_sh + extra * 0.8 - (1.0 - uf), cloth);
        }
    }
    if ff > 0.0 {
        let end = lerp(a.elbow, a.hand, ff);
        let flare = if look.top == 4 { 2.0 } else { 0.0 };
        hem_seg(c, a.elbow, end, w_el + extra, w_wr + extra + flare, cloth);
        if look.top == 3 {
            // ribbed cuff
            let cuff = lerp(a.elbow, a.hand, ff);
            c.disc(cuff.0, cuff.1, (w_wr + extra) * 0.5, shadow(cloth, 0.12));
        }
    }
    if look.top == 5 {
        // steel pauldron
        c.ellipse(a.sh.0, a.sh.1 + 0.5, 3.6, 2.8, rgb(168, 178, 196));
    }
    // fist
    let fist = add(a.hand, dir(a.fore_deg), 0.8);
    c.disc(fist.0, fist.1, 2.4, skin);
}

/// Trouser coverage: (thigh fraction, shin fraction, extra width, colour factor).
fn pant(bottom: u8) -> Option<(f32, f32, f32)> {
    match bottom {
        1 => Some((1.0, 0.85, 1.0)), // trousers
        2 => Some((0.72, 0.0, 1.0)), // ranger shorts
        4 => Some((1.0, 0.85, 1.6)), // cargo pants
        5 => Some((1.0, 0.95, 3.2)), // hakama (very wide)
        6 => Some((1.0, 0.92, 0.3)), // leggings
        7 => Some((1.08, 0.0, 2.6)), // baggy shorts past the knee
        _ => None,                   // none / skirt
    }
}

/// How far up the shin each shoe's shaft reaches (fraction of the shin from the ankle).
fn shoe_shaft(shoes: u8) -> f32 {
    match shoes {
        0 => 0.32,
        1 => 0.1,
        3 => 0.55,
        5 => 0.6,
        6 => 0.26,
        _ => 0.0,
    }
}

fn draw_leg(c: &mut Canvas, look: &Appearance, pal: &Pal, l: &Leg) {
    let skin = pal.skin;
    let fem = look.feminine();
    let (w_hip, w_knee, w_calf, w_ank) = if fem { (6.4, 4.2, 4.6, 3.0) } else { (6.0, 4.5, 5.0, 3.2) };
    seg(c, l.hip, l.knee, w_hip, w_knee, skin);
    // calf bulge on the back of the shin
    let calf = add(lerp(l.knee, l.ankle, 0.32), dir(l.shin_deg - 90.0), 0.6);
    c.taper(l.knee, calf, w_knee, w_calf, skin);
    seg(c, calf, l.ankle, w_calf, w_ank, skin);
    // underwear on the upper thigh
    if !fem {
        let end = lerp(l.hip, l.knee, 0.4);
        hem_seg(c, l.hip, end, w_hip + 0.8, w_hip + 0.4, pal.under);
    }
    // trousers / shorts follow the bones
    if let Some((tf, sf, extra)) = pant(look.bottom) {
        let col = if look.bottom == 6 { shadow(pal.bottom, 0.45) } else { pal.bottom };
        let te = lerp(l.hip, l.knee, tf.min(1.0));
        if tf >= 1.0 {
            seg(c, l.hip, te, w_hip + extra, w_knee + extra, col);
        } else {
            hem_seg(c, l.hip, te, w_hip + extra, w_knee + extra + 0.6, col);
        }
        if tf > 1.0 {
            let past = lerp(l.knee, l.ankle, tf - 1.0);
            hem_seg(c, l.knee, past, w_knee + extra, w_calf + extra, col);
        }
        if sf > 0.0 {
            let se = lerp(l.knee, l.ankle, sf);
            hem_seg(c, l.knee, se, w_knee + extra, w_ank + extra, col);
        }
        if look.bottom == 4 {
            let pk = lerp(l.hip, l.knee, 0.55);
            c.rect(pk.0.round() as i32 - 1, pk.1.round() as i32 - 1, 3, 3, shadow(col, 0.25));
        }

    }
    draw_shoe(c, look.shoes, l);
}

fn draw_shoe(c: &mut Canvas, style: u8, l: &Leg) {
    let f = l.foot_deg.to_radians();
    let t = (f.cos(), f.sin()); // toe direction
    let u = (f.sin(), -f.cos()); // foot "up"
    let o = (l.ankle.0 - u.0 * ANKLE_H, l.ankle.1 - u.1 * ANKLE_H); // sole under the ankle
    let pt = |x: f32, y: f32| (o.0 + t.0 * x + u.0 * y, o.1 + t.1 * x + u.1 * y);
    // shaft up the shin
    let shaft = shoe_shaft(style);
    let (upper, sole, accent): (Px, Px, Px) = match style {
        0 => (rgb(132, 82, 46), rgb(92, 56, 32), rgb(186, 130, 80)),
        1 => (rgb(232, 232, 240), rgb(250, 250, 252), rgb(214, 64, 72)),
        2 => (rgb(196, 140, 92), rgb(140, 96, 56), rgb(140, 96, 56)),
        3 => (rgb(168, 176, 194), rgb(120, 128, 146), rgb(232, 238, 248)),
        4 => (rgb(244, 184, 200), rgb(250, 222, 230), rgb(255, 255, 255)),
        5 => (rgb(56, 46, 52), rgb(36, 30, 34), rgb(206, 166, 82)),
        _ => (rgb(186, 44, 56), rgb(252, 252, 252), rgb(255, 255, 255)),
    };
    if shaft > 0.0 {
        let top = lerp(l.ankle, l.knee, shaft);
        seg(c, l.ankle, top, 4.2, 4.4, upper);
        if style == 6 || style == 1 {
            // sock peeking out of sneakers
            let s = lerp(l.ankle, l.knee, shaft + 0.06);
            c.disc(s.0, s.1, 2.0, rgb(246, 246, 250));
        }
    }
    match style {
        2 => {
            // sandals: bare foot with straps
            let foot = [pt(-2.5, 0.0), pt(5.5, 0.0), pt(6.0, 1.5), pt(3.0, 2.8), pt(-2.0, 3.2)];
            c.poly(&foot, rgb(0, 0, 0));
            c.poly(&foot, upper);
            c.poly(&[pt(-3.0, -0.5), pt(6.5, -0.5), pt(6.5, 0.6), pt(-3.0, 0.6)], sole);
        }
        4 => {
            c.poly(&[pt(-3.0, 0.0), pt(6.5, 0.0), pt(7.0, 2.0), pt(4.0, 3.6), pt(-2.5, 4.0)], upper);
            let pom = pt(5.0, 3.2);
            c.disc(pom.0, pom.1, 1.3, accent);
        }
        _ => {
            // chunky shoe: heel, toe cap, sole stripe, laces
            let body = [pt(-3.2, 0.0), pt(6.8, 0.0), pt(7.4, 1.8), pt(5.6, 3.8), pt(2.0, 4.8), pt(-1.8, 5.4), pt(-3.4, 3.6)];
            c.poly(&body, upper);
            c.poly(&[pt(-3.6, -0.6), pt(7.2, -0.6), pt(7.6, 1.0), pt(-3.6, 1.0)], sole);
            if style == 1 || style == 6 {
                // white toe cap + laces like the reference sneakers
                c.poly(&[pt(4.2, 0.6), pt(7.2, 0.6), pt(7.0, 2.2), pt(5.0, 2.6)], sole);
                for i in 0..3 {
                    let lp = pt(1.0 + i as f32 * 1.3, 3.8 - i as f32 * 0.35);
                    c.set(lp.0.round() as i32, lp.1.round() as i32, accent);
                }
            } else if style == 5 || style == 0 {
                let b = pt(0.0, 4.0);
                c.set(b.0.round() as i32, b.1.round() as i32, accent);
            }
        }
    }
}

// =================================================================== torso & clothing

fn draw_torso(c: &mut Canvas, look: &Appearance, pal: &Pal, r: &Rig) {
    let fem = r.fem;
    let poly = |c: &mut Canvas, pts: &[(f32, f32)], col: Px| {
        let w: Vec<P2> = pts.iter().map(|&(x, y)| r.t(x, y)).collect();
        c.poly(&w, col);
    };
    // neck
    poly(c, &[(-1.6, 13.5), (1.8, 13.5), (1.8, NECK_Y + 3.0), (-1.6, NECK_Y + 3.0)], shadow(pal.skin, 0.1));
    // body
    let body: Vec<(f32, f32)> = if fem {
        vec![(-5.2, -2.0), (4.8, -2.0), (4.6, 1.5), (3.2, 6.0), (4.6, 9.5), (4.4, 12.0), (3.6, 14.5), (-3.2, 14.8), (-4.4, 13.0), (-4.0, 8.0), (-3.6, 5.5), (-4.6, 2.0)]
    } else {
        vec![(-4.6, -2.0), (4.6, -2.0), (4.2, 4.0), (4.0, 7.0), (4.8, 10.5), (4.6, 13.6), (2.0, 15.4), (-3.4, 15.4), (-5.6, 13.6), (-5.2, 8.0), (-4.2, 4.0)]
    };
    poly(c, &body, pal.skin);
    // muscle/collar hints
    if !fem {
        let a = r.t(1.0, 9.5);
        let b = r.t(3.5, 9.0);
        c.linef(a, b, shadow(pal.skin, 0.18));
    }
    let belly = r.t(1.8, 4.0);
    c.set(belly.0.round() as i32, belly.1.round() as i32, shadow(pal.skin, 0.25));

    // underwear (always under clothes)
    if fem {
        poly(c, &[(-5.0, -2.2), (4.8, -2.2), (4.4, 1.2), (1.0, 0.2), (-4.6, 1.4)], pal.under);
        poly(c, &[(-4.8, 1.0), (4.6, 1.0), (4.6, 1.6), (-4.8, 1.6)], pal.under_trim);
        // sports-bra style top
        poly(c, &[(-4.2, 8.2), (4.8, 8.2), (4.8, 12.6), (3.4, 13.4), (-3.8, 13.2)], pal.under);
        poly(c, &[(-4.2, 8.2), (4.8, 8.2), (4.8, 9.0), (-4.2, 9.0)], pal.under_trim);
    } else {
        poly(c, &[(-4.9, -2.4), (4.9, -2.4), (4.6, 1.6), (-4.6, 1.6)], pal.under);
        poly(c, &[(-4.7, 0.6), (4.7, 0.6), (4.7, 1.8), (-4.7, 1.8)], pal.under_trim);
    }
    // bottom waist (trousers/shorts/skirts start here)
    if look.bottom != 0 {
        let col = if look.bottom == 6 { shadow(pal.bottom, 0.45) } else { pal.bottom };
        let wide = match look.bottom {
            5 => 1.6,
            7 => 1.2,
            _ => 0.4,
        };
        poly(c, &[(-5.0 - wide, -3.0), (5.0 + wide, -3.0), (4.8, 2.2), (-4.8, 2.2)], col);
        poly(c, &[(-4.8, 1.4), (4.8, 1.4), (4.8, 2.4), (-4.8, 2.4)], shadow(col, 0.2));
    }
    // top
    let top = pal.top;
    let grow = |pts: &[(f32, f32)], g: f32| -> Vec<(f32, f32)> {
        pts.iter().map(|&(x, y)| (if x > 0.0 { x + g } else { x - g }, y)).collect()
    };
    match look.top {
        1 => {
            // tunic: covers the hips, belt, V neck
            let mut b = grow(&body, 0.6);
            b[0].1 = -4.0;
            b[1].1 = -4.0;
            poly(c, &b, top);
            poly(c, &[(-0.5, 15.2), (2.6, 15.2), (1.2, 11.5)], light(top, 0.3));
            poly(c, &[(-5.0, 0.0), (5.0, 0.0), (5.0, 1.6), (-5.0, 1.6)], rgb(112, 72, 42));
            poly(c, &[(1.4, 0.0), (3.0, 0.0), (3.0, 1.6), (1.4, 1.6)], rgb(228, 196, 90));
        }
        2 => {
            // white tee + open leather vest
            let mut b = grow(&body, 0.4);
            b[0].1 = 1.0;
            b[1].1 = 1.0;
            poly(c, &b, rgb(238, 232, 218));
            poly(c, &[(-5.4, 0.5), (0.4, 0.5), (0.4, 15.3), (-3.6, 15.6), (-6.0, 13.5)], top);
            poly(c, &[(2.8, 0.5), (5.2, 0.5), (5.3, 10.5), (4.8, 13.8), (2.4, 15.2)], top);
        }
        3 => {
            // oversized hoodie: puffy, kangaroo pocket, drawstrings
            let mut b = grow(&body, 1.6);
            b[0].1 = -4.0;
            b[1].1 = -4.0;
            poly(c, &b, top);
            poly(c, &[(-2.6, -2.6), (4.6, -2.6), (4.2, 2.4), (-2.0, 2.4)], shadow(top, 0.14));
            for x in [1.6f32, 3.0] {
                let a = r.t(x, 15.0);
                let b2 = r.t(x, 10.5);
                c.linef(a, b2, rgb(236, 236, 240));
            }
            poly(c, &[(-6.0, -4.0), (6.4, -4.0), (6.4, -2.8), (-6.0, -2.8)], shadow(top, 0.2));
        }
        4 => {
            // robe body (hem drawn as a skirt that follows the knees)
            let b = grow(&body, 0.8);
            poly(c, &b, top);
            poly(c, &[(-5.2, 0.0), (5.2, 0.0), (5.2, 1.6), (-5.2, 1.6)], light(top, 0.4));
            let a = r.t(1.0, 15.0);
            let b2 = r.t(1.0, 1.6);
            c.linef(a, b2, shadow(top, 0.3));
        }
        5 => {
            // undershirt + breastplate
            let b = grow(&body, 0.4);
            poly(c, &b, shadow(top, 0.1));
            poly(c, &[(-4.8, 5.0), (5.2, 5.0), (5.4, 12.0), (4.2, 14.8), (-3.6, 15.0), (-5.4, 12.5)], rgb(170, 180, 198));
            let a = r.t(-2.0, 12.0);
            let b2 = r.t(3.0, 12.0);
            c.linef(a, b2, rgb(232, 238, 248));
            poly(c, &[(-5.0, -1.0), (5.0, -1.0), (5.0, 1.4), (-5.0, 1.4)], rgb(112, 72, 42));
        }
        6 => {
            // jacket + scarf
            let mut b = grow(&body, 0.8);
            b[0].1 = -3.0;
            b[1].1 = -3.0;
            poly(c, &b, top);
            let z1 = r.t(1.2, 14.5);
            let z2 = r.t(1.2, -2.5);
            c.linef(z1, z2, shadow(top, 0.35));
            let scarf = scarf_color(look);
            poly(c, &[(-4.6, 13.0), (4.4, 13.0), (4.2, 16.2), (-4.4, 16.2)], scarf);
        }
        8 => {
            // adventurer's garb: loose linen shirt, crossbody satchel strap, rope belt
            let mut b = grow(&body, 0.5);
            b[0].1 = -3.2;
            b[1].1 = -3.2;
            poly(c, &b, top);
            poly(c, &[(0.0, 15.2), (2.4, 15.2), (1.0, 12.2)], shadow(top, 0.25));
            let strap = rgb(120, 78, 44);
            let s1 = r.t(3.8, 14.6);
            let s2 = r.t(-5.0, -0.5);
            c.linef(s1, s2, strap);
            c.linef((s1.0 - 1.0, s1.1), (s2.0 - 1.0, s2.1), strap);
            poly(c, &[(-5.2, -1.2), (5.2, -1.2), (5.2, 0.4), (-5.2, 0.4)], rgb(196, 162, 108));
            let knot = r.t(2.6, -0.4);
            c.disc(knot.0, knot.1, 1.0, rgb(170, 136, 86));
        }
        7 => {
            // sport crop top (midriff shows, like the reference)
            let b: Vec<(f32, f32)> = body.iter().filter(|p| p.1 >= 7.5).cloned().collect();
            let mut pts = vec![(-4.6, 7.4), (4.8, 7.4)];
            pts.extend(b.iter().filter(|p| p.0 > 0.0).map(|&(x, y)| (x + 0.4, y)));
            pts.extend(b.iter().filter(|p| p.0 <= 0.0).map(|&(x, y)| (x - 0.4, y)));
            poly(c, &pts, top);
            poly(c, &[(-4.6, 7.4), (4.8, 7.4), (4.8, 8.4), (-4.6, 8.4)], shadow(top, 0.2));
        }
        _ => {}
    }
}

/// Skirt-like garments: drawn on their own layer over both legs. Their hem
/// points follow the thighs/knees so they swing with the legs.
fn draw_skirt(look: &Appearance, pal: &Pal, r: &Rig) -> Option<Canvas> {
    let mut c = Canvas::new(SIZE, HEIGHT);
    let fwd_of = |deg: f32| dir(deg + 90.0); // perpendicular, toward the front
    match (look.bottom, look.top) {
        (3, _) => {
            // pleated skirt: hem at ~45% down each thigh
            let col = pal.bottom;
            let wl = r.t(-5.4, 1.5);
            let wr = r.t(5.2, 1.5);
            let hf = add(lerp(r.fl.hip, r.fl.knee, 0.48), fwd_of(r.fl.thigh_deg), 3.6);
            let hb = add(lerp(r.bl.hip, r.bl.knee, 0.48), fwd_of(r.bl.thigh_deg), -3.6);
            let mid = (lerp(hf, hb, 0.5).0, hf.1.max(hb.1) + 0.5);
            c.poly(&[wl, wr, hf, mid, hb], col);
            for i in 1..4 {
                let t = i as f32 / 4.0;
                let top = lerp(wl, wr, t);
                let bot = if t < 0.5 { lerp(hb, mid, t * 2.0) } else { lerp(mid, hf, (t - 0.5) * 2.0) };
                c.linef(lerp(top, bot, 0.35), bot, shadow(col, 0.28));
            }
            Some(c)
        }
        (_, 4) => {
            // robe hem down to the knees
            let col = pal.top;
            let wl = r.t(-5.6, 2.0);
            let wr = r.t(5.6, 2.0);
            let hf = add(r.fl.knee, fwd_of(r.fl.thigh_deg), 3.4);
            let hb = add(r.bl.knee, fwd_of(r.bl.thigh_deg), -3.4);
            let mid = (lerp(hf, hb, 0.5).0, hf.1.max(hb.1) + 1.0);
            c.poly(&[wl, wr, hf, mid, hb], col);
            let s = r.t(1.0, 1.6);
            c.linef(s, mid, shadow(col, 0.3));
            Some(c)
        }
        _ => None,
    }
}

// =================================================================== head, face, hair

fn draw_head(c: &mut Canvas, pal: &Pal, (hx, hy): P2) {
    let s = pal.skin;
    c.ellipse(hx - 0.5, hy - 0.6, 7.4, 7.0, s);
    // 3/4 jaw and chin toward the front
    c.poly(&[(hx - 4.5, hy + 2.0), (hx + 6.6, hy - 0.5), (hx + 7.4, hy + 2.6), (hx + 5.6, hy + 6.4), (hx + 2.8, hy + 7.8), (hx - 1.0, hy + 6.2)], s);
    // nose
    c.set((hx + 7.6).round() as i32, (hy + 2.0).round() as i32, s);
    // ear
    c.ellipse(hx - 2.6, hy + 1.6, 1.6, 2.2, shadow(s, 0.08));
}

fn draw_face(c: &mut Canvas, pal: &Pal, face: u8, (hx, hy): P2, p: &Pose) {
    let x = hx.round() as i32;
    let y = hy.round() as i32 - 1;
    let white = rgb(255, 255, 255);
    let iris = pal.eye;
    let lash = INK;
    let skin_dk = shadow(pal.skin, 0.4);
    let mouth_c = rgb(150, 56, 66);
    let blush = rgb(244, 146, 156);
    // Near eye: 4 wide, lash line on top with a flick at the back.
    let near = |c: &mut Canvas| {
        c.rect(x, y, 5, 1, lash);
        c.set(x - 1, y - 1, lash);
        c.set(x, y + 1, white);
        c.rect(x + 1, y + 1, 3, 3, iris);
        c.rect(x + 2, y + 2, 2, 1, shadow(iris, 0.5));
        c.set(x + 1, y + 1, white);
        c.rect(x + 1, y + 3, 3, 1, light(iris, 0.3));
        c.set(x, y + 2, white);
        c.set(x, y + 3, light(pal.skin, 0.2));
    };
    // Far eye: narrow, near the face edge.
    let far = |c: &mut Canvas| {
        c.rect(x + 6, y, 2, 1, lash);
        c.rect(x + 6, y + 1, 1, 3, iris);
        c.set(x + 6, y + 1, white);
    };
    let closed_line = |c: &mut Canvas| {
        c.rect(x, y + 2, 5, 1, lash);
        c.rect(x + 6, y + 2, 2, 1, lash);
    };
    let happy = |c: &mut Canvas| {
        c.set(x, y + 3, lash);
        c.rect(x + 1, y + 2, 3, 1, lash);
        c.set(x + 4, y + 3, lash);
        c.set(x + 6, y + 3, lash);
        c.set(x + 7, y + 2, lash);
    };
    let brows = |c: &mut Canvas, angry: bool| {
        if angry {
            c.set(x, y - 3, lash);
            c.rect(x + 1, y - 2, 2, 1, lash);
            c.set(x + 3, y - 1, lash);
            c.set(x + 6, y - 1, lash);
        } else {
            c.rect(x + 1, y - 2, 3, 1, shadow(pal.hair, 0.2));
            c.set(x + 6, y - 2, shadow(pal.hair, 0.2));
        }
    };
    if p.hurt {
        // >< eyes
        c.set(x + 1, y + 1, lash);
        c.set(x + 2, y + 2, lash);
        c.set(x + 1, y + 3, lash);
        c.set(x + 3, y + 1, lash);
        c.set(x + 3, y + 3, lash);
        c.set(x + 6, y + 2, lash);
        c.rect(x + 4, y + 6, 2, 2, mouth_c);
        return;
    }
    let blink = p.blink;
    match face {
        0 => {
            // Calm
            if blink { closed_line(c) } else { near(c); far(c) }
            brows(c, false);
            c.set(x + 5, y + 6, skin_dk);
        }
        1 => {
            // Determined
            if blink { closed_line(c) } else { near(c); far(c) }
            brows(c, true);
            c.rect(x + 4, y + 6, 2, 1, lash);
        }
        2 => {
            // Cheerful
            happy(c);
            brows(c, false);
            c.rect(x - 1, y + 4, 2, 1, blush);
            c.rect(x + 6, y + 4, 1, 1, blush);
            c.rect(x + 4, y + 5, 2, 2, mouth_c);
            c.rect(x + 4, y + 5, 2, 1, lash);
        }
        3 => {
            // Sleepy: heavy lids
            c.rect(x, y + 1, 5, 1, lash);
            c.rect(x + 1, y + 2, 3, 2, iris);
            c.set(x + 1, y + 2, light(iris, 0.4));
            c.rect(x + 6, y + 1, 2, 1, lash);
            c.set(x + 6, y + 2, iris);
            c.set(x + 5, y + 6, skin_dk);
        }
        4 => {
            // Fierce: narrowed red eyes, fang
            c.rect(x, y + 1, 5, 1, lash);
            c.rect(x + 1, y + 2, 3, 2, rgb(206, 44, 56));
            c.set(x + 1, y + 2, white);
            c.rect(x + 6, y + 1, 2, 1, lash);
            c.set(x + 6, y + 2, rgb(206, 44, 56));
            brows(c, true);
            c.rect(x + 4, y + 6, 2, 1, lash);
            c.set(x + 5, y + 7, white);
        }
        _ => {
            // Sly wink
            near(c);
            c.set(x + 6, y + 2, lash);
            c.set(x + 7, y + 1, lash);
            c.rect(x + 4, y + 6, 2, 1, lash);
            c.set(x + 6, y + 5, lash);
        }
    }
    if p.mouth && face != 2 {
        c.rect(x + 4, y + 5, 2, 2, mouth_c);
    }
}

/// A flowing hair strand: a chain of tapered segments whose angle bends with
/// motion (trail = streams backward, lift = rises). Angles: 0 = down,
/// positive = toward the back of the head.
#[allow(clippy::too_many_arguments)]
fn strand(c: &mut Canvas, root: P2, len: f32, base: f32, p: &Pose, w0: f32, w1: f32, col: Px, curl: f32) -> Vec<P2> {
    let mut pts = vec![root];
    let n = 4;
    let mut a = root;
    let mut w = w0;
    for i in 0..n {
        let t = (i + 1) as f32 / n as f32;
        let ang = base + (p.hair_trail * 55.0 + p.hair_lift * 95.0) * t + curl * (i as f32 - 1.5);
        let r = ang.to_radians();
        let d = (-r.sin(), r.cos());
        let b = add(a, d, len / n as f32);
        let wn = w0 + (w1 - w0) * t;
        c.taper(a, b, w, wn, col);
        c.disc(b.0, b.1, wn * 0.5, col);
        pts.push(b);
        a = b;
        w = wn;
    }
    pts
}

/// A strand with its own dark edge, so neighbouring locs/strands stay
/// visibly separate inside one hair silhouette.
#[allow(clippy::too_many_arguments)]
fn strand_edged(c: &mut Canvas, root: P2, len: f32, base: f32, p: &Pose, w0: f32, w1: f32, col: Px, curl: f32) {
    strand(c, root, len, base, p, w0 + 1.2, w1 + 1.2, shadow(col, 0.55), curl);
    let pts = strand(c, root, len, base, p, w0, w1, col, curl);
    // twist texture: alternating light/dark ticks along the rope
    let mut k = 0;
    for s in pts.windows(2) {
        let (a, b) = (s[0], s[1]);
        let l = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
        let steps = (l / 2.0).max(1.0) as i32;
        for j in 0..steps {
            let q = lerp(a, b, j as f32 / steps as f32);
            let cl = if k % 2 == 0 { light(col, 0.28) } else { shadow(col, 0.3) };
            c.set(q.0.round() as i32, q.1.round() as i32, cl);
            k += 1;
        }
    }
}

fn back_hair(c: &mut Canvas, look: &Appearance, r: &Rig, pal: &Pal, p: &Pose) {
    let (hx, hy) = r.head;
    let col = pal.hair;
    let dark = shadow(col, 0.22);
    match look.hair {
        1 => {
            // bob: rounded mass to the jaw
            c.poly(&[(hx - 8.8, hy - 4.0), (hx - 1.0, hy - 6.0), (hx - 0.5, hy + 6.0), (hx - 8.5, hy + 6.5)], col);
            c.disc(hx - 5.0, hy + 5.5, 4.0, col);
        }
        2 => {
            // high ponytail: a full, messy tail like the reference
            let root = (hx - 5.0, hy - 7.5);
            for (i, off) in [-2.0f32, -0.7, 0.6, 1.9].iter().enumerate() {
                let cl = [dark, col, light(col, 0.12), dark][i];
                strand(c, (root.0 + off * 0.4, root.1 + off), 24.0 - i as f32 * 1.5, 50.0 + off * 7.0, p, 6.0, 1.4, cl, -5.0);
            }
            c.poly(&[(hx - 8.4, hy - 3.0), (hx - 2.0, hy - 3.0), (hx - 2.0, hy + 6.0), (hx - 7.6, hy + 5.0)], col);
        }
        3 => {
            for i in 0..6 {
                let fi = i as f32;
                let root = (hx - 6.5 + fi * 0.4, hy - 5.0 + fi * 1.8);
                let cl = if i % 2 == 0 { col } else { dark };
                strand(c, root, 22.0 - fi * 0.8, 12.0 + fi * 4.0, p, 4.6, 1.4, cl, 1.5);
            }
        }
        6 => {
            for i in 0..4 {
                let fi = i as f32;
                strand(c, (hx - 6.0, hy - 4.0 + fi * 2.2), 9.0, 40.0 + fi * 10.0, p, 4.0, 1.0, if i % 2 == 0 { col } else { dark }, 4.0);
            }
        }
        7 => {
            // braid
            let mut pnt = (hx - 6.0, hy + 2.0);
            for i in 0..6 {
                c.disc(pnt.0, pnt.1, 2.4 - i as f32 * 0.15, if i % 2 == 0 { col } else { dark });
                pnt = (pnt.0 - 0.6 - p.hair_trail * 0.8, pnt.1 + 2.6 - p.hair_lift * 1.6);
            }
        }
        8 => {
            // long locs: separate twisted ropes hanging from the crown down the back
            for i in 0..9 {
                let fi = i as f32;
                let rr = (-35.0 + fi * 19.0f32).to_radians();
                let root = (hx - 1.5 - rr.cos() * 6.4, hy - 3.5 - rr.sin() * 5.4);
                let cl = if i % 3 == 1 { light(col, 0.1) } else if i % 3 == 2 { dark } else { col };
                let len = 22.0 - (fi - 4.0).abs() * 1.4;
                strand_edged(c, root, len, 6.0 + fi * 4.5, p, 2.8, 2.0, cl, 1.8);
            }
        }
        _ => {}
    }
}

fn front_hair(c: &mut Canvas, look: &Appearance, pal: &Pal, r: &Rig, p: &Pose) {
    let (hx, hy) = r.head;
    let col = pal.hair;
    let dark = shadow(col, 0.22);
    let lite = light(col, 0.15);
    // Cap over the skull, leaving the face open.
    let mut cap = Canvas::new(c.w, c.h);
    let bump = if look.hair == 9 { 1.0 } else { 0.0 };
    cap.ellipse(hx - 1.0, hy - 2.4, 8.6 + bump, 7.0 + bump, col);
    for y in 0..cap.h {
        for x in 0..cap.w {
            let fx = x as f32 + 0.5 - hx;
            let fy = y as f32 + 0.5 - hy;
            let face = fx > -1.2 && fy > -2.6 + (fx * 0.12).max(0.0);
            let below_ear = fx > -4.0 && fy > 1.0;
            if face || below_ear {
                cap.px[(y * cap.w + x) as usize] = [0, 0, 0, 0];
            }
        }
    }
    c.draw(&cap, 0, 0);
    let tri = |c: &mut Canvas, a: P2, b: P2, t: P2, cl: Px| c.poly(&[(hx + a.0, hy + a.1), (hx + b.0, hy + b.1), (hx + t.0, hy + t.1)], cl);
    match look.hair {
        0 => {
            // windswept spikes (back/up) + sharp forward bangs
            let tr = p.hair_trail * 3.0;
            let lf = p.hair_lift * 3.0;
            tri(c, (-5.0, -9.0), (-1.0, -9.0), (-14.0 - tr, -14.0 - lf), col);
            tri(c, (-7.5, -6.0), (-4.0, -8.0), (-16.0 - tr, -8.0 - lf), dark);
            tri(c, (-8.5, -2.5), (-6.5, -6.0), (-15.0 - tr, -2.0 - lf * 0.5), col);
            tri(c, (-1.0, -9.5), (3.0, -9.0), (-6.0 - tr, -16.0 - lf), lite);
            tri(c, (2.0, -9.0), (6.5, -7.0), (3.0, -14.0 - lf), col);
            tri(c, (-1.0, -4.0), (3.0, -4.0), (3.5, 0.5), col);
            tri(c, (2.0, -4.5), (6.0, -4.5), (7.0, -0.5), dark);
            tri(c, (4.5, -5.0), (8.0, -4.0), (9.5, -1.5), col);
        }
        1 => {
            c.poly(&[(hx - 1.5, hy - 4.5), (hx + 7.0, hy - 5.0), (hx + 7.8, hy - 1.5), (hx - 1.0, hy - 2.0)], col);
            c.poly(&[(hx + 6.0, hy - 4.0), (hx + 8.4, hy - 3.0), (hx + 8.4, hy + 5.0), (hx + 6.6, hy + 5.4)], col);
        }
        2 => {
            tri(c, (-1.0, -5.0), (4.0, -5.0), (5.5, -1.0), col);
            tri(c, (3.0, -5.0), (7.5, -4.0), (8.0, -0.5), dark);
            c.rect((hx - 6.0).round() as i32, (hy - 9.0).round() as i32, 3, 3, rgb(206, 44, 60));
        }
        3 => {
            tri(c, (-1.0, -5.0), (3.5, -5.0), (3.0, 0.0), col);
            tri(c, (2.5, -5.0), (7.0, -4.0), (7.5, 1.0), dark);
            strand(c, (hx + 7.0, hy - 2.0), 11.0, -8.0, p, 2.6, 1.2, col, 0.0);
        }
        4 => {
            // side swoop over the far eye
            c.poly(&[(hx - 4.0, hy - 9.0), (hx + 8.0, hy - 6.0), (hx + 9.4, hy + 3.0), (hx + 6.0, hy - 1.0), (hx + 1.0, hy - 3.5)], col);
            c.poly(&[(hx + 3.0, hy - 6.0), (hx + 8.0, hy - 5.0), (hx + 8.6, hy + 1.0)], lite);
        }
        5 => {
            c.disc(hx - 5.5, hy - 8.0, 3.8, col);
            c.disc(hx + 2.0, hy - 10.0, 3.8, col);
            c.disc(hx - 5.5, hy - 8.6, 1.4, dark);
            c.disc(hx + 2.0, hy - 10.6, 1.4, dark);
            tri(c, (0.0, -5.0), (5.0, -5.0), (5.0, -1.0), col);
        }
        6 => {
            // shaggy layered: jagged bangs and tufts
            for i in 0..5 {
                let fx = -1.5 + i as f32 * 2.2;
                tri(c, (fx, -5.0), (fx + 2.6, -5.0), (fx + 1.8, -0.5 - (i % 2) as f32 * 1.5), if i % 2 == 0 { col } else { dark });
            }
            tri(c, (-7.0, -7.0), (-3.0, -9.5), (-10.5, -11.0), col);
            tri(c, (-2.0, -9.5), (2.0, -9.5), (-1.0, -13.0), lite);
            tri(c, (2.0, -9.0), (6.0, -7.5), (6.5, -12.0), col);
        }
        7 => {
            tri(c, (1.0, -5.0), (5.5, -5.0), (6.0, -2.0), col);
            for i in 0..7 {
                let a = std::f32::consts::PI * (1.0 + i as f32 * 0.16);
                c.disc(hx - 1.0 + a.cos() * 8.4, hy - 2.6 + a.sin() * 6.8, 1.9, if i % 2 == 0 { lite } else { dark });
            }
        }
        8 => {
            // locs falling over the brow and in front of the ear
            strand_edged(c, (hx + 1.0, hy - 7.0), 7.0, -35.0, p, 2.6, 2.0, col, 0.0);
            strand_edged(c, (hx + 4.0, hy - 6.5), 6.0, -40.0, p, 2.4, 1.8, dark, 0.0);
            strand_edged(c, (hx - 3.0, hy - 4.0), 15.0, 4.0, p, 2.8, 2.0, light(col, 0.08), 0.6);
        }
        9 => {
            // short curls: bumpy outline
            for i in 0..11 {
                let a = std::f32::consts::PI * (0.85 + i as f32 * 0.115);
                c.disc(hx - 1.0 + a.cos() * 8.4, hy - 2.4 + a.sin() * 7.0, 2.3, if i % 3 == 0 { lite } else { col });
            }
            for i in 0..3 {
                c.disc(hx + 1.0 + i as f32 * 2.4, hy - 5.0, 1.9, if i % 2 == 0 { col } else { dark });
            }
        }
        _ => {}
    }
}

fn hair_shine(c: &mut Canvas, look: &Appearance, pal: &Pal, (hx, hy): P2) {
    let shine = light(pal.hair, 0.42);
    // A short "angel ring" arc on the crown, only over hair-coloured pixels.
    let pts: [(i32, i32); 5] = [(-5, -7), (-4, -8), (-3, -8), (-2, -9), (-1, -9)];
    let base = pal.hair;
    for (dx, dy) in pts.iter().copied() {
        let (dx, dy): (i32, i32) = (dx, dy);
        let x = (hx.round() as i32) + dx;
        let y = (hy.round() as i32) + dy + if look.hair == 5 { 1 } else { 0 };
        let px = c.get(x, y);
        let close = (px[0] as i32 - base[0] as i32).abs() + (px[1] as i32 - base[1] as i32).abs() + (px[2] as i32 - base[2] as i32).abs() < 90;
        if px[3] > 0 && close {
            c.set(x, y, shine);
        }
    }
}

// =================================================================== weapons

fn steel() -> Px {
    rgb(210, 220, 236)
}

fn draw_weapon(c: &mut Canvas, w: Weapon, a: &Arm, d: P2, back: &Arm, p: &Pose) {
    let hand = add(a.hand, dir(a.fore_deg), 0.8);
    match w {
        Weapon::SwordShield => {
            let guard = add(hand, d, 2.0);
            let tip = add(hand, d, 21.0);
            c.taper(guard, tip, 3.6, 1.2, steel());
            c.linef(add(hand, d, 3.5), add(hand, d, 18.5), rgb(255, 255, 255));
            let perp = (d.1, -d.0);
            c.capsule(add(guard, perp, -3.2), add(guard, perp, 3.2), 2.0, rgb(214, 170, 70));
            let pommel = add(hand, d, -2.8);
            c.disc(pommel.0, pommel.1, 1.4, rgb(214, 170, 70));
        }
        Weapon::Club => {
            // a knobbly wooden club, thicker toward the end
            let wood = rgb(156, 104, 60);
            let grip = add(hand, d, -2.0);
            let tip = add(hand, d, 16.0);
            c.taper(grip, tip, 2.2, 4.4, wood);
            c.disc(tip.0, tip.1, 2.4, wood);
            c.linef(add(hand, d, 4.0), add(hand, d, 14.0), rgb(196, 146, 96));
            let perp = (d.1, -d.0);
            for k in [8.0f32, 12.0] {
                let s = add(add(hand, d, k), perp, 2.6);
                c.set(s.0.round() as i32, s.1.round() as i32, rgb(90, 60, 36));
            }
            c.capsule(add(hand, d, -1.0), add(hand, d, 1.5), 2.0, rgb(220, 210, 190));
        }
        Weapon::Stick => {
            // a crooked branch with a twig and a leaf
            let wood = rgb(132, 92, 56);
            let mid = add(hand, d, 8.0);
            let perp = (d.1, -d.0);
            let tip = add(add(hand, d, 17.0), perp, 1.2);
            c.taper(add(hand, d, -2.0), mid, 1.6, 1.4, wood);
            c.taper(mid, tip, 1.4, 1.0, wood);
            let twig = add(add(mid, d, 3.0), perp, 3.0);
            c.linef(add(mid, d, 1.0), twig, wood);
            c.disc(twig.0, twig.1, 1.2, rgb(110, 180, 80));
        }
        Weapon::WoodSword => {
            let wood = rgb(196, 150, 96);
            let guard = add(hand, d, 2.0);
            let tip = add(hand, d, 18.0);
            c.taper(guard, tip, 3.2, 1.4, wood);
            c.linef(add(hand, d, 3.5), add(hand, d, 15.5), light(wood, 0.3));
            let perp = (d.1, -d.0);
            c.capsule(add(guard, perp, -3.0), add(guard, perp, 3.0), 1.8, rgb(126, 84, 50));
            c.capsule(add(hand, d, -2.6), add(hand, d, 0.5), 1.8, rgb(126, 84, 50));
        }
        Weapon::TwinDaggers => dagger(c, hand, d),
        Weapon::Gauntlets => {}
        Weapon::Bow => {
            // bow perpendicular to the arrow line (= forearm direction)
            let perp = (d.1, -d.0);
            let wood = rgb(146, 92, 50);
            let pts: Vec<P2> = (0..=10)
                .map(|i| {
                    let t = i as f32 / 10.0 * 2.0 - 1.0;
                    let bulge = (1.0 - t * t) * 4.0;
                    (hand.0 + perp.0 * t * 12.0 + d.0 * bulge, hand.1 + perp.1 * t * 12.0 + d.1 * bulge)
                })
                .collect();
            for i in 0..pts.len() - 1 {
                c.capsule(pts[i], pts[i + 1], 2.2, wood);
            }
            let drawn = p.ba.1 > 120.0;
            let nock = if drawn { add(back.hand, dir(back.fore_deg), 0.8) } else { add(hand, d, -1.0) };
            c.linef(pts[0], nock, rgb(238, 238, 238));
            c.linef(nock, pts[10], rgb(238, 238, 238));
            if drawn {
                c.linef(nock, add(hand, d, 7.0), rgb(186, 136, 84));
            }
        }
        Weapon::Staff => {
            let base = add(hand, d, -10.0);
            let tip = add(hand, d, 18.0);
            c.capsule(base, tip, 2.4, rgb(132, 88, 54));
            c.disc(tip.0, tip.1, 3.4, rgb(130, 220, 255));
            c.set(tip.0.round() as i32 - 1, tip.1.round() as i32 - 1, rgb(255, 255, 255));
        }
    }
}

fn dagger(c: &mut Canvas, hand: P2, d: P2) {
    let tip = add(hand, d, 11.0);
    c.taper(add(hand, d, 1.5), tip, 2.8, 1.0, steel());
    c.disc(hand.0, hand.1, 1.7, rgb(110, 60, 140));
}

fn gauntlet(c: &mut Canvas, hand: P2) {
    c.disc(hand.0, hand.1, 2.9, rgb(74, 96, 150));
    c.set(hand.0.round() as i32 + 1, hand.1.round() as i32 - 1, rgb(240, 204, 90));
}

fn shield(c: &mut Canvas, at: P2) {
    c.ellipse(at.0, at.1, 5.0, 6.6, rgb(132, 82, 46));
    c.ellipse(at.0, at.1, 3.8, 5.4, rgb(70, 112, 178));
    c.disc(at.0, at.1, 1.6, rgb(232, 196, 90));
}

// =================================================================== defeated pose

fn render_dead(look: &Appearance, pal: &Pal) -> Canvas {
    let mut out = Canvas::new(SIZE, HEIGHT);
    let gy = FEET_Y as f32 - 3.0;
    let cx = FEET_X as f32;
    let leg_col = if look.bottom == 0 || look.bottom == 3 { pal.skin } else { pal.bottom };
    let top_col = if look.top == 0 { pal.skin } else { pal.top };
    let mut legs = Canvas::new(SIZE, HEIGHT);
    seg(&mut legs, (cx - 4.0, gy), (cx - 26.0, gy + 0.5), 5.5, 4.0, leg_col);
    legs.ellipse(cx - 27.0, gy, 3.0, 2.5, rgb(60, 50, 60));
    stamp(&mut out, legs, 0.0);
    let mut body = Canvas::new(SIZE, HEIGHT);
    body.ellipse(cx + 3.0, gy - 0.5, 9.0, 4.5, top_col);
    if look.top == 0 && look.feminine() {
        body.ellipse(cx + 5.0, gy - 1.0, 3.0, 4.0, pal.under);
    }
    body.ellipse(cx - 5.0, gy, 3.5, 4.0, if look.bottom == 0 { pal.under } else { leg_col });
    stamp(&mut out, body, 0.0);
    let mut head = Canvas::new(SIZE, HEIGHT);
    head.ellipse(cx + 18.0, gy - 3.0, 7.0, 6.5, pal.skin);
    head.ellipse(cx + 19.0, gy - 6.5, 8.0, 5.0, pal.hair);
    stamp(&mut out, head, 0.0);
    for ex in [cx as i32 + 16, cx as i32 + 20] {
        out.set(ex, gy as i32 - 3, INK);
        out.set(ex + 1, gy as i32 - 2, INK);
        out.set(ex, gy as i32 - 1, INK);
    }
    out
}

/// Frame index for looping animations.
pub fn frame_for_loop(anim: Anim, t: f32) -> u8 {
    ((t * anim.fps()) as u32 % anim.frames() as u32) as u8
}

/// Frame index of an attack from its phase.
pub fn attack_frame(phase: galebound_core::sim::AttackPhase, phase_progress: f32) -> u8 {
    use galebound_core::sim::AttackPhase::*;
    match phase {
        Startup => 0,
        Active => 1,
        Recovery => if phase_progress < 0.5 { 2 } else { 3 },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use galebound_core::appearance::SLOTS;
    use galebound_core::classes::EVERY_CLASS;

    fn all_anims() -> Vec<Anim> {
        let mut v = vec![Anim::Idle, Anim::Run, Anim::JumpUp, Anim::Fall, Anim::Dodge, Anim::Hurt, Anim::Dead, Anim::Talk];
        for m in [Motion::Jab, Motion::Thrust, Motion::Sweep, Motion::Uppercut, Motion::Spin, Motion::Spike, Motion::Slam, Motion::Kick, Motion::Cast, Motion::Shoot, Motion::Dive, Motion::Rise, Motion::Throw] {
            v.push(Anim::Attack(m));
        }
        v
    }

    /// Every cosmetic option x weapon x animation frame renders, stays inside
    /// the canvas and has a body.
    #[test]
    fn all_combinations_render_inside_canvas() {
        let weapons: Vec<Option<Weapon>> = std::iter::once(None)
            .chain(EVERY_CLASS.iter().map(|c| Some(c.info().weapon)))
            .chain([Some(Weapon::Club), Some(Weapon::WoodSword)])
            .collect();
        for &slot in SLOTS {
            for v in 0..slot.count() {
                for body in 0..2u8 {
                    let mut look = Appearance { body, top: 1, bottom: 1, ..Default::default() };
                    look.set(slot, v as u8);
                    for &w in &weapons {
                        for a in all_anims() {
                            for f in 0..a.frames() {
                                let c = render(&look, w, a, f);
                                let b = c.bounds().expect("sprite is empty");
                                assert!(b.0 > 0 && b.1 > 0 && b.2 < SIZE - 1 && b.3 < HEIGHT - 1, "{:?}={} {:?} {:?} f{} touches edge {:?}", slot, v, w, a, f, b);
                                let filled = c.px.iter().filter(|p| p[3] > 0).count();
                                assert!(filled > 300, "sprite too sparse");
                            }
                        }
                    }
                }
            }
        }
    }

    /// Standing and running poses keep a foot on the ground line.
    #[test]
    fn feet_are_planted() {
        for a in [Anim::Idle, Anim::Run, Anim::Talk, Anim::Attack(Motion::Jab), Anim::Attack(Motion::Kick)] {
            for f in 0..a.frames() {
                for w in [None, Some(Weapon::Gauntlets), Some(Weapon::SwordShield)] {
                    let p = pose_for(a, f, w);
                    let r = build_rig(&p, false);
                    let lowest = (r.fl.ankle.1).max(r.bl.ankle.1) + ANKLE_H;
                    assert!((lowest - FEET_Y as f32).abs() < 1.6 + p.lift, "{:?} f{} foot at {}", a, f, lowest);
                }
            }
        }
    }

    /// Joints bend the anatomically correct way in every pose.
    #[test]
    fn joint_limits() {
        for a in all_anims() {
            for f in 0..a.frames() {
                let p = pose_for(a, f, None);
                for leg in [p.fl, p.bl] {
                    assert!(leg.1 >= 0.0 && leg.1 <= 160.0, "{:?} knee {}", a, leg.1);
                }
                for arm in [p.fa, p.ba] {
                    assert!(arm.1 >= 0.0 && arm.1 <= 165.0, "{:?} elbow {}", a, arm.1);
                }
            }
        }
    }

    #[test]
    fn every_attack_kind_has_a_motion() {
        use galebound_core::classes::AttackKind::*;
        for &c in EVERY_CLASS {
            for k in [LightNeutral, LightSide, LightDown, AirNeutral, AirSide, AirDown, HeavyNeutral, HeavySide, HeavyDown, AirHeavyNeutral, AirHeavyDown] {
                let _ = motion_for(c, k);
            }
        }
    }
}
