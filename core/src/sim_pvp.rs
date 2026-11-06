//! The client side of admins and duels (0.14.0). A child module of `sim`.
//!
//! Admins: the server tells everyone who has the admin role (for the badge
//! by their name) and sends admins the results of their commands. When an
//! admin acts on you (teleport, heal, level, god mode) the server sends an
//! `AdminDo` that your game carries out.
//!
//! Duels: while fighting, your attacks and projectiles also hit your
//! opponent; your game reports each hit, and theirs applies it. Duels never
//! kill: at 0 HP you stay on 1 HP and yield.

use super::*;
use crate::net::{AdminRow, DUEL_COUNTDOWN, DUEL_FIGHT, DUEL_OVER};

/// Duel hits do this fraction of the damage they'd do to a monster, so
/// fights last a while.
pub const DUEL_DAMAGE: f32 = 0.35;

#[derive(Clone, Debug, PartialEq)]
pub struct DuelState {
    pub foe: u32,
    pub name: String,
    pub phase: u32,
    pub secs: f32,
    /// We've yielded and are waiting for the server to end it.
    pub yielded: bool,
}

/// A hit-list tag for the duel opponent (monster ids are small numbers).
fn foe_tag(id: u32) -> u32 {
    u32::MAX - id
}

impl Sim {
    /// Does this player have the admin role?
    pub fn is_admin_name(&self, name: &str) -> bool {
        self.admins.iter().any(|a| a.eq_ignore_ascii_case(name))
    }
    pub fn am_admin(&self) -> bool {
        self.online && self.is_admin_name(&self.save.name)
    }

    /// Send an admin command (the line without its slash).
    pub fn admin_cmd(&mut self, line: &str) {
        if self.online {
            self.outbox.push(ClientMsg::Admin { cmd: line.trim().trim_start_matches('/').to_string() });
        }
    }

    pub fn duel_fighting(&self) -> bool {
        self.duel.as_ref().map(|d| d.phase == DUEL_FIGHT && !d.yielded).unwrap_or(false)
    }

    /// The opponent's id and body, while a duel is on and they're here.
    pub(super) fn duel_foe(&self) -> Option<(u32, Aabb)> {
        if !self.duel_fighting() {
            return None;
        }
        let foe = self.duel.as_ref()?.foe;
        let o = self.others.iter().find(|o| o.id == foe && !o.dead && o.since < 2.0)?;
        Some((foe, Aabb::new(o.pos.x - PLAYER_W * 0.5, o.pos.y - PLAYER_H, PLAYER_W, PLAYER_H)))
    }

    pub(super) fn foe_tag_for(id: u32) -> u32 {
        foe_tag(id)
    }

    /// Our attack connected with the duel opponent: show it and report it.
    pub(super) fn duel_strike(&mut self, foe: u32, at: V2, base: f32, kb: V2, heavy: bool, crit_bonus: f32) {
        let st = self.player.stats;
        let crit = self.rng.f() * 100.0 < (st.crit_chance + crit_bonus).min(100.0);
        let var = self.rng.range(0.9, 1.1);
        let dmg = (base * DUEL_DAMAGE * var * if crit { st.crit_damage / 100.0 } else { 1.0 }).round().max(1.0);
        self.outbox.push(ClientMsg::DuelHit { target: foe, damage: dmg, kbx: kb.x, kby: kb.y });
        self.hitstop = if heavy { 0.07 } else { 0.035 };
        self.shake = self.shake.max(if heavy { 4.0 } else { 2.0 });
        self.burst(at, if crit { FxKind::Crit } else { FxKind::Hit }, if heavy { 12 } else { 7 }, 320.0);
        if self.settings.show_damage_numbers {
            self.float(V2::new(at.x, at.y - 30.0), format!("{}", dmg as i32), if crit { FloatKind::Crit } else { FloatKind::Damage });
        }
    }

    /// In a duel, a hit that would defeat you leaves you on 1 HP and yields.
    /// Returns true if it did.
    pub(super) fn duel_guard(&mut self) -> bool {
        if !self.duel_fighting() {
            return false;
        }
        self.player.hp = 1.0;
        if let Some(d) = self.duel.as_mut() {
            d.yielded = true;
        }
        self.social_cmd(net::SocialCmd::DuelYield);
        true
    }

    /// Give up the duel.
    pub fn duel_yield(&mut self) {
        if self.duel.as_ref().map(|d| d.phase != DUEL_OVER).unwrap_or(false) {
            self.social_cmd(net::SocialCmd::DuelYield);
        }
    }

    /// Admin and duel messages from the server.
    pub(super) fn pvp_apply(&mut self, m: ServerMsg) {
        match m {
            ServerMsg::Admins { names } => self.admins = names,
            ServerMsg::AdminPlayers(rows) => self.admin_rows = rows,
            ServerMsg::AdminLog { text } => {
                self.admin_log.push(text.clone());
                if self.admin_log.len() > 80 {
                    self.admin_log.remove(0);
                }
                self.push_line(net::CH_SYSTEM, "", "", &format!("[Admin] {}", text));
            }
            ServerMsg::AdminDo { action, map, x, y, n } => self.admin_do(&action, &map, x, y, n),
            ServerMsg::Duel { foe, name, phase, secs } => {
                if phase == DUEL_OVER {
                    let me = self.save.name.clone();
                    let (title, body) = if name.is_empty() {
                        ("Duel: a draw!", "Time ran out.")
                    } else if name.eq_ignore_ascii_case(&me) {
                        ("You won the duel!", "Well fought.")
                    } else {
                        ("You lost the duel", "Better luck next time.")
                    };
                    self.toast(title, body);
                    self.duel = None;
                    // both fighters get back on their feet
                    if self.player.state != PState::Dead {
                        self.player.hp = self.player.max_hp;
                        self.player.mp = self.player.max_mp;
                    }
                    return;
                }
                let started = phase == DUEL_FIGHT && self.duel.as_ref().map(|d| d.phase == DUEL_COUNTDOWN).unwrap_or(false);
                self.duel = Some(DuelState { foe, name: name.clone(), phase, secs, yielded: false });
                if phase == DUEL_COUNTDOWN {
                    self.toast(&format!("Duel with {}!", name), "Get ready...");
                } else if started {
                    self.toast("FIGHT!", &format!("First to drop loses. {} seconds.", secs.round() as i32));
                    self.shake = self.shake.max(4.0);
                }
            }
            ServerMsg::DuelHit { from, damage, kbx, kby } => {
                let Some(d) = &self.duel else { return };
                if d.phase != DUEL_FIGHT || d.foe != from || d.yielded {
                    return;
                }
                let fx = self.others.iter().find(|o| o.id == from).map(|o| o.pos.x).unwrap_or(self.player.body.center().x - kbx.signum());
                self.player.since_combat = 0.0;
                self.hurt_player(damage, fx, V2::new(kbx.abs(), kby), false);
            }
            _ => {}
        }
    }

    fn admin_do(&mut self, action: &str, map: &str, x: f32, y: f32, n: f32) {
        match action {
            "tp" => {
                let base = world::base_key(map).to_string();
                let inst = map.strip_prefix(&base).unwrap_or("").to_string();
                if world::load_map(&base).is_none() {
                    return;
                }
                if base == self.map.key && inst == self.instance {
                    self.player.body.pos = V2::new(x - PLAYER_W * 0.5, y - PLAYER_H);
                    self.player.body.vel = V2::ZERO;
                } else {
                    self.forced_instance = Some(inst);
                    self.change_map(&base, x);
                }
                self.player.invuln = self.player.invuln.max(1.5);
                let c = self.player.body.center();
                self.burst(c, FxKind::LevelUp, 24, 240.0);
            }
            "heal" => {
                if self.player.state == PState::Dead {
                    self.save.tomb = None;
                    self.stand_up(1.0);
                }
                self.player.hp = self.player.max_hp;
                self.player.mp = self.player.max_mp;
                let c = self.player.body.center();
                self.burst(c, FxKind::Heal, 24, 180.0);
                self.float(c, "Fully healed!".into(), FloatKind::Heal);
            }
            "level" => {
                let to = (n as u32).clamp(1, crate::admin::MAX_LEVEL);
                let from = self.save.level;
                if to == from {
                    return;
                }
                self.save.level = to;
                self.save.exp = 0;
                if to > from {
                    self.save.ap += (to - from) * stats::AP_PER_LEVEL;
                    self.learn_new_skills(from);
                } else {
                    // going down: hand back every point for the new level
                    self.save.stats = stats::START_STATS;
                    self.save.ap = (to - 1) * stats::AP_PER_LEVEL;
                }
                self.recompute_stats();
                self.player.hp = self.player.max_hp;
                self.player.mp = self.player.max_mp;
                self.toast(&format!("Level {}!", to), "An admin set your level. Spend your ability points in the Character window.");
                self.wants_save = true;
            }
            "god" => {
                self.god = n > 0.5;
                self.toast(if self.god { "God mode on" } else { "God mode off" }, if self.god { "Nothing can hurt you." } else { "" });
            }
            _ => {}
        }
    }

    /// Admin list rows (for the panel).
    pub fn admin_players(&self) -> &[AdminRow] {
        &self.admin_rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::appearance::Appearance;

    fn online() -> Sim {
        let mut save = CharacterSave::new("Ana", Appearance::default());
        save.map = "whistling_fields".into();
        let mut s = Sim::new(save, Settings::default(), 3);
        s.go_online(1);
        s.tick(&InputState::default());
        s
    }

    fn other(s: &mut Sim, id: u32, name: &str, x: f32) {
        s.net_apply(ServerMsg::PlayerInfo(net::PlayerInfo { id, name: name.into(), level: 10, ..Default::default() }));
        let f = s.player.body.feet();
        s.net_apply(ServerMsg::PlayerPos(net::PlayerPos { id, x, y: f.y, hp: 1.0, ..Default::default() }));
        s.tick(&InputState::default());
    }

    #[test]
    fn admin_badges_commands_and_actions() {
        let mut s = online();
        assert!(!s.am_admin());
        s.net_apply(ServerMsg::Admins { names: vec!["ana".into()] });
        assert!(s.am_admin());
        s.outbox.clear();
        s.admin_cmd("/give Bo elixir 3");
        assert_eq!(s.outbox.pop(), Some(ClientMsg::Admin { cmd: "give Bo elixir 3".into() }));
        // god mode stops damage
        s.net_apply(ServerMsg::AdminDo { action: "god".into(), map: String::new(), x: 0.0, y: 0.0, n: 1.0 });
        let hp = s.player.hp;
        s.player.invuln = 0.0;
        s.hurt_player(500.0, 0.0, V2::ZERO, false);
        assert_eq!(s.player.hp, hp);
        // level, heal, teleport
        s.net_apply(ServerMsg::AdminDo { action: "level".into(), map: String::new(), x: 0.0, y: 0.0, n: 12.0 });
        assert_eq!(s.save.level, 12);
        assert_eq!(s.save.ap, 11 * stats::AP_PER_LEVEL);
        s.net_apply(ServerMsg::AdminDo { action: "level".into(), map: String::new(), x: 0.0, y: 0.0, n: 3.0 });
        assert_eq!(s.save.ap, 2 * stats::AP_PER_LEVEL);
        s.net_apply(ServerMsg::AdminDo { action: "tp".into(), map: "ironhold".into(), x: 600.0, y: 640.0, n: 0.0 });
        assert_eq!(s.map.key, "ironhold");
        // into someone's house: their private copy
        s.net_apply(ServerMsg::AdminDo { action: "tp".into(), map: "player_house#Bo".into(), x: 300.0, y: 640.0, n: 0.0 });
        assert_eq!(s.net_map(), "player_house#Bo");
        s.net_apply(ServerMsg::AdminLog { text: "Kicked Bo".into() });
        assert_eq!(s.admin_log.last().unwrap(), "Kicked Bo");
    }

    #[test]
    fn duels_hit_the_opponent_and_never_kill() {
        let mut s = online();
        let f = s.player.body.feet();
        other(&mut s, 7, "Bo", f.x + 30.0);
        s.net_apply(ServerMsg::Duel { foe: 7, name: "Bo".into(), phase: DUEL_COUNTDOWN, secs: 3.0 });
        assert!(!s.duel_fighting());
        s.net_apply(ServerMsg::Duel { foe: 7, name: "Bo".into(), phase: DUEL_FIGHT, secs: 90.0 });
        assert!(s.duel_fighting());
        // our jab lands on Bo and is reported
        s.outbox.clear();
        s.player.facing = 1.0;
        let mut i = InputState::default();
        i.set(crate::input::Action::Light, true, true);
        s.tick(&i);
        for _ in 0..30 {
            s.tick(&InputState::default());
        }
        assert!(s.outbox.iter().any(|m| matches!(m, ClientMsg::DuelHit { target: 7, .. })), "no duel hit reported");
        // Bo's hits hurt, but a finishing blow leaves us on 1 HP and yields
        let hp = s.player.hp;
        s.player.invuln = 0.0;
        s.net_apply(ServerMsg::DuelHit { from: 7, damage: 10.0, kbx: 100.0, kby: -50.0 });
        assert!(s.player.hp < hp);
        s.outbox.clear();
        s.player.invuln = 0.0;
        s.net_apply(ServerMsg::DuelHit { from: 7, damage: 99_999.0, kbx: 100.0, kby: -50.0 });
        assert_eq!(s.player.hp, 1.0);
        assert_ne!(s.player.state, PState::Dead);
        assert!(s.outbox.contains(&ClientMsg::Social(net::SocialCmd::DuelYield)));
        // hits from anyone else are ignored
        let hp = s.player.hp;
        s.net_apply(ServerMsg::DuelHit { from: 9, damage: 50.0, kbx: 0.0, kby: 0.0 });
        assert_eq!(s.player.hp, hp);
        // the end: everyone back to full
        s.net_apply(ServerMsg::Duel { foe: 7, name: "Bo".into(), phase: DUEL_OVER, secs: 0.0 });
        assert!(s.duel.is_none());
        assert_eq!(s.player.hp, s.player.max_hp);
    }
}
