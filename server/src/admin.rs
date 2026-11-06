//! Admins and duels, on the server.
//!
//! **Admins.** The host types `admin <name>` in the server window to give a
//! player the admin role (`unadmin <name>` takes it away, `admins` lists
//! them). Admins can pass the role on with `/admin <name>`. The list is kept
//! in `admins.txt` in the data folder. Admins run the commands in
//! `galebound_core::admin` from chat or the Admin Panel; every one is
//! checked here and written to the server window.
//!
//! **Duels.** A player challenges another on the same map; once accepted
//! there's a 3 second countdown, then up to 90 seconds of fighting. Each
//! side's game reports its own hits on the other (like hits on monsters);
//! the server checks they're really duelling and close enough, and passes
//! them on. Duels are never lethal: the loser's game yields at 1 HP.

use super::*;
use galebound_core::admin::{self, COMMANDS};
use galebound_core::net::{AdminRow, DUEL_COUNTDOWN, DUEL_FIGHT, DUEL_OVER, INV_DUEL};

/// Seconds of countdown before a duel starts, and the longest it can last.
pub const DUEL_COUNTDOWN_SECS: f32 = 3.0;
pub const DUEL_SECS: f32 = 90.0;
/// Duel hits are only passed on when the fighters are this close.
const DUEL_HIT_RANGE: f32 = 900.0;
/// The most one duel hit may do (sanity check on what clients send).
const DUEL_HIT_CAP: f32 = 5_000.0;
/// You can only challenge someone this close.
const DUEL_ASK_RANGE: f32 = 1_200.0;
/// The most monsters one /spawn makes.
const SPAWN_MAX: u32 = 10;

pub(crate) struct Duel {
    pub ids: [u32; 2],
    pub map: String,
    pub phase: u32,
    /// Seconds left in this phase.
    pub left: f32,
}

fn admins_path(dir: &std::path::Path) -> PathBuf {
    dir.join("admins.txt")
}

/// The admin list saved in the data folder (lowercase names, one a line).
pub(crate) fn load_admins(dir: &std::path::Path) -> HashSet<String> {
    std::fs::read_to_string(admins_path(dir)).map(|s| s.lines().map(|l| l.trim().to_lowercase()).filter(|l| !l.is_empty()).collect()).unwrap_or_default()
}

impl Server {
    pub fn is_admin(&self, name: &str) -> bool {
        self.admins.contains(&name.to_lowercase())
    }

    fn save_admins(&self) {
        let mut v: Vec<&String> = self.admins.iter().collect();
        v.sort();
        let text: String = v.iter().map(|n| format!("{}\n", n)).collect();
        let _ = save::write_atomic(&admins_path(&self.cfg.data_dir), &text);
    }

    /// Admin names as players see them (proper capitals when we know them).
    fn admin_names(&self) -> Vec<String> {
        let mut v: Vec<String> = self.admins.iter().map(|n| self.social.display_name(n).unwrap_or_else(|| n.clone())).collect();
        v.sort();
        v
    }

    pub(crate) fn send_admins(&self, i: Option<usize>) {
        let m = ServerMsg::Admins { names: self.admin_names() };
        match i {
            Some(i) => self.clients[i].send(&m),
            None => self.broadcast_all(&m),
        }
    }

    /// Give or take the admin role. `by` is who did it ("the host" from the
    /// server window). Returns what happened, for the log.
    pub fn set_admin(&mut self, name: &str, on: bool, by: &str) -> String {
        let name = name.trim();
        if name.is_empty() {
            return "Whose admin role? (admin <name>)".into();
        }
        let known = self.names.contains(&name.to_lowercase());
        if !known {
            return format!("There's no character called {} on this server.", name);
        }
        let shown = self.social.display_name(&name.to_lowercase()).unwrap_or_else(|| name.to_string());
        let changed = if on { self.admins.insert(name.to_lowercase()) } else { self.admins.remove(&name.to_lowercase()) };
        if !changed {
            return format!("{} {} an admin.", shown, if on { "is already" } else { "isn't" });
        }
        self.save_admins();
        self.send_admins(None);
        if let Some(j) = self.idx_by_name(&shown) {
            self.sys(j, if on { format!("{} made you an admin. Press F9 for the Admin Panel, or type /help.", by) } else { format!("{} took away your admin role.", by) });
            if !on {
                // god mode goes with the role
                self.clients[j].send(&ServerMsg::AdminDo { action: "god".into(), map: String::new(), x: 0.0, y: 0.0, n: 0.0 });
            }
        }
        let text = format!("{} {} {}", by, if on { "made an admin:" } else { "removed admin:" }, shown);
        self.say(text.clone());
        text
    }

    /// Admins, from the server window.
    pub fn list_admins(&mut self) {
        let names = self.admin_names();
        self.say(if names.is_empty() { "No admins yet. Type: admin <name>".into() } else { format!("Admins: {}", names.join(", ")) });
    }

    pub fn kick(&mut self, name: &str, reason: &str, by: &str) -> String {
        let Some(j) = self.idx_by_name(name) else { return format!("{} isn't online.", name) };
        let shown = self.clients[j].name();
        let why = if reason.is_empty() { String::new() } else { format!(": {}", reason) };
        self.clients[j].send(&ServerMsg::Error { text: format!("You were kicked by {}{}", by, why) });
        self.clients[j].gone = true;
        let t = format!("{} kicked {}{}", by, shown, why);
        self.say(t.clone());
        t
    }

    pub(crate) fn muted_for(&self, name: &str) -> u32 {
        self.muted.get(&name.to_lowercase()).map(|until| until.saturating_duration_since(Instant::now()).as_secs() as u32).unwrap_or(0)
    }

    /// Players can't chat while muted; tells them how long is left.
    pub(crate) fn check_mute(&self, i: usize) -> bool {
        let left = self.muted_for(&self.clients[i].name());
        if left > 0 {
            self.sys(i, format!("You're muted for another {} minute(s).", left.div_ceil(60)));
            return true;
        }
        false
    }

    fn admin_rows(&self) -> Vec<AdminRow> {
        let mut v: Vec<AdminRow> = self
            .clients
            .iter()
            .filter(|c| c.in_world())
            .map(|c| {
                let n = c.name();
                AdminRow { level: c.info.level.max(1), class: c.info.class.clone(), map: c.map.clone(), admin: self.is_admin(&n), muted: self.muted_for(&n), name: n }
            })
            .collect();
        v.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        v
    }

    fn alog(&self, i: usize, text: impl Into<String>) {
        self.clients[i].send(&ServerMsg::AdminLog { text: text.into() });
    }

    fn ado(&self, j: usize, action: &str, map: &str, x: f32, y: f32, n: f32) {
        self.clients[j].send(&ServerMsg::AdminDo { action: action.into(), map: map.into(), x, y, n });
    }

    /// An admin command from a player.
    pub(crate) fn admin_cmd(&mut self, i: usize, line: &str) {
        if !self.clients[i].in_world() {
            return;
        }
        let me = self.clients[i].name();
        if !self.is_admin(&me) {
            return self.sys(i, "Only admins can use that command.".into());
        }
        let (cmd, args) = admin::split(line);
        let known: Vec<String> = self.clients.iter().filter(|c| c.in_world()).map(|c| c.name()).collect();
        // the player a command is about ("me" = you)
        let target = |s: &Self, who: &str| -> Option<usize> {
            if who.eq_ignore_ascii_case("me") || who.is_empty() {
                Some(i)
            } else {
                s.idx_by_name(who)
            }
        };
        let (who, rest) = admin::take_name(&args, &known);
        let num = |t: &str| t.split_whitespace().next().and_then(|v| v.replace(',', "").parse::<f64>().ok());
        let result: String = match cmd.as_str() {
            "help" | "commands" => {
                for c in COMMANDS {
                    self.alog(i, format!("{} - {}", c.usage, c.help));
                }
                return;
            }
            "players" | "who" => {
                let rows = self.admin_rows();
                self.clients[i].send(&ServerMsg::AdminPlayers(rows));
                return;
            }
            "announce" => {
                let text: String = args.chars().filter(|c| !c.is_control()).take(net::CHAT_MAX).collect();
                if text.is_empty() {
                    return self.alog(i, "Usage: /announce <text>");
                }
                self.broadcast_all(&ServerMsg::System { text: format!("[Announcement] {}", text) });
                format!("{} announced: {}", me, text)
            }
            "goto" | "tp" => {
                let Some(j) = target(self, &who).filter(|&j| j != i) else { return self.alog(i, "Usage: /goto <player>") };
                let (map, p) = (self.clients[j].map.clone(), self.clients[j].pos);
                if map.is_empty() {
                    return self.alog(i, "They're still loading in.");
                }
                self.ado(i, "tp", &map, p.x, p.y, 0.0);
                format!("{} went to {}", me, self.clients[j].name())
            }
            "summon" | "bring" => {
                let Some(j) = target(self, &who).filter(|&j| j != i) else { return self.alog(i, "Usage: /summon <player>") };
                let (map, p) = (self.clients[i].map.clone(), self.clients[i].pos);
                self.ado(j, "tp", &map, p.x + 40.0, p.y, 0.0);
                self.sys(j, format!("{} summoned you.", me));
                format!("{} summoned {}", me, self.clients[j].name())
            }
            "heal" => {
                let Some(j) = target(self, &who) else { return self.alog(i, format!("{} isn't online.", who)) };
                self.ado(j, "heal", "", 0.0, 0.0, 0.0);
                if j != i {
                    self.sys(j, format!("{} healed you.", me));
                }
                format!("{} healed {}", me, self.clients[j].name())
            }
            "give" => {
                let Some(j) = target(self, &who) else { return self.alog(i, format!("{} isn't online.", who)) };
                let mut parts = rest.split_whitespace();
                let Some(id) = parts.next() else { return self.alog(i, "Usage: /give <player> <item id> [count]") };
                let count = parts.next().and_then(|n| n.parse::<u32>().ok()).unwrap_or(1).clamp(1, 999);
                let Some(def) = items::item(&id.to_lowercase()).or_else(|| items::ITEMS.iter().find(|d| d.name.eq_ignore_ascii_case(&rest.replace(&format!(" {}", count), "")))) else {
                    return self.alog(i, format!("No item called '{}'. Use an item id like elixir or galeheart_blade.", id));
                };
                self.clients[j].send(&ServerMsg::Grant { gold: 0, cost: 0, items: vec![(def.id.to_string(), count)], reason: format!("From {}", me) });
                format!("{} gave {} {}x {}", me, self.clients[j].name(), count, def.name)
            }
            "gold" => {
                let Some(j) = target(self, &who) else { return self.alog(i, format!("{} isn't online.", who)) };
                let Some(n) = num(&rest).filter(|n| *n >= 1.0) else { return self.alog(i, "Usage: /gold <player> <amount>") };
                let n = n.min(100_000_000.0) as u32;
                self.clients[j].send(&ServerMsg::Grant { gold: n, cost: 0, items: vec![], reason: format!("From {}", me) });
                format!("{} gave {} {} gold", me, self.clients[j].name(), n)
            }
            "level" => {
                let Some(j) = target(self, &who) else { return self.alog(i, format!("{} isn't online.", who)) };
                let Some(n) = num(&rest) else { return self.alog(i, "Usage: /level <player> <1-50>") };
                let n = (n as u32).clamp(1, admin::MAX_LEVEL);
                self.ado(j, "level", "", 0.0, 0.0, n as f32);
                format!("{} set {}'s level to {}", me, self.clients[j].name(), n)
            }
            "god" => {
                let on = !self.god.contains(&me.to_lowercase());
                if on {
                    self.god.insert(me.to_lowercase());
                } else {
                    self.god.remove(&me.to_lowercase());
                }
                self.ado(i, "god", "", 0.0, 0.0, on as u8 as f32);
                format!("{} turned god mode {}", me, if on { "on" } else { "off" })
            }
            "spawn" => {
                let mut words: Vec<&str> = args.split_whitespace().collect();
                let count = words.last().and_then(|w| w.parse::<u32>().ok());
                if count.is_some() {
                    words.pop();
                }
                let Some(kind) = admin::monster(&words.join(" ")) else { return self.alog(i, "Usage: /spawn <monster> [count], e.g. /spawn mossling 3") };
                let n = count.unwrap_or(1).clamp(1, SPAWN_MAX);
                let map = self.clients[i].map.clone();
                let p = self.clients[i].pos;
                let facing = if p.facing < 0 { -1.0 } else { 1.0 };
                for k in 0..n {
                    let id = self.new_id();
                    let x = p.x + facing * (120.0 + k as f32 * 50.0);
                    let e = Enemy::new(id, kind, x, p.y, None, &mut self.rng);
                    if let Some(z) = self.zones.get_mut(&map) {
                        z.enemies.push(e);
                    }
                }
                format!("{} spawned {}x {} on {}", me, n, kind.def().name, map)
            }
            "clear" => {
                let map = self.clients[i].map.clone();
                let n = self.zones.get(&map).map(|z| z.enemies.len()).unwrap_or(0);
                if let Some(z) = self.zones.get_mut(&map) {
                    z.enemies.clear();
                    z.contrib.clear();
                    let list = enemy_list(z);
                    self.broadcast_map(&map, &ServerMsg::Enemies(list), None);
                }
                format!("{} cleared {} monster(s) from {}", me, n, map)
            }
            "mute" => {
                let Some(j) = target(self, &who).filter(|&j| j != i) else { return self.alog(i, "Usage: /mute <player> [minutes]") };
                let mins = num(&rest).unwrap_or(10.0).clamp(1.0, 24.0 * 60.0);
                let n = self.clients[j].name();
                self.muted.insert(n.to_lowercase(), Instant::now() + Duration::from_secs_f64(mins * 60.0));
                self.sys(j, format!("{} muted you for {} minute(s).", me, mins as u32));
                format!("{} muted {} for {} minute(s)", me, n, mins as u32)
            }
            "unmute" => {
                let Some(j) = target(self, &who) else { return self.alog(i, format!("{} isn't online.", who)) };
                let n = self.clients[j].name();
                self.muted.remove(&n.to_lowercase());
                self.sys(j, "You can chat again.".into());
                format!("{} unmuted {}", me, n)
            }
            "kick" => {
                let Some(j) = target(self, &who).filter(|&j| j != i) else { return self.alog(i, "Usage: /kick <player> [reason]") };
                let n = self.clients[j].name();
                self.kick(&n, &rest, &me)
            }
            "admin" | "unadmin" => {
                let on = cmd == "admin";
                let name = if who.eq_ignore_ascii_case("me") { me.clone() } else { who.clone() };
                if !on && social::eq(&name, &me) {
                    return self.alog(i, "You can't remove your own admin role (ask the host or another admin).");
                }
                self.set_admin(&name, on, &me)
            }
            _ => return self.alog(i, format!("Unknown admin command '/{}'. Type /help.", cmd)),
        };
        self.say(format!("[admin] {}", result));
        self.alog(i, result);
        // keep the panel's list fresh after anything that changes it
        if matches!(cmd.as_str(), "mute" | "unmute" | "kick" | "admin" | "unadmin" | "level") {
            let rows = self.admin_rows();
            self.clients[i].send(&ServerMsg::AdminPlayers(rows));
        }
    }

    // ------------------------------------------------------------------ duels

    pub(crate) fn duel_of(&self, id: u32) -> Option<usize> {
        self.duels.iter().position(|d| d.ids.contains(&id))
    }

    fn send_duel(&self, d: usize, winner: &str) {
        let du = &self.duels[d];
        for (k, id) in du.ids.iter().enumerate() {
            if let Some(c) = self.clients.iter().find(|c| c.id == *id) {
                let foe = du.ids[1 - k];
                c.send(&ServerMsg::Duel { foe, name: if du.phase == DUEL_OVER { winner.to_string() } else { self.name_of(foe) }, phase: du.phase, secs: du.left });
            }
        }
    }

    fn name_of(&self, id: u32) -> String {
        self.clients.iter().find(|c| c.id == id).map(|c| c.name()).unwrap_or_default()
    }

    /// End a duel. `loser` (a player id) loses; None is a draw.
    pub(crate) fn end_duel(&mut self, d: usize, loser: Option<u32>, why: &str) {
        let ids = self.duels[d].ids;
        let map = self.duels[d].map.clone();
        let (winner, text) = match loser {
            Some(l) => {
                let w = if ids[0] == l { ids[1] } else { ids[0] };
                let (wn, ln) = (self.name_of(w), self.name_of(l));
                (wn.clone(), format!("{} defeated {} in a duel{}!", wn, ln, why))
            }
            None => (String::new(), format!("The duel between {} and {} ended in a draw{}.", self.name_of(ids[0]), self.name_of(ids[1]), why)),
        };
        self.duels[d].phase = DUEL_OVER;
        self.duels[d].left = 0.0;
        self.send_duel(d, &winner);
        self.duels.remove(d);
        self.broadcast_map(&map, &ServerMsg::ChatCh { channel: CH_SYSTEM, from: String::new(), to: String::new(), text: text.clone() }, None);
        self.say(text);
    }

    /// A player left the world or their map: they forfeit any duel.
    pub(crate) fn forfeit_duel(&mut self, id: u32, why: &str) {
        if let Some(d) = self.duel_of(id) {
            self.end_duel(d, Some(id), why);
        }
    }

    pub(crate) fn update_duels(&mut self, dt: f32) {
        let mut d = 0;
        while d < self.duels.len() {
            self.duels[d].left -= dt;
            if self.duels[d].left > 0.0 {
                d += 1;
                continue;
            }
            if self.duels[d].phase == DUEL_COUNTDOWN {
                self.duels[d].phase = DUEL_FIGHT;
                self.duels[d].left = DUEL_SECS;
                self.send_duel(d, "");
                d += 1;
            } else {
                self.end_duel(d, None, " (time ran out)");
            }
        }
    }

    pub(crate) fn duel_social(&mut self, i: usize, cmd: SocialCmd) {
        let me = self.clients[i].name();
        let my_id = self.clients[i].id;
        match cmd {
            SocialCmd::DuelRequest { name } => {
                if self.duel_of(my_id).is_some() {
                    return self.sys(i, "You're already in a duel.".into());
                }
                let Some(j) = self.idx_by_name(&name) else { return self.sys(i, format!("{} isn't online.", name.trim())) };
                let (a, b) = (&self.clients[i], &self.clients[j]);
                if a.map != b.map || (a.center() - b.center()).len() > DUEL_ASK_RANGE {
                    return self.sys(i, "You can only challenge someone near you.".into());
                }
                if a.pos.dead || b.pos.dead {
                    return self.sys(i, "Nobody can duel as a tombstone.".into());
                }
                if self.duel_of(b.id).is_some() {
                    return self.sys(i, format!("{} is already in a duel.", b.name()));
                }
                if self.invite(i, INV_DUEL, &name, String::new()).is_some() {
                    self.sys(i, format!("You challenged {} to a duel.", name.trim()));
                }
            }
            SocialCmd::DuelAnswer { from, accept } => {
                let Some(inv) = self.take_invite(INV_DUEL, &from, &me) else { return self.sys(i, "That challenge has expired.".into()) };
                let Some(j) = self.idx_by_name(&inv.from) else { return };
                if !accept {
                    return self.sys(j, format!("{} declined your duel.", me));
                }
                let other = self.clients[j].id;
                if self.duel_of(my_id).is_some() || self.duel_of(other).is_some() {
                    return self.sys(i, "One of you is already in a duel.".into());
                }
                if self.clients[i].map != self.clients[j].map {
                    return self.sys(i, format!("{} isn't on your map any more.", inv.from));
                }
                let map = self.clients[i].map.clone();
                self.duels.push(Duel { ids: [other, my_id], map: map.clone(), phase: DUEL_COUNTDOWN, left: DUEL_COUNTDOWN_SECS });
                self.send_duel(self.duels.len() - 1, "");
                let text = format!("{} and {} are duelling!", inv.from, me);
                self.broadcast_map(&map, &ServerMsg::ChatCh { channel: CH_SYSTEM, from: String::new(), to: String::new(), text }, None);
            }
            SocialCmd::DuelYield => {
                if let Some(d) = self.duel_of(my_id) {
                    self.end_duel(d, Some(my_id), "");
                }
            }
            _ => {}
        }
    }

    /// A duel hit, reported by the attacker's game: pass it on if it's fair.
    pub(crate) fn duel_hit(&mut self, i: usize, target: u32, damage: f32, kbx: f32, kby: f32) {
        let my_id = self.clients[i].id;
        let Some(d) = self.duel_of(my_id) else { return };
        let du = &self.duels[d];
        if du.phase != DUEL_FIGHT || !du.ids.contains(&target) || target == my_id {
            return;
        }
        let Some(j) = self.clients.iter().position(|c| c.id == target) else { return };
        let (a, b) = (&self.clients[i], &self.clients[j]);
        if a.map != du.map || b.map != du.map || (a.center() - b.center()).len() > DUEL_HIT_RANGE || !damage.is_finite() || damage <= 0.0 {
            return;
        }
        self.clients[j].send(&ServerMsg::DuelHit { from: my_id, damage: damage.min(DUEL_HIT_CAP), kbx: kbx.clamp(-1500.0, 1500.0), kby: kby.clamp(-1500.0, 1500.0) });
    }
}
