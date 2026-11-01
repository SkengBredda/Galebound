//! Social windows: Friends & Party (U), Guild (G), Achievements with
//! Cosmetics and Attendance (H), player profiles, trading, invitation
//! popups and the right-click player menu.

use crate::gfx::{self, alpha, rgb, rgba};
use crate::icons;
use crate::sprites;
use crate::widgets::{self, ButtonStyle, TextField, Ui};
use galebound_core::achievements::{self, ACHIEVEMENTS, ATTENDANCE, COSMETICS};
use galebound_core::appearance::Appearance;
use galebound_core::classes::ClassId;
use galebound_core::items::{self, EQUIP_SLOTS};
use galebound_core::net::{SocialCmd, INV_FRIEND, INV_GUILD, INV_PARTY, INV_TRADE};
use galebound_core::social::{self, MemberNet, GUILD_BUFFS, RANK_LEADER, RANK_MEMBER, RANK_OFFICER};
use galebound_core::sim::Sim;
use macroquad::prelude::{is_key_down, is_mouse_button_pressed, mouse_wheel, Color, KeyCode, MouseButton};

const WHITE: Color = Color::new(1.0, 1.0, 1.0, 1.0);

#[derive(Clone, Copy, PartialEq, Debug)]
enum Field {
    Friend,
    Party,
    Block,
    GuildName,
    GuildInvite,
    Notice,
    TradeGold,
}

/// Something you can do to another player, from the right-click menu.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Act {
    Whisper,
    Profile,
    Party,
    Trade,
    GuildInvite,
    AddFriend,
    RemoveFriend,
    Block,
    Unblock,
    PartyPromote,
    PartyKick,
    MakeOfficer,
    MakeMember,
    MakeLeader,
    GuildKick,
}

impl Act {
    fn label(self) -> &'static str {
        match self {
            Act::Whisper => "Whisper",
            Act::Profile => "View profile",
            Act::Party => "Invite to party",
            Act::Trade => "Trade",
            Act::GuildInvite => "Invite to guild",
            Act::AddFriend => "Add friend",
            Act::RemoveFriend => "Remove friend",
            Act::Block => "Block",
            Act::Unblock => "Unblock",
            Act::PartyPromote => "Make party leader",
            Act::PartyKick => "Remove from party",
            Act::MakeOfficer => "Make officer",
            Act::MakeMember => "Make member",
            Act::MakeLeader => "Hand over leadership",
            Act::GuildKick => "Remove from guild",
        }
    }
}

struct Menu {
    name: String,
    x: f32,
    y: f32,
    acts: Vec<Act>,
}

pub struct SocialUi {
    pub social: bool,
    pub guild: bool,
    pub achievements: bool,
    pub profile: bool,
    social_tab: usize,
    ach_tab: usize,
    ach_scroll: f32,
    member_scroll: f32,
    friend: TextField,
    party_name: TextField,
    block_name: TextField,
    guild_name: TextField,
    guild_invite: TextField,
    notice: TextField,
    trade_gold: TextField,
    focus: Option<Field>,
    menu: Option<Menu>,
    msg: Option<(String, f32)>,
    /// What we've put into the current trade (sent to the server on change).
    offer: Vec<(String, u32)>,
    offer_gold: u32,
    trade_with: String,
    attendance_prompt: bool,
    /// Ask for a chat line to be started (e.g. "/w Name ").
    pub start_chat: Option<String>,
}

impl Default for SocialUi {
    fn default() -> Self {
        SocialUi {
            social: false,
            guild: false,
            achievements: false,
            profile: false,
            social_tab: 0,
            ach_tab: 0,
            ach_scroll: 0.0,
            member_scroll: 0.0,
            friend: TextField::new("Character name", false, 14),
            party_name: TextField::new("Character name", false, 14),
            block_name: TextField::new("Character name", false, 14),
            guild_name: TextField::new("Guild name", false, 16),
            guild_invite: TextField::new("Character name", false, 14),
            notice: TextField::new("Guild notice", false, 80),
            trade_gold: TextField::new("0", false, 9),
            focus: None,
            menu: None,
            msg: None,
            offer: Vec::new(),
            offer_gold: 0,
            trade_with: String::new(),
            attendance_prompt: true,
            start_chat: None,
        }
    }
}

fn item_tex(def: &items::ItemDef) -> macroquad::prelude::Texture2D {
    sprites::get(&format!("icon_i_{}", def.id), || icons::item(def))
}

fn px_color(p: crate::pixel::Px) -> Color {
    rgba(p[0], p[1], p[2], p[3])
}

fn class_name(key: &str) -> &'static str {
    ClassId::from_key(key).map(|c| c.info().name).unwrap_or("Novice")
}

fn map_name(key: &str) -> String {
    galebound_core::world::load_map(key).map(|m| m.name.to_string()).unwrap_or_default()
}

fn tab(ui: &Ui, x: f32, y: f32, w: f32, label: &str, on: bool) -> bool {
    let style = if on { ButtonStyle::Primary } else { ButtonStyle::Secondary };
    widgets::button(ui, x, y, w, 30.0, label, style, true)
}

fn small(ui: &Ui, x: f32, y: f32, w: f32, label: &str, style: ButtonStyle, enabled: bool) -> bool {
    widgets::button(ui, x, y, w, 26.0, label, style, enabled)
}

fn progress_bar(x: f32, y: f32, w: f32, h: f32, frac: f32, c: Color) {
    gfx::rrect(x, y, w, h, h * 0.5, rgba(60, 50, 40, 70));
    if frac > 0.0 {
        gfx::rrect(x, y, (w * frac.clamp(0.0, 1.0)).max(h), h, h * 0.5, c);
    }
}

fn time_played(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    if h > 0 {
        format!("{}h {}m", h, m)
    } else {
        format!("{}m", m)
    }
}

impl SocialUi {
    /// Screenshot / debug helpers.
    #[allow(dead_code)]
    pub fn show(&mut self, social_tab: usize, ach_tab: usize) {
        self.social_tab = social_tab;
        self.ach_tab = ach_tab;
    }
    #[allow(dead_code)]
    pub fn menu_for(&mut self, sim: &Sim, name: &str, x: f32, y: f32) {
        self.open_menu(sim, name, x, y);
    }
    #[allow(dead_code)]
    pub fn hide_attendance(&mut self) {
        self.attendance_prompt = false;
    }

    /// Is a text box focused (so keys type instead of playing)?
    pub fn typing(&self) -> bool {
        self.focus.is_some()
    }

    pub fn any_open(&self, sim: &Sim) -> bool {
        self.social || self.guild || self.achievements || (self.profile && sim.profile.is_some()) || sim.trade.is_some() || self.menu.is_some()
    }

    /// Esc closes the top-most social window.
    pub fn close_one(&mut self, sim: &mut Sim) -> bool {
        self.focus = None;
        if self.menu.take().is_some() {
            return true;
        }
        if sim.trade.is_some() {
            sim.social_cmd(SocialCmd::TradeCancel);
            sim.trade = None;
            return true;
        }
        if self.profile && sim.profile.is_some() {
            self.profile = false;
            sim.profile = None;
            return true;
        }
        for w in [&mut self.achievements, &mut self.guild, &mut self.social] {
            if *w {
                *w = false;
                return true;
            }
        }
        false
    }

    pub fn toggle_social(&mut self, sim: &mut Sim) {
        self.social = !self.social;
        if self.social {
            sim.social_cmd(SocialCmd::Refresh);
        }
        self.focus = None;
    }
    pub fn toggle_guild(&mut self, sim: &mut Sim) {
        self.guild = !self.guild;
        if self.guild {
            sim.social_cmd(SocialCmd::Refresh);
        }
        self.focus = None;
    }
    pub fn toggle_achievements(&mut self) {
        self.achievements = !self.achievements;
        self.focus = None;
    }

    fn say(&mut self, s: impl Into<String>) {
        self.msg = Some((s.into(), 3.0));
    }

    /// Right-click in the world: open the menu for the player under the mouse.
    pub fn right_click(&mut self, sim: &Sim, cam_x: f32, cam_y: f32) -> bool {
        if !is_mouse_button_pressed(MouseButton::Right) {
            return false;
        }
        let (wx, wy) = gfx::mouse_world(cam_x, cam_y);
        let hit = sim.others.iter().find(|o| (wx - o.pos.x).abs() < 22.0 && wy < o.pos.y + 6.0 && wy > o.pos.y - 80.0 && o.since < 10.0);
        if let Some(o) = hit {
            let (mx, my) = gfx::mouse();
            self.open_menu(sim, &o.name.clone(), mx, my);
            return true;
        }
        false
    }

    fn open_menu(&mut self, sim: &Sim, name: &str, x: f32, y: f32) {
        let me = &sim.save.name;
        if name.eq_ignore_ascii_case(me) {
            return;
        }
        let online = sim.others.iter().any(|o| o.name.eq_ignore_ascii_case(name))
            || sim.social.friends.iter().any(|f| f.online && f.name.eq_ignore_ascii_case(name))
            || sim.party.as_ref().map(|p| p.members.iter().any(|m| m.online() && m.name.eq_ignore_ascii_case(name))).unwrap_or(false)
            || sim.guild.as_ref().map(|g| g.members.iter().any(|m| m.online() && m.name.eq_ignore_ascii_case(name))).unwrap_or(false);
        let nearby = sim.others.iter().any(|o| o.name.eq_ignore_ascii_case(name));
        let mut acts = Vec::new();
        if online {
            acts.push(Act::Whisper);
            acts.push(Act::Profile);
        }
        let my_party = sim.party.as_ref();
        let in_my_party = my_party.map(|p| p.members.iter().any(|m| m.name.eq_ignore_ascii_case(name))).unwrap_or(false);
        let i_lead = my_party.map(|p| p.leader.eq_ignore_ascii_case(me)).unwrap_or(true);
        if online && !in_my_party && i_lead {
            acts.push(Act::Party);
        }
        if in_my_party && my_party.map(|p| p.leader.eq_ignore_ascii_case(me)).unwrap_or(false) {
            acts.push(Act::PartyPromote);
            acts.push(Act::PartyKick);
        }
        if nearby {
            acts.push(Act::Trade);
        }
        if let Some(g) = &sim.guild {
            let my_rank = g.rank_of(me);
            let member = g.members.iter().find(|m| m.name.eq_ignore_ascii_case(name));
            match member {
                None if online && my_rank >= RANK_OFFICER => acts.push(Act::GuildInvite),
                Some(m) => {
                    if my_rank == RANK_LEADER {
                        acts.push(if m.rank == RANK_OFFICER { Act::MakeMember } else { Act::MakeOfficer });
                        acts.push(Act::MakeLeader);
                    }
                    if my_rank > m.rank && my_rank >= RANK_OFFICER {
                        acts.push(Act::GuildKick);
                    }
                }
                _ => {}
            }
        }
        if sim.social.is_friend(name) {
            acts.push(Act::RemoveFriend);
        } else {
            acts.push(Act::AddFriend);
        }
        acts.push(if sim.social.is_blocked(name) { Act::Unblock } else { Act::Block });
        self.menu = Some(Menu { name: name.to_string(), x, y, acts });
    }

    fn act(&mut self, sim: &mut Sim, name: &str, a: Act) {
        let n = name.to_string();
        match a {
            Act::Whisper => self.start_chat = Some(format!("/w {} ", n)),
            Act::Profile => {
                sim.social_cmd(SocialCmd::Profile { name: n });
                self.profile = true;
            }
            Act::Party => sim.social_cmd(SocialCmd::PartyInvite { name: n }),
            Act::Trade => sim.social_cmd(SocialCmd::TradeRequest { name: n }),
            Act::GuildInvite => sim.social_cmd(SocialCmd::GuildInvite { name: n }),
            Act::AddFriend => sim.social_cmd(SocialCmd::FriendAdd { name: n }),
            Act::RemoveFriend => sim.social_cmd(SocialCmd::FriendRemove { name: n }),
            Act::Block => sim.social_cmd(SocialCmd::Block { name: n }),
            Act::Unblock => sim.social_cmd(SocialCmd::Unblock { name: n }),
            Act::PartyPromote => sim.social_cmd(SocialCmd::PartyPromote { name: n }),
            Act::PartyKick => sim.social_cmd(SocialCmd::PartyKick { name: n }),
            Act::MakeOfficer => sim.social_cmd(SocialCmd::GuildRank { name: n, rank: RANK_OFFICER }),
            Act::MakeMember => sim.social_cmd(SocialCmd::GuildRank { name: n, rank: RANK_MEMBER }),
            Act::MakeLeader => sim.social_cmd(SocialCmd::GuildRank { name: n, rank: RANK_LEADER }),
            Act::GuildKick => sim.social_cmd(SocialCmd::GuildKick { name: n }),
        }
    }

    /// Draw and run everything. `ui` is this frame's input.
    pub fn frame(&mut self, sim: &mut Sim, ui: &Ui, dt: f32) {
        gfx::clear_cam();
        if let Some(m) = &mut self.msg {
            m.1 -= dt;
        }
        if self.msg.as_ref().map(|m| m.1 <= 0.0).unwrap_or(false) {
            self.msg = None;
        }
        // a click outside every text box drops focus
        if ui.clicked {
            self.focus = None;
        }
        // the context menu is modal: it gets clicks first
        let menu_ui = Ui { chars: Vec::new(), clicked: ui.clicked, enabled: ui.enabled };
        let blocked = self.menu.is_some();
        let ui_main = Ui { chars: ui.chars.clone(), clicked: ui.clicked && !blocked, enabled: ui.enabled && !blocked };
        if self.social {
            self.social_window(sim, &ui_main);
        }
        if self.guild {
            self.guild_window(sim, &ui_main);
        }
        if self.achievements {
            self.achievements_window(sim, &ui_main);
        }
        if self.profile {
            self.profile_window(sim, &ui_main);
        }
        self.trade_window(sim, &ui_main);
        self.invites(sim, &ui_main);
        self.attendance_banner(sim, &ui_main);
        if let Some(m) = self.menu.take() {
            let w = 200.0;
            let h = 34.0 + m.acts.len() as f32 * 30.0;
            let x = m.x.min(gfx::VW - w - 8.0);
            let y = m.y.min(gfx::VH - h - 8.0);
            gfx::rrect(x, y, w, h, 8.0, rgba(24, 22, 40, 240));
            gfx::rect_lines(x + 1.0, y + 1.0, w - 2.0, h - 2.0, 1.0, rgba(120, 220, 255, 140));
            gfx::text(&m.name, x + 12.0, y + 8.0, 16.0, rgb(255, 236, 170));
            let mut chosen = None;
            for (k, a) in m.acts.iter().enumerate() {
                let ry = y + 32.0 + k as f32 * 30.0;
                let hover = gfx::mouse_in(x, ry, w, 28.0);
                if hover {
                    gfx::rect(x + 2.0, ry, w - 4.0, 28.0, rgba(120, 220, 255, 50));
                }
                let col = if matches!(a, Act::Block | Act::GuildKick | Act::PartyKick | Act::RemoveFriend) { rgb(255, 160, 150) } else { rgb(236, 240, 255) };
                gfx::text(a.label(), x + 14.0, ry + 6.0, 15.0, col);
                if hover && menu_ui.clicked {
                    chosen = Some(*a);
                }
            }
            let inside = gfx::mouse_in(x, y, w, h);
            match chosen {
                Some(a) => self.act(sim, &m.name, a),
                None if menu_ui.clicked && !inside => {}
                None => self.menu = Some(m),
            }
        }
        if let Some((t, _)) = &self.msg {
            gfx::text_centered_outlined(t, gfx::VW * 0.5, 84.0, 16.0, rgb(255, 230, 150));
        }
    }

    // ------------------------------------------------------------------ fields

    fn field(&mut self, ui: &Ui, f: Field, x: f32, y: f32, w: f32, time: f64) -> bool {
        let focused = self.focus == Some(f);
        let tf = match f {
            Field::Friend => &mut self.friend,
            Field::Party => &mut self.party_name,
            Field::Block => &mut self.block_name,
            Field::GuildName => &mut self.guild_name,
            Field::GuildInvite => &mut self.guild_invite,
            Field::Notice => &mut self.notice,
            Field::TradeGold => &mut self.trade_gold,
        };
        if focused {
            tf.type_into(ui);
            if f == Field::TradeGold {
                tf.text.retain(|c| c.is_ascii_digit());
            }
        }
        if tf.draw(ui, x, y, w, 32.0, "", focused, time) {
            self.focus = Some(f);
        }
        focused && (macroquad::prelude::is_key_pressed(KeyCode::Enter) || macroquad::prelude::is_key_pressed(KeyCode::KpEnter))
    }

    fn take(&mut self, f: Field) -> String {
        let tf = match f {
            Field::Friend => &mut self.friend,
            Field::Party => &mut self.party_name,
            Field::Block => &mut self.block_name,
            Field::GuildName => &mut self.guild_name,
            Field::GuildInvite => &mut self.guild_invite,
            Field::Notice => &mut self.notice,
            Field::TradeGold => &mut self.trade_gold,
        };
        self.focus = None;
        std::mem::take(&mut tf.text).trim().to_string()
    }

    // ------------------------------------------------------------------ friends & party

    fn social_window(&mut self, sim: &mut Sim, ui: &Ui) {
        let (x, y, w, h) = (20.0, 70.0, 430.0, 540.0);
        widgets::panel(x, y, w, h);
        widgets::title("Friends & Party", x + w * 0.5, y + 14.0, 22.0);
        if widgets::button(ui, x + w - 52.0, y + 12.0, 34.0, 30.0, "X", ButtonStyle::Secondary, true) {
            self.social = false;
        }
        let tabs = ["Friends", "Party", "Blocked"];
        for (k, t) in tabs.iter().enumerate() {
            if tab(ui, x + 22.0 + k as f32 * 130.0, y + 50.0, 124.0, t, self.social_tab == k) {
                self.social_tab = k;
                self.focus = None;
            }
        }
        let time = sim.time as f64;
        let mut yy = y + 98.0;
        match self.social_tab {
            0 => {
                gfx::text("Add a friend", x + 24.0, yy, 14.0, widgets::ink_soft());
                yy += 22.0;
                let enter = self.field(ui, Field::Friend, x + 24.0, yy, 250.0, time);
                if (small(ui, x + 288.0, yy + 3.0, 116.0, "Send request", ButtonStyle::Primary, !self.friend.text.trim().is_empty()) || enter) && !self.friend.text.trim().is_empty() {
                    let n = self.take(Field::Friend);
                    sim.social_cmd(SocialCmd::FriendAdd { name: n });
                }
                yy += 46.0;
                for from in sim.social.incoming.clone() {
                    gfx::rrect(x + 20.0, yy - 4.0, w - 40.0, 34.0, 6.0, rgba(255, 220, 120, 70));
                    gfx::text(&format!("{} wants to be friends", from), x + 30.0, yy + 4.0, 15.0, widgets::ink());
                    if small(ui, x + w - 170.0, yy, 70.0, "Accept", ButtonStyle::Primary, true) {
                        sim.social_cmd(SocialCmd::FriendAnswer { name: from.clone(), accept: true });
                    }
                    if small(ui, x + w - 94.0, yy, 70.0, "No", ButtonStyle::Secondary, true) {
                        sim.social_cmd(SocialCmd::FriendAnswer { name: from.clone(), accept: false });
                    }
                    yy += 40.0;
                }
                let online = sim.social.friends.iter().filter(|f| f.online).count();
                gfx::text(&format!("Friends ({} online / {})", online, sim.social.friends.len()), x + 24.0, yy, 14.0, widgets::ink_soft());
                yy += 22.0;
                if sim.social.friends.is_empty() {
                    gfx::text_wrapped("No friends yet. Add someone by name, or right-click a player in the world.", x + 24.0, yy, w - 48.0, 14.0, widgets::ink_soft());
                }
                for f in sim.social.friends.clone() {
                    if yy > y + h - 60.0 {
                        break;
                    }
                    let hover = ui.enabled && gfx::mouse_in(x + 20.0, yy - 2.0, w - 40.0, 34.0);
                    if hover {
                        gfx::rrect(x + 20.0, yy - 2.0, w - 40.0, 34.0, 6.0, rgba(120, 180, 255, 50));
                    }
                    gfx::circle(x + 34.0, yy + 12.0, 5.0, if f.online { rgb(80, 200, 100) } else { rgb(150, 144, 136) });
                    gfx::text(&f.name, x + 48.0, yy + 3.0, 16.0, if f.online { widgets::ink() } else { widgets::ink_soft() });
                    gfx::text(&format!("Lv.{} {}", f.level, class_name(&f.class)), x + 190.0, yy + 5.0, 13.0, widgets::ink_soft());
                    let place = if f.online { map_name(&f.map) } else { "Offline".to_string() };
                    gfx::text(&place, x + 300.0, yy + 5.0, 12.0, if f.online { widgets::accent() } else { widgets::ink_soft() });
                    if hover && ui.clicked {
                        let (mx, my) = gfx::mouse();
                        self.open_menu(sim, &f.name, mx, my);
                    }
                    yy += 36.0;
                }
                for to in sim.social.outgoing.clone() {
                    if yy > y + h - 50.0 {
                        break;
                    }
                    gfx::text(&format!("Waiting for {} to accept...", to), x + 34.0, yy + 4.0, 13.0, widgets::ink_soft());
                    yy += 24.0;
                }
            }
            1 => match sim.party.clone() {
                Some(p) => {
                    gfx::text(&format!("Party ({}/{})  -  share EXP with members on your map", p.members.len(), social::PARTY_MAX), x + 24.0, yy, 13.0, widgets::ink_soft());
                    yy += 26.0;
                    for m in &p.members {
                        self.member_row(sim, ui, m, x + 20.0, yy, w - 40.0, Some(&p.leader), None);
                        yy += 44.0;
                    }
                    yy += 8.0;
                    if p.leader.eq_ignore_ascii_case(&sim.save.name) && p.members.len() < social::PARTY_MAX {
                        gfx::text("Invite by name", x + 24.0, yy, 14.0, widgets::ink_soft());
                        yy += 22.0;
                        let enter = self.field(ui, Field::Party, x + 24.0, yy, 250.0, time);
                        if (small(ui, x + 288.0, yy + 3.0, 116.0, "Invite", ButtonStyle::Primary, !self.party_name.text.trim().is_empty()) || enter) && !self.party_name.text.trim().is_empty() {
                            let n = self.take(Field::Party);
                            sim.social_cmd(SocialCmd::PartyInvite { name: n });
                        }
                    }
                    if widgets::button(ui, x + w - 150.0, y + h - 56.0, 126.0, 34.0, "Leave party", ButtonStyle::Danger, true) {
                        sim.social_cmd(SocialCmd::PartyLeave);
                    }
                    gfx::text("Party chat: /p message", x + 24.0, y + h - 46.0, 13.0, widgets::ink_soft());
                }
                None => {
                    gfx::text_wrapped(
                        "You're not in a party. Party members on the same map share EXP from every monster. Invite someone by name, or right-click a player in the world.",
                        x + 24.0,
                        yy,
                        w - 48.0,
                        14.0,
                        widgets::ink_soft(),
                    );
                    yy += 80.0;
                    let enter = self.field(ui, Field::Party, x + 24.0, yy, 250.0, time);
                    if (small(ui, x + 288.0, yy + 3.0, 116.0, "Invite", ButtonStyle::Primary, !self.party_name.text.trim().is_empty()) || enter) && !self.party_name.text.trim().is_empty() {
                        let n = self.take(Field::Party);
                        sim.social_cmd(SocialCmd::PartyInvite { name: n });
                    }
                }
            },
            _ => {
                gfx::text_wrapped("Blocked players can't whisper you or invite you, and you won't see their chat.", x + 24.0, yy, w - 48.0, 14.0, widgets::ink_soft());
                yy += 46.0;
                let enter = self.field(ui, Field::Block, x + 24.0, yy, 250.0, time);
                if (small(ui, x + 288.0, yy + 3.0, 116.0, "Block", ButtonStyle::Danger, !self.block_name.text.trim().is_empty()) || enter) && !self.block_name.text.trim().is_empty() {
                    let n = self.take(Field::Block);
                    sim.social_cmd(SocialCmd::Block { name: n });
                }
                yy += 50.0;
                for b in sim.social.blocked.clone() {
                    gfx::text(&b, x + 34.0, yy + 4.0, 16.0, widgets::ink());
                    if small(ui, x + w - 130.0, yy, 100.0, "Unblock", ButtonStyle::Secondary, true) {
                        sim.social_cmd(SocialCmd::Unblock { name: b.clone() });
                    }
                    yy += 36.0;
                }
            }
        }
    }

    /// A party / guild member: status, name, level & class, map, HP (party).
    #[allow(clippy::too_many_arguments)]
    fn member_row(&mut self, sim: &Sim, ui: &Ui, m: &MemberNet, x: f32, y: f32, w: f32, leader: Option<&str>, rank: Option<u8>) {
        let hover = ui.enabled && gfx::mouse_in(x, y - 2.0, w, 40.0);
        gfx::rrect(x, y - 2.0, w, 40.0, 6.0, if hover { rgba(120, 180, 255, 60) } else { rgba(255, 255, 255, 50) });
        gfx::circle(x + 14.0, y + 10.0, 5.0, if m.online() { rgb(80, 200, 100) } else { rgb(150, 144, 136) });
        let mut nx = x + 26.0;
        let crown = leader.map(|l| l.eq_ignore_ascii_case(&m.name)).unwrap_or(false) || rank == Some(RANK_LEADER);
        if crown {
            let c = rgb(240, 196, 70);
            gfx::tri(nx, y + 14.0, nx + 4.0, y + 4.0, nx + 6.0, y + 14.0, c);
            gfx::tri(nx + 5.0, y + 14.0, nx + 9.0, y + 2.0, nx + 13.0, y + 14.0, c);
            gfx::tri(nx + 12.0, y + 14.0, nx + 14.0, y + 4.0, nx + 18.0, y + 14.0, c);
            nx += 22.0;
        }
        gfx::text(&m.name, nx, y + 1.0, 16.0, if m.online() { widgets::ink() } else { widgets::ink_soft() });
        let right = |t: &str, yy: f32, size: f32, c: Color| gfx::text(t, x + w - 12.0 - gfx::text_w(t, size), yy, size, c);
        if let Some(r) = rank {
            right(social::rank_name(r), y + 2.0, 12.0, if r >= RANK_OFFICER { widgets::accent() } else { widgets::ink_soft() });
        } else if m.online() {
            progress_bar(x + w - 122.0, y + 5.0, 110.0, 8.0, m.hp, rgb(226, 72, 80));
        }
        gfx::text(&format!("Lv.{} {}", m.level, class_name(&m.class)), nx, y + 20.0, 12.0, widgets::ink_soft());
        if m.online() {
            right(&map_name(&m.map), y + 20.0, 12.0, widgets::accent());
        } else {
            right("Offline", y + 20.0, 12.0, widgets::ink_soft());
        }
        if hover && ui.clicked && !m.name.eq_ignore_ascii_case(&sim.save.name) {
            let (mx, my) = gfx::mouse();
            self.open_menu(sim, &m.name, mx, my);
        }
    }

    // ------------------------------------------------------------------ guild

    fn guild_window(&mut self, sim: &mut Sim, ui: &Ui) {
        let (x, y, w, h) = (gfx::VW * 0.5 - 300.0, 56.0, 600.0, 600.0);
        widgets::panel(x, y, w, h);
        if widgets::button(ui, x + w - 52.0, y + 12.0, 34.0, 30.0, "X", ButtonStyle::Secondary, true) {
            self.guild = false;
        }
        let time = sim.time as f64;
        let Some(g) = sim.guild.clone() else {
            widgets::title("Guild", x + w * 0.5, y + 14.0, 22.0);
            let mut yy = y + 60.0;
            yy += gfx::text_wrapped(
                "Guilds are your crew for the long road: a shared chat (/g), a daily guild quest that everyone works on together, a daily reward chest, and guild-wide buffs that grow as your guild levels up.",
                x + 26.0,
                yy,
                w - 52.0,
                15.0,
                widgets::ink(),
            );
            yy += 20.0;
            for b in GUILD_BUFFS {
                gfx::text(&format!("Guild Lv.{}  {}: {}", b.level, b.name, b.desc), x + 30.0, yy, 13.0, widgets::ink_soft());
                yy += 20.0;
            }
            yy += 30.0;
            gfx::text(&format!("Found your own guild ({} gold)", social::GUILD_COST), x + 26.0, yy, 15.0, widgets::ink());
            yy += 26.0;
            let enter = self.field(ui, Field::GuildName, x + 26.0, yy, 280.0, time);
            let can = sim.save.gold >= social::GUILD_COST && !self.guild_name.text.trim().is_empty();
            if (small(ui, x + 320.0, yy + 3.0, 150.0, "Found guild", ButtonStyle::Primary, can) || (enter && can)) && can {
                let n = self.take(Field::GuildName);
                match social::validate_guild_name(&n) {
                    Ok(n) => sim.social_cmd(SocialCmd::GuildCreate { name: n }),
                    Err(e) => self.say(e),
                }
            }
            if sim.save.gold < social::GUILD_COST {
                gfx::text(&format!("You have {} gold.", sim.save.gold), x + 26.0, yy + 42.0, 13.0, widgets::danger());
            }
            gfx::text("Or ask a guild's leader or officer to invite you.", x + 26.0, yy + 70.0, 13.0, widgets::ink_soft());
            return;
        };
        widgets::title(&g.name, x + w * 0.5, y + 14.0, 24.0);
        let me = sim.save.name.clone();
        let my_rank = g.rank_of(&me);
        let level = g.level();
        let mut yy = y + 52.0;
        // level and EXP
        gfx::text(&format!("Guild Lv.{}", level), x + 26.0, yy, 17.0, widgets::ink());
        let (lo, hi) = (social::GUILD_LEVELS[(level - 1) as usize], social::guild_next(level));
        match hi {
            Some(hi) => {
                progress_bar(x + 140.0, yy + 4.0, 220.0, 12.0, (g.exp - lo) as f32 / (hi - lo).max(1) as f32, rgb(110, 190, 110));
                gfx::text(&format!("{} / {} guild EXP", g.exp, hi), x + 372.0, yy + 2.0, 12.0, widgets::ink_soft());
            }
            None => gfx::text("Max level!", x + 140.0, yy + 2.0, 14.0, widgets::accent()),
        }
        yy += 26.0;
        // buffs
        for b in GUILD_BUFFS {
            let on = level >= b.level;
            let c = if on { rgb(40, 130, 60) } else { rgba(90, 80, 70, 150) };
            gfx::text(&format!("Lv.{} {}", b.level, b.name), x + 30.0, yy, 12.0, c);
            gfx::text(b.desc, x + 274.0, yy, 12.0, c);
            yy += 17.0;
        }
        yy += 8.0;
        // today's guild quest
        let q = g.quest_def();
        gfx::rrect(x + 20.0, yy, w - 40.0, 64.0, 8.0, rgba(255, 255, 255, 70));
        gfx::text(&format!("Today's guild quest: {}", q.name), x + 30.0, yy + 6.0, 15.0, widgets::ink());
        gfx::text(q.desc, x + 30.0, yy + 26.0, 13.0, widgets::ink_soft());
        progress_bar(x + 30.0, yy + 46.0, 240.0, 10.0, g.quest_progress as f32 / q.target as f32, rgb(90, 150, 240));
        gfx::text(&format!("{}/{}", g.quest_progress.min(q.target), q.target), x + 278.0, yy + 42.0, 12.0, widgets::ink_soft());
        let label = if g.quest_claimed { "Claimed" } else { "Claim" };
        if small(ui, x + w - 120.0, yy + 18.0, 90.0, label, ButtonStyle::Primary, g.quest_done() && !g.quest_claimed) {
            sim.social_cmd(SocialCmd::GuildClaimQuest);
        }
        yy += 72.0;
        // daily chest
        let (cg, ci) = social::guild_daily_reward(level);
        let items: Vec<String> = ci.iter().map(|(id, n)| format!("{}x {}", n, items::item(id).map(|d| d.name).unwrap_or(id))).collect();
        gfx::text(&format!("Daily chest: {} gold + {}", cg, items.join(", ")), x + 30.0, yy + 6.0, 13.0, widgets::ink());
        if small(ui, x + w - 120.0, yy, 90.0, if g.daily_claimed { "Opened" } else { "Open" }, ButtonStyle::Primary, !g.daily_claimed) {
            sim.social_cmd(SocialCmd::GuildClaimDaily);
        }
        yy += 34.0;
        // notice
        gfx::text("Notice", x + 26.0, yy, 13.0, widgets::ink_soft());
        if my_rank >= RANK_OFFICER {
            if self.focus != Some(Field::Notice) && self.notice.text.is_empty() {
                self.notice.text = g.notice.clone();
            }
            let enter = self.field(ui, Field::Notice, x + 80.0, yy - 6.0, 400.0, time);
            if (small(ui, x + 492.0, yy - 3.0, 80.0, "Set", ButtonStyle::Secondary, true) || enter) && self.notice.text.trim() != g.notice {
                let t = self.take(Field::Notice);
                sim.social_cmd(SocialCmd::GuildNotice { text: t });
            }
        } else {
            gfx::text(&g.notice, x + 80.0, yy, 14.0, widgets::ink());
        }
        yy += 38.0;
        // members
        let online = g.members.iter().filter(|m| m.online()).count();
        gfx::text(&format!("Members ({} online / {} of {})", online, g.members.len(), social::GUILD_MAX), x + 26.0, yy, 13.0, widgets::ink_soft());
        yy += 20.0;
        let list_top = yy;
        let list_h = y + h - 70.0 - list_top;
        if ui.enabled && gfx::mouse_in(x, list_top, w, list_h) {
            self.member_scroll = (self.member_scroll - mouse_wheel().1.signum() * 44.0 * (mouse_wheel().1 != 0.0) as i32 as f32).clamp(0.0, (g.members.len() as f32 * 44.0 - list_h).max(0.0));
        }
        for (k, m) in g.members.iter().enumerate() {
            let ry = list_top + k as f32 * 44.0 - self.member_scroll;
            if ry < list_top - 1.0 || ry + 40.0 > list_top + list_h {
                continue;
            }
            self.member_row(sim, ui, m, x + 20.0, ry, w - 40.0, None, Some(m.rank));
        }
        // invite / leave / disband
        let by = y + h - 56.0;
        if my_rank >= RANK_OFFICER {
            let enter = self.field(ui, Field::GuildInvite, x + 26.0, by, 220.0, time);
            if (small(ui, x + 254.0, by + 3.0, 90.0, "Invite", ButtonStyle::Primary, !self.guild_invite.text.trim().is_empty()) || enter) && !self.guild_invite.text.trim().is_empty() {
                let n = self.take(Field::GuildInvite);
                sim.social_cmd(SocialCmd::GuildInvite { name: n });
            }
        }
        if my_rank == RANK_LEADER {
            if is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift) {
                if small(ui, x + 390.0, by + 3.0, 90.0, "Disband", ButtonStyle::Danger, true) {
                    sim.social_cmd(SocialCmd::GuildDisband);
                }
            } else {
                small(ui, x + 390.0, by + 3.0, 90.0, "Disband", ButtonStyle::Danger, false);
                if gfx::mouse_in(x + 390.0, by, 90.0, 30.0) {
                    gfx::text_centered_outlined("Hold Shift to disband", x + 435.0, by - 18.0, 12.0, rgb(255, 200, 190));
                }
            }
        }
        if small(ui, x + 490.0, by + 3.0, 90.0, "Leave", ButtonStyle::Secondary, true) {
            sim.social_cmd(SocialCmd::GuildLeave);
        }
    }

    // ------------------------------------------------------------------ achievements, cosmetics, attendance

    fn achievements_window(&mut self, sim: &mut Sim, ui: &Ui) {
        let (x, y, w, h) = (gfx::VW * 0.5 - 280.0, 60.0, 560.0, 590.0);
        widgets::panel(x, y, w, h);
        widgets::title("Achievements", x + w * 0.5, y + 14.0, 22.0);
        if widgets::button(ui, x + w - 52.0, y + 12.0, 34.0, 30.0, "X", ButtonStyle::Secondary, true) {
            self.achievements = false;
        }
        let claim_dot = sim.can_claim_attendance();
        for (k, t) in ["Achievements", "Cosmetics", if claim_dot { "Attendance (!)" } else { "Attendance" }].iter().enumerate() {
            if tab(ui, x + 24.0 + k as f32 * 172.0, y + 50.0, 164.0, t, self.ach_tab == k) {
                self.ach_tab = k;
            }
        }
        let score = achievements::score(&sim.save);
        gfx::text(&format!("Score {}   |   {} / {} unlocked   |   {} Gale Marks", score, sim.save.achievements.len(), ACHIEVEMENTS.len(), sim.save.marks), x + 26.0, y + 92.0, 14.0, widgets::ink());
        let top = y + 118.0;
        match self.ach_tab {
            0 => {
                let row = 50.0;
                let view = h - (top - y) - 24.0;
                if ui.enabled && gfx::mouse_in(x, top, w, view) && mouse_wheel().1 != 0.0 {
                    self.ach_scroll = (self.ach_scroll - mouse_wheel().1.signum() * row).clamp(0.0, (ACHIEVEMENTS.len() as f32 * row - view).max(0.0));
                }
                // unlocked first, then by how close you are
                let mut list: Vec<&achievements::AchievementDef> = ACHIEVEMENTS.iter().collect();
                list.sort_by_key(|a| !sim.save.achievements.iter().any(|x| x == a.id));
                for (k, a) in list.iter().enumerate() {
                    let ry = top + k as f32 * row - self.ach_scroll;
                    if ry < top - 1.0 || ry + row - 6.0 > top + view {
                        continue;
                    }
                    let done = sim.save.achievements.iter().any(|x| x == a.id);
                    gfx::rrect(x + 20.0, ry, w - 40.0, row - 6.0, 6.0, if done { rgba(250, 210, 90, 90) } else { rgba(255, 255, 255, 50) });
                    // a medal
                    let (cx, cy) = (x + 44.0, ry + 22.0);
                    gfx::circle(cx, cy, 13.0, if done { rgb(240, 190, 60) } else { rgb(170, 160, 150) });
                    gfx::circle(cx, cy, 9.0, if done { rgb(255, 226, 120) } else { rgb(200, 192, 182) });
                    gfx::text_centered(&a.points.to_string(), cx, cy - 7.0, 12.0, widgets::ink());
                    gfx::text(a.name, x + 66.0, ry + 4.0, 16.0, if done { widgets::ink() } else { widgets::ink_soft() });
                    gfx::text(a.desc, x + 66.0, ry + 24.0, 13.0, widgets::ink_soft());
                    if let Some((have, need)) = achievements::progress(a, &sim.save) {
                        if !done {
                            progress_bar(x + w - 170.0, ry + 10.0, 130.0, 8.0, have as f32 / need.max(1) as f32, rgb(90, 150, 240));
                            gfx::text(&format!("{}/{}", have.min(need), need), x + w - 170.0, ry + 22.0, 12.0, widgets::ink_soft());
                        }
                    }
                    if done {
                        gfx::text("Unlocked", x + w - 110.0, ry + 14.0, 13.0, rgb(160, 110, 20));
                    }
                }
            }
            1 => {
                gfx::text_wrapped("Exclusive cosmetics, bought only with Gale Marks from achievements. They're worn over whatever gear is in that slot.", x + 26.0, top, w - 52.0, 13.0, widgets::ink_soft());
                for (k, c) in COSMETICS.iter().enumerate() {
                    let (col, rowi) = (k % 4, k / 4);
                    let (cx, cy) = (x + 24.0 + col as f32 * 128.0, top + 46.0 + rowi as f32 * 204.0);
                    let owned = sim.save.cosmetics.iter().any(|x| x == c.id);
                    let worn = sim.wearing_cosmetic(c.id);
                    gfx::rrect(cx, cy, 120.0, 196.0, 8.0, if worn { rgba(120, 200, 255, 90) } else { rgba(255, 255, 255, 60) });
                    // preview: you, wearing it
                    let mut look: Appearance = sim.save.worn_look();
                    match c.kind {
                        achievements::CosmeticKind::Hat(hat) => look.hat = hat,
                        achievements::CosmeticKind::Back(b) => look.back = b,
                    }
                    crate::render::char_ui(&look, None, crate::charsprite::Anim::Idle, 0, cx + 60.0, cy + 108.0, 0.72, false);
                    gfx::text_centered(c.name, cx + 60.0, cy + 118.0, 13.0, widgets::ink());
                    if owned {
                        if small(ui, cx + 10.0, cy + 152.0, 100.0, if worn { "Take off" } else { "Wear" }, ButtonStyle::Secondary, true) {
                            let _ = sim.wear_cosmetic(c.id, !worn);
                        }
                    } else {
                        gfx::text_centered(&format!("{} Marks", c.price), cx + 60.0, cy + 136.0, 12.0, widgets::accent());
                        if small(ui, cx + 10.0, cy + 156.0, 100.0, "Buy", ButtonStyle::Primary, sim.save.marks >= c.price) {
                            if let Err(e) = sim.buy_cosmetic(c.id) {
                                self.say(e);
                            }
                        }
                    }
                    if ui.enabled && gfx::mouse_in(cx, cy, 120.0, 140.0) {
                        gfx::text_centered_outlined(c.desc, x + w * 0.5, y + h - 30.0, 13.0, rgb(255, 240, 200));
                    }
                }
            }
            _ => {
                gfx::text_wrapped("Log in each day to collect a reward. The rewards cycle every 7 days; day 7 is the big one.", x + 26.0, top, w - 52.0, 14.0, widgets::ink_soft());
                let next = sim.save.attendance.next_index();
                let can = sim.can_claim_attendance();
                for (k, r) in ATTENDANCE.iter().enumerate() {
                    let (cx, cy) = (x + 24.0 + (k % 4) as f32 * 128.0, top + 48.0 + (k / 4) as f32 * 156.0);
                    let claimed_this_cycle = k < next;
                    let today = k == next && can;
                    gfx::rrect(cx, cy, 120.0, 146.0, 8.0, if today { rgba(255, 210, 90, 140) } else if claimed_this_cycle { rgba(120, 200, 120, 80) } else { rgba(255, 255, 255, 60) });
                    gfx::text_centered(&format!("Day {}", k + 1), cx + 60.0, cy + 6.0, 16.0, widgets::ink());
                    let mut ly = cy + 30.0;
                    if r.gold > 0 {
                        gfx::circle(cx + 20.0, ly + 7.0, 6.0, rgb(240, 196, 70));
                        gfx::text(&format!("{} gold", r.gold), cx + 32.0, ly, 13.0, widgets::ink());
                        ly += 22.0;
                    }
                    for (id, n) in r.items {
                        if let Some(d) = items::item(id) {
                            gfx::tex(&item_tex(d), cx + 8.0, ly - 6.0, 0.7, false, WHITE);
                            gfx::text(&format!("{}x {}", n, d.name), cx + 36.0, ly, 11.0, widgets::ink());
                            ly += 24.0;
                        }
                    }
                    if r.marks > 0 {
                        gfx::text(&format!("{} Gale Marks", r.marks), cx + 12.0, ly, 13.0, widgets::accent());
                    }
                    if claimed_this_cycle {
                        gfx::text_centered("Collected", cx + 60.0, cy + 124.0, 13.0, rgb(40, 130, 60));
                    }
                }
                let by = top + 380.0;
                gfx::text(&format!("Days collected: {}", sim.save.attendance.count), x + 26.0, by + 8.0, 15.0, widgets::ink());
                if widgets::button(ui, x + w - 230.0, by, 200.0, 40.0, if can { "Collect today's reward" } else { "Come back tomorrow" }, ButtonStyle::Primary, can) {
                    if let Err(e) = sim.claim_attendance() {
                        self.say(e);
                    }
                }
            }
        }
    }

    fn attendance_banner(&mut self, sim: &mut Sim, ui: &Ui) {
        if !self.attendance_prompt || !sim.can_claim_attendance() || self.achievements {
            return;
        }
        let (w, h) = (420.0, 64.0);
        let (x, y) = (16.0, 196.0);
        gfx::rrect(x, y, w, h, 10.0, rgba(24, 22, 40, 225));
        gfx::rect_lines(x + 1.0, y + 1.0, w - 2.0, h - 2.0, 1.0, rgba(255, 220, 120, 180));
        gfx::text(&format!("Daily reward ready: day {}!", sim.save.attendance.count + 1), x + 16.0, y + 10.0, 16.0, rgb(255, 230, 150));
        gfx::text("Collect it now or later from Achievements [H].", x + 16.0, y + 34.0, 12.0, rgb(200, 206, 226));
        if widgets::button(ui, x + w - 180.0, y + 14.0, 84.0, 34.0, "Collect", ButtonStyle::Primary, true) {
            let _ = sim.claim_attendance();
            self.attendance_prompt = false;
        }
        if widgets::button(ui, x + w - 90.0, y + 14.0, 74.0, 34.0, "Later", ButtonStyle::Secondary, true) {
            self.attendance_prompt = false;
        }
    }

    // ------------------------------------------------------------------ profile

    fn profile_window(&mut self, sim: &mut Sim, ui: &Ui) {
        let Some(p) = sim.profile.clone() else { return };
        let (x, y, w, h) = (gfx::VW - 460.0, 70.0, 430.0, 520.0);
        widgets::panel(x, y, w, h);
        widgets::title(&p.name, x + w * 0.5, y + 14.0, 22.0);
        if widgets::button(ui, x + w - 52.0, y + 12.0, 34.0, 30.0, "X", ButtonStyle::Secondary, true) {
            self.profile = false;
            sim.profile = None;
            return;
        }
        let look = Appearance::decode(&p.look).unwrap_or_default();
        let weapon = p.gear.first().filter(|g| !g.is_empty()).and_then(|g| items::weapon_art(g));
        gfx::rrect(x + 22.0, y + 52.0, 130.0, 170.0, 8.0, rgba(140, 190, 240, 90));
        crate::render::char_ui(&look, weapon, crate::charsprite::Anim::Idle, ((sim.time * 4.0) as u8) % 4, x + 87.0, y + 210.0, 1.05, false);
        let tx = x + 170.0;
        let mut yy = y + 56.0;
        let line = |t: &str, v: String, yy: &mut f32| {
            gfx::text(t, tx, *yy, 13.0, widgets::ink_soft());
            gfx::text(&v, tx + 92.0, *yy - 1.0, 15.0, widgets::ink());
            *yy += 24.0;
        };
        line("Class", class_name(&p.class).to_string(), &mut yy);
        line("Level", p.level.to_string(), &mut yy);
        gfx::text("EXP", tx, yy, 13.0, widgets::ink_soft());
        progress_bar(tx + 92.0, yy + 3.0, 100.0, 10.0, p.exp_frac, rgb(110, 190, 240));
        gfx::text(&format!("{:.1}%", p.exp_frac * 100.0), tx + 92.0, yy + 15.0, 11.0, widgets::ink_soft());
        yy += 32.0;
        line("Guild", if p.guild.is_empty() { "-".into() } else { p.guild.clone() }, &mut yy);
        line("Score", format!("{}  ({} achieved)", p.score, p.achievements), &mut yy);
        line("Played", time_played(p.play_seconds), &mut yy);
        // gear
        let gy = y + 240.0;
        gfx::text("Equipped", x + 24.0, gy, 14.0, widgets::ink_soft());
        let mut hovered: Option<&'static items::ItemDef> = None;
        for (k, slot) in EQUIP_SLOTS.iter().enumerate() {
            let sx = x + 24.0 + (k % 6) as f32 * 64.0;
            let sy = gy + 22.0 + (k / 6) as f32 * 66.0;
            let def = p.gear.get(k).filter(|g| !g.is_empty()).and_then(|g| items::item(g));
            gfx::rrect(sx, sy, 46.0, 46.0, 6.0, def.map(|d| alpha(px_color(icons::tier_color(d.tier)), 0.5)).unwrap_or(rgba(0, 0, 0, 30)));
            match def {
                Some(d) => {
                    gfx::tex(&item_tex(d), sx + 5.0, sy + 5.0, 1.0, false, WHITE);
                    if gfx::mouse_in(sx, sy, 46.0, 46.0) {
                        hovered = Some(d);
                    }
                }
                None => gfx::text_centered(&slot.name()[..1], sx + 23.0, sy + 14.0, 16.0, rgb(190, 170, 140)),
            }
            gfx::text_centered(slot.short(), sx + 23.0, sy + 48.0, 11.0, widgets::ink_soft());
        }
        // actions
        let by = y + h - 56.0;
        let name = p.name.clone();
        if small(ui, x + 22.0, by, 110.0, "Whisper", ButtonStyle::Secondary, true) {
            self.start_chat = Some(format!("/w {} ", name));
        }
        if !sim.social.is_friend(&name) && small(ui, x + 140.0, by, 110.0, "Add friend", ButtonStyle::Primary, true) {
            sim.social_cmd(SocialCmd::FriendAdd { name: name.clone() });
        }
        if small(ui, x + 298.0, by, 110.0, "More...", ButtonStyle::Secondary, true) {
            let (mx, my) = gfx::mouse();
            self.open_menu(sim, &name, mx - 120.0, my - 200.0);
        }
        if let Some(d) = hovered {
            let lines = d.stat_lines();
            let th = 30.0 + lines.len() as f32 * 18.0;
            let (mx, my) = gfx::mouse();
            let (tx, ty) = ((mx + 16.0).min(gfx::VW - 230.0), (my + 10.0).min(gfx::VH - th - 4.0));
            gfx::rrect(tx, ty, 220.0, th, 6.0, rgba(24, 22, 40, 235));
            gfx::text(d.name, tx + 10.0, ty + 6.0, 15.0, px_color(icons::tier_color(d.tier)));
            for (i, l) in lines.iter().enumerate() {
                gfx::text(l, tx + 10.0, ty + 28.0 + i as f32 * 18.0, 13.0, rgb(160, 240, 170));
            }
        }
    }

    // ------------------------------------------------------------------ trade

    fn trade_window(&mut self, sim: &mut Sim, ui: &Ui) {
        let Some(t) = sim.trade.clone() else {
            self.offer.clear();
            self.offer_gold = 0;
            self.trade_with.clear();
            return;
        };
        if self.trade_with != t.with {
            // a new trade
            self.trade_with = t.with.clone();
            self.offer.clear();
            self.offer_gold = 0;
            self.trade_gold.text.clear();
        }
        let (w, h) = (640.0, 470.0);
        let (x, y) = (gfx::VW * 0.5 - w * 0.5, 80.0);
        widgets::panel(x, y, w, h);
        widgets::title(&format!("Trading with {}", t.with), x + w * 0.5, y + 14.0, 22.0);
        let time = sim.time as f64;
        let col_w = (w - 60.0) * 0.5;
        let mut changed = false;
        // --- your side
        let lx = x + 22.0;
        gfx::text("Your offer", lx, y + 52.0, 16.0, widgets::ink());
        self.offer_list(&t.mine, lx, y + 76.0, col_w, ui, true, &mut changed);
        gfx::text("Gold", lx, y + 254.0, 14.0, widgets::ink_soft());
        if !t.my_lock {
            self.field(ui, Field::TradeGold, lx + 50.0, y + 246.0, 140.0, time);
            let g: u32 = self.trade_gold.text.parse().unwrap_or(0);
            if g != self.offer_gold {
                self.offer_gold = g;
                changed = true;
            }
        } else {
            gfx::text(&format!("{}", t.my_gold), lx + 56.0, y + 252.0, 16.0, widgets::ink());
        }
        // pick items from your bag
        gfx::text("Click items in your bag to offer them (Shift: whole stack)", lx, y + 292.0, 12.0, widgets::ink_soft());
        let bag: Vec<(String, u32)> = sim
            .save
            .inventory
            .slots
            .iter()
            .filter(|s| items::item(&s.id).map(|d| d.can_trade()).unwrap_or(false))
            .map(|s| (s.id.clone(), s.count))
            .collect();
        for (k, (id, n)) in bag.iter().enumerate().take(16) {
            let (sx, sy) = (lx + (k % 8) as f32 * 36.0, y + 312.0 + (k / 8) as f32 * 38.0);
            let Some(d) = items::item(id) else { continue };
            let offered = self.offer.iter().filter(|o| &o.0 == id).map(|o| o.1).sum::<u32>();
            let left = sim.save.inventory.count(id).saturating_sub(offered);
            gfx::rrect(sx, sy, 32.0, 32.0, 5.0, rgba(0, 0, 0, if left > 0 { 30 } else { 10 }));
            gfx::tex(&item_tex(d), sx + 2.0, sy + 2.0, 0.78, false, if left > 0 { WHITE } else { Color::new(1.0, 1.0, 1.0, 0.3) });
            if *n > 1 {
                gfx::text_outlined(&format!("{}", left), sx + 18.0, sy + 18.0, 11.0, WHITE);
            }
            if !t.my_lock && left > 0 && ui.enabled && ui.clicked && gfx::mouse_in(sx, sy, 32.0, 32.0) && self.offer.len() < social::TRADE_SLOTS {
                let take = if is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift) { left } else { 1 };
                match self.offer.iter_mut().find(|o| &o.0 == id) {
                    Some(o) => o.1 += take,
                    None => self.offer.push((id.clone(), take)),
                }
                changed = true;
            }
        }
        // --- their side
        let rx = x + 38.0 + col_w;
        gfx::text(&format!("{}'s offer", t.with), rx, y + 52.0, 16.0, widgets::ink());
        let mut dummy = false;
        self.offer_list(&t.theirs, rx, y + 76.0, col_w, ui, false, &mut dummy);
        gfx::text(&format!("Gold   {}", t.their_gold), rx, y + 254.0, 14.0, widgets::ink());
        let status = |locked: bool, ok: bool| if ok { ("Confirmed", rgb(40, 140, 60)) } else if locked { ("Locked in", rgb(180, 120, 20)) } else { ("Choosing...", widgets::ink_soft()) };
        let (s1, c1) = status(t.my_lock, t.my_ok);
        let (s2, c2) = status(t.their_lock, t.their_ok);
        gfx::text(s1, lx + 200.0, y + 52.0, 14.0, c1);
        gfx::text(s2, rx + 200.0, y + 52.0, 14.0, c2);
        // offer changes go to the server (and unlock both sides)
        if changed && !t.my_lock {
            match sim.trade_offer_problem(&self.offer, self.offer_gold) {
                Some(e) => {
                    self.say(e);
                    self.offer_gold = self.offer_gold.min(sim.save.gold);
                    self.trade_gold.text = self.offer_gold.to_string();
                }
                None => sim.social_cmd(SocialCmd::TradeOffer { items: self.offer.clone(), gold: self.offer_gold }),
            }
        }
        // buttons
        let by = y + h - 60.0;
        let fits = sim.trade_fits(&t);
        if !fits {
            gfx::text("Not enough room in your bag for their offer!", rx, y + 300.0, 13.0, widgets::danger());
        }
        if widgets::button(ui, rx, by, 140.0, 38.0, if t.my_lock { "Unlock" } else { "Lock offer" }, ButtonStyle::Secondary, !t.my_ok) {
            sim.social_cmd(SocialCmd::TradeLock { locked: !t.my_lock });
        }
        let can_ok = t.my_lock && t.their_lock && !t.my_ok && fits;
        if widgets::button(ui, rx + 150.0, by, 140.0, 38.0, "Confirm", ButtonStyle::Primary, can_ok) {
            sim.social_cmd(SocialCmd::TradeConfirm);
        }
        if widgets::button(ui, lx + 160.0, by, 120.0, 38.0, "Cancel", ButtonStyle::Danger, true) {
            sim.social_cmd(SocialCmd::TradeCancel);
            sim.trade = None;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn offer_list(&mut self, list: &[(String, u32)], x: f32, y: f32, w: f32, ui: &Ui, mine: bool, changed: &mut bool) {
        gfx::rrect(x, y, w, 160.0, 8.0, rgba(255, 255, 255, 70));
        if list.is_empty() && !(mine && !self.offer.is_empty()) {
            gfx::text("(nothing yet)", x + 12.0, y + 12.0, 13.0, widgets::ink_soft());
        }
        let shown: Vec<(String, u32)> = if mine { self.offer.clone() } else { list.to_vec() };
        let mut remove = None;
        for (k, (id, n)) in shown.iter().enumerate() {
            let Some(d) = items::item(id) else { continue };
            let (sx, sy) = (x + 8.0 + (k % 2) as f32 * (w * 0.5), y + 8.0 + (k / 2) as f32 * 38.0);
            gfx::tex(&item_tex(d), sx, sy, 0.8, false, WHITE);
            gfx::text(&format!("{}x {}", n, d.name), sx + 34.0, sy + 8.0, 12.0, widgets::ink());
            if mine && ui.enabled && ui.clicked && gfx::mouse_in(sx, sy, w * 0.5 - 8.0, 32.0) {
                remove = Some(k);
            }
        }
        if let Some(k) = remove {
            self.offer.remove(k);
            *changed = true;
        }
    }

    // ------------------------------------------------------------------ invitations

    fn invites(&mut self, sim: &mut Sim, ui: &Ui) {
        let list = sim.invites.clone();
        let (w, h) = (440.0, 70.0);
        for (k, (kind, from, extra, left)) in list.iter().enumerate() {
            let (x, y) = (gfx::VW - w - 16.0, 110.0 + k as f32 * (h + 8.0));
            gfx::rrect(x, y, w, h, 10.0, rgba(24, 22, 40, 230));
            gfx::rect_lines(x + 1.0, y + 1.0, w - 2.0, h - 2.0, 1.0, rgba(120, 220, 255, 160));
            let text = match *kind {
                INV_FRIEND => format!("{} wants to be your friend.", from),
                INV_PARTY => format!("{} invites you to a party.", from),
                INV_GUILD => format!("{} invites you to join {}.", from, extra),
                INV_TRADE => format!("{} wants to trade with you.", from),
                _ => format!("{} sent an invitation.", from),
            };
            gfx::text(&text, x + 16.0, y + 12.0, 15.0, rgb(240, 244, 255));
            progress_bar(x + 16.0, y + 48.0, 200.0, 5.0, left / 60.0, rgb(120, 220, 255));
            let answer = if widgets::button(ui, x + w - 190.0, y + 30.0, 84.0, 30.0, "Accept", ButtonStyle::Primary, true) {
                Some(true)
            } else if widgets::button(ui, x + w - 98.0, y + 30.0, 84.0, 30.0, "Decline", ButtonStyle::Secondary, true) {
                Some(false)
            } else {
                None
            };
            if let Some(accept) = answer {
                let cmd = match *kind {
                    INV_FRIEND => SocialCmd::FriendAnswer { name: from.clone(), accept },
                    INV_PARTY => SocialCmd::PartyAnswer { from: from.clone(), accept },
                    INV_GUILD => SocialCmd::GuildAnswer { guild: extra.clone(), accept },
                    _ => SocialCmd::TradeAnswer { from: from.clone(), accept },
                };
                sim.social_cmd(cmd);
                sim.invites.retain(|v| !(v.0 == *kind && &v.1 == from));
            }
        }
    }
}
