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

pub mod social;

use galebound_core::achievements;
use galebound_core::auth::{AccountStore, AuthError};
use galebound_core::net::{SocialCmd, CH_GUILD, CH_PARTY, CH_SYSTEM, CH_WHISPER, INV_FRIEND, INV_GUILD, INV_PARTY, INV_TRADE};
use galebound_core::social::{self as gsocial, FriendNet, GuildNet, MemberNet, PartyNet, ProfileNet, SocialNet, TradeNet};
use social::SocialStore;
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

mod admin;
pub use admin::{DUEL_COUNTDOWN_SECS, DUEL_SECS};

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
/// Extra dungeon monster HP for each party member after the first.
pub const DUNGEON_HP_PER_MEMBER: f32 = 0.6;
const MAX_CONNECTIONS: usize = 64;
/// Invitations (party, guild, trade) expire after this long.
const INVITE_LIFE: Duration = Duration::from_secs(60);
/// Trading partners must stand this close.
const TRADE_RANGE: f32 = 600.0;

/// A party: members by name (the first is not special; `leader` is).
struct Party {
    leader: String,
    members: Vec<String>,
}

/// A trade in progress between two players (by player id).
struct Trade {
    ids: [u32; 2],
    items: [Vec<(String, u32)>; 2],
    gold: [u32; 2],
    lock: [bool; 2],
    ok: [bool; 2],
}

struct Invite {
    kind: u32,
    from: String,
    to: String,
    extra: String,
    at: Instant,
}

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
    /// Give (true) or take (false) a player's admin role.
    Admin(String, bool),
    Admins,
    Kick(String),
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
    /// The character in play (as last saved).
    fn character(&self) -> Option<&CharacterSave> {
        self.account.as_ref()?.characters.get(self.slot?)
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
    social: SocialStore,
    parties: Vec<Party>,
    trades: Vec<Trade>,
    invites: Vec<Invite>,
    /// Guilds whose quest progress changed since the last update went out.
    guild_dirty: HashSet<String>,
    /// For tests: pretend it's this day.
    pub day_override: Option<u32>,
    /// Lowercase names of players with the admin role (admins.txt).
    admins: HashSet<String>,
    /// Muted players (lowercase name) and when it wears off.
    muted: HashMap<String, Instant>,
    /// Admins with god mode on (lowercase names).
    god: HashSet<String>,
    duels: Vec<admin::Duel>,
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
        let accounts = store.all_accounts();
        let names = accounts.iter().flat_map(|a| a.characters.iter().map(|c| c.name.to_lowercase())).collect();
        let mut social = SocialStore::load(cfg.data_dir.clone());
        for c in accounts.iter().flat_map(|a| a.characters.iter()) {
            social.remember(&c.name, c.level, c.job.map(|j| j.key()).unwrap_or(c.class.key()));
        }
        let cfg_dir = cfg.data_dir.clone();
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(7);
        Ok(Server {
            cfg,
            listeners,
            clients: Vec::new(),
            zones: HashMap::new(),
            next_id: 1,
            rng: Rng::new(seed),
            names,
            store,
            tick: 0,
            log: Vec::new(),
            social,
            parties: Vec::new(),
            trades: Vec::new(),
            invites: Vec::new(),
            guild_dirty: HashSet::new(),
            day_override: None,
            admins: admin::load_admins(&cfg_dir),
            muted: HashMap::new(),
            god: HashSet::new(),
            duels: Vec::new(),
        })
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
                            let names: Vec<String> = self
                                .clients
                                .iter()
                                .filter(|c| c.in_world())
                                .map(|c| format!("{}{} ({})", c.name(), if self.is_admin(&c.name()) { " [admin]" } else { "" }, c.map))
                                .collect();
                            self.say(format!("{} / {} players online: {}", names.len(), self.cfg.max_players, names.join(", ")));
                        }
                        Command::Admin(name, on) => {
                            let t = self.set_admin(&name, on, "The host");
                            self.say(t);
                        }
                        Command::Admins => self.list_admins(),
                        Command::Kick(name) => {
                            let t = self.kick(&name, "", "the host");
                            self.say(t);
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
        self.update_duels(1.0 / TICK_HZ as f32);
        self.tick += 1;
        if self.tick % (TICK_HZ / SNAPSHOT_HZ) as u64 == 0 {
            self.snapshots();
        }
        // party HP / map changes, and guild quest progress, twice a second
        if self.tick % (TICK_HZ as u64 / 2) == 0 {
            for p in 0..self.parties.len() {
                self.send_party(p);
            }
            let dirty: Vec<String> = self.guild_dirty.drain().collect();
            for g in dirty {
                if let Some(gi) = self.social.guild_named(&g) {
                    self.send_guild(gi);
                }
            }
            self.invites.retain(|v| v.at.elapsed() < INVITE_LIFE);
        }
        // friends' levels and whereabouts, every few seconds
        if self.tick % (TICK_HZ as u64 * 5) == 0 {
            for i in 0..self.clients.len() {
                if self.clients[i].in_world() {
                    self.send_friends(i);
                }
            }
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
                self.social.remember(&name, 1, "novice");
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
                    self.social.forget(&c.name);
                    self.save_social();
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
                info.guild = self.social.guild_of(&info.name).map(|g| self.social.guilds[g].name.clone()).unwrap_or_default();
                self.social.remember(&info.name, info.level, &info.class);
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
            ClientMsg::Social(cmd) => {
                if self.clients[i].in_world() {
                    self.social_cmd(i, cmd);
                }
            }
            ClientMsg::Admin { cmd } => self.admin_cmd(i, &cmd),
            ClientMsg::DuelHit { target, damage, kbx, kby } => self.duel_hit(i, target, damage, kbx, kby),
            ClientMsg::Act(mut a) => {
                let c = &self.clients[i];
                if !c.in_world() || c.pos.dead || c.map.is_empty() {
                    return;
                }
                a.id = c.id;
                let map = c.map.clone();
                self.broadcast_map(&map, &ServerMsg::Act(a), Some(c.id));
            }
            ClientMsg::Aid { skill, x, y } => self.aid(i, &skill, V2::new(x, y)),
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
                if text.is_empty() || self.check_mute(i) {
                    return;
                }
                let from = self.clients[i].name();
                self.say(format!("<{}> {}", from, text));
                for c in self.clients.iter().filter(|c| c.in_world()) {
                    if !self.social.blocks(&c.name(), &from) {
                        c.send(&ServerMsg::Chat { from: from.clone(), text: text.clone() });
                    }
                }
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
        let day = self.today();
        self.clients[i].send(&ServerMsg::Entered { id, day });
        let name = self.clients[i].name();
        self.say(format!("{} entered the world ({} online)", name, self.players_online()));
        self.broadcast_all(&ServerMsg::System { text: format!("{} has joined.", name) });
        if let Some(c) = self.clients[i].character() {
            let (lv, cl) = (c.level, c.job.map(|j| j.key()).unwrap_or(c.class.key()).to_string());
            self.social.remember(&name, lv, &cl);
        }
        // who the admins are (for their badge), and social state
        self.send_admins(Some(i));
        if self.is_admin(&name) {
            self.sys(i, "You're an admin. Press F9 for the Admin Panel, or type /help.".into());
        }
        self.send_friends(i);
        self.clients[i].send(&ServerMsg::Party(None));
        match self.social.guild_of(&name) {
            Some(g) => self.send_guild(g),
            None => self.clients[i].send(&ServerMsg::Guild(None)),
        }
        for f in self.social.friends_of(&name) {
            if let Some(j) = self.idx_by_name(&f) {
                self.sys(j, format!("Your friend {} is now online.", name));
                self.send_friends(j);
            }
        }
    }

    fn leave_world(&mut self, i: usize) {
        if !self.clients[i].in_world() {
            return;
        }
        let name = self.clients[i].name();
        let (id, map) = (self.clients[i].id, self.clients[i].map.clone());
        self.forfeit_duel(id, " (they left)");
        self.god.remove(&name.to_lowercase());
        self.broadcast_map(&map, &ServerMsg::PlayerGone { id }, Some(id));
        self.leave_party(&name);
        self.cancel_trade(id, &format!("{} left", name));
        self.clients[i].slot = None;
        self.clients[i].map = String::new();
        self.say(format!("{} left the world", name));
        self.broadcast_all(&ServerMsg::System { text: format!("{} has left.", name) });
        for f in self.social.friends_of(&name) {
            if let Some(j) = self.idx_by_name(&f) {
                self.send_friends(j);
            }
        }
        if let Some(g) = self.social.guild_of(&name) {
            self.send_guild(g);
        }
    }

    fn change_map(&mut self, i: usize, map: String, pos: PlayerPos) {
        let id = self.clients[i].id;
        if self.duel_of(id).is_some() {
            self.forfeit_duel(id, " (they left the area)");
        }
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
        // A dungeon run ("map#leader") gets tougher monsters for a bigger party.
        let hp_mult = self.instance_hp_mult(key);
        for (k, x, y) in z.map.fixed_enemies.clone() {
            let id = self.new_id();
            let mut e = Enemy::new(id, k, x, y, None, &mut self.rng);
            e.max_hp *= hp_mult;
            e.hp = e.max_hp;
            z.enemies.push(e);
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

    /// Monster HP multiplier for a private map copy: +60% per extra party member.
    fn instance_hp_mult(&self, key: &str) -> f32 {
        let Some((base, owner)) = key.split_once('#') else { return 1.0 };
        if !world::is_dungeon(base) {
            return 1.0;
        }
        let n = self.party_of(owner).map(|p| self.parties[p].members.len()).unwrap_or(1).max(1);
        1.0 + DUNGEON_HP_PER_MEMBER * (n - 1) as f32
    }

    fn update_zones(&mut self, dt: f32) {
        let keys: Vec<String> = self.zones.keys().cloned().collect();
        for key in keys {
            // where the players on this map are
            let players: Vec<(V2, bool)> = self.clients.iter().filter(|c| c.in_world() && c.map == key).map(|c| (c.center(), !c.pos.dead)).collect();
            // Paladins (Provoke) count as three times closer: monsters go for them first
            let threat: Vec<f32> = self.clients.iter().filter(|c| c.in_world() && c.map == key).map(|c| if is_tank(&c.info) { 3.0 } else { 1.0 }).collect();
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
                let target = players
                    .iter()
                    .zip(threat.iter())
                    .filter(|(p, _)| p.1)
                    .map(|(p, t)| (p.0, (p.0.x - c.x).abs() / t))
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                    .map(|(p, _)| p);
                let ctx = enemies::Ctx { level: &lvl, player: target.unwrap_or(V2::new(-99999.0, -99999.0)), player_alive: target.is_some(), dt };
                enemies::update(e, &ctx, &mut out);
            }
            // monsters a boss called up (Glacia's crystals, a revived Twin)
            let hp_mult = self.instance_hp_mult(&key);
            for sp in std::mem::take(&mut out.spawns) {
                let id = self.new_id();
                let mut e = Enemy::new(id, sp.kind, sp.x, sp.y, None, &mut self.rng);
                e.max_hp *= hp_mult;
                e.hp = e.max_hp * sp.hp_frac;
                z.enemies.push(e);
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

    /// A support ability or Mender turret pulse: pass it on to party
    /// members on the same map nearby.
    fn aid(&mut self, i: usize, skill: &str, at: V2) {
        use galebound_core::skills::{self, SkillEffect, SummonKind};
        let c = &self.clients[i];
        if !c.in_world() || c.pos.dead || !at.x.is_finite() || !at.y.is_finite() {
            return;
        }
        let ok = matches!(skills::skill(skill).map(|s| s.effect), Some(SkillEffect::Party { .. }) | Some(SkillEffect::Summon { kind: SummonKind::Mender, .. }));
        if !ok || (c.center() - at).len() > HIT_RANGE {
            return;
        }
        let (name, map) = (c.name(), c.map.clone());
        let Some(p) = self.party_of(&name) else { return };
        for m in self.parties[p].members.clone() {
            if social::eq(&m, &name) {
                continue;
            }
            if let Some(o) = self.clients.iter().find(|o| social::eq(&o.name(), &m) && o.in_world() && o.map == map && !o.pos.dead) {
                if (o.center() - at).len() <= galebound_core::sim::AID_RANGE {
                    o.send(&ServerMsg::Aid { from: name.clone(), skill: skill.to_string() });
                }
            }
        }
    }

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
            let kind = e.kind;
            // Glacia's crystals and the Twins react to deaths
            enemies::on_death(kind, &mut z.enemies);
            self.kill(&key, enemy);
        }
    }

    /// Hand out EXP to everyone who helped and spill the loot.
    fn kill(&mut self, key: &str, enemy: u32) {
        let Some(z) = self.zones.get_mut(key) else { return };
        let Some(e) = z.enemies.iter().find(|e| e.id == enemy) else { return };
        let (kind, at) = (e.kind, e.body.center());
        let def = kind.def();
        let mut contrib = z.contrib.remove(&enemy).unwrap_or_default();
        let total: f32 = contrib.iter().map(|c| c.1).sum::<f32>().max(1.0);
        // party members on the same map share in every kill
        let helpers: Vec<String> = contrib.iter().filter_map(|(pid, _)| self.clients.iter().find(|c| c.id == *pid)).map(|c| c.name()).collect();
        for h in &helpers {
            if let Some(p) = self.party_of(h) {
                for m in self.parties[p].members.clone() {
                    if let Some(c) = self.clients.iter().find(|c| social::eq(&c.name(), &m) && c.map == key && c.in_world()) {
                        if !contrib.iter().any(|x| x.0 == c.id) {
                            contrib.push((c.id, 0.0));
                        }
                    }
                }
            }
        }
        // guild quests: each guild counts a kill once, however many members helped
        let mut guilds_seen: Vec<usize> = Vec::new();
        for (pid, _) in &contrib {
            let Some(name) = self.clients.iter().find(|c| c.id == *pid).map(|c| c.name()) else { continue };
            if let Some(g) = self.social.guild_of(&name) {
                if !guilds_seen.contains(&g) {
                    guilds_seen.push(g);
                }
            }
        }
        for g in guilds_seen {
            self.guild_kill(g, key, def.boss);
        }
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

    // ------------------------------------------------------------------ social

    /// Today's date on the server: days since 1970 (UTC).
    pub fn today(&self) -> u32 {
        self.day_override.unwrap_or_else(|| (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) / 86_400) as u32)
    }

    fn save_social(&mut self) {
        if let Err(e) = self.social.save() {
            self.say(format!("couldn't save social data: {}", e));
        }
    }

    /// The in-world player with this character name.
    fn idx_by_name(&self, name: &str) -> Option<usize> {
        self.clients.iter().position(|c| c.in_world() && social::eq(&c.name(), name))
    }

    fn sys(&self, i: usize, text: String) {
        self.clients[i].send(&ServerMsg::ChatCh { channel: CH_SYSTEM, from: String::new(), to: String::new(), text });
    }

    fn send_friends(&self, i: usize) {
        let me = self.clients[i].name();
        let friend = |n: &String| {
            let online = self.idx_by_name(n);
            let k = self.social.known.get(&n.to_lowercase()).cloned().unwrap_or_default();
            match online {
                Some(j) => {
                    let c = &self.clients[j];
                    FriendNet { name: n.clone(), online: true, level: c.info.level.max(k.level), class: if c.info.class.is_empty() { k.class } else { c.info.class.clone() }, map: c.map.clone() }
                }
                None => FriendNet { name: n.clone(), online: false, level: k.level, class: k.class, map: String::new() },
            }
        };
        let mut friends: Vec<FriendNet> = self.social.friends_of(&me).iter().map(friend).collect();
        friends.sort_by(|a, b| b.online.cmp(&a.online).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
        let net = SocialNet { friends, incoming: self.social.incoming(&me), outgoing: self.social.outgoing(&me), blocked: self.social.blocked_by(&me) };
        self.clients[i].send(&ServerMsg::Friends(net));
    }

    fn member_net(&self, name: &str, rank: u8) -> MemberNet {
        match self.idx_by_name(name) {
            Some(j) => {
                let c = &self.clients[j];
                MemberNet { id: c.id, name: c.name(), level: c.info.level, class: c.info.class.clone(), map: c.map.clone(), hp: if c.pos.dead { 0.0 } else { c.pos.hp }, rank }
            }
            None => {
                let k = self.social.known.get(&name.to_lowercase()).cloned().unwrap_or_default();
                MemberNet { id: 0, name: name.to_string(), level: k.level, class: k.class, map: String::new(), hp: 0.0, rank }
            }
        }
    }

    fn party_of(&self, name: &str) -> Option<usize> {
        self.parties.iter().position(|p| p.members.iter().any(|m| social::eq(m, name)))
    }

    fn send_party(&self, p: usize) {
        let party = &self.parties[p];
        let net = PartyNet { leader: party.leader.clone(), members: party.members.iter().map(|m| self.member_net(m, 0)).collect() };
        for m in &party.members {
            if let Some(j) = self.idx_by_name(m) {
                self.clients[j].send(&ServerMsg::Party(Some(net.clone())));
            }
        }
    }

    fn leave_party(&mut self, name: &str) {
        let Some(p) = self.party_of(name) else { return };
        self.parties[p].members.retain(|m| !social::eq(m, name));
        if let Some(j) = self.idx_by_name(name) {
            self.clients[j].send(&ServerMsg::Party(None));
        }
        let party = &mut self.parties[p];
        if party.members.len() <= 1 {
            // a party of one is no party
            for m in party.members.clone() {
                if let Some(j) = self.idx_by_name(&m) {
                    self.clients[j].send(&ServerMsg::Party(None));
                    self.sys(j, "Your party has disbanded.".into());
                }
            }
            self.parties.remove(p);
            return;
        }
        if social::eq(&party.leader, name) {
            party.leader = party.members[0].clone();
        }
        for m in self.parties[p].members.clone() {
            if let Some(j) = self.idx_by_name(&m) {
                self.sys(j, format!("{} left the party.", name));
            }
        }
        self.send_party(p);
    }

    fn guild_net_for(&self, g: usize, viewer: &str) -> GuildNet {
        let gd = &self.social.guilds[g];
        let day = self.today();
        let (qi, _) = gsocial::guild_quest(day);
        let fresh = gd.day == day;
        let mut members: Vec<MemberNet> = gd.members.iter().map(|(n, r)| self.member_net(n, *r)).collect();
        members.sort_by(|a, b| b.online().cmp(&a.online()).then(b.rank.cmp(&a.rank)).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
        GuildNet {
            name: gd.name.clone(),
            exp: gd.exp,
            members,
            quest: qi as u32,
            quest_progress: if fresh { gd.quest_progress } else { 0 },
            quest_claimed: fresh && gd.quest_claimed.iter().any(|x| social::eq(x, viewer)),
            daily_claimed: fresh && gd.daily_claimed.iter().any(|x| social::eq(x, viewer)),
            notice: gd.notice.clone(),
        }
    }

    fn send_guild(&self, g: usize) {
        for (n, _) in &self.social.guilds[g].members {
            if let Some(j) = self.idx_by_name(n) {
                self.clients[j].send(&ServerMsg::Guild(Some(self.guild_net_for(g, n))));
            }
        }
    }

    /// Someone's guild changed: update their name tag for everyone nearby.
    fn retag(&mut self, name: &str) {
        let Some(j) = self.idx_by_name(name) else { return };
        let tag = self.social.guild_of(name).map(|g| self.social.guilds[g].name.clone()).unwrap_or_default();
        self.clients[j].info.guild = tag;
        let (map, info, id) = (self.clients[j].map.clone(), self.clients[j].info.clone(), self.clients[j].id);
        if !map.is_empty() {
            self.broadcast_map(&map, &ServerMsg::PlayerInfo(info.clone()), Some(id));
        }
    }

    fn guild_kill(&mut self, g: usize, map: &str, boss: bool) {
        let day = self.today();
        let (_, q) = gsocial::guild_quest(day);
        let gd = &mut self.social.guilds[g];
        gd.roll_day(day);
        gd.exp += 1;
        let counts = match q.goal {
            gsocial::GuildGoal::Monsters => true,
            gsocial::GuildGoal::Bosses => boss,
            gsocial::GuildGoal::MonstersOn(m) => m == map,
        };
        if counts && gd.quest_progress < q.target {
            gd.quest_progress += 1;
            if gd.quest_progress >= q.target && !gd.quest_paid {
                gd.quest_paid = true;
                gd.exp += q.guild_exp;
                let (name, members) = (gd.name.clone(), gd.members.clone());
                for (n, _) in members {
                    if let Some(j) = self.idx_by_name(&n) {
                        self.clients[j].send(&ServerMsg::ChatCh { channel: CH_GUILD, from: String::new(), to: String::new(), text: format!("Guild quest complete: {}! Claim your reward in the Guild window.", q.name) });
                    }
                }
                let _ = name;
            }
        }
        let name = self.social.guilds[g].name.clone();
        self.guild_dirty.insert(name);
        if self.tick % 60 == 0 {
            self.save_social();
        }
    }

    fn invite(&mut self, i: usize, kind: u32, to: &str, extra: String) -> Option<usize> {
        let me = self.clients[i].name();
        let Some(j) = self.idx_by_name(to) else {
            self.sys(i, format!("{} isn't online.", to));
            return None;
        };
        let to = self.clients[j].name();
        if social::eq(&me, &to) {
            return None;
        }
        if self.social.blocks(&to, &me) {
            self.sys(i, format!("{} isn't accepting invitations.", to));
            return None;
        }
        self.invites.retain(|v| !(v.kind == kind && social::eq(&v.from, &me) && social::eq(&v.to, &to)));
        self.invites.push(Invite { kind, from: me.clone(), to: to.clone(), extra: extra.clone(), at: Instant::now() });
        self.clients[j].send(&ServerMsg::Invite { kind, from: me, extra });
        Some(j)
    }

    /// Take a pending invitation (if it's still there).
    fn take_invite(&mut self, kind: u32, from: &str, to: &str) -> Option<Invite> {
        let k = self.invites.iter().position(|v| v.kind == kind && social::eq(&v.from, from) && social::eq(&v.to, to))?;
        Some(self.invites.remove(k))
    }

    fn trade_of(&self, id: u32) -> Option<(usize, usize)> {
        self.trades.iter().enumerate().find_map(|(t, tr)| tr.ids.iter().position(|x| *x == id).map(|side| (t, side)))
    }

    fn send_trade(&self, t: usize) {
        let tr = &self.trades[t];
        for side in 0..2 {
            let o = 1 - side;
            let with = self.clients.iter().find(|c| c.id == tr.ids[o]).map(|c| c.name()).unwrap_or_default();
            let net = TradeNet {
                with,
                mine: tr.items[side].clone(),
                my_gold: tr.gold[side],
                theirs: tr.items[o].clone(),
                their_gold: tr.gold[o],
                my_lock: tr.lock[side],
                their_lock: tr.lock[o],
                my_ok: tr.ok[side],
                their_ok: tr.ok[o],
            };
            if let Some(c) = self.clients.iter().find(|c| c.id == tr.ids[side]) {
                c.send(&ServerMsg::Trade { state: Some(net), reason: String::new() });
            }
        }
    }

    fn cancel_trade(&mut self, id: u32, reason: &str) {
        let Some((t, _)) = self.trade_of(id) else { return };
        let tr = self.trades.remove(t);
        for pid in tr.ids {
            if let Some(c) = self.clients.iter().find(|c| c.id == pid) {
                c.send(&ServerMsg::Trade { state: None, reason: reason.to_string() });
            }
        }
    }

    fn social_cmd(&mut self, i: usize, cmd: SocialCmd) {
        let me = self.clients[i].name();
        let my_id = self.clients[i].id;
        let clean = |t: &str| -> String { t.trim().chars().filter(|c| !c.is_control()).take(net::CHAT_MAX).collect() };
        if matches!(cmd, SocialCmd::Whisper { .. } | SocialCmd::PartyChat { .. } | SocialCmd::GuildChat { .. }) && self.check_mute(i) {
            return;
        }
        if matches!(cmd, SocialCmd::DuelRequest { .. } | SocialCmd::DuelAnswer { .. } | SocialCmd::DuelYield) {
            return self.duel_social(i, cmd);
        }
        match cmd {
            SocialCmd::Whisper { to, text } => {
                let text = clean(&text);
                if text.is_empty() {
                    return;
                }
                let Some(j) = self.idx_by_name(&to) else { return self.sys(i, format!("{} isn't online.", to)) };
                let to = self.clients[j].name();
                if self.social.blocks(&to, &me) {
                    return self.sys(i, format!("{} isn't receiving whispers.", to));
                }
                let m = ServerMsg::ChatCh { channel: CH_WHISPER, from: me.clone(), to: to.clone(), text };
                self.clients[j].send(&m);
                self.clients[i].send(&m);
            }
            SocialCmd::PartyChat { text } => {
                let text = clean(&text);
                let Some(p) = self.party_of(&me) else { return self.sys(i, "You're not in a party.".into()) };
                if text.is_empty() {
                    return;
                }
                for m in self.parties[p].members.clone() {
                    if let Some(j) = self.idx_by_name(&m) {
                        self.clients[j].send(&ServerMsg::ChatCh { channel: CH_PARTY, from: me.clone(), to: String::new(), text: text.clone() });
                    }
                }
            }
            SocialCmd::GuildChat { text } => {
                let text = clean(&text);
                let Some(g) = self.social.guild_of(&me) else { return self.sys(i, "You're not in a guild.".into()) };
                if text.is_empty() {
                    return;
                }
                for (n, _) in self.social.guilds[g].members.clone() {
                    if let Some(j) = self.idx_by_name(&n) {
                        self.clients[j].send(&ServerMsg::ChatCh { channel: CH_GUILD, from: me.clone(), to: String::new(), text: text.clone() });
                    }
                }
            }
            SocialCmd::FriendAdd { name } => {
                let Some(to) = self.social.canonical(&name) else { return self.sys(i, format!("There's no one called {} here.", name.trim())) };
                match self.social.request_friend(&me, &to) {
                    Ok(social::Requested::Sent) => {
                        self.sys(i, format!("Friend request sent to {}.", to));
                        if let Some(j) = self.idx_by_name(&to) {
                            self.clients[j].send(&ServerMsg::Invite { kind: INV_FRIEND, from: me.clone(), extra: String::new() });
                            self.send_friends(j);
                        }
                    }
                    Ok(social::Requested::Friends) => {
                        self.sys(i, format!("You and {} are now friends!", to));
                        if let Some(j) = self.idx_by_name(&to) {
                            self.sys(j, format!("You and {} are now friends!", me));
                            self.send_friends(j);
                        }
                    }
                    Err(e) => self.sys(i, e),
                }
                self.save_social();
                self.send_friends(i);
            }
            SocialCmd::FriendAnswer { name, accept } => {
                let from = self.social.canonical(&name).unwrap_or(name);
                match self.social.answer_friend(&me, &from, accept) {
                    Ok(()) if accept => {
                        self.sys(i, format!("You and {} are now friends!", from));
                        if let Some(j) = self.idx_by_name(&from) {
                            self.sys(j, format!("{} accepted your friend request.", me));
                            self.send_friends(j);
                        }
                    }
                    Ok(()) => {}
                    Err(e) => self.sys(i, e),
                }
                self.save_social();
                self.send_friends(i);
            }
            SocialCmd::FriendRemove { name } => {
                self.social.remove_friend(&me, &name);
                self.save_social();
                self.send_friends(i);
                if let Some(j) = self.idx_by_name(&name) {
                    self.send_friends(j);
                }
            }
            SocialCmd::Block { name } => {
                let Some(who) = self.social.canonical(&name) else { return self.sys(i, format!("There's no one called {} here.", name.trim())) };
                self.social.block(&me, &who);
                self.save_social();
                self.sys(i, format!("You won't see chat, whispers or invitations from {} any more.", who));
                self.send_friends(i);
                if let Some(j) = self.idx_by_name(&who) {
                    self.send_friends(j);
                }
            }
            SocialCmd::Unblock { name } => {
                self.social.unblock(&me, &name);
                self.save_social();
                self.send_friends(i);
            }
            SocialCmd::PartyInvite { name } => {
                if let Some(p) = self.party_of(&me) {
                    if !social::eq(&self.parties[p].leader, &me) {
                        return self.sys(i, "Only the party leader can invite.".into());
                    }
                    if self.parties[p].members.len() >= gsocial::PARTY_MAX {
                        return self.sys(i, "Your party is full.".into());
                    }
                }
                if self.party_of(&name).is_some() {
                    return self.sys(i, format!("{} is already in a party.", name.trim()));
                }
                if self.invite(i, INV_PARTY, &name, String::new()).is_some() {
                    self.sys(i, format!("Party invitation sent to {}.", name.trim()));
                }
            }
            SocialCmd::PartyAnswer { from, accept } => {
                let Some(inv) = self.take_invite(INV_PARTY, &from, &me) else { return self.sys(i, "That invitation has expired.".into()) };
                let Some(j) = self.idx_by_name(&inv.from) else { return };
                if !accept {
                    return self.sys(j, format!("{} declined your party invitation.", me));
                }
                if self.party_of(&me).is_some() {
                    return self.sys(i, "Leave your party first.".into());
                }
                let p = match self.party_of(&inv.from) {
                    Some(p) => p,
                    None => {
                        self.parties.push(Party { leader: inv.from.clone(), members: vec![inv.from.clone()] });
                        self.parties.len() - 1
                    }
                };
                if self.parties[p].members.len() >= gsocial::PARTY_MAX {
                    return self.sys(i, "That party is full.".into());
                }
                self.parties[p].members.push(me.clone());
                for m in self.parties[p].members.clone() {
                    if let Some(k) = self.idx_by_name(&m) {
                        self.sys(k, format!("{} joined the party.", me));
                    }
                }
                self.send_party(p);
            }
            SocialCmd::PartyLeave => self.leave_party(&me),
            SocialCmd::PartyKick { name } => {
                let Some(p) = self.party_of(&me) else { return };
                if !social::eq(&self.parties[p].leader, &me) || social::eq(&name, &me) {
                    return self.sys(i, "Only the party leader can remove members.".into());
                }
                if self.party_of(&name) == Some(p) {
                    if let Some(j) = self.idx_by_name(&name) {
                        self.sys(j, "You were removed from the party.".into());
                    }
                    self.leave_party(&name);
                }
            }
            SocialCmd::PartyPromote { name } => {
                let Some(p) = self.party_of(&me) else { return };
                if !social::eq(&self.parties[p].leader, &me) {
                    return self.sys(i, "Only the party leader can do that.".into());
                }
                if let Some(m) = self.parties[p].members.iter().find(|m| social::eq(m, &name)).cloned() {
                    self.parties[p].leader = m.clone();
                    for x in self.parties[p].members.clone() {
                        if let Some(k) = self.idx_by_name(&x) {
                            self.sys(k, format!("{} is now the party leader.", m));
                        }
                    }
                    self.send_party(p);
                }
            }
            SocialCmd::GuildCreate { name } => {
                let gold = self.clients[i].character().map(|c| c.gold).unwrap_or(0);
                if gold < gsocial::GUILD_COST {
                    return self.sys(i, format!("Founding a guild costs {} gold.", gsocial::GUILD_COST));
                }
                let day = self.today();
                match self.social.create_guild(&name, &me, day) {
                    Ok(g) => {
                        let gname = self.social.guilds[g].name.clone();
                        self.clients[i].send(&ServerMsg::Grant { gold: 0, cost: gsocial::GUILD_COST, items: vec![], reason: format!("Founded {}", gname) });
                        self.save_social();
                        self.say(format!("{} founded the guild {}", me, gname));
                        self.send_guild(g);
                        self.retag(&me);
                    }
                    Err(e) => self.sys(i, e),
                }
            }
            SocialCmd::GuildInvite { name } => {
                let Some(g) = self.social.guild_of(&me) else { return self.sys(i, "You're not in a guild.".into()) };
                if !self.social.can_manage(&me) {
                    return self.sys(i, "Only the leader and officers can invite.".into());
                }
                if self.social.guild_of(&name).is_some() {
                    return self.sys(i, format!("{} is already in a guild.", name.trim()));
                }
                let gname = self.social.guilds[g].name.clone();
                if self.invite(i, INV_GUILD, &name, gname).is_some() {
                    self.sys(i, format!("Guild invitation sent to {}.", name.trim()));
                }
            }
            SocialCmd::GuildAnswer { guild, accept } => {
                let inv = self.invites.iter().position(|v| v.kind == INV_GUILD && social::eq(&v.to, &me) && social::eq(&v.extra, &guild)).map(|k| self.invites.remove(k));
                let Some(inv) = inv else { return self.sys(i, "That invitation has expired.".into()) };
                if !accept {
                    if let Some(j) = self.idx_by_name(&inv.from) {
                        self.sys(j, format!("{} declined your guild invitation.", me));
                    }
                    return;
                }
                let Some(g) = self.social.guild_named(&guild) else { return self.sys(i, "That guild no longer exists.".into()) };
                match self.social.join_guild(g, &me) {
                    Ok(()) => {
                        self.save_social();
                        for (n, _) in self.social.guilds[g].members.clone() {
                            if let Some(k) = self.idx_by_name(&n) {
                                self.clients[k].send(&ServerMsg::ChatCh { channel: CH_GUILD, from: String::new(), to: String::new(), text: format!("{} joined the guild!", me) });
                            }
                        }
                        self.send_guild(g);
                        self.retag(&me);
                    }
                    Err(e) => self.sys(i, e),
                }
            }
            SocialCmd::GuildLeave => {
                let Some(g) = self.social.guild_of(&me) else { return };
                let members = self.social.guilds[g].members.clone();
                let gone = self.social.leave_guild(&me);
                self.save_social();
                self.clients[i].send(&ServerMsg::Guild(None));
                self.sys(i, "You left the guild.".into());
                self.retag(&me);
                if !gone {
                    if let Some(g) = self.social.guild_of(&members.iter().find(|m| !social::eq(&m.0, &me)).map(|m| m.0.clone()).unwrap_or_default()) {
                        self.send_guild(g);
                    }
                }
            }
            SocialCmd::GuildKick { name } => match self.social.kick(&me, &name) {
                Ok(()) => {
                    self.save_social();
                    if let Some(j) = self.idx_by_name(&name) {
                        self.clients[j].send(&ServerMsg::Guild(None));
                        self.sys(j, "You were removed from the guild.".into());
                    }
                    self.retag(&name);
                    if let Some(g) = self.social.guild_of(&me) {
                        self.send_guild(g);
                    }
                }
                Err(e) => self.sys(i, e),
            },
            SocialCmd::GuildRank { name, rank } => match self.social.set_rank(&me, &name, rank) {
                Ok(()) => {
                    self.save_social();
                    if let Some(g) = self.social.guild_of(&me) {
                        self.send_guild(g);
                    }
                }
                Err(e) => self.sys(i, e),
            },
            SocialCmd::GuildDisband => {
                let Some(g) = self.social.guild_of(&me) else { return };
                if self.social.guilds[g].rank(&me) != Some(gsocial::RANK_LEADER) {
                    return self.sys(i, "Only the leader can disband the guild.".into());
                }
                let gd = self.social.guilds.remove(g);
                self.save_social();
                for (n, _) in gd.members {
                    if let Some(j) = self.idx_by_name(&n) {
                        self.clients[j].send(&ServerMsg::Guild(None));
                        self.sys(j, format!("{} has been disbanded.", gd.name));
                    }
                    self.retag(&n);
                }
            }
            SocialCmd::GuildNotice { text } => {
                let Some(g) = self.social.guild_of(&me) else { return };
                if !self.social.can_manage(&me) {
                    return self.sys(i, "Only the leader and officers can change the notice.".into());
                }
                self.social.guilds[g].notice = clean(&text);
                self.save_social();
                self.send_guild(g);
            }
            SocialCmd::GuildClaimDaily | SocialCmd::GuildClaimQuest => {
                let quest = matches!(cmd, SocialCmd::GuildClaimQuest);
                let Some(g) = self.social.guild_of(&me) else { return };
                let day = self.today();
                let gd = &mut self.social.guilds[g];
                gd.roll_day(day);
                let level = gd.level();
                let (gold, items, reason) = if quest {
                    let (_, q) = gsocial::guild_quest(day);
                    if gd.quest_progress < q.target {
                        return self.sys(i, "Today's guild quest isn't finished yet.".into());
                    }
                    if gd.quest_claimed.iter().any(|x| social::eq(x, &me)) {
                        return self.sys(i, "You've already claimed today's guild quest reward.".into());
                    }
                    gd.quest_claimed.push(me.clone());
                    let (g, it) = gsocial::guild_quest_reward(level);
                    (g, it, format!("Guild quest: {}", q.name))
                } else {
                    if gd.daily_claimed.iter().any(|x| social::eq(x, &me)) {
                        return self.sys(i, "You've already opened today's guild chest.".into());
                    }
                    gd.daily_claimed.push(me.clone());
                    gd.exp += 20;
                    let (g, it) = gsocial::guild_daily_reward(level);
                    (g, it, "Guild daily chest".to_string())
                };
                let items: Vec<(String, u32)> = items.iter().map(|(id, n)| (id.to_string(), *n)).collect();
                self.clients[i].send(&ServerMsg::Grant { gold, cost: 0, items, reason });
                self.save_social();
                self.send_guild(g);
            }
            SocialCmd::TradeRequest { name } => {
                if self.trade_of(my_id).is_some() {
                    return self.sys(i, "You're already trading.".into());
                }
                let Some(j) = self.idx_by_name(&name) else { return self.sys(i, format!("{} isn't online.", name.trim())) };
                let (a, b) = (&self.clients[i], &self.clients[j]);
                if a.map != b.map || (a.center() - b.center()).len() > TRADE_RANGE {
                    return self.sys(i, "Stand closer to trade.".into());
                }
                if self.trade_of(b.id).is_some() {
                    return self.sys(i, format!("{} is busy trading.", b.name()));
                }
                if self.invite(i, INV_TRADE, &name, String::new()).is_some() {
                    self.sys(i, format!("Trade request sent to {}.", name.trim()));
                }
            }
            SocialCmd::TradeAnswer { from, accept } => {
                let Some(inv) = self.take_invite(INV_TRADE, &from, &me) else { return self.sys(i, "That trade request has expired.".into()) };
                let Some(j) = self.idx_by_name(&inv.from) else { return };
                if !accept {
                    return self.sys(j, format!("{} declined the trade.", me));
                }
                let other = self.clients[j].id;
                if self.trade_of(my_id).is_some() || self.trade_of(other).is_some() {
                    return self.sys(i, "One of you is already trading.".into());
                }
                self.trades.push(Trade { ids: [other, my_id], items: [vec![], vec![]], gold: [0, 0], lock: [false; 2], ok: [false; 2] });
                self.send_trade(self.trades.len() - 1);
            }
            SocialCmd::TradeOffer { items, gold } => {
                let Some((t, side)) = self.trade_of(my_id) else { return };
                let mut clean_items: Vec<(String, u32)> = Vec::new();
                for (id, n) in items.into_iter().take(gsocial::TRADE_SLOTS) {
                    if items::item(&id).map(|d| d.can_trade()).unwrap_or(false) && n > 0 {
                        clean_items.push((id, n.min(9999)));
                    }
                }
                let tr = &mut self.trades[t];
                tr.items[side] = clean_items;
                tr.gold[side] = gold;
                // any change unlocks both sides
                tr.lock = [false; 2];
                tr.ok = [false; 2];
                self.send_trade(t);
            }
            SocialCmd::TradeLock { locked } => {
                let Some((t, side)) = self.trade_of(my_id) else { return };
                self.trades[t].lock[side] = locked;
                self.trades[t].ok = [false; 2];
                self.send_trade(t);
            }
            SocialCmd::TradeConfirm => {
                let Some((t, side)) = self.trade_of(my_id) else { return };
                if !(self.trades[t].lock[0] && self.trades[t].lock[1]) {
                    return;
                }
                self.trades[t].ok[side] = true;
                if self.trades[t].ok[0] && self.trades[t].ok[1] {
                    let tr = self.trades.remove(t);
                    let names: Vec<String> = tr.ids.iter().map(|id| self.clients.iter().find(|c| c.id == *id).map(|c| c.name()).unwrap_or_default()).collect();
                    for s in 0..2 {
                        let o = 1 - s;
                        if let Some(c) = self.clients.iter().find(|c| c.id == tr.ids[s]) {
                            c.send(&ServerMsg::TradeDone { with: names[o].clone(), give: tr.items[s].clone(), give_gold: tr.gold[s], get: tr.items[o].clone(), get_gold: tr.gold[o] });
                            c.send(&ServerMsg::Trade { state: None, reason: String::new() });
                        }
                    }
                    self.say(format!("trade: {} <-> {}", names[0], names[1]));
                } else {
                    self.send_trade(t);
                }
            }
            SocialCmd::TradeCancel => self.cancel_trade(my_id, &format!("{} cancelled the trade", me)),
            SocialCmd::Profile { name } => {
                let Some(j) = self.idx_by_name(&name) else { return self.sys(i, format!("{} isn't online.", name.trim())) };
                let Some(p) = self.profile(j) else { return };
                self.clients[i].send(&ServerMsg::Profile(p));
            }
            SocialCmd::DuelRequest { .. } | SocialCmd::DuelAnswer { .. } | SocialCmd::DuelYield => {}
            SocialCmd::Refresh => {
                self.send_friends(i);
                match self.party_of(&me) {
                    Some(p) => self.send_party(p),
                    None => self.clients[i].send(&ServerMsg::Party(None)),
                }
                match self.social.guild_of(&me) {
                    Some(g) => self.send_guild(g),
                    None => self.clients[i].send(&ServerMsg::Guild(None)),
                }
            }
        }
    }

    /// A player's profile, from their latest save and what they've told us.
    fn profile(&self, j: usize) -> Option<ProfileNet> {
        let c = &self.clients[j];
        let save = c.character()?;
        let level = c.info.level.max(save.level);
        let need = galebound_core::sim::exp_to_next(save.level).max(1);
        Some(ProfileNet {
            name: c.name(),
            class: if c.info.class.is_empty() { save.job.map(|j| j.key()).unwrap_or(save.class.key()).to_string() } else { c.info.class.clone() },
            level,
            exp_frac: if level == save.level { (save.exp as f32 / need as f32).clamp(0.0, 1.0) } else { 0.0 },
            play_seconds: save.play_seconds,
            score: achievements::score(save),
            achievements: save.achievements.len() as u32,
            guild: c.info.guild.clone(),
            gear: save.inventory.equip.iter().map(|e| e.clone().unwrap_or_default()).collect(),
            look: if c.info.look.is_empty() { save.worn_look().encode() } else { c.info.look.clone() },
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guild_quests_count_kills_and_pay_out_once() {
        let dir = std::env::temp_dir().join(format!("gb_gq_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut s = Server::bind(Config { port: 0, max_players: 4, data_dir: dir.clone(), motd: String::new() }).unwrap();
        // day 0 % 7 = "Clear the Roads": any 120 monsters
        s.day_override = Some(7000);
        let (_, q) = gsocial::guild_quest(7000);
        assert_eq!(q.goal, gsocial::GuildGoal::Monsters);
        let g = s.social.create_guild("Testers", "Ann", 7000).unwrap();
        for _ in 0..q.target - 1 {
            s.guild_kill(g, "dawnwind_glade", false);
        }
        assert!(!s.social.guilds[g].quest_paid);
        let before = s.social.guilds[g].exp;
        s.guild_kill(g, "dawnwind_glade", false);
        s.guild_kill(g, "dawnwind_glade", false);
        assert!(s.social.guilds[g].quest_paid);
        assert_eq!(s.social.guilds[g].quest_progress, q.target, "progress stops at the target");
        assert_eq!(s.social.guilds[g].exp, before + 2 + q.guild_exp, "quest EXP paid once");
        // a new day starts a new quest
        s.day_override = Some(7001);
        s.guild_kill(g, "dawnwind_glade", false);
        assert!(!s.social.guilds[g].quest_paid);
        std::fs::remove_dir_all(dir).ok();
    }
}

/// A Paladin of level 30+ has Provoke: monsters target them first.
fn is_tank(info: &galebound_core::net::PlayerInfo) -> bool {
    info.class == "paladin" && info.level >= galebound_core::jobs::JOB_LEVEL
}
