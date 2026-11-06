//! `galebound-server`: host a Galebound world for your friends.
//!
//!     cargo run --release -p galebound-server -- [--port 7777] [--max-players 16] [--data <folder>] [--motd "text"]
//!
//! Type `help` in the server window for commands.

use galebound_server::{Command, Config, Server};
use std::io::BufRead;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

fn main() {
    let mut cfg = Config::default();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        let val = args.get(i + 1).cloned();
        match args[i].as_str() {
            "--port" => cfg.port = val.and_then(|v| v.parse().ok()).unwrap_or(cfg.port),
            "--max-players" => cfg.max_players = val.and_then(|v| v.parse().ok()).unwrap_or(cfg.max_players).max(1),
            "--data" => cfg.data_dir = val.map(Into::into).unwrap_or(cfg.data_dir),
            "--motd" => cfg.motd = val.unwrap_or(cfg.motd),
            "-h" | "--help" => {
                println!("galebound-server [--port 7777] [--max-players 16] [--data <folder>] [--motd \"text\"]");
                return;
            }
            other => {
                eprintln!("Unknown option {} (try --help)", other);
                return;
            }
        }
        i += 2;
    }
    let mut server = match Server::bind(cfg.clone()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Couldn't start the server on port {}: {}", cfg.port, e);
            eprintln!("Is another server already running? Try --port 7778.");
            std::process::exit(1);
        }
    };
    println!("==============================================================");
    println!(" Galebound server {} - listening on port {}", env!("CARGO_PKG_VERSION"), server.port());
    println!(" Up to {} players. Saves: {}", cfg.max_players, cfg.data_dir.display());
    println!();
    println!(" You:     log in with server  localhost");
    println!(" Friends: log in with your playit.gg tunnel address, with its port,");
    println!("          e.g. galebound.playit.game:42311 (keep playit running)");
    println!();
    println!(" Type 'help' for commands. 'admin <name>' makes someone an admin.");
    println!(" 'stop' saves and shuts down.");
    println!("==============================================================");
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines().map_while(Result::ok) {
            let line = line.trim().to_string();
            let (cmd, rest) = line.split_once(' ').unwrap_or((line.as_str(), ""));
            let c = match cmd {
                "list" | "who" => Command::List,
                "say" => Command::Say(rest.to_string()),
                "stop" | "quit" | "exit" => Command::Stop,
                "admin" | "op" => Command::Admin(rest.to_string(), true),
                "unadmin" | "deop" => Command::Admin(rest.to_string(), false),
                "admins" => Command::Admins,
                "kick" => Command::Kick(rest.to_string()),
                "help" => {
                    println!("  list            who's online and where");
                    println!("  say <text>      message everyone");
                    println!("  admin <name>    give a character the admin role (F9 in game)");
                    println!("  unadmin <name>  take the admin role away");
                    println!("  admins          list the admins");
                    println!("  kick <name>     disconnect a player");
                    println!("  stop            shut the server down");
                    continue;
                }
                "" => continue,
                _ => {
                    println!("  unknown command (try help)");
                    continue;
                }
            };
            let stop = matches!(c, Command::Stop);
            if tx.send(c).is_err() || stop {
                break;
            }
        }
    });
    server.run(Arc::new(AtomicBool::new(false)), Some(rx));
    println!("Server stopped.");
}
