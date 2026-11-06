//! The server's long-lived social data: friendships, friend requests, block
//! lists and guilds. Kept in memory and written to two small text files in
//! the server's data folder (`social.txt`, `guilds.txt`) after each change.
//!
//! Characters are identified by name (names are unique on a server,
//! ignoring case). Parties and trades are short-lived and live in the
//! server's main loop instead.

use galebound_core::social::{self, GUILD_MAX, RANK_LEADER, RANK_MEMBER, RANK_OFFICER};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq)]
pub struct Guild {
    pub name: String,
    pub exp: u32,
    /// (character name, rank). Exactly one leader.
    pub members: Vec<(String, u8)>,
    pub notice: String,
    /// The day the counters below belong to.
    pub day: u32,
    pub quest_progress: u32,
    /// Has today's quest already paid out its guild EXP?
    pub quest_paid: bool,
    pub quest_claimed: Vec<String>,
    pub daily_claimed: Vec<String>,
}

impl Guild {
    pub fn level(&self) -> u32 {
        social::guild_level(self.exp)
    }
    pub fn has(&self, name: &str) -> bool {
        self.members.iter().any(|m| eq(&m.0, name))
    }
    pub fn rank(&self, name: &str) -> Option<u8> {
        self.members.iter().find(|m| eq(&m.0, name)).map(|m| m.1)
    }
    pub fn leader(&self) -> &str {
        self.members.iter().find(|m| m.1 == RANK_LEADER).map(|m| m.0.as_str()).unwrap_or("")
    }
    /// Start a new day: today's quest and chests reset.
    pub fn roll_day(&mut self, day: u32) {
        if self.day != day {
            self.day = day;
            self.quest_progress = 0;
            self.quest_paid = false;
            self.quest_claimed.clear();
            self.daily_claimed.clear();
        }
    }
}

pub fn eq(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// Last known level and class of a character (for offline friends).
#[derive(Clone, Debug, Default)]
pub struct Known {
    pub name: String,
    pub level: u32,
    pub class: String,
}

/// Why a friend request did what it did.
#[derive(Debug, PartialEq)]
pub enum Requested {
    /// Request sent and waiting.
    Sent,
    /// They had already asked you: you're friends now.
    Friends,
}

#[derive(Default)]
pub struct SocialStore {
    dir: Option<PathBuf>,
    pub friends: Vec<(String, String)>,
    /// (from, to)
    pub requests: Vec<(String, String)>,
    /// (who, whom they block)
    pub blocks: Vec<(String, String)>,
    pub guilds: Vec<Guild>,
    /// Every character on the server, by lower-case name.
    pub known: HashMap<String, Known>,
}

impl SocialStore {
    pub fn load(dir: PathBuf) -> SocialStore {
        let mut s = SocialStore { dir: Some(dir.clone()), ..Default::default() };
        if let Ok(text) = std::fs::read_to_string(dir.join("social.txt")) {
            for line in text.lines() {
                let p: Vec<&str> = line.split('\t').collect();
                if p.len() != 3 {
                    continue;
                }
                let pair = (p[1].to_string(), p[2].to_string());
                match p[0] {
                    "friend" => s.friends.push(pair),
                    "request" => s.requests.push(pair),
                    "block" => s.blocks.push(pair),
                    _ => {}
                }
            }
        }
        if let Ok(text) = std::fs::read_to_string(dir.join("guilds.txt")) {
            let mut cur: Option<Guild> = None;
            for line in text.lines() {
                if line == "[guild]" {
                    if let Some(g) = cur.take() {
                        s.guilds.push(g);
                    }
                    cur = Some(Guild { name: String::new(), exp: 0, members: vec![], notice: String::new(), day: 0, quest_progress: 0, quest_paid: false, quest_claimed: vec![], daily_claimed: vec![] });
                    continue;
                }
                let (Some(g), Some((k, v))) = (cur.as_mut(), line.split_once('=')) else { continue };
                let list = |v: &str| v.split('\t').filter(|x| !x.is_empty()).map(String::from).collect::<Vec<_>>();
                match k {
                    "name" => g.name = v.to_string(),
                    "exp" => g.exp = v.parse().unwrap_or(0),
                    "member" => {
                        if let Some((n, r)) = v.split_once('\t') {
                            g.members.push((n.to_string(), r.parse().unwrap_or(RANK_MEMBER)));
                        }
                    }
                    "notice" => g.notice = v.to_string(),
                    "day" => g.day = v.parse().unwrap_or(0),
                    "quest" => g.quest_progress = v.parse().unwrap_or(0),
                    "quest_paid" => g.quest_paid = v == "1",
                    "quest_claimed" => g.quest_claimed = list(v),
                    "daily_claimed" => g.daily_claimed = list(v),
                    _ => {}
                }
            }
            if let Some(g) = cur.take() {
                s.guilds.push(g);
            }
            s.guilds.retain(|g| !g.name.is_empty() && !g.members.is_empty());
        }
        s
    }

    pub fn save(&self) -> std::io::Result<()> {
        let Some(dir) = &self.dir else { return Ok(()) };
        let mut t = String::from("# Galebound friends, friend requests and block lists\n");
        for (k, list) in [("friend", &self.friends), ("request", &self.requests), ("block", &self.blocks)] {
            for (a, b) in list {
                t.push_str(&format!("{}\t{}\t{}\n", k, a, b));
            }
        }
        write_atomic(&dir.join("social.txt"), &t)?;
        let mut g = String::from("# Galebound guilds\n");
        for x in &self.guilds {
            g.push_str("[guild]\n");
            g.push_str(&format!("name={}\nexp={}\nnotice={}\nday={}\nquest={}\nquest_paid={}\n", x.name, x.exp, x.notice.replace('\n', " "), x.day, x.quest_progress, x.quest_paid as u8));
            for (n, r) in &x.members {
                g.push_str(&format!("member={}\t{}\n", n, r));
            }
            g.push_str(&format!("quest_claimed={}\ndaily_claimed={}\n", x.quest_claimed.join("\t"), x.daily_claimed.join("\t")));
        }
        write_atomic(&dir.join("guilds.txt"), &g)
    }

    // ------------------------------------------------------------------ names

    pub fn remember(&mut self, name: &str, level: u32, class: &str) {
        self.known.insert(name.to_lowercase(), Known { name: name.to_string(), level, class: class.to_string() });
    }
    /// The real spelling of a lowercase character name, if we know it.
    pub fn display_name(&self, lower: &str) -> Option<String> {
        self.known.get(lower).map(|k| k.name.clone())
    }
    /// The real spelling of a character name, if it exists.
    pub fn canonical(&self, name: &str) -> Option<String> {
        self.known.get(&name.trim().to_lowercase()).map(|k| k.name.clone())
    }

    // ------------------------------------------------------------------ friends

    pub fn are_friends(&self, a: &str, b: &str) -> bool {
        self.friends.iter().any(|(x, y)| (eq(x, a) && eq(y, b)) || (eq(x, b) && eq(y, a)))
    }
    pub fn friends_of(&self, a: &str) -> Vec<String> {
        self.friends
            .iter()
            .filter_map(|(x, y)| if eq(x, a) { Some(y.clone()) } else if eq(y, a) { Some(x.clone()) } else { None })
            .collect()
    }
    /// Does `who` block `whom`?
    pub fn blocks(&self, who: &str, whom: &str) -> bool {
        self.blocks.iter().any(|(x, y)| eq(x, who) && eq(y, whom))
    }
    pub fn blocked_by(&self, who: &str) -> Vec<String> {
        self.blocks.iter().filter(|(x, _)| eq(x, who)).map(|(_, y)| y.clone()).collect()
    }
    pub fn incoming(&self, who: &str) -> Vec<String> {
        self.requests.iter().filter(|(_, t)| eq(t, who)).map(|(f, _)| f.clone()).collect()
    }
    pub fn outgoing(&self, who: &str) -> Vec<String> {
        self.requests.iter().filter(|(f, _)| eq(f, who)).map(|(_, t)| t.clone()).collect()
    }

    pub fn request_friend(&mut self, from: &str, to: &str) -> Result<Requested, String> {
        if eq(from, to) {
            return Err("You can't befriend yourself (but we appreciate the self-love).".into());
        }
        if self.are_friends(from, to) {
            return Err(format!("{} is already your friend.", to));
        }
        if self.blocks(to, from) {
            return Err(format!("{} isn't accepting friend requests.", to));
        }
        if self.friends_of(from).len() >= social::FRIENDS_MAX {
            return Err("Your friends list is full.".into());
        }
        if self.requests.iter().any(|(f, t)| eq(f, to) && eq(t, from)) {
            self.answer_friend(from, to, true)?;
            return Ok(Requested::Friends);
        }
        if !self.requests.iter().any(|(f, t)| eq(f, from) && eq(t, to)) {
            self.requests.push((from.to_string(), to.to_string()));
        }
        Ok(Requested::Sent)
    }

    /// `me` answers the request `from` sent.
    pub fn answer_friend(&mut self, me: &str, from: &str, accept: bool) -> Result<(), String> {
        let had = self.requests.iter().any(|(f, t)| eq(f, from) && eq(t, me));
        self.requests.retain(|(f, t)| !(eq(f, from) && eq(t, me)) && !(eq(f, me) && eq(t, from)));
        if !had {
            return Err("That request has expired.".into());
        }
        if accept && !self.are_friends(me, from) {
            self.friends.push((from.to_string(), me.to_string()));
        }
        Ok(())
    }

    pub fn remove_friend(&mut self, a: &str, b: &str) {
        self.friends.retain(|(x, y)| !((eq(x, a) && eq(y, b)) || (eq(x, b) && eq(y, a))));
        self.requests.retain(|(f, t)| !(eq(f, a) && eq(t, b)));
    }

    pub fn block(&mut self, who: &str, whom: &str) {
        if !self.blocks(who, whom) && !eq(who, whom) {
            self.blocks.push((who.to_string(), whom.to_string()));
        }
        self.remove_friend(who, whom);
        self.requests.retain(|(f, t)| !(eq(f, whom) && eq(t, who)));
    }
    pub fn unblock(&mut self, who: &str, whom: &str) {
        self.blocks.retain(|(x, y)| !(eq(x, who) && eq(y, whom)));
    }

    // ------------------------------------------------------------------ guilds

    pub fn guild_of(&self, name: &str) -> Option<usize> {
        self.guilds.iter().position(|g| g.has(name))
    }
    pub fn guild_named(&self, name: &str) -> Option<usize> {
        self.guilds.iter().position(|g| eq(&g.name, name))
    }

    pub fn create_guild(&mut self, name: &str, leader: &str, day: u32) -> Result<usize, String> {
        let name = social::validate_guild_name(name)?;
        if self.guild_of(leader).is_some() {
            return Err("Leave your guild first.".into());
        }
        if self.guild_named(&name).is_some() {
            return Err("A guild with that name already exists.".into());
        }
        self.guilds.push(Guild {
            name,
            exp: 0,
            members: vec![(leader.to_string(), RANK_LEADER)],
            notice: "Welcome to the guild!".into(),
            day,
            quest_progress: 0,
            quest_paid: false,
            quest_claimed: vec![],
            daily_claimed: vec![],
        });
        Ok(self.guilds.len() - 1)
    }

    pub fn join_guild(&mut self, g: usize, name: &str) -> Result<(), String> {
        if self.guild_of(name).is_some() {
            return Err("Already in a guild.".into());
        }
        let gd = self.guilds.get_mut(g).ok_or("That guild no longer exists.")?;
        if gd.members.len() >= GUILD_MAX {
            return Err("That guild is full.".into());
        }
        gd.members.push((name.to_string(), RANK_MEMBER));
        Ok(())
    }

    /// Someone leaves (or is removed). A leaving leader hands over to an
    /// officer, else the longest-standing member; the last one out closes it.
    /// Returns true if the guild was disbanded.
    pub fn leave_guild(&mut self, name: &str) -> bool {
        let Some(g) = self.guild_of(name) else { return false };
        let gd = &mut self.guilds[g];
        let was_leader = gd.rank(name) == Some(RANK_LEADER);
        gd.members.retain(|m| !eq(&m.0, name));
        if gd.members.is_empty() {
            self.guilds.remove(g);
            return true;
        }
        if was_leader {
            let next = gd.members.iter().position(|m| m.1 == RANK_OFFICER).unwrap_or(0);
            gd.members[next].1 = RANK_LEADER;
        }
        false
    }

    /// `by` changes `target`'s rank. Leaders can do anything (giving the
    /// leader rank away makes them an officer); officers can't touch leaders
    /// or other officers.
    pub fn set_rank(&mut self, by: &str, target: &str, rank: u8) -> Result<(), String> {
        let g = self.guild_of(by).ok_or("You're not in a guild.")?;
        let gd = &mut self.guilds[g];
        let my = gd.rank(by).unwrap_or(RANK_MEMBER);
        let theirs = gd.rank(target).ok_or(format!("{} isn't in your guild.", target))?;
        if my != RANK_LEADER || eq(by, target) || rank > RANK_LEADER {
            return Err("Only the guild leader can change ranks.".into());
        }
        let _ = theirs;
        for m in gd.members.iter_mut() {
            if eq(&m.0, target) {
                m.1 = rank;
            } else if rank == RANK_LEADER && eq(&m.0, by) {
                m.1 = RANK_OFFICER;
            }
        }
        Ok(())
    }

    pub fn can_manage(&self, by: &str) -> bool {
        self.guild_of(by).and_then(|g| self.guilds[g].rank(by)).map(|r| r >= RANK_OFFICER).unwrap_or(false)
    }

    /// Officers can remove members; the leader can remove anyone else.
    pub fn kick(&mut self, by: &str, target: &str) -> Result<(), String> {
        let g = self.guild_of(by).ok_or("You're not in a guild.")?;
        let my = self.guilds[g].rank(by).unwrap_or(RANK_MEMBER);
        let theirs = self.guilds[g].rank(target).ok_or(format!("{} isn't in your guild.", target))?;
        if my <= theirs || my < RANK_OFFICER {
            return Err("You can't remove them.".into());
        }
        self.leave_guild(target);
        Ok(())
    }

    /// A character was deleted: forget them everywhere.
    pub fn forget(&mut self, name: &str) {
        self.friends.retain(|(a, b)| !eq(a, name) && !eq(b, name));
        self.requests.retain(|(a, b)| !eq(a, name) && !eq(b, name));
        self.blocks.retain(|(a, b)| !eq(a, name) && !eq(b, name));
        self.leave_guild(name);
        self.known.remove(&name.to_lowercase());
    }
}

/// Write a file through a temporary copy, so a crash never leaves it half written.
fn write_atomic(path: &std::path::Path, text: &str) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn friends_requests_and_blocks() {
        let mut s = SocialStore::default();
        assert_eq!(s.request_friend("Ann", "Bo"), Ok(Requested::Sent));
        assert_eq!(s.incoming("bo"), vec!["Ann".to_string()]);
        s.answer_friend("Bo", "Ann", true).unwrap();
        assert!(s.are_friends("bo", "ANN"));
        assert!(s.request_friend("Ann", "Bo").is_err(), "already friends");
        // crossing requests become a friendship straight away
        s.request_friend("Cy", "Ann").unwrap();
        assert_eq!(s.request_friend("Ann", "Cy"), Ok(Requested::Friends));
        assert_eq!(s.friends_of("Ann").len(), 2);
        // blocking ends the friendship and stops requests
        s.block("Bo", "Ann");
        assert!(!s.are_friends("Ann", "Bo"));
        assert!(s.request_friend("Ann", "Bo").is_err());
        s.unblock("Bo", "Ann");
        assert!(s.request_friend("Ann", "Bo").is_ok());
    }

    #[test]
    fn guild_membership_ranks_and_files() {
        let dir = std::env::temp_dir().join(format!("gb_social_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut s = SocialStore::load(dir.clone());
        let g = s.create_guild("Second~Wind", "Ann", 100).unwrap();
        assert!(s.create_guild("second~wind", "Bo", 100).is_err(), "names are unique");
        s.join_guild(g, "Bo").unwrap();
        s.join_guild(g, "Cy").unwrap();
        assert!(s.kick("Bo", "Cy").is_err(), "members can't kick");
        s.set_rank("Ann", "Bo", RANK_OFFICER).unwrap();
        s.kick("Bo", "Cy").unwrap();
        assert!(s.kick("Bo", "Ann").is_err(), "officers can't kick the leader");
        s.guilds[g].exp = 700;
        s.friends.push(("Ann".into(), "Bo".into()));
        s.save().unwrap();
        let back = SocialStore::load(dir.clone());
        assert_eq!(back.guilds, s.guilds);
        assert_eq!(back.friends, s.friends);
        assert_eq!(back.guilds[0].level(), 2);
        // the leader leaves: the officer takes over; the last one out closes it
        s.leave_guild("Ann");
        assert_eq!(s.guilds[0].leader(), "Bo");
        assert!(s.leave_guild("Bo"));
        assert!(s.guilds.is_empty());
        std::fs::remove_dir_all(dir).ok();
    }
}
