//! The game simulation: player controller, combat, enemies, projectiles,
//! NPC interaction, quests and map transitions. Runs at a fixed tick rate
//! and knows nothing about rendering or windowing.

use crate::classes::{self, AttackDef, AttackKind, ClassId, ClassInfo, MainStat, ProjStyle};
use crate::input::{Action, InputState, HOTBAR_ACTIONS};
use crate::items::{self, Effect, EquipSlot, ItemKind, Special};
use crate::skills::{self, HotbarSlot, SkillDef, SkillEffect, HOTBAR_SLOTS};
use crate::stats::{self, Derived, StatMods};
use crate::math::{approach, Aabb, Rng, V2};
use crate::physics::{self, Body, Level};
use crate::pets::{self, PetSave, PetSpecies};
use crate::quests::{self, Dialog, DialogAction, QuestEvent};
use crate::save::{CharacterSave, QuestState, Settings};
use crate::enemies::{self, EnemyKind};
use crate::net::{self, ClientMsg, DropNet, EnemyNet, PlayerInfo, PlayerPos, ServerMsg};
use crate::world::{self, Map};

pub const TICK: f32 = 1.0 / 120.0;
pub const PLAYER_W: f32 = 28.0;
pub const PLAYER_H: f32 = 64.0;
const COYOTE: f32 = 0.09;
const JUMP_BUFFER: f32 = 0.12;
const ATTACK_BUFFER: f32 = 0.14;
const DODGE_TIME: f32 = 0.34;
/// Invincible for the whole dodge and a little after it.
const DODGE_IFRAMES: f32 = 0.42;
const DODGE_COOLDOWN: f32 = 0.6;
const DODGE_SPEED: f32 = 520.0;
const HURT_IFRAMES: f32 = 0.9;
const POTION_COOLDOWN: f32 = 1.0;
/// Gold per waystone trip.
pub const WAYSTONE_COST: u32 = 25;
const INTERACT_RANGE: f32 = 90.0;
const CLIMB_SPEED: f32 = 190.0;
const CLIMB_REACH: f32 = 18.0;
/// Seconds loot stays on the ground before it vanishes.
pub const DROP_LIFE: f32 = 120.0;
/// How far from the player (horizontally) loot can be picked up.
pub const PICKUP_REACH: f32 = 70.0;
/// Holding the pick-up key grabs another item this often.
const PICKUP_REPEAT: f32 = 0.12;
/// How long a picked-up item takes to fly to whoever collected it.
pub const COLLECT_FLY: f32 = 0.18;
/// Characters above this level lose EXP when they die...
pub const DEATH_PENALTY_LEVEL: u32 = 10;
/// ...this share of their current EXP (a Revive Scroll gives it back).
pub const DEATH_EXP_LOSS: f32 = 0.03;
/// Seconds of the defeat animation before the tombstone drops.
pub const TOMB_DELAY: f32 = 0.7;

/// A climbable the player can grab right now (`up` = pressing Up, else Down).
fn find_climb(climbs: &[world::Climb], body: &Body, up: bool) -> Option<usize> {
    let cx = body.center().x;
    let feet = body.feet().y;
    climbs.iter().position(|c| {
        if (cx - c.x).abs() > CLIMB_REACH {
            return false;
        }
        if up {
            feet > c.top + 2.0 && feet <= c.bottom + 4.0
        } else {
            ((feet - c.top).abs() < 8.0 && body.on_ground) || (feet > c.top + 2.0 && feet < c.bottom - 4.0)
        }
    })
}

pub fn exp_to_next(level: u32) -> u32 {
    (30.0 * (level as f32).powf(1.55)).round() as u32
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PState {
    Normal,
    Attack,
    Dodge,
    Hitstun,
    Dead,
    /// On a ladder, rope or vine.
    Climb,
}

#[derive(Clone, Debug)]
pub struct AttackState {
    pub kind: AttackKind,
    pub def: AttackDef,
    pub t: f32,
    pub fired: bool,
    pub hit_ids: Vec<u32>,
    /// Set when this attack is an ability from the hotbar.
    pub skill: Option<&'static SkillDef>,
    /// Multi-hit abilities clear `hit_ids` once per wave.
    pub wave: u8,
}

impl AttackState {
    pub fn phase(&self) -> AttackPhase {
        if self.t < self.def.startup {
            AttackPhase::Startup
        } else if self.t < self.def.startup + self.def.active {
            AttackPhase::Active
        } else {
            AttackPhase::Recovery
        }
    }
    /// 0..1 through the whole move, for animation.
    pub fn progress(&self) -> f32 {
        (self.t / self.def.total()).clamp(0.0, 1.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AttackPhase {
    Startup,
    Active,
    Recovery,
}

#[derive(Clone, Debug)]
pub struct Player {
    pub body: Body,
    pub facing: f32,
    pub state: PState,
    pub state_t: f32,
    pub class: ClassId,
    pub info: ClassInfo,
    pub hp: f32,
    pub mp: f32,
    pub max_hp: f32,
    pub max_mp: f32,
    pub air_jumps_left: u8,
    pub air_dodge_used: bool,
    pub recovery_used: bool,
    pub coyote: f32,
    pub jump_buffer: f32,
    pub attack_buffer: Option<(bool, f32)>,
    pub attack: Option<AttackState>,
    pub dodge_dir: V2,
    pub dodge_cd: f32,
    pub invuln: f32,
    pub hurt_flash: f32,
    pub since_hurt: f32,
    pub potion_cd: f32,
    pub anim_t: f32,
    pub land_squash: f32,
    pub jumped_from_ground: bool,
    /// Combat stats derived from main stats, level, gear and buffs.
    pub stats: Derived,
    /// Index of the climbable being climbed, and climbing distance (for the animation).
    pub climb: usize,
    pub climb_t: f32,
    pub buffs: Vec<Buff>,
    /// Remaining cooldown per ability id.
    pub cooldowns: Vec<(&'static str, f32)>,
}

/// A temporary stat boost from an ability or a tonic.
#[derive(Clone, Debug)]
pub struct Buff {
    pub id: &'static str,
    pub name: &'static str,
    pub mods: StatMods,
    pub left: f32,
    pub total: f32,
    pub fx: skills::SkillFx,
}

impl Player {
    pub fn cooldown(&self, id: &str) -> f32 {
        self.cooldowns.iter().find(|c| c.0 == id).map(|c| c.1).unwrap_or(0.0)
    }
    pub fn is_invulnerable(&self) -> bool {
        self.invuln > 0.0
            || self.state == PState::Dead
            || (self.state == PState::Attack
                && self.attack.as_ref().map(|a| a.def.invuln && a.phase() != AttackPhase::Recovery).unwrap_or(false))
    }
}

pub use crate::enemies::{EState, Enemy};

/// What's lying on the ground.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DropKind {
    Item(&'static str),
    Gold,
}

/// Who picked a drop up. Pets use the same pick-up path as players.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Collector {
    Player,
    Pet(u32),
}

/// Loot or gold lying in the world, waiting to be picked up.
#[derive(Clone, Debug)]
pub struct GroundDrop {
    pub id: u32,
    pub kind: DropKind,
    /// Stack size (or amount of gold).
    pub count: u32,
    pub body: Body,
    pub age: f32,
    /// Set once collected: the drop flies to the collector, then disappears.
    pub taken: Option<(Collector, V2, f32)>,
    /// Online: seconds left waiting for the server to answer a pick-up.
    pub pending: f32,
}

impl GroundDrop {
    pub fn name(&self) -> String {
        match self.kind {
            DropKind::Gold => format!("{} gold", self.count),
            DropKind::Item(id) => {
                let n = items::item(id).map(|d| d.name).unwrap_or(id);
                if self.count > 1 {
                    format!("{}x {}", self.count, n)
                } else {
                    n.to_string()
                }
            }
        }
    }
}

/// Your pet out in the world (its level, hunger etc. live in the save).
#[derive(Clone, Debug)]
pub struct PetActor {
    /// Feet position.
    pub pos: V2,
    pub facing: f32,
    pub moving: bool,
    pub anim_t: f32,
    /// Drop it's fetching.
    pub target: Option<u32>,
    /// Drops it couldn't take (bag full); retried every few seconds.
    skip: Vec<u32>,
    skip_t: f32,
    warned_low: bool,
}

/// The network name of a drop's contents.
pub fn drop_item_key(k: DropKind) -> String {
    match k {
        DropKind::Gold => net::GOLD.to_string(),
        DropKind::Item(id) => id.to_string(),
    }
}

/// Another player on your map (online).
#[derive(Clone, Debug)]
pub struct OtherPlayer {
    pub id: u32,
    pub name: String,
    /// Feet position where they're drawn (glides toward the latest update).
    pub pos: V2,
    pub dead: bool,
    pub info: PlayerInfo,
    /// Their latest reported state.
    pub st: PlayerPos,
    pub pet_pos: V2,
    /// Seconds since we last heard where they are.
    pub since: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Team {
    Player,
    Enemy,
}

#[derive(Clone, Debug)]
pub struct Projectile {
    pub team: Team,
    pub pos: V2,
    pub vel: V2,
    pub w: f32,
    pub h: f32,
    pub gravity: f32,
    pub life: f32,
    pub max_life: f32,
    pub pierce: bool,
    pub damage: f32,
    pub kb: V2,
    pub hitstun: f32,
    pub style: ProjStyle,
    pub hit_ids: Vec<u32>,
    pub grounded: bool,
    pub heavy: bool,
    /// Extra crit chance (percentage points), for abilities.
    pub crit_bonus: f32,
    /// Magic damage (enemy shots: reduced by M.DEF instead of P.DEF).
    pub magic: bool,
    /// Distance flown so far and the most it may fly (see [`max_range`]).
    pub travelled: f32,
    pub range: f32,
}

/// How far a projectile of this style may fly before it fizzles out.
/// Roughly a screen's width for the longest bows and bolts, less for thrown
/// things; monster shots reach about two-thirds of a screen.
pub fn max_range(style: ProjStyle, team: Team) -> f32 {
    let r: f32 = match style {
        ProjStyle::Arrow => 760.0,
        ProjStyle::Bolt => 640.0,
        ProjStyle::Knife => 420.0,
        ProjStyle::Orb => 480.0,
        ProjStyle::Star => 560.0,
        ProjStyle::Caltrop => 260.0,
        ProjStyle::Shockwave => 520.0,
    };
    if team == Team::Enemy { r.min(620.0) } else { r }
}

impl Projectile {
    pub fn aabb(&self) -> Aabb {
        Aabb::new(self.pos.x - self.w * 0.5, self.pos.y - self.h * 0.5, self.w, self.h)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FxKind {
    Hit,
    Crit,
    Dust,
    Heal,
    LevelUp,
    Spark,
}

#[derive(Clone, Debug)]
pub struct Particle {
    pub pos: V2,
    pub vel: V2,
    pub life: f32,
    pub max_life: f32,
    pub kind: FxKind,
    pub size: f32,
}

#[derive(Clone, Debug)]
pub struct FloatText {
    pub pos: V2,
    pub text: String,
    pub life: f32,
    pub kind: FloatKind,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FloatKind {
    Damage,
    Crit,
    PlayerHurt,
    Heal,
    Info,
    Exp,
    Loot,
}

#[derive(Clone, Debug)]
pub struct Toast {
    pub title: String,
    pub body: String,
    pub life: f32,
}

#[derive(Clone, Debug)]
pub struct ActiveDialog {
    pub dialog: Dialog,
    pub line: usize,
    /// Index of the highlighted choice on the final line (0 = accept/buy).
    pub choice: usize,
    pub message: Option<String>,
}

impl ActiveDialog {
    pub fn on_last_line(&self) -> bool {
        self.line + 1 >= self.dialog.lines.len()
    }
    pub fn choices(&self) -> Option<[&'static str; 2]> {
        if !self.on_last_line() {
            return None;
        }
        match self.dialog.action {
            DialogAction::OfferQuest(_) => Some(["Accept", "Not now"]),
            DialogAction::Shop(_) => Some(["Browse wares", "Goodbye"]),
            DialogAction::Waystone => Some(["Travel", "Cancel"]),
            DialogAction::SummonBoss(_) => Some(["Summon it (100 gold)", "Not now"]),
            DialogAction::ClassChange(c) => Some([
                match c {
                    ClassId::Vanguard => "Become a Vanguard",
                    ClassId::Duskblade => "Become a Duskblade",
                    ClassId::Stormfist => "Become a Stormfist",
                    ClassId::Windrunner => "Become a Windrunner",
                    ClassId::Aethermancer => "Become an Aethermancer",
                    ClassId::Novice => "Stay a Novice",
                },
                "Not yet",
            ]),
            _ => None,
        }
    }
}

pub struct Sim {
    pub save: CharacterSave,
    pub settings: Settings,
    pub map: Map,
    pub player: Player,
    pub enemies: Vec<Enemy>,
    pub projectiles: Vec<Projectile>,
    pub particles: Vec<Particle>,
    pub floats: Vec<FloatText>,
    pub toasts: Vec<Toast>,
    pub dialog: Option<ActiveDialog>,
    pub hitstop: f32,
    pub shake: f32,
    pub time: f32,
    pub zone_timers: Vec<f32>,
    next_id: u32,
    rng: Rng,
    /// Set when progress worth saving happened; the host writes the save.
    pub wants_save: bool,
    /// Set when the map changed; the host snaps its camera and clears it.
    pub map_changed: bool,
    pub region_cooldown: f32,
    pub play_time_accum: f32,
    pub quest_log_open: bool,
    /// Shop window the host should show (opened by talking to a shopkeeper).
    pub shop_open: Option<&'static str>,
    /// Waystone travel menu the host should show.
    pub teleport_open: bool,
    /// Set when the player's look may have changed (gear); the host refreshes.
    pub look_changed: bool,
    /// Loot and gold on the ground.
    pub drops: Vec<GroundDrop>,
    pickup_cd: f32,
    /// Last place the player stood (a tombstone lands here after a fall).
    pub last_ground: V2,
    /// Other players on this map (empty until multiplayer).
    pub others: Vec<OtherPlayer>,
    /// The active pet's body in the world.
    pub pet: Option<PetActor>,
    /// Playing on a server: monsters, loot and other players come from it.
    pub online: bool,
    /// Messages for the server, sent by the host after each frame.
    pub outbox: Vec<ClientMsg>,
    /// Our player id on the server.
    pub my_id: u32,
    /// Chat and server notices (newest last): (from, text, age).
    pub chat: Vec<(String, String, f32)>,
}

fn derive_stats(save: &CharacterSave, buffs: &[Buff]) -> Derived {
    let mut mods = save.inventory.gear_mods();
    // A fed pet's passives count like gear.
    if let Some(p) = save.active_pet.and_then(|i| save.pets.get(i)) {
        mods = mods + p.stat_mods();
    }
    for b in buffs {
        mods = mods + b.mods;
    }
    stats::derive(save.class, save.level, save.stats, &mods)
}

fn ground_y_at(map: &Map, x: f32, from_y: f32) -> f32 {
    // Highest surface at or below `from_y` under x.
    let mut best = f32::MAX;
    for s in map.solids.iter().chain(map.platforms.iter()) {
        if x >= s.left() && x <= s.right() && s.top() >= from_y - 1.0 && s.top() < best {
            best = s.top();
        }
    }
    if best == f32::MAX {
        // Nothing below: use the highest surface at this x.
        for s in &map.solids {
            if x >= s.left() && x <= s.right() {
                best = best.min(s.top());
            }
        }
    }
    if best == f32::MAX {
        world::GROUND
    } else {
        best
    }
}

impl Sim {
    pub fn new(mut save: CharacterSave, settings: Settings, seed: u64) -> Sim {
        let map = world::load_map(&save.map).unwrap_or_else(|| {
            save.map = world::START_MAP.into();
            save.x = -1.0;
            world::load_map(world::START_MAP).unwrap()
        });
        let info = save.class.info();
        let derived = derive_stats(&save, &[]);
        let (max_hp, max_mp) = (derived.max_hp, derived.max_mp);
        let (sx, sy) = if save.x >= 0.0 { (save.x, save.y) } else { map.spawn };
        let mut body = Body::new(sx - PLAYER_W * 0.5, sy - PLAYER_H, PLAYER_W, PLAYER_H);
        body.on_ground = false;
        let hp = if save.hp <= 0.0 { max_hp } else { save.hp.min(max_hp) };
        let mp = if save.mp < 0.0 { max_mp } else { save.mp.min(max_mp) };
        let player = Player {
            body,
            facing: 1.0,
            state: PState::Normal,
            state_t: 0.0,
            class: save.class,
            info: info.clone(),
            hp,
            mp,
            max_hp,
            max_mp,
            air_jumps_left: info.air_jumps,
            air_dodge_used: false,
            recovery_used: false,
            coyote: 0.0,
            jump_buffer: 0.0,
            attack_buffer: None,
            attack: None,
            dodge_dir: V2::ZERO,
            dodge_cd: 0.0,
            invuln: 1.0,
            hurt_flash: 0.0,
            since_hurt: 99.0,
            potion_cd: 0.0,
            anim_t: 0.0,
            land_squash: 0.0,
            jumped_from_ground: false,
            stats: derived,
            climb: 0,
            climb_t: 0.0,
            buffs: Vec::new(),
            cooldowns: Vec::new(),
        };
        let mut sim = Sim {
            save,
            settings,
            map,
            player,
            enemies: Vec::new(),
            projectiles: Vec::new(),
            particles: Vec::new(),
            floats: Vec::new(),
            toasts: Vec::new(),
            dialog: None,
            hitstop: 0.0,
            shake: 0.0,
            time: 0.0,
            zone_timers: Vec::new(),
            next_id: 1,
            rng: Rng::new(seed),
            wants_save: false,
            map_changed: true,
            region_cooldown: 0.0,
            play_time_accum: 0.0,
            quest_log_open: false,
            shop_open: None,
            teleport_open: false,
            look_changed: true,
            drops: Vec::new(),
            pickup_cd: 0.0,
            last_ground: V2::new(sx, sy),
            others: Vec::new(),
            pet: None,
            online: false,
            outbox: Vec::new(),
            my_id: 0,
            chat: Vec::new(),
        };
        // Logged out as a tombstone? Still one.
        if sim.save.tomb.is_some() {
            sim.player.state = PState::Dead;
            sim.player.state_t = TOMB_DELAY + 1.0;
            sim.player.hp = 0.0;
        }
        sim.populate_map();
        if sim.save.quests.is_empty() && sim.map.key == world::START_MAP {
            sim.toast("Welcome to Galebound!", "Talk to Elder Wynn (the '!' above her head) to begin.");
        }
        sim
    }

    fn level(&self) -> Level<'_> {
        // Closed gates act as solids; built per query (only a handful of rects).
        Level { solids: &self.map.solids, platforms: &self.map.platforms, bounds: self.map.bounds() }
    }

    fn solids_with_gates(&self) -> Vec<Aabb> {
        let mut v = self.map.solids.clone();
        for g in &self.map.gates {
            if !self.save.has_flag(g.open_flag) {
                v.push(g.solid);
            }
        }
        v
    }

    fn populate_map(&mut self) {
        self.enemies.clear();
        self.projectiles.clear();
        self.zone_timers = vec![0.0; self.map.spawns.len()];
        if self.online {
            // the server sends this map's monsters; ask for any quest boss
            self.maybe_spawn_boss();
            return;
        }
        let fixed = self.map.fixed_enemies.clone();
        for (k, x, y) in fixed {
            self.spawn_enemy(k, x, y, None);
        }
        for zi in 0..self.map.spawns.len() {
            let z = self.map.spawns[zi].clone();
            for _ in 0..z.max {
                let x = self.rng.range(z.x0, z.x1);
                self.spawn_enemy(z.kind, x, z.y, Some(zi));
            }
        }
        self.maybe_spawn_boss();
    }

    /// Spawn any quest boss whose quest is active on this map.
    fn maybe_spawn_boss(&mut self) {
        let bosses = self.map.bosses.clone();
        for b in bosses {
            let active = self.save.quest(b.quest).map(|q| q.state == QuestState::Active).unwrap_or(false);
            if active && !self.enemies.iter().any(|e| e.kind == b.kind) {
                if self.online {
                    self.outbox.push(ClientMsg::Boss { kind: b.kind.index(), map: self.map.key.to_string() });
                } else {
                    self.spawn_enemy(b.kind, b.x, b.y, None);
                }
            }
        }
    }

    /// Pay a quest NPC to call their boss into its arena on this map.
    pub fn summon_boss(&mut self, kind: EnemyKind) -> Result<(), String> {
        let Some(b) = self.map.bosses.iter().find(|b| b.kind == kind).cloned() else {
            return Err("It doesn't live around here".into());
        };
        if self.enemies.iter().any(|e| e.kind == kind && e.state != enemies::EState::Dying) {
            return Err(format!("The {} is already here!", kind.name()));
        }
        if self.save.gold < quests::BOSS_SUMMON_COST {
            return Err(format!("You need {} gold", quests::BOSS_SUMMON_COST));
        }
        self.save.gold -= quests::BOSS_SUMMON_COST;
        if self.online {
            self.outbox.push(ClientMsg::Boss { kind: kind.index(), map: self.map.key.to_string() });
        } else {
            self.spawn_enemy(b.kind, b.x, b.y, None);
        }
        self.toast(&format!("The {} appears!", kind.name()), "Watch for its charge.");
        self.wants_save = true;
        Ok(())
    }

    pub(crate) fn spawn_enemy(&mut self, kind: EnemyKind, x: f32, feet_y: f32, zone: Option<usize>) {
        let id = self.next_id;
        self.next_id += 1;
        let e = Enemy::new(id, kind, x, feet_y, zone, &mut self.rng);
        self.enemies.push(e);
    }

    // ------------------------------------------------------------------ feedback

    pub fn toast(&mut self, title: &str, body: &str) {
        self.toasts.push(Toast { title: title.into(), body: body.into(), life: 4.5 });
    }
    fn float(&mut self, pos: V2, text: String, kind: FloatKind) {
        self.floats.push(FloatText { pos, text, life: 1.0, kind });
    }
    fn burst(&mut self, pos: V2, kind: FxKind, n: usize, speed: f32) {
        for _ in 0..n {
            let a = self.rng.range(0.0, std::f32::consts::TAU);
            let s = self.rng.range(speed * 0.3, speed);
            let life = self.rng.range(0.25, 0.6);
            let size = self.rng.range(2.0, 5.0);
            self.particles.push(Particle { pos, vel: V2::new(a.cos() * s, a.sin() * s), life, max_life: life, kind, size });
        }
    }

    // ------------------------------------------------------------------ main tick

    pub fn tick(&mut self, input: &InputState) {
        let dt = TICK;
        self.time += dt;
        self.play_time_accum += dt;
        if self.play_time_accum >= 1.0 {
            self.play_time_accum -= 1.0;
            self.save.play_seconds += 1;
        }

        // UI-ish things run even during hitstop.
        for t in &mut self.toasts {
            t.life -= dt;
        }
        self.toasts.retain(|t| t.life > 0.0);
        self.shake = (self.shake - dt * 30.0).max(0.0);

        self.update_others(dt);
        for c in &mut self.chat {
            c.2 += dt;
        }
        if input.pressed(Action::QuestLog) && self.dialog.is_none() {
            self.quest_log_open = !self.quest_log_open;
        }

        if self.dialog.is_some() {
            self.update_dialog(input);
            self.update_fx(dt);
            return;
        }

        if self.hitstop > 0.0 {
            self.hitstop -= dt;
            return;
        }

        // The world runs a little slower than real time (see GAME_SPEED).
        let gdt = dt * physics::GAME_SPEED;
        self.update_player(input, gdt);
        self.update_drops(gdt);
        self.update_pet(dt);
        self.update_enemies(gdt);
        self.update_projectiles(gdt);
        self.update_spawns(dt);
        self.update_regions(dt);
        self.update_fx(gdt);
    }

    fn update_fx(&mut self, dt: f32) {
        for p in &mut self.particles {
            p.pos = p.pos + p.vel * dt;
            p.vel.y += 500.0 * dt;
            p.vel = p.vel * (1.0 - 3.0 * dt);
            p.life -= dt;
        }
        self.particles.retain(|p| p.life > 0.0);
        for f in &mut self.floats {
            f.pos.y -= 50.0 * dt;
            f.life -= dt * 0.8;
        }
        self.floats.retain(|f| f.life > 0.0);
    }

    // ------------------------------------------------------------------ dialog

    fn update_dialog(&mut self, input: &InputState) {
        let advance = input.pressed(Action::Interact)
            || input.pressed(Action::Jump)
            || input.pressed(Action::Light);
        let cancel = input.pressed(Action::Menu);
        let mut close = cancel;
        {
            let d = self.dialog.as_mut().unwrap();
            if d.choices().is_some() && (input.pressed(Action::Left) || input.pressed(Action::Right) || input.pressed(Action::Up) || input.pressed(Action::Down)) {
                d.choice = 1 - d.choice;
            }
        }
        if advance && !close {
            let (last, action, choice) = {
                let d = self.dialog.as_ref().unwrap();
                (d.on_last_line(), d.dialog.action.clone(), d.choice)
            };
            if !last {
                let d = self.dialog.as_mut().unwrap();
                d.line += 1;
                d.choice = 0;
                d.message = None;
            } else {
                match action {
                    DialogAction::OfferQuest(id) => {
                        if choice == 0 {
                            quests::accept(&mut self.save, id);
                            if let Some(def) = quests::quest_def(id) {
                                self.toast("Quest accepted", def.name);
                                // a boss quest calls its boss right away
                                self.maybe_spawn_boss();
                            }
                            self.wants_save = true;
                        }
                        close = true;
                    }
                    DialogAction::CompleteQuest(id) => {
                        if let Some(r) = quests::complete(&mut self.save, id) {
                            let body = format!(
                                "+{} EXP  +{} gold{}",
                                r.exp,
                                r.gold,
                                if r.potions > 0 { format!("  +{} Red Potions", r.potions) } else { String::new() }
                            );
                            self.toast(&format!("Quest complete: {}", r.quest_name), &body);
                            self.gain_exp(r.exp);
                            if let Some(n) = r.chained {
                                self.toast("New quest", n);
                            }
                            self.wants_save = true;
                        }
                        close = true;
                    }
                    DialogAction::Shop(shop) => {
                        if choice == 0 {
                            self.shop_open = Some(shop);
                        }
                        close = true;
                    }
                    DialogAction::Waystone => {
                        if choice == 0 {
                            self.teleport_open = true;
                        }
                        close = true;
                    }
                    DialogAction::ClassChange(class) => {
                        if choice == 0 {
                            if let Err(e) = self.change_class(class) {
                                self.info_float(&e);
                            }
                        }
                        close = true;
                    }
                    DialogAction::Heal => {
                        self.heal_full();
                        close = true;
                    }
                    DialogAction::SummonBoss(kind) => {
                        if choice == 0 {
                            if let Err(e) = self.summon_boss(kind) {
                                self.info_float(&e);
                            }
                        }
                        close = true;
                    }
                    DialogAction::None => close = true,
                }
            }
        }
        if close {
            let was_mira = self.dialog.as_ref().map(|d| d.dialog.npc_id == "mira").unwrap_or(false);
            self.dialog = None;
            if was_mira {
                self.heal_full();
            }
        }
    }

    fn heal_full(&mut self) {
        if self.player.hp < self.player.max_hp || self.player.mp < self.player.max_mp {
            self.player.hp = self.player.max_hp;
            self.player.mp = self.player.max_mp;
            let c = self.player.body.center();
            self.burst(c, FxKind::Heal, 18, 160.0);
            self.float(c, "Fully healed!".into(), FloatKind::Heal);
        }
    }

    // ------------------------------------------------------------------ player

    fn update_player(&mut self, input: &InputState, dt: f32) {
        let solids = self.solids_with_gates();
        let lvl = Level { solids: &solids, platforms: &self.map.platforms, bounds: self.map.bounds() };
        let tap_up = self.settings.tap_up_to_jump;
        let p = &mut self.player;
        p.anim_t += dt;
        p.state_t += dt;
        p.invuln = (p.invuln - dt).max(0.0);
        p.hurt_flash = (p.hurt_flash - dt).max(0.0);
        p.dodge_cd = (p.dodge_cd - dt).max(0.0);
        p.potion_cd = (p.potion_cd - dt).max(0.0);
        p.land_squash = (p.land_squash - dt * 6.0).max(0.0);
        p.since_hurt += dt;

        // Ability cooldowns, buffs and regen run on real seconds (they're
        // shown as seconds), not the slowed-down world clock.
        let real = dt / physics::GAME_SPEED;
        for c in &mut p.cooldowns {
            c.1 -= real;
        }
        p.cooldowns.retain(|c| c.1 > 0.0);
        let before = p.buffs.len();
        for b in &mut p.buffs {
            b.left -= real;
        }
        p.buffs.retain(|b| b.left > 0.0);
        let buffs_expired = p.buffs.len() != before;

        // Regen: MP always, HP out of combat (never while defeated).
        if p.state != PState::Dead {
            p.mp = (p.mp + p.stats.mp_regen * real).min(p.max_mp);
        }
        if p.since_hurt > 6.0 && p.state != PState::Dead {
            p.hp = (p.hp + p.max_hp * 0.015 * real).min(p.max_hp);
        }

        if buffs_expired {
            let d = derive_stats(&self.save, &p.buffs);
            p.stats = d;
            p.max_hp = d.max_hp;
            p.max_mp = d.max_mp;
            p.hp = p.hp.min(p.max_hp);
            p.mp = p.mp.min(p.max_mp);
        }
        if p.state == PState::Dead {
            // Lies there as a tombstone until Respawn, a revive or a Revive Scroll.
            self.update_dead(input, dt);
            return;
        }
        self.pickup_cd = (self.pickup_cd - dt).max(0.0);
        let p = &mut self.player;

        let ax = input.axis_x();
        let down = input.held(Action::Down);
        let jump_pressed = input.pressed(Action::Jump) || (tap_up && input.pressed(Action::Up));
        let jump_held = input.held(Action::Jump) || (tap_up && input.held(Action::Up));

        if jump_pressed {
            p.jump_buffer = JUMP_BUFFER;
        } else {
            p.jump_buffer = (p.jump_buffer - dt).max(0.0);
        }
        if input.pressed(Action::Light) {
            p.attack_buffer = Some((false, ATTACK_BUFFER));
        } else if input.pressed(Action::Heavy) {
            p.attack_buffer = Some((true, ATTACK_BUFFER));
        } else if let Some((h, t)) = p.attack_buffer {
            p.attack_buffer = if t - dt > 0.0 { Some((h, t - dt)) } else { None };
        }

        let grounded = p.body.on_ground;
        if grounded {
            p.coyote = COYOTE;
            p.air_jumps_left = p.info.air_jumps;
            p.air_dodge_used = false;
            p.recovery_used = false;
        } else {
            p.coyote = (p.coyote - dt).max(0.0);
        }

        let run = p.info.run_speed * p.stats.speed_mult;
        let mut events: Vec<QuestEvent<'static>> = Vec::new();
        let mut wants_dodge = input.pressed(Action::Dodge);
        let mut start_attack: Option<bool> = None;

        // Grab a ladder / rope / vine.
        if p.state == PState::Normal && (input.held(Action::Up) || down) && !(down && jump_pressed) {
            if let Some(ci) = find_climb(&self.map.climbs, &p.body, input.held(Action::Up)) {
                let c = &self.map.climbs[ci];
                p.state = PState::Climb;
                p.state_t = 0.0;
                p.climb = ci;
                p.body.vel = V2::ZERO;
                p.body.on_ground = false;
                p.jump_buffer = 0.0;
                p.attack_buffer = None;
                if p.body.feet().y <= c.top + 8.0 {
                    p.body.pos.y = c.top - p.body.h + 10.0;
                }
            }
        }
        let jump_pressed = jump_pressed && p.state != PState::Climb || (p.state == PState::Climb && input.pressed(Action::Jump));

        match p.state {
            PState::Climb => {
                wants_dodge = false;
                let c = self.map.climbs[p.climb].clone();
                let dir = input.held(Action::Down) as i32 as f32 - input.held(Action::Up) as i32 as f32;
                p.body.vel = V2::ZERO;
                let tx = c.x - p.body.w * 0.5;
                p.body.pos.x = approach(p.body.pos.x, tx, 600.0 * dt);
                p.body.pos.y += dir * CLIMB_SPEED * dt;
                if dir != 0.0 {
                    p.climb_t += dt;
                }
                let feet = p.body.pos.y + p.body.h;
                if jump_pressed {
                    p.state = PState::Normal;
                    p.state_t = 0.0;
                    p.body.vel = V2::new(ax * run * 0.8, -p.info.jump_speed * physics::PLAYER_JUMP * 0.7);
                    p.air_jumps_left = p.info.air_jumps;
                    p.air_dodge_used = false;
                    if ax != 0.0 {
                        p.facing = ax.signum();
                    }
                } else if feet <= c.top {
                    p.body.pos.y = c.top - p.body.h;
                    p.state = PState::Normal;
                    p.state_t = 0.0;
                    p.body.on_ground = true;
                } else if feet >= c.bottom && dir > 0.0 {
                    p.body.pos.y = c.bottom - p.body.h;
                    p.state = PState::Normal;
                    p.state_t = 0.0;
                    p.body.on_ground = true;
                }
            }
            PState::Normal => {
                // Horizontal movement
                let target = ax * run;
                let accel = if grounded { 2600.0 } else { 1700.0 };
                p.body.vel.x = approach(p.body.vel.x, target, accel * dt);
                if ax != 0.0 {
                    p.facing = ax.signum();
                }
                // Jumping / dropping through platforms
                if p.jump_buffer > 0.0 {
                    let on_platform = grounded && !physics::standing_on(&Body { drop_timer: 0.0, ..p.body.clone() }, &Level { solids: &solids, platforms: &[], bounds: lvl.bounds });
                    if down && on_platform {
                        p.body.drop_timer = 0.22;
                        p.body.on_ground = false;
                        p.body.pos.y += 2.0;
                        p.jump_buffer = 0.0;
                    } else if grounded || p.coyote > 0.0 {
                        p.body.vel.y = -p.info.jump_speed * physics::PLAYER_JUMP;
                        p.body.on_ground = false;
                        p.coyote = 0.0;
                        p.jump_buffer = 0.0;
                        p.jumped_from_ground = true;
                        let f = p.body.feet();
                        self.particles.push(Particle { pos: f, vel: V2::new(-60.0, -20.0), life: 0.3, max_life: 0.3, kind: FxKind::Dust, size: 4.0 });
                        self.particles.push(Particle { pos: f, vel: V2::new(60.0, -20.0), life: 0.3, max_life: 0.3, kind: FxKind::Dust, size: 4.0 });
                    } else if p.air_jumps_left > 0 {
                        p.air_jumps_left -= 1;
                        p.body.vel.y = -p.info.jump_speed * physics::PLAYER_JUMP * 0.92;
                        p.jump_buffer = 0.0;
                        let f = p.body.feet();
                        for i in 0..6 {
                            let a = i as f32 / 6.0 * std::f32::consts::TAU;
                            self.particles.push(Particle { pos: f, vel: V2::new(a.cos() * 120.0, a.sin() * 40.0 + 40.0), life: 0.35, max_life: 0.35, kind: FxKind::Spark, size: 3.0 });
                        }
                    }
                }
                // Variable jump height
                let cut = -250.0 * physics::PLAYER_LIFT;
                if !jump_held && p.body.vel.y < cut && p.jumped_from_ground {
                    p.body.vel.y = cut;
                }
                if let Some((heavy, _)) = p.attack_buffer {
                    start_attack = Some(heavy);
                }
            }
            PState::Attack => {
                // allow buffered dodge to cancel recovery? Only after hit lands (Brawlhalla dodge-cancel feel is mostly from true combos)
                wants_dodge = false;
                let friction = if grounded { 1900.0 } else { 520.0 };
                let a = p.attack.as_ref().unwrap();
                let lunging = a.def.lunge.is_some() && a.phase() == AttackPhase::Active;
                if !lunging {
                    let target = if grounded { 0.0 } else { ax * run * 0.6 };
                    p.body.vel.x = approach(p.body.vel.x, target, friction * dt);
                }
            }
            PState::Dodge => {
                wants_dodge = false;
                if p.state_t >= DODGE_TIME {
                    p.state = PState::Normal;
                    p.state_t = 0.0;
                    p.body.vel = p.body.vel * 0.35;
                } else if grounded && p.dodge_dir.x != 0.0 {
                    p.body.vel.x = p.dodge_dir.x * DODGE_SPEED * (1.0 - p.state_t / DODGE_TIME).max(0.25);
                } else if !grounded {
                    let k = (1.0 - p.state_t / DODGE_TIME).max(0.0);
                    p.body.vel.x = p.dodge_dir.x * DODGE_SPEED * k;
                    p.body.vel.y = p.dodge_dir.y * DODGE_SPEED * k;
                } else {
                    p.body.vel.x = approach(p.body.vel.x, 0.0, 4000.0 * dt);
                }
            }
            PState::Hitstun => {
                wants_dodge = false;
                p.body.vel.x = approach(p.body.vel.x, 0.0, 900.0 * dt);
                if p.state_t >= 0.32 {
                    p.state = PState::Normal;
                    p.state_t = 0.0;
                }
            }
            PState::Dead => {}
        }

        // Dodge start
        if wants_dodge && p.dodge_cd <= 0.0 && (grounded || !p.air_dodge_used) {
            let up = input.held(Action::Up);
            let mut dir = V2::new(ax, if up { -1.0 } else if down { 1.0 } else { 0.0 });
            if !grounded {
                if dir.x == 0.0 && dir.y == 0.0 {
                    dir = V2::ZERO;
                }
                let l = dir.len();
                if l > 0.0 {
                    dir = dir * (1.0 / l);
                }
                p.air_dodge_used = true;
            } else {
                dir.y = 0.0;
            }
            p.dodge_dir = dir;
            p.state = PState::Dodge;
            p.state_t = 0.0;
            p.attack = None;
            p.attack_buffer = None;
            p.invuln = p.invuln.max(DODGE_IFRAMES);
            p.dodge_cd = if grounded { DODGE_COOLDOWN } else { DODGE_COOLDOWN * 0.5 };
            if !grounded && dir.len() == 0.0 {
                p.body.vel = V2::new(p.body.vel.x * 0.3, p.body.vel.y.min(0.0) * 0.3);
            }
            events.push(QuestEvent::Dodged);
            let c = p.body.center();
            for _ in 0..5 {
                self.particles.push(Particle { pos: c, vel: V2::new(-dir.x * 120.0, -dir.y * 120.0), life: 0.25, max_life: 0.25, kind: FxKind::Spark, size: 6.0 });
            }
            start_attack = None;
        }

        // Attack start
        if let Some(heavy) = start_attack {
            let p = &mut self.player;
            let grounded = p.body.on_ground;
            let side = ax != 0.0;
            let kind = AttackKind::select(heavy, grounded, side, down);
            let def = classes::attack_def(p.class, kind);
            p.attack_buffer = None;
            if kind == AttackKind::AirHeavyNeutral && p.recovery_used {
                // One recovery per airtime.
            } else if p.mp < def.mp_cost {
                let c = p.body.center();
                self.float(V2::new(c.x, c.y - 50.0), "Not enough MP".into(), FloatKind::Info);
            } else {
                let p = &mut self.player;
                p.mp -= def.mp_cost;
                if side {
                    p.facing = ax.signum();
                }
                if kind == AttackKind::AirHeavyNeutral {
                    p.recovery_used = true;
                }
                p.state = PState::Attack;
                p.state_t = 0.0;
                p.attack = Some(AttackState { kind, def, t: 0.0, fired: false, hit_ids: Vec::new(), skill: None, wave: 0 });
            }
        }

        // Hotbar (abilities and consumables)
        for (i, a) in HOTBAR_ACTIONS.iter().enumerate() {
            if input.pressed(*a) {
                self.use_hotbar(i);
            }
        }

        // Attack progression
        if self.player.state == PState::Attack {
            self.step_attack(dt);
        }

        // Physics
        let p = &mut self.player;
        let fast_fall = down && !p.body.on_ground && p.body.vel.y > 0.0 && p.state == PState::Normal;
        let gscale = match p.state {
            PState::Dodge if !p.body.on_ground => 0.0,
            PState::Attack => {
                let a = p.attack.as_ref().unwrap();
                if a.def.lunge.map(|l| l.1 > 0.0).unwrap_or(false) && a.phase() == AttackPhase::Active {
                    0.0
                } else if !p.body.on_ground && a.phase() == AttackPhase::Startup && !a.kind.is_heavy() {
                    0.5 // slight hang time when attacking in the air
                } else {
                    1.0
                }
            }
            _ => 1.0,
        };
        let was_ground = p.body.on_ground;
        let vy_before = p.body.vel.y;
        if p.state != PState::Climb {
            physics::apply_gravity(&mut p.body, dt, fast_fall, gscale * physics::PLAYER_GRAVITY);
            let lvl = Level { solids: &solids, platforms: &self.map.platforms, bounds: self.map.bounds() };
            physics::move_and_collide(&mut p.body, &lvl, dt);
        }
        if p.body.on_ground && p.state != PState::Dead {
            self.last_ground = p.body.feet();
        }
        if p.body.on_ground && !was_ground {
            p.jumped_from_ground = false;
            if vy_before > 400.0 {
                p.land_squash = 1.0;
                let f = p.body.feet();
                for s in [-1.0f32, 1.0] {
                    self.particles.push(Particle { pos: f, vel: V2::new(s * 90.0, -30.0), life: 0.3, max_life: 0.3, kind: FxKind::Dust, size: 5.0 });
                }
            }
            // Land-cancel dives
            if let Some(a) = &mut p.attack {
                if a.def.ends_on_land && a.phase() == AttackPhase::Active {
                    a.t = a.def.startup + a.def.active;
                    self.shake = self.shake.max(4.0);
                }
            }
        }

        // Potion
        if input.pressed(Action::Potion) {
            self.drink_potion();
        }

        // Pick up loot: press to grab the nearest drop, hold to keep grabbing.
        if input.pressed(Action::Pickup) || (input.held(Action::Pickup) && self.pickup_cd <= 0.0) {
            self.pick_up();
            self.pickup_cd = PICKUP_REPEAT;
        }

        // Fell out of the world
        if self.player.body.pos.y > self.map.kill_y {
            self.player.hp = 0.0;
            self.player_died();
        }

        // Interact / portals
        if input.pressed(Action::Interact) || input.pressed(Action::Up) {
            self.try_interact(input.pressed(Action::Interact));
        }

        // Contact damage from enemies
        self.enemy_contacts();

        for e in events {
            self.quest_event(e);
        }
    }

    fn step_attack(&mut self, dt: f32) {
        let p = &mut self.player;
        let Some(a) = p.attack.as_mut() else { return };
        let prev_t = a.t;
        a.t += dt;
        let def = a.def.clone();
        let kind = a.kind;
        // Active window just started
        if prev_t < def.startup && a.t >= def.startup && !a.fired {
            a.fired = true;
            if let Some((lx, ly)) = def.lunge {
                p.body.vel.x = lx * p.facing;
                if ly != 0.0 {
                    p.body.vel.y = if ly < 0.0 { ly * physics::PLAYER_LIFT } else { ly };
                    if ly < 0.0 {
                        p.body.on_ground = false;
                    }
                }
            }
            if let Some(pd) = &def.projectile {
                let c = p.body.center();
                let (dmg, kbx, kby, hs, crit_bonus) = match a.skill.map(|s| s.effect) {
                    Some(SkillEffect::Strike { damage, crit_bonus, .. }) => (damage, def.kbx, def.kby, def.hitstun, crit_bonus),
                    _ => {
                        let (d, x, y, h) = classes::projectile_damage(p.class, kind);
                        (d, x, y, h, 0.0)
                    }
                };
                let mult = p.info.power * p.stats.attack_mult(p.info.magic);
                let n = pd.count.max(1);
                for i in 0..n {
                    let off = if n > 1 { (i as f32 / (n - 1) as f32 - 0.5) * pd.spread } else { 0.0 };
                    let spawn = V2::new(c.x + p.facing * 22.0, c.y - 4.0);
                    self.projectiles.push(Projectile {
                        team: Team::Player,
                        pos: spawn,
                        vel: V2::new(pd.vx * p.facing + if pd.style == ProjStyle::Caltrop { off } else { 0.0 } + if pd.style == ProjStyle::Caltrop { (i as f32 - 1.0) * 120.0 * p.facing } else { 0.0 }, pd.vy + if pd.style == ProjStyle::Caltrop { 0.0 } else { off }),
                        w: pd.w,
                        h: pd.h,
                        gravity: if pd.style == ProjStyle::Caltrop { 1260.0 } else { pd.gravity },
                        life: pd.life,
                        max_life: pd.life,
                        pierce: pd.pierce,
                        damage: dmg * mult,
                        kb: V2::new(kbx, kby),
                        hitstun: hs,
                        style: pd.style,
                        hit_ids: Vec::new(),
                        grounded: false,
                        heavy: kind.is_heavy() || a.skill.is_some(),
                        crit_bonus,
                        magic: p.info.magic,
                        travelled: 0.0,
                        range: max_range(pd.style, Team::Player),
                    });
                }
            }
            if def.damage > 0.0 && kind.is_heavy() {
                self.shake = self.shake.max(2.0);
            }
        }
        // Multi-hit abilities: each wave can hit the same enemies again.
        {
            let a = self.player.attack.as_mut().unwrap();
            if let Some(SkillEffect::Strike { hits, .. }) = a.skill.map(|s| s.effect) {
                if hits > 1 && a.phase() == AttackPhase::Active {
                    let wave = (((a.t - def.startup) / def.active) * hits as f32).floor().clamp(0.0, hits as f32 - 1.0) as u8;
                    if wave != a.wave {
                        a.wave = wave;
                        a.hit_ids.clear();
                    }
                }
            }
        }
        // Active hitbox
        let p = &self.player;
        let a = p.attack.as_ref().unwrap();
        let (crit_bonus, from_skill) = match a.skill.map(|s| s.effect) {
            Some(SkillEffect::Strike { crit_bonus, .. }) => (crit_bonus, true),
            _ => (0.0, false),
        };
        if a.phase() == AttackPhase::Active && def.damage > 0.0 {
            let hb = attack_hitbox(p, &def);
            let mut hits: Vec<(usize, f32)> = Vec::new();
            for (i, e) in self.enemies.iter().enumerate() {
                if e.state == EState::Dying || a.hit_ids.contains(&e.id) {
                    continue;
                }
                if hb.overlaps(&e.body.aabb()) {
                    let dir = if def.both_sides { (e.body.center().x - p.body.center().x).signum() } else { p.facing };
                    hits.push((i, if dir == 0.0 { p.facing } else { dir }));
                }
            }
            let mult = p.info.power * p.stats.attack_mult(p.info.magic);
            for (i, dir) in hits {
                let id = self.enemies[i].id;
                self.player.attack.as_mut().unwrap().hit_ids.push(id);
                self.damage_enemy_crit(i, def.damage * mult, V2::new(def.kbx * dir, def.kby), def.hitstun, kind.is_heavy() || from_skill, crit_bonus);
                if !from_skill {
                    self.player.mp = (self.player.mp + 1.5).min(self.player.max_mp);
                }
            }
        }
        let p = &mut self.player;
        let a = p.attack.as_ref().unwrap();
        if a.t >= def.total() {
            p.attack = None;
            p.state = PState::Normal;
            p.state_t = 0.0;
        }
    }

    #[allow(dead_code)]
    fn damage_enemy(&mut self, i: usize, base: f32, kb: V2, hitstun: f32, heavy: bool) {
        self.damage_enemy_crit(i, base, kb, hitstun, heavy, 0.0);
    }

    fn damage_enemy_crit(&mut self, i: usize, base: f32, kb: V2, hitstun: f32, heavy: bool, crit_bonus: f32) {
        let st = self.player.stats;
        let crit = self.rng.f() * 100.0 < (st.crit_chance + crit_bonus).min(100.0);
        let var = self.rng.range(0.9, 1.1);
        let dmg = (base * var * if crit { st.crit_damage / 100.0 } else { 1.0 }).round().max(1.0);
        let e = &mut self.enemies[i];
        let center = e.body.center();
        let kind = e.kind;
        if kind == EnemyKind::TrainingDummy {
            e.wobble = 1.0;
            e.flash = 0.12;
            e.facing = -kb.x.signum();
        } else if self.online {
            // The server owns its HP: show the hit now, report it, and let
            // the next snapshot carry the result.
            e.flash = 0.12;
            let id = e.id;
            self.outbox.push(ClientMsg::Hit { enemy: id, damage: dmg, kbx: kb.x, kby: kb.y, hitstun });
        } else {
            enemies::apply_hit(e, dmg, kb, hitstun);
        }
        self.hitstop = if heavy { 0.07 } else { 0.035 };
        self.shake = self.shake.max(if heavy { 5.0 } else { 2.5 });
        self.burst(center, if crit { FxKind::Crit } else { FxKind::Hit }, if heavy { 14 } else { 8 }, 340.0);
        if self.settings.show_damage_numbers {
            self.float(V2::new(center.x, center.y - 30.0), format!("{}", dmg as i32), if crit { FloatKind::Crit } else { FloatKind::Damage });
        }
        if kind == EnemyKind::TrainingDummy {
            self.quest_event(QuestEvent::DummyHit { heavy });
        } else if !self.online && self.enemies[i].hp <= 0.0 {
            self.kill_enemy(i);
        }
    }

    pub(crate) fn kill_enemy(&mut self, i: usize) {
        let e = &mut self.enemies[i];
        if e.state == EState::Dying {
            return;
        }
        e.state = EState::Dying;
        e.state_t = 0.0;
        let kind = e.kind;
        let c = e.body.center();
        let def = kind.def();
        let (exp, gold) = (def.exp, def.gold);
        self.burst(c, FxKind::Spark, 20, 260.0);
        self.float(V2::new(c.x, c.y - 50.0), format!("+{} EXP", exp), FloatKind::Exp);
        // Gold and loot spill onto the ground: pick them up with the Pickup key.
        if gold > 0 {
            self.spawn_drop(DropKind::Gold, gold, c, 70.0);
        }
        for &(id, chance) in kind.drops() {
            if self.rng.f() < chance {
                if let Some(def) = items::item(id) {
                    self.spawn_drop(DropKind::Item(def.id), 1, c, 170.0);
                }
            }
        }
        self.gain_exp(exp);
        self.quest_event(QuestEvent::Killed(kind));
        if def.boss {
            self.toast(&format!("{} defeated!", def.name), "Return to turn in your quest.");
            self.shake = 10.0;
            self.wants_save = true;
        }
    }

    fn gain_exp(&mut self, amount: u32) {
        self.save.exp += amount;
        let mut gained = 0u32;
        while self.save.exp >= exp_to_next(self.save.level) {
            self.save.exp -= exp_to_next(self.save.level);
            self.save.level += 1;
            gained += 1;
        }
        let leveled = gained > 0;
        if leveled {
            let old_level = self.save.level - gained;
            self.save.ap += gained * stats::AP_PER_LEVEL;
            self.recompute_stats();
            self.player.hp = self.player.max_hp;
            self.player.mp = self.player.max_mp;
            let c = self.player.body.center();
            self.burst(c, FxKind::LevelUp, 40, 300.0);
            self.float(V2::new(c.x, c.y - 70.0), format!("LEVEL UP! Lv.{}", self.save.level), FloatKind::Heal);
            self.toast(
                &format!("Level {}!", self.save.level),
                &format!("+{} ability points to spend (Character window). HP and Mana restored.", gained * stats::AP_PER_LEVEL),
            );
            self.learn_new_skills(old_level);
            if self.save.level >= classes::CLASS_CHANGE_LEVEL && old_level < classes::CLASS_CHANGE_LEVEL && self.save.class == ClassId::Novice {
                self.toast("Class change available!", "Visit Aurel in Breezehollow, then head west to the class towns.");
            }
            self.wants_save = true;
        }
    }

    fn quest_event(&mut self, ev: QuestEvent) {
        let ready = quests::on_event(&mut self.save, ev);
        for name in ready {
            self.toast("Objective complete", &format!("{} - return to turn it in.", name));
            self.wants_save = true;
        }
    }

    /// The Potion key drinks the strongest HP potion you carry.
    fn drink_potion(&mut self) {
        let best = ["hp_potion_big", "trainee_hp_potion", "hp_potion", "elixir"].into_iter().find(|id| self.save.inventory.count(id) > 0);
        match best {
            Some(id) => {
                self.use_item(id);
            }
            None => {
                let c = self.player.body.center();
                self.float(V2::new(c.x, c.y - 50.0), "No HP potions!".into(), FloatKind::Info);
            }
        }
    }

    // ------------------------------------------------------------------ stats, items, abilities

    /// Recompute derived stats after a change to level, main stats, gear or buffs.
    pub fn recompute_stats(&mut self) {
        let d = derive_stats(&self.save, &self.player.buffs);
        let p = &mut self.player;
        p.stats = d;
        p.max_hp = d.max_hp;
        p.max_mp = d.max_mp;
        p.hp = p.hp.min(p.max_hp);
        p.mp = p.mp.min(p.max_mp);
    }

    /// Spend one ability point on a main stat.
    pub fn spend_ap(&mut self, stat: MainStat) -> bool {
        if self.save.ap == 0 {
            return false;
        }
        self.save.ap -= 1;
        self.save.stats.add(stat, 1);
        let (old_hp, old_mp) = (self.player.max_hp, self.player.max_mp);
        self.recompute_stats();
        // Grow current HP/MP along with the max.
        self.player.hp += (self.player.max_hp - old_hp).max(0.0);
        self.player.mp += (self.player.max_mp - old_mp).max(0.0);
        self.wants_save = true;
        true
    }

    /// Spend all unspent points the way the class prefers.
    pub fn auto_spend_ap(&mut self) {
        let ap = self.save.ap;
        if ap == 0 {
            return;
        }
        self.save.ap = 0;
        self.save.stats = stats::auto_assign(self.save.class, self.save.stats, ap);
        self.recompute_stats();
        self.wants_save = true;
    }

    /// Novice -> class at level 10. Refunds ability points and hands over the class weapon.
    pub fn change_class(&mut self, class: ClassId) -> Result<(), String> {
        if self.save.class != ClassId::Novice {
            return Err("You have already chosen a class.".into());
        }
        if self.save.level < classes::CLASS_CHANGE_LEVEL {
            return Err(format!("Reach level {} first.", classes::CLASS_CHANGE_LEVEL));
        }
        if class == ClassId::Novice {
            return Err("Pick a class.".into());
        }
        self.save.class = class;
        // Refund every point so the build can be remade around the new main stats.
        let spent = (self.save.stats.total() - stats::START_STATS.total()).max(0) as u32;
        self.save.stats = stats::START_STATS;
        self.save.ap += spent;
        // Hand over the class weapon; the old Novice weapon goes to the bag.
        let wid = items::starter_weapon(class);
        let slot = EquipSlot::Weapon.index();
        if let Some(old) = self.save.inventory.equip[slot].take() {
            self.save.inventory.add(&old, 1);
        }
        self.save.inventory.equip[slot] = Some(wid.to_string());
        let info = class.info();
        self.player.class = class;
        self.player.info = info.clone();
        self.player.attack = None;
        self.player.state = PState::Normal;
        self.recompute_stats();
        self.player.hp = self.player.max_hp;
        self.player.mp = self.player.max_mp;
        let c = self.player.body.center();
        self.burst(c, FxKind::LevelUp, 60, 340.0);
        self.float(V2::new(c.x, c.y - 70.0), format!("You are now a {}!", info.name), FloatKind::Heal);
        self.toast(
            &format!("Class changed: {}", info.name),
            &format!("{} ability points refunded. Main stats: {} and {}.", self.save.ap, info.primary.name(), info.secondary.name()),
        );
        self.learn_new_skills(0);
        // A Skill Reset Coupon, so the new build can be redone later.
        if self.save.inventory.add("skill_reset_coupon", 1) > 0 {
            let f = self.player.body.center();
            self.spawn_drop(DropKind::Item("skill_reset_coupon"), 1, f, 40.0);
        }
        self.toast("Skill Reset Coupon received", "Use it from your bag any time to refund every ability point you've spent.");
        self.quest_event(QuestEvent::ChangedClass);
        self.look_changed = true;
        self.wants_save = true;
        Ok(())
    }

    /// Skill Reset Coupon: refund every spent ability point.
    pub fn reset_ability_points(&mut self) -> Result<u32, String> {
        if self.player.state == PState::Dead {
            return Err("You can't do that while defeated".into());
        }
        if self.save.inventory.count("skill_reset_coupon") == 0 {
            return Err("You need a Skill Reset Coupon".into());
        }
        let spent = (self.save.stats.total() - stats::START_STATS.total()).max(0) as u32;
        if spent == 0 {
            return Err("You haven't spent any ability points".into());
        }
        self.save.inventory.remove("skill_reset_coupon", 1);
        self.save.stats = stats::START_STATS;
        self.save.ap += spent;
        self.recompute_stats();
        let c = self.player.body.center();
        self.burst(c, FxKind::LevelUp, 30, 260.0);
        self.toast("Ability points reset", &format!("{} points refunded. Spend them in the Character window.", spent));
        self.wants_save = true;
        Ok(spent)
    }

    /// Abilities the character can use right now.
    pub fn known_skills(&self) -> Vec<&'static SkillDef> {
        skills::class_skills(self.save.class).into_iter().filter(|s| skills::knows(self.save.class, self.save.level, s)).collect()
    }

    /// Announce skills learned since `old_level` and put them in empty hotbar slots.
    fn learn_new_skills(&mut self, old_level: u32) {
        let class = self.save.class;
        let new: Vec<&'static SkillDef> = skills::class_skills(class)
            .into_iter()
            .filter(|s| skills::knows(class, self.save.level, s) && (s.level > old_level || (old_level == 0 && s.class == class)))
            .collect();
        for s in new {
            let on_bar = self.save.hotbar.iter().any(|h| *h == HotbarSlot::Skill(s.id.to_string()));
            if !on_bar {
                if let Some(i) = self.save.hotbar.iter().position(|h| *h == HotbarSlot::Empty) {
                    self.save.hotbar[i] = HotbarSlot::Skill(s.id.to_string());
                }
            }
            self.toast("New ability!", &format!("{} - {}", s.name, s.desc));
        }
    }

    pub fn set_hotbar(&mut self, i: usize, slot: HotbarSlot) {
        if i < HOTBAR_SLOTS {
            if self.save.hotbar.len() < HOTBAR_SLOTS {
                self.save.hotbar.resize(HOTBAR_SLOTS, HotbarSlot::Empty);
            }
            // Only one copy of an entry on the bar.
            if slot != HotbarSlot::Empty {
                for h in self.save.hotbar.iter_mut() {
                    if *h == slot {
                        *h = HotbarSlot::Empty;
                    }
                }
            }
            self.save.hotbar[i] = slot;
            self.wants_save = true;
        }
    }

    pub fn swap_hotbar(&mut self, a: usize, b: usize) {
        if a < self.save.hotbar.len() && b < self.save.hotbar.len() {
            self.save.hotbar.swap(a, b);
            self.wants_save = true;
        }
    }

    pub fn use_hotbar(&mut self, i: usize) {
        match self.save.hotbar.get(i).cloned() {
            Some(HotbarSlot::Skill(id)) => {
                if let Some(s) = skills::skill(&id) {
                    self.use_skill(s);
                }
            }
            Some(HotbarSlot::Item(id)) => {
                self.use_item(&id);
            }
            _ => {}
        }
    }

    fn info_float(&mut self, text: &str) {
        let c = self.player.body.center();
        self.float(V2::new(c.x, c.y - 50.0), text.to_string(), FloatKind::Info);
    }

    /// Use an ability. Returns true if it went off.
    pub fn use_skill(&mut self, s: &'static SkillDef) -> bool {
        let class = self.save.class;
        if !skills::knows(class, self.save.level, s) {
            self.info_float(&format!("Learn at Lv.{}", s.level));
            return false;
        }
        let p = &self.player;
        if p.state == PState::Dead {
            return false;
        }
        if p.cooldown(s.id) > 0.0 {
            self.info_float("Not ready yet");
            return false;
        }
        if p.mp < s.mp {
            self.info_float("Not enough Mana");
            return false;
        }
        let busy = matches!(p.state, PState::Hitstun | PState::Dodge | PState::Climb) || (p.state == PState::Attack && p.attack.as_ref().map(|a| a.phase() != AttackPhase::Recovery).unwrap_or(false));
        if busy {
            return false;
        }
        let c = p.body.center();
        match s.effect {
            SkillEffect::Strike { anim, .. } => {
                let Some(def) = skills::skill_attack(class, s) else { return false };
                let p = &mut self.player;
                p.state = PState::Attack;
                p.state_t = 0.0;
                p.attack_buffer = None;
                p.attack = Some(AttackState { kind: anim, def, t: 0.0, fired: false, hit_ids: Vec::new(), skill: Some(s), wave: 0 });
            }
            SkillEffect::Buff { mods, secs } => {
                let p = &mut self.player;
                p.buffs.retain(|b| b.id != s.id);
                p.buffs.push(Buff { id: s.id, name: s.name, mods, left: secs, total: secs, fx: s.fx });
                self.recompute_stats();
                self.burst(c, FxKind::LevelUp, 24, 220.0);
            }
            SkillEffect::Heal { hp, mp } => {
                let p = &mut self.player;
                let h = (p.max_hp * hp).round();
                p.hp = (p.hp + h).min(p.max_hp);
                p.mp = (p.mp + p.max_mp * mp).min(p.max_mp);
                self.burst(c, FxKind::Heal, 20, 160.0);
                if h > 0.0 {
                    self.float(V2::new(c.x, c.y - 50.0), format!("+{}", h as i32), FloatKind::Heal);
                }
            }
        }
        let p = &mut self.player;
        p.mp -= s.mp;
        p.cooldowns.retain(|c| c.0 != s.id);
        p.cooldowns.push((s.id, s.cooldown));
        self.float(V2::new(c.x, c.y - 64.0), s.name.to_string(), FloatKind::Exp);
        true
    }

    /// Drink / use a consumable by id. Returns true if it was used.
    pub fn use_item(&mut self, id: &str) -> bool {
        let Some(def) = items::item(id) else { return false };
        let effect = match def.kind {
            ItemKind::Consumable(e) => e,
            ItemKind::Special(sp) => return self.use_special(def, sp),
            _ => return false,
        };
        if self.save.inventory.count(id) == 0 {
            self.info_float(&format!("No {} left", def.name));
            return false;
        }
        if self.player.state == PState::Dead {
            return false;
        }
        if self.player.potion_cd > 0.0 {
            return false;
        }
        let c = self.player.body.center();
        match effect {
            Effect::Restore { hp, mp } => {
                let p = &self.player;
                let hp_full = hp == 0.0 || p.hp >= p.max_hp;
                let mp_full = mp == 0.0 || p.mp >= p.max_mp;
                if hp_full && mp_full {
                    self.info_float(if hp > 0.0 && mp == 0.0 { "HP is full" } else if mp > 0.0 && hp == 0.0 { "Mana is full" } else { "Already full" });
                    return false;
                }
                let p = &mut self.player;
                let h = (p.max_hp * hp).round();
                let m = (p.max_mp * mp).round();
                p.hp = (p.hp + h).min(p.max_hp);
                p.mp = (p.mp + m).min(p.max_mp);
                self.burst(c, FxKind::Heal, 14, 140.0);
                let txt = match (h > 0.0, m > 0.0) {
                    (true, true) => format!("+{} HP +{} MP", h as i32, m as i32),
                    (true, false) => format!("+{}", h as i32),
                    _ => format!("+{} MP", m as i32),
                };
                self.float(V2::new(c.x, c.y - 50.0), txt, FloatKind::Heal);
            }
            Effect::Buff { mods, secs } => {
                let p = &mut self.player;
                p.buffs.retain(|b| b.id != def.id);
                p.buffs.push(Buff { id: def.id, name: def.name, mods, left: secs, total: secs, fx: skills::SkillFx::Holy });
                self.recompute_stats();
                self.burst(c, FxKind::LevelUp, 16, 200.0);
                self.float(V2::new(c.x, c.y - 50.0), def.name.to_string(), FloatKind::Heal);
            }
        }
        self.save.inventory.remove(id, 1);
        self.player.potion_cd = POTION_COOLDOWN;
        self.wants_save = true;
        true
    }

    fn use_special(&mut self, def: &'static items::ItemDef, sp: Special) -> bool {
        if self.save.inventory.count(def.id) == 0 {
            self.info_float(&format!("No {} left", def.name));
            return false;
        }
        let r = match sp {
            Special::Revive => self.revive_with_scroll(),
            Special::SkillReset => self.reset_ability_points().map(|_| ()),
            Special::Pet(species) => self.adopt_pet(species),
            Special::PetFood => self.feed_pet().map(|_| ()),
        };
        match r {
            Ok(()) => true,
            Err(e) => {
                self.info_float(&e);
                false
            }
        }
    }

    /// Why this character can't equip the item, if they can't.
    pub fn equip_problem(&self, id: &str) -> Option<String> {
        let def = items::item(id)?;
        def.slot()?;
        if let Some(c) = def.class {
            if c != self.save.class {
                return Some(format!("{} only", c.info().name));
            }
        }
        if self.save.level < def.level {
            return Some(format!("Requires Lv.{}", def.level));
        }
        None
    }

    /// Use or equip the item in an inventory slot.
    pub fn activate_slot(&mut self, idx: usize) {
        let Some(st) = self.save.inventory.slots.get(idx) else { return };
        let id = st.id.clone();
        match items::item(&id).map(|d| d.kind) {
            Some(ItemKind::Consumable(_)) => {
                self.use_item(&id);
            }
            Some(ItemKind::Equip(_)) => {
                if let Err(e) = self.equip_from_slot(idx) {
                    self.info_float(&e);
                }
            }
            Some(ItemKind::Material) => self.info_float("Sell it to any shop"),
            Some(ItemKind::Special(_)) => {
                self.use_item(&id);
            }
            None => {}
        }
    }

    pub fn equip_from_slot(&mut self, idx: usize) -> Result<(), String> {
        let id = self.save.inventory.slots.get(idx).ok_or("No item")?.id.clone();
        if let Some(e) = self.equip_problem(&id) {
            return Err(e);
        }
        let def = items::item(&id).ok_or("Unknown item")?;
        let slot = def.slot().ok_or("Can't equip that")?;
        self.save.inventory.take_slot(idx, 1);
        if let Some(old) = self.save.inventory.equip[slot.index()].replace(id) {
            self.save.inventory.add(&old, 1);
        }
        self.recompute_stats();
        self.look_changed = true;
        self.wants_save = true;
        Ok(())
    }

    pub fn unequip(&mut self, slot: EquipSlot) -> Result<(), String> {
        let Some(id) = self.save.inventory.equip[slot.index()].clone() else { return Ok(()) };
        if !self.save.inventory.room_for(&id, 1) {
            return Err("Your bag is full".into());
        }
        self.save.inventory.equip[slot.index()] = None;
        self.save.inventory.add(&id, 1);
        self.recompute_stats();
        self.look_changed = true;
        self.wants_save = true;
        Ok(())
    }

    /// Waystones you can travel to.
    pub fn attuned_waystones(&self) -> Vec<&'static str> {
        world::WAYSTONES.iter().copied().filter(|k| self.save.has_flag(&format!("ws_{}", k))).collect()
    }

    /// Fast travel to another town's waystone.
    pub fn teleport(&mut self, key: &str) -> Result<(), String> {
        if self.player.state == PState::Dead {
            return Err("You can't travel while defeated".into());
        }
        if !world::WAYSTONES.contains(&key) || !self.save.has_flag(&format!("ws_{}", key)) {
            return Err("You haven't visited that waystone yet".into());
        }
        if self.map.key == key {
            return Err("You're already here".into());
        }
        if self.save.gold < WAYSTONE_COST {
            return Err(format!("Travel costs {} gold", WAYSTONE_COST));
        }
        let m = world::load_map(key).ok_or("Unknown place")?;
        let x = m.npcs.iter().find(|n| n.id == world::waystone_id(key)).map(|n| n.x + 70.0).unwrap_or(m.spawn.0);
        self.save.gold -= WAYSTONE_COST;
        self.change_map(key, x);
        let c = self.player.body.center();
        self.burst(c, FxKind::LevelUp, 40, 260.0);
        Ok(())
    }

    /// Reorder the bag.
    pub fn swap_inventory(&mut self, a: usize, b: usize) {
        let n = self.save.inventory.slots.len();
        if a < n && b < n && a != b {
            self.save.inventory.slots.swap(a, b);
            self.wants_save = true;
        }
    }

    /// Buy from the open shop. Returns the gold spent.
    pub fn buy(&mut self, id: &str, n: u32) -> Result<u32, String> {
        let shop = self.shop_open.and_then(items::shop).ok_or("The shop is closed")?;
        if !shop.stock.contains(&id) {
            return Err("Not sold here".into());
        }
        let discounts_left = items::TRAINEE_DISCOUNT_BUYS.saturating_sub(self.save.trainee_buys);
        let (cost, discounted) = items::purchase_cost(id, n, discounts_left).ok_or("Unknown item")?;
        if self.save.gold < cost {
            return Err("Not enough gold".into());
        }
        if !self.save.inventory.room_for(id, n) {
            return Err("Your bag is full".into());
        }
        self.save.gold -= cost;
        self.save.trainee_buys += discounted;
        self.save.inventory.add(id, n);
        self.wants_save = true;
        Ok(cost)
    }

    /// Price of one more of this item for this character (trainee discounts applied).
    pub fn unit_price(&self, id: &str) -> u32 {
        let left = items::TRAINEE_DISCOUNT_BUYS.saturating_sub(self.save.trainee_buys);
        items::purchase_cost(id, 1, left).map(|c| c.0).unwrap_or(0)
    }

    /// Discounted trainee potions this character can still buy.
    pub fn trainee_discounts_left(&self) -> u32 {
        items::TRAINEE_DISCOUNT_BUYS.saturating_sub(self.save.trainee_buys)
    }

    /// Sell `n` from an inventory slot to the open shop.
    pub fn sell(&mut self, idx: usize, n: u32) -> Result<u32, String> {
        if self.shop_open.is_none() {
            return Err("The shop is closed".into());
        }
        let st = self.save.inventory.slots.get(idx).ok_or("No item")?;
        let def = items::item(&st.id).ok_or("Unknown item")?;
        if def.bound {
            return Err(format!("{} is bound to you and can't be sold", def.name));
        }
        if !def.can_sell() {
            return Err("Can't sell that".into());
        }
        let (_, sold) = self.save.inventory.take_slot(idx, n).ok_or("No item")?;
        let gold = def.sell_price() * sold;
        self.save.gold += gold;
        self.wants_save = true;
        Ok(gold)
    }

    pub fn nearest_npc(&self) -> Option<usize> {
        let c = self.player.body.feet();
        let mut best: Option<(usize, f32)> = None;
        for (i, n) in self.map.npcs.iter().enumerate() {
            let d = (n.x - c.x).abs();
            if d < INTERACT_RANGE && (n.y - c.y).abs() < 80.0 && best.map(|b| d < b.1).unwrap_or(true) {
                best = Some((i, d));
            }
        }
        best.map(|b| b.0)
    }

    pub fn nearby_portal(&self) -> Option<usize> {
        let r = self.player.body.aabb();
        self.map.portals.iter().position(|p| p.area.overlaps(&r))
    }

    pub fn nearby_sign(&self) -> Option<usize> {
        let c = self.player.body.feet();
        self.map.signs.iter().position(|s| (s.x - c.x).abs() < 60.0 && (s.y - c.y).abs() < 60.0)
    }

    fn try_interact(&mut self, interact_key: bool) {
        if self.player.state != PState::Normal || !self.player.body.on_ground {
            return;
        }
        if interact_key {
            if let Some(o) = self.revivable().map(|i| self.others[i].clone()) {
                self.outbox.push(ClientMsg::Revive { target: o.id });
                let c = self.player.body.center();
                self.burst(o.pos, FxKind::Heal, 24, 200.0);
                self.float(V2::new(c.x, c.y - 60.0), format!("Reviving {}...", o.name), FloatKind::Heal);
                return;
            }
        }
        if let Some(i) = self.nearest_npc() {
            if interact_key {
                let npc = self.map.npcs[i].clone();
                // Face each other
                self.player.facing = (npc.x - self.player.body.center().x).signum();
                let mut dialog = quests::talk(npc.id, npc.name, &mut self.save);
                if let DialogAction::SummonBoss(k) = dialog.action {
                    // no point offering while it's already here
                    let here = self.enemies.iter().any(|e| e.kind == k && e.state != enemies::EState::Dying);
                    if here || !self.map.bosses.iter().any(|b| b.kind == k) {
                        dialog.action = DialogAction::None;
                        if dialog.lines.len() > 1 {
                            dialog.lines.pop();
                        }
                    }
                }
                self.dialog = Some(ActiveDialog { dialog, line: 0, choice: 0, message: None });
                self.player.body.vel.x = 0.0;
                self.wants_save = true;
                return;
            }
        }
        if let Some(pi) = self.nearby_portal() {
            let portal = self.map.portals[pi].clone();
            if portal.min_level > self.save.level {
                let c = self.player.body.center();
                self.float(V2::new(c.x, c.y - 60.0), format!("You must be level {} to enter.", portal.min_level), FloatKind::Info);
                return;
            }
            if let Some(f) = portal.requires_flag {
                if !self.save.has_flag(f) {
                    let c = self.player.body.center();
                    self.float(V2::new(c.x, c.y - 60.0), portal.locked_text.to_string(), FloatKind::Info);
                    return;
                }
            }
            self.change_map(portal.target_map, portal.target_x);
        }
    }

    pub fn change_map(&mut self, key: &str, x: f32) {
        let Some(map) = world::load_map(key) else { return };
        // Arrive on solid ground, never on a rooftop platform above it.
        // (ground blocks reach the bottom of the map; floating shelves don't)
        let gy = map.solids.iter().filter(|s| x >= s.left() && x <= s.right() && s.top() > 0.0 && s.bottom() > 850.0).map(|s| s.top()).fold(f32::MAX, f32::min);
        let gy = if gy == f32::MAX { ground_y_at(&map, x, 0.0) } else { gy };
        self.map = map;
        let p = &mut self.player;
        p.body.pos = V2::new(x - PLAYER_W * 0.5, gy - PLAYER_H);
        p.body.vel = V2::ZERO;
        p.body.on_ground = true;
        p.state = PState::Normal;
        p.attack = None;
        p.invuln = 0.8;
        self.save.map = self.map.key.to_string();
        self.shop_open = None;
        self.teleport_open = false;
        // Attune to this map's waystone.
        let key = self.map.key;
        if world::WAYSTONES.contains(&key) {
            let flag = format!("ws_{}", key);
            if !self.save.has_flag(&flag) {
                self.save.set_flag(&flag);
                self.toast("Waystone attuned", "Touch any waystone to travel back here.");
            }
        }
        self.particles.clear();
        self.floats.clear();
        self.drops.clear();
        self.others.clear();
        self.pet = None; // reappears next to you
        self.last_ground = self.player.body.feet();
        self.populate_map();
        self.map_changed = true;
        self.wants_save = true;
        let name = self.map.name;
        self.toast(name, "");
    }

    /// The Respawn button: back to the nearest checkpoint behind you with full
    /// HP and Mana. Any EXP lost on death stays lost.
    pub fn respawn(&mut self) -> bool {
        if self.player.state != PState::Dead {
            return false;
        }
        self.save.tomb = None;
        let (sx, _) = self.map.spawn;
        // Respawn at the furthest checkpoint behind the player.
        let px = self.player.body.pos.x;
        let x = self.map.checkpoints.iter().copied().filter(|&c| c <= px + 1.0).fold(sx, f32::max);
        let key = self.map.key;
        let boss = self.enemies.iter().find(|e| e.kind.is_boss()).map(|e| e.kind.name());
        self.change_map(key, x);
        let p = &mut self.player;
        p.hp = p.max_hp;
        p.mp = p.max_mp;
        p.invuln = 2.0;
        if let Some(name) = boss {
            self.toast("You were defeated...", &format!("The {} regains its strength. Jump or dodge over its charge!", name));
        } else {
            self.toast("Back on your feet", "You return to the last checkpoint.");
        }
        self.wants_save = true;
        true
    }

    /// Read a Revive Scroll while defeated: stand up where you fell and get
    /// back any EXP the defeat cost.
    pub fn revive_with_scroll(&mut self) -> Result<(), String> {
        if self.player.state != PState::Dead {
            return Err("Use it when you've been defeated".into());
        }
        if self.save.inventory.count("revive_scroll") == 0 {
            return Err("You don't have a Revive Scroll".into());
        }
        self.save.inventory.remove("revive_scroll", 1);
        let lost = self.save.tomb.take().unwrap_or(0);
        self.save.exp += lost;
        self.stand_up(0.5);
        let body = if lost > 0 { format!("{} EXP restored.", lost) } else { "No EXP lost.".to_string() };
        self.toast("Revived!", &body);
        Ok(())
    }

    /// Another player revived you (for multiplayer). The EXP loss stands.
    pub fn revive_by_player(&mut self, by: &str) -> bool {
        if self.player.state != PState::Dead {
            return false;
        }
        self.save.tomb = None;
        self.stand_up(0.3);
        self.toast(&format!("{} revived you!", by), "");
        true
    }

    fn stand_up(&mut self, frac: f32) {
        let p = &mut self.player;
        p.state = PState::Normal;
        p.state_t = 0.0;
        p.attack = None;
        p.body.vel = V2::ZERO;
        p.hp = (p.max_hp * frac).max(1.0);
        p.mp = p.mp.max(p.max_mp * frac);
        p.invuln = 3.0;
        p.since_hurt = 0.0;
        let c = p.body.center();
        self.burst(c, FxKind::Heal, 30, 220.0);
        self.burst(c, FxKind::LevelUp, 20, 260.0);
        self.wants_save = true;
    }

    /// While defeated: the body settles, then lies there as a tombstone.
    fn update_dead(&mut self, input: &InputState, dt: f32) {
        let solids = self.solids_with_gates();
        let lvl = Level { solids: &solids, platforms: &self.map.platforms, bounds: self.map.bounds() };
        let p = &mut self.player;
        let fric = if p.body.on_ground { 1400.0 } else { 160.0 };
        p.body.vel.x = approach(p.body.vel.x, 0.0, fric * dt);
        physics::apply_gravity(&mut p.body, dt, false, 1.0);
        physics::move_and_collide(&mut p.body, &lvl, dt);
        if p.body.pos.y > self.map.kill_y {
            // Fell out of the world: the tombstone appears where you last stood.
            let g = self.last_ground;
            p.body.pos = V2::new(g.x - PLAYER_W * 0.5, g.y - PLAYER_H - 2.0);
            p.body.vel = V2::ZERO;
            p.body.on_ground = false;
        }
        // The hotbar still works for Revive Scrolls.
        for (i, a) in HOTBAR_ACTIONS.iter().enumerate() {
            if input.pressed(*a) {
                if let Some(HotbarSlot::Item(id)) = self.save.hotbar.get(i).cloned() {
                    if matches!(items::item(&id).map(|d| d.kind), Some(ItemKind::Special(Special::Revive))) {
                        self.use_item(&id);
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------------ online

    /// Switch to playing on a server. Monsters and loot will come from it.
    pub fn go_online(&mut self, my_id: u32) {
        self.online = true;
        self.my_id = my_id;
        self.enemies.clear();
        self.drops.clear();
        self.others.clear();
        self.populate_map();
    }

    /// A dead friend close enough to revive (the Interact key).
    pub fn revivable(&self) -> Option<usize> {
        if self.player.state == PState::Dead {
            return None;
        }
        let f = self.player.body.feet();
        self.others.iter().position(|o| o.dead && (o.pos.x - f.x).abs() < 80.0 && (o.pos.y - f.y).abs() < 60.0)
    }

    fn update_others(&mut self, dt: f32) {
        let k = 1.0 - (-dt * 15.0).exp();
        for o in &mut self.others {
            o.since += dt;
            let t = V2::new(o.st.x, o.st.y);
            if (t - o.pos).len() > 400.0 {
                o.pos = t;
            } else {
                o.pos = o.pos + (t - o.pos) * k;
            }
            let pt = V2::new(o.st.pet_x, o.st.pet_y);
            o.pet_pos = if (pt - o.pet_pos).len() > 400.0 { pt } else { o.pet_pos + (pt - o.pet_pos) * k };
            o.dead = o.st.dead;
        }
    }

    /// The server says we (and anyone else who helped) beat a monster.
    fn net_kill(&mut self, enemy: u32, kind: EnemyKind, exp: u32) {
        let at = self.enemies.iter().find(|e| e.id == enemy).map(|e| e.body.center()).unwrap_or(self.player.body.center());
        self.burst(at, FxKind::Spark, 20, 260.0);
        self.float(V2::new(at.x, at.y - 50.0), format!("+{} EXP", exp), FloatKind::Exp);
        self.gain_exp(exp);
        self.quest_event(QuestEvent::Killed(kind));
        let def = kind.def();
        if def.boss {
            self.toast(&format!("{} defeated!", def.name), "Return to turn in your quest.");
            self.shake = 10.0;
        }
        self.wants_save = true;
    }

    /// Apply one message from the server.
    pub fn net_apply(&mut self, m: ServerMsg) {
        match m {
            ServerMsg::Enemies(list) => self.net_enemies(list),
            ServerMsg::Shot(s) => {
                let style = crate::classes::ProjStyle::from_u32(s.style);
                self.projectiles.push(Projectile {
                    team: Team::Enemy,
                    pos: V2::new(s.x, s.y),
                    vel: V2::new(s.vx, s.vy),
                    w: s.size,
                    h: s.size,
                    gravity: s.gravity,
                    life: s.life,
                    max_life: s.life,
                    pierce: s.pierce,
                    damage: s.damage,
                    kb: V2::new(s.kbx, s.kby),
                    hitstun: 0.3,
                    style,
                    hit_ids: vec![],
                    grounded: s.grounded,
                    heavy: s.pierce,
                    crit_bonus: 0.0,
                    magic: matches!(style, ProjStyle::Orb | ProjStyle::Star | ProjStyle::Bolt),
                    travelled: 0.0,
                    range: max_range(style, Team::Enemy),
                });
            }
            ServerMsg::Shake { amount } => {
                if self.settings.screen_shake {
                    self.shake = self.shake.max(amount);
                }
            }
            ServerMsg::Kill { enemy, kind, exp } => {
                if let Some(k) = EnemyKind::from_index(kind) {
                    self.net_kill(enemy, k, exp);
                }
            }
            ServerMsg::DropAdd(d) => {
                let kind = if d.item == net::GOLD {
                    DropKind::Gold
                } else {
                    match items::item(&d.item) {
                        Some(def) => DropKind::Item(def.id),
                        None => return,
                    }
                };
                if self.drops.iter().any(|x| x.id == d.id) {
                    return;
                }
                let mut body = Body::new(d.x, d.y, 18.0, 18.0);
                body.vel = V2::new(d.vx, d.vy);
                self.drops.push(GroundDrop { id: d.id, kind, count: d.count, body, age: 0.0, taken: None, pending: 0.0 });
            }
            ServerMsg::DropGone { id, by } => {
                if let Some(i) = self.drops.iter().position(|d| d.id == id && d.taken.is_none()) {
                    // someone else took it: it flies to them (or just vanishes)
                    match self.others.iter().find(|o| o.id == by) {
                        Some(o) => self.drops[i].taken = Some((Collector::Pet(u32::MAX), V2::new(o.pos.x, o.pos.y - 32.0), 0.0)),
                        None if by == self.my_id => {}
                        None => {
                            self.drops.remove(i);
                        }
                    }
                }
            }
            ServerMsg::Granted { drop, item, count } => self.net_granted(drop, &item, count),
            ServerMsg::PlayerInfo(info) => {
                if info.id == self.my_id {
                    return;
                }
                match self.others.iter_mut().find(|o| o.id == info.id) {
                    Some(o) => {
                        o.name = info.name.clone();
                        o.info = info;
                    }
                    None => self.others.push(OtherPlayer {
                        id: info.id,
                        name: info.name.clone(),
                        pos: V2::ZERO,
                        dead: false,
                        info,
                        st: PlayerPos::default(),
                        pet_pos: V2::ZERO,
                        since: 99.0,
                    }),
                }
            }
            ServerMsg::PlayerPos(p) => {
                if let Some(o) = self.others.iter_mut().find(|o| o.id == p.id) {
                    if o.since > 5.0 {
                        // first update (or after a long gap): appear right there
                        o.pos = V2::new(p.x, p.y);
                        o.pet_pos = V2::new(p.pet_x, p.pet_y);
                    }
                    o.st = p;
                    o.since = 0.0;
                }
            }
            ServerMsg::PlayerGone { id } => self.others.retain(|o| o.id != id),
            ServerMsg::Revived { by } => {
                self.revive_by_player(&by);
            }
            ServerMsg::Chat { from, text } => self.push_chat(&from, &text),
            ServerMsg::System { text } => self.push_chat("", &text),
            _ => {}
        }
    }

    pub fn push_chat(&mut self, from: &str, text: &str) {
        self.chat.push((from.to_string(), text.to_string(), 0.0));
        if self.chat.len() > 60 {
            self.chat.remove(0);
        }
    }

    fn net_enemies(&mut self, list: Vec<EnemyNet>) {
        self.enemies.retain(|e| list.iter().any(|n| n.id == e.id));
        for n in list {
            let (Some(kind), Some(state)) = (EnemyKind::from_index(n.kind), enemies::EState::from_u32(n.state)) else { continue };
            let i = match self.enemies.iter().position(|e| e.id == n.id) {
                Some(i) => i,
                None => {
                    let mut e = Enemy::new(n.id, kind, 0.0, 0.0, None, &mut self.rng);
                    e.body.pos = V2::new(n.x, n.y);
                    self.enemies.push(e);
                    self.enemies.len() - 1
                }
            };
            let e = &mut self.enemies[i];
            e.net = Some(V2::new(n.x, n.y));
            e.body.vel = V2::new(n.vx, n.vy);
            e.facing = n.facing as f32;
            if e.state != state || (n.state_t < e.state_t - 0.2) {
                e.state = state;
                e.state_t = n.state_t;
            }
            e.hp = n.hp;
            e.max_hp = n.max_hp;
            e.boss_phase2 = n.phase2;
        }
    }

    fn net_granted(&mut self, drop: u32, item: &str, count: u32) {
        let at = self.drops.iter().find(|d| d.id == drop).map(|d| d.body.center()).unwrap_or(self.player.body.center());
        if item == net::GOLD {
            let bonus = self.active_pet().map(|p| p.gold_find()).unwrap_or(0.0);
            let amount = (count as f32 * (1.0 + bonus)).round() as u32;
            self.save.gold += amount;
            self.float(V2::new(at.x, at.y - 30.0), format!("+{} gold", amount), FloatKind::Loot);
        } else if let Some(def) = items::item(item) {
            let left = self.save.inventory.add(def.id, count);
            if left > 0 {
                // bag filled up meanwhile: put the rest back on the ground
                self.outbox.push(ClientMsg::DropItem(DropNet { id: 0, item: def.id.to_string(), count: left, x: at.x - 9.0, y: at.y - 9.0, vx: 0.0, vy: -200.0 }));
            }
            let got = count - left;
            if got > 0 {
                let label = if got > 1 { format!("+{}x {}", got, def.name) } else { format!("+ {}", def.name) };
                self.float(V2::new(at.x, at.y - 30.0), label, FloatKind::Loot);
            }
        }
        // fly to whoever picked it up: the pet if it went for it, else you
        let pet_at = self.pet.as_ref().filter(|a| a.target == Some(drop)).map(|a| V2::new(a.pos.x, a.pos.y - 14.0));
        if let Some(d) = self.drops.iter_mut().find(|d| d.id == drop) {
            d.taken = Some(match pet_at {
                Some(p) => (Collector::Pet(0), p, 0.0),
                None => (Collector::Player, self.player.body.center(), 0.0),
            });
        }
        self.wants_save = true;
    }

    /// What others see of us (sent when it changes).
    pub fn my_info(&self, look: &str) -> PlayerInfo {
        let pet = self.active_pet();
        PlayerInfo {
            id: self.my_id,
            name: self.save.name.clone(),
            look: look.to_string(),
            weapon: self.save.inventory.equip[EquipSlot::Weapon.index()].clone().unwrap_or_default(),
            level: self.save.level,
            class: self.save.class.key().to_string(),
            pet: pet.map(|p| p.species.key().to_string()).unwrap_or_default(),
            pet_name: pet.map(|p| p.name.clone()).unwrap_or_default(),
            pet_level: pet.map(|p| p.level).unwrap_or(0),
        }
    }

    // ------------------------------------------------------------------ pets

    /// The pet that's out with you.
    pub fn active_pet(&self) -> Option<&PetSave> {
        self.save.active_pet.and_then(|i| self.save.pets.get(i))
    }

    /// Use a pet from the Pet Shop: it becomes yours and comes out.
    pub fn adopt_pet(&mut self, species: PetSpecies) -> Result<(), String> {
        if self.save.pets.len() >= pets::MAX_PETS {
            return Err(format!("You can only keep {} pets", pets::MAX_PETS));
        }
        if self.save.inventory.remove(species.item_id(), 1) == 0 {
            return Err(format!("You don't have a {}", species.name()));
        }
        self.save.pets.push(PetSave::new(species));
        let i = self.save.pets.len() - 1;
        if self.save.active_pet.is_none() {
            self.summon_pet(i)?;
        }
        let first = species.passives()[0];
        self.toast(&format!("You adopted a {}!", species.name()), &format!("It picks up loot near you. Passive: {} - {}", first.name, first.desc));
        self.wants_save = true;
        Ok(())
    }

    /// Bring a pet out (sending any other pet home).
    pub fn summon_pet(&mut self, i: usize) -> Result<(), String> {
        if i >= self.save.pets.len() {
            return Err("No such pet".into());
        }
        if self.save.active_pet == Some(i) {
            return Ok(());
        }
        self.save.active_pet = Some(i);
        self.pet = None;
        self.recompute_stats();
        let c = self.player.body.center();
        self.burst(c, FxKind::Spark, 16, 200.0);
        self.wants_save = true;
        Ok(())
    }

    /// Send the active pet home (its hunger stops dropping).
    pub fn dismiss_pet(&mut self) {
        if self.save.active_pet.take().is_some() {
            self.pet = None;
            self.recompute_stats();
            self.wants_save = true;
        }
    }

    /// Feed the active pet one Pet Food from the bag. Returns levels gained.
    pub fn feed_pet(&mut self) -> Result<u32, String> {
        let Some(i) = self.save.active_pet else { return Err("Summon a pet first (Pets window)".into()) };
        if self.save.inventory.count("pet_food") == 0 {
            return Err("No Pet Food - buy some at a Pet Shop".into());
        }
        self.save.inventory.remove("pet_food", 1);
        let was_starving = self.save.pets[i].starving();
        let before = self.save.pets[i].level;
        let gained = self.save.pets[i].feed();
        let pet = self.save.pets[i].clone();
        let at = self.pet.as_ref().map(|a| a.pos).unwrap_or(self.player.body.feet());
        self.burst(V2::new(at.x, at.y - 14.0), FxKind::Heal, 10, 120.0);
        self.float(V2::new(at.x, at.y - 40.0), if gained > 0 { format!("{} Lv.{}!", pet.name, pet.level) } else { format!("+{} pet EXP", pets::FOOD_EXP) }, FloatKind::Exp);
        for lv in before + 1..=pet.level {
            if let Some(p) = pet.passive_at(lv) {
                self.toast(&format!("{} learned {}!", pet.name, p.name), p.desc);
            }
        }
        if gained > 0 || was_starving {
            self.recompute_stats();
        }
        self.wants_save = true;
        Ok(gained)
    }

    fn update_pet(&mut self, dt: f32) {
        let Some(i) = self.save.active_pet else {
            self.pet = None;
            return;
        };
        if i >= self.save.pets.len() {
            self.save.active_pet = None;
            self.pet = None;
            return;
        }
        let feet = self.player.body.feet();
        let mut a = self.pet.take().unwrap_or(PetActor {
            pos: V2::new(feet.x - self.player.facing * 40.0, feet.y),
            facing: self.player.facing,
            moving: false,
            anim_t: 0.0,
            target: None,
            skip: Vec::new(),
            skip_t: 4.0,
            warned_low: false,
        });
        a.anim_t += dt;

        // Hunger drains while the pet is out.
        let was_starving = self.save.pets[i].starving();
        let pet = &mut self.save.pets[i];
        pet.hunger = (pet.hunger - pets::HUNGER_PER_SEC * dt).max(0.0);
        let pet = pet.clone();
        if pet.starving() && !was_starving {
            self.toast(&format!("{} is starving!", pet.name), "Feed it Pet Food - it won't loot or help you until it eats.");
            self.recompute_stats();
            self.wants_save = true;
        } else if pet.hunger < pets::HUNGER_LOW && !a.warned_low && !pet.starving() {
            a.warned_low = true;
            self.toast(&format!("{} is getting hungry", pet.name), "Feed it Pet Food soon.");
        } else if pet.hunger >= pets::HUNGER_LOW {
            a.warned_low = false;
        }

        let alive = self.player.state != PState::Dead;
        // Healing passives.
        let (hr, mr) = pet.regen();
        if alive && (hr > 0.0 || mr > 0.0) {
            let p = &mut self.player;
            p.hp = (p.hp + p.max_hp * hr * dt).min(p.max_hp);
            p.mp = (p.mp + p.max_mp * mr * dt).min(p.max_mp);
        }

        // Pick something to fetch: the nearest loot within range of its owner.
        a.skip_t -= dt;
        if a.skip_t <= 0.0 {
            a.skip.clear();
            a.skip_t = 4.0;
        }
        let pc = self.player.body.center();
        let radius = pet.loot_radius();
        let still_there = |id: u32, drops: &[GroundDrop]| drops.iter().any(|d| d.id == id && d.taken.is_none());
        if a.target.map(|t| !still_there(t, &self.drops)).unwrap_or(false) {
            a.target = None;
        }
        if pet.starving() || !alive {
            a.target = None;
        } else if a.target.is_none() {
            a.target = self
                .drops
                .iter()
                .filter(|d| d.taken.is_none() && d.age > 0.5 && d.body.on_ground && !a.skip.contains(&d.id))
                .map(|d| (d.id, (d.body.center().x - pc.x).abs() + (d.body.center().y - pc.y).abs() * 0.5))
                .filter(|(_, dist)| *dist <= radius)
                .min_by(|x, y| x.1.total_cmp(&y.1))
                .map(|(id, _)| id);
        }

        // Move: to the target, or trot along behind you.
        let goal = match a.target.and_then(|t| self.drops.iter().find(|d| d.id == t)) {
            Some(d) => d.body.feet(),
            None => V2::new(feet.x - self.player.facing * 46.0, feet.y),
        };
        let delta = goal - a.pos;
        let dist = delta.len();
        if dist > 1200.0 {
            a.pos = goal; // fell far behind (map edge, climbing): catch up
        }
        let speed = if a.target.is_some() { 480.0 } else { (dist * 4.0).clamp(0.0, 520.0) };
        let step = (speed * dt).min(dist);
        a.moving = step > 0.4 && !pet.starving();
        if dist > 0.01 && !pet.starving() {
            a.pos = a.pos + delta * (step / dist);
            if delta.x.abs() > 2.0 {
                a.facing = delta.x.signum();
            }
        } else if pet.starving() {
            // too hungry to keep up: it plods slowly
            a.pos = a.pos + delta * ((60.0 * dt).min(dist) / dist.max(0.01));
        } else if a.target.is_none() {
            a.facing = self.player.facing;
        }

        // Reached the loot: pick it up for its owner.
        if let Some(t) = a.target {
            if let Some(di) = self.drops.iter().position(|d| d.id == t) {
                let c = self.drops[di].body.feet();
                if (c - a.pos).len() < 26.0 {
                    let at = V2::new(a.pos.x, a.pos.y - 14.0);
                    if self.collect(di, Collector::Pet(i as u32), at).is_err() {
                        a.skip.push(t);
                        self.float(V2::new(a.pos.x, a.pos.y - 40.0), "Bag full!".into(), FloatKind::Info);
                    }
                    a.target = None;
                }
            }
        }
        self.pet = Some(a);
    }

    // ------------------------------------------------------------------ ground loot

    /// Toss a drop into the world from `at`.
    pub fn spawn_drop(&mut self, kind: DropKind, count: u32, at: V2, spread: f32) {
        if self.online {
            let vx = self.rng.range(-1.0, 1.0) * spread;
            self.outbox.push(ClientMsg::DropItem(DropNet { id: 0, item: drop_item_key(kind), count, x: at.x - 9.0, y: at.y - 9.0, vx, vy: -360.0 }));
            return;
        }
        let id = self.next_id;
        self.next_id += 1;
        let mut body = Body::new(at.x - 9.0, at.y - 9.0, 18.0, 18.0);
        body.vel = V2::new(self.rng.range(-1.0, 1.0) * spread, self.rng.range(-430.0, -300.0));
        self.drops.push(GroundDrop { id, kind, count, body, age: 0.0, taken: None, pending: 0.0 });
    }

    fn update_drops(&mut self, dt: f32) {
        if self.drops.is_empty() {
            return;
        }
        let solids = self.solids_with_gates();
        let lvl = Level { solids: &solids, platforms: &self.map.platforms, bounds: self.map.bounds() };
        let pc = self.player.body.center();
        for d in self.drops.iter_mut() {
            d.age += dt;
            if let Some((who, target, t)) = &mut d.taken {
                if *who == Collector::Player {
                    *target = pc;
                }
                *t += dt;
                let k = (*t / COLLECT_FLY).min(1.0);
                let c = d.body.center();
                let n = c + (*target - c) * k;
                d.body.pos = V2::new(n.x - d.body.w * 0.5, n.y - d.body.h * 0.5);
                continue;
            }
            d.pending = (d.pending - dt).max(0.0);
            physics::apply_gravity(&mut d.body, dt, false, 1.0);
            physics::move_and_collide(&mut d.body, &lvl, dt);
            if d.body.on_ground {
                d.body.vel.x = approach(d.body.vel.x, 0.0, 900.0 * dt);
            }
        }
        let kill_y = self.map.kill_y;
        let online = self.online;
        self.drops.retain(|d| match d.taken {
            Some((_, _, t)) => t < COLLECT_FLY,
            // online, the server says when loot expires
            None => online || (d.age < DROP_LIFE && d.body.pos.y < kill_y),
        });
    }

    /// Index of the closest drop that can be picked up from `pos`.
    pub fn drop_near(&self, pos: V2, reach: f32) -> Option<usize> {
        let mut best: Option<(usize, f32)> = None;
        for (i, d) in self.drops.iter().enumerate() {
            if d.taken.is_some() || d.age < 0.25 || d.pending > 0.0 {
                continue;
            }
            let c = d.body.center();
            let dx = (c.x - pos.x).abs();
            if dx < reach && (c.y - pos.y).abs() < 72.0 && best.map(|b| dx < b.1).unwrap_or(true) {
                best = Some((i, dx));
            }
        }
        best.map(|b| b.0)
    }

    /// Pick up the nearest drop (the Pickup key). Returns true if something was taken.
    pub fn pick_up(&mut self) -> bool {
        if self.player.state == PState::Dead {
            return false;
        }
        let c = self.player.body.center();
        let Some(i) = self.drop_near(c, PICKUP_REACH) else { return false };
        match self.collect(i, Collector::Player, c) {
            Ok(()) => true,
            Err(e) => {
                self.info_float(&e);
                false
            }
        }
    }

    /// A pet at `pos` grabs up to `max` drops within `reach`. Returns how many
    /// it took. Loot goes into the owner's bag exactly as if they'd picked it up.
    pub fn pet_collect(&mut self, pet: u32, pos: V2, reach: f32, max: usize) -> usize {
        let mut n = 0;
        while n < max {
            let Some(i) = self.drop_near(pos, reach) else { break };
            if self.collect(i, Collector::Pet(pet), pos).is_err() {
                break;
            }
            n += 1;
        }
        n
    }

    /// Move a drop into the bag (or gold purse) and start its fly-in.
    fn collect(&mut self, i: usize, who: Collector, to: V2) -> Result<(), String> {
        let d = self.drops[i].clone();
        if self.online {
            // Ask the server; whoever asks first gets it.
            if let DropKind::Item(id) = d.kind {
                if !self.save.inventory.room_for(id, 1) {
                    return Err("Your bag is full".into());
                }
            }
            self.drops[i].pending = 2.0;
            self.outbox.push(ClientMsg::PickUp { drop: d.id, pet: matches!(who, Collector::Pet(_)) });
            let _ = to;
            return Ok(());
        }
        let at = d.body.center();
        match d.kind {
            DropKind::Gold => {
                // A pet's Gold Sense adds a little extra.
                let bonus = self.active_pet().map(|p| p.gold_find()).unwrap_or(0.0);
                let amount = (d.count as f32 * (1.0 + bonus)).round() as u32;
                self.save.gold += amount;
                self.float(V2::new(at.x, at.y - 30.0), format!("+{} gold", amount), FloatKind::Loot);
            }
            DropKind::Item(id) => {
                let left = self.save.inventory.add(id, d.count);
                if left == d.count {
                    return Err("Your bag is full".into());
                }
                let got = d.count - left;
                let name = items::item(id).map(|x| x.name).unwrap_or(id);
                let label = if got > 1 { format!("+{}x {}", got, name) } else { format!("+ {}", name) };
                self.float(V2::new(at.x, at.y - 30.0), label, FloatKind::Loot);
                if left > 0 {
                    // Only part of the stack fitted; the rest stays on the ground.
                    self.drops[i].count = left;
                    self.wants_save = true;
                    return Ok(());
                }
            }
        }
        self.drops[i].taken = Some((who, to, 0.0));
        self.wants_save = true;
        Ok(())
    }

    /// Drop items from a bag slot onto the ground. Bound items refuse.
    pub fn drop_item(&mut self, idx: usize, n: u32) -> Result<(), String> {
        if self.player.state == PState::Dead {
            return Err("You can't do that while defeated".into());
        }
        let st = self.save.inventory.slots.get(idx).ok_or("No item")?;
        let def = items::item(&st.id).ok_or("Unknown item")?;
        if !def.can_drop() {
            return Err(format!("{} is bound to you and can't be dropped", def.name));
        }
        let (_, taken) = self.save.inventory.take_slot(idx, n).ok_or("No item")?;
        let c = self.player.body.center();
        if self.online {
            self.outbox.push(ClientMsg::DropItem(DropNet { id: 0, item: def.id.to_string(), count: taken, x: c.x - 9.0, y: c.y - 9.0, vx: self.player.facing * 150.0, vy: -320.0 }));
            self.wants_save = true;
            return Ok(());
        }
        let id = self.next_id;
        self.next_id += 1;
        let mut body = Body::new(c.x - 9.0, c.y - 9.0, 18.0, 18.0);
        body.vel = V2::new(self.player.facing * 150.0, -320.0);
        self.drops.push(GroundDrop { id, kind: DropKind::Item(def.id), count: taken, body, age: 0.0, taken: None, pending: 0.0 });
        self.wants_save = true;
        Ok(())
    }

    fn player_died(&mut self) {
        let p = &mut self.player;
        if p.state == PState::Dead {
            return;
        }
        p.state = PState::Dead;
        p.state_t = 0.0;
        p.attack = None;
        p.buffs.clear();
        p.body.vel = V2::new(-p.facing * 120.0, -410.0);
        self.shake = 8.0;
        self.dialog = None;
        self.shop_open = None;
        self.teleport_open = false;
        // Above level 10, a defeat costs some EXP (a Revive Scroll gives it back).
        let lost = if self.save.level > DEATH_PENALTY_LEVEL { (self.save.exp as f32 * DEATH_EXP_LOSS).floor() as u32 } else { 0 };
        self.save.exp -= lost;
        self.save.tomb = Some(lost);
        if lost > 0 {
            let c = self.player.body.center();
            self.float(V2::new(c.x, c.y - 60.0), format!("-{} EXP", lost), FloatKind::PlayerHurt);
        }
        self.recompute_stats();
        self.wants_save = true;
    }

    /// EXP lost to the current defeat (0 if not defeated).
    pub fn death_exp_lost(&self) -> u32 {
        self.save.tomb.unwrap_or(0)
    }

    /// Damage the player from an enemy/hazard.
    fn hurt_player(&mut self, dmg: f32, from_x: f32, kb: V2, magic: bool) {
        let p = &mut self.player;
        if p.is_invulnerable() {
            return;
        }
        let taken = p.stats.damage_taken(magic);
        let dmg = (dmg * taken * self.rng.range(0.9, 1.1)).round().max(1.0);
        let p = &mut self.player;
        p.hp -= dmg;
        p.since_hurt = 0.0;
        p.hurt_flash = 0.25;
        p.invuln = HURT_IFRAMES;
        let dir = (p.body.center().x - from_x).signum();
        let dir = if dir == 0.0 { -p.facing } else { dir };
        let wk = 1.0 / p.info.weight;
        if kb != V2::ZERO {
            p.body.vel = V2::new(kb.x * dir * wk, kb.y * wk);
            p.body.on_ground = false;
            p.state = PState::Hitstun;
            p.state_t = 0.0;
            p.attack = None;
        }
        let c = p.body.center();
        self.float(V2::new(c.x, c.y - 40.0), format!("{}", dmg as i32), FloatKind::PlayerHurt);
        self.burst(c, FxKind::Hit, 8, 220.0);
        self.hitstop = 0.05;
        if self.settings.screen_shake {
            self.shake = self.shake.max(5.0);
        }
        if self.player.hp <= 0.0 {
            self.player.hp = 0.0;
            self.player_died();
        }
    }

    fn enemy_contacts(&mut self) {
        let pr = self.player.body.aabb();
        let mut hit: Option<(f32, f32, V2)> = None;
        for e in &self.enemies {
            let def = e.def();
            if e.state == EState::Dying || (e.state == EState::Hitstun && !def.boss) || def.contact <= 0.0 {
                continue;
            }
            let attacking = e.attacking();
            let dmg = if attacking { def.attack } else { def.contact };
            let shrink = if attacking { 0.0 } else { 4.0 };
            if e.body.aabb().expanded(-shrink).overlaps(&pr) {
                let kb = if attacking { V2::new(430.0, -350.0) } else { V2::new(250.0, -215.0) };
                hit = Some((dmg, e.body.center().x, kb));
                break;
            }
        }
        if let Some((d, x, kb)) = hit {
            self.hurt_player(d, x, kb, false);
        }
    }

    // ------------------------------------------------------------------ enemies

    fn update_enemies(&mut self, dt: f32) {
        if self.online {
            // Glide toward the server's positions between snapshots.
            let k = 1.0 - (-dt * 14.0).exp();
            for e in &mut self.enemies {
                e.state_t += dt;
                e.flash = (e.flash - dt).max(0.0);
                e.wobble = (e.wobble - dt * 2.5).max(0.0);
                if let Some(t) = e.net {
                    let t = t + e.body.vel * dt;
                    e.net = Some(t);
                    e.body.pos = e.body.pos + (t - e.body.pos) * k;
                }
            }
            return;
        }
        let solids = self.solids_with_gates();
        let lvl = Level { solids: &solids, platforms: &self.map.platforms, bounds: self.map.bounds() };
        let ctx = enemies::Ctx { level: &lvl, player: self.player.body.center(), player_alive: self.player.state != PState::Dead, dt };
        let mut out = enemies::Out::default();
        for e in self.enemies.iter_mut() {
            enemies::update(e, &ctx, &mut out);
        }
        for s in out.shots {
            self.projectiles.push(Projectile {
                team: Team::Enemy,
                pos: s.pos,
                vel: s.vel,
                w: s.size,
                h: s.size,
                gravity: s.gravity,
                life: s.life,
                max_life: s.life,
                pierce: s.pierce,
                damage: s.damage,
                kb: s.kb,
                hitstun: 0.3,
                style: s.style,
                hit_ids: vec![],
                grounded: s.grounded,
                heavy: s.pierce,
                crit_bonus: 0.0,
                magic: matches!(s.style, ProjStyle::Orb | ProjStyle::Star | ProjStyle::Bolt),
                travelled: 0.0,
                range: max_range(s.style, Team::Enemy),
            });
        }
        if self.settings.screen_shake {
            self.shake = self.shake.max(out.shake);
        }
        // Remove the dead after their fade; zone respawn timers start then.
        let kill_y = self.map.kill_y;
        let mut removed_zones = Vec::new();
        self.enemies.retain(|e| {
            let gone = (e.state == EState::Dying && e.state_t > 0.6) || e.body.pos.y > kill_y;
            if gone {
                if let Some(z) = e.zone {
                    removed_zones.push(z);
                }
            }
            !gone
        });
        for z in removed_zones {
            if z < self.zone_timers.len() && self.zone_timers[z] <= 0.0 {
                self.zone_timers[z] = self.map.spawns[z].respawn;
            }
        }
    }

    fn update_spawns(&mut self, dt: f32) {
        if self.online {
            return;
        }
        for zi in 0..self.map.spawns.len() {
            if self.zone_timers[zi] > 0.0 {
                self.zone_timers[zi] -= dt;
                if self.zone_timers[zi] <= 0.0 {
                    let z = self.map.spawns[zi].clone();
                    let alive = self.enemies.iter().filter(|e| e.zone == Some(zi)).count();
                    if alive < z.max {
                        // Don't pop enemies in right next to the player.
                        let mut x = self.rng.range(z.x0, z.x1);
                        if (x - self.player.body.center().x).abs() < 250.0 {
                            x = if x < (z.x0 + z.x1) * 0.5 { z.x1 - 30.0 } else { z.x0 + 30.0 };
                        }
                        self.spawn_enemy(z.kind, x, z.y - 40.0, Some(zi));
                        let pos = V2::new(x, z.y - 20.0);
                        self.burst(pos, FxKind::Dust, 10, 120.0);
                        if alive + 1 < z.max {
                            self.zone_timers[zi] = z.respawn;
                        }
                    }
                }
            }
        }
    }

    fn update_projectiles(&mut self, dt: f32) {
        let solids = self.solids_with_gates();
        let mut hits_enemy: Vec<(usize, f32, V2, f32, bool, f32)> = Vec::new();
        let mut hurt: Option<(f32, f32, V2, bool)> = None;
        let pr = self.player.body.aabb();
        let player_inv = self.player.is_invulnerable();
        for p in self.projectiles.iter_mut() {
            p.life -= dt;
            if !p.grounded {
                p.vel.y += p.gravity * dt;
            }
            let step = p.vel * dt;
            p.pos = p.pos + step;
            if !p.grounded {
                p.travelled += step.len();
                if p.travelled > p.range {
                    // fade out over the last stretch
                    p.life = p.life.min(0.06);
                }
            }
            let r = p.aabb();
            let in_solid = solids.iter().any(|s| s.overlaps(&r));
            if in_solid {
                match p.style {
                    ProjStyle::Caltrop if p.team == Team::Player => {
                        // Settle on the ground and linger as a trap.
                        if p.vel.y > 0.0 {
                            p.pos.y -= p.vel.y * dt;
                            p.vel = V2::ZERO;
                            p.gravity = 0.0;
                            p.grounded = true;
                        } else {
                            p.life = 0.0;
                        }
                    }
                    ProjStyle::Shockwave => {}
                    _ => p.life = 0.0,
                }
            }
            if p.life <= 0.0 {
                continue;
            }
            match p.team {
                Team::Player => {
                    for (ei, e) in self.enemies.iter().enumerate() {
                        if e.state == EState::Dying || p.hit_ids.contains(&e.id) {
                            continue;
                        }
                        if r.overlaps(&e.body.aabb()) {
                            p.hit_ids.push(e.id);
                            let dir = if p.vel.x.abs() > 1.0 { p.vel.x.signum() } else { (e.body.center().x - p.pos.x).signum() };
                            hits_enemy.push((ei, p.damage, V2::new(p.kb.x * dir, p.kb.y), p.hitstun, p.heavy, p.crit_bonus));
                            if !p.pierce {
                                p.life = 0.0;
                                break;
                            }
                        }
                    }
                }
                Team::Enemy => {
                    if !player_inv && hurt.is_none() && r.overlaps(&pr) {
                        hurt = Some((p.damage, p.pos.x - p.vel.x.signum() * 10.0, p.kb, p.magic));
                        if !p.pierce {
                            p.life = 0.0;
                        }
                    }
                }
            }
        }
        for (ei, dmg, kb, hs, heavy, crit_bonus) in hits_enemy {
            if ei < self.enemies.len() && self.enemies[ei].state != EState::Dying {
                self.damage_enemy_crit(ei, dmg, kb, hs, heavy, crit_bonus);
                if crit_bonus == 0.0 {
                    self.player.mp = (self.player.mp + 1.0).min(self.player.max_mp);
                }
            }
        }
        if let Some((d, x, kb, magic)) = hurt {
            self.hurt_player(d, x, kb, magic);
        }
        let w = self.map.width;
        self.projectiles.retain(|p| p.life > 0.0 && p.pos.x > -100.0 && p.pos.x < w + 100.0 && p.pos.y < 1400.0);
    }

    fn update_regions(&mut self, dt: f32) {
        self.region_cooldown -= dt;
        if self.region_cooldown > 0.0 {
            return;
        }
        self.region_cooldown = 0.2;
        let r = self.player.body.aabb();
        let ids: Vec<&'static str> = self.map.regions.iter().filter(|g| g.area.overlaps(&r)).map(|g| g.id).collect();
        for id in ids {
            self.quest_event(QuestEvent::Reached(id));
        }
    }

    /// Copy runtime state into the save before writing it to disk.
    pub fn snapshot(&mut self) -> CharacterSave {
        let p = &self.player;
        self.save.hp = if p.state == PState::Dead { p.max_hp } else { p.hp };
        self.save.mp = p.mp;
        self.save.map = self.map.key.to_string();
        if p.body.on_ground || p.state == PState::Dead {
            let f = p.body.feet();
            self.save.x = f.x;
            self.save.y = f.y;
        }
        self.save.clone()
    }

    pub fn level_ref(&self) -> Level<'_> {
        self.level()
    }
}

/// World-space hitbox of an attack for the current facing.
pub fn attack_hitbox(p: &Player, def: &AttackDef) -> Aabb {
    let c = p.body.center();
    if def.both_sides {
        Aabb::new(c.x - def.hw * 0.5 + def.hx * p.facing, c.y + def.hy - def.hh * 0.5, def.hw, def.hh)
    } else {
        let cx = c.x + def.hx * p.facing;
        Aabb::new(cx - def.hw * 0.5 + p.facing * def.hw * 0.25, c.y + def.hy - def.hh * 0.5, def.hw, def.hh)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::appearance::Appearance;
    use crate::input::Action;

    fn press(a: &[Action], held: &[Action]) -> InputState {
        let mut i = InputState::default();
        for &x in held {
            i.set(x, true, false);
        }
        for &x in a {
            i.set(x, true, true);
        }
        i
    }

    fn run(sim: &mut Sim, secs: f32, input: InputState) {
        let n = (secs / TICK) as usize;
        let mut inp = input;
        for i in 0..n {
            sim.tick(&inp);
            if i == 0 {
                inp.clear_pressed();
            }
        }
    }

    fn new_sim(class: ClassId) -> Sim {
        let mut save = CharacterSave::new("Test", Appearance::default());
        if class != ClassId::Novice {
            save.class = class;
            save.level = 10;
            save.inventory.equip[0] = Some(items::starter_weapon(class).to_string());
        }
        Sim::new(save, Settings::default(), 7)
    }

    #[test]
    fn player_lands_and_walks() {
        let mut s = new_sim(ClassId::Vanguard);
        run(&mut s, 1.0, InputState::default());
        assert!(s.player.body.on_ground, "landed at spawn");
        let x0 = s.player.body.pos.x;
        run(&mut s, 0.6, press(&[], &[Action::Right]));
        assert!(s.player.body.pos.x > x0 + 100.0);
    }

    #[test]
    fn double_jump_reaches_higher() {
        let mut s = new_sim(ClassId::Vanguard);
        run(&mut s, 1.0, InputState::default());
        let y0 = s.player.body.pos.y;
        // single jump apex
        run(&mut s, 0.01, press(&[Action::Jump], &[Action::Jump]));
        let mut min1 = y0;
        for _ in 0..80 {
            s.tick(&press(&[], &[Action::Jump]));
            min1 = min1.min(s.player.body.pos.y);
        }
        run(&mut s, 1.0, InputState::default());
        run(&mut s, 0.01, press(&[Action::Jump], &[Action::Jump]));
        for _ in 0..30 {
            s.tick(&press(&[], &[Action::Jump]));
        }
        run(&mut s, 0.01, press(&[Action::Jump], &[Action::Jump]));
        let mut min2 = y0;
        for _ in 0..100 {
            s.tick(&press(&[], &[Action::Jump]));
            min2 = min2.min(s.player.body.pos.y);
        }
        assert!(y0 - min1 > 120.0, "single jump height {}", y0 - min1);
        assert!(min2 < min1 - 60.0, "double jump higher: {} vs {}", y0 - min2, y0 - min1);
    }

    #[test]
    fn every_attack_runs_for_every_class() {
        for &c in crate::classes::ALL_CLASSES {
            let mut s = new_sim(c);
            run(&mut s, 1.0, InputState::default());
            for heavy in [false, true] {
                for dir in [None, Some(Action::Right), Some(Action::Down)] {
                    s.player.mp = s.player.max_mp;
                    let held: Vec<Action> = dir.into_iter().collect();
                    let btn = if heavy { Action::Heavy } else { Action::Light };
                    run(&mut s, 0.01, press(&[btn], &held));
                    assert_eq!(s.player.state, PState::Attack, "{:?} heavy={} dir={:?}", c, heavy, dir);
                    run(&mut s, 1.6, InputState::default());
                    assert_ne!(s.player.state, PState::Attack, "{:?} attack ended", c);
                }
            }
        }
    }

    #[test]
    fn hitting_dummy_progresses_quest() {
        let mut s = new_sim(ClassId::Stormfist);
        for id in ["first_breeze", "gust_dodger"] {
            quests::accept(&mut s.save, id);
            let q = s.save.quest_mut(id).unwrap();
            q.state = QuestState::Done;
        }
        quests::accept(&mut s.save, "sparring_basics");
        s.change_map("dawnwind_glade", 2900.0);
        run(&mut s, 0.5, InputState::default());
        for _ in 0..8 {
            run(&mut s, 0.01, press(&[Action::Light], &[]));
            run(&mut s, 0.5, InputState::default());
        }
        let q = s.save.quest("sparring_basics").unwrap();
        assert_eq!(q.state, QuestState::ReadyToTurnIn, "progress {}", q.progress);
    }

    #[test]
    fn mosslings_can_be_killed_and_respawn() {
        let mut s = new_sim(ClassId::Vanguard);
        s.change_map("dawnwind_glade", 3700.0);
        let before = s.enemies.iter().filter(|e| e.kind == EnemyKind::Mossling).count();
        assert!(before > 0);
        let idx = s.enemies.iter().position(|e| e.kind == EnemyKind::Mossling).unwrap();
        s.damage_enemy(idx, 9999.0, V2::new(100.0, -100.0), 0.2, false);
        assert_eq!(s.save.exp > 0, true);
        run(&mut s, 6.0, InputState::default());
        let after = s.enemies.iter().filter(|e| e.kind == EnemyKind::Mossling).count();
        assert_eq!(after, before, "respawned");
    }

    #[test]
    fn gate_blocks_until_open() {
        let mut s = new_sim(ClassId::Duskblade);
        s.change_map("dawnwind_glade", 6300.0);
        run(&mut s, 2.0, press(&[], &[Action::Right]));
        assert!(s.player.body.pos.x + PLAYER_W <= 6440.5, "blocked by gate at {}", s.player.body.pos.x);
        s.save.set_flag("gate_open");
        run(&mut s, 2.0, press(&[], &[Action::Right]));
        assert!(s.player.body.pos.x > 6500.0);
        run(&mut s, 0.01, press(&[Action::Up], &[]));
        // The run above may have already pushed past the portal; walk back onto it.
        s.player.body.pos.x = 6600.0;
        run(&mut s, 0.2, InputState::default());
        run(&mut s, 0.01, press(&[Action::Up], &[]));
        assert_eq!(s.map.key, "breezehollow");
    }

    #[test]
    fn boss_spawns_and_dies() {
        let mut s = new_sim(ClassId::Aethermancer);
        s.save.quests.push(crate::save::QuestEntry { id: "beak_hunt".into(), state: QuestState::Done, progress: 3 });
        s.change_map("dawnwind_glade", 5050.0);
        run(&mut s, 0.5, InputState::default());
        run(&mut s, 0.01, press(&[Action::Interact], &[]));
        assert!(s.dialog.is_some());
        for _ in 0..5 {
            run(&mut s, 0.01, press(&[Action::Interact], &[]));
        }
        assert!(s.dialog.is_none(), "{:?}", s.dialog.as_ref().map(|d| (&d.dialog.lines, d.line)));
        assert!(s.enemies.iter().any(|e| e.kind == EnemyKind::Bramblehorn), "boss spawned");
        // Let the boss act for a while (exercise AI paths) with the player invulnerable.
        for _ in 0..20 {
            s.player.invuln = 1.0;
            run(&mut s, 0.5, InputState::default());
        }
        let i = s.enemies.iter().position(|e| e.kind == EnemyKind::Bramblehorn).unwrap();
        s.damage_enemy(i, 99999.0, V2::ZERO, 0.1, true);
        run(&mut s, 1.0, InputState::default());
        assert_eq!(s.save.quest("the_bramblehorn").unwrap().state, QuestState::ReadyToTurnIn);
    }

    #[test]
    fn falling_out_leaves_a_tombstone_then_respawns() {
        let mut s = new_sim(ClassId::Windrunner);
        run(&mut s, 1.0, InputState::default());
        let stood = s.last_ground;
        s.player.body.pos.y = 2000.0;
        run(&mut s, 3.0, InputState::default());
        assert_eq!(s.player.state, PState::Dead, "stays down until Respawn is clicked");
        assert!(s.player.body.pos.y < s.map.kill_y, "tombstone is back on the map");
        assert!((s.player.body.feet().x - stood.x).abs() < 40.0, "tombstone where you last stood");
        assert!(s.save.tomb.is_some());
        run(&mut s, 20.0, InputState::default());
        assert_eq!(s.player.state, PState::Dead, "no automatic respawn");
        assert!(s.respawn());
        assert_ne!(s.player.state, PState::Dead);
        assert_eq!(s.player.hp, s.player.max_hp);
        assert!(s.save.tomb.is_none());
    }

    #[test]
    fn death_costs_exp_above_level_10_and_scrolls_refund_it() {
        // Level 10: no penalty.
        let mut s = new_sim(ClassId::Vanguard);
        s.save.exp = 1000;
        run(&mut s, 0.5, InputState::default());
        s.player.invuln = 0.0;
        s.player.state = PState::Normal;
        s.hurt_player(99999.0, 0.0, V2::ZERO, false);
        assert_eq!(s.player.state, PState::Dead);
        assert_eq!(s.save.exp, 1000, "no EXP loss at level 10");
        assert!(s.respawn());
        // Level 11: lose 3% of current EXP.
        s.save.level = 11;
        s.save.exp = 1000;
        s.recompute_stats();
        s.player.hp = s.player.max_hp;
        s.player.invuln = 0.0;
        s.player.state = PState::Normal;
        s.hurt_player(99999.0, 0.0, V2::ZERO, false);
        assert_eq!(s.save.exp, 970);
        assert_eq!(s.death_exp_lost(), 30);
        // Enemies and potions can't touch a tombstone.
        assert!(s.player.is_invulnerable());
        assert!(!s.use_item("hp_potion"));
        assert!(!s.pick_up());
        // No scroll: can't revive that way.
        assert!(s.revive_with_scroll().is_err());
        s.save.inventory.add("revive_scroll", 1);
        // A Revive Scroll on the hotbar works while defeated and refunds the EXP.
        s.set_hotbar(2, HotbarSlot::Item("revive_scroll".into()));
        run(&mut s, 1.0, InputState::default());
        run(&mut s, 0.02, press(&[Action::Hotbar3], &[]));
        assert_ne!(s.player.state, PState::Dead, "scroll revived on the spot");
        assert_eq!(s.save.exp, 1000, "scroll negates the EXP loss");
        assert_eq!(s.save.inventory.count("revive_scroll"), 0);
        assert!(s.save.tomb.is_none());
        // Revived by another player: back up, but the loss stands.
        s.player.invuln = 0.0;
        s.player.state = PState::Normal;
        s.hurt_player(99999.0, 0.0, V2::ZERO, false);
        assert!(s.revive_by_player("Friend"));
        assert_eq!(s.save.exp, 970);
    }

    #[test]
    fn tombstone_survives_logging_out() {
        let mut s = new_sim(ClassId::Vanguard);
        s.save.level = 12;
        s.save.exp = 500;
        run(&mut s, 0.5, InputState::default());
        s.player.invuln = 0.0;
        s.hurt_player(99999.0, 0.0, V2::ZERO, false);
        run(&mut s, 1.0, InputState::default());
        let save = s.snapshot();
        assert_eq!(save.tomb, Some(15));
        let s2 = Sim::new(save, Settings::default(), 3);
        assert_eq!(s2.player.state, PState::Dead, "still a tombstone after loading");
    }

    /// A simple "bot" holds right and jumps / double-jumps whenever it's
    /// blocked, proving the tutorial terrain is traversable with real physics.
    #[test]
    fn bot_can_climb_windstep_rise() {
        for &c in crate::classes::ALL_CLASSES {
            let mut s = new_sim(c);
            quests::accept(&mut s.save, "first_breeze");
            run(&mut s, 0.5, InputState::default());
            let mut jumped_at = -1.0f32;
            let mut t = 0.0;
            while t < 20.0 && s.save.quest("first_breeze").unwrap().state == QuestState::Active {
                let p = &s.player;
                let blocked = p.body.hit_wall || p.body.vel.x.abs() < 50.0;
                let mut pressed = vec![];
                if p.body.on_ground && blocked && t - jumped_at > 0.2 {
                    pressed.push(Action::Jump);
                    jumped_at = t;
                } else if !p.body.on_ground && p.body.vel.y > -80.0 && p.air_jumps_left > 0 && blocked {
                    pressed.push(Action::Jump);
                }
                let inp = press(&pressed, &[Action::Right, Action::Jump]);
                s.tick(&inp);
                t += TICK;
            }
            assert_eq!(
                s.save.quest("first_breeze").unwrap().state,
                QuestState::ReadyToTurnIn,
                "{:?} could not reach the lookout (stuck at x={})",
                c,
                s.player.body.pos.x
            );
        }
    }

    #[test]
    fn bot_can_cross_the_whole_glade() {
        let mut s = new_sim(ClassId::Duskblade);
        s.save.set_flag("gate_open");
        s.change_map("dawnwind_glade", 2300.0);
        let mut t = 0.0;
        let mut jumped_at = -1.0f32;
        while t < 40.0 && s.map.key == "dawnwind_glade" {
            s.player.invuln = 1.0;
            let p = &s.player;
            let blocked = p.body.hit_wall || p.body.vel.x.abs() < 50.0;
            let mut pressed = vec![];
            if p.body.on_ground && blocked && t - jumped_at > 0.2 {
                pressed.push(Action::Jump);
                jumped_at = t;
            }
            if s.nearby_portal().is_some() {
                pressed.push(Action::Up);
            }
            s.tick(&press(&pressed, &[Action::Right, Action::Jump]));
            t += TICK;
        }
        assert_eq!(s.map.key, "breezehollow", "stuck at x={}", s.player.body.pos.x);
    }

    /// A bot crosses every new zone from the west entrance to the east portal.
    #[test]
    fn bot_crosses_new_zones() {
        for (map, flag) in [("whistling_fields", "woods_open"), ("thornvale_woods", "cliffs_open"), ("windcrag_cliffs", "skyroad_open")] {
            let mut s = new_sim(ClassId::Windrunner);
            s.save.set_flag(flag);
            s.change_map(map, 220.0);
            let east = s.map.portals.iter().map(|p| p.area.left()).fold(0.0, f32::max);
            let mut t = 0.0;
            let mut jumped_at = -1.0f32;
            while t < 60.0 && s.player.body.pos.x < east - 40.0 {
                s.player.invuln = 1.0;
                let p = &s.player;
                let blocked = p.body.hit_wall || p.body.vel.x.abs() < 40.0;
                let mut pressed = vec![];
                if p.body.on_ground && blocked && t - jumped_at > 0.25 {
                    pressed.push(Action::Jump);
                    jumped_at = t;
                } else if !p.body.on_ground && p.body.vel.y > -60.0 && p.air_jumps_left > 0 && blocked {
                    pressed.push(Action::Jump);
                }
                s.tick(&press(&pressed, &[Action::Right, Action::Jump]));
                t += TICK;
                assert_eq!(s.map.key, map, "fell or warped out of {}", map);
            }
            assert!(s.player.body.pos.x >= east - 40.0, "{}: stuck at x={} (portal at {})", map, s.player.body.pos.x, east);
        }
    }

    /// Plays the entire quest chain headlessly: talk to every NPC, accept,
    /// kill what's asked, turn in - until the summit is cleared.
    #[test]
    fn whole_quest_chain_completes() {
        let mut s = new_sim(ClassId::Stormfist);
        let maps = ["dawnwind_glade", "breezehollow", "whistling_fields", "thornvale_woods", "windcrag_cliffs"];
        for _round in 0..200 {
            if s.save.has_flag("summit_cleared") {
                break;
            }
            for m in maps {
                let map = world::load_map(m).unwrap();
                for n in &map.npcs {
                    let d = quests::talk(n.id, n.name, &mut s.save);
                    match d.action {
                        DialogAction::OfferQuest(id) => quests::accept(&mut s.save, id),
                        DialogAction::CompleteQuest(id) => {
                            quests::complete(&mut s.save, id);
                        }
                        _ => {}
                    }
                }
            }
            // satisfy whatever objectives are active
            let active: Vec<String> = s.save.quests.iter().filter(|q| q.state == QuestState::Active).map(|q| q.id.clone()).collect();
            for id in active {
                let def = quests::quest_def(&id).unwrap();
                for _ in 0..def.objective.target() {
                    match def.objective {
                        quests::Objective::Kill(k, _) => {
                            quests::on_event(&mut s.save, QuestEvent::Killed(k));
                        }
                        quests::Objective::Reach(r) => {
                            quests::on_event(&mut s.save, QuestEvent::Reached(r));
                        }
                        quests::Objective::Dodge(_) => {
                            quests::on_event(&mut s.save, QuestEvent::Dodged);
                        }
                        quests::Objective::DummyLight(_) => {
                            quests::on_event(&mut s.save, QuestEvent::DummyHit { heavy: false });
                        }
                        quests::Objective::DummyHeavy(_) => {
                            quests::on_event(&mut s.save, QuestEvent::DummyHit { heavy: true });
                        }
                        quests::Objective::ChangeClass => {
                            quests::on_event(&mut s.save, QuestEvent::ChangedClass);
                        }
                        _ => {}
                    }
                }
            }
        }
        assert!(s.save.has_flag("summit_cleared"), "quest chain stalled; quests: {:?}", s.save.quests.iter().map(|q| (&q.id, q.state)).collect::<Vec<_>>());
        for q in quests::QUESTS {
            assert_eq!(s.save.quest(q.id).map(|e| e.state), Some(QuestState::Done), "{} not completed", q.id);
        }
        for f in ["fields_open", "woods_open", "cliffs_open", "gate_open"] {
            assert!(s.save.has_flag(f), "flag {} never set", f);
        }
    }

    #[test]
    fn thunderhorn_appears_for_its_quest() {
        let mut s = new_sim(ClassId::Vanguard);
        quests::accept(&mut s.save, "storm_summit");
        s.change_map("windcrag_cliffs", 3300.0);
        assert!(s.enemies.iter().any(|e| e.kind == EnemyKind::Thunderhorn));
        for _ in 0..10 {
            s.player.invuln = 1.0;
            run(&mut s, 0.5, InputState::default());
        }
        let i = s.enemies.iter().position(|e| e.kind == EnemyKind::Thunderhorn).unwrap();
        s.damage_enemy(i, 99999.0, V2::ZERO, 0.1, true);
        run(&mut s, 1.0, InputState::default());
        assert_eq!(s.save.quest("storm_summit").unwrap().state, QuestState::ReadyToTurnIn);
    }

    #[test]
    fn accepting_kaels_quest_on_the_summit_calls_the_thunderhorn() {
        let mut s = new_sim(ClassId::Vanguard);
        quests::accept(&mut s.save, "hawk_eyes");
        s.save.quest_mut("hawk_eyes").unwrap().state = QuestState::Done;
        quests::accept(&mut s.save, "summit_call");
        s.save.quest_mut("summit_call").unwrap().state = QuestState::Done;
        s.change_map("windcrag_cliffs", 3200.0);
        assert!(!s.enemies.iter().any(|e| e.kind == EnemyKind::Thunderhorn));
        let kael = s.map.npcs.iter().position(|n| n.id == "stormwatcher_kael").unwrap();
        s.player.body.pos.x = s.map.npcs[kael].x - 40.0;
        run(&mut s, 0.4, InputState::default());
        run(&mut s, 0.02, press(&[Action::Interact], &[]));
        assert!(matches!(s.dialog.as_ref().unwrap().dialog.action, DialogAction::OfferQuest("storm_summit")));
        while s.dialog.is_some() {
            run(&mut s, 0.02, press(&[Action::Interact], &[]));
            run(&mut s, 0.1, InputState::default());
        }
        assert!(s.enemies.iter().any(|e| e.kind == EnemyKind::Thunderhorn), "boss appears as soon as the quest is taken");
    }

    #[test]
    fn quest_npcs_summon_their_boss_for_100_gold() {
        let mut s = new_sim(ClassId::Vanguard);
        quests::accept(&mut s.save, "storm_summit");
        s.save.quest_mut("storm_summit").unwrap().state = QuestState::Done;
        s.change_map("windcrag_cliffs", 3200.0);
        let d = quests::talk("stormwatcher_kael", "Kael", &mut s.save);
        assert_eq!(d.action, DialogAction::SummonBoss(EnemyKind::Thunderhorn));
        s.save.gold = 50;
        assert!(s.summon_boss(EnemyKind::Thunderhorn).is_err(), "too poor");
        s.save.gold = 250;
        s.summon_boss(EnemyKind::Thunderhorn).unwrap();
        assert_eq!(s.save.gold, 150);
        assert!(s.enemies.iter().any(|e| e.kind == EnemyKind::Thunderhorn));
        assert!(s.summon_boss(EnemyKind::Thunderhorn).is_err(), "only one at a time");
        assert_eq!(s.save.gold, 150);
        // Ilsa does the same for the Bramblehorn.
        quests::accept(&mut s.save, "the_bramblehorn");
        let d = quests::talk("warden_ilsa", "Ilsa", &mut s.save);
        assert_eq!(d.action, DialogAction::SummonBoss(EnemyKind::Bramblehorn));
    }

    #[test]
    fn no_map_has_more_than_four_flyers() {
        for key in world::all_map_keys() {
            let m = world::load_map(key).unwrap();
            let n: usize = m.spawns.iter().filter(|z| z.kind.def().ai.is_floater()).map(|z| z.max).sum::<usize>()
                + m.fixed_enemies.iter().filter(|f| f.0.def().ai.is_floater()).count();
            assert!(n <= world::MAX_FLYERS, "{} has {} flyers", key, n);
        }
    }

    #[test]
    fn projectiles_have_a_maximum_range() {
        for style in [ProjStyle::Arrow, ProjStyle::Bolt, ProjStyle::Knife, ProjStyle::Orb, ProjStyle::Star] {
            let mut s = new_sim(ClassId::Windrunner);
            s.change_map("breezehollow", 1500.0);
            run(&mut s, 0.3, InputState::default());
            let c = s.player.body.center();
            s.projectiles.push(Projectile {
                team: Team::Player, pos: V2::new(c.x, c.y - 200.0), vel: V2::new(900.0, 0.0), w: 10.0, h: 10.0, gravity: 0.0,
                life: 30.0, max_life: 30.0, pierce: true, damage: 1.0, kb: V2::ZERO, hitstun: 0.0, style, hit_ids: vec![],
                grounded: false, heavy: false, crit_bonus: 0.0, magic: false, travelled: 0.0, range: max_range(style, Team::Player),
            });
            run(&mut s, 3.0, InputState::default());
            assert!(s.projectiles.is_empty(), "{:?} flew forever", style);
        }
    }

    #[test]
    fn flyer_shots_hurt_without_knockback() {
        let mut s = new_sim(ClassId::Vanguard);
        s.change_map("breezehollow", 1500.0);
        run(&mut s, 0.5, InputState::default());
        s.player.invuln = 0.0;
        let hp = s.player.hp;
        let c = s.player.body.center();
        let mut sh = None;
        // a Gustling-style shot: aimed, no knockback
        let mut e = enemies::Enemy::new(999, EnemyKind::Gustling, c.x + 200.0, c.y, None, &mut s.rng);
        e.state = enemies::EState::Telegraph;
        e.state_t = 10.0;
        let lvl_s = s.map.solids.clone();
        let lvl = Level { solids: &lvl_s, platforms: &s.map.platforms, bounds: s.map.bounds() };
        let mut out = enemies::Out::default();
        enemies::update(&mut e, &enemies::Ctx { level: &lvl, player: c, player_alive: true, dt: TICK }, &mut out);
        if let Some(x) = out.shots.first() {
            sh = Some(x.clone());
        }
        let sh = sh.expect("gustling fired");
        assert_eq!(sh.kb, V2::ZERO);
        s.hurt_player(10.0, c.x + 50.0, sh.kb, true);
        assert!(s.player.hp < hp);
        assert_ne!(s.player.state, PState::Hitstun, "no stagger from flyer shots");
        assert!(s.player.body.on_ground);
    }

    #[test]
    fn dodging_keeps_you_invincible_throughout() {
        let mut s = new_sim(ClassId::Vanguard);
        s.change_map("breezehollow", 1500.0);
        run(&mut s, 0.5, InputState::default());
        run(&mut s, 0.02, press(&[Action::Dodge, Action::Right], &[Action::Right]));
        assert_eq!(s.player.state, PState::Dodge);
        let mut t = 0.0;
        while s.player.state == PState::Dodge {
            assert!(s.player.is_invulnerable(), "vulnerable mid-dodge at {:.2}s", t);
            run(&mut s, 0.01, InputState::default());
            t += 0.01;
        }
    }

    /// Stand in a portal and press Up.
    fn take_portal(s: &mut Sim, target: &str) {
        let p = s.map.portals.iter().find(|p| p.target_map == target).unwrap_or_else(|| panic!("no portal {} -> {}", s.map.key, target)).clone();
        let c = p.area.center();
        s.player.body.pos.x = c.x - PLAYER_W * 0.5;
        s.player.body.pos.y = p.area.bottom() - PLAYER_H - 1.0;
        run(s, 0.4, InputState::default());
        run(s, 0.1, press(&[Action::Up], &[]));
        assert_eq!(s.map.key, target, "portal to {} didn't work", target);
        run(s, 0.3, InputState::default());
    }

    /// Talk to an NPC on this map, step through the dialog and pick choice 0.
    fn talk_and_accept(s: &mut Sim, npc: &str) {
        let n = s.map.npcs.iter().find(|n| n.id == npc).unwrap_or_else(|| panic!("{} not on {}", npc, s.map.key)).clone();
        s.player.body.pos.x = n.x - 60.0;
        run(s, 0.5, InputState::default());
        run(s, 0.05, press(&[Action::Interact], &[]));
        assert!(s.dialog.is_some(), "no dialog with {}", npc);
        for _ in 0..12 {
            if s.dialog.is_none() {
                break;
            }
            run(s, 0.05, press(&[Action::Interact], &[]));
        }
        assert!(s.dialog.is_none());
    }

    #[test]
    fn new_characters_are_novices_and_earn_ap() {
        let mut s = new_sim(ClassId::Novice);
        assert_eq!(s.save.class, ClassId::Novice);
        assert_eq!(s.save.inventory.equip[0].as_deref(), Some("gnarled_stick"));
        let hp1 = s.player.max_hp;
        s.gain_exp(exp_to_next(1) + exp_to_next(2));
        assert_eq!(s.save.level, 3);
        assert_eq!(s.save.ap, 10);
        assert!(s.player.max_hp > hp1);
        let str0 = s.save.stats.str;
        let hp_before = s.player.max_hp;
        assert!(s.spend_ap(MainStat::Str));
        assert_eq!(s.save.stats.str, str0 + 1);
        assert!(s.player.max_hp > hp_before, "STR adds HP");
        // Second Wind is learned at level 3 and lands on the hotbar.
        assert!(s.save.hotbar.contains(&HotbarSlot::Skill("second_wind".into())));
    }

    #[test]
    fn novice_walks_to_ironhold_and_becomes_a_vanguard() {
        let mut s = new_sim(ClassId::Novice);
        s.save.level = 10;
        s.save.ap = 45;
        s.auto_spend_ap();
        s.recompute_stats();
        s.change_map("breezehollow", 3000.0);
        take_portal(&mut s, "wayfarers_crossroads");
        take_portal(&mut s, "ironhold_road");
        take_portal(&mut s, "ironhold");
        assert!(s.save.has_flag("ws_ironhold"), "waystone attuned on arrival");
        // Too low? The instructor refuses below level 10 (checked in dialog).
        talk_and_accept(&mut s, "instructor_ironhold");
        assert_eq!(s.save.class, ClassId::Vanguard);
        assert_eq!(s.player.info.name, "Vanguard");
        assert_eq!(s.save.inventory.equip[0].as_deref(), Some("iron_sword"));
        assert_eq!(s.save.inventory.count("gnarled_stick"), 1, "old stick goes to the bag");
        assert_eq!(s.save.ap, 45, "points refunded");
        assert!(s.save.hotbar.contains(&HotbarSlot::Skill("shield_slam".into())));
        // Can't change twice.
        assert!(s.change_class(ClassId::Duskblade).is_err());
        let d = quests::talk("instructor_shadowmere", "Mistress Vey", &mut s.save);
        assert_eq!(d.action, DialogAction::None);
    }

    #[test]
    fn low_level_novice_cannot_change_class() {
        let mut s = new_sim(ClassId::Novice);
        let d = quests::talk("instructor_starfall", "Archmage Solenne", &mut s.save);
        assert_eq!(d.action, DialogAction::None);
        assert!(s.change_class(ClassId::Aethermancer).is_err());
    }

    #[test]
    fn shops_buy_sell_and_equip() {
        let mut s = new_sim(ClassId::Novice);
        s.save.level = 6;
        s.save.gold = 500;
        s.recompute_stats();
        s.change_map("ironhold", 400.0);
        take_portal(&mut s, "ironhold_armor_in");
        talk_and_accept(&mut s, "ironhold_armor");
        assert_eq!(s.shop_open, Some("ironhold_armor"));
        assert!(s.buy("leather_vest", 1).is_ok());
        assert_eq!(s.save.gold, 410);
        assert!(s.buy("plate_armor", 1).is_err(), "can't afford");
        assert!(s.buy("hp_potion", 1).is_err(), "not sold here");
        let pdef = s.player.stats.pdef;
        let idx = s.save.inventory.slots.iter().position(|x| x.id == "leather_vest").unwrap();
        s.equip_from_slot(idx).unwrap();
        assert!(s.player.stats.pdef > pdef);
        assert!(s.equip_problem("plate_armor").is_some(), "level 15 item");
        assert!(s.equip_problem("iron_sword").is_some(), "Vanguard only");
        let idx = s.save.inventory.slots.iter().position(|x| x.id == "hp_potion").unwrap();
        let gold = s.save.gold;
        assert_eq!(s.sell(idx, 1).unwrap(), 6);
        assert_eq!(s.save.gold, gold + 6);
        take_portal(&mut s, "ironhold");
        assert!((s.player.body.feet().y - world::GROUND).abs() < 2.0, "arrived on a rooftop instead of the street");
        assert_eq!(s.shop_open, None, "leaving closes the shop");
        assert!(s.buy("leather_vest", 1).is_err());
    }

    #[test]
    fn abilities_cost_mana_go_on_cooldown_and_hit() {
        let mut s = new_sim(ClassId::Novice);
        s.change_map("dawnwind_glade", 2900.0);
        run(&mut s, 0.5, InputState::default());
        s.player.facing = 1.0;
        let mp = s.player.mp;
        run(&mut s, 0.02, press(&[Action::Hotbar1], &[]));
        assert!(s.player.mp < mp, "Power Strike costs mana");
        assert!(s.player.cooldown("power_strike") > 0.0);
        assert_eq!(s.player.attack.as_ref().and_then(|a| a.skill).map(|k| k.id), Some("power_strike"));
        let mp2 = s.player.mp;
        run(&mut s, 1.2, InputState::default());
        run(&mut s, 0.02, press(&[Action::Hotbar1], &[]));
        assert!(s.player.mp >= mp2, "still on cooldown, no mana spent");
        // Buffs raise stats and wear off.
        s.save.level = 6;
        s.recompute_stats();
        let brace = skills::skill("brace").unwrap();
        let pdef = s.player.stats.pdef;
        assert!(s.use_skill(brace));
        assert!(s.player.stats.pdef > pdef * 1.3);
        run(&mut s, 16.0, InputState::default());
        assert!((s.player.stats.pdef - pdef).abs() < 0.01, "buff expired");
    }

    #[test]
    fn every_class_ability_runs() {
        for &c in classes::ALL_CLASSES {
            let mut s = new_sim(c);
            s.save.level = 20;
            s.recompute_stats();
            for sk in s.known_skills() {
                s.player.mp = s.player.max_mp;
                s.player.state = PState::Normal;
                s.player.attack = None;
                assert!(s.use_skill(sk), "{:?} {}", c, sk.id);
                run(&mut s, 2.0, InputState::default());
                assert_eq!(s.player.state, PState::Normal, "{} never finished", sk.id);
            }
        }
    }

    #[test]
    fn every_damaging_ability_hurts_a_monster_in_reach() {
        for &c in classes::ALL_CLASSES {
            for sk in skills::class_skills(c) {
                if !matches!(sk.effect, SkillEffect::Strike { .. }) {
                    continue;
                }
                let mut s = new_sim(c);
                s.save.level = 20;
                s.recompute_stats();
                s.change_map("dawnwind_glade", 2900.0);
                s.enemies.clear();
                run(&mut s, 0.5, InputState::default());
                s.player.facing = 1.0;
                let feet = s.player.body.pos.y + s.player.body.h;
                let x = s.player.body.center().x + 70.0;
                s.spawn_enemy(EnemyKind::Rotstump, x, feet, None);
                s.enemies[0].hp = 1.0e6;
                let hp = s.enemies[0].hp;
                s.player.mp = s.player.max_mp;
                assert!(s.use_skill(sk), "{:?} {}", c, sk.id);
                run(&mut s, 1.5, InputState::default());
                assert!(s.enemies.is_empty() || s.enemies[0].hp < hp, "{:?} {} did no damage", c, sk.id);
            }
        }
    }

    #[test]
    fn defence_and_crit_stats_apply() {
        let mut s = new_sim(ClassId::Vanguard);
        s.change_map("breezehollow", 1500.0);
        run(&mut s, 0.5, InputState::default());
        s.player.invuln = 0.0;
        let hp = s.player.hp;
        s.hurt_player(40.0, 0.0, V2::ZERO, false);
        let soft = hp - s.player.hp;
        s.save.inventory.equip[1] = Some("plate_armor".into());
        s.recompute_stats();
        s.player.hp = s.player.max_hp;
        let hp = s.player.hp;
        s.player.invuln = 0.0;
        s.player.state = PState::Normal;
        s.hurt_player(40.0, 0.0, V2::ZERO, false);
        let hard = hp - s.player.hp;
        assert!(hard < soft, "armour reduces damage ({} vs {})", hard, soft);
    }

    #[test]
    fn climbing_ladders_reaches_the_shelves() {
        for w in world::WILDS {
            let mut s = new_sim(ClassId::Vanguard);
            s.change_map(w.key, 220.0);
            run(&mut s, 0.3, InputState::default());
            let c = s.map.climbs[0].clone();
            s.player.body.pos.x = c.x - PLAYER_W * 0.5;
            s.player.body.pos.y = c.bottom - PLAYER_H;
            s.player.body.on_ground = true;
            s.player.invuln = 99.0;
            run(&mut s, 0.1, press(&[Action::Up], &[Action::Up]));
            assert_eq!(s.player.state, PState::Climb, "{}: couldn't grab", w.key);
            for _ in 0..40 {
                s.player.invuln = 99.0;
                run(&mut s, 0.05, press(&[], &[Action::Up]));
                if s.player.state != PState::Climb {
                    break;
                }
            }
            assert_eq!(s.player.state, PState::Normal, "{}: never reached the top", w.key);
            run(&mut s, 0.3, InputState::default());
            assert!((s.player.body.feet().y - c.top).abs() < 2.0, "{}: standing at {} not on the shelf {}", w.key, s.player.body.feet().y, c.top);
            // and back down
            run(&mut s, 0.05, press(&[Action::Down], &[Action::Down]));
            assert_eq!(s.player.state, PState::Climb, "{}: couldn't grab from the top", w.key);
            run(&mut s, 3.0, press(&[], &[Action::Down]));
            assert!((s.player.body.feet().y - c.bottom).abs() < 2.0, "{}: didn't climb down", w.key);
        }
    }

    #[test]
    fn bot_walks_every_road() {
        for w in world::WILDS.iter().filter(|w| w.east.is_some()) {
            let mut s = new_sim(ClassId::Windrunner);
            s.change_map(w.key, 220.0);
            let east = s.map.portals.iter().map(|p| p.area.left()).fold(0.0, f32::max);
            let mut t = 0.0;
            let mut jumped_at = -1.0f32;
            while t < 60.0 && s.player.body.pos.x < east - 40.0 {
                s.player.invuln = 1.0;
                let p = &s.player;
                let blocked = p.body.hit_wall || p.body.vel.x.abs() < 40.0;
                let mut pressed = vec![];
                if p.body.on_ground && blocked && t - jumped_at > 0.25 {
                    pressed.push(Action::Jump);
                    jumped_at = t;
                }
                s.tick(&press(&pressed, &[Action::Right, Action::Jump]));
                t += TICK;
            }
            assert!(s.player.body.pos.x >= east - 40.0, "{}: stuck at x={}", w.key, s.player.body.pos.x);
        }
    }

    #[test]
    fn monsters_drop_loot_and_gold_on_the_ground() {
        let mut s = new_sim(ClassId::Vanguard);
        s.change_map("old_quarry", 220.0);
        run(&mut s, 0.5, InputState::default());
        s.enemies.clear();
        let x = s.player.body.center().x;
        for _ in 0..30 {
            s.spawn_enemy(EnemyKind::QuarryCrab, x + 30.0, world::GROUND, None);
            let i = s.enemies.len() - 1;
            s.kill_enemy(i);
        }
        assert_eq!(s.save.inventory.count("crab_shell"), 0, "nothing goes straight into the bag");
        let shells = s.drops.iter().filter(|d| d.kind == DropKind::Item("crab_shell")).count();
        assert!(shells > 5, "crab shells should drop often ({})", shells);
        assert!(s.drops.iter().any(|d| d.kind == DropKind::Gold), "gold drops too");
        // Let them land, then pick everything up with F (hold).
        for _ in 0..20 {
            s.player.invuln = 9.0;
            run(&mut s, 0.1, InputState::default());
        }
        assert!(s.drops.iter().all(|d| d.body.on_ground), "drops settle on the ground");
        let gold0 = s.save.gold;
        let n0 = s.drops.len();
        run(&mut s, 0.02, press(&[Action::Pickup], &[Action::Pickup]));
        run(&mut s, 0.3, InputState::default());
        assert_eq!(s.drops.len(), n0 - 1, "one press picks up one drop");
        for _ in 0..60 {
            s.player.invuln = 9.0;
            // keep collecting what's in reach; walk to the rest
            if let Some(d) = s.drops.iter().find(|d| d.taken.is_none()) {
                s.player.body.pos.x = d.body.center().x - PLAYER_W * 0.5;
            }
            run(&mut s, 0.15, press(&[], &[Action::Pickup]));
        }
        run(&mut s, 0.5, InputState::default());
        assert!(s.drops.is_empty(), "{} drops left", s.drops.len());
        assert!(s.save.gold > gold0);
        assert_eq!(s.save.inventory.count("crab_shell") as usize, shells);
    }

    #[test]
    fn pets_bag_full_and_despawn() {
        let mut s = new_sim(ClassId::Vanguard);
        run(&mut s, 0.5, InputState::default());
        let c = s.player.body.center();
        s.spawn_drop(DropKind::Item("moss_clump"), 3, c, 0.0);
        s.spawn_drop(DropKind::Gold, 12, c, 0.0);
        run(&mut s, 1.0, InputState::default());
        // A pet picks things up for its owner.
        let gold = s.save.gold;
        assert_eq!(s.pet_collect(1, c, 120.0, 5), 2);
        assert_eq!(s.save.inventory.count("moss_clump"), 3);
        assert_eq!(s.save.gold, gold + 12);
        // Bag full: the drop stays on the ground.
        for _ in 0..30 {
            s.save.inventory.add("copper_ring", 1);
        }
        s.spawn_drop(DropKind::Item("wool_tuft"), 1, c, 0.0);
        run(&mut s, 1.0, InputState::default());
        assert!(!s.pick_up());
        assert_eq!(s.drops.len(), 1);
        // Loot vanishes after a while.
        for _ in 0..15 {
            s.player.invuln = 99.0;
            run(&mut s, 10.0, InputState::default());
        }
        assert!(s.drops.is_empty());
    }

    #[test]
    fn dropping_items_but_not_bound_ones() {
        let mut s = new_sim(ClassId::Novice);
        run(&mut s, 0.5, InputState::default());
        s.save.inventory.add("wool_tuft", 2);
        s.save.inventory.add("trainee_hp_potion", 2);
        let i = s.save.inventory.slots.iter().position(|x| x.id == "wool_tuft").unwrap();
        assert!(s.drop_item(i, 2).is_ok());
        assert_eq!(s.drops.len(), 1);
        let i = s.save.inventory.slots.iter().position(|x| x.id == "trainee_hp_potion").unwrap();
        assert!(s.drop_item(i, 1).is_err(), "bound");
        assert_eq!(s.save.inventory.count("trainee_hp_potion"), 2);
    }

    #[test]
    fn tutorial_vendor_discounts_bound_potions_and_buys_loot() {
        let mut s = new_sim(ClassId::Novice);
        s.save.gold = 1000;
        s.change_map("dawnwind_glade", 3300.0);
        talk_and_accept(&mut s, "quartermaster_hesper");
        assert_eq!(s.shop_open, Some(items::GLADE_SHOP));
        // First 10 trainee potions are 40% off (15 -> 9 gold).
        assert_eq!(s.unit_price("trainee_hp_potion"), 9);
        assert_eq!(s.buy("trainee_hp_potion", 6).unwrap(), 54);
        assert_eq!(s.buy("trainee_mp_potion", 6).unwrap(), 4 * 11 + 2 * 18, "4 discounted left, then full price");
        assert_eq!(s.trainee_discounts_left(), 0);
        assert_eq!(s.unit_price("trainee_hp_potion"), 15);
        // Level 8 gear is on sale.
        assert!(s.buy("ironwood_blade", 1).is_ok());
        assert!(s.equip_problem("ironwood_blade").is_some(), "needs level 8");
        // Bound potions can't be sold; monster loot can.
        let i = s.save.inventory.slots.iter().position(|x| x.id == "trainee_hp_potion").unwrap();
        assert!(s.sell(i, 1).is_err());
        s.save.inventory.add("moss_clump", 5);
        let i = s.save.inventory.slots.iter().position(|x| x.id == "moss_clump").unwrap();
        assert_eq!(s.sell(i, 5).unwrap(), 10);
        // The Potion key drinks trainee potions too.
        s.player.hp = 1.0;
        let n = s.save.inventory.count("hp_potion") + s.save.inventory.count("trainee_hp_potion");
        s.drink_potion();
        assert_eq!(s.save.inventory.count("hp_potion") + s.save.inventory.count("trainee_hp_potion"), n - 1);
    }

    #[test]
    fn class_change_awards_a_skill_reset_coupon() {
        let mut s = new_sim(ClassId::Novice);
        s.save.level = 10;
        s.save.ap = 45;
        s.recompute_stats();
        s.change_class(ClassId::Stormfist).unwrap();
        assert_eq!(s.save.inventory.count("skill_reset_coupon"), 1);
        assert!(s.reset_ability_points().is_err(), "nothing spent yet");
        s.auto_spend_ap();
        assert_eq!(s.save.ap, 0);
        let i = s.save.inventory.slots.iter().position(|x| x.id == "skill_reset_coupon").unwrap();
        s.activate_slot(i);
        assert_eq!(s.save.ap, 45, "all points refunded");
        assert_eq!(s.save.stats, stats::START_STATS);
        assert_eq!(s.save.inventory.count("skill_reset_coupon"), 0, "coupon used up");
        let i = s.save.inventory.slots.len();
        s.save.inventory.add("skill_reset_coupon", 1);
        assert!(s.drop_item(i, 1).is_err(), "coupon is bound");
    }

    #[test]
    fn tutorial_quests_hand_out_novice_gear() {
        let mut s = new_sim(ClassId::Novice);
        quests::accept(&mut s.save, "first_breeze");
        quests::on_event(&mut s.save, QuestEvent::Reached("lookout"));
        s.change_map("dawnwind_glade", 2300.0);
        talk_and_accept(&mut s, "scout_tamsin");
        assert_eq!(s.save.inventory.count("novice_charm"), 1);
        talk_and_accept(&mut s, "scout_tamsin"); // accept Gust Dodger
        for _ in 0..3 {
            s.player.dodge_cd = 0.0;
            run(&mut s, 0.02, press(&[Action::Dodge], &[]));
            run(&mut s, 0.7, InputState::default());
        }
        s.player.body.pos.x = 2300.0;
        talk_and_accept(&mut s, "scout_tamsin");
        assert_eq!(s.save.inventory.count("novice_boots"), 1);
    }

    #[test]
    fn every_main_town_has_a_pet_shop() {
        for town in ["breezehollow", "ironhold", "shadowmere", "tempest_monastery", "leafwhisper", "starfall_spire"] {
            let m = world::load_map(town).unwrap();
            let id = world::pet_keeper_id(town);
            assert!(m.npcs.iter().any(|n| n.id == id), "{} has no pet keeper", town);
            assert_eq!(quests::npc_shop(id), Some(items::PET_SHOP));
        }
        let shop = items::shop(items::PET_SHOP).unwrap();
        for &sp in pets::ALL_SPECIES {
            assert_eq!(items::item(sp.item_id()).unwrap().price, 10_000);
            assert!(shop.stock.contains(&sp.item_id()));
        }
        assert_eq!(items::item("pet_food").unwrap().price, 100);
    }

    #[test]
    fn buying_and_adopting_a_pet_that_loots_for_you() {
        let mut s = new_sim(ClassId::Vanguard);
        s.save.gold = 9_999;
        s.change_map("breezehollow", 2500.0);
        talk_and_accept(&mut s, "pet_keeper_breezehollow");
        assert_eq!(s.shop_open, Some(items::PET_SHOP));
        assert!(s.buy("pet_breeze_fox", 1).is_err(), "10,000 gold");
        s.save.gold = 10_300;
        assert_eq!(s.buy("pet_breeze_fox", 1).unwrap(), 10_000);
        assert_eq!(s.buy("pet_food", 3).unwrap(), 300);
        assert_eq!(s.save.gold, 0);
        // Use it from the bag to adopt: it comes out straight away.
        let i = s.save.inventory.slots.iter().position(|x| x.id == "pet_breeze_fox").unwrap();
        s.activate_slot(i);
        assert_eq!(s.save.pets.len(), 1);
        assert_eq!(s.save.active_pet, Some(0));
        assert_eq!(s.save.inventory.count("pet_breeze_fox"), 0);
        run(&mut s, 0.5, InputState::default());
        assert!(s.pet.is_some(), "the pet appears in the world");
        // Loot near you is fetched automatically; loot far away isn't.
        let c = s.player.body.center();
        s.spawn_drop(DropKind::Gold, 100, V2::new(c.x + 200.0, c.y), 0.0);
        s.spawn_drop(DropKind::Item("moss_clump"), 1, V2::new(c.x - 150.0, c.y), 0.0);
        s.spawn_drop(DropKind::Item("wool_tuft"), 1, V2::new(c.x + 1500.0, c.y), 0.0);
        run(&mut s, 4.0, InputState::default());
        assert_eq!(s.save.gold, 100, "picked up by the pet (Gold Sense is level 5)");
        assert_eq!(s.save.inventory.count("moss_clump"), 1);
        assert_eq!(s.save.inventory.count("wool_tuft"), 0, "out of range");
        assert_eq!(s.drops.len(), 1);
        // The pet stays close behind you.
        let pet = s.pet.as_ref().unwrap();
        assert!((pet.pos.x - s.player.body.feet().x).abs() < 80.0);
    }

    #[test]
    fn pet_hunger_feeding_levels_and_passives() {
        let mut s = new_sim(ClassId::Vanguard);
        s.change_map("breezehollow", 1500.0);
        s.save.inventory.add("pet_puffling", 1);
        assert!(s.use_item("pet_puffling"));
        let pdef_with = s.player.stats.pdef;
        // Hunger drains while out: about 30 minutes from full.
        s.save.pets[0].hunger = 1.0;
        run(&mut s, 20.0, InputState::default());
        assert!(s.save.pets[0].starving());
        assert!(s.player.stats.pdef < pdef_with, "passives switch off when starving");
        // A starving pet doesn't loot.
        let c = s.player.body.center();
        s.spawn_drop(DropKind::Item("moss_clump"), 1, V2::new(c.x + 120.0, c.y), 0.0);
        run(&mut s, 3.0, InputState::default());
        assert_eq!(s.save.inventory.count("moss_clump"), 0);
        // Feed it: hunger back, EXP up, passives back.
        assert!(s.feed_pet().is_err(), "no food");
        s.save.inventory.add("pet_food", 30);
        s.set_hotbar(3, HotbarSlot::Item("pet_food".into()));
        run(&mut s, 0.02, press(&[Action::Hotbar4], &[]));
        assert!(s.save.pets[0].hunger > 25.0);
        assert_eq!(s.save.pets[0].exp, pets::FOOD_EXP);
        assert!((s.player.stats.pdef - pdef_with).abs() < 0.01, "passive back on");
        run(&mut s, 3.0, InputState::default());
        assert_eq!(s.save.inventory.count("moss_clump"), 1, "fed pets loot again");
        // Level 5 unlocks Cosy Fleece (HP regen).
        while s.save.pets[0].level < 5 {
            s.feed_pet().unwrap();
        }
        s.player.hp = 10.0;
        s.player.since_hurt = 0.0;
        run(&mut s, 2.0, InputState::default());
        assert!(s.player.hp > 10.0 + s.player.max_hp * 0.005 * 1.5, "pet heals you");
        // Sending it home stops the passives; hunger stays where it was.
        let hunger = s.save.pets[0].hunger;
        s.dismiss_pet();
        assert!(s.pet.is_none());
        assert!(s.player.stats.pdef < pdef_with);
        run(&mut s, 10.0, InputState::default());
        assert_eq!(s.save.pets[0].hunger, hunger, "no hunger loss at home");
        // A second pet: summoning swaps.
        s.save.inventory.add("pet_sparkwing", 1);
        assert!(s.use_item("pet_sparkwing"));
        assert_eq!(s.save.active_pet, Some(1), "no pet was out, so the new one comes out");
        assert_eq!(s.active_pet().unwrap().species, PetSpecies::Sparkwing);
        s.summon_pet(0).unwrap();
        assert_eq!(s.active_pet().unwrap().species, PetSpecies::Puffling, "summoning swaps pets");
    }

    #[test]
    fn fox_gold_sense_adds_gold() {
        let mut s = new_sim(ClassId::Vanguard);
        s.save.pets.push(PetSave { level: 5, ..PetSave::new(PetSpecies::BreezeFox) });
        s.summon_pet(0).unwrap();
        run(&mut s, 0.5, InputState::default());
        let c = s.player.body.center();
        s.spawn_drop(DropKind::Gold, 100, c, 0.0);
        run(&mut s, 3.0, InputState::default());
        assert_eq!(s.save.gold, 110);
    }

    fn online_sim() -> Sim {
        let mut s = new_sim(ClassId::Vanguard);
        s.change_map("dawnwind_glade", 3700.0);
        s.go_online(7);
        run(&mut s, 0.5, InputState::default());
        s.outbox.clear();
        s
    }

    fn enemy_net(id: u32, kind: EnemyKind, x: f32, y: f32, hp: f32) -> EnemyNet {
        EnemyNet { id, kind: kind.index(), x, y, vx: 0.0, vy: 0.0, facing: -1, state: 0, state_t: 0.0, hp, max_hp: 50.0, phase2: false }
    }

    #[test]
    fn online_monsters_come_from_the_server() {
        let mut s = online_sim();
        assert!(s.enemies.is_empty(), "no local monsters online");
        let f = s.player.body.feet();
        let ey = f.y - EnemyKind::Mossling.def().h;
        s.net_apply(ServerMsg::Enemies(vec![enemy_net(500, EnemyKind::Mossling, f.x + 30.0, ey, 50.0)]));
        assert_eq!(s.enemies.len(), 1);
        // Our hits are reported, not applied: the server owns their HP.
        s.player.facing = 1.0;
        s.player.invuln = 99.0;
        run(&mut s, 0.02, press(&[Action::Light], &[]));
        run(&mut s, 0.6, InputState::default());
        let hit = s.outbox.iter().find_map(|m| if let ClientMsg::Hit { enemy, damage, .. } = m { Some((*enemy, *damage)) } else { None });
        assert!(matches!(hit, Some((500, d)) if d > 0.0), "hit reported: {:?}", s.outbox);
        assert_eq!(s.enemies[0].hp, 50.0, "HP unchanged until the server says so");
        // Snapshots move it smoothly and update its HP.
        s.net_apply(ServerMsg::Enemies(vec![enemy_net(500, EnemyKind::Mossling, f.x + 90.0, ey, 20.0)]));
        run(&mut s, 0.5, InputState::default());
        assert!((s.enemies[0].body.pos.x - (f.x + 90.0)).abs() < 3.0);
        assert_eq!(s.enemies[0].hp, 20.0);
        // The kill comes from the server, with EXP and quest credit.
        quests::accept(&mut s.save, "mossling_trouble");
        let exp = s.save.exp;
        s.net_apply(ServerMsg::Kill { enemy: 500, kind: EnemyKind::Mossling.index(), exp: 9 });
        assert_eq!(s.save.exp, exp + 9);
        assert_eq!(s.save.quest("mossling_trouble").unwrap().progress, 1);
        // Gone from the next snapshot = gone here.
        s.net_apply(ServerMsg::Enemies(vec![]));
        assert!(s.enemies.is_empty());
        // Monster shots arrive and can hurt us.
        s.player.invuln = 0.0;
        s.player.state = PState::Normal;
        let hp = s.player.hp;
        let c = s.player.body.center();
        s.net_apply(ServerMsg::Shot(net::ShotNet { x: c.x - 60.0, y: c.y, vx: 600.0, vy: 0.0, size: 14.0, gravity: 0.0, life: 2.0, pierce: false, damage: 20.0, kbx: 200.0, kby: -100.0, style: 2, grounded: false }));
        run(&mut s, 0.3, InputState::default());
        assert!(s.player.hp < hp, "shot hit");
    }

    #[test]
    fn online_loot_is_claimed_through_the_server() {
        let mut s = online_sim();
        let c = s.player.body.center();
        s.net_apply(ServerMsg::DropAdd(DropNet { id: 900, item: "moss_clump".into(), count: 2, x: c.x + 20.0, y: c.y, vx: 0.0, vy: -100.0 }));
        s.net_apply(ServerMsg::DropAdd(DropNet { id: 901, item: net::GOLD.into(), count: 30, x: c.x + 40.0, y: c.y, vx: 0.0, vy: -100.0 }));
        run(&mut s, 1.0, InputState::default());
        assert_eq!(s.drops.len(), 2, "drops fall onto the ground here too");
        run(&mut s, 0.02, press(&[Action::Pickup], &[]));
        let asked: Vec<u32> = s.outbox.iter().filter_map(|m| if let ClientMsg::PickUp { drop, .. } = m { Some(*drop) } else { None }).collect();
        assert_eq!(asked.len(), 1);
        assert_eq!(s.save.inventory.count("moss_clump") + s.save.gold, 0, "nothing until the server agrees");
        s.net_apply(ServerMsg::Granted { drop: 900, item: "moss_clump".into(), count: 2 });
        assert_eq!(s.save.inventory.count("moss_clump"), 2);
        // Someone else takes the gold.
        s.net_apply(ServerMsg::DropGone { id: 901, by: 3 });
        run(&mut s, 0.5, InputState::default());
        assert!(s.drops.is_empty());
        assert_eq!(s.save.gold, 0);
        // Dropping an item sends it to the server for everyone to see.
        s.outbox.clear();
        let i = s.save.inventory.slots.iter().position(|x| x.id == "moss_clump").unwrap();
        s.drop_item(i, 1).unwrap();
        assert!(s.outbox.iter().any(|m| matches!(m, ClientMsg::DropItem(d) if d.item == "moss_clump" && d.count == 1)));
        assert!(s.drops.is_empty(), "it appears when the server confirms");
    }

    #[test]
    fn online_players_and_revives() {
        let mut s = online_sim();
        let f = s.player.body.feet();
        s.net_apply(ServerMsg::PlayerInfo(PlayerInfo { id: 3, name: "Friend".into(), level: 9, ..Default::default() }));
        s.net_apply(ServerMsg::PlayerPos(PlayerPos { id: 3, x: f.x + 40.0, y: f.y, dead: true, ..Default::default() }));
        run(&mut s, 0.2, InputState::default());
        assert_eq!(s.others.len(), 1);
        assert!(s.others[0].dead);
        assert_eq!(s.revivable(), Some(0));
        run(&mut s, 0.02, press(&[Action::Interact], &[]));
        assert!(s.outbox.iter().any(|m| *m == ClientMsg::Revive { target: 3 }));
        // We fall; a friend revives us (keeping the EXP loss).
        s.save.level = 12;
        s.save.exp = 1000;
        s.player.invuln = 0.0;
        s.player.state = PState::Normal;
        s.hurt_player(99999.0, 0.0, V2::ZERO, false);
        assert_eq!(s.save.exp, 970);
        s.net_apply(ServerMsg::Revived { by: "Friend".into() });
        assert_ne!(s.player.state, PState::Dead);
        assert_eq!(s.save.exp, 970);
        // Chat and leaving.
        s.net_apply(ServerMsg::Chat { from: "Friend".into(), text: "welcome back".into() });
        assert_eq!(s.chat.last().unwrap().1, "welcome back");
        s.net_apply(ServerMsg::PlayerGone { id: 3 });
        assert!(s.others.is_empty());
        // Quest bosses are requested from the server instead of spawned.
        s.outbox.clear();
        quests::accept(&mut s.save, "the_bramblehorn");
        s.change_map("dawnwind_glade", 5000.0);
        assert!(s.outbox.iter().any(|m| *m == ClientMsg::Boss { kind: EnemyKind::Bramblehorn.index(), map: "dawnwind_glade".into() }));
        assert!(s.enemies.is_empty());
    }

    #[test]
    fn waystones_fast_travel() {
        let mut s = new_sim(ClassId::Vanguard);
        s.save.gold = 100;
        s.change_map("breezehollow", 1500.0);
        assert!(s.save.has_flag("ws_breezehollow"));
        assert!(s.teleport("ironhold").is_err(), "not attuned yet");
        s.save.set_flag("ws_ironhold");
        assert!(s.teleport("ironhold").is_ok());
        assert_eq!(s.map.key, "ironhold");
        assert_eq!(s.save.gold, 100 - WAYSTONE_COST);
        assert!(s.teleport("ironhold").is_err(), "already here");
    }
}
