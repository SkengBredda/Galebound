//! End-to-end: a real server on a real socket, with real clients.

use galebound_core::appearance::Appearance;
use galebound_core::net::{ClientMsg, NetClient, PlayerInfo, PlayerPos, ServerMsg, SocialCmd, CH_PARTY, CH_SYSTEM, CH_WHISPER, GOLD, INV_FRIEND, INV_GUILD, INV_PARTY, INV_TRADE};
use galebound_core::save::{self, CharacterSave};
use galebound_server::{Config, Server};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const T: Duration = Duration::from_secs(5);

struct Running {
    port: u16,
    stop: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
    dir: std::path::PathBuf,
}

impl Drop for Running {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn start(name: &str, max_players: usize) -> Running {
    let dir = std::env::temp_dir().join(format!("galebound_test_{}_{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let cfg = Config { port: 0, max_players, data_dir: dir.clone(), motd: "test".into() };
    let mut s = Server::bind(cfg).unwrap();
    let port = s.port();
    let stop = Arc::new(AtomicBool::new(false));
    let st = stop.clone();
    let handle = std::thread::spawn(move || s.run(st, None));
    Running { port, stop, handle: Some(handle), dir }
}

fn connect(r: &Running) -> NetClient {
    NetClient::connect(&format!("127.0.0.1:{}", r.port), T).expect("connect")
}

/// Wait for a message the predicate accepts (dropping others).
fn wait(c: &mut NetClient, what: &str, pred: impl Fn(&ServerMsg) -> bool) -> ServerMsg {
    let end = Instant::now() + T;
    while Instant::now() < end {
        let mut msgs = c.poll().into_iter();
        while let Some(m) = msgs.next() {
            if pred(&m) {
                c.unpoll(msgs); // keep what came after it
                return m;
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("timed out waiting for {}", what);
}

fn account(c: &mut NetClient, m: ClientMsg) -> Result<save::Account, String> {
    match c.request(&m, T, |m| matches!(m, ServerMsg::Account { .. }))? {
        ServerMsg::Account { account, .. } => Ok(save::decode_account(&account).unwrap()),
        _ => unreachable!(),
    }
}

/// Register, make a character and step into the world. Returns our player id.
fn join(c: &mut NetClient, email: &str, name: &str) -> u32 {
    account(c, ClientMsg::Register { email: email.into(), password: "password1".into(), confirm: "password1".into() }).unwrap();
    let ch = CharacterSave::new(name, Appearance::default());
    let acc = account(c, ClientMsg::CreateChar { save: save::encode_character(&ch) }).unwrap();
    assert_eq!(acc.characters.len(), 1);
    assert_eq!(acc.hash_hex, "-", "password data never leaves the server");
    match c.request(&ClientMsg::Enter { slot: 0 }, T, |m| matches!(m, ServerMsg::Entered { .. })).unwrap() {
        ServerMsg::Entered { id, .. } => id,
        _ => unreachable!(),
    }
}

fn pos(map: &str, x: f32, y: f32, dead: bool) -> ClientMsg {
    ClientMsg::Pos { map: map.into(), pos: PlayerPos { x, y, hp: if dead { 0.0 } else { 1.0 }, dead, facing: 1, ..Default::default() } }
}

#[test]
fn two_players_share_a_world() {
    let r = start("share", 16);
    let mut a = connect(&r);
    let mut b = connect(&r);
    let ida = join(&mut a, "a@test.com", "Alpha");
    // Names are unique on the server.
    account(&mut b, ClientMsg::Register { email: "b@test.com".into(), password: "password1".into(), confirm: "password1".into() }).unwrap();
    let dup = account(&mut b, ClientMsg::CreateChar { save: save::encode_character(&CharacterSave::new("alpha", Appearance::default())) });
    assert!(dup.unwrap_err().contains("already has that name"));
    let acc = account(&mut b, ClientMsg::CreateChar { save: save::encode_character(&CharacterSave::new("Beta", Appearance::default())) }).unwrap();
    assert_eq!(acc.characters[0].level, 1);
    let idb = match b.request(&ClientMsg::Enter { slot: 0 }, T, |m| matches!(m, ServerMsg::Entered { .. })).unwrap() {
        ServerMsg::Entered { id, .. } => id,
        _ => unreachable!(),
    };

    // Both stand in Mossy Hollow and see each other.
    a.send(&ClientMsg::Info(PlayerInfo { name: "ignored".into(), level: 3, ..Default::default() }));
    a.send(&pos("dawnwind_glade", 3700.0, 700.0, false));
    b.send(&ClientMsg::Info(PlayerInfo { level: 2, ..Default::default() }));
    b.send(&pos("dawnwind_glade", 3720.0, 700.0, false));
    let m = wait(&mut b, "Alpha's info", |m| matches!(m, ServerMsg::PlayerInfo(p) if p.id == ida));
    if let ServerMsg::PlayerInfo(p) = m {
        assert_eq!(p.name, "Alpha", "the server sets the real name");
    }
    a.send(&pos("dawnwind_glade", 3705.0, 700.0, false));
    wait(&mut b, "Alpha moving", |m| matches!(m, ServerMsg::PlayerPos(p) if p.id == ida && p.x == 3705.0));

    // Monsters are shared: both get the same snapshot.
    let list = match wait(&mut a, "monsters", |m| matches!(m, ServerMsg::Enemies(l) if !l.is_empty())) {
        ServerMsg::Enemies(l) => l,
        _ => unreachable!(),
    };
    let moss = galebound_core::enemies::EnemyKind::Mossling.index();
    let target = *list.iter().find(|e| e.kind == moss).expect("mosslings in the hollow");
    wait(&mut b, "same monster for Beta", |m| matches!(m, ServerMsg::Enemies(l) if l.iter().any(|e| e.id == target.id)));

    // A hit from across the map is ignored.
    a.send(&pos("dawnwind_glade", 200.0, 640.0, false));
    a.send(&ClientMsg::Hit { enemy: target.id, damage: 9999.0, kbx: 0.0, kby: 0.0, hitstun: 0.1 });
    std::thread::sleep(Duration::from_millis(150));
    assert!(!a.poll().iter().any(|m| matches!(m, ServerMsg::Kill { .. })), "out-of-range hit counted");

    // Both hit it; both get EXP when it dies, split by damage (min 20% each).
    let ex = target.x + 20.0;
    a.send(&pos("dawnwind_glade", ex, 700.0, false));
    b.send(&pos("dawnwind_glade", ex + 10.0, 700.0, false));
    std::thread::sleep(Duration::from_millis(50));
    b.send(&ClientMsg::Hit { enemy: target.id, damage: 1.0, kbx: 10.0, kby: -10.0, hitstun: 0.1 });
    std::thread::sleep(Duration::from_millis(60));
    a.send(&ClientMsg::Hit { enemy: target.id, damage: 9999.0, kbx: 10.0, kby: -10.0, hitstun: 0.1 });
    let ka = wait(&mut a, "Alpha's kill", |m| matches!(m, ServerMsg::Kill { enemy, .. } if *enemy == target.id));
    let kb = wait(&mut b, "Beta's assist", |m| matches!(m, ServerMsg::Kill { enemy, .. } if *enemy == target.id));
    let full = galebound_core::enemies::EnemyKind::Mossling.def().exp;
    if let (ServerMsg::Kill { exp: ea, .. }, ServerMsg::Kill { exp: eb, .. }) = (ka, kb) {
        assert!(ea >= full * 9 / 10, "Alpha did nearly all the damage ({} of {})", ea, full);
        assert!(eb >= 1 && eb < ea, "Beta still gets a share ({})", eb);
    }

    // Its gold lands on the ground for everyone; Beta grabs it first.
    let gold = match wait(&mut b, "gold drop", |m| matches!(m, ServerMsg::DropAdd(d) if d.item == GOLD)) {
        ServerMsg::DropAdd(d) => d,
        _ => unreachable!(),
    };
    wait(&mut a, "Alpha sees the drop", |m| matches!(m, ServerMsg::DropAdd(d) if d.id == gold.id));
    std::thread::sleep(Duration::from_millis(400));
    // walk over to it, then grab it
    b.send(&pos("dawnwind_glade", gold.x + 9.0, 700.0, false));
    a.send(&pos("dawnwind_glade", gold.x + 9.0, 700.0, false));
    std::thread::sleep(Duration::from_millis(60));
    b.send(&ClientMsg::PickUp { drop: gold.id, pet: false });
    std::thread::sleep(Duration::from_millis(60));
    a.send(&ClientMsg::PickUp { drop: gold.id, pet: false });
    let g = wait(&mut b, "Beta gets the gold", |m| matches!(m, ServerMsg::Granted { .. }));
    assert_eq!(g, ServerMsg::Granted { drop: gold.id, item: GOLD.into(), count: gold.count });
    wait(&mut a, "Alpha told it's gone", |m| matches!(m, ServerMsg::DropGone { id, by } if *id == gold.id && *by == idb));
    std::thread::sleep(Duration::from_millis(150));
    assert!(!a.poll().iter().any(|m| matches!(m, ServerMsg::Granted { .. })), "loot can only be taken once");

    // Dropping an item makes it appear for the other player; bound items don't drop.
    a.send(&ClientMsg::DropItem(galebound_core::net::DropNet { id: 0, item: "trainee_hp_potion".into(), count: 1, x: ex, y: 660.0, vx: 0.0, vy: -100.0 }));
    a.send(&ClientMsg::DropItem(galebound_core::net::DropNet { id: 0, item: "wool_tuft".into(), count: 2, x: ex, y: 660.0, vx: 0.0, vy: -100.0 }));
    let d = wait(&mut b, "dropped item", |m| matches!(m, ServerMsg::DropAdd(d) if d.item != GOLD));
    assert!(matches!(d, ServerMsg::DropAdd(ref d) if d.item == "wool_tuft" && d.count == 2));

    // Chat reaches everyone.
    a.send(&ClientMsg::Chat { text: "  hello\tfriends  ".into() });
    let c = wait(&mut b, "chat", |m| matches!(m, ServerMsg::Chat { .. }));
    assert_eq!(c, ServerMsg::Chat { from: "Alpha".into(), text: "hellofriends".into() });

    // Beta falls; Alpha revives them.
    b.send(&pos("dawnwind_glade", ex + 10.0, 700.0, true));
    std::thread::sleep(Duration::from_millis(100));
    a.send(&ClientMsg::Revive { target: idb });
    let rv = wait(&mut b, "revive", |m| matches!(m, ServerMsg::Revived { .. }));
    assert_eq!(rv, ServerMsg::Revived { by: "Alpha".into() });

    // Quest bosses appear when asked for, once.
    let bramble = galebound_core::enemies::EnemyKind::Bramblehorn.index();
    a.send(&ClientMsg::Boss { kind: bramble, map: "dawnwind_glade".into() });
    a.send(&ClientMsg::Boss { kind: bramble, map: "dawnwind_glade".into() });
    let l = match wait(&mut a, "boss in snapshot", |m| matches!(m, ServerMsg::Enemies(l) if l.iter().any(|e| e.kind == bramble))) {
        ServerMsg::Enemies(l) => l,
        _ => unreachable!(),
    };
    assert_eq!(l.iter().filter(|e| e.kind == bramble).count(), 1);

    // Leaving tells the others.
    b.send(&ClientMsg::Leave);
    wait(&mut a, "Beta gone", |m| matches!(m, ServerMsg::PlayerGone { id } if *id == idb));
}

#[test]
fn saves_logins_and_limits() {
    let r = start("saves", 2);
    let mut a = connect(&r);
    join(&mut a, "a@test.com", "Alpha");
    // Save some progress.
    let mut ch = CharacterSave::new("Hacker", Appearance::default());
    ch.level = 5;
    ch.gold = 321;
    a.send(&ClientMsg::Save { save: save::encode_character(&ch) });
    a.send(&ClientMsg::Ping { t: 1.0 });
    wait(&mut a, "pong", |m| matches!(m, ServerMsg::Pong { .. }));

    // Wrong password, then a "remember me" login from a second computer,
    // which signs the first one out.
    let mut a2 = connect(&r);
    let e = account(&mut a2, ClientMsg::Login { email: "a@test.com".into(), password: "nope-nope".into(), remember: false }).unwrap_err();
    assert!(e.contains("incorrect"), "{}", e);
    let token = match a2.request(&ClientMsg::Login { email: "A@Test.com ".into(), password: "password1".into(), remember: true }, T, |m| matches!(m, ServerMsg::Account { .. })).unwrap() {
        ServerMsg::Account { account, token } => {
            let acc = save::decode_account(&account).unwrap();
            assert_eq!(acc.characters[0].level, 5, "progress was saved");
            assert_eq!(acc.characters[0].gold, 321);
            assert_eq!(acc.characters[0].name, "Alpha", "saving can't rename a character");
            token
        }
        _ => unreachable!(),
    };
    assert!(!token.is_empty());
    wait(&mut a, "signed out elsewhere", |m| matches!(m, ServerMsg::Error { text } if text.contains("another computer")));

    // The token logs in on its own.
    let mut a3 = connect(&r);
    assert!(account(&mut a3, ClientMsg::LoginToken { email: "a@test.com".into(), token: token.clone() }).is_ok());
    assert!(account(&mut a3, ClientMsg::LoginToken { email: "a@test.com".into(), token: "00".into() }).is_err());

    // The server holds 2 players: the third is turned away.
    let mut b = connect(&r);
    join(&mut b, "b@test.com", "Beta");
    a3.send(&ClientMsg::Enter { slot: 0 });
    let mut c = connect(&r);
    account(&mut c, ClientMsg::Register { email: "c@test.com".into(), password: "password1".into(), confirm: "password1".into() }).unwrap();
    account(&mut c, ClientMsg::CreateChar { save: save::encode_character(&CharacterSave::new("Gamma", Appearance::default())) }).unwrap();
    let full = c.request(&ClientMsg::Enter { slot: 0 }, T, |m| matches!(m, ServerMsg::Entered { .. })).unwrap_err();
    assert!(full.contains("full"), "{}", full);

    // Deleting a character frees its name.
    let acc = account(&mut c, ClientMsg::DeleteChar { slot: 0 }).unwrap();
    assert!(acc.characters.is_empty());
    assert!(account(&mut c, ClientMsg::CreateChar { save: save::encode_character(&CharacterSave::new("Gamma", Appearance::default())) }).is_ok());
}

#[test]
fn version_mismatch_is_explained() {
    let r = start("version", 4);
    // A raw client speaking a future protocol gets told the server's version.
    let mut c = connect(&r);
    let w = c.request(&ClientMsg::Hello { protocol: 999 }, T, |m| matches!(m, ServerMsg::Welcome { .. })).unwrap();
    assert!(matches!(w, ServerMsg::Welcome { protocol, .. } if protocol == galebound_core::net::PROTOCOL));
    // "localhost" works whichever address it resolves to first (IPv6 ::1 on Windows).
    assert!(NetClient::connect(&format!("localhost:{}", r.port), T).is_ok(), "localhost");
    assert!(NetClient::connect(&format!("[::1]:{}", r.port), T).is_ok() || std::net::TcpListener::bind("[::1]:0").is_err(), "IPv6 loopback");
    // And nothing listening is a clear error.
    let err = NetClient::connect("127.0.0.1:1", Duration::from_millis(500)).err().unwrap();
    assert!(err.contains("No server answered"), "{}", err);
}

/// Sitting on the login / character screens sends nothing by hand, so the
/// connection must keep itself alive past the server's idle limit.
#[test]
fn idle_menus_stay_connected() {
    let r = start("idle", 2);
    let mut a = connect(&r);
    std::thread::sleep(galebound_core::net::KEEPALIVE * 3);
    assert!(a.poll().iter().all(|m| !matches!(m, ServerMsg::Pong { .. })), "keepalive echoes are hidden");
    assert!(!a.is_closed());
    join(&mut a, "idle@test.com", "Idler");
}

fn soc(c: &NetClient, cmd: SocialCmd) {
    c.send(&ClientMsg::Social(cmd));
}

fn system_text(c: &mut NetClient, what: &str, needle: &'static str) -> String {
    match wait(c, what, |m| matches!(m, ServerMsg::ChatCh { channel, text, .. } if *channel == CH_SYSTEM && text.contains(needle))) {
        ServerMsg::ChatCh { text, .. } => text,
        _ => unreachable!(),
    }
}

#[test]
fn friends_party_guild_and_trade() {
    let r = start("social", 16);
    let (mut a, mut b, mut c) = (connect(&r), connect(&r), connect(&r));
    let _ida = join(&mut a, "a@soc.com", "Alpha");
    let idb = join(&mut b, "b@soc.com", "Beta");
    let _idc = join(&mut c, "c@soc.com", "Cyan");
    for (cl, x) in [(&a, 3700.0), (&b, 3720.0), (&c, 3740.0)] {
        cl.send(&ClientMsg::Info(PlayerInfo { level: 5, class: "novice".into(), ..Default::default() }));
        cl.send(&pos("dawnwind_glade", x, 700.0, false));
    }
    std::thread::sleep(Duration::from_millis(100));

    // --- friends: request, popup, accept
    soc(&a, SocialCmd::FriendAdd { name: "beta".into() });
    wait(&mut b, "friend invite", |m| matches!(m, ServerMsg::Invite { kind, from, .. } if *kind == INV_FRIEND && from == "Alpha"));
    soc(&b, SocialCmd::FriendAnswer { name: "Alpha".into(), accept: true });
    let f = wait(&mut a, "friends list", |m| matches!(m, ServerMsg::Friends(s) if s.friends.iter().any(|f| f.name == "Beta" && f.online)));
    if let ServerMsg::Friends(s) = f {
        assert_eq!(s.friends[0].map, "dawnwind_glade");
    }
    soc(&a, SocialCmd::FriendAdd { name: "Nobody Here".into() });
    system_text(&mut a, "unknown name", "no one called");

    // --- whispers, and blocking
    soc(&a, SocialCmd::Whisper { to: "beta".into(), text: "psst".into() });
    let w = wait(&mut b, "whisper", |m| matches!(m, ServerMsg::ChatCh { channel, .. } if *channel == CH_WHISPER));
    assert_eq!(w, ServerMsg::ChatCh { channel: CH_WHISPER, from: "Alpha".into(), to: "Beta".into(), text: "psst".into() });
    wait(&mut a, "whisper echo", |m| matches!(m, ServerMsg::ChatCh { channel, to, .. } if *channel == CH_WHISPER && to == "Beta"));
    soc(&c, SocialCmd::Block { name: "alpha".into() });
    wait(&mut c, "block list", |m| matches!(m, ServerMsg::Friends(s) if s.blocked == vec!["Alpha".to_string()]));
    soc(&a, SocialCmd::Whisper { to: "Cyan".into(), text: "hello?".into() });
    system_text(&mut a, "blocked whisper", "isn't receiving whispers");
    a.send(&ClientMsg::Chat { text: "from alpha".into() });
    std::thread::sleep(Duration::from_millis(60));
    b.send(&ClientMsg::Chat { text: "from beta".into() });
    let mut seen = Vec::new();
    let end = Instant::now() + T;
    while Instant::now() < end && !seen.iter().any(|t: &String| t == "from beta") {
        for m in c.poll() {
            if let ServerMsg::Chat { text, .. } = m {
                seen.push(text);
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(seen, vec!["from beta".to_string()], "Cyan never hears from Alpha");
    soc(&a, SocialCmd::PartyInvite { name: "Cyan".into() });
    system_text(&mut a, "blocked invite", "isn't accepting");

    // --- party: invite, join, party chat, shared EXP
    soc(&a, SocialCmd::PartyInvite { name: "Beta".into() });
    wait(&mut b, "party invite", |m| matches!(m, ServerMsg::Invite { kind, .. } if *kind == INV_PARTY));
    soc(&b, SocialCmd::PartyAnswer { from: "Alpha".into(), accept: true });
    let p = wait(&mut a, "party", |m| matches!(m, ServerMsg::Party(Some(p)) if p.members.len() == 2));
    if let ServerMsg::Party(Some(p)) = p {
        assert_eq!(p.leader, "Alpha");
        assert!(p.members.iter().all(|m| m.online()));
    }
    soc(&b, SocialCmd::PartyChat { text: "ready".into() });
    wait(&mut a, "party chat", |m| matches!(m, ServerMsg::ChatCh { channel, from, .. } if *channel == CH_PARTY && from == "Beta"));
    let list = match wait(&mut a, "monsters", |m| matches!(m, ServerMsg::Enemies(l) if !l.is_empty())) {
        ServerMsg::Enemies(l) => l,
        _ => unreachable!(),
    };
    let moss = galebound_core::enemies::EnemyKind::Mossling.index();
    let target = *list.iter().find(|e| e.kind == moss).unwrap();
    a.send(&pos("dawnwind_glade", target.x + 20.0, 700.0, false));
    std::thread::sleep(Duration::from_millis(60));
    a.send(&ClientMsg::Hit { enemy: target.id, damage: 99999.0, kbx: 0.0, kby: 0.0, hitstun: 0.1 });
    wait(&mut a, "Alpha's kill", |m| matches!(m, ServerMsg::Kill { enemy, .. } if *enemy == target.id));
    wait(&mut b, "Beta shares the kill without hitting it", |m| matches!(m, ServerMsg::Kill { enemy, .. } if *enemy == target.id));

    // --- guild: found (costs gold), invite, name tag, daily chest
    let mut rich = CharacterSave::new("Alpha", Appearance::default());
    rich.gold = 6000;
    a.send(&ClientMsg::Save { save: save::encode_character(&rich) });
    soc(&a, SocialCmd::GuildCreate { name: "Second~Wind".into() });
    let g = wait(&mut a, "guild cost", |m| matches!(m, ServerMsg::Grant { .. }));
    assert!(matches!(g, ServerMsg::Grant { cost: 5000, .. }));
    wait(&mut a, "guild", |m| matches!(m, ServerMsg::Guild(Some(g)) if g.name == "Second~Wind"));
    wait(&mut b, "Alpha's guild tag", |m| matches!(m, ServerMsg::PlayerInfo(p) if p.name == "Alpha" && p.guild == "Second~Wind"));
    soc(&a, SocialCmd::GuildInvite { name: "Beta".into() });
    wait(&mut b, "guild invite", |m| matches!(m, ServerMsg::Invite { kind, extra, .. } if *kind == INV_GUILD && extra == "Second~Wind"));
    soc(&b, SocialCmd::GuildAnswer { guild: "Second~Wind".into(), accept: true });
    wait(&mut a, "two members", |m| matches!(m, ServerMsg::Guild(Some(g)) if g.members.len() == 2));
    soc(&b, SocialCmd::GuildClaimDaily);
    let gr = wait(&mut b, "daily chest", |m| matches!(m, ServerMsg::Grant { .. }));
    assert!(matches!(gr, ServerMsg::Grant { gold, cost: 0, .. } if gold > 0));
    soc(&b, SocialCmd::GuildClaimDaily);
    system_text(&mut b, "only once a day", "already opened");
    soc(&b, SocialCmd::GuildKick { name: "Alpha".into() });
    system_text(&mut b, "members can't kick the leader", "can't remove");

    // --- trade: request, offers (bound items refused), lock, confirm
    // (stand together: trading needs you to be close)
    a.send(&pos("dawnwind_glade", 3700.0, 700.0, false));
    b.send(&pos("dawnwind_glade", 3720.0, 700.0, false));
    std::thread::sleep(Duration::from_millis(80));
    soc(&a, SocialCmd::TradeRequest { name: "Beta".into() });
    wait(&mut b, "trade request", |m| matches!(m, ServerMsg::Invite { kind, .. } if *kind == INV_TRADE));
    soc(&b, SocialCmd::TradeAnswer { from: "Alpha".into(), accept: true });
    wait(&mut a, "trade window", |m| matches!(m, ServerMsg::Trade { state: Some(t), .. } if t.with == "Beta"));
    soc(&a, SocialCmd::TradeOffer { items: vec![("elixir".into(), 2), ("trainee_hp_potion".into(), 5)], gold: 10 });
    let t = wait(&mut b, "Alpha's offer", |m| matches!(m, ServerMsg::Trade { state: Some(t), .. } if !t.theirs.is_empty()));
    if let ServerMsg::Trade { state: Some(t), .. } = t {
        assert_eq!(t.theirs, vec![("elixir".to_string(), 2)], "bound items can't be traded");
        assert_eq!(t.their_gold, 10);
    }
    soc(&b, SocialCmd::TradeOffer { items: vec![], gold: 5 });
    std::thread::sleep(Duration::from_millis(50));
    soc(&a, SocialCmd::TradeConfirm);
    soc(&a, SocialCmd::TradeLock { locked: true });
    soc(&b, SocialCmd::TradeLock { locked: true });
    std::thread::sleep(Duration::from_millis(50));
    soc(&a, SocialCmd::TradeConfirm);
    soc(&b, SocialCmd::TradeConfirm);
    let da = wait(&mut a, "trade done", |m| matches!(m, ServerMsg::TradeDone { .. }));
    assert_eq!(da, ServerMsg::TradeDone { with: "Beta".into(), give: vec![("elixir".into(), 2)], give_gold: 10, get: vec![], get_gold: 5 });
    let db = wait(&mut b, "trade done", |m| matches!(m, ServerMsg::TradeDone { .. }));
    assert_eq!(db, ServerMsg::TradeDone { with: "Alpha".into(), give: vec![], give_gold: 5, get: vec![("elixir".into(), 2)], get_gold: 10 });

    // --- profiles
    soc(&b, SocialCmd::Profile { name: "alpha".into() });
    let pr = wait(&mut b, "profile", |m| matches!(m, ServerMsg::Profile(_)));
    if let ServerMsg::Profile(p) = pr {
        assert_eq!(p.name, "Alpha");
        assert_eq!(p.guild, "Second~Wind");
        assert_eq!(p.gear.len(), galebound_core::items::EQUIP_COUNT);
    }

    // --- leaving: party shrinks away, friends see you go offline
    a.send(&ClientMsg::Leave);
    wait(&mut b, "party over", |m| matches!(m, ServerMsg::Party(None)));
    wait(&mut b, "Alpha offline", |m| matches!(m, ServerMsg::Friends(s) if s.friends.iter().any(|f| f.name == "Alpha" && !f.online)));
    let _ = idb;
}
