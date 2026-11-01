//! Multiplayer protocol and connections.
//!
//! One TCP connection per player (Nagle off, so small messages go out at
//! once). Every message is one line of text: a tag, then tab-separated fields.
//! Strings are escaped so tabs and newlines can't break the framing. Plain
//! text keeps the protocol easy to read in a log and needs no extra crates.
//!
//! Who decides what:
//! - The **server** owns accounts and saves, the monsters on every map
//!   (their AI, HP and deaths), loot on the ground, who picks up what, chat and
//!   revives.
//! - Each **client** moves and animates its own character and does its own
//!   hit detection (like classic side-scrolling MMOs), then tells the server
//!   what it hit. The server sanity-checks hits (range and damage caps).

use std::io::{BufRead, BufReader, Write};
use std::net::{Shutdown, TcpStream, ToSocketAddrs};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

/// Bumped whenever the messages change; client and server must match.
pub const PROTOCOL: u32 = 1;
pub const DEFAULT_PORT: u16 = 7777;
/// Default player limit for a server.
pub const MAX_PLAYERS: usize = 16;
/// Longest chat message.
pub const CHAT_MAX: usize = 120;
/// Longest line we'll accept (a full character save is a few KB).
const LINE_MAX: usize = 64 * 1024;

// ------------------------------------------------------------------ escaping

fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => o.push_str("\\\\"),
            '\t' => o.push_str("\\t"),
            '\n' => o.push_str("\\n"),
            '\r' => {}
            c => o.push(c),
        }
    }
    o
}

fn unesc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next() {
                Some('t') => o.push('\t'),
                Some('n') => o.push('\n'),
                Some(x) => o.push(x),
                None => {}
            }
        } else {
            o.push(c);
        }
    }
    o
}

/// Builds one message line.
struct W(String);
impl W {
    fn new(tag: &str) -> W {
        W(tag.to_string())
    }
    fn s(mut self, v: &str) -> W {
        self.0.push('\t');
        self.0.push_str(&esc(v));
        self
    }
    fn n(mut self, v: impl std::fmt::Display) -> W {
        self.0.push('\t');
        self.0.push_str(&v.to_string());
        self
    }
    fn f(mut self, v: f32) -> W {
        self.0.push('\t');
        // one decimal is plenty for positions; keep small values exact-ish
        if v.fract() == 0.0 {
            self.0.push_str(&format!("{}", v as i64));
        } else {
            self.0.push_str(&format!("{:.2}", v));
        }
        self
    }
    fn b(self, v: bool) -> W {
        self.n(v as u8)
    }
}

/// Reads one message line field by field.
struct R<'a> {
    it: std::str::Split<'a, char>,
}
impl<'a> R<'a> {
    fn s(&mut self) -> Option<String> {
        self.it.next().map(unesc)
    }
    fn u(&mut self) -> Option<u32> {
        self.it.next()?.parse().ok()
    }
    fn f(&mut self) -> Option<f32> {
        self.it.next()?.parse().ok()
    }
    fn i(&mut self) -> Option<i32> {
        self.it.next()?.parse().ok()
    }
    fn b(&mut self) -> Option<bool> {
        Some(self.u()? != 0)
    }
    fn d(&mut self) -> Option<f64> {
        self.it.next()?.parse().ok()
    }
}

fn split(line: &str) -> (&str, R<'_>) {
    let mut it = line.split('\t');
    let tag = it.next().unwrap_or("");
    (tag, R { it })
}

// ------------------------------------------------------------------ payloads

/// Who a player is and what they look like (sent when it changes).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct PlayerInfo {
    pub id: u32,
    pub name: String,
    /// `Appearance::encode()` of their look (gear applied).
    pub look: String,
    /// Equipped weapon item id ("" for none).
    pub weapon: String,
    pub level: u32,
    pub class: String,
    /// Pet species key ("" when no pet is out) and its name.
    pub pet: String,
    pub pet_name: String,
    pub pet_level: u32,
}

/// Where a player is and what they're doing (sent ~15 times a second).
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct PlayerPos {
    pub id: u32,
    /// Feet position.
    pub x: f32,
    pub y: f32,
    pub facing: i32,
    /// Animation (client-defined code) and frame.
    pub anim: u32,
    pub frame: u32,
    /// HP as a fraction 0..1.
    pub hp: f32,
    pub dead: bool,
    /// Pet feet position, facing and frame (pet_on = false when none).
    pub pet_on: bool,
    pub pet_x: f32,
    pub pet_y: f32,
    pub pet_facing: i32,
    pub pet_frame: u32,
}

/// A monster as the server sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnemyNet {
    pub id: u32,
    /// Index into `enemies::ALL_KINDS`.
    pub kind: u32,
    /// Body top-left.
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub facing: i32,
    /// `EState as u32`.
    pub state: u32,
    pub state_t: f32,
    pub hp: f32,
    pub max_hp: f32,
    pub phase2: bool,
}

/// Loot on the ground. `item` is an item id, or [`GOLD`] for gold.
#[derive(Clone, Debug, PartialEq)]
pub struct DropNet {
    pub id: u32,
    pub item: String,
    pub count: u32,
    /// Body top-left and launch velocity.
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
}

/// `DropNet::item` for a pile of gold.
pub const GOLD: &str = "$gold";

/// A monster's projectile.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShotNet {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub size: f32,
    pub gravity: f32,
    pub life: f32,
    pub pierce: bool,
    pub damage: f32,
    pub kbx: f32,
    pub kby: f32,
    /// `ProjStyle as u32`.
    pub style: u32,
    pub grounded: bool,
}

// ------------------------------------------------------------------ messages

#[derive(Clone, Debug, PartialEq)]
pub enum ClientMsg {
    Hello { protocol: u32 },
    Register { email: String, password: String, confirm: String },
    Login { email: String, password: String, remember: bool },
    LoginToken { email: String, token: String },
    /// Revoke the "remember me" token.
    Forget,
    /// A new character (an encoded `CharacterSave`).
    CreateChar { save: String },
    DeleteChar { slot: u32 },
    /// Account settings (encoded, see `save::encode_settings`).
    Settings { text: String },
    Enter { slot: u32 },
    /// Back to character select.
    Leave,
    /// Progress for the character in play (an encoded `CharacterSave`).
    Save { save: String },
    Info(PlayerInfo),
    Pos { map: String, pos: PlayerPos },
    Hit { enemy: u32, damage: f32, kbx: f32, kby: f32, hitstun: f32 },
    PickUp { drop: u32, pet: bool },
    DropItem(DropNet),
    /// "My quest needs this boss here": spawns it if it isn't already.
    Boss { kind: u32 },
    Revive { target: u32 },
    Chat { text: String },
    Ping { t: f64 },
}

#[derive(Clone, Debug, PartialEq)]
pub enum ServerMsg {
    Welcome { protocol: u32, motd: String },
    Error { text: String },
    /// Your account (password data stripped) and, after a "remember me"
    /// login, the new token.
    Account { account: String, token: String },
    Entered { id: u32 },
    Left,
    PlayerInfo(PlayerInfo),
    PlayerPos(PlayerPos),
    PlayerGone { id: u32 },
    /// Every monster on your map.
    Enemies(Vec<EnemyNet>),
    Shot(ShotNet),
    Shake { amount: f32 },
    /// You helped defeat this monster: here's your EXP.
    Kill { enemy: u32, kind: u32, exp: u32 },
    DropAdd(DropNet),
    /// Someone picked it up (`by` = player id) or it expired (`by` = 0).
    DropGone { id: u32, by: u32 },
    /// The drop you asked for is yours.
    Granted { drop: u32, item: String, count: u32 },
    Revived { by: String },
    Chat { from: String, text: String },
    /// Server notices (someone joined, left...).
    System { text: String },
    Pong { t: f64 },
}

fn put_info(w: W, p: &PlayerInfo) -> W {
    w.n(p.id).s(&p.name).s(&p.look).s(&p.weapon).n(p.level).s(&p.class).s(&p.pet).s(&p.pet_name).n(p.pet_level)
}
fn get_info(r: &mut R) -> Option<PlayerInfo> {
    Some(PlayerInfo { id: r.u()?, name: r.s()?, look: r.s()?, weapon: r.s()?, level: r.u()?, class: r.s()?, pet: r.s()?, pet_name: r.s()?, pet_level: r.u()? })
}
fn put_pos(w: W, p: &PlayerPos) -> W {
    w.n(p.id)
        .f(p.x)
        .f(p.y)
        .n(p.facing)
        .n(p.anim)
        .n(p.frame)
        .f(p.hp)
        .b(p.dead)
        .b(p.pet_on)
        .f(p.pet_x)
        .f(p.pet_y)
        .n(p.pet_facing)
        .n(p.pet_frame)
}
fn get_pos(r: &mut R) -> Option<PlayerPos> {
    Some(PlayerPos {
        id: r.u()?,
        x: r.f()?,
        y: r.f()?,
        facing: r.i()?,
        anim: r.u()?,
        frame: r.u()?,
        hp: r.f()?,
        dead: r.b()?,
        pet_on: r.b()?,
        pet_x: r.f()?,
        pet_y: r.f()?,
        pet_facing: r.i()?,
        pet_frame: r.u()?,
    })
}
fn put_drop(w: W, d: &DropNet) -> W {
    w.n(d.id).s(&d.item).n(d.count).f(d.x).f(d.y).f(d.vx).f(d.vy)
}
fn get_drop(r: &mut R) -> Option<DropNet> {
    Some(DropNet { id: r.u()?, item: r.s()?, count: r.u()?, x: r.f()?, y: r.f()?, vx: r.f()?, vy: r.f()? })
}

impl ClientMsg {
    pub fn encode(&self) -> String {
        use ClientMsg::*;
        match self {
            Hello { protocol } => W::new("hello").n(protocol),
            Register { email, password, confirm } => W::new("register").s(email).s(password).s(confirm),
            Login { email, password, remember } => W::new("login").s(email).s(password).b(*remember),
            LoginToken { email, token } => W::new("token").s(email).s(token),
            Forget => W::new("forget"),
            CreateChar { save } => W::new("create").s(save),
            DeleteChar { slot } => W::new("delete").n(slot),
            Settings { text } => W::new("settings").s(text),
            Enter { slot } => W::new("enter").n(slot),
            Leave => W::new("leave"),
            Save { save } => W::new("save").s(save),
            Info(p) => put_info(W::new("info"), p),
            Pos { map, pos } => put_pos(W::new("pos").s(map), pos),
            Hit { enemy, damage, kbx, kby, hitstun } => W::new("hit").n(enemy).f(*damage).f(*kbx).f(*kby).f(*hitstun),
            PickUp { drop, pet } => W::new("pickup").n(drop).b(*pet),
            DropItem(d) => put_drop(W::new("dropitem"), d),
            Boss { kind } => W::new("boss").n(kind),
            Revive { target } => W::new("revive").n(target),
            Chat { text } => W::new("chat").s(text),
            Ping { t } => W::new("ping").n(t),
        }
        .0
    }

    pub fn decode(line: &str) -> Option<ClientMsg> {
        use ClientMsg::*;
        let (tag, mut r) = split(line);
        Some(match tag {
            "hello" => Hello { protocol: r.u()? },
            "register" => Register { email: r.s()?, password: r.s()?, confirm: r.s()? },
            "login" => Login { email: r.s()?, password: r.s()?, remember: r.b()? },
            "token" => LoginToken { email: r.s()?, token: r.s()? },
            "forget" => Forget,
            "create" => CreateChar { save: r.s()? },
            "delete" => DeleteChar { slot: r.u()? },
            "settings" => Settings { text: r.s()? },
            "enter" => Enter { slot: r.u()? },
            "leave" => Leave,
            "save" => Save { save: r.s()? },
            "info" => Info(get_info(&mut r)?),
            "pos" => Pos { map: r.s()?, pos: get_pos(&mut r)? },
            "hit" => Hit { enemy: r.u()?, damage: r.f()?, kbx: r.f()?, kby: r.f()?, hitstun: r.f()? },
            "pickup" => PickUp { drop: r.u()?, pet: r.b()? },
            "dropitem" => DropItem(get_drop(&mut r)?),
            "boss" => Boss { kind: r.u()? },
            "revive" => Revive { target: r.u()? },
            "chat" => Chat { text: r.s()? },
            "ping" => Ping { t: r.d()? },
            _ => return None,
        })
    }
}

impl ServerMsg {
    pub fn encode(&self) -> String {
        use ServerMsg::*;
        match self {
            Welcome { protocol, motd } => W::new("welcome").n(protocol).s(motd),
            Error { text } => W::new("error").s(text),
            Account { account, token } => W::new("account").s(account).s(token),
            Entered { id } => W::new("entered").n(id),
            Left => W::new("left"),
            PlayerInfo(p) => put_info(W::new("pinfo"), p),
            PlayerPos(p) => put_pos(W::new("ppos"), p),
            PlayerGone { id } => W::new("pgone").n(id),
            Enemies(list) => {
                // all monsters in one field: ';' between monsters, ',' between values
                let body: Vec<String> = list
                    .iter()
                    .map(|e| {
                        format!(
                            "{},{},{:.1},{:.1},{:.0},{:.0},{},{},{:.2},{:.0},{:.0},{}",
                            e.id, e.kind, e.x, e.y, e.vx, e.vy, e.facing, e.state, e.state_t, e.hp, e.max_hp, e.phase2 as u8
                        )
                    })
                    .collect();
                W::new("enemies").s(&body.join(";"))
            }
            Shot(s) => W::new("shot")
                .f(s.x)
                .f(s.y)
                .f(s.vx)
                .f(s.vy)
                .f(s.size)
                .f(s.gravity)
                .f(s.life)
                .b(s.pierce)
                .f(s.damage)
                .f(s.kbx)
                .f(s.kby)
                .n(s.style)
                .b(s.grounded),
            Shake { amount } => W::new("shake").f(*amount),
            Kill { enemy, kind, exp } => W::new("kill").n(enemy).n(kind).n(exp),
            DropAdd(d) => put_drop(W::new("dropadd"), d),
            DropGone { id, by } => W::new("dropgone").n(id).n(by),
            Granted { drop, item, count } => W::new("granted").n(drop).s(item).n(count),
            Revived { by } => W::new("revived").s(by),
            Chat { from, text } => W::new("chat").s(from).s(text),
            System { text } => W::new("system").s(text),
            Pong { t } => W::new("pong").n(t),
        }
        .0
    }

    pub fn decode(line: &str) -> Option<ServerMsg> {
        use ServerMsg::*;
        let (tag, mut r) = split(line);
        Some(match tag {
            "welcome" => Welcome { protocol: r.u()?, motd: r.s()? },
            "error" => Error { text: r.s()? },
            "account" => Account { account: r.s()?, token: r.s()? },
            "entered" => Entered { id: r.u()? },
            "left" => Left,
            "pinfo" => PlayerInfo(get_info(&mut r)?),
            "ppos" => PlayerPos(get_pos(&mut r)?),
            "pgone" => PlayerGone { id: r.u()? },
            "enemies" => {
                let body = r.s()?;
                let mut list = Vec::new();
                for part in body.split(';').filter(|p| !p.is_empty()) {
                    let v: Vec<&str> = part.split(',').collect();
                    if v.len() != 12 {
                        return None;
                    }
                    let f = |i: usize| v[i].parse::<f32>().ok();
                    list.push(EnemyNet {
                        id: v[0].parse().ok()?,
                        kind: v[1].parse().ok()?,
                        x: f(2)?,
                        y: f(3)?,
                        vx: f(4)?,
                        vy: f(5)?,
                        facing: v[6].parse().ok()?,
                        state: v[7].parse().ok()?,
                        state_t: f(8)?,
                        hp: f(9)?,
                        max_hp: f(10)?,
                        phase2: v[11] == "1",
                    });
                }
                Enemies(list)
            }
            "shot" => Shot(ShotNet {
                x: r.f()?,
                y: r.f()?,
                vx: r.f()?,
                vy: r.f()?,
                size: r.f()?,
                gravity: r.f()?,
                life: r.f()?,
                pierce: r.b()?,
                damage: r.f()?,
                kbx: r.f()?,
                kby: r.f()?,
                style: r.u()?,
                grounded: r.b()?,
            }),
            "shake" => Shake { amount: r.f()? },
            "kill" => Kill { enemy: r.u()?, kind: r.u()?, exp: r.u()? },
            "dropadd" => DropAdd(get_drop(&mut r)?),
            "dropgone" => DropGone { id: r.u()?, by: r.u()? },
            "granted" => Granted { drop: r.u()?, item: r.s()?, count: r.u()? },
            "revived" => Revived { by: r.s()? },
            "chat" => Chat { from: r.s()?, text: r.s()? },
            "system" => System { text: r.s()? },
            "pong" => Pong { t: r.d()? },
            _ => return None,
        })
    }
}

// ------------------------------------------------------------------ connections

/// Start background threads for a connected socket: one reads lines into
/// `incoming` (None = the connection closed), one writes queued lines.
pub fn spawn_io(stream: TcpStream) -> std::io::Result<(Sender<String>, Receiver<Option<String>>, TcpStream)> {
    stream.set_nodelay(true)?;
    let read_half = stream.try_clone()?;
    let mut write_half = stream.try_clone()?;
    let (in_tx, in_rx) = mpsc::channel::<Option<String>>();
    let (out_tx, out_rx) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        let mut r = BufReader::new(read_half);
        let mut line = String::new();
        loop {
            line.clear();
            match r.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    if line.len() > LINE_MAX {
                        break;
                    }
                    let l = line.trim_end_matches(['\n', '\r']).to_string();
                    if in_tx.send(Some(l)).is_err() {
                        return;
                    }
                }
            }
        }
        let _ = in_tx.send(None);
    });
    std::thread::spawn(move || {
        while let Ok(mut l) = out_rx.recv() {
            l.push('\n');
            // batch whatever else is already queued into one write
            while let Ok(more) = out_rx.try_recv() {
                l.push_str(&more);
                l.push('\n');
            }
            if write_half.write_all(l.as_bytes()).is_err() {
                break;
            }
        }
        let _ = write_half.shutdown(Shutdown::Both);
    });
    Ok((out_tx, in_rx, stream))
}

/// "host", "host:port" or "1.2.3.4:port" -> something we can connect to.
pub fn with_port(addr: &str) -> String {
    let a = addr.trim();
    let has_port = match a.rsplit_once(':') {
        Some((h, p)) => !h.is_empty() && !h.ends_with(':') && p.parse::<u16>().is_ok(),
        None => false,
    };
    if has_port {
        a.to_string()
    } else {
        format!("{}:{}", a, DEFAULT_PORT)
    }
}

/// The client's connection to a server.
pub struct NetClient {
    tx: Sender<String>,
    rx: Receiver<Option<String>>,
    stream: TcpStream,
    pub addr: String,
    /// Messages that arrived while waiting for a reply to a request.
    held: std::collections::VecDeque<ServerMsg>,
    closed: bool,
    pub motd: String,
    alive: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

/// How often the client says "still here" on its own, whatever screen it is
/// on (the server drops connections it hasn't heard from in a while).
pub const KEEPALIVE: Duration = Duration::from_secs(4);

impl NetClient {
    /// Connect and check that client and server speak the same protocol.
    pub fn connect(addr: &str, timeout: Duration) -> Result<NetClient, String> {
        let full = with_port(addr);
        // A name like "localhost" can mean several addresses (Windows lists
        // the IPv6 one first). Try each, IPv4 first, until one answers.
        let mut addrs: Vec<std::net::SocketAddr> = full.to_socket_addrs().map_err(|_| format!("Can't find the server '{}'", addr.trim()))?.collect();
        if addrs.is_empty() {
            return Err(format!("Can't find the server '{}'", addr.trim()));
        }
        addrs.sort_by_key(|a| a.is_ipv6());
        let each = (timeout / addrs.len() as u32).max(Duration::from_millis(800));
        let stream = addrs
            .iter()
            .find_map(|a| TcpStream::connect_timeout(a, each).ok())
            .ok_or_else(|| format!("No server answered at {}. Is it running?", full))?;
        let (tx, rx, stream) = spawn_io(stream).map_err(|e| e.to_string())?;
        let alive = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        {
            // Keepalive: menus and character creation send nothing else, and a
            // stalled window (being dragged, loading) can't send either.
            let tx = tx.clone();
            let alive = alive.clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(KEEPALIVE);
                if !alive.load(std::sync::atomic::Ordering::Relaxed) || tx.send(ClientMsg::Ping { t: -1.0 }.encode()).is_err() {
                    break;
                }
            });
        }
        let mut c = NetClient { tx, rx, stream, addr: full, held: Default::default(), closed: false, motd: String::new(), alive };
        let reply = c.request(&ClientMsg::Hello { protocol: PROTOCOL }, timeout, |m| matches!(m, ServerMsg::Welcome { .. }))?;
        if let ServerMsg::Welcome { protocol, motd } = reply {
            if protocol != PROTOCOL {
                return Err(format!("This server runs a different game version (protocol {} vs {}). Update the game.", protocol, PROTOCOL));
            }
            c.motd = motd;
        }
        Ok(c)
    }

    pub fn send(&self, m: &ClientMsg) {
        let _ = self.tx.send(m.encode());
    }

    pub fn is_closed(&self) -> bool {
        self.closed
    }

    /// Everything that has arrived (never blocks).
    pub fn poll(&mut self) -> Vec<ServerMsg> {
        let mut v: Vec<ServerMsg> = self.held.drain(..).collect();
        loop {
            match self.rx.try_recv() {
                Ok(Some(l)) => {
                    match ServerMsg::decode(&l) {
                        None => {}
                        Some(ServerMsg::Pong { t }) if t < 0.0 => {} // keepalive echo
                        Some(m) => v.push(m),
                    }
                }
                Ok(None) | Err(mpsc::TryRecvError::Disconnected) => {
                    self.closed = true;
                    break;
                }
                Err(mpsc::TryRecvError::Empty) => break,
            }
        }
        v
    }

    /// Put messages back so the next `poll` returns them first.
    pub fn unpoll(&mut self, msgs: impl IntoIterator<Item = ServerMsg>) {
        let mut v: Vec<ServerMsg> = msgs.into_iter().collect();
        v.extend(self.held.drain(..));
        self.held = v.into();
    }

    /// Send a message and wait for the reply that `is_reply` accepts (or an
    /// Error from the server). Other messages are kept for `poll`.
    pub fn request(&mut self, m: &ClientMsg, timeout: Duration, is_reply: impl Fn(&ServerMsg) -> bool) -> Result<ServerMsg, String> {
        self.send(m);
        let deadline = Instant::now() + timeout;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.rx.recv_timeout(left) {
                Ok(Some(l)) => {
                    let Some(msg) = ServerMsg::decode(&l) else { continue };
                    if matches!(msg, ServerMsg::Pong { t } if t < 0.0) {
                        continue;
                    }
                    if let ServerMsg::Error { text } = &msg {
                        return Err(text.clone());
                    }
                    if is_reply(&msg) {
                        return Ok(msg);
                    }
                    self.held.push_back(msg);
                }
                Ok(None) | Err(RecvTimeoutError::Disconnected) => {
                    self.closed = true;
                    return Err("Lost connection to the server".into());
                }
                Err(RecvTimeoutError::Timeout) => return Err("The server didn't answer in time".into()),
            }
        }
    }
}

impl Drop for NetClient {
    fn drop(&mut self) {
        self.alive.store(false, std::sync::atomic::Ordering::Relaxed);
        let _ = self.stream.shutdown(Shutdown::Both);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_roundtrip() {
        let info = PlayerInfo { id: 3, name: "Sk\teng\\".into(), look: "1,2,3".into(), weapon: "ash_bow".into(), level: 12, class: "windrunner".into(), pet: "puffling".into(), pet_name: "Puff\nball".into(), pet_level: 4 };
        let pos = PlayerPos { id: 3, x: 120.5, y: 640.0, facing: -1, anim: 17, frame: 2, hp: 0.25, dead: false, pet_on: true, pet_x: 80.0, pet_y: 640.0, pet_facing: 1, pet_frame: 1 };
        let drop = DropNet { id: 9, item: GOLD.into(), count: 40, x: 1.5, y: 2.0, vx: -30.0, vy: -300.0 };
        let cms = vec![
            ClientMsg::Hello { protocol: PROTOCOL },
            ClientMsg::Register { email: "a@b.co".into(), password: "pa ss\tword".into(), confirm: "x".into() },
            ClientMsg::Login { email: "a@b.co".into(), password: "p".into(), remember: true },
            ClientMsg::LoginToken { email: "a@b.co".into(), token: "abc".into() },
            ClientMsg::Forget,
            ClientMsg::CreateChar { save: "name=X\nlevel=1\n".into() },
            ClientMsg::DeleteChar { slot: 2 },
            ClientMsg::Settings { text: "layout=wasd\n".into() },
            ClientMsg::Enter { slot: 1 },
            ClientMsg::Leave,
            ClientMsg::Save { save: "a\nb".into() },
            ClientMsg::Info(info.clone()),
            ClientMsg::Pos { map: "breezehollow".into(), pos },
            ClientMsg::Hit { enemy: 5, damage: 33.5, kbx: -200.0, kby: -150.0, hitstun: 0.25 },
            ClientMsg::PickUp { drop: 9, pet: true },
            ClientMsg::DropItem(drop.clone()),
            ClientMsg::Boss { kind: 3 },
            ClientMsg::Revive { target: 7 },
            ClientMsg::Chat { text: "hi\tthere".into() },
            ClientMsg::Ping { t: 12.5 },
        ];
        for m in cms {
            assert_eq!(ClientMsg::decode(&m.encode()), Some(m.clone()), "{:?}", m);
            assert!(!m.encode().contains('\n'));
        }
        let e = EnemyNet { id: 4, kind: 2, x: 10.5, y: 600.0, vx: 3.0, vy: -40.0, facing: 1, state: 3, state_t: 0.25, hp: 40.0, max_hp: 50.0, phase2: true };
        let sms = vec![
            ServerMsg::Welcome { protocol: PROTOCOL, motd: "Hi".into() },
            ServerMsg::Error { text: "no".into() },
            ServerMsg::Account { account: "email=x\n[character]\n".into(), token: "".into() },
            ServerMsg::Entered { id: 3 },
            ServerMsg::Left,
            ServerMsg::PlayerInfo(info),
            ServerMsg::PlayerPos(pos),
            ServerMsg::PlayerGone { id: 3 },
            ServerMsg::Enemies(vec![e, EnemyNet { id: 5, phase2: false, ..e }]),
            ServerMsg::Enemies(vec![]),
            ServerMsg::Shot(ShotNet { x: 1.0, y: 2.0, vx: 3.0, vy: 4.0, size: 12.0, gravity: 0.0, life: 2.0, pierce: false, damage: 9.0, kbx: 200.0, kby: -100.0, style: 2, grounded: false }),
            ServerMsg::Shake { amount: 6.0 },
            ServerMsg::Kill { enemy: 4, kind: 2, exp: 17 },
            ServerMsg::DropAdd(drop),
            ServerMsg::DropGone { id: 9, by: 3 },
            ServerMsg::Granted { drop: 9, item: "moss_clump".into(), count: 1 },
            ServerMsg::Revived { by: "Friend".into() },
            ServerMsg::Chat { from: "A".into(), text: "b".into() },
            ServerMsg::System { text: "A joined".into() },
            ServerMsg::Pong { t: 1.25 },
        ];
        for m in sms {
            assert_eq!(ServerMsg::decode(&m.encode()), Some(m.clone()), "{:?}", m);
        }
        assert_eq!(ClientMsg::decode("nonsense\t1"), None);
        assert_eq!(ClientMsg::decode("hit\tx"), None);
    }

    #[test]
    fn addresses_get_the_default_port() {
        assert_eq!(with_port("100.64.1.2"), "100.64.1.2:7777");
        assert_eq!(with_port(" localhost:9000 "), "localhost:9000");
        assert_eq!(with_port("my-pc"), "my-pc:7777");
    }
}
