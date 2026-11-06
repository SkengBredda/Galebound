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
pub const PROTOCOL: u32 = 6;
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
    /// Guild name ("" for none). Filled in by the server.
    pub guild: String,
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

/// A player started an attack or used an ability, so everyone nearby can see
/// it: the swing, any projectiles (harmless copies) and ability effects.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ActNet {
    /// Player id (filled in by the server).
    pub id: u32,
    /// `AttackKind::index()`, or [`ACT_SKILL`] for a buff / heal ability.
    pub kind: u32,
    /// Ability id, "" for a basic attack.
    pub skill: String,
    /// Where the attacker's centre was, and which way they faced.
    pub x: f32,
    pub y: f32,
    pub facing: i32,
}

/// `ActNet::kind` for an ability that isn't an attack (buffs, heals).
pub const ACT_SKILL: u32 = 255;

fn put_act(w: W, a: &ActNet) -> W {
    w.n(a.id).n(a.kind).s(&a.skill).f(a.x).f(a.y).n(a.facing)
}
fn get_act(r: &mut R) -> Option<ActNet> {
    Some(ActNet { id: r.u()?, kind: r.u()?, skill: r.s()?, x: r.f()?, y: r.f()?, facing: r.i()? })
}

/// Chat channels (and the chat window's tabs, with "All" showing everything).
pub const CH_ALL: u32 = 0;
pub const CH_PARTY: u32 = 1;
pub const CH_GUILD: u32 = 2;
pub const CH_WHISPER: u32 = 3;
pub const CH_SYSTEM: u32 = 4;
/// Your own game's log (EXP, loot, achievements): only on the System tab.
pub const CH_LOG: u32 = 5;

/// Everything social a player can ask the server to do.
#[derive(Clone, Debug, PartialEq)]
pub enum SocialCmd {
    Whisper { to: String, text: String },
    PartyChat { text: String },
    GuildChat { text: String },
    FriendAdd { name: String },
    FriendAnswer { name: String, accept: bool },
    FriendRemove { name: String },
    Block { name: String },
    Unblock { name: String },
    PartyInvite { name: String },
    PartyAnswer { from: String, accept: bool },
    PartyLeave,
    PartyKick { name: String },
    PartyPromote { name: String },
    GuildCreate { name: String },
    GuildInvite { name: String },
    GuildAnswer { guild: String, accept: bool },
    GuildLeave,
    GuildKick { name: String },
    /// Set a member's rank (officer / member), or hand over leadership.
    GuildRank { name: String, rank: u8 },
    GuildDisband,
    GuildNotice { text: String },
    GuildClaimDaily,
    GuildClaimQuest,
    TradeRequest { name: String },
    TradeAnswer { from: String, accept: bool },
    TradeOffer { items: Vec<(String, u32)>, gold: u32 },
    TradeLock { locked: bool },
    TradeConfirm,
    TradeCancel,
    Profile { name: String },
    /// Ask for the friends list, party and guild again.
    Refresh,
    /// Challenge a player on your map to a duel.
    DuelRequest { name: String },
    DuelAnswer { from: String, accept: bool },
    /// Give up (also sent when your HP runs out in a duel).
    DuelYield,
}

impl SocialCmd {
    fn parts(&self) -> (&'static str, Vec<String>) {
        use SocialCmd::*;
        let b = |x: bool| (x as u8).to_string();
        match self {
            Whisper { to, text } => ("whisper", vec![to.clone(), text.clone()]),
            PartyChat { text } => ("pchat", vec![text.clone()]),
            GuildChat { text } => ("gchat", vec![text.clone()]),
            FriendAdd { name } => ("fadd", vec![name.clone()]),
            FriendAnswer { name, accept } => ("fans", vec![name.clone(), b(*accept)]),
            FriendRemove { name } => ("frem", vec![name.clone()]),
            Block { name } => ("block", vec![name.clone()]),
            Unblock { name } => ("unblock", vec![name.clone()]),
            PartyInvite { name } => ("pinv", vec![name.clone()]),
            PartyAnswer { from, accept } => ("pans", vec![from.clone(), b(*accept)]),
            PartyLeave => ("pleave", vec![]),
            PartyKick { name } => ("pkick", vec![name.clone()]),
            PartyPromote { name } => ("ppromote", vec![name.clone()]),
            GuildCreate { name } => ("gcreate", vec![name.clone()]),
            GuildInvite { name } => ("ginv", vec![name.clone()]),
            GuildAnswer { guild, accept } => ("gans", vec![guild.clone(), b(*accept)]),
            GuildLeave => ("gleave", vec![]),
            GuildKick { name } => ("gkick", vec![name.clone()]),
            GuildRank { name, rank } => ("grank", vec![name.clone(), rank.to_string()]),
            GuildDisband => ("gdisband", vec![]),
            GuildNotice { text } => ("gnotice", vec![text.clone()]),
            GuildClaimDaily => ("gdaily", vec![]),
            GuildClaimQuest => ("gquest", vec![]),
            TradeRequest { name } => ("treq", vec![name.clone()]),
            TradeAnswer { from, accept } => ("tans", vec![from.clone(), b(*accept)]),
            TradeOffer { items, gold } => ("toffer", vec![crate::social::encode_items(items), gold.to_string()]),
            TradeLock { locked } => ("tlock", vec![b(*locked)]),
            TradeConfirm => ("tok", vec![]),
            TradeCancel => ("tcancel", vec![]),
            Profile { name } => ("profile", vec![name.clone()]),
            DuelRequest { name } => ("dreq", vec![name.clone()]),
            DuelAnswer { from, accept } => ("dans", vec![from.clone(), b(*accept)]),
            DuelYield => ("dyield", vec![]),
            Refresh => ("refresh", vec![]),
        }
    }
    fn from_parts(verb: &str, a: &[String]) -> Option<SocialCmd> {
        use SocialCmd::*;
        let s = |i: usize| a.get(i).cloned();
        let b = |i: usize| a.get(i).map(|x| x == "1");
        Some(match verb {
            "whisper" => Whisper { to: s(0)?, text: s(1)? },
            "pchat" => PartyChat { text: s(0)? },
            "gchat" => GuildChat { text: s(0)? },
            "fadd" => FriendAdd { name: s(0)? },
            "fans" => FriendAnswer { name: s(0)?, accept: b(1)? },
            "frem" => FriendRemove { name: s(0)? },
            "block" => Block { name: s(0)? },
            "unblock" => Unblock { name: s(0)? },
            "pinv" => PartyInvite { name: s(0)? },
            "pans" => PartyAnswer { from: s(0)?, accept: b(1)? },
            "pleave" => PartyLeave,
            "pkick" => PartyKick { name: s(0)? },
            "ppromote" => PartyPromote { name: s(0)? },
            "gcreate" => GuildCreate { name: s(0)? },
            "ginv" => GuildInvite { name: s(0)? },
            "gans" => GuildAnswer { guild: s(0)?, accept: b(1)? },
            "gleave" => GuildLeave,
            "gkick" => GuildKick { name: s(0)? },
            "grank" => GuildRank { name: s(0)?, rank: s(1)?.parse().ok()? },
            "gdisband" => GuildDisband,
            "gnotice" => GuildNotice { text: s(0)? },
            "gdaily" => GuildClaimDaily,
            "gquest" => GuildClaimQuest,
            "treq" => TradeRequest { name: s(0)? },
            "tans" => TradeAnswer { from: s(0)?, accept: b(1)? },
            "toffer" => TradeOffer { items: crate::social::decode_items(&s(0)?), gold: s(1)?.parse().ok()? },
            "tlock" => TradeLock { locked: b(0)? },
            "tok" => TradeConfirm,
            "tcancel" => TradeCancel,
            "profile" => Profile { name: s(0)? },
            "dreq" => DuelRequest { name: s(0)? },
            "dans" => DuelAnswer { from: s(0)?, accept: s(1)? == "1" },
            "dyield" => DuelYield,
            "refresh" => Refresh,
            _ => return None,
        })
    }
}

/// What kind of invitation a popup is for.
pub const INV_FRIEND: u32 = 0;
pub const INV_PARTY: u32 = 1;
pub const INV_GUILD: u32 = 2;
pub const INV_TRADE: u32 = 3;
pub const INV_DUEL: u32 = 4;

/// Duel phases in [`ServerMsg::Duel`].
pub const DUEL_OVER: u32 = 0;
pub const DUEL_COUNTDOWN: u32 = 1;
pub const DUEL_FIGHT: u32 = 2;

/// A row of the admin panel's player list.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct AdminRow {
    pub name: String,
    pub level: u32,
    pub class: String,
    pub map: String,
    pub admin: bool,
    /// Seconds of mute left (0 = can talk).
    pub muted: u32,
}

fn encode_rows(rows: &[AdminRow]) -> String {
    rows.iter().map(|r| format!("{},{},{},{},{},{}", r.name.replace([',', ';'], ""), r.level, r.class, r.map, r.admin as u8, r.muted)).collect::<Vec<_>>().join(";")
}

fn decode_rows(s: &str) -> Vec<AdminRow> {
    s.split(';')
        .filter_map(|p| {
            let v: Vec<&str> = p.split(',').collect();
            (v.len() == 6).then(|| AdminRow {
                name: v[0].to_string(),
                level: v[1].parse().unwrap_or(1),
                class: v[2].to_string(),
                map: v[3].to_string(),
                admin: v[4] == "1",
                muted: v[5].parse().unwrap_or(0),
            })
        })
        .collect()
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
    Boss { kind: u32, map: String },
    Revive { target: u32 },
    Chat { text: String },
    Ping { t: f64 },
    Act(ActNet),
    Social(SocialCmd),
    /// An admin command line, without the slash (e.g. "kick Bob"). The
    /// server checks you're an admin.
    Admin { cmd: String },
    /// In a duel, your attack hit your opponent.
    DuelHit { target: u32, damage: f32, kbx: f32, kby: f32 },
    /// A support ability (or a Mender turret pulse) at (x, y): the server
    /// passes it to party members on the same map within `AID_RANGE`.
    Aid { skill: String, x: f32, y: f32 },
}

#[derive(Clone, Debug, PartialEq)]
pub enum ServerMsg {
    Welcome { protocol: u32, motd: String },
    Error { text: String },
    /// Your account (password data stripped) and, after a "remember me"
    /// login, the new token.
    Account { account: String, token: String },
    /// You're in. `day` is the server's date (days since 1970), used for
    /// daily rewards.
    Entered { id: u32, day: u32 },
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
    Act(ActNet),
    /// A chat line on a channel (`CH_*`). For whispers `to` is the receiver
    /// (so the sender sees "To X: ...").
    ChatCh { channel: u32, from: String, to: String, text: String },
    /// Friends, requests and block list.
    Friends(crate::social::SocialNet),
    /// Your party (None = not in one).
    Party(Option<crate::social::PartyNet>),
    Guild(Option<crate::social::GuildNet>),
    Profile(crate::social::ProfileNet),
    /// Someone invites you (`INV_*`); `extra` is the guild name for guild invites.
    Invite { kind: u32, from: String, extra: String },
    /// The trade window's state (None = closed); `reason` says why it closed.
    Trade { state: Option<crate::social::TradeNet>, reason: String },
    /// Both sides confirmed: hand over `give`, receive `get`.
    TradeDone { with: String, give: Vec<(String, u32)>, give_gold: u32, get: Vec<(String, u32)>, get_gold: u32 },
    /// The server gives you something (guild rewards), or charges you
    /// (`cost`, e.g. founding a guild).
    Grant { gold: u32, cost: u32, items: Vec<(String, u32)>, reason: String },
    /// Everyone with the admin role (sent on joining and when it changes).
    Admins { names: Vec<String> },
    /// The admin panel's player list.
    AdminPlayers(Vec<AdminRow>),
    /// The result of an admin command (shown in the panel and the System tab).
    AdminLog { text: String },
    /// An admin did something to you: "tp" (go to `map` at x, y), "heal",
    /// "level" (set your level to `n`), "god" (`n` = 1 on, 0 off).
    AdminDo { action: String, map: String, x: f32, y: f32, n: f32 },
    /// A duel's state: `phase` is DUEL_COUNTDOWN / DUEL_FIGHT with `secs`
    /// left, or DUEL_OVER with the winner's name in `name` ("" = a draw).
    Duel { foe: u32, name: String, phase: u32, secs: f32 },
    /// Your duel opponent hit you.
    DuelHit { from: u32, damage: f32, kbx: f32, kby: f32 },
    /// A party member's support ability reached you.
    Aid { from: String, skill: String },
}

fn put_info(w: W, p: &PlayerInfo) -> W {
    w.n(p.id).s(&p.name).s(&p.look).s(&p.weapon).n(p.level).s(&p.class).s(&p.pet).s(&p.pet_name).n(p.pet_level).s(&p.guild)
}
fn get_info(r: &mut R) -> Option<PlayerInfo> {
    Some(PlayerInfo { id: r.u()?, name: r.s()?, look: r.s()?, weapon: r.s()?, level: r.u()?, class: r.s()?, pet: r.s()?, pet_name: r.s()?, pet_level: r.u()?, guild: r.s()? })
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
            Boss { kind, map } => W::new("boss").n(kind).s(map),
            Revive { target } => W::new("revive").n(target),
            Chat { text } => W::new("chat").s(text),
            Ping { t } => W::new("ping").n(t),
            Act(a) => put_act(W::new("act"), a),
            Admin { cmd } => W::new("admin").s(cmd),
            DuelHit { target, damage, kbx, kby } => W::new("duelhit").n(target).f(*damage).f(*kbx).f(*kby),
            Aid { skill, x, y } => W::new("aid").s(skill).f(*x).f(*y),
            Social(c) => {
                let (verb, args) = c.parts();
                let mut w = W::new("soc").s(verb);
                for a in &args {
                    w = w.s(a);
                }
                w
            }
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
            "boss" => Boss { kind: r.u()?, map: r.s()? },
            "revive" => Revive { target: r.u()? },
            "chat" => Chat { text: r.s()? },
            "ping" => Ping { t: r.d()? },
            "act" => Act(get_act(&mut r)?),
            "admin" => Admin { cmd: r.s()? },
            "duelhit" => DuelHit { target: r.u()?, damage: r.f()?, kbx: r.f()?, kby: r.f()? },
            "aid" => Aid { skill: r.s()?, x: r.f()?, y: r.f()? },
            "soc" => {
                let verb = r.s()?;
                let mut args = Vec::new();
                while let Some(a) = r.s() {
                    args.push(a);
                }
                Social(SocialCmd::from_parts(&verb, &args)?)
            }
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
            Entered { id, day } => W::new("entered").n(id).n(day),
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
            Act(a) => put_act(W::new("act"), a),
            ChatCh { channel, from, to, text } => W::new("chatch").n(channel).s(from).s(to).s(text),
            Friends(s) => W::new("friends").s(&s.encode()),
            Party(p) => W::new("party").s(&p.as_ref().map(|p| p.encode()).unwrap_or_default()),
            Guild(g) => W::new("guild").s(&g.as_ref().map(|g| g.encode()).unwrap_or_default()),
            Profile(p) => W::new("profile").s(&p.encode()),
            Invite { kind, from, extra } => W::new("invite").n(kind).s(from).s(extra),
            Trade { state, reason } => W::new("trade").s(&state.as_ref().map(|t| t.encode()).unwrap_or_default()).s(reason),
            TradeDone { with, give, give_gold, get, get_gold } => W::new("tradedone")
                .s(with)
                .s(&crate::social::encode_items(give))
                .n(give_gold)
                .s(&crate::social::encode_items(get))
                .n(get_gold),
            Grant { gold, cost, items, reason } => W::new("grant").n(gold).n(cost).s(&crate::social::encode_items(items)).s(reason),
            Admins { names } => W::new("admins").s(&names.join(",")),
            AdminPlayers(rows) => W::new("aplayers").s(&encode_rows(rows)),
            AdminLog { text } => W::new("alog").s(text),
            AdminDo { action, map, x, y, n } => W::new("ado").s(action).s(map).f(*x).f(*y).f(*n),
            Duel { foe, name, phase, secs } => W::new("duel").n(foe).s(name).n(phase).f(*secs),
            DuelHit { from, damage, kbx, kby } => W::new("duelhit").n(from).f(*damage).f(*kbx).f(*kby),
            Aid { from, skill } => W::new("aid").s(from).s(skill),
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
            "entered" => Entered { id: r.u()?, day: r.u().unwrap_or(0) },
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
            "act" => Act(get_act(&mut r)?),
            "chatch" => ChatCh { channel: r.u()?, from: r.s()?, to: r.s()?, text: r.s()? },
            "friends" => Friends(crate::social::SocialNet::decode(&r.s()?)?),
            "party" => {
                let s = r.s()?;
                Party(if s.is_empty() { None } else { Some(crate::social::PartyNet::decode(&s)?) })
            }
            "guild" => {
                let s = r.s()?;
                Guild(if s.is_empty() { None } else { Some(crate::social::GuildNet::decode(&s)?) })
            }
            "profile" => Profile(crate::social::ProfileNet::decode(&r.s()?)?),
            "invite" => Invite { kind: r.u()?, from: r.s()?, extra: r.s()? },
            "trade" => {
                let s = r.s()?;
                Trade { state: if s.is_empty() { None } else { Some(crate::social::TradeNet::decode(&s)?) }, reason: r.s()? }
            }
            "tradedone" => TradeDone {
                with: r.s()?,
                give: crate::social::decode_items(&r.s()?),
                give_gold: r.u()?,
                get: crate::social::decode_items(&r.s()?),
                get_gold: r.u()?,
            },
            "grant" => Grant { gold: r.u()?, cost: r.u()?, items: crate::social::decode_items(&r.s()?), reason: r.s()? },
            "admins" => Admins { names: r.s()?.split(',').filter(|n| !n.is_empty()).map(String::from).collect() },
            "aplayers" => AdminPlayers(decode_rows(&r.s()?)),
            "alog" => AdminLog { text: r.s()? },
            "ado" => AdminDo { action: r.s()?, map: r.s()?, x: r.f()?, y: r.f()?, n: r.f()? },
            "duel" => Duel { foe: r.u()?, name: r.s()?, phase: r.u()?, secs: r.f()? },
            "duelhit" => DuelHit { from: r.u()?, damage: r.f()?, kbx: r.f()?, kby: r.f()? },
            "aid" => Aid { from: r.s()?, skill: r.s()? },
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
        let info = PlayerInfo { id: 3, name: "Sk\teng\\".into(), look: "1,2,3".into(), weapon: "ash_bow".into(), level: 12, class: "windrunner".into(), pet: "puffling".into(), pet_name: "Puff\nball".into(), pet_level: 4, guild: "Second~Wind".into() };
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
            ClientMsg::Boss { kind: 3, map: "dawnwind_glade".into() },
            ClientMsg::Revive { target: 7 },
            ClientMsg::Chat { text: "hi\tthere".into() },
            ClientMsg::Ping { t: 12.5 },
            ClientMsg::Act(ActNet { id: 0, kind: 4, skill: "cyclone".into(), x: 10.5, y: -3.0, facing: -1 }),
            ClientMsg::Social(SocialCmd::Whisper { to: "Mia Rose".into(), text: "hi\nthere".into() }),
            ClientMsg::Social(SocialCmd::TradeOffer { items: vec![("elixir".into(), 3), ("copper_ring".into(), 1)], gold: 99 }),
            ClientMsg::Social(SocialCmd::GuildRank { name: "Kaito".into(), rank: 1 }),
            ClientMsg::Social(SocialCmd::PartyLeave),
            ClientMsg::Social(SocialCmd::TradeOffer { items: vec![], gold: 0 }),
            ClientMsg::Social(SocialCmd::DuelRequest { name: "Mia Rose".into() }),
            ClientMsg::Social(SocialCmd::DuelAnswer { from: "Kaito".into(), accept: true }),
            ClientMsg::Social(SocialCmd::DuelYield),
            ClientMsg::Admin { cmd: "give Mia Rose elixir 3".into() },
            ClientMsg::DuelHit { target: 4, damage: 31.5, kbx: -200.0, kby: -150.0 },
            ClientMsg::Aid { skill: "sanctuary".into(), x: 10.5, y: -3.0 },
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
            ServerMsg::Entered { id: 3, day: 20400 },
            ServerMsg::ChatCh { channel: CH_WHISPER, from: "A".into(), to: "B c".into(), text: "psst\tthere".into() },
            ServerMsg::Party(None),
            ServerMsg::Guild(None),
            ServerMsg::Invite { kind: INV_GUILD, from: "Kaito".into(), extra: "Second~Wind".into() },
            ServerMsg::Trade { state: Some(crate::social::TradeNet { with: "K".into(), mine: vec![("elixir".into(), 2)], ..Default::default() }), reason: String::new() },
            ServerMsg::Trade { state: None, reason: "Kaito cancelled".into() },
            ServerMsg::TradeDone { with: "K".into(), give: vec![("elixir".into(), 1)], give_gold: 5, get: vec![], get_gold: 70 },
            ServerMsg::Admins { names: vec!["Ana".into(), "Mia Rose".into()] },
            ServerMsg::Admins { names: vec![] },
            ServerMsg::AdminPlayers(vec![AdminRow { name: "Ana".into(), level: 20, class: "vanguard".into(), map: "tempest_vault#Ana".into(), admin: true, muted: 0 }, AdminRow { name: "Bo".into(), muted: 300, ..Default::default() }]),
            ServerMsg::AdminLog { text: "Kicked Bo".into() },
            ServerMsg::AdminDo { action: "tp".into(), map: "ironhold".into(), x: 300.0, y: 640.0, n: 0.0 },
            ServerMsg::Duel { foe: 7, name: String::new(), phase: DUEL_COUNTDOWN, secs: 3.0 },
            ServerMsg::DuelHit { from: 7, damage: 20.0, kbx: 1.0, kby: -1.0 },
            ServerMsg::Aid { from: "Mia Rose".into(), skill: "mender_turret".into() },
            ServerMsg::Grant { gold: 400, cost: 0, items: vec![("elixir".into(), 2), ("revive_scroll".into(), 1)], reason: "Guild quest".into() },
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
            ServerMsg::Act(ActNet { id: 9, kind: ACT_SKILL, skill: "brace".into(), x: 1.0, y: 2.0, facing: 1 }),
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
