//! Anime pixel-art character sprites, generated in code.
//!
//! **To change poses and animations, edit `poses.rs`** (and see
//! CHARACTERS.md). This file draws whatever pose it is given.
//!
//! A 2D skeleton (pelvis -> spine -> neck -> head, shoulders -> upper arm ->
//! forearm -> fist, hips -> thigh -> shin -> foot) is posed from the
//! keyframes in poses.rs, modelled on the reference sprite sheets (idle, guard, run, jump,
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
use crate::poses::{pose_for, Limb, Plant, Pose, SwooshAt};
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
    /// Heel drop: the front leg stamps straight down (fist and bow classes).
    Stomp,
    /// Hundred Fists: rapid punches, front fist then back fist, one per hit.
    Flurry,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Anim {
    /// Standing relaxed: arms down, weapon lowered (NPCs, and players out of combat).
    Idle,
    /// Fighting stance, shown for a few seconds after attacking or being hit.
    Battle,
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
            Anim::Idle | Anim::Battle => 4,
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
            Anim::Idle => 3.0,
            Anim::Battle => 4.0,
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
    let ranged = matches!(w, Weapon::Bow | Weapon::Staff | Weapon::DualPistols);
    let ranged_motion = if w == Weapon::Bow { Motion::Shoot } else { Motion::Cast };
    match kind {
        LightNeutral => Motion::Jab,
        LightSide => if ranged { ranged_motion } else { Motion::Thrust },
        LightDown => Motion::Sweep,
        AirNeutral => Motion::Spin,
        AirSide => if ranged { ranged_motion } else { Motion::Thrust },
        AirDown => match class {
            ClassId::Stormfist | ClassId::Windrunner => Motion::Stomp,
            _ => Motion::Spike,
        },
        HeavyNeutral => match class {
            ClassId::Duskblade => Motion::Throw,
            ClassId::Vanguard | ClassId::Stormfist | ClassId::Novice => Motion::Uppercut,
            _ => ranged_motion,
        },
        HeavySide => match class {
            ClassId::Stormfist => Motion::Kick,
            ClassId::Gunslinger => Motion::Spin,
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

/// Torso width scale (chunkier, sturdier bodies).
const TW: f32 = 1.2;

impl Rig {
    /// Torso-local point scaled to the body's width (for torso and clothes).
    fn tw(&self, x: f32, y: f32) -> P2 {
        self.t(x * TW, y)
    }
    /// Torso-local point (x forward, y up from the pelvis) to sprite pixels.
    fn t(&self, x: f32, y: f32) -> P2 {
        let r = self.lean.to_radians();
        let up = (r.sin(), -r.cos());
        let fwd = (r.cos(), r.sin());
        (self.pelvis.0 + fwd.0 * x + up.0 * y, self.pelvis.1 + fwd.1 * x + up.1 * y)
    }
}

fn build_leg(hip: P2, leg: Limb, foot_override: Option<f32>) -> Leg {
    let (thigh, knee) = (leg.swing, leg.bend);
    let knee_bend = knee.clamp(0.0, 160.0);
    let k = add(hip, dir(thigh), THIGH);
    let shin_deg = thigh - knee_bend;
    let a = add(k, dir(shin_deg), SHIN);
    let auto = if shin_deg < 0.0 { -shin_deg * 0.75 } else { -shin_deg * 0.25 };
    Leg { hip, knee: k, ankle: a, thigh_deg: thigh, shin_deg, foot_deg: foot_override.unwrap_or(auto.clamp(-25.0, 75.0)) }
}

fn build_arm(sh: P2, lean: f32, arm: Limb) -> Arm {
    let (upper, elbow) = (arm.swing, arm.bend);
    let ua = lean + upper;
    let fa = ua + elbow.clamp(0.0, 165.0);
    let e = add(sh, dir(ua), UPPER);
    let h = add(e, dir(fa), FORE);
    Arm { sh, elbow: e, hand: h, fore_deg: fa }
}

fn build_rig(p: &Pose, fem: bool) -> Rig {
    // Legs relative to a pelvis at the origin, then plant on the ground.
    let fl0 = build_leg((1.2, 0.0), p.front_leg, p.front_foot);
    let bl0 = build_leg((-1.2, 0.0), p.back_leg, None);
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
    let sh_f = rig.tw(1.5 + sw * 0.5, 13.0);
    let sh_b = rig.tw(-2.6 - sw, 13.4);
    rig.fa = build_arm(sh_f, p.lean, p.front_arm);
    rig.ba = build_arm(sh_b, p.lean, p.back_arm);
    rig.head = rig.t(1.4, HEAD_Y);
    rig
}

/// Sprite pixels per world unit (the same as `gfx::WORLD_PX`).
const PX_PER_UNIT: f32 = 0.85;

/// A point given like a hitbox (world units from the character's middle,
/// x forward, y down) in sprite pixels.
fn world_to_sprite((x, y): P2) -> P2 {
    let middle_y = FEET_Y as f32 - galebound_core::sim::PLAYER_H * 0.5 * PX_PER_UNIT;
    (FEET_X as f32 + x * PX_PER_UNIT, middle_y + y * PX_PER_UNIT)
}

/// Turns the front arm so the hand and weapon point at the pose's `aim`
/// spot (see poses.rs). Poses without an aim are unchanged.
fn apply_aim(mut p: Pose, fem: bool) -> Pose {
    if let Some(aim) = p.aim {
        let r = build_rig(&p, fem);
        let target = world_to_sprite(aim);
        let toward = (target.0 - r.fa.sh.0).atan2(target.1 - r.fa.sh.1).to_degrees();
        let bend = p.front_arm.bend.min(12.0);
        p.front_arm = Limb { swing: toward - p.lean - bend * 0.5, bend };
    }
    p
}

/// The finished pose for a frame: the animation book's pose, with any aim applied.
fn final_pose(look: &Appearance, weapon: Option<Weapon>, anim: Anim, frame: u8) -> Pose {
    apply_aim(pose_for(anim, frame % anim.frames(), weapon), look.feminine())
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
    let p = final_pose(&look, weapon, anim, frame);
    let r = build_rig(&p, look.feminine());
    let mut out = Canvas::new(SIZE, HEIGHT);
    let new = || Canvas::new(SIZE, HEIGHT);

    // 0. Cloaks and packs behind everything (wings go over the back hair)
    let winged = matches!(look.back, 3 | 5 | 6);
    if look.back != 0 && !winged {
        let mut c = new();
        draw_back(&mut c, &look, &pal, &r, &p);
        stamp(&mut out, c, 0.0);
    }
    // 1. Hair behind the body
    {
        let mut c = new();
        back_hair(&mut c, &look, &r, &pal, &p);
        if winged {
            stamp(&mut out, c, 0.0);
            c = new();
            draw_back(&mut c, &look, &pal, &r, &p);
        }
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
            dagger(&mut c, r.ba.hand, dir(r.ba.fore_deg + p.back_weapon));
        }
        if weapon == Some(Weapon::DualPistols) {
            pistol(&mut c, r.ba.hand, dir(r.ba.fore_deg + p.back_weapon - 50.0));
        }
        if weapon == Some(Weapon::CosmicOrbs) {
            let h = r.ba.hand;
            c.disc(h.0 - 2.0, h.1 - 5.0, 2.6, rgb(200, 150, 255));
            c.set(h.0.round() as i32 - 3, h.1.round() as i32 - 6, rgb(255, 255, 255));
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
        {
            let mut h = Hd { c: &mut c, o: r.head, k: HEAD_K };
            draw_head(&mut h, &pal, r.head);
            front_hair(&mut h, &look, &pal, &r, &p);
            if look.hat != 0 {
                draw_hat(&mut h, &look, &pal, r.head, &p);
            }
        }
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
    if weapon == Some(Weapon::Greatshield) {
        let mut c = new();
        let h = r.ba.hand;
        greatshield(&mut c, (h.0 + 3.0, h.1 - 2.0));
        stamp(&mut out, c, 0.0);
    }
    // 9. Front arm + weapon
    {
        let wd = dir(r.fa.fore_deg + p.weapon);
        let mut wc = new();
        if let Some(w) = weapon {
            draw_weapon(&mut wc, w, &r.fa, wd, &r.ba, &p);
        }
        let mut wcs = wc.clone();
        cel(&mut wcs);
        let weapon_layer = wcs.outlined(INK);
        let behind = matches!(weapon, Some(Weapon::Bow) | Some(Weapon::Greatbow));
        if behind {
            out.draw(&weapon_layer, 0, 0);
        }
        let mut c = new();
        draw_arm(&mut c, &look, &pal, &r.fa, false);
        if matches!(weapon, Some(Weapon::Gauntlets) | Some(Weapon::Nunchucks)) {
            gauntlet(&mut c, r.fa.hand);
        }
        stamp(&mut out, c, 0.0);
        if !behind {
            out.draw(&weapon_layer, 0, 0);
        }
    }
    // 10. Motion swoosh
    if let Some(sw) = p.swoosh {
        let (a0, a1, at) = (sw.from, sw.to, sw.at);
        let (center, reach) = match at {
            SwooshAt::Shoulder => (
                r.fa.sh,
                match weapon {
                    Some(Weapon::Greatsword) | Some(Weapon::Kusarigama) => 34.0,
                    Some(Weapon::WarStaff) | Some(Weapon::Katana) => 32.0,
                    Some(Weapon::SwordShield) | Some(Weapon::Staff) | Some(Weapon::Greatshield) => 30.0,
                    Some(Weapon::TwinDaggers) | Some(Weapon::Nunchucks) => 24.0,
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
    let (w_sh, w_el, w_wr) = if look.feminine() { (4.6, 4.0, 3.4) } else { (5.4, 4.8, 4.0) };
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
    let fist = add(a.hand, dir(a.fore_deg), 1.0);
    c.disc(fist.0, fist.1, 3.1, skin);
    // knuckle line
    let k = add(fist, dir(a.fore_deg), 1.6);
    c.set(k.0.round() as i32, k.1.round() as i32, shadow(skin, 0.25));
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
    let (w_hip, w_knee, w_calf, w_ank) = if fem { (7.4, 5.0, 5.6, 3.6) } else { (7.4, 5.6, 6.2, 4.0) };
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
    // shoes are drawn a size up: big chunky sneakers and boots
    let pt = |x: f32, y: f32| {
        let (x, y) = (x * 1.25 + 0.6, y * 1.2);
        (o.0 + t.0 * x + u.0 * y, o.1 + t.1 * x + u.1 * y)
    };
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
        seg(c, l.ankle, top, 5.2, 5.4, upper);
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
        let w: Vec<P2> = pts.iter().map(|&(x, y)| r.tw(x, y)).collect();
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
        let a = r.tw(1.0, 9.5);
        let b = r.tw(3.5, 9.0);
        c.linef(a, b, shadow(pal.skin, 0.18));
    }
    let belly = r.tw(1.8, 4.0);
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
                let a = r.tw(x, 15.0);
                let b2 = r.tw(x, 10.5);
                c.linef(a, b2, rgb(236, 236, 240));
            }
            poly(c, &[(-6.0, -4.0), (6.4, -4.0), (6.4, -2.8), (-6.0, -2.8)], shadow(top, 0.2));
        }
        4 => {
            // robe body (hem drawn as a skirt that follows the knees)
            let b = grow(&body, 0.8);
            poly(c, &b, top);
            poly(c, &[(-5.2, 0.0), (5.2, 0.0), (5.2, 1.6), (-5.2, 1.6)], light(top, 0.4));
            let a = r.tw(1.0, 15.0);
            let b2 = r.tw(1.0, 1.6);
            c.linef(a, b2, shadow(top, 0.3));
        }
        5 => {
            // undershirt + breastplate
            let b = grow(&body, 0.4);
            poly(c, &b, shadow(top, 0.1));
            poly(c, &[(-4.8, 5.0), (5.2, 5.0), (5.4, 12.0), (4.2, 14.8), (-3.6, 15.0), (-5.4, 12.5)], rgb(170, 180, 198));
            let a = r.tw(-2.0, 12.0);
            let b2 = r.tw(3.0, 12.0);
            c.linef(a, b2, rgb(232, 238, 248));
            poly(c, &[(-5.0, -1.0), (5.0, -1.0), (5.0, 1.4), (-5.0, 1.4)], rgb(112, 72, 42));
        }
        6 => {
            // jacket + scarf
            let mut b = grow(&body, 0.8);
            b[0].1 = -3.0;
            b[1].1 = -3.0;
            poly(c, &b, top);
            let z1 = r.tw(1.2, 14.5);
            let z2 = r.tw(1.2, -2.5);
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
            let s1 = r.tw(3.8, 14.6);
            let s2 = r.tw(-5.0, -0.5);
            c.linef(s1, s2, strap);
            c.linef((s1.0 - 1.0, s1.1), (s2.0 - 1.0, s2.1), strap);
            poly(c, &[(-5.2, -1.2), (5.2, -1.2), (5.2, 0.4), (-5.2, 0.4)], rgb(196, 162, 108));
            let knot = r.tw(2.6, -0.4);
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
            let wl = r.tw(-5.4, 1.5);
            let wr = r.tw(5.2, 1.5);
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
            let wl = r.tw(-5.6, 2.0);
            let wr = r.tw(5.6, 2.0);
            let hf = add(r.fl.knee, fwd_of(r.fl.thigh_deg), 3.4);
            let hb = add(r.bl.knee, fwd_of(r.bl.thigh_deg), -3.4);
            let mid = (lerp(hf, hb, 0.5).0, hf.1.max(hb.1) + 1.0);
            c.poly(&[wl, wr, hf, mid, hb], col);
            let s = r.tw(1.0, 1.6);
            c.linef(s, mid, shadow(col, 0.3));
            Some(c)
        }
        _ => None,
    }
}

// =================================================================== head, face, hair

/// How much bigger the head (and hair) is drawn than the body's scale:
/// big heads and big eyes give the characters their anime charm.
const HEAD_K: f32 = 1.34;

/// Draws head-relative shapes scaled up by `k` around the head centre `o`,
/// so the head and every hairstyle grow together.
struct Hd<'a> {
    c: &'a mut Canvas,
    o: P2,
    k: f32,
}

impl<'a> Hd<'a> {
    fn p(&self, (x, y): P2) -> P2 {
        (self.o.0 + (x - self.o.0) * self.k, self.o.1 + (y - self.o.1) * self.k)
    }
    fn ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, col: Px) {
        let q = self.p((cx, cy));
        self.c.ellipse(q.0, q.1, rx * self.k, ry * self.k, col);
    }
    fn disc(&mut self, cx: f32, cy: f32, r: f32, col: Px) {
        let q = self.p((cx, cy));
        self.c.disc(q.0, q.1, r * self.k, col);
    }
    fn poly(&mut self, pts: &[P2], col: Px) {
        let v: Vec<P2> = pts.iter().map(|&q| self.p(q)).collect();
        self.c.poly(&v, col);
    }
    fn taper(&mut self, a: P2, b: P2, wa: f32, wb: f32, col: Px) {
        let (a, b) = (self.p(a), self.p(b));
        self.c.taper(a, b, wa * self.k, wb * self.k, col);
    }
    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, col: Px) {
        let (x, y, w, h) = (x as f32, y as f32, w as f32, h as f32);
        self.poly(&[(x, y), (x + w, y), (x + w, y + h), (x, y + h)], col);
    }
    fn set(&mut self, x: i32, y: i32, col: Px) {
        let q = self.p((x as f32 + 0.5, y as f32 + 0.5));
        self.c.set(q.0.floor() as i32, q.1.floor() as i32, col);
    }
}

fn draw_head(c: &mut Hd, pal: &Pal, (hx, hy): P2) {
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
    // Drawn at the scaled head size: big anime eyes (near eye 5x6, far eye
    // 2x5), each with a lash line, a dark pupil band and white highlights.
    let x = (hx + 0.4).round() as i32;
    let y = (hy - 2.2).round() as i32;
    let white = rgb(255, 255, 255);
    let iris = pal.eye;
    let iris_lo = light(iris, 0.35);
    let pupil = shadow(iris, 0.55);
    let lash = INK;
    let skin_dk = shadow(pal.skin, 0.38);
    let mouth_c = rgb(150, 56, 66);
    let blush = mix(pal.skin, rgb(244, 120, 140), 0.45);
    let near = |c: &mut Canvas, col: Px| {
        let lo = light(col, 0.35);
        let pu = shadow(col, 0.55);
        // lash line, thicker toward the back, with a flick
        c.rect(x, y, 6, 1, lash);
        c.rect(x - 1, y - 1, 2, 1, lash);
        c.set(x + 5, y + 1, lash);
        // sclera at the back, iris filling the rest
        c.rect(x, y + 1, 1, 5, white);
        c.rect(x + 1, y + 1, 4, 5, col);
        c.rect(x + 2, y + 2, 2, 3, pu);
        c.rect(x + 1, y + 5, 4, 1, lo);
        // highlights
        c.rect(x + 1, y + 1, 2, 2, white);
        c.set(x + 4, y + 4, white);
        // lower lid
        c.rect(x + 1, y + 6, 4, 1, shadow(pal.skin, 0.2));
    };
    let far = |c: &mut Canvas, col: Px| {
        c.rect(x + 7, y, 3, 1, lash);
        c.rect(x + 8, y + 1, 2, 5, col);
        c.set(x + 9, y + 3, shadow(col, 0.55));
        c.set(x + 8, y + 1, white);
        c.set(x + 8, y + 5, light(col, 0.35));
    };
    let closed_line = |c: &mut Canvas| {
        c.rect(x, y + 3, 6, 1, lash);
        c.set(x + 5, y + 4, lash);
        c.rect(x + 7, y + 3, 3, 1, lash);
    };
    let happy = |c: &mut Canvas| {
        // ^ ^ smiling eyes
        c.set(x, y + 4, lash);
        c.set(x + 1, y + 3, lash);
        c.rect(x + 2, y + 2, 2, 1, lash);
        c.set(x + 4, y + 3, lash);
        c.set(x + 5, y + 4, lash);
        c.set(x + 7, y + 4, lash);
        c.set(x + 8, y + 3, lash);
        c.set(x + 9, y + 4, lash);
    };
    let brows = |c: &mut Canvas, angry: bool| {
        let bc = shadow(pal.hair, 0.3);
        if angry {
            c.set(x, y - 4, lash);
            c.rect(x + 1, y - 3, 2, 1, lash);
            c.rect(x + 3, y - 2, 2, 1, lash);
            c.rect(x + 7, y - 2, 2, 1, lash);
        } else {
            c.rect(x + 1, y - 3, 4, 1, bc);
            c.rect(x + 7, y - 3, 2, 1, bc);
        }
    };
    let cheeks = |c: &mut Canvas| {
        c.rect(x - 1, y + 7, 3, 1, blush);
        c.rect(x + 8, y + 7, 2, 1, blush);
    };
    // little nose shadow and the mouth line
    c.set(x + 8, y + 8, skin_dk);
    if p.hurt {
        // > < eyes
        for (dx, dy) in [(1, 1), (2, 2), (3, 3), (2, 4), (1, 5)] {
            c.set(x + dx, y + dy, lash);
        }
        for (dx, dy) in [(9, 1), (8, 2), (7, 3), (8, 4), (9, 5)] {
            c.set(x + dx, y + dy, lash);
        }
        c.rect(x + 5, y + 10, 3, 2, mouth_c);
        return;
    }
    let blink = p.blink;
    match face {
        0 => {
            // Calm
            if blink { closed_line(c) } else { near(c, iris); far(c, iris) }
            brows(c, false);
            c.rect(x + 5, y + 10, 2, 1, skin_dk);
        }
        1 => {
            // Determined
            if blink { closed_line(c) } else { near(c, iris); far(c, iris) }
            brows(c, true);
            c.rect(x + 5, y + 10, 3, 1, lash);
        }
        2 => {
            // Cheerful
            happy(c);
            brows(c, false);
            cheeks(c);
            c.rect(x + 5, y + 9, 3, 2, mouth_c);
            c.rect(x + 5, y + 9, 3, 1, lash);
            c.set(x + 6, y + 10, rgb(240, 130, 140));
        }
        3 => {
            // Sleepy: heavy lids over the top half of the eyes
            near(c, iris);
            far(c, iris);
            c.rect(x - 1, y, 7, 3, shadow(pal.skin, 0.06));
            c.rect(x, y + 2, 6, 1, lash);
            c.rect(x + 7, y, 3, 3, shadow(pal.skin, 0.06));
            c.rect(x + 7, y + 2, 3, 1, lash);
            c.rect(x + 5, y + 10, 2, 1, skin_dk);
        }
        4 => {
            // Fierce: narrowed red eyes, fang
            let red = rgb(206, 44, 56);
            near(c, red);
            far(c, red);
            c.rect(x - 1, y, 7, 2, shadow(pal.skin, 0.04));
            c.rect(x, y + 1, 6, 1, lash);
            c.rect(x + 7, y, 3, 2, shadow(pal.skin, 0.04));
            c.rect(x + 7, y + 1, 3, 1, lash);
            brows(c, true);
            c.rect(x + 5, y + 10, 3, 1, lash);
            c.set(x + 6, y + 11, white);
        }
        _ => {
            // Sly wink
            near(c, iris);
            c.set(x + 7, y + 4, lash);
            c.rect(x + 8, y + 3, 2, 1, lash);
            brows(c, false);
            cheeks(c);
            c.rect(x + 5, y + 10, 3, 1, lash);
            c.set(x + 8, y + 9, lash);
        }
    }
    let _ = (iris_lo, pupil);
    if p.mouth && face != 2 {
        c.rect(x + 5, y + 9, 3, 2, mouth_c);
    }
}

/// A flowing hair strand: a chain of tapered segments whose angle bends with
/// motion (trail = streams backward, lift = rises). Angles: 0 = down,
/// positive = toward the back of the head.
#[allow(clippy::too_many_arguments)]
fn strand(c: &mut Hd, root: P2, len: f32, base: f32, p: &Pose, w0: f32, w1: f32, col: Px, curl: f32) -> Vec<P2> {
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
fn strand_edged(c: &mut Hd, root: P2, len: f32, base: f32, p: &Pose, w0: f32, w1: f32, col: Px, curl: f32) {
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
    let c = &mut Hd { c, o: r.head, k: HEAD_K };
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

fn front_hair(c: &mut Hd, look: &Appearance, pal: &Pal, r: &Rig, p: &Pose) {
    let (hx, hy) = r.head;
    let col = pal.hair;
    let dark = shadow(col, 0.22);
    let lite = light(col, 0.15);
    // Cap over the skull, leaving the face open.
    let mut cap = Canvas::new(c.c.w, c.c.h);
    let bump = if look.hair == 9 { 1.0 } else { 0.0 };
    Hd { c: &mut cap, o: c.o, k: c.k }.ellipse(hx - 1.0, hy - 2.4, 8.6 + bump, 7.0 + bump, col);
    for y in 0..cap.h {
        for x in 0..cap.w {
            let fx = (x as f32 + 0.5 - hx) / c.k;
            let fy = (y as f32 + 0.5 - hy) / c.k;
            let face = fx > -1.2 && fy > -2.6 + (fx * 0.12).max(0.0);
            let below_ear = fx > -4.0 && fy > 1.0;
            if face || below_ear {
                cap.px[(y * cap.w + x) as usize] = [0, 0, 0, 0];
            }
        }
    }
    c.c.draw(&cap, 0, 0);
    let tri = |c: &mut Hd, a: P2, b: P2, t: P2, cl: Px| c.poly(&[(hx + a.0, hy + a.1), (hx + b.0, hy + b.1), (hx + t.0, hy + t.1)], cl);
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
        let x = (hx + dx as f32 * HEAD_K).round() as i32;
        let y = (hy + (dy as f32 + if look.hair == 5 { 1.0 } else { 0.0 }) * HEAD_K).round() as i32;
        let px = c.get(x, y);
        let close = (px[0] as i32 - base[0] as i32).abs() + (px[1] as i32 - base[1] as i32).abs() + (px[2] as i32 - base[2] as i32).abs() < 90;
        if px[3] > 0 && close {
            c.set(x, y, shine);
        }
    }
}

// =================================================================== hats and backs

fn draw_hat(c: &mut Hd, look: &Appearance, pal: &Pal, (hx, hy): P2, p: &Pose) {
    let tr = p.hair_trail;
    let lf = p.hair_lift;
    match look.hat {
        1 => {
            // bandana: a band over the brow, knot tails flutter behind
            let red = rgb(196, 60, 64);
            c.poly(&[(hx - 8.8, hy - 4.6), (hx + 7.4, hy - 6.4), (hx + 7.6, hy - 3.8), (hx - 8.6, hy - 2.0)], red);
            c.poly(&[(hx - 8.8, hy - 4.6), (hx + 7.4, hy - 6.4), (hx + 7.4, hy - 5.6), (hx - 8.8, hy - 3.8)], light(red, 0.2));
            let knot = (hx - 9.0, hy - 3.4);
            c.taper(knot, (knot.0 - 5.0 - tr * 3.0, knot.1 + 3.0 - lf * 4.0), 2.4, 1.2, shadow(red, 0.12));
            c.taper(knot, (knot.0 - 3.5 - tr * 2.0, knot.1 + 5.0 - lf * 3.0), 2.2, 1.0, red);
        }
        2 => {
            // leather cap with a short peak
            let col = rgb(150, 100, 60);
            c.poly(&[(hx - 9.4, hy - 3.0), (hx - 7.0, hy - 9.0), (hx - 1.0, hy - 10.6), (hx + 5.0, hy - 9.0), (hx + 7.6, hy - 4.6), (hx - 9.4, hy - 2.6)], col);
            c.poly(&[(hx + 3.0, hy - 5.4), (hx + 11.4, hy - 4.6), (hx + 10.6, hy - 3.2), (hx + 3.0, hy - 3.8)], shadow(col, 0.18));
            c.poly(&[(hx - 6.0, hy - 8.6), (hx - 1.0, hy - 10.0), (hx - 1.0, hy - 9.0), (hx - 6.0, hy - 7.6)], light(col, 0.25));
        }
        3 => {
            // pointed sage hat, tip bending back
            let col = rgb(78, 78, 170);
            let tip = (hx - 8.0 - tr * 3.0, hy - 20.0 + lf * 2.0);
            c.poly(&[(hx - 7.4, hy - 6.4), (hx + 5.4, hy - 7.0), (hx - 1.0, hy - 15.0), tip, (hx - 4.0, hy - 13.0)], col);
            c.ellipse(hx - 1.0, hy - 6.4, 11.4, 2.4, shadow(col, 0.1));
            c.poly(&[(hx - 7.0, hy - 8.2), (hx + 5.0, hy - 8.8), (hx + 5.0, hy - 7.0), (hx - 7.0, hy - 6.4)], rgb(230, 196, 90));
        }
        4 => {
            // iron sallet: dome, neck guard, a ridge
            let st = rgb(176, 184, 200);
            c.poly(&[(hx - 10.0, hy + 3.0), (hx - 9.6, hy - 5.0), (hx - 6.0, hy - 10.4), (hx + 1.0, hy - 11.4), (hx + 6.4, hy - 8.6), (hx + 8.4, hy - 4.2), (hx + 2.0, hy - 3.6), (hx - 3.0, hy - 2.0), (hx - 6.0, hy + 3.6)], st);
            c.poly(&[(hx - 4.0, hy - 10.8), (hx + 1.0, hy - 11.6), (hx + 1.0, hy - 10.4), (hx - 4.0, hy - 9.8)], light(st, 0.4));
            c.poly(&[(hx - 1.0, hy - 4.0), (hx + 8.4, hy - 4.8), (hx + 8.6, hy - 3.4), (hx - 1.0, hy - 2.6)], shadow(st, 0.22));
            for i in 0..3 {
                c.disc(hx - 7.0 + i as f32 * 3.0, hy - 4.0, 0.6, rgb(240, 244, 252));
            }
        }
        5 => {
            // feathered hat with a wide brim
            let col = rgb(150, 104, 64);
            c.ellipse(hx - 1.0, hy - 9.4, 7.0, 4.0, col);
            c.ellipse(hx - 0.5, hy - 6.4, 12.4, 2.2, shadow(col, 0.1));
            c.poly(&[(hx - 7.6, hy - 8.0), (hx + 5.8, hy - 8.4), (hx + 6.0, hy - 6.8), (hx - 7.6, hy - 6.6)], rgb(196, 60, 64));
            let q = (hx - 6.0, hy - 9.0);
            c.taper(q, (q.0 - 7.0 - tr * 3.0, q.1 - 7.0 + lf * 3.0), 2.6, 0.8, rgb(244, 244, 250));
        }
        6 => {
            // hood over the hair, open at the face, falling to the shoulders
            let col = rgb(70, 120, 76);
            let mut tmp = Canvas::new(c.c.w, c.c.h);
            {
                let mut t = Hd { c: &mut tmp, o: c.o, k: c.k };
                t.ellipse(hx - 1.6, hy - 2.4, 10.2, 9.6, col);
                t.poly(&[(hx - 11.0, hy), (hx - 2.0, hy + 4.0), (hx - 4.0, hy + 11.0), (hx - 12.0, hy + 9.0)], col);
                t.taper((hx - 9.0, hy - 6.0), (hx - 16.0 - tr * 3.0, hy + 2.0 - lf * 3.0), 4.0, 1.4, shadow(col, 0.1));
            }
            for y in 0..tmp.h {
                for x in 0..tmp.w {
                    let fx = (x as f32 + 0.5 - hx) / c.k;
                    let fy = (y as f32 + 0.5 - hy) / c.k;
                    if fx > -0.6 && fy > -5.8 + (fx * 0.1).max(0.0) && fy < 9.0 {
                        tmp.px[(y * tmp.w + x) as usize] = [0, 0, 0, 0];
                    }
                }
            }
            c.c.draw(&tmp, 0, 0);
        }
        7 => {
            // silver circlet with a blue stone
            c.poly(&[(hx - 9.0, hy - 5.2), (hx + 7.4, hy - 7.0), (hx + 7.4, hy - 5.8), (hx - 9.0, hy - 4.0)], rgb(214, 220, 236));
            c.disc(hx + 4.6, hy - 6.6, 1.6, rgb(80, 150, 240));
            c.set((hx + 4.0).round() as i32, (hy - 7.4).round() as i32, rgb(220, 240, 255));
        }
        8 => {
            // cat ears
            let col = pal.hair;
            let inner = rgb(240, 160, 176);
            c.poly(&[(hx - 8.0, hy - 6.0), (hx - 3.4, hy - 9.6), (hx - 8.4, hy - 15.0)], col);
            c.poly(&[(hx - 7.4, hy - 7.0), (hx - 4.6, hy - 9.0), (hx - 7.6, hy - 12.6)], inner);
            c.poly(&[(hx + 0.4, hy - 9.8), (hx + 5.6, hy - 7.2), (hx + 3.4, hy - 15.2)], col);
            c.poly(&[(hx + 1.6, hy - 9.6), (hx + 4.6, hy - 7.8), (hx + 3.4, hy - 12.6)], inner);
        }
        9 => {
            // flower crown
            let cols = [rgb(248, 150, 176), rgb(255, 230, 120), rgb(250, 250, 255), rgb(176, 140, 240)];
            for i in 0..8 {
                let a = std::f32::consts::PI * (1.05 + i as f32 * 0.12);
                let (x, y) = (hx - 1.0 + a.cos() * 8.6, hy - 3.4 + a.sin() * 6.4);
                c.disc(x, y, 1.8, cols[i % 4]);
                c.disc(x, y, 0.6, rgb(250, 200, 80));
            }
            c.disc(hx - 9.0, hy - 2.0, 1.2, rgb(110, 180, 90));
        }
        10 => {
            // starry halo floating above
            let gold = rgb(255, 226, 120);
            for i in 0..18 {
                let a = i as f32 / 18.0 * std::f32::consts::TAU;
                c.disc(hx - 1.0 + a.cos() * 6.4, hy - 14.0 + a.sin() * 1.6, 0.9, if a.sin() > 0.0 { gold } else { shadow(gold, 0.2) });
            }
            c.disc(hx - 7.0, hy - 15.0, 0.7, rgb(255, 255, 255));
            c.disc(hx + 5.0, hy - 13.0, 0.7, rgb(255, 255, 255));
        }
        11 => {
            // crown of gales: a little golden crown
            let gold = rgb(240, 196, 70);
            c.poly(&[(hx - 6.6, hy - 8.0), (hx + 4.6, hy - 9.0), (hx + 4.8, hy - 12.2), (hx + 2.4, hy - 10.6), (hx - 0.6, hy - 14.0), (hx - 2.4, hy - 10.2), (hx - 5.6, hy - 12.6)], gold);
            c.poly(&[(hx - 6.6, hy - 8.0), (hx + 4.6, hy - 9.0), (hx + 4.6, hy - 8.0), (hx - 6.6, hy - 7.0)], shadow(gold, 0.25));
            c.disc(hx - 0.6, hy - 10.0, 1.1, rgb(80, 200, 220));
        }
        _ => {}
    }
}

/// Cloaks, wings and packs, hung from the shoulders and fluttering with
/// movement. Drawn behind the body.
fn draw_back(c: &mut Canvas, look: &Appearance, pal: &Pal, r: &Rig, p: &Pose) {
    let tr = p.hair_trail;
    let lf = p.hair_lift;
    let cape = |c: &mut Canvas, len: f32, col: Px, stars: bool| {
        let top_b = r.tw(-4.6, 14.4);
        let top_f = r.tw(0.6, 15.0);
        let sway = tr * 7.0 + 2.5;
        let hem_b = r.t(-9.5 - sway - len * 0.12, 13.0 - len + lf * len * 0.4);
        let hem_f = r.t(-3.0 - sway * 0.6, 13.0 - len - 1.0 + lf * len * 0.3);
        c.poly(&[top_b, top_f, hem_f, hem_b], col);
        c.linef(lerp(top_b, top_f, 0.5), lerp(hem_b, hem_f, 0.5), shadow(col, 0.25));
        if stars {
            for i in 0..6 {
                let q = lerp(lerp(top_b, hem_b, 0.2 + i as f32 * 0.13), lerp(top_f, hem_f, 0.2 + i as f32 * 0.13), (i % 3) as f32 * 0.35 + 0.15);
                c.set(q.0.round() as i32, q.1.round() as i32, rgb(255, 240, 170));
            }
        }
        let clasp = r.tw(0.0, 14.8);
        c.disc(clasp.0, clasp.1, 1.3, rgb(230, 196, 90));
    };
    let wings = |c: &mut Canvas, col: Px, big: f32, bat: bool| {
        let root = r.tw(-3.4, 11.5);
        let flap = (lf * 18.0) + if p.plant == Plant::Air { 12.0 } else { 0.0 };
        // angles: 0 = down, negative = toward the back
        for (k, base) in [(0, -150.0f32), (1, -122.0), (2, -96.0)] {
            let a = base + flap * 0.6 - tr * 8.0;
            let len = (17.0 - k as f32 * 3.5) * big;
            let tip = add(root, dir(a), len);
            if bat {
                c.poly(&[root, tip, add(root, dir(a + 26.0), len * 0.7)], col);
            } else {
                c.taper(root, tip, 5.5 * big, 1.6, if k % 2 == 0 { col } else { shadow(col, 0.1) });
            }
        }
        if bat {
            for (k, base) in [(0, -150.0f32), (1, -122.0), (2, -96.0)] {
                let a = base + flap * 0.6 - tr * 8.0;
                let tip = add(root, dir(a), (17.0 - k as f32 * 3.5) * big);
                c.linef(root, tip, shadow(col, 0.4));
            }
        }
    };
    match look.back {
        1 => cape(c, 13.0, rgb(150, 66, 60), false),
        2 => cape(c, 22.0, rgb(58, 104, 186), false),
        3 => wings(c, rgb(244, 244, 252), 1.0, false),
        4 => {
            // travel pack with a bedroll
            let a = r.tw(-8.4, 3.0);
            let b = r.tw(-3.0, 12.6);
            let col = rgb(150, 104, 62);
            c.poly(&[a, (b.0, a.1), b, (a.0, b.1)], col);
            let roll = r.tw(-5.6, 13.6);
            c.ellipse(roll.0, roll.1, 4.0, 2.0, rgb(80, 120, 170));
            let flap = r.tw(-5.6, 9.0);
            c.disc(flap.0, flap.1, 1.0, rgb(230, 196, 90));
        }
        5 => wings(c, rgb(255, 255, 255), 1.45, false),
        6 => wings(c, rgb(90, 50, 110), 1.35, true),
        7 => cape(c, 24.0, rgb(40, 44, 96), true),
        _ => {}
    }
    let _ = pal;
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
            let drawn = p.back_arm.bend > 120.0;
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
        // ------------------------------------------------------------ 0.15.0
        Weapon::DualPistols => pistol(c, hand, add_angle(d, -50.0)),
        Weapon::Greatsword => {
            // a broad two-hander: wide blade, long grip, heavy crossguard
            let perp = (d.1, -d.0);
            let guard = add(hand, d, 2.5);
            let tip = add(hand, d, 22.0);
            c.taper(guard, tip, 5.0, 1.6, steel());
            c.linef(add(hand, d, 4.0), add(hand, d, 19.5), rgb(255, 255, 255));
            c.linef(add(add(hand, d, 6.0), perp, 1.6), add(add(hand, d, 17.0), perp, 1.0), rgb(150, 160, 180));
            c.capsule(add(guard, perp, -4.6), add(guard, perp, 4.6), 2.2, rgb(160, 60, 50));
            c.capsule(add(hand, d, -5.0), add(hand, d, 1.0), 1.9, rgb(70, 46, 40));
            let pommel = add(hand, d, -5.5);
            c.disc(pommel.0, pommel.1, 1.7, rgb(214, 170, 70));
        }
        Weapon::Greatshield => {
            // a short arming sword; the greatshield is on the other arm
            let guard = add(hand, d, 2.0);
            let tip = add(hand, d, 17.0);
            c.taper(guard, tip, 3.2, 1.2, steel());
            let perp = (d.1, -d.0);
            c.capsule(add(guard, perp, -2.8), add(guard, perp, 2.8), 1.8, rgb(232, 196, 90));
        }
        Weapon::Nunchucks => {
            // one stick in the hand, the other swinging on its chain
            let wood = rgb(60, 40, 34);
            let top = add(hand, d, 7.0);
            c.capsule(add(hand, d, -1.0), top, 1.5, wood);
            let swing = add_angle(d, 65.0 + p.weapon * 0.3);
            let link = add(top, d, 1.5);
            let end = add(link, swing, 3.0);
            c.linef(top, end, rgb(200, 200, 210));
            c.capsule(end, add(end, swing, 7.0), 1.5, wood);
            c.disc(top.0, top.1, 1.0, rgb(240, 204, 90));
        }
        Weapon::WarStaff => {
            // a long ironshod quarterstaff
            let base = add(hand, d, -11.0);
            let tip = add(hand, d, 18.0);
            c.capsule(base, tip, 2.2, rgb(96, 64, 40));
            for k in [-11.0f32, -7.5, 14.5, 18.0] {
                let q = add(hand, d, k);
                c.disc(q.0, q.1, 2.4, rgb(150, 156, 170));
            }
            let mid = add(hand, d, 4.0);
            c.disc(mid.0, mid.1, 1.4, rgb(200, 60, 50));
        }
        Weapon::Katana => {
            // a slim, gently curved blade
            let perp = (d.1, -d.0);
            let guard = add(hand, d, 2.0);
            let mid = add(add(hand, d, 11.0), perp, 0.8);
            let tip = add(add(hand, d, 20.0), perp, 2.2);
            c.taper(guard, mid, 2.2, 1.8, steel());
            c.taper(mid, tip, 1.8, 0.8, steel());
            c.linef(add(hand, d, 4.0), mid, rgb(255, 255, 255));
            c.disc(guard.0, guard.1, 1.8, rgb(40, 36, 44));
            c.capsule(add(hand, d, -4.0), add(hand, d, 1.0), 1.5, rgb(170, 40, 50));
        }
        Weapon::Kusarigama => {
            // a sickle, with its chain trailing behind
            let perp = (d.1, -d.0);
            let top = add(hand, d, 9.0);
            c.capsule(add(hand, d, -1.0), top, 1.5, rgb(70, 50, 40));
            let curve = [(0.0f32, 0.0f32), (2.0, 2.5), (5.0, 3.5), (8.0, 2.5), (10.0, 0.0)];
            for w in curve.windows(2) {
                let a = add(add(top, perp, w[0].0), d, w[0].1);
                let b = add(add(top, perp, w[1].0), d, w[1].1);
                c.capsule(a, b, 1.1, steel());
            }
            for k in 0..5 {
                let q = add(add(hand, d, -2.0 - k as f32 * 2.2), perp, -(k as f32) * 1.1);
                c.disc(q.0, q.1, 0.8, rgb(180, 184, 196));
            }
        }
        Weapon::Crossbow => {
            // stock along the arm, bow arms across the front
            let perp = (d.1, -d.0);
            let stock_end = add(hand, d, -5.0);
            let front = add(hand, d, 11.0);
            c.capsule(stock_end, front, 2.0, rgb(110, 72, 44));
            let a = add(add(front, perp, 7.0), d, -2.0);
            let b = add(add(front, perp, -7.0), d, -2.0);
            c.capsule(a, front, 1.4, rgb(70, 70, 84));
            c.capsule(b, front, 1.4, rgb(70, 70, 84));
            c.linef(a, add(hand, d, 4.0), rgb(236, 236, 236));
            c.linef(b, add(hand, d, 4.0), rgb(236, 236, 236));
            c.linef(add(hand, d, 4.0), add(front, d, 2.0), rgb(210, 220, 236));
        }
        Weapon::Greatbow => {
            let perp = (d.1, -d.0);
            let wood = rgb(84, 56, 40);
            let pts: Vec<P2> = (0..=12)
                .map(|i| {
                    let t = i as f32 / 12.0 * 2.0 - 1.0;
                    let bulge = (1.0 - t * t) * 5.0;
                    (hand.0 + perp.0 * t * 13.0 + d.0 * bulge, hand.1 + perp.1 * t * 13.0 + d.1 * bulge)
                })
                .collect();
            for i in 0..pts.len() - 1 {
                c.capsule(pts[i], pts[i + 1], 2.5, wood);
            }
            c.disc(pts[0].0, pts[0].1, 1.6, rgb(214, 170, 70));
            c.disc(pts[12].0, pts[12].1, 1.6, rgb(214, 170, 70));
            let drawn = p.back_arm.bend > 120.0;
            let nock = if drawn { add(back.hand, dir(back.fore_deg), 0.8) } else { add(hand, d, -1.0) };
            c.linef(pts[0], nock, rgb(238, 238, 238));
            c.linef(nock, pts[12], rgb(238, 238, 238));
            if drawn {
                c.linef(nock, add(hand, d, 8.0), rgb(186, 136, 84));
            }
        }
        Weapon::CosmicOrbs => {
            // glowing orbs floating just above the open hand
            let perp = (d.1, -d.0);
            for (k, col) in [(0.0f32, rgb(150, 200, 255)), (1.0, rgb(255, 210, 120))] {
                let q = add(add(hand, d, 4.0 + k * 3.0), perp, 3.0 + k * 3.5);
                c.disc(q.0, q.1, 2.8 - k * 0.6, col);
                c.set(q.0.round() as i32 - 1, q.1.round() as i32 - 1, rgb(255, 255, 255));
            }
        }
        Weapon::Cards => {
            // a fanned hand of cards
            let perp = (d.1, -d.0);
            for k in [-1.0f32, 0.0, 1.0] {
                let dd = add_angle(d, k * 18.0);
                let pp = (dd.1, -dd.0);
                let base = add(hand, dd, 1.5);
                let top = add(hand, dd, 7.0);
                c.poly(&[add(base, pp, -1.8), add(base, pp, 1.8), add(top, pp, 1.8), add(top, pp, -1.8)], rgb(250, 248, 240));
                let m = add(hand, dd, 4.5);
                c.set(m.0.round() as i32, m.1.round() as i32, if k == 0.0 { rgb(200, 40, 50) } else { rgb(30, 30, 40) });
            }
            let _ = perp;
        }
        Weapon::HandCannon => {
            // a stubby, thick-barrelled cannon with brass bands
            let back_end = add(hand, d, -3.0);
            let muzzle = add(hand, d, 12.0);
            c.capsule(back_end, muzzle, 3.4, rgb(70, 72, 84));
            for k in [1.0f32, 7.0] {
                let q = add(hand, d, k);
                c.disc(q.0, q.1, 3.6, rgb(200, 160, 70));
            }
            c.disc(muzzle.0, muzzle.1, 3.9, rgb(50, 50, 58));
            c.disc(muzzle.0, muzzle.1, 1.8, rgb(20, 20, 24));
        }
        Weapon::OmniRifle => {
            // a long rifle with a scope and a glowing core
            let perp = (d.1, -d.0);
            c.capsule(add(hand, d, -7.0), add(hand, d, 4.0), 2.0, rgb(80, 70, 90));
            c.capsule(add(hand, d, 4.0), add(hand, d, 18.0), 1.2, rgb(150, 156, 176));
            let sc = add(add(hand, d, 5.0), perp, -2.6);
            c.capsule(sc, add(sc, d, 6.0), 1.1, rgb(40, 40, 48));
            let core = add(hand, d, 2.0);
            c.disc(core.0, core.1, 1.3, rgb(120, 230, 255));
        }
    }
}

fn add_angle(d: P2, deg: f32) -> P2 {
    let (s, co) = deg.to_radians().sin_cos();
    (d.0 * co - d.1 * s, d.0 * s + d.1 * co)
}

fn pistol(c: &mut Canvas, hand: P2, d: P2) {
    // barrel forward, grip down into the hand
    let perp = (d.1, -d.0);
    let muzzle = add(add(hand, d, 8.0), perp, -2.0);
    let breech = add(hand, perp, -2.0);
    c.capsule(breech, muzzle, 1.5, rgb(64, 64, 76));
    c.capsule(hand, add(hand, perp, -2.4), 1.4, rgb(120, 76, 44));
    c.set(muzzle.0.round() as i32, muzzle.1.round() as i32, rgb(210, 220, 236));
    let cyl = add(add(hand, d, 2.0), perp, -2.0);
    c.disc(cyl.0, cyl.1, 1.4, rgb(214, 170, 70));
}

fn greatshield(c: &mut Canvas, at: P2) {
    // a tall tower shield with a gold cross
    c.ellipse(at.0, at.1, 6.6, 9.4, rgb(150, 150, 168));
    c.ellipse(at.0, at.1, 5.4, 8.2, rgb(230, 230, 240));
    c.rect(at.0.round() as i32 - 1, at.1.round() as i32 - 6, 2, 12, rgb(220, 180, 70));
    c.rect(at.0.round() as i32 - 4, at.1.round() as i32 - 2, 8, 2, rgb(220, 180, 70));
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
        let mut v = vec![Anim::Idle, Anim::Battle, Anim::Run, Anim::JumpUp, Anim::Fall, Anim::Dodge, Anim::Hurt, Anim::Dead, Anim::Talk];
        for m in [Motion::Jab, Motion::Thrust, Motion::Sweep, Motion::Uppercut, Motion::Spin, Motion::Spike, Motion::Slam, Motion::Kick, Motion::Cast, Motion::Shoot, Motion::Dive, Motion::Rise, Motion::Throw, Motion::Stomp, Motion::Flurry] {
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
            .chain(galebound_core::jobs::ALL_JOBS.iter().map(|j| Some(j.info().weapon)))
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

    /// Every hat and back item, in every animation, stays inside the canvas.
    #[test]
    fn hats_and_backs_fit() {
        for h in 0..galebound_core::appearance::HATS.len() as u8 {
            for b in 0..galebound_core::appearance::BACKS.len() as u8 {
                if h != 0 && b != 0 && h != b {
                    continue;
                }
                for body in 0..2u8 {
                    let look = Appearance { body, hair: 0, top: 1, bottom: 1, hat: h, back: b, ..Default::default() };
                    for a in all_anims() {
                        for f in 0..a.frames() {
                            let c = render(&look, Some(Weapon::Staff), a, f);
                            let bb = c.bounds().expect("sprite is empty");
                            assert!(bb.0 > 0 && bb.1 > 0 && bb.2 < SIZE - 1 && bb.3 < HEIGHT - 1, "hat {} back {} {:?} f{} touches edge {:?}", h, b, a, f, bb);
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
                for leg in [p.front_leg, p.back_leg] {
                    assert!(leg.bend >= 0.0 && leg.bend <= 160.0, "{:?} knee {}", a, leg.bend);
                }
                for arm in [p.front_arm, p.back_arm] {
                    assert!(arm.bend >= 0.0 && arm.bend <= 165.0, "{:?} elbow {}", a, arm.bend);
                }
            }
        }
    }

    /// Where a strike visibly lands: the tip of the weapon (or fist), or
    /// the foot for kicks and sweeps. In world units from the body's middle.
    fn strike_point(look: &Appearance, w: Option<Weapon>, m: Motion) -> P2 {
        let p = final_pose(look, w, Anim::Attack(m), 1);
        let r = build_rig(&p, look.feminine());
        let reach = match w {
            Some(Weapon::SwordShield) => 21.0,
            Some(Weapon::Club) => 16.0,
            Some(Weapon::Stick) => 17.0,
            Some(Weapon::WoodSword) => 18.0,
            Some(Weapon::TwinDaggers) => 11.0,
            Some(Weapon::Staff) => 18.0,
            _ => 2.5,
        };
        let pt = match m {
            Motion::Kick | Motion::Sweep | Motion::Dive | Motion::Stomp => add(r.fl.ankle, dir(r.fl.shin_deg), 3.0),
            _ => add(r.fa.hand, dir(r.fa.fore_deg + p.weapon), reach),
        };
        let middle = world_to_sprite((0.0, 0.0));
        ((pt.0 - middle.0) / PX_PER_UNIT, (pt.1 - middle.1) / PX_PER_UNIT)
    }

    /// Every melee strike's weapon, fist or foot lands inside the attack's
    /// real hitbox (`core/src/classes.rs`), for every class. If you edit a
    /// pose or a hitbox and this fails, the message says which one is off.
    #[test]
    fn strikes_land_in_their_hitboxes() {
        use galebound_core::classes::{attack_def, AttackKind::*};
        let looks = [Appearance { body: 0, ..Default::default() }, Appearance { body: 1, ..Default::default() }];
        let mut off = Vec::new();
        for &c in EVERY_CLASS {
            let w = Some(c.info().weapon);
            for k in [LightNeutral, LightSide, LightDown, AirNeutral, AirSide, AirDown, HeavyNeutral, HeavySide, HeavyDown, AirHeavyNeutral, AirHeavyDown] {
                let d = attack_def(c, k);
                if d.hw <= 0.0 || d.both_sides {
                    continue; // projectiles and all-round bursts
                }
                let m = motion_for(c, k);
                let (x0, x1) = (d.hx - d.hw * 0.25 - 4.0, d.hx + d.hw * 0.75 + 4.0);
                let (y0, y1) = (d.hy - d.hh * 0.5 - 4.0, d.hy + d.hh * 0.5 + 4.0);
                for look in &looks {
                    let (x, y) = strike_point(look, w, m);
                    if !(x >= x0 && x <= x1 && y >= y0 && y <= y1) {
                        off.push(format!(
                            "{:?} {:?} ({:?} pose): strike lands at ({:.0}, {:.0}) but the hitbox is x {:.0}..{:.0}, y {:.0}..{:.0}",
                            c, k, m, x, y, x0, x1, y0, y1
                        ));
                    }
                }
            }
        }
        off.dedup();
        assert!(off.is_empty(), "strikes outside their hitboxes:\n{}", off.join("\n"));
    }

    /// The relaxed idle keeps hands low; the battle stance raises them.
    #[test]
    fn idle_is_relaxed_and_battle_is_raised() {
        let look = Appearance::default();
        for w in [None, Some(Weapon::Gauntlets), Some(Weapon::SwordShield), Some(Weapon::TwinDaggers), Some(Weapon::Staff), Some(Weapon::Bow)] {
            for f in 0..4 {
                let idle = build_rig(&final_pose(&look, w, Anim::Idle, f), false);
                assert!(idle.fa.hand.1 > idle.pelvis.1 - 4.0, "{:?} idle hand too high", w);
                assert!(idle.ba.hand.1 > idle.pelvis.1 - 4.0, "{:?} idle back hand too high", w);
            }
        }
        for w in [None, Some(Weapon::Gauntlets)] {
            let battle = build_rig(&final_pose(&look, w, Anim::Battle, 0), false);
            assert!(battle.fa.hand.1 < battle.pelvis.1 - 8.0, "{:?} battle fists should be up", w);
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

/// Sprite sheet export for art review: `GB_SHEET=out.png cargo test --release sheet_export -- --ignored`
#[cfg(test)]
mod sheet {
    use super::*;
    use galebound_core::classes::ClassId;

    /// Draws every class in every pose into `pose_preview.png` (run it with
    /// preview-poses.bat). One row per class; the columns are:
    ///
    /// idle, idle (breathing), battle stance, battle (breathing), run x2,
    /// jump, fall, dodge, hurt, climb, then the HIT frame of each attack:
    /// light neutral, light side, light down, air neutral, air side,
    /// air down, heavy neutral, heavy side, heavy down, air heavy neutral,
    /// air heavy down.
    ///
    /// Attack cells show the real hitbox: red = where the attack hits.
    /// A good pose has the weapon, fist or foot inside the red box.
    #[test]
    #[ignore]
    fn pose_preview() {
        use galebound_core::classes::{attack_def, AttackKind::*, EVERY_CLASS};
        let path = std::env::var("GB_PREVIEW").unwrap_or("pose_preview.png".into());
        let scale = 2i32;
        let (cw, ch) = (144i32, 132i32);
        let feet_x = 40i32; // where the feet go inside a cell
        let looks = [
            Appearance { body: 0, hair: 0, hair_color: 5, face: 4, top: 6, top_dye: 1, bottom: 1, bottom_dye: 4, shoes: 0, skin: 2, ..Default::default() },
            Appearance { body: 1, hair: 2, hair_color: 9, face: 0, top: 7, top_dye: 8, bottom: 3, bottom_dye: 2, shoes: 6, skin: 1, ..Default::default() },
            Appearance { body: 0, hair: 8, hair_color: 0, face: 1, top: 3, top_dye: 8, bottom: 7, bottom_dye: 9, shoes: 1, skin: 6, ..Default::default() },
            Appearance { body: 1, hair: 3, hair_color: 3, face: 2, top: 1, top_dye: 3, bottom: 6, bottom_dye: 8, shoes: 1, skin: 0, ..Default::default() },
            Appearance { body: 0, hair: 5, hair_color: 2, face: 3, top: 2, top_dye: 5, bottom: 2, bottom_dye: 6, shoes: 3, skin: 4, ..Default::default() },
            Appearance { body: 1, hair: 6, hair_color: 7, face: 5, top: 4, top_dye: 7, bottom: 4, bottom_dye: 1, shoes: 2, skin: 3, ..Default::default() },
        ];
        let plain: Vec<(Anim, u8)> = vec![
            (Anim::Idle, 0), (Anim::Idle, 2), (Anim::Battle, 0), (Anim::Battle, 2), (Anim::Run, 1), (Anim::Run, 5),
            (Anim::JumpUp, 1), (Anim::Fall, 1), (Anim::Dodge, 0), (Anim::Hurt, 0), (Anim::Climb, 0),
        ];
        let kinds = [LightNeutral, LightSide, LightDown, AirNeutral, AirSide, AirDown, HeavyNeutral, HeavySide, HeavyDown, AirHeavyNeutral, AirHeavyDown];
        let cols = plain.len() + kinds.len();
        let rows = EVERY_CLASS.len();
        let (w, h) = (cw * cols as i32, ch * rows as i32);
        let mut img = vec![0u8; (w * h * 4) as usize];
        fn put(img: &mut [u8], w: i32, h: i32, x: i32, y: i32, p: Px) {
            if x >= 0 && y >= 0 && x < w && y < h {
                let k = ((y * w + x) * 4) as usize;
                img[k..k + 4].copy_from_slice(&p);
            }
        }
        for (r, &class) in EVERY_CLASS.iter().enumerate() {
            let look = &looks[r % looks.len()];
            let weapon = Some(class.info().weapon);
            for c in 0..cols {
                let (ox, oy) = (c as i32 * cw, r as i32 * ch);
                // checkered background, a little darker for attack cells
                for y in 0..ch {
                    for x in 0..cw {
                        let even = (x / 8 + y / 8) % 2 == 0;
                        let mut bg: Px = if even { [214, 228, 240, 255] } else { [200, 216, 232, 255] };
                        if c >= plain.len() {
                            bg = shadow(bg, 0.06);
                        }
                        if x == cw - 1 || y == ch - 1 {
                            bg = [150, 160, 180, 255];
                        }
                        put(&mut img, w, h, ox + x, oy + y, bg);
                    }
                }
                let (anim, frame, hitbox) = if c < plain.len() {
                    (plain[c].0, plain[c].1, None)
                } else {
                    let k = kinds[c - plain.len()];
                    let d = attack_def(class, k);
                    let hb = if d.hw > 0.0 && d.hh > 0.0 { Some(d) } else { None };
                    (Anim::Attack(motion_for(class, k)), 1, hb)
                };
                let shift = feet_x - FEET_X;
                if let Some(d) = &hitbox {
                    // the hitbox, as attack_hitbox() places it when facing right
                    let cx = if d.both_sides { d.hx } else { d.hx + d.hw * 0.25 };
                    let a = world_to_sprite((cx - d.hw * 0.5, d.hy - d.hh * 0.5));
                    let b = world_to_sprite((cx + d.hw * 0.5, d.hy + d.hh * 0.5));
                    let (x0, y0, x1, y1) = (a.0.round() as i32 + shift, a.1.round() as i32, b.0.round() as i32 + shift, b.1.round() as i32);
                    for y in y0.max(0)..=y1.min(ch - 2) {
                        for x in x0.max(0)..=x1.min(cw - 2) {
                            let edge = x == x0 || x == x1 || y == y0 || y == y1;
                            let k = (((oy + y) * w + ox + x) * 4) as usize;
                            let under: Px = [img[k], img[k + 1], img[k + 2], 255];
                            put(&mut img, w, h, ox + x, oy + y, if edge { [230, 40, 50, 255] } else { mix(under, [255, 90, 90, 255], 0.22) });
                        }
                    }
                }
                let sprite = render(look, weapon, anim, frame);
                for y in 0..sprite.h {
                    for x in 0..sprite.w {
                        let p = sprite.get(x, y);
                        if p[3] > 0 && x + shift < cw - 1 && x + shift >= 0 && y < ch - 1 {
                            put(&mut img, w, h, ox + x + shift, oy + y, p);
                        }
                    }
                }
            }
        }
        // scale up so it's easy to look at
        let (sw, sh) = (w * scale, h * scale);
        let mut big = vec![0u8; (sw * sh * 4) as usize];
        for y in 0..sh {
            for x in 0..sw {
                let s = (((y / scale) * w + x / scale) * 4) as usize;
                let d = ((y * sw + x) * 4) as usize;
                big[d..d + 4].copy_from_slice(&img[s..s + 4]);
            }
        }
        std::fs::write(&path, crate::pixel::png_rgba(sw as u32, sh as u32, &big)).unwrap();
        println!("wrote {}", path);
    }
    #[test]
    #[ignore]
    fn sheet_export() {
        let path = std::env::var("GB_SHEET").unwrap_or("sheet.png".into());
        let scale = std::env::var("GB_SCALE").ok().and_then(|s| s.parse().ok()).unwrap_or(3i32);
        if std::env::var("GB_GEAR").is_ok() {
            // every hat (idle) and every back item (running), on one sheet
            let mut cells: Vec<Canvas> = Vec::new();
            for h in 1..galebound_core::appearance::HATS.len() as u8 {
                let l = Appearance { body: (h % 2), hair: h % 10, hair_color: h % 10, face: h % 6, top: 1 + h % 8, top_dye: h % 10, bottom: 1 + h % 7, bottom_dye: (h + 3) % 10, shoes: h % 7, skin: h % 8, hat: h, back: 0 };
                cells.push(render(&l, None, Anim::Idle, 0));
            }
            for b in 1..galebound_core::appearance::BACKS.len() as u8 {
                let l = Appearance { body: (b % 2), hair: b % 10, hair_color: (b + 4) % 10, face: b % 6, top: 1 + b % 8, top_dye: b % 10, bottom: 1 + b % 7, bottom_dye: (b + 3) % 10, shoes: b % 7, skin: (b + 3) % 8, hat: 0, back: b };
                cells.push(render(&l, None, Anim::Idle, 0));
                cells.push(render(&l, None, Anim::Run, 2));
            }
            let cols = 9;
            let rows = cells.len().div_ceil(cols) as i32;
            let (w, h) = (SIZE * cols as i32 * scale, HEIGHT * rows * scale);
            let mut out = vec![0u8; (w * h * 4) as usize];
            for (i, c) in cells.iter().enumerate() {
                let (cx, cy) = ((i % cols) as i32 * SIZE, (i / cols) as i32 * HEIGHT);
                for y in 0..c.h {
                    for x in 0..c.w {
                        let mut p = c.get(x, y);
                        if p[3] == 0 {
                            p = if (x / 8 + y / 8) % 2 == 0 { [214, 228, 240, 255] } else { [200, 216, 232, 255] };
                        }
                        for sy in 0..scale {
                            for sx in 0..scale {
                                let k = ((((cy + y) * scale + sy) * w + (cx + x) * scale + sx) * 4) as usize;
                                out[k..k + 4].copy_from_slice(&p);
                            }
                        }
                    }
                }
            }
            std::fs::write(&path, crate::pixel::png_rgba(w as u32, h as u32, &out)).unwrap();
            return;
        }
        let looks = [
            Appearance { body: 0, hair: 8, hair_color: 0, face: 1, top: 3, top_dye: 8, bottom: 7, bottom_dye: 9, shoes: 1, skin: 6, ..Default::default() },
            Appearance { body: 1, hair: 2, hair_color: 9, face: 0, top: 7, top_dye: 8, bottom: 3, bottom_dye: 2, shoes: 6, skin: 1, ..Default::default() },
            Appearance { body: 1, hair: 3, hair_color: 3, face: 2, top: 3, top_dye: 8, bottom: 6, bottom_dye: 8, shoes: 1, skin: 0, ..Default::default() },
            Appearance { body: 0, hair: 0, hair_color: 5, face: 4, top: 6, top_dye: 1, bottom: 1, bottom_dye: 4, shoes: 0, skin: 2, ..Default::default() },
        ];
        let weapons = [None, None, Some(ClassId::Aethermancer.info().weapon), Some(ClassId::Vanguard.info().weapon)];
        let mut frames: Vec<(Anim, u8)> = Vec::new();
        for f in 0..Anim::Idle.frames() { frames.push((Anim::Idle, f)); }
        for f in 0..Anim::Run.frames() { frames.push((Anim::Run, f)); }
        for f in 0..4 { frames.push((Anim::Attack(Motion::Jab), f)); }
        for f in 0..4 { frames.push((Anim::Attack(Motion::Kick), f)); }
        frames.push((Anim::JumpUp, 0));
        frames.push((Anim::Fall, 1));
        let (cw, ch) = (SIZE, HEIGHT);
        let w = cw * frames.len() as i32;
        let h = ch * looks.len() as i32;
        let mut out = vec![0u8; (w * h * 4 * scale * scale) as usize];
        for (r, look) in looks.iter().enumerate() {
            for (i, &(a, f)) in frames.iter().enumerate() {
                let c = render(look, weapons[r], a, f);
                for y in 0..c.h {
                    for x in 0..c.w {
                        let mut p = c.get(x, y);
                        if p[3] == 0 {
                            p = if (x / 8 + y / 8) % 2 == 0 { [214, 228, 240, 255] } else { [200, 216, 232, 255] };
                        }
                        for sy in 0..scale {
                            for sx in 0..scale {
                                let ox = (i as i32 * cw + x) * scale + sx;
                                let oy = (r as i32 * ch + y) * scale + sy;
                                let k = ((oy * w * scale + ox) * 4) as usize;
                                out[k..k + 4].copy_from_slice(&p);
                            }
                        }
                    }
                }
            }
        }
        std::fs::write(&path, crate::pixel::png_rgba((w * scale) as u32, (h * scale) as u32, &out)).unwrap();
    }
}

