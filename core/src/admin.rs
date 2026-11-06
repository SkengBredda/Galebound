//! Admin commands. The host gives a player the admin role from the server
//! window (`admin <name>`), and admins can give it to others. Admins type
//! these in chat with a slash (`/kick Bob`) or use the Admin Panel (F9); the
//! server checks the role and carries them out.
//!
//! `me` can stand in for your own name wherever a player is asked for.

/// The highest level `/level` sets.
pub const MAX_LEVEL: u32 = crate::classes::LEVEL_CAP;

/// One admin command: its name, how to use it and what it does.
#[derive(Clone, Copy, Debug)]
pub struct AdminCmd {
    pub name: &'static str,
    pub usage: &'static str,
    pub help: &'static str,
}

const fn c(name: &'static str, usage: &'static str, help: &'static str) -> AdminCmd {
    AdminCmd { name, usage, help }
}

pub const COMMANDS: &[AdminCmd] = &[
    c("help", "/help", "List the admin commands."),
    c("players", "/players", "Refresh the Admin Panel's player list."),
    c("announce", "/announce <text>", "A server-wide announcement everyone sees on screen."),
    c("goto", "/goto <player>", "Teleport to a player."),
    c("summon", "/summon <player>", "Bring a player to you."),
    c("heal", "/heal <player>", "Fully heal a player (or revive them)."),
    c("give", "/give <player> <item id> [count]", "Give a player an item (e.g. /give me elixir 5)."),
    c("gold", "/gold <player> <amount>", "Give a player gold."),
    c("level", "/level <player> <1-60>", "Set a player's level."),
    c("god", "/god", "Toggle god mode for yourself (no damage)."),
    c("spawn", "/spawn <monster> [count]", "Spawn monsters next to you (e.g. /spawn mossling 3)."),
    c("clear", "/clear", "Remove every monster on your map (they come back as usual)."),
    c("mute", "/mute <player> [minutes]", "Stop a player chatting (default 10 minutes)."),
    c("unmute", "/unmute <player>", "Let a muted player chat again."),
    c("kick", "/kick <player> [reason]", "Disconnect a player."),
    c("admin", "/admin <player>", "Give a player the admin role."),
    c("unadmin", "/unadmin <player>", "Take the admin role away."),
];

pub fn command(name: &str) -> Option<&'static AdminCmd> {
    COMMANDS.iter().find(|c| c.name.eq_ignore_ascii_case(name))
}

/// Is this typed chat line (with its slash) an admin command?
pub fn is_admin_line(line: &str) -> bool {
    let Some(rest) = line.strip_prefix('/') else { return false };
    let word = rest.split_whitespace().next().unwrap_or("");
    command(word).is_some()
}

/// Split a command line into its command and the rest.
pub fn split(line: &str) -> (String, String) {
    let line = line.trim().trim_start_matches('/');
    match line.split_once(' ') {
        Some((a, b)) => (a.to_lowercase(), b.trim().to_string()),
        None => (line.to_lowercase(), String::new()),
    }
}

/// Character names may contain spaces: take the longest known name that
/// starts `args`, and return it with what follows. Falls back to the first
/// word.
pub fn take_name(args: &str, known: &[String]) -> (String, String) {
    let lower = args.to_lowercase();
    let mut best: Option<&String> = None;
    for n in known {
        let nl = n.to_lowercase();
        if (lower == nl || lower.starts_with(&format!("{} ", nl))) && best.map(|b| n.len() > b.len()).unwrap_or(true) {
            best = Some(n);
        }
    }
    match best {
        Some(n) => (n.clone(), args[n.len()..].trim().to_string()),
        None => match args.split_once(' ') {
            Some((a, b)) => (a.to_string(), b.trim().to_string()),
            None => (args.to_string(), String::new()),
        },
    }
}

/// The monster a name means ("mossling", "Mossling", "the gatekeeper").
pub fn monster(name: &str) -> Option<crate::enemies::EnemyKind> {
    let n = name.trim().to_lowercase().replace([' ', '_'], "");
    let n = n.trim_start_matches("the");
    crate::enemies::ALL_KINDS.iter().copied().find(|k| k.key() == n || k.def().name.to_lowercase().replace([' ', '_'], "").trim_start_matches("the") == n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_with_spaces_and_commands() {
        let known = vec!["Mia".to_string(), "Mia Rose".to_string(), "Bo".to_string()];
        assert_eq!(take_name("Mia Rose elixir 3", &known), ("Mia Rose".into(), "elixir 3".into()));
        assert_eq!(take_name("Mia elixir", &known), ("Mia".into(), "elixir".into()));
        assert_eq!(take_name("Zed 5", &known), ("Zed".into(), "5".into()));
        assert_eq!(split("/Give Mia 5"), ("give".into(), "Mia 5".into()));
        assert!(is_admin_line("/kick Bo"));
        assert!(!is_admin_line("/w Bo hi"));
        assert!(!is_admin_line("hello"));
        assert_eq!(monster("Mossling"), Some(crate::enemies::EnemyKind::Mossling));
        assert_eq!(monster("the gatekeeper"), Some(crate::enemies::EnemyKind::Gatekeeper));
        assert_eq!(monster("nope"), None);
    }
}
