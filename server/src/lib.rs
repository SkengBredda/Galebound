//! The Galebound dedicated server.
//!
//! It owns accounts and saves, runs every occupied map's monsters (AI, HP,
//! deaths, respawns), keeps loot on the ground and decides who gets it, and
//! relays players' positions, chat and revives. Each player's own movement and
//! combat run in their game client, which reports hits to the server (the way
//! classic side-scrolling MMOs work); the server range- and cap-checks them.
//!
//! Standard library only: one listener, two small threads per connection for
//! socket I/O, and a single 60 Hz game loop that owns all the state.

use galebound_core::auth::{AccountStore, AuthError};
use galebound_core::enemies::{self, EState, Enemy, EnemyKind};
use galebound_core::items;
use galebound_core::math::{Rng, V2};
use galebound_core::net::{self, ClientMsg, DropNet, EnemyNet, PlayerInfo, PlayerPos, ServerMsg, ShotNet};
use galebound_core::physics::{self, Body, Level};
use galebound_core::save::{self, Account, CharacterSave, MAX_CHARACTERS};
use galebound_core::sim::DROP_LIFE;
use galebound_core::world::{self, Map};
use std::collections::{HashMap, HashSet};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Monster simulation rate.
pub const TICK_HZ: u32 = 60;
/// Monster snapshots per second.
pub const SNAPSHOT_HZ: u32 = 15;
/// Players who haven't sent anything for this long are disconnected.
const IDLE_TIMEOUT: Duration = Duration::from_secs(45);
/// Empty maps keep their monsters for this long, then reset.
const ZONE_LINGER: f32 = 60.0;
/// How far from a monster a hit may be reported from.
const HIT_RANGE: f32 = 900.0;
/// No single hit may do more than this.
const HIT_CAP: f32 = 100_000.0;
/// Reach for picking loot up (players / pets).
const PICKUP_RANGE: f32 = 200.0;
const PET_PICKUP_RANGE: f32 = 900.0;
/// Everyone who helped beat a monster gets at least this share of its EXP;
/// the rest is split by damage dealt.
pub const MIN_EXP_SHARE: f32 = 0.2;
const MAX_CONNECTIONS: usize = 64;

#[derive(Clone, Debug)]
pub struct Config {
    pub port: u16,
    pub max_players: usize,
    pub data_dir: PathBuf,
    pub motd: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            port: net::DEFAULT_PORT,
            max_players: net::MAX_PLAYERS,
            data_dir: save::data_dir().join("server"),
            motd: "Welcome to Galebound!".into(),
        }
    }
}

/// Things the console (or a test) can ask the running server to do.
pub enum Command {
    Say(String),
    List,
    Stop,
}

struct Client {
    id: u32,
    tx: Sender<String>,
    rx: Receiver<Option<String>>,
    auth: AccountStore,
    account: Option<Account>,
    /// Character slot in play, if any.
    slot: Option<usize>,
    info: PlayerInfo,
    pos: PlayerPos,
    map: String,
    last_heard: Instant,
    gone: bool,
}

impl Client {
    fn send(&self, m: &ServerMsg) {
        let _ = self.tx.send(m.encode());
    }
    fn in_world(&self) -> bool {
        self.slot.is_some()
    }
    fn name(&self) -> String {
        match (&self.account, self.slot) {
            (Some(a), Some(s)) => a.characters.get(s).map(|c| c.name.clone()).unwrap_or_default(),
            _ => String::new(),
        }
    }
    fn center(&self) -> V2 {
        V2::new(self.pos.x, self.pos.y - 32.0)
    }
}

struct SDrop {
    id: u32,
    item: String,
    count: u32,
    body: Body,
    age: f32,
}

/// One map's shared monsters and loot.
struct Zone {
    map: Map,
    enemies: Vec<Enemy>,
    timers: Vec<f32>,
    drops: Vec<SDrop>,
    /// Damage dealt to each monster, by player id.
    contrib: HashMap<u32, Vec<(u32, f32)>>,
    empty_for: f32,
}

pub struct Server {
    cfg: Config,
    /// IPv4, plus IPv6 where the system allows it.
    listeners: Vec<TcpListener>,
    clients: Vec<Client>,
    zones: HashMap<String, Zone>,
    next_id: u32,
    rng: Rng,
    /// Every character name in use (lower-case), across all accounts.
    names: HashSet<String>,
    store: AccountStore,
    tick: u64,
    pub log: Vec<String>,
}

impl Server {
    /// Bind the port and load the account index. Port 0 picks a free port.
    pub fn bind(cfg: Config) -> std::io::Result<Server> {
        let listener = TcpListener::bind(("0.0.0.0", cfg.port))?;
        listener.set_nonblocking(true)?;
        let port = listener.local_addr()?.port();
        let mut listeners = vec![listener];
        // Also listen on IPv6 (Windows tries "localhost" as ::1 first). On
        // systems where IPv6 shares the IPv4 socket this fails, which is fine.
        if let Ok(l6) = TcpListener::bind(("::", port)) {
            if l6.set_nonblocking(true).is_ok() {
                listeners.push(l6);
            }
        }
        std::fs::create_dir_all(&cfg.data_dir)?;
        let store = AccountStore::new(cfg.data_dir.clone());
        let names = store.all_accounts().iter().flat_map(|a| a.characters.iter().map(|c| c.name.to_lowercase())).collect();
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(7);
        Ok(Server { cfg, listeners, clients: Vec::new(), zones: HashMap::new(), next_id: 1, rng: Rng::new(seed), names, store, tick: 0, log: Vec::new() })
    }

    pub fn port(&self) -> u16 {
        self.listeners[0].local_addr().map(|a| a.port()).unwrap_or(0)
    }

    pub fn players_online(&self) -> usize {
        self.clients.iter().filter(|c| c.in_world()).count()
    }

    fn say(&mut self, s: String) {
        println!("[server] {}", s);
        self.log.push(s);
        if self.log.len() > 500 {
            self.log.remove(0);
        }
    }

    fn new_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Run until `stop` is set or a Stop command arrives.
    pub fn run(&mut self, stop: Arc<AtomicBool>, commands: Option<Receiver<Command>>) {
        let step = Duration::from_secs_f64(1.0 / TICK_HZ as f64);
        let mut next = Instant::now();
        while !stop.load(Ordering::Relaxed) {
            if let Some(rx) = &commands {
                while let Ok(c) = rx.try_recv() {
                    match c {
                        Command::Stop => stop.store(true, Ordering::Relaxed),
                        Command::Say(t) => self.broadcast_all(&ServerMsg::System { text: format!("[Server] {}", t) }),
                        Command::List => {
                            let names: Vec<String> = self.clients.iter().filter(|c| c.in_world()).map(|c| format!("{} ({})", c.name(), c.map)).collect();
                            self.say(format!("{} / {} players online: {}", names.len(), self.cfg.max_players, names.join(", ")));
                        }
                    }
                }
            }
            self.step();
            next += step;
            let now = Instant::now();
            if next > now {
                std::thread::sleep(next - now);
            } else {
                next = now; // fell behind; don't try to catch up in a burst
            }
        }
        // Say goodbye so clients can tell the server went away on purpose.
        self.broadcast_all(&ServerMsg::Error { text: "The server is shutting down.".into() });
        std::thread::sleep(Duration::from_millis(100));
    }

    /// One game-loop step: new connections, messages, monsters, snapshots.
    pub fn step(&mut self) {
        self.accept();
        for i in 0..self.clients.len() {
            loop {
                match self.clients[i].rx.try_recv() {
                    Ok(Some(line)) => {
                        self.clients[i].last_heard = Instant::now();
                        match ClientMsg::decode(&line) {
                            Some(m) => self.handle(i, m),
                            None => self.clients[i].send(&ServerMsg::Error { text: "Unknown message".into() }),
                        }
                    }
                    Ok(None) | Err(TryRecvError::Disconnected) => {
                        self.clients[i].gone = true;
                        break;
                    }
                    Err(TryRecvError::Empty) => break,
                }
            }
            if self.clients[i].last_heard.elapsed() > IDLE_TIMEOUT {
                self.clients[i].gone = true;
            }
        }
        // Drop disconnected players.
        let mut i = 0;
        while i < self.clients.len() {
            if self.clients[i].gone {
                self.leave_world(i);
                let c = self.clients.remove(i);
                drop(c);
            } else {
                i += 1;
            }
        }
        // monsters and loot run at the same calmer pace as the clients
        let dt = 1.0 / TICK_HZ as f32 * physics::GAME_SPEED;
        self.update_zones(dt);
        self.tick += 1;
        if self.tick % (TICK_HZ / SNAPSHOT_HZ) as u64 == 0 {
            self.snapshots();
        }
    }

    fn accept(&mut self) {
        let mut incoming = Vec::new();
        for l in &self.listeners {
            while let Ok(c) = l.accept() {
                incoming.push(c);
            }
        }
        for (stream, addr) in incoming {
            if self.clients.len() >= MAX_CONNECTIONS {
                continue;
            }
            let _ = stream.set_nonblocking(false);
            let Ok((tx, rx, _s)) = net::spawn_io(stream) else { continue };
            let id = self.new_id();
            self.say(format!("connection #{} from {}", id, addr));
            self.clients.push(Client {
                id,
                tx,
                rx,
                auth: AccountStore::new(self.cfg.data_dir.clone()),
                account: None,
                slot: None,
                info: PlayerInfo::default(),
                pos: PlayerPos::default(),
                map: String::new(),
                last_heard: Instant::now(),
                gone: false,
            });
        }
    }

    // ------------------------------------------------------------------ sending

    fn broadcast_all(&self, m: &ServerMsg) {
        for c in self.clients.iter().filter(|c| c.in_world()) {
            c.send(m);
        }
    }

    /// Everyone in the world on `map` except `except`.
    fn broadcast_map(&self, map: &str, m: &ServerMsg, except: Option<u32>) {
        for c in self.clients.iter().filter(|c| c.in_world() && c.map == map && Some(c.id) != except) {
            c.send(m);
        }
    }

    fn reply_account(&self, i: usize, token: String) {
        if let Some(a) = &self.clients[i].account {
            let text = save::encode_account(&save::public_account(a));
            self.clients[i].send(&ServerMsg::Account { account: text, token });
        }
    }

    fn persist(&mut self, i: usize) {
        if let Some(a) = &self.clients[i].account {
            if let Err(e) = self.store.save(a) {
                let msg = format!("couldn't save {}: {}", a.email, e);
                self.say(msg);
            }
        }
    }

    // ------------------------------------------------------------------ messages

    fn handle(&mut self, i: usize, m: ClientMsg) {
        let err = |s: &Self, t: &str| s.clients[i].send(&ServerMsg::Error { text: t.to_string() });
        match m {
            ClientMsg::Hello { .. } => self.clients[i].send(&ServerMsg::Welcome { protocol: net::PROTOCOL, motd: self.cfg.motd.clone() }),
            ClientMsg::Ping { t } => self.clients[i].send(&ServerMsg::Pong { t }),
            ClientMsg::Register { email, password, confirm } => {
                let r = self.clients[i].auth.register(&email, &password, &confirm);
                self.logged_in(i, r, false);
            }
            ClientMsg::Login { email, password, remember } => {
                let r = self.clients[i].auth.login(&email, &password);
                self.logged_in(i, r, remember);
            }
            ClientMsg::LoginToken { email, token } => {
                let r = self.clients[i].auth.login_token(&email, &token);
                self.logged_in(i, r, false);
            }
            ClientMsg::Forget => {
                if let Some(a) = self.clients[i].account.as_mut() {
                    a.remember_hash = None;
                    self.persist(i);
                }
            }
            ClientMsg::CreateChar { save: text } => {
                if self.clients[i].account.is_none() || self.clients[i].in_world() {
                    return err(self, "Log in first");
                }
                let Some(c) = save::decode_character(&text) else { return err(self, "That character couldn't be read") };
                let name = match save::validate_name(&c.name) {
                    Ok(n) => n,
                    Err(e) => return err(self, &e),
                };
                if self.names.contains(&name.to_lowercase()) {
                    return err(self, "Someone on this server already has that name");
                }
                if self.clients[i].account.as_ref().map(|a| a.characters.len()).unwrap_or(0) >= MAX_CHARACTERS {
                    return err(self, "You already have the most characters allowed");
                }
                // Every new character starts the same way, whatever the client sent.
                let fresh = CharacterSave::new(&name, c.look);
                self.names.insert(name.to_lowercase());
                self.clients[i].account.as_mut().unwrap().characters.push(fresh);
                self.persist(i);
                self.reply_account(i, String::new());
                self.say(format!("new character: {}", name));
            }
            ClientMsg::DeleteChar { slot } => {
                if self.clients[i].in_world() {
                    return err(self, "Leave the world first");
                }
                let Some(a) = self.clients[i].account.as_mut() else { return err(self, "Log in first") };
                if (slot as usize) < a.characters.len() {
                    let c = a.characters.remove(slot as usize);
                    self.names.remove(&c.name.to_lowercase());
                    self.persist(i);
                }
                self.reply_account(i, String::new());
            }
            ClientMsg::Settings { text } => {
                if let Some(a) = self.clients[i].account.as_mut() {
                    a.settings = save::decode_settings(&text);
                    self.persist(i);
                }
            }
            ClientMsg::Enter { slot } => self.enter(i, slot as usize),
            ClientMsg::Leave => {
                self.leave_world(i);
                self.clients[i].send(&ServerMsg::Left);
            }
            ClientMsg::Save { save: text } => {
                let Some(slot) = self.clients[i].slot else { return };
                let Some(mut c) = save::decode_character(&text) else { return err(self, "Save couldn't be read") };
                if let Some(a) = self.clients[i].account.as_mut() {
                    if let Some(old) = a.characters.get(slot) {
                        c.name = old.name.clone(); // names can't change by saving
                    }
                    if slot < a.characters.len() {
                        a.characters[slot] = c;
                    }
                }
                self.persist(i);
            }
            ClientMsg::Info(mut info) => {
                if !self.clients[i].in_world() {
                    return;
                }
                info.id = self.clients[i].id;
                info.name = self.clients[i].name();
                self.clients[i].info = info.clone();
                if !self.clients[i].map.is_empty() {
                    let map = self.clients[i].map.clone();
                    self.broadcast_map(&map, &ServerMsg::PlayerInfo(info), Some(self.clients[i].id));
                }
            }
            ClientMsg::Pos { map, mut pos } => {
                if !self.clients[i].in_world() {
                    return;
                }
                pos.id = self.clients[i].id;
                if map != self.clients[i].map {
                    if world::load_map(&map).is_none() {
                        return;
                    }
                    self.change_map(i, map, pos);
                } else {
                    self.clients[i].pos = pos;
                }
                let m = self.clients[i].map.clone();
                self.broadcast_map(&m, &ServerMsg::PlayerPos(pos), Some(pos.id));
            }
            ClientMsg::Hit { enemy, damage, kbx, kby, hitstun } => self.hit(i, enemy, damage, V2::new(kbx, kby), hitstun),
            ClientMsg::PickUp { drop, pet } => self.pick_up(i, drop, pet),
            ClientMsg::DropItem(d) => self.player_drop(i, d),
            ClientMsg::Boss { kind, map } => self.boss(i, kind, map),
            ClientMsg::Act(mut a) => {
                let c = &self.clients[i];
                if !c.in_world() || c.pos.dead || c.map.is_empty() {
                    return;
                }
                a.id = c.id;
                let map = c.map.clone();
                self.broadcast_map(&map, &ServerMsg::Act(a), Some(c.id));
            }
            ClientMsg::Revive { target } => {
                let me = &self.clients[i];
                if !me.in_world() || me.pos.dead {
                    return;
                }
                let (by, map, at) = (me.name(), me.map.clone(), V2::new(me.pos.x, me.pos.y));
                let found = self.clients.iter().position(|c| c.id == target && c.in_world() && c.map == map && c.pos.dead);
                if let Some(t) = found {
                    let tp = self.clients[t].pos;
                    if (tp.x - at.x).abs() < 160.0 && (tp.y - at.y).abs() < 120.0 {
                        self.clients[t].send(&ServerMsg::Revived { by: by.clone() });
                        let tn = self.clients[t].name();
                        self.say(format!("{} revived {}", by, tn));
                    }
                }
            }
            ClientMsg::Chat { text } => {
                if !self.clients[i].in_world() {
                    return;
                }
                let text: String = text.trim().chars().filter(|c| !c.is_control()).take(net::CHAT_MAX).collect();
                if text.is_empty() {
                    return;
                }
                let from = self.clients[i].name();
                self.say(format!("<{}> {}", from, text));
                self.broadcast_all(&ServerMsg::Chat { from, text });
            }
        }
    }

    fn logged_in(&mut self, i: usize, r: Result<Account, AuthError>, remember: bool) {
        match r {
            Ok(mut acc) => {
                // One session per account: the older one is signed out.
                let email = acc.email.clone();
                for j in 0..self.clients.len() {
                    if j != i && self.clients[j].account.as_ref().map(|a| a.email == email).unwrap_or(false) {
                        self.clients[j].send(&ServerMsg::Error { text: "You logged in from another computer.".into() });
                        self.clients[j].gone = true;
                        self.leave_world(j);
                        self.clients[j].account = None;
                    }
                }
                let token = if remember { self.clients[i].auth.issue_token(&mut acc).unwrap_or_default() } else { String::new() };
                self.clients[i].account = Some(acc);
                self.reply_account(i, token);
            }
            Err(e) => self.clients[i].send(&ServerMsg::Error { text: e.to_string() }),
        }
    }

    fn enter(&mut self, i: usize, slot: usize) {
        let ok = self.clients[i].account.as_ref().map(|a| slot < a.characters.len()).unwrap_or(false);
        if !ok {
            return self.clients[i].send(&ServerMsg::Error { text: "Pick a character first".into() });
        }
        if self.clients[i].in_world() {
            self.leave_world(i);
        }
        if self.players_online() >= self.cfg.max_players {
            return self.clients[i].send(&ServerMsg::Error { text: format!("The server is full ({} / {} players). Try again soon.", self.players_online(), self.cfg.max_players) });
        }
        self.clients[i].slot = Some(slot);
        self.clients[i].map = String::new(); // set by their first position update
        let id = self.clients[i].id;
        self.clients[i].send(&ServerMsg::Entered { id });
        let name = self.clients[i].name();
        self.say(format!("{} entered the world ({} online)", name, self.players_online()));
        self.broadcast_all(&ServerMsg::System { text: format!("{} has joined.", name) });
    }

    fn leave_world(&mut self, i: usize) {
        if !self.clients[i].in_world() {
            return;
        }
        let name = self.clients[i].name();
        let (id, map) = (self.clients[i].id, self.clients[i].map.clone());
        self.broadcast_map(&map, &ServerMsg::PlayerGone { id }, Some(id));
        self.clients[i].slot = None;
        self.clients[i].map = String::new();
        self.say(format!("{} left the world", name));
        self.broadcast_all(&ServerMsg::System { text: format!("{} has left.", name) });
    }

    fn change_map(&mut self, i: usize, map: String, pos: PlayerPos) {
        let id = self.clients[i].id;
        let old = std::mem::replace(&mut self.clients[i].map, map.clone());
        self.clients[i].pos = pos;
        if !old.is_empty() {
            self.broadcast_map(&old, &ServerMsg::PlayerGone { id }, Some(id));
        }
        if !self.zones.contains_key(&map) {
            let z = self.new_zone(&map);
            self.zones.insert(map.clone(), z);
        }
        // Tell the newcomer who's here and what's lying around...
        let me = &self.clients[i];
        for c in self.clients.iter().filter(|c| c.in_world() && c.map == map && c.id != id) {
            me.send(&ServerMsg::PlayerInfo(c.info.clone()));
            me.send(&ServerMsg::PlayerPos(c.pos));
        }
        if let Some(z) = self.zones.get(&map) {
            for d in &z.drops {
                me.send(&ServerMsg::DropAdd(DropNet { id: d.id, item: d.item.clone(), count: d.count, x: d.body.pos.x, y: d.body.pos.y, vx: 0.0, vy: 0.0 }));
            }
            me.send(&ServerMsg::Enemies(enemy_list(z)));
        }
        // ...and everyone here about the newcomer.
        let info = self.clients[i].info.clone();
        self.broadcast_map(&map, &ServerMsg::PlayerInfo(info), Some(id));
    }

    // ------------------------------------------------------------------ zones

    fn new_zone(&mut self, key: &str) -> Zone {
        let map = world::load_map(key).expect("map exists");
        let mut z = Zone { enemies: Vec::new(), timers: vec![0.0; map.spawns.len()], drops: Vec::new(), contrib: HashMap::new(), empty_for: 0.0, map };
        for (k, x, y) in z.map.fixed_enemies.clone() {
            let id = self.new_id();
            z.enemies.push(Enemy::new(id, k, x, y, None, &mut self.rng));
        }
        for zi in 0..z.map.spawns.len() {
            let sp = z.map.spawns[zi].clone();
            for _ in 0..sp.max {
                let x = self.rng.range(sp.x0, sp.x1);
                let id = self.new_id();
                z.enemies.push(Enemy::new(id, sp.kind, x, sp.y, Some(zi), &mut self.rng));
            }
        }
        z
    }

    fn update_zones(&mut self, dt: f32) {
        let keys: Vec<String> = self.zones.keys().cloned().collect();
        for key in keys {
            // where the players on this map are
            let players: Vec<(V2, bool)> = self.clients.iter().filter(|c| c.in_world() && c.map == key).map(|c| (c.center(), !c.pos.dead)).collect();
            let mut z = self.zones.remove(&key).unwrap();
            if players.is_empty() {
                z.empty_for += dt;
                if z.empty_for > ZONE_LINGER {
                    continue; // nobody's been here for a while: reset the map
                }
                self.zones.insert(key, z);
                continue;
            }
            z.empty_for = 0.0;
            let mut solids = z.map.solids.clone();
            solids.extend(z.map.gates.iter().map(|g| g.solid));
            let lvl = Level { solids: &solids, platforms: &z.map.platforms, bounds: z.map.bounds() };
            let mut out = enemies::Out::default();
            for e in z.enemies.iter_mut() {
                // each monster goes for the nearest player who's still standing
                let c = e.body.center();
                let target = players.iter().filter(|p| p.1).map(|p| p.0).min_by(|a, b| (a.x - c.x).abs().total_cmp(&(b.x - c.x).abs()));
                let ctx = enemies::Ctx { level: &lvl, player: target.unwrap_or(V2::new(-99999.0, -99999.0)), player_alive: target.is_some(), dt };
                enemies::update(e, &ctx, &mut out);
            }
            for s in out.shots {
                self.broadcast_map(
                    &key,
                    &ServerMsg::Shot(ShotNet {
                        x: s.pos.x,
                        y: s.pos.y,
                        vx: s.vel.x,
                        vy: s.vel.y,
                        size: s.size,
                        gravity: s.gravity,
                        life: s.life,
                        pierce: s.pierce,
                        damage: s.damage,
                        kbx: s.kb.x,
                        kby: s.kb.y,
                        style: s.style as u32,
                        grounded: s.grounded,
                    }),
                    None,
                );
            }
            if out.shake > 0.0 {
                self.broadcast_map(&key, &ServerMsg::Shake { amount: out.shake }, None);
            }
            // the dead fade out, then their spawn zone starts counting down
            let kill_y = z.map.kill_y;
            let mut freed = Vec::new();
            z.enemies.retain(|e| {
                let gone = (e.state == EState::Dying && e.state_t > 0.6) || e.body.pos.y > kill_y;
                if gone {
                    if let Some(zi) = e.zone {
                        freed.push(zi);
                    }
                }
                !gone
            });
            for zi in freed {
                if zi < z.timers.len() && z.timers[zi] <= 0.0 {
                    z.timers[zi] = z.map.spawns[zi].respawn;
                }
            }
            for zi in 0..z.timers.len() {
                if z.timers[zi] > 0.0 {
                    z.timers[zi] -= dt;
                    if z.timers[zi] <= 0.0 {
                        let sp = z.map.spawns[zi].clone();
                        let alive = z.enemies.iter().filter(|e| e.zone == Some(zi)).count();
                        if alive < sp.max {
                            let mut x = self.rng.range(sp.x0, sp.x1);
                            if players.iter().any(|p| (p.0.x - x).abs() < 250.0) {
                                x = if x < (sp.x0 + sp.x1) * 0.5 { sp.x1 - 30.0 } else { sp.x0 + 30.0 };
                            }
                            let id = self.new_id();
                            z.enemies.push(Enemy::new(id, sp.kind, x, sp.y - 40.0, Some(zi), &mut self.rng));
                            if alive + 1 < sp.max {
                                z.timers[zi] = sp.respawn;
                            }
                        }
                    }
                }
            }
            // loot falls, settles and eventually vanishes
            let lvl = Level { solids: &solids, platforms: &z.map.platforms, bounds: z.map.bounds() };
            let mut expired = Vec::new();
            for d in z.drops.iter_mut() {
                d.age += dt;
                physics::apply_gravity(&mut d.body, dt, false, 1.0);
                physics::move_and_collide(&mut d.body, &lvl, dt);
                if d.body.on_ground {
                    d.body.vel.x = galebound_core::math::approach(d.body.vel.x, 0.0, 900.0 * dt);
                }
                if d.age > DROP_LIFE || d.body.pos.y > kill_y {
                    expired.push(d.id);
                }
            }
            z.drops.retain(|d| !expired.contains(&d.id));
            z.contrib.retain(|id, _| z.enemies.iter().any(|e| e.id == *id));
            self.zones.insert(key.clone(), z);
            for id in expired {
                self.broadcast_map(&key, &ServerMsg::DropGone { id, by: 0 }, None);
            }
        }
    }

    fn snapshots(&self) {
        for (key, z) in &self.zones {
            if z.empty_for > 0.0 {
                continue;
            }
            let m = ServerMsg::Enemies(enemy_list(z));
            self.broadcast_map(key, &m, None);
        }
    }

    // ------------------------------------------------------------------ combat and loot

    fn hit(&mut self, i: usize, enemy: u32, damage: f32, kb: V2, hitstun: f32) {
        let c = &self.clients[i];
        if !c.in_world() || c.pos.dead || !damage.is_finite() {
            return;
        }
        let (pid, at, key) = (c.id, c.center(), c.map.clone());
        let Some(z) = self.zones.get_mut(&key) else { return };
        let Some(e) = z.enemies.iter_mut().find(|e| e.id == enemy) else { return };
        if e.state == EState::Dying || e.kind == EnemyKind::TrainingDummy {
            return;
        }
        if (e.body.center() - at).len() > HIT_RANGE {
            return; // too far away to be real
        }
        let dmg = damage.clamp(0.0, HIT_CAP);
        enemies::apply_hit(e, dmg, V2::new(kb.x.clamp(-2000.0, 2000.0), kb.y.clamp(-2000.0, 2000.0)), hitstun.clamp(0.0, 1.5));
        let list = z.contrib.entry(enemy).or_default();
        match list.iter_mut().find(|x| x.0 == pid) {
            Some(x) => x.1 += dmg,
            None => list.push((pid, dmg)),
        }
        if e.hp <= 0.0 {
            e.state = EState::Dying;
            e.state_t = 0.0;
            self.kill(&key, enemy);
        }
    }

    /// Hand out EXP to everyone who helped and spill the loot.
    fn kill(&mut self, key: &str, enemy: u32) {
        let Some(z) = self.zones.get_mut(key) else { return };
        let Some(e) = z.enemies.iter().find(|e| e.id == enemy) else { return };
        let (kind, at) = (e.kind, e.body.center());
        let def = kind.def();
        let contrib = z.contrib.remove(&enemy).unwrap_or_default();
        let total: f32 = contrib.iter().map(|c| c.1).sum::<f32>().max(1.0);
        for (pid, dmg) in &contrib {
            let share = MIN_EXP_SHARE + (1.0 - MIN_EXP_SHARE) * dmg / total;
            let exp = ((def.exp as f32 * share).round() as u32).max(1);
            if let Some(c) = self.clients.iter().find(|c| c.id == *pid && c.map == key) {
                c.send(&ServerMsg::Kill { enemy, kind: kind.index(), exp });
            }
        }
        if def.gold > 0 {
            self.spawn_drop(key, net::GOLD.to_string(), def.gold, at, 70.0);
        }
        for &(id, chance) in kind.drops() {
            if self.rng.f() < chance {
                self.spawn_drop(key, id.to_string(), 1, at, 170.0);
            }
        }
    }

    fn spawn_drop(&mut self, key: &str, item: String, count: u32, at: V2, spread: f32) {
        let id = self.new_id();
        let vx = self.rng.range(-1.0, 1.0) * spread;
        let vy = self.rng.range(-430.0, -300.0);
        self.spawn_drop_exact(key, DropNet { id, item, count, x: at.x - 9.0, y: at.y - 9.0, vx, vy });
    }

    fn spawn_drop_exact(&mut self, key: &str, d: DropNet) {
        let Some(z) = self.zones.get_mut(key) else { return };
        let mut body = Body::new(d.x, d.y, 18.0, 18.0);
        body.vel = V2::new(d.vx, d.vy);
        z.drops.push(SDrop { id: d.id, item: d.item.clone(), count: d.count, body, age: 0.0 });
        self.broadcast_map(key, &ServerMsg::DropAdd(d), None);
    }

    fn pick_up(&mut self, i: usize, drop: u32, pet: bool) {
        let c = &self.clients[i];
        if !c.in_world() {
            return;
        }
        let (pid, at, key) = (c.id, c.center(), c.map.clone());
        let Some(z) = self.zones.get_mut(&key) else { return };
        let Some(k) = z.drops.iter().position(|d| d.id == drop) else { return };
        let reach = if pet { PET_PICKUP_RANGE } else { PICKUP_RANGE };
        if (z.drops[k].body.center() - at).len() > reach {
            return;
        }
        let d = z.drops.remove(k);
        self.clients[i].send(&ServerMsg::Granted { drop, item: d.item.clone(), count: d.count });
        self.broadcast_map(&key, &ServerMsg::DropGone { id: drop, by: pid }, Some(pid));
    }

    fn player_drop(&mut self, i: usize, mut d: DropNet) {
        let c = &self.clients[i];
        if !c.in_world() || c.map.is_empty() {
            return;
        }
        let Some(def) = items::item(&d.item) else { return };
        if def.bound {
            return;
        }
        let at = c.center();
        let key = c.map.clone();
        // only near the player, at a sane speed
        if (V2::new(d.x, d.y) - at).len() > 300.0 {
            d.x = at.x - 9.0;
            d.y = at.y - 9.0;
        }
        d.vx = d.vx.clamp(-400.0, 400.0);
        d.vy = d.vy.clamp(-600.0, 0.0);
        d.count = d.count.clamp(1, 999);
        d.id = self.new_id();
        self.spawn_drop_exact(&key, d);
    }

    fn boss(&mut self, i: usize, kind: u32, key: String) {
        let Some(kind) = EnemyKind::from_index(kind) else { return };
        if !self.clients[i].in_world() || !kind.is_boss() {
            return;
        }
        // The request names the map: it can arrive just before the position
        // update that moves this player there.
        let name = self.clients[i].name();
        if !self.zones.contains_key(&key) {
            let Some(m) = world::load_map(&key) else { return };
            if !m.bosses.iter().any(|b| b.kind == kind) {
                return;
            }
            let z = self.new_zone(&key);
            self.zones.insert(key.clone(), z);
        }
        let id = self.new_id();
        let z = self.zones.get_mut(&key).unwrap();
        let Some(b) = z.map.bosses.iter().find(|b| b.kind == kind).cloned() else { return };
        if z.enemies.iter().any(|e| e.kind == kind) {
            return;
        }
        z.enemies.push(Enemy::new(id, kind, b.x, b.y, None, &mut self.rng));
        self.say(format!("{} summoned the {} in {}", name, kind.name(), key));
    }
}

fn enemy_list(z: &Zone) -> Vec<EnemyNet> {
    z.enemies
        .iter()
        .map(|e| EnemyNet {
            id: e.id,
            kind: e.kind.index(),
            x: e.body.pos.x,
            y: e.body.pos.y,
            vx: e.body.vel.x,
            vy: e.body.vel.y,
            facing: if e.facing < 0.0 { -1 } else { 1 },
            state: e.state as u32,
            state_t: e.state_t,
            hp: e.hp,
            max_hp: e.max_hp,
            phase2: e.boss_phase2,
        })
        .collect()
}
