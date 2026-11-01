//! End-to-end: a real server on a real socket, with real clients.

use galebound_core::appearance::Appearance;
use galebound_core::net::{ClientMsg, NetClient, PlayerInfo, PlayerPos, ServerMsg, GOLD};
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
        ServerMsg::Entered { id } => id,
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
        ServerMsg::Entered { id } => id,
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
