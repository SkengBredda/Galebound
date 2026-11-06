//! Game controllers, with no extra libraries:
//! - **Windows**: Xbox-style pads through XInput (loaded at run time).
//! - **Linux**: the kernel joystick API (`/dev/input/js0`..`js3`), with the
//!   usual Xbox-pad button layout.
//! - **Android**: pads connected by Bluetooth or USB. Our patched miniquad
//!   passes their buttons as keys F13-F25 and turns sticks / hats into
//!   D-pad arrows.
//!
//! Layout: left stick / D-pad move, A jump, X light attack, Y heavy attack,
//! B dodge, RB talk, LB pick up, L3 potion, R3 quest log, Start menu,
//! Back / Select bag. Hold LT + A/B/X/Y for hotbar 1-4, RT + A/B/X/Y for 5-8.

use galebound_core::input::Action;
use std::cell::RefCell;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Btn {
    A,
    B,
    X,
    Y,
    Lb,
    Rb,
    Lt,
    Rt,
    Back,
    Start,
    Ls,
    Rs,
    Up,
    Down,
    Left,
    Right,
}

const N: usize = 16;

#[derive(Default)]
struct Pad {
    now: [bool; N],
    prev: [bool; N],
    /// Seen a controller since the game started?
    seen: bool,
    backend: Backend,
}

thread_local! {
    static PAD: RefCell<Pad> = RefCell::new(Pad::default());
}

/// Read the controller. Call once per frame, before reading buttons.
pub fn update() {
    PAD.with(|p| {
        let mut p = p.borrow_mut();
        p.prev = p.now;
        let mut now = [false; N];
        let any = p.backend.poll(&mut now);
        if any || now.iter().any(|b| *b) {
            p.seen = true;
        }
        p.now = now;
    });
}

pub fn down(b: Btn) -> bool {
    PAD.with(|p| p.borrow().now[b as usize])
}

pub fn pressed(b: Btn) -> bool {
    PAD.with(|p| {
        let p = p.borrow();
        p.now[b as usize] && !p.prev[b as usize]
    })
}

/// Is a controller in use?
#[allow(dead_code)]
pub fn connected() -> bool {
    PAD.with(|p| p.borrow().seen)
}

/// Menu helpers (D-pad or stick, A to choose, B / Start to go back).
pub fn nav_up() -> bool {
    pressed(Btn::Up)
}
pub fn nav_down() -> bool {
    pressed(Btn::Down)
}
pub fn nav_ok() -> bool {
    pressed(Btn::A)
}
pub fn nav_back() -> bool {
    pressed(Btn::B) || pressed(Btn::Start)
}

/// What the controller is doing to one game action: (held, pressed).
pub fn action(a: Action) -> (bool, bool) {
    let (lt, rt) = (down(Btn::Lt), down(Btn::Rt));
    let face = |b: Btn| (down(b), pressed(b));
    let plain = |b: Btn| if lt || rt { (false, false) } else { face(b) };
    let shift = |want_rt: bool, b: Btn| if (want_rt && rt) || (!want_rt && lt && !rt) { face(b) } else { (false, false) };
    let none = (false, false);
    match a {
        Action::Left => face(Btn::Left),
        Action::Right => face(Btn::Right),
        Action::Up => face(Btn::Up),
        Action::Down => face(Btn::Down),
        Action::Jump => plain(Btn::A),
        Action::Light => plain(Btn::X),
        Action::Heavy => plain(Btn::Y),
        Action::Dodge => plain(Btn::B),
        Action::Interact => face(Btn::Rb),
        Action::Pickup => face(Btn::Lb),
        Action::Potion => face(Btn::Ls),
        Action::QuestLog => face(Btn::Rs),
        Action::Menu => face(Btn::Start),
        Action::Inventory => face(Btn::Back),
        Action::Hotbar1 => shift(false, Btn::A),
        Action::Hotbar2 => shift(false, Btn::B),
        Action::Hotbar3 => shift(false, Btn::X),
        Action::Hotbar4 => shift(false, Btn::Y),
        Action::Hotbar5 => shift(true, Btn::A),
        Action::Hotbar6 => shift(true, Btn::B),
        Action::Hotbar7 => shift(true, Btn::X),
        Action::Hotbar8 => shift(true, Btn::Y),
        _ => none,
    }
}

// ------------------------------------------------------------------ backends

/// Analog stick -> D-pad directions (with a dead zone).
#[cfg_attr(target_os = "android", allow(dead_code))]
fn stick_dirs(now: &mut [bool; N], x: f32, y: f32) {
    now[Btn::Left as usize] |= x < -0.45;
    now[Btn::Right as usize] |= x > 0.45;
    now[Btn::Up as usize] |= y < -0.6;
    now[Btn::Down as usize] |= y > 0.6;
}

#[cfg(target_os = "android")]
#[derive(Default)]
struct Backend;

#[cfg(target_os = "android")]
impl Backend {
    fn poll(&mut self, now: &mut [bool; N]) -> bool {
        use macroquad::prelude::{is_key_down, KeyCode as K};
        let map = [
            (K::F13, Btn::A),
            (K::F14, Btn::B),
            (K::F15, Btn::X),
            (K::F16, Btn::Y),
            (K::F17, Btn::Lb),
            (K::F18, Btn::Rb),
            (K::F19, Btn::Lt),
            (K::F20, Btn::Rt),
            (K::F21, Btn::Ls),
            (K::F22, Btn::Rs),
            (K::F23, Btn::Start),
            (K::F24, Btn::Back),
            (K::Back, Btn::Start),
            (K::Up, Btn::Up),
            (K::Down, Btn::Down),
            (K::Left, Btn::Left),
            (K::Right, Btn::Right),
        ];
        for (k, b) in map {
            now[b as usize] |= is_key_down(k);
        }
        false
    }
}

#[cfg(windows)]
#[derive(Default)]
struct Backend {
    xinput: Option<xinput::XInput>,
    tried: bool,
    slot: Option<u32>,
    rescan: f32,
}

#[cfg(windows)]
impl Backend {
    fn poll(&mut self, now: &mut [bool; N]) -> bool {
        if !self.tried {
            self.tried = true;
            self.xinput = xinput::XInput::load();
        }
        let Some(x) = &self.xinput else { return false };
        // Asking empty slots is slow, so only look for a pad now and then.
        if self.slot.is_none() {
            self.rescan -= macroquad::prelude::get_frame_time();
            if self.rescan > 0.0 {
                return false;
            }
            self.rescan = 2.0;
            self.slot = (0..4).find(|&i| x.state(i).is_some());
        }
        let Some(slot) = self.slot else { return false };
        let Some(g) = x.state(slot) else {
            self.slot = None;
            return false;
        };
        let bit = |m: u16| g.buttons & m != 0;
        let set = [
            (0x1000, Btn::A),
            (0x2000, Btn::B),
            (0x4000, Btn::X),
            (0x8000, Btn::Y),
            (0x0100, Btn::Lb),
            (0x0200, Btn::Rb),
            (0x0020, Btn::Back),
            (0x0010, Btn::Start),
            (0x0040, Btn::Ls),
            (0x0080, Btn::Rs),
            (0x0001, Btn::Up),
            (0x0002, Btn::Down),
            (0x0004, Btn::Left),
            (0x0008, Btn::Right),
        ];
        for (m, b) in set {
            now[b as usize] |= bit(m);
        }
        now[Btn::Lt as usize] |= g.lt > 100;
        now[Btn::Rt as usize] |= g.rt > 100;
        stick_dirs(now, g.lx as f32 / 32767.0, -(g.ly as f32) / 32767.0);
        true
    }
}

#[cfg(windows)]
mod xinput {
    use std::ffi::c_void;

    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    pub struct Gamepad {
        pub buttons: u16,
        pub lt: u8,
        pub rt: u8,
        pub lx: i16,
        pub ly: i16,
        pub rx: i16,
        pub ry: i16,
    }

    #[repr(C)]
    #[derive(Default)]
    struct State {
        packet: u32,
        pad: Gamepad,
    }

    type GetState = unsafe extern "system" fn(u32, *mut State) -> u32;

    #[link(name = "kernel32")]
    extern "system" {
        fn LoadLibraryA(name: *const u8) -> *mut c_void;
        fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
    }

    pub struct XInput {
        get: GetState,
    }

    impl XInput {
        /// Find XInput in whichever version Windows has.
        pub fn load() -> Option<XInput> {
            for dll in [&b"xinput1_4.dll\0"[..], &b"xinput1_3.dll\0"[..], &b"xinput9_1_0.dll\0"[..]] {
                unsafe {
                    let m = LoadLibraryA(dll.as_ptr());
                    if m.is_null() {
                        continue;
                    }
                    let f = GetProcAddress(m, b"XInputGetState\0".as_ptr());
                    if !f.is_null() {
                        return Some(XInput { get: std::mem::transmute::<*mut c_void, GetState>(f) });
                    }
                }
            }
            None
        }
        pub fn state(&self, slot: u32) -> Option<Gamepad> {
            let mut s = State::default();
            let r = unsafe { (self.get)(slot, &mut s) };
            (r == 0).then_some(s.pad)
        }
    }
}

#[cfg(target_os = "linux")]
#[derive(Default)]
struct Backend {
    dev: Option<std::fs::File>,
    buttons: [bool; 16],
    axes: [f32; 8],
    rescan: f32,
}

#[cfg(target_os = "linux")]
impl Backend {
    fn poll(&mut self, now: &mut [bool; N]) -> bool {
        use std::io::Read;
        use std::os::unix::fs::OpenOptionsExt;
        if self.dev.is_none() {
            self.rescan -= macroquad::prelude::get_frame_time();
            if self.rescan > 0.0 {
                return false;
            }
            self.rescan = 2.0;
            const O_NONBLOCK: i32 = 0o4000;
            self.dev = (0..4).find_map(|i| std::fs::OpenOptions::new().read(true).custom_flags(O_NONBLOCK).open(format!("/dev/input/js{}", i)).ok());
            self.buttons = [false; 16];
            self.axes = [0.0; 8];
        }
        let Some(dev) = self.dev.as_mut() else { return false };
        // struct js_event { u32 time; i16 value; u8 type; u8 number; }
        let mut buf = [0u8; 8 * 32];
        loop {
            match dev.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    for e in buf[..n].chunks_exact(8) {
                        let value = i16::from_le_bytes([e[4], e[5]]);
                        let (kind, num) = (e[6] & 0x7f, e[7] as usize);
                        match kind {
                            1 if num < 16 => self.buttons[num] = value != 0,
                            2 if num < 8 => self.axes[num] = value as f32 / 32767.0,
                            _ => {}
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => {
                    self.dev = None; // unplugged
                    return false;
                }
            }
        }
        // the Linux xpad layout (Xbox and most modern pads)
        let b = &self.buttons;
        let map = [(0, Btn::A), (1, Btn::B), (2, Btn::X), (3, Btn::Y), (4, Btn::Lb), (5, Btn::Rb), (6, Btn::Back), (7, Btn::Start), (9, Btn::Ls), (10, Btn::Rs)];
        for (i, btn) in map {
            now[btn as usize] |= b[i];
        }
        let a = &self.axes;
        now[Btn::Lt as usize] |= a[2] > 0.0;
        now[Btn::Rt as usize] |= a[5] > 0.0;
        stick_dirs(now, a[0], a[1]);
        stick_dirs(now, a[6], a[7]);
        true
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "android")))]
#[derive(Default)]
struct Backend;

#[cfg(not(any(windows, target_os = "linux", target_os = "android")))]
impl Backend {
    fn poll(&mut self, _now: &mut [bool; N]) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(btns: &[Btn], prev: &[Btn]) {
        PAD.with(|p| {
            let mut p = p.borrow_mut();
            p.now = [false; N];
            p.prev = [false; N];
            for b in btns {
                p.now[*b as usize] = true;
            }
            for b in prev {
                p.prev[*b as usize] = true;
            }
        });
    }

    #[test]
    fn controller_layout() {
        set(&[Btn::A, Btn::Left], &[]);
        assert_eq!(action(Action::Jump), (true, true));
        assert_eq!(action(Action::Left), (true, true));
        assert_eq!(action(Action::Hotbar1), (false, false));
        // held from last frame: held, not pressed again
        set(&[Btn::A], &[Btn::A]);
        assert_eq!(action(Action::Jump), (true, false));
        // triggers turn the face buttons into hotbar keys
        set(&[Btn::Lt, Btn::X], &[Btn::Lt]);
        assert_eq!(action(Action::Hotbar3), (true, true));
        assert_eq!(action(Action::Light), (false, false), "no attack while picking a hotbar slot");
        set(&[Btn::Rt, Btn::Y], &[Btn::Rt]);
        assert_eq!(action(Action::Hotbar8), (true, true));
        assert_eq!(action(Action::Hotbar4), (false, false));
        set(&[Btn::Start, Btn::Rb, Btn::Lb], &[]);
        assert!(action(Action::Menu).1 && action(Action::Interact).1 && action(Action::Pickup).1);
        assert!(nav_back());
    }

    #[test]
    fn sticks_have_a_dead_zone() {
        let mut now = [false; N];
        stick_dirs(&mut now, 0.3, 0.5);
        assert!(now.iter().all(|b| !b));
        stick_dirs(&mut now, -0.9, 0.9);
        assert!(now[Btn::Left as usize] && now[Btn::Down as usize]);
    }
}
