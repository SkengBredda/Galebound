//! Friends, parties, guilds and trading: the rules both the server and the
//! game share, and the compact formats the server uses to send social data.
//!
//! The server owns every social relationship (who's friends with whom,
//! parties, guilds, trades in progress). Each game client shows them and
//! asks the server to change them.

use crate::stats::StatMods;

pub const PARTY_MAX: usize = 6;
pub const GUILD_MAX: usize = 30;
pub const FRIENDS_MAX: usize = 50;
/// Gold it costs to found a guild.
pub const GUILD_COST: u32 = 5000;
/// Guild ranks.
pub const RANK_MEMBER: u8 = 0;
pub const RANK_OFFICER: u8 = 1;
pub const RANK_LEADER: u8 = 2;

pub fn rank_name(r: u8) -> &'static str {
    match r {
        RANK_LEADER => "Leader",
        RANK_OFFICER => "Officer",
        _ => "Member",
    }
}

/// Guild names follow the character name rules, 3-16 characters.
pub fn validate_guild_name(name: &str) -> Result<String, String> {
    let n = name.trim();
    let len = n.chars().count();
    if !(3..=16).contains(&len) {
        return Err("Guild names are 3-16 characters.".into());
    }
    if !n.chars().all(|c| c.is_alphanumeric() || c == ' ' || c == '~' || c == '-') {
        return Err("Use letters, numbers, spaces, ~ and - only.".into());
    }
    if n.contains("  ") {
        return Err("No double spaces, please.".into());
    }
    Ok(n.to_string())
}

// ------------------------------------------------------------------ guild levels and buffs

/// Guild EXP needed to reach each level (index = level - 1).
pub const GUILD_LEVELS: [u32; 5] = [0, 600, 1800, 4000, 8000];
pub const GUILD_MAX_LEVEL: u32 = GUILD_LEVELS.len() as u32;

pub fn guild_level(exp: u32) -> u32 {
    GUILD_LEVELS.iter().filter(|&&t| exp >= t).count() as u32
}

/// EXP needed for the next guild level (None at the top).
pub fn guild_next(level: u32) -> Option<u32> {
    GUILD_LEVELS.get(level as usize).copied()
}

/// One guild-wide buff, unlocked at a guild level.
pub struct GuildBuff {
    pub level: u32,
    pub name: &'static str,
    pub desc: &'static str,
}

pub const GUILD_BUFFS: &[GuildBuff] = &[
    GuildBuff { level: 1, name: "Shared Wind", desc: "+3% EXP from monsters" },
    GuildBuff { level: 2, name: "Banner of Might", desc: "+4% attack" },
    GuildBuff { level: 3, name: "Iron Oath", desc: "+40 Max HP, +6% P.DEF and M.DEF" },
    GuildBuff { level: 4, name: "Tailwind", desc: "+4% move speed, +2% Crit Chance" },
    GuildBuff { level: 5, name: "Galebound Legend", desc: "+5% more EXP, +4% more attack" },
];

/// Stat bonuses every member gets from their guild's level.
pub fn guild_mods(level: u32) -> StatMods {
    let mut m = StatMods::default();
    if level >= 2 {
        m.atk_pct += 0.04;
    }
    if level >= 3 {
        m.hp += 40.0;
        m.pdef_pct += 0.06;
        m.mdef_pct += 0.06;
    }
    if level >= 4 {
        m.speed_pct += 0.04;
        m.crit += 2.0;
    }
    if level >= 5 {
        m.atk_pct += 0.04;
    }
    m
}

/// Extra EXP from monsters for guild members (fraction).
pub fn guild_exp_bonus(level: u32) -> f32 {
    match level {
        0 => 0.0,
        1..=4 => 0.03,
        _ => 0.08,
    }
}

// ------------------------------------------------------------------ guild quests and rewards

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuildGoal {
    /// Monsters defeated by any member, anywhere.
    Monsters,
    /// Quest bosses defeated by members.
    Bosses,
    /// Monsters defeated on one map.
    MonstersOn(&'static str),
}

pub struct GuildQuest {
    pub name: &'static str,
    pub desc: &'static str,
    pub goal: GuildGoal,
    pub target: u32,
    /// Guild EXP when the guild completes it.
    pub guild_exp: u32,
}

/// The daily guild quests, one per day in rotation.
pub const GUILD_QUESTS: &[GuildQuest] = &[
    GuildQuest { name: "Clear the Roads", desc: "Defeat 120 monsters together.", goal: GuildGoal::Monsters, target: 120, guild_exp: 300 },
    GuildQuest { name: "Glade Patrol", desc: "Defeat 50 monsters in Dawnwind Glade.", goal: GuildGoal::MonstersOn("dawnwind_glade"), target: 50, guild_exp: 220 },
    GuildQuest { name: "Giant Hunt", desc: "Defeat 2 quest bosses.", goal: GuildGoal::Bosses, target: 2, guild_exp: 350 },
    GuildQuest { name: "Fields of Wool", desc: "Defeat 60 monsters in Whistling Fields.", goal: GuildGoal::MonstersOn("whistling_fields"), target: 60, guild_exp: 260 },
    GuildQuest { name: "Lights Out", desc: "Defeat 60 monsters in Thornvale Woods.", goal: GuildGoal::MonstersOn("thornvale_woods"), target: 60, guild_exp: 300 },
    GuildQuest { name: "Storm Watch", desc: "Defeat 60 monsters on Windcrag Cliffs.", goal: GuildGoal::MonstersOn("windcrag_cliffs"), target: 60, guild_exp: 340 },
    GuildQuest { name: "Great Culling", desc: "Defeat 250 monsters together.", goal: GuildGoal::Monsters, target: 250, guild_exp: 500 },
];

/// Today's guild quest (`day` = days since 1970, server time).
pub fn guild_quest(day: u32) -> (usize, &'static GuildQuest) {
    let i = day as usize % GUILD_QUESTS.len();
    (i, &GUILD_QUESTS[i])
}

/// What one member gets for claiming the finished guild quest.
pub fn guild_quest_reward(level: u32) -> (u32, &'static [(&'static str, u32)]) {
    (300 + 150 * level, &[("elixir", 2), ("revive_scroll", 1)])
}

/// What one member gets from the guild's daily reward chest.
pub fn guild_daily_reward(level: u32) -> (u32, &'static [(&'static str, u32)]) {
    let items: &'static [(&'static str, u32)] = if level >= 3 { &[("hp_potion_big", 3), ("mp_potion_big", 3)] } else { &[("hp_potion", 3), ("mp_potion", 3)] };
    (100 + 100 * level, items)
}

// ------------------------------------------------------------------ payloads

// Lists travel in one message field: records split by ';', values by ','.
// Names, map keys and item ids never contain those characters.

fn clean(s: &str) -> String {
    s.replace([',', ';', '|'], " ")
}

fn split_records(s: &str) -> impl Iterator<Item = Vec<&str>> {
    s.split(';').filter(|r| !r.is_empty()).map(|r| r.split(',').collect())
}

/// An item list "id:n,id:n" (for trades and rewards).
pub fn encode_items(items: &[(String, u32)]) -> String {
    items.iter().map(|(id, n)| format!("{}:{}", clean(id), n)).collect::<Vec<_>>().join(",")
}

pub fn decode_items(s: &str) -> Vec<(String, u32)> {
    s.split(',')
        .filter_map(|p| {
            let (id, n) = p.split_once(':')?;
            let n: u32 = n.parse().ok()?;
            (n > 0 && !id.is_empty()).then(|| (id.to_string(), n))
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct FriendNet {
    pub name: String,
    pub online: bool,
    pub level: u32,
    pub class: String,
    pub map: String,
}

/// Your friends, friend requests (sent to you / by you) and block list.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct SocialNet {
    pub friends: Vec<FriendNet>,
    pub incoming: Vec<String>,
    pub outgoing: Vec<String>,
    pub blocked: Vec<String>,
}

impl SocialNet {
    pub fn encode(&self) -> String {
        let f: Vec<String> = self.friends.iter().map(|f| format!("{},{},{},{},{}", clean(&f.name), f.online as u8, f.level, clean(&f.class), clean(&f.map))).collect();
        format!("{}|{}|{}|{}", f.join(";"), self.incoming.join(";"), self.outgoing.join(";"), self.blocked.join(";"))
    }
    pub fn decode(s: &str) -> Option<SocialNet> {
        let parts: Vec<&str> = s.split('|').collect();
        if parts.len() != 4 {
            return None;
        }
        let mut out = SocialNet::default();
        for r in split_records(parts[0]) {
            if r.len() == 5 {
                out.friends.push(FriendNet { name: r[0].into(), online: r[1] == "1", level: r[2].parse().unwrap_or(1), class: r[3].into(), map: r[4].into() });
            }
        }
        let names = |p: &str| p.split(';').filter(|n| !n.is_empty()).map(String::from).collect::<Vec<_>>();
        out.incoming = names(parts[1]);
        out.outgoing = names(parts[2]);
        out.blocked = names(parts[3]);
        Some(out)
    }
    pub fn is_friend(&self, name: &str) -> bool {
        self.friends.iter().any(|f| f.name.eq_ignore_ascii_case(name))
    }
    pub fn is_blocked(&self, name: &str) -> bool {
        self.blocked.iter().any(|b| b.eq_ignore_ascii_case(name))
    }
}

/// A party or guild member.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MemberNet {
    /// Player id while they're in the world, 0 when offline.
    pub id: u32,
    pub name: String,
    pub level: u32,
    pub class: String,
    pub map: String,
    /// HP fraction (parties).
    pub hp: f32,
    /// Guild rank.
    pub rank: u8,
}

impl MemberNet {
    pub fn online(&self) -> bool {
        self.id != 0
    }
}

fn enc_members(m: &[MemberNet]) -> String {
    m.iter()
        .map(|m| format!("{},{},{},{},{},{:.2},{}", m.id, clean(&m.name), m.level, clean(&m.class), clean(&m.map), m.hp, m.rank))
        .collect::<Vec<_>>()
        .join(";")
}

fn dec_members(s: &str) -> Vec<MemberNet> {
    split_records(s)
        .filter(|r| r.len() == 7)
        .map(|r| MemberNet {
            id: r[0].parse().unwrap_or(0),
            name: r[1].into(),
            level: r[2].parse().unwrap_or(1),
            class: r[3].into(),
            map: r[4].into(),
            hp: r[5].parse().unwrap_or(1.0),
            rank: r[6].parse().unwrap_or(0),
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct PartyNet {
    pub leader: String,
    pub members: Vec<MemberNet>,
}

impl PartyNet {
    pub fn encode(&self) -> String {
        format!("{}|{}", clean(&self.leader), enc_members(&self.members))
    }
    pub fn decode(s: &str) -> Option<PartyNet> {
        let (l, m) = s.split_once('|')?;
        Some(PartyNet { leader: l.into(), members: dec_members(m) })
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct GuildNet {
    pub name: String,
    pub exp: u32,
    pub members: Vec<MemberNet>,
    /// Today's guild quest (index into GUILD_QUESTS) and how it's going.
    pub quest: u32,
    pub quest_progress: u32,
    /// You've already claimed today's quest reward / daily chest.
    pub quest_claimed: bool,
    pub daily_claimed: bool,
    pub notice: String,
}

impl GuildNet {
    pub fn level(&self) -> u32 {
        guild_level(self.exp)
    }
    pub fn quest_def(&self) -> &'static GuildQuest {
        &GUILD_QUESTS[self.quest as usize % GUILD_QUESTS.len()]
    }
    pub fn quest_done(&self) -> bool {
        self.quest_progress >= self.quest_def().target
    }
    pub fn rank_of(&self, name: &str) -> u8 {
        self.members.iter().find(|m| m.name.eq_ignore_ascii_case(name)).map(|m| m.rank).unwrap_or(RANK_MEMBER)
    }
    pub fn encode(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}|{}|{}",
            clean(&self.name),
            self.exp,
            enc_members(&self.members),
            self.quest,
            self.quest_progress,
            self.quest_claimed as u8,
            self.daily_claimed as u8,
            clean(&self.notice)
        )
    }
    pub fn decode(s: &str) -> Option<GuildNet> {
        let p: Vec<&str> = s.split('|').collect();
        if p.len() != 8 {
            return None;
        }
        Some(GuildNet {
            name: p[0].into(),
            exp: p[1].parse().ok()?,
            members: dec_members(p[2]),
            quest: p[3].parse().ok()?,
            quest_progress: p[4].parse().ok()?,
            quest_claimed: p[5] == "1",
            daily_claimed: p[6] == "1",
            notice: p[7].into(),
        })
    }
}

/// What the profile window shows about another player.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ProfileNet {
    pub name: String,
    pub class: String,
    pub level: u32,
    /// Progress to the next level, 0..1.
    pub exp_frac: f32,
    pub play_seconds: u64,
    pub score: u32,
    pub achievements: u32,
    pub guild: String,
    /// Equipped item ids, one per equipment slot ("" = empty).
    pub gear: Vec<String>,
    /// `Appearance::encode()` with gear applied.
    pub look: String,
}

impl ProfileNet {
    pub fn encode(&self) -> String {
        format!(
            "{}|{}|{}|{:.4}|{}|{}|{}|{}|{}|{}",
            clean(&self.name),
            clean(&self.class),
            self.level,
            self.exp_frac,
            self.play_seconds,
            self.score,
            self.achievements,
            clean(&self.guild),
            self.gear.iter().map(|g| clean(g)).collect::<Vec<_>>().join(";"),
            self.look
        )
    }
    pub fn decode(s: &str) -> Option<ProfileNet> {
        let p: Vec<&str> = s.split('|').collect();
        if p.len() != 10 {
            return None;
        }
        Some(ProfileNet {
            name: p[0].into(),
            class: p[1].into(),
            level: p[2].parse().ok()?,
            exp_frac: p[3].parse().ok()?,
            play_seconds: p[4].parse().ok()?,
            score: p[5].parse().ok()?,
            achievements: p[6].parse().ok()?,
            guild: p[7].into(),
            gear: p[8].split(';').map(String::from).collect(),
            look: p[9].into(),
        })
    }
}

/// A trade in progress, as one side sees it.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TradeNet {
    pub with: String,
    pub mine: Vec<(String, u32)>,
    pub my_gold: u32,
    pub theirs: Vec<(String, u32)>,
    pub their_gold: u32,
    pub my_lock: bool,
    pub their_lock: bool,
    pub my_ok: bool,
    pub their_ok: bool,
}

impl TradeNet {
    pub fn encode(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}{}{}{}",
            clean(&self.with),
            encode_items(&self.mine),
            self.my_gold,
            encode_items(&self.theirs),
            self.their_gold,
            self.my_lock as u8,
            self.their_lock as u8,
            self.my_ok as u8,
            self.their_ok as u8
        )
    }
    pub fn decode(s: &str) -> Option<TradeNet> {
        let p: Vec<&str> = s.split('|').collect();
        if p.len() != 6 || p[5].len() != 4 {
            return None;
        }
        let f: Vec<bool> = p[5].chars().map(|c| c == '1').collect();
        Some(TradeNet {
            with: p[0].into(),
            mine: decode_items(p[1]),
            my_gold: p[2].parse().ok()?,
            theirs: decode_items(p[3]),
            their_gold: p[4].parse().ok()?,
            my_lock: f[0],
            their_lock: f[1],
            my_ok: f[2],
            their_ok: f[3],
        })
    }
}

/// Most stacks one side can put into a trade.
pub const TRADE_SLOTS: usize = 8;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payloads_roundtrip() {
        let s = SocialNet {
            friends: vec![FriendNet { name: "Kaito".into(), online: true, level: 12, class: "vanguard".into(), map: "breezehollow".into() }, FriendNet { name: "Mia Rose".into(), ..Default::default() }],
            incoming: vec!["Bob".into()],
            outgoing: vec![],
            blocked: vec!["Troll".into(), "Spam Bot".into()],
        };
        assert_eq!(SocialNet::decode(&s.encode()), Some(s.clone()));
        let m = vec![MemberNet { id: 4, name: "Skeng".into(), level: 20, class: "duskblade".into(), map: "thornvale_woods".into(), hp: 0.5, rank: RANK_LEADER }, MemberNet { name: "Off Line".into(), ..Default::default() }];
        let p = PartyNet { leader: "Skeng".into(), members: m.clone() };
        assert_eq!(PartyNet::decode(&p.encode()), Some(p));
        let g = GuildNet { name: "Second~Wind".into(), exp: 700, members: m, quest: 3, quest_progress: 12, quest_claimed: false, daily_claimed: true, notice: "Raid at 8, bring potions; and, food".into() };
        let back = GuildNet::decode(&g.encode()).unwrap();
        assert_eq!(back.name, g.name);
        assert_eq!(back.members, g.members);
        assert_eq!(back.level(), 2);
        assert_eq!(back.notice, "Raid at 8  bring potions  and  food");
        let pr = ProfileNet { name: "Skeng".into(), class: "novice".into(), level: 9, exp_frac: 0.5, play_seconds: 3600, score: 45, achievements: 6, guild: String::new(), gear: vec!["gnarled_stick".into(), "".into(), "novice_garb".into()], look: "0,1,2,3,4,5,6,7,0,1".into() };
        assert_eq!(ProfileNet::decode(&pr.encode()), Some(pr));
        let t = TradeNet { with: "Kaito".into(), mine: vec![("elixir".into(), 2)], my_gold: 50, theirs: vec![], their_gold: 0, my_lock: true, their_lock: false, my_ok: false, their_ok: false };
        assert_eq!(TradeNet::decode(&t.encode()), Some(t));
    }

    #[test]
    fn guild_rules() {
        assert_eq!(guild_level(0), 1);
        assert_eq!(guild_level(599), 1);
        assert_eq!(guild_level(600), 2);
        assert_eq!(guild_level(100_000), GUILD_MAX_LEVEL);
        assert!(guild_mods(5).atk_pct > guild_mods(2).atk_pct);
        assert!(validate_guild_name("Second~Wind").is_ok());
        assert!(validate_guild_name("ab").is_err());
        assert!(validate_guild_name("bad;name").is_err());
        for d in 0..14 {
            let (i, q) = guild_quest(d);
            assert!(q.target > 0 && i < GUILD_QUESTS.len());
            if let GuildGoal::MonstersOn(m) = q.goal {
                assert!(crate::world::load_map(m).is_some(), "{}", m);
            }
        }
    }
}
