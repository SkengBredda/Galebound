//! Client-side bits of playing online: the last server you used, the
//! "remember me" token a server gave you, and how animations are sent.

use crate::charsprite::{Anim, Motion};
use galebound_core::save::{data_dir, write_atomic};

fn cfg_path() -> std::path::PathBuf {
    data_dir().join("client.cfg")
}
fn remember_path() -> std::path::PathBuf {
    data_dir().join("remember_online.txt")
}

/// The server address typed last time ("localhost" the first time).
pub fn last_server() -> String {
    std::fs::read_to_string(cfg_path())
        .ok()
        .and_then(|t| t.lines().find_map(|l| l.strip_prefix("server=").map(|s| s.trim().to_string())))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "localhost".to_string())
}

pub fn set_last_server(addr: &str) {
    let _ = write_atomic(&cfg_path(), &format!("server={}\n", addr.trim()));
}

/// A saved "remember me" login: server, e-mail and the server's token
/// (never the password).
#[derive(Clone, Debug, PartialEq)]
pub struct Remembered {
    pub server: String,
    pub email: String,
    pub token: String,
}

pub fn remembered() -> Option<Remembered> {
    let t = std::fs::read_to_string(remember_path()).ok()?;
    let get = |k: &str| t.lines().find_map(|l| l.strip_prefix(k).map(|s| s.trim().to_string()));
    Some(Remembered { server: get("server=")?, email: get("email=")?, token: get("token=")? })
}

pub fn remember(r: &Remembered) {
    let body = format!(
        "# Galebound 'remember me'. Delete this file to sign out on this computer.\nserver={}\nemail={}\ntoken={}\n",
        r.server, r.email, r.token
    );
    let _ = write_atomic(&remember_path(), &body);
}

pub fn forget() {
    let _ = std::fs::remove_file(remember_path());
}

// New motions go at the END so older clients keep the same numbers.
const MOTIONS: [Motion; 14] = [
    Motion::Jab,
    Motion::Thrust,
    Motion::Sweep,
    Motion::Uppercut,
    Motion::Spin,
    Motion::Spike,
    Motion::Slam,
    Motion::Kick,
    Motion::Cast,
    Motion::Shoot,
    Motion::Dive,
    Motion::Rise,
    Motion::Throw,
    Motion::Stomp,
];

/// Animation -> number, for telling other players what we're doing.
pub fn anim_code(a: Anim) -> u32 {
    match a {
        Anim::Idle => 0,
        Anim::Run => 1,
        Anim::JumpUp => 2,
        Anim::Fall => 3,
        Anim::Dodge => 4,
        Anim::Hurt => 5,
        Anim::Dead => 6,
        Anim::Talk => 7,
        Anim::Climb => 8,
        Anim::Battle => 9,
        Anim::Attack(m) => 16 + MOTIONS.iter().position(|x| *x == m).unwrap_or(0) as u32,
    }
}

pub fn anim_from_code(c: u32) -> Anim {
    match c {
        1 => Anim::Run,
        2 => Anim::JumpUp,
        3 => Anim::Fall,
        4 => Anim::Dodge,
        5 => Anim::Hurt,
        6 => Anim::Dead,
        7 => Anim::Talk,
        8 => Anim::Climb,
        9 => Anim::Battle,
        c if c >= 16 => Anim::Attack(MOTIONS.get((c - 16) as usize).copied().unwrap_or(Motion::Jab)),
        _ => Anim::Idle,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn anim_codes_roundtrip() {
        let mut all = vec![Anim::Idle, Anim::Run, Anim::JumpUp, Anim::Fall, Anim::Dodge, Anim::Hurt, Anim::Dead, Anim::Talk, Anim::Climb, Anim::Battle];
        all.extend(MOTIONS.iter().map(|m| Anim::Attack(*m)));
        for a in all {
            assert_eq!(anim_from_code(anim_code(a)), a);
        }
    }
}
