//! Quest definitions, progress tracking and NPC dialogue.

use crate::save::{CharacterSave, QuestEntry, QuestState};
use crate::classes::{ClassId, CLASS_CHANGE_LEVEL};
use crate::world::EnemyKind;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Objective {
    Reach(&'static str),
    Dodge(u32),
    DummyLight(u32),
    DummyHeavy(u32),
    Kill(EnemyKind, u32),
    TalkTo(&'static str),
    TalkMany(&'static [&'static str]),
    /// Change from Novice to any class.
    ChangeClass,
}

impl Objective {
    pub fn target(&self) -> u32 {
        match *self {
            Objective::Reach(_) | Objective::TalkTo(_) | Objective::ChangeClass => 1,
            Objective::Dodge(n) | Objective::DummyLight(n) | Objective::DummyHeavy(n) | Objective::Kill(_, n) => n,
            Objective::TalkMany(l) => l.len() as u32,
        }
    }
}

#[derive(Clone, Debug)]
pub struct QuestDef {
    pub id: &'static str,
    pub name: &'static str,
    pub giver: &'static str,
    pub turn_in: &'static str,
    pub requires: Option<&'static str>,
    /// Minimum character level before the quest is offered.
    pub min_level: u32,
    pub objective: Objective,
    pub summary: &'static str,
    pub offer: &'static [&'static str],
    pub reminder: &'static str,
    pub complete: &'static [&'static str],
    pub exp: u32,
    pub gold: u32,
    pub potions: u32,
    /// Item rewards (id, count) handed over on completion.
    pub items: &'static [(&'static str, u32)],
    /// Story flag set when the quest is accepted / completed.
    pub flag_on_accept: Option<&'static str>,
    pub flag_on_complete: Option<&'static str>,
    /// Quest automatically given after this one is completed.
    pub chain_to: Option<&'static str>,
}

pub const QUESTS: &[QuestDef] = &[
    QuestDef {
        id: "first_breeze",
        name: "First Breeze",
        giver: "elder_wynn",
        turn_in: "scout_tamsin",
        requires: None,
        min_level: 0,
        objective: Objective::Reach("lookout"),
        summary: "Climb Windstep Rise and reach the Lookout. Jump again in mid-air to double jump.",
        offer: &[
            "Ah, you're awake! The wind carried you to our glade last night, traveller.",
            "Before you head for Breezehollow, let's see you move. Climb Windstep Rise to the Lookout.",
            "Press {Jump} to jump, and press it again in mid-air to double jump. Scout Tamsin waits past the Rise.",
        ],
        reminder: "The Lookout is atop Windstep Rise, to the east. Double jump with {Jump} in mid-air!",
        complete: &["You made it up the Rise? The Elder sent word you'd come. Not bad for a newcomer!", "She asked me to give you this - her wind charm. It's brought luck to every traveller who's worn it."],
        exp: 20,
        gold: 5,
        potions: 0,
        items: &[("novice_charm", 1)],
        flag_on_accept: None,
        flag_on_complete: None,
        chain_to: None,
    },
    QuestDef {
        id: "gust_dodger",
        name: "Gust Dodger",
        giver: "scout_tamsin",
        turn_in: "scout_tamsin",
        requires: Some("first_breeze"),
        min_level: 0,
        objective: Objective::Dodge(3),
        summary: "Dodge 3 times with {Dodge}. You're invulnerable during the start of a dodge.",
        offer: &[
            "Out here, the one who doesn't get hit wins. Watch: press {Dodge} to dodge.",
            "You can dodge on the ground or in the air, and hold a direction to dodge that way. Show me three!",
        ],
        reminder: "Press {Dodge} to dodge. Three times!",
        complete: &["Light on your feet! Take these potions - press {Potion} to drink one in a pinch.", "And have my old scouting boots. They'll keep you quick."],
        exp: 20,
        gold: 5,
        potions: 2,
        items: &[("novice_boots", 1)],
        flag_on_accept: None,
        flag_on_complete: None,
        chain_to: None,
    },
    QuestDef {
        id: "sparring_basics",
        name: "Sparring Basics",
        giver: "sergeant_brask",
        turn_in: "sergeant_brask",
        requires: Some("gust_dodger"),
        min_level: 0,
        objective: Objective::DummyLight(6),
        summary: "Hit the training dummies 6 times with Light attacks ({Light}).",
        offer: &[
            "ATTENTION! You want to survive past the Hollow? Then you train.",
            "{Light} is your light attack. Hold {Left}/{Right} for a lunge, {Down} for a sweep. In the air they change again.",
            "Six clean light hits on my dummies. Move it!",
        ],
        reminder: "Six light hits ({Light}) on the dummies, recruit!",
        complete: &["Acceptable. Mix directions - a sweep into an air slash is a classic combo.", "Take my spare practice sword. It hits harder than that stick of yours. Equip it from your bag ({Inventory})."],
        exp: 25,
        gold: 5,
        potions: 0,
        items: &[("novice_sword", 1)],
        flag_on_accept: None,
        flag_on_complete: None,
        chain_to: None,
    },
    QuestDef {
        id: "signature_moves",
        name: "Signature Moves",
        giver: "sergeant_brask",
        turn_in: "sergeant_brask",
        requires: Some("sparring_basics"),
        min_level: 0,
        objective: Objective::DummyHeavy(3),
        summary: "Land 3 Heavy attacks ({Heavy}) on the dummies. Try them with different directions!",
        offer: &[
            "Now the good stuff. {Heavy} unleashes your class's signature moves.",
            "Neutral, side and down heavies are all different - and {Heavy} in the air is your recovery. Land three!",
        ],
        reminder: "Three heavy hits ({Heavy})! Some cost MP - watch the blue bar.",
        complete: &[
            "Now THAT'S a fighter. Ranger Pell needs help in Mossy Hollow, just east of here.",
        ],
        exp: 30,
        gold: 10,
        potions: 1,
        items: &[],
        flag_on_accept: None,
        flag_on_complete: None,
        chain_to: None,
    },
    QuestDef {
        id: "mossling_trouble",
        name: "Mossling Trouble",
        giver: "ranger_pell",
        turn_in: "ranger_pell",
        requires: Some("signature_moves"),
        min_level: 0,
        objective: Objective::Kill(EnemyKind::Mossling, 8),
        summary: "Defeat 8 Mosslings in Mossy Hollow.",
        offer: &[
            "Oh good, Brask sent help. The Mosslings have overrun the Hollow - they're blocking the road to the Old Gate.",
            "Thin them out for me? Eight should do.",
        ],
        reminder: "The Hollow's just down the slope. Eight Mosslings!",
        complete: &["The Hollow's breathing again. Thank you!", "Here - my old tunic. Patched, but it's turned a few claws in its time."],
        exp: 60,
        gold: 20,
        potions: 1,
        items: &[("novice_tunic", 1)],
        flag_on_accept: None,
        flag_on_complete: None,
        chain_to: None,
    },
    QuestDef {
        id: "beak_hunt",
        name: "Beak Hunt",
        giver: "ranger_pell",
        turn_in: "ranger_pell",
        requires: Some("mossling_trouble"),
        min_level: 0,
        objective: Objective::Kill(EnemyKind::Burrowbeak, 3),
        summary: "Defeat 3 Burrowbeaks. They telegraph their peck - dodge it!",
        offer: &[
            "One more thing. Burrowbeaks nest in the Hollow too - they puff up right before they lunge.",
            "When you see that, dodge through and punish. Three of them, please.",
        ],
        reminder: "Watch for the puff-up, dodge, then strike. Three Burrowbeaks.",
        complete: &["You're ready. Warden Ilsa guards the Old Gate - she'll want to meet you."],
        exp: 70,
        gold: 20,
        potions: 1,
        items: &[],
        flag_on_accept: None,
        flag_on_complete: None,
        chain_to: None,
    },
    QuestDef {
        id: "the_bramblehorn",
        name: "The Bramblehorn",
        giver: "warden_ilsa",
        turn_in: "warden_ilsa",
        requires: Some("beak_hunt"),
        min_level: 0,
        objective: Objective::Kill(EnemyKind::Bramblehorn, 1),
        summary: "Defeat the Bramblehorn by the Old Gate.",
        offer: &[
            "So you're the one everyone's talking about. Good timing.",
            "A Bramblehorn has rooted itself before the Gate. It charges - jump or dodge over it, then hit it from behind.",
            "Bring it down and I'll open the Gate to Breezehollow.",
        ],
        reminder: "The Bramblehorn waits by the Gate. Jump or dodge its charge!",
        complete: &["Incredible! The Old Gate is open. Breezehollow lies beyond - Mayor Odessa will want to meet you.", "Keep this Revive Scroll. If you ever fall, read it and you'll be back on your feet without losing a thing."],
        exp: 150,
        gold: 50,
        potions: 2,
        items: &[("revive_scroll", 1)],
        flag_on_accept: Some("boss_awake"),
        flag_on_complete: Some("gate_open"),
        chain_to: Some("report_to_mayor"),
    },
    QuestDef {
        id: "report_to_mayor",
        name: "Welcome to Breezehollow",
        giver: "warden_ilsa",
        turn_in: "mayor_odessa",
        requires: Some("the_bramblehorn"),
        min_level: 0,
        objective: Objective::TalkTo("mayor_odessa"),
        summary: "Go through the Old Gate and speak with Mayor Odessa in Breezehollow.",
        offer: &[],
        reminder: "",
        complete: &[
            "Welcome, welcome! Ilsa's messenger-hawk told me all about the Bramblehorn.",
            "Breezehollow is a small town, but it's a safe one. Make yourself at home!",
        ],
        exp: 50,
        gold: 30,
        potions: 0,
        items: &[],
        flag_on_accept: None,
        flag_on_complete: Some("welcomed"),
        chain_to: Some("meet_the_town"),
    },
    QuestDef {
        id: "meet_the_town",
        name: "Meet the Townsfolk",
        giver: "mayor_odessa",
        turn_in: "mayor_odessa",
        requires: Some("report_to_mayor"),
        min_level: 0,
        objective: Objective::TalkMany(&["mira", "garrick", "mentor_aurel"]),
        summary: "Introduce yourself to Mira, Garrick and Aurel.",
        offer: &["Do say hello to Mira the brewer, Garrick at the forge, and Aurel - they'll look after you."],
        reminder: "Mira, Garrick and Aurel - have you met them all?",
        complete: &["Wonderful. You're one of us now. When you're ready, come see me - the farms to the east need help."],
        exp: 40,
        gold: 25,
        potions: 2,
        items: &[],
        flag_on_accept: None,
        flag_on_complete: None,
        chain_to: None,
    },
    // ------------------------------------------------------------ Whistling Fields
    QuestDef {
        id: "road_east",
        name: "The Road East",
        giver: "mayor_odessa",
        turn_in: "farmer_tilly",
        requires: Some("meet_the_town"),
        min_level: 0,
        objective: Objective::TalkTo("farmer_tilly"),
        summary: "Take the east road to the Whistling Fields and find Farmer Tilly.",
        offer: &[
            "Farmer Tilly sent word from the Whistling Fields. Her flock has gone... strange.",
            "The east road is open to you now. Go and lend her a hand?",
        ],
        reminder: "Tilly's farm is just past the east road, by the first windmill.",
        complete: &["You came! Oh thank goodness. My Puffwools have gone completely wild."],
        exp: 60,
        gold: 20,
        potions: 1,
        items: &[],
        flag_on_accept: Some("fields_open"),
        flag_on_complete: None,
        chain_to: None,
    },
    QuestDef {
        id: "woolly_menace",
        name: "Woolly Menace",
        giver: "farmer_tilly",
        turn_in: "farmer_tilly",
        requires: Some("road_east"),
        min_level: 0,
        objective: Objective::Kill(EnemyKind::Puffwool, 10),
        summary: "Calm the wild Puffwools in the fields (defeat 10).",
        offer: &[
            "Ever since the wind turned sour, the Puffwools charge anyone who comes near.",
            "Bonk ten of them for me - gently! They'll be back to grazing by morning.",
        ],
        reminder: "Ten Puffwools, please. They're all over the terraces.",
        complete: &["Look at them - fluffy and peaceful again. You're a natural!"],
        exp: 220,
        gold: 40,
        potions: 2,
        items: &[],
        flag_on_accept: None,
        flag_on_complete: None,
        chain_to: None,
    },
    QuestDef {
        id: "gust_busters",
        name: "Gust Busters",
        giver: "farmer_tilly",
        turn_in: "farmer_tilly",
        requires: Some("woolly_menace"),
        min_level: 0,
        objective: Objective::Kill(EnemyKind::Gustling, 6),
        summary: "Defeat 6 Gustlings. Dodge their wind orbs!",
        offer: &[
            "The Gustlings are what's souring the wind. Little troublemakers, spitting gusts at everyone.",
            "Disperse six of them. Dodge their orbs - they pack a punch.",
        ],
        reminder: "Six Gustlings. They float over the middle fields.",
        complete: &["The breeze smells sweet again! Windcaller Oren at the far end of the fields will want to hear about this."],
        exp: 260,
        gold: 45,
        potions: 1,
        items: &[],
        flag_on_accept: None,
        flag_on_complete: None,
        chain_to: Some("meet_oren"),
    },
    QuestDef {
        id: "meet_oren",
        name: "The Windcaller",
        giver: "farmer_tilly",
        turn_in: "windcaller_oren",
        requires: Some("gust_busters"),
        min_level: 0,
        objective: Objective::TalkTo("windcaller_oren"),
        summary: "Find Windcaller Oren at the eastern end of the Whistling Fields.",
        offer: &[],
        reminder: "",
        complete: &["So you're the one who calmed the Gustlings. The wind told me you'd come."],
        exp: 80,
        gold: 10,
        potions: 0,
        items: &[],
        flag_on_accept: None,
        flag_on_complete: None,
        chain_to: None,
    },
    QuestDef {
        id: "into_thornvale",
        name: "Into Thornvale",
        giver: "windcaller_oren",
        turn_in: "gravekeeper_morrow",
        requires: Some("meet_oren"),
        min_level: 0,
        objective: Objective::TalkTo("gravekeeper_morrow"),
        summary: "Enter Thornvale Woods and speak with Gravekeeper Morrow.",
        offer: &[
            "The sour wind came from Thornvale Woods, beyond this road. Something stirs there after dark.",
            "Old Morrow keeps the graves at the forest's edge. Find him - and keep your lantern close.",
        ],
        reminder: "The path to Thornvale is just behind me.",
        complete: &["A visitor? At this hour? ...Oren sent you. Good. I could use the help."],
        exp: 100,
        gold: 15,
        potions: 1,
        items: &[],
        flag_on_accept: Some("woods_open"),
        flag_on_complete: None,
        chain_to: None,
    },
    // ------------------------------------------------------------ Thornvale Woods
    QuestDef {
        id: "restless_caps",
        name: "Restless Caps",
        giver: "gravekeeper_morrow",
        turn_in: "gravekeeper_morrow",
        requires: Some("into_thornvale"),
        min_level: 0,
        objective: Objective::Kill(EnemyKind::Gravecap, 10),
        summary: "Defeat 10 Gravecaps sprouting among the graves.",
        offer: &[
            "Gravecaps. Mushrooms that grow where the old roots run deep, and they don't like company.",
            "Clear ten of them out of my graveyard before they spread.",
        ],
        reminder: "Ten Gravecaps. The hollow is crawling with them.",
        complete: &["Hah! You've got grit. The graves can rest a little easier."],
        exp: 420,
        gold: 60,
        potions: 2,
        items: &[],
        flag_on_accept: None,
        flag_on_complete: None,
        chain_to: None,
    },
    QuestDef {
        id: "lights_out",
        name: "Lights Out",
        giver: "gravekeeper_morrow",
        turn_in: "gravekeeper_morrow",
        requires: Some("restless_caps"),
        min_level: 0,
        objective: Objective::Kill(EnemyKind::Lanternwisp, 8),
        summary: "Snuff out 8 Lanternwisps. They hurl starfire from above.",
        offer: &[
            "See those lights drifting between the trees? Lanternwisps. Lost travellers follow them and never come back.",
            "Snuff out eight. They float out of reach - jump, double jump, and hit them hard.",
        ],
        reminder: "Eight Lanternwisps. Look up between the trees.",
        complete: &["Darker out there now - the good kind of dark. The Hermit Sable lives at the far end of the woods. She'll know what's behind all this."],
        exp: 480,
        gold: 70,
        potions: 1,
        items: &[],
        flag_on_accept: None,
        flag_on_complete: None,
        chain_to: Some("find_sable"),
    },
    QuestDef {
        id: "find_sable",
        name: "The Hermit",
        giver: "gravekeeper_morrow",
        turn_in: "hermit_sable",
        requires: Some("lights_out"),
        min_level: 0,
        objective: Objective::TalkTo("hermit_sable"),
        summary: "Find Hermit Sable at the eastern edge of Thornvale Woods.",
        offer: &[],
        reminder: "",
        complete: &["Morrow sent you through the woods alone, at night? He must trust you. Sit. Listen."],
        exp: 120,
        gold: 15,
        potions: 0,
        items: &[],
        flag_on_accept: None,
        flag_on_complete: None,
        chain_to: None,
    },
    QuestDef {
        id: "rotten_roots",
        name: "Rotten Roots",
        giver: "hermit_sable",
        turn_in: "hermit_sable",
        requires: Some("find_sable"),
        min_level: 0,
        objective: Objective::Kill(EnemyKind::Rotstump, 6),
        summary: "Defeat 6 Rotstumps on the high ridges.",
        offer: &[
            "The forest's roots are rotting. The Rotstumps carry the rot wherever they walk.",
            "Six of them prowl the ridges. They're slow, but if one rumbles - get out of the way.",
        ],
        reminder: "Six Rotstumps, up on the ridges.",
        complete: &["The rot is fading. But it began somewhere above us... on the cliffs, where the storms are born."],
        exp: 600,
        gold: 90,
        potions: 2,
        items: &[],
        flag_on_accept: None,
        flag_on_complete: None,
        chain_to: None,
    },
    QuestDef {
        id: "the_high_road",
        name: "The High Road",
        giver: "hermit_sable",
        turn_in: "ranger_sela",
        requires: Some("rotten_roots"),
        min_level: 0,
        objective: Objective::TalkTo("ranger_sela"),
        summary: "Take the hidden path to Windcrag Cliffs and meet Ranger Sela.",
        offer: &[
            "There's a hidden path behind my hut. It climbs to Windcrag Cliffs.",
            "Ranger Sela watches the cliffs. Tell her Sable sent you.",
        ],
        reminder: "The cliff path is just east of here.",
        complete: &["Sable's friend? Then you're mine too. Welcome to the cliffs - mind the edge."],
        exp: 150,
        gold: 20,
        potions: 1,
        items: &[],
        flag_on_accept: Some("cliffs_open"),
        flag_on_complete: None,
        chain_to: None,
    },
    // ------------------------------------------------------------ Windcrag Cliffs
    QuestDef {
        id: "crab_cracking",
        name: "Crab Cracking",
        giver: "ranger_sela",
        turn_in: "ranger_sela",
        requires: Some("the_high_road"),
        min_level: 0,
        objective: Objective::Kill(EnemyKind::CragCrab, 8),
        summary: "Defeat 8 Crag Crabs on the cliff ledges.",
        offer: &[
            "Crag Crabs have taken over the climb. Shells like granite, and they lunge sideways when you get close.",
            "Crack eight of them so the path is safe again.",
        ],
        reminder: "Eight Crag Crabs. Watch for the claw-raise before they lunge.",
        complete: &["Nice work. The ledges are passable again."],
        exp: 800,
        gold: 110,
        potions: 2,
        items: &[],
        flag_on_accept: None,
        flag_on_complete: None,
        chain_to: None,
    },
    QuestDef {
        id: "hawk_eyes",
        name: "Hawk Eyes",
        giver: "ranger_sela",
        turn_in: "ranger_sela",
        requires: Some("crab_cracking"),
        min_level: 0,
        objective: Objective::Kill(EnemyKind::Galehawk, 6),
        summary: "Defeat 6 Galehawks. They screech, then dive - dodge!",
        offer: &[
            "Galehawks hunt anything that climbs. They hang in the wind, screech, then dive like arrows.",
            "When you hear the screech, dodge. Then knock them out of the sky. Six should do.",
        ],
        reminder: "Six Galehawks. Dodge the dive, then punish.",
        complete: &["The sky's clearer. Stormwatcher Kael is waiting at the summit - he's been watching that storm for weeks."],
        exp: 900,
        gold: 120,
        potions: 3,
        items: &[],
        flag_on_accept: None,
        flag_on_complete: None,
        chain_to: Some("summit_call"),
    },
    QuestDef {
        id: "summit_call",
        name: "To the Summit",
        giver: "ranger_sela",
        turn_in: "stormwatcher_kael",
        requires: Some("hawk_eyes"),
        min_level: 0,
        objective: Objective::TalkTo("stormwatcher_kael"),
        summary: "Climb to the summit of Windcrag Cliffs and speak with Stormwatcher Kael.",
        offer: &[],
        reminder: "",
        complete: &["You made the climb. Good. Look there - the storm has a heart, and it has horns."],
        exp: 200,
        gold: 25,
        potions: 1,
        items: &[],
        flag_on_accept: None,
        flag_on_complete: None,
        chain_to: None,
    },
    QuestDef {
        id: "storm_summit",
        name: "Storm at the Summit",
        giver: "stormwatcher_kael",
        turn_in: "stormwatcher_kael",
        requires: Some("summit_call"),
        min_level: 0,
        objective: Objective::Kill(EnemyKind::Thunderhorn, 1),
        summary: "Defeat the Thunderhorn on the summit.",
        offer: &[
            "The Thunderhorn. It drinks the wind and spits lightning - that's what soured the breeze all the way to Breezehollow.",
            "It charges like its little cousin in the glade, but harder, and it calls bolts from the sky. Bring potions.",
            "Bring it down, and the winds will be free again.",
        ],
        reminder: "The Thunderhorn waits on the summit. Jump its charge, dodge the bolts.",
        complete: &[
            "The storm is breaking... you did it!",
            "Breezehollow will sing about this. And beyond the summit lies the Skyroad - but that's a story for another day.",
        ],
        exp: 2500,
        gold: 400,
        potions: 4,
        items: &[],
        flag_on_accept: None,
        flag_on_complete: Some("summit_cleared"),
        chain_to: None,
    },
    QuestDef {
        id: "choose_your_path",
        name: "Choose Your Path",
        giver: "mentor_aurel",
        turn_in: "mentor_aurel",
        requires: Some("meet_the_town"),
        min_level: CLASS_CHANGE_LEVEL,
        objective: Objective::ChangeClass,
        summary: "Take the North Road (east Breezehollow) to Wayfarer's Crossroads, pick a class town and change class with its instructor.",
        offer: &[
            "Level ten! You're no Novice anymore - well, you are, but not for long.",
            "Take the North Road, just behind me, to Wayfarer's Crossroads. Five roads lead from there, one to each class town.",
            "Ironhold trains Vanguards, Shadowmere trains Duskblades, Tempest Monastery trains Stormfists, Leafwhisper trains Windrunners and Starfall Spire trains Aethermancers.",
            "Talk to a town's instructor to take up its class. Choose well - the path you pick is yours for good. Then come back and tell me!",
        ],
        reminder: "Take the North Road behind me to Wayfarer's Crossroads and pick a class town. The instructor there will change your class.",
        complete: &["Look at you! I can see it in how you stand. Your new abilities are in your Abilities window ({Abilities}) - drag them to your hotbar."],
        exp: 150,
        gold: 120,
        potions: 3,
        items: &[],
        flag_on_accept: None,
        flag_on_complete: None,
        chain_to: None,
    },
];

pub fn quest_def(id: &str) -> Option<&'static QuestDef> {
    QUESTS.iter().find(|q| q.id == id)
}

/// Events the simulation reports so quests can track progress.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum QuestEvent<'a> {
    Reached(&'a str),
    Dodged,
    DummyHit { heavy: bool },
    Killed(EnemyKind),
    ChangedClass,
}

/// Apply an event; returns names of quests that just became ready to turn in.
pub fn on_event(save: &mut CharacterSave, ev: QuestEvent) -> Vec<&'static str> {
    let mut ready = Vec::new();
    for q in save.quests.iter_mut() {
        if q.state != QuestState::Active {
            continue;
        }
        let Some(def) = quest_def(&q.id) else { continue };
        let hit = match (def.objective, ev) {
            (Objective::Reach(r), QuestEvent::Reached(e)) => r == e,
            (Objective::Dodge(_), QuestEvent::Dodged) => true,
            (Objective::DummyLight(_), QuestEvent::DummyHit { heavy: false }) => true,
            (Objective::DummyHeavy(_), QuestEvent::DummyHit { heavy: true }) => true,
            (Objective::Kill(k, _), QuestEvent::Killed(e)) => k == e,
            (Objective::ChangeClass, QuestEvent::ChangedClass) => true,
            _ => false,
        };
        if hit {
            q.progress = (q.progress + 1).min(def.objective.target());
            if q.progress >= def.objective.target() {
                q.state = QuestState::ReadyToTurnIn;
                ready.push(def.name);
            }
        }
    }
    ready
}

#[derive(Clone, Debug, PartialEq)]
pub enum DialogAction {
    None,
    OfferQuest(&'static str),
    CompleteQuest(&'static str),
    /// Open a shop window (shop id from items::SHOPS).
    Shop(&'static str),
    /// Offer to change class (Novice at level 10+).
    ClassChange(ClassId),
    /// Open the waystone fast-travel menu.
    Waystone,
    Heal,
}

#[derive(Clone, Debug)]
pub struct Dialog {
    pub npc_id: &'static str,
    pub speaker: &'static str,
    pub lines: Vec<String>,
    pub action: DialogAction,
}

fn idle_lines(npc: &str, save: &CharacterSave) -> Vec<String> {
    let l: &[&str] = match npc {
        "elder_wynn" => &["The wind always knows where it's going. Follow it east, young one."],
        "scout_tamsin" => &["Keep moving and nothing can touch you. That's the scout's way."],
        "sergeant_brask" => &["The dummies are always here if you want to practise. Try air attacks!"],
        "quartermaster_hesper" => {
            if save.trainee_buys < crate::items::TRAINEE_DISCOUNT_BUYS {
                &[
                    "Supplies for the road! Trainee potions are 40% off for your first ten - my treat for new recruits.",
                    "They're bound to you, mind: no selling, trading or dropping them. I'll buy any monster loot you've picked up, though.",
                ]
            } else {
                &["Back again? I've got potions, sturdy Lv 8 gear, and I'll buy whatever the Hollow coughed up."]
            }
        }
        "ranger_pell" => &["The Hollow's quieter thanks to you. Mostly."],
        "warden_ilsa" => {
            if save.has_flag("gate_open") {
                &["The Gate stands open. Breezehollow is just beyond."]
            } else {
                &["Few pass this Gate. Prove yourself to Ranger Pell first."]
            }
        }
        "mayor_odessa" => &["Breezehollow welcomes all travellers. Mostly the polite ones."],
        "mira" => &["Potions! Fresh potions! I'll patch you up for free, too - just ask."],
        "garrick" => &["Need something sturdier than a stick? Take a look at what's on the rack."],
        "mentor_aurel" => {
            if save.class == ClassId::Novice {
                &[
                    "Everyone starts as a Novice. At level 10 you can take up a class in one of the five class towns.",
                    "Until then: spend your ability points ({Character}) and master your directional moves.",
                ]
            } else {
                &["The path suits you. Keep training - new abilities come at levels 12, 15 and 20."]
            }
        }
        "wayfarer_jory" => &[
            "Five roads, five towns, five ways to fight. Ironhold and Shadowmere lie west, the Monastery and Leafwhisper north, Starfall Spire at the far end.",
            "Every town has a general store, a weaponsmith and an armourer. Just walk in through the door.",
        ],
        "ironhold_general" | "shadowmere_general" | "tempest_general" | "leafwhisper_general" | "starfall_general" => &["Potions, ethers, tonics. Everything an adventurer needs to stay upright."],
        "ironhold_weapons" | "shadowmere_weapons" | "tempest_weapons" | "leafwhisper_weapons" | "starfall_weapons" => &["Finest weapons in the land. Class weapons need the right class to wield, mind."],
        "ironhold_armor" | "shadowmere_armor" | "tempest_armor" | "leafwhisper_armor" | "starfall_armor" => &["Armour, boots and trinkets. Good gear is cheaper than a funeral."],
        "inn_ironhold" | "inn_shadowmere" | "inn_tempest" | "inn_leafwhisper" | "inn_starfall" => &["Rest a while, traveller. You look like you've had a rough road."],
        "kid_bo" => &["Did you REALLY beat the Bramblehorn?! Can you teach me the double jump?"],
        "farmer_tilly" => &["The Puffwools say thank you. Well, they say 'baa', but I know what they mean."],
        "windcaller_oren" => &["Listen... the wind always tells you where trouble is. Right now it's whispering 'east'."],
        "gravekeeper_morrow" => &["Quiet night. Just how I like 'em."],
        "hermit_sable" => &["The forest remembers kindness. And grudges. Mostly grudges."],
        "ranger_sela" => &["Keep your footing. The wind up here has a mean streak."],
        "stormwatcher_kael" => {
            if save.has_flag("summit_cleared") {
                &["Clear skies for the first time in months. Thank you, friend."]
            } else {
                &["The storm is watching us. Are you ready?"]
            }
        }
        _ => &["..."],
    };
    l.iter().map(|s| s.to_string()).collect()
}

fn talk_many_line(npc: &str) -> &'static str {
    match npc {
        "mira" => "A new face! I'm Mira. Need potions, I've got you - and I'll heal you up whenever you visit.",
        "garrick" => "Garrick. I make swords, shields, bows... if it's sharp, I made it. Come back when you've got ore.",
        "mentor_aurel" => "I'm Aurel. I train every class here. Come see me when you hit level 10 for your advancement.",
        _ => "Nice to meet you!",
    }
}

/// Figure out what an NPC says right now, and what happens when the dialog ends.
/// May update TalkTo/TalkMany progress immediately.
pub fn talk(npc_id: &'static str, speaker: &'static str, save: &mut CharacterSave) -> Dialog {
    // 1) Quests ready to hand in here.
    for q in &save.quests {
        if q.state == QuestState::ReadyToTurnIn {
            if let Some(def) = quest_def(&q.id) {
                if def.turn_in == npc_id {
                    return Dialog {
                        npc_id,
                        speaker,
                        lines: def.complete.iter().map(|s| s.to_string()).collect(),
                        action: DialogAction::CompleteQuest(def.id),
                    };
                }
            }
        }
    }
    // 2) Talk objectives.
    let mut talk_many: Option<(&'static str, String)> = None;
    for q in save.quests.iter_mut() {
        if q.state != QuestState::Active {
            continue;
        }
        let Some(def) = quest_def(&q.id) else { continue };
        match def.objective {
            Objective::TalkTo(t) if t == npc_id => {
                q.progress = 1;
                q.state = QuestState::ReadyToTurnIn;
                return Dialog {
                    npc_id,
                    speaker,
                    lines: def.complete.iter().map(|s| s.to_string()).collect(),
                    action: DialogAction::CompleteQuest(def.id),
                };
            }
            Objective::TalkMany(list) if list.contains(&npc_id) => {
                let flag = format!("met_{}_{}", def.id, npc_id);
                talk_many = Some((def.id, flag));
            }
            _ => {}
        }
    }
    if let Some((qid, flag)) = talk_many {
        if !save.has_flag(&flag) {
            save.set_flag(&flag);
            let def = quest_def(qid).unwrap();
            let q = save.quest_mut(qid).unwrap();
            q.progress = (q.progress + 1).min(def.objective.target());
            if q.progress >= def.objective.target() {
                q.state = QuestState::ReadyToTurnIn;
            }
            let action = if npc_id == "mira" { DialogAction::Heal } else { DialogAction::None };
            return Dialog { npc_id, speaker, lines: vec![talk_many_line(npc_id).to_string()], action };
        }
    }
    // 3) New quests offered here.
    for def in QUESTS {
        if def.giver != npc_id || def.offer.is_empty() || save.quest(def.id).is_some() {
            continue;
        }
        let prereq_ok = save.level >= def.min_level
            && def
                .requires
                .map(|r| save.quest(r).map(|q| q.state == QuestState::Done).unwrap_or(false))
                .unwrap_or(true);
        if prereq_ok {
            return Dialog {
                npc_id,
                speaker,
                lines: def.offer.iter().map(|s| s.to_string()).collect(),
                action: DialogAction::OfferQuest(def.id),
            };
        }
    }
    // 4) Reminders for active quests from this NPC.
    for q in &save.quests {
        if q.state == QuestState::Active {
            if let Some(def) = quest_def(&q.id) {
                if (def.giver == npc_id || def.turn_in == npc_id) && !def.reminder.is_empty() {
                    return Dialog { npc_id, speaker, lines: vec![def.reminder.to_string()], action: DialogAction::None };
                }
            }
        }
    }
    // 5) Waystones.
    if npc_id.starts_with("waystone_") {
        return Dialog {
            npc_id,
            speaker: "Waystone",
            lines: vec!["The waystone hums. Touch it to travel to any waystone you've visited.".into()],
            action: DialogAction::Waystone,
        };
    }
    // 6) Class instructors.
    if let Some(class) = instructor_class(npc_id) {
        return instructor_dialog(npc_id, speaker, class, save);
    }
    let action = if let Some(shop) = npc_shop(npc_id) {
        DialogAction::Shop(shop)
    } else if npc_id.starts_with("inn_") {
        DialogAction::Heal
    } else {
        DialogAction::None
    };
    Dialog { npc_id, speaker, lines: idle_lines(npc_id, save), action }
}

pub fn accept(save: &mut CharacterSave, id: &str) {
    if save.quest(id).is_some() {
        return;
    }
    if let Some(def) = quest_def(id) {
        save.quests.push(QuestEntry { id: def.id.to_string(), state: QuestState::Active, progress: 0 });
        if let Some(f) = def.flag_on_accept {
            save.set_flag(f);
        }
    }
}

pub struct Reward {
    pub exp: u32,
    pub gold: u32,
    pub potions: u32,
    /// Item rewards (id, count); anything that didn't fit in the bag is in `overflow`.
    pub items: &'static [(&'static str, u32)],
    pub overflow: Vec<(&'static str, u32)>,
    pub quest_name: &'static str,
    pub chained: Option<&'static str>,
}

/// Everything a quest gives, for showing before it's accepted:
/// (item id or "" for plain text, label, count).
pub fn reward_list(def: &QuestDef) -> Vec<(&'static str, String)> {
    let mut v: Vec<(&'static str, String)> = Vec::new();
    if def.exp > 0 {
        v.push(("", format!("{} EXP", def.exp)));
    }
    if def.gold > 0 {
        v.push(("", format!("{} gold", def.gold)));
    }
    if def.potions > 0 {
        v.push(("hp_potion", format!("{}x Red Potion", def.potions)));
    }
    for &(id, n) in def.items {
        let name = crate::items::item(id).map(|d| d.name).unwrap_or(id);
        v.push((id, if n > 1 { format!("{}x {}", n, name) } else { name.to_string() }));
    }
    v
}

/// Mark a quest done, set flags, auto-accept any chained quest. EXP is
/// returned for the caller to apply (it may cause level ups).
pub fn complete(save: &mut CharacterSave, id: &str) -> Option<Reward> {
    let def = quest_def(id)?;
    let q = save.quest_mut(id)?;
    if q.state != QuestState::ReadyToTurnIn {
        return None;
    }
    q.state = QuestState::Done;
    save.gold += def.gold;
    let mut overflow = Vec::new();
    let left = save.inventory.add("hp_potion", def.potions);
    if left > 0 {
        overflow.push(("hp_potion", left));
    }
    for &(item, n) in def.items {
        let left = save.inventory.add(item, n);
        if left > 0 {
            overflow.push((item, left));
        }
    }
    if let Some(f) = def.flag_on_complete {
        save.set_flag(f);
    }
    let mut chained = None;
    if let Some(next) = def.chain_to {
        accept(save, next);
        chained = quest_def(next).map(|d| d.name);
    }
    Some(Reward { exp: def.exp, gold: def.gold, potions: def.potions, items: def.items, overflow, quest_name: def.name, chained })
}

/// Quest that the tracker HUD should show, with progress text.
pub fn tracker_lines(save: &CharacterSave) -> Vec<(String, String, bool)> {
    let mut v = Vec::new();
    for q in &save.quests {
        if q.state == QuestState::Done {
            continue;
        }
        if let Some(def) = quest_def(&q.id) {
            let ready = q.state == QuestState::ReadyToTurnIn;
            let prog = if ready {
                format!("Return to {}", npc_display_name(def.turn_in))
            } else {
                match def.objective {
                    Objective::Reach(_) | Objective::TalkTo(_) => def.summary.to_string(),
                    _ => format!("{} ({}/{})", def.summary, q.progress, def.objective.target()),
                }
            };
            v.push((def.name.to_string(), prog, ready));
        }
    }
    v
}

pub fn npc_display_name(id: &str) -> &'static str {
    match id {
        "elder_wynn" => "Elder Wynn",
        "scout_tamsin" => "Tamsin",
        "sergeant_brask" => "Sergeant Brask",
        "quartermaster_hesper" => "Hesper",
        "ranger_pell" => "Pell",
        "warden_ilsa" => "Warden Ilsa",
        "mayor_odessa" => "Mayor Odessa",
        "mira" => "Mira",
        "garrick" => "Garrick",
        "mentor_aurel" => "Aurel",
        "farmer_tilly" => "Tilly",
        "windcaller_oren" => "Oren",
        "gravekeeper_morrow" => "Morrow",
        "hermit_sable" => "Sable",
        "ranger_sela" => "Sela",
        "stormwatcher_kael" => "Kael",
        "wayfarer_jory" => "Jory",
        other => TOWN_NPCS.iter().find(|(id, _)| *id == other).map(|(_, n)| *n).unwrap_or("someone"),
    }
}

/// Display names for the class-town NPCs.
pub const TOWN_NPCS: &[(&str, &str)] = &[
    ("instructor_ironhold", "Commander Hale"),
    ("instructor_shadowmere", "Mistress Vey"),
    ("instructor_tempest", "Master Kaito"),
    ("instructor_leafwhisper", "Warden Faelin"),
    ("instructor_starfall", "Archmage Solenne"),
    ("inn_ironhold", "Innkeeper Bess"),
    ("inn_shadowmere", "Madame Cindra"),
    ("inn_tempest", "Abbot Ren"),
    ("inn_leafwhisper", "Healer Ama"),
    ("inn_starfall", "Sister Celes"),
    ("ironhold_general", "Greta"),
    ("ironhold_weapons", "Torvald"),
    ("ironhold_armor", "Hilde"),
    ("shadowmere_general", "Old Nim"),
    ("shadowmere_weapons", "Rook"),
    ("shadowmere_armor", "Lisette"),
    ("tempest_general", "Brother Pan"),
    ("tempest_weapons", "Sister Rei"),
    ("tempest_armor", "Brother Oto"),
    ("leafwhisper_general", "Moss"),
    ("leafwhisper_weapons", "Fletcher Wren"),
    ("leafwhisper_armor", "Bramble"),
    ("starfall_general", "Alchemist Iri"),
    ("starfall_weapons", "Curator Vale"),
    ("starfall_armor", "Weaver Lune"),
];

/// The class an instructor NPC teaches.
pub fn instructor_class(npc_id: &str) -> Option<ClassId> {
    Some(match npc_id {
        "instructor_ironhold" => ClassId::Vanguard,
        "instructor_shadowmere" => ClassId::Duskblade,
        "instructor_tempest" => ClassId::Stormfist,
        "instructor_leafwhisper" => ClassId::Windrunner,
        "instructor_starfall" => ClassId::Aethermancer,
        _ => return None,
    })
}

/// The shop an NPC runs, if any.
pub fn npc_shop(npc_id: &str) -> Option<&'static str> {
    match npc_id {
        "mira" => Some("breezehollow_general"),
        "quartermaster_hesper" => Some(crate::items::GLADE_SHOP),
        "garrick" => Some("breezehollow_smith"),
        other => crate::items::shop(other).map(|s| s.id),
    }
}

fn instructor_dialog(npc_id: &'static str, speaker: &'static str, class: ClassId, save: &CharacterSave) -> Dialog {
    let info = class.info();
    let pitch: &str = match class {
        ClassId::Vanguard => "Vanguards stand in front so others don't have to. Sword, shield and a spine of iron.",
        ClassId::Duskblade => "Duskblades strike from the shadows and are gone before the body hits the floor.",
        ClassId::Stormfist => "A Stormfist needs no blade. Discipline, speed and fists that crackle like thunder.",
        ClassId::Windrunner => "Windrunners read the wind and loose arrows that never miss. Stay light, stay far.",
        _ => "Aethermancers bend the raw aether of the sky. Fragile in body, terrifying in power.",
    };
    let (lines, action) = if save.class == class {
        (vec![format!("Train hard, {}. The {} path rewards those who never stop.", save.name, info.name)], DialogAction::None)
    } else if save.class != ClassId::Novice {
        (vec![format!("You already walk the {} path. Respect to you - but I have nothing to teach you.", save.class.info().name)], DialogAction::None)
    } else if save.level < CLASS_CHANGE_LEVEL {
        (
            vec![
                pitch.to_string(),
                format!("Come back when you reach level {} and I'll make a {} of you.", CLASS_CHANGE_LEVEL, info.name),
            ],
            DialogAction::None,
        )
    } else {
        (
            vec![
                pitch.to_string(),
                format!(
                    "Main stats: {} first, then {}. Your ability points will be refunded so you can rebuild around them.",
                    info.primary.long_name(),
                    info.secondary.long_name()
                ),
                format!("Will you become a {}? This choice is permanent.", info.name),
            ],
            DialogAction::ClassChange(class),
        )
    };
    Dialog { npc_id, speaker, lines, action }
}

/// Which NPCs should show a marker: '!' new quest, '?' turn-in.
pub fn npc_marker(npc_id: &str, save: &CharacterSave) -> Option<char> {
    if instructor_class(npc_id).is_some() && save.class == ClassId::Novice && save.level >= CLASS_CHANGE_LEVEL {
        return Some('!');
    }
    for q in &save.quests {
        if let Some(def) = quest_def(&q.id) {
            if q.state == QuestState::ReadyToTurnIn && def.turn_in == npc_id {
                return Some('?');
            }
            if q.state == QuestState::Active {
                if let Objective::TalkTo(t) = def.objective {
                    if t == npc_id {
                        return Some('?');
                    }
                }
                if let Objective::TalkMany(l) = def.objective {
                    if l.contains(&npc_id) && !save.has_flag(&format!("met_{}_{}", def.id, npc_id)) {
                        return Some('?');
                    }
                }
            }
        }
    }
    for def in QUESTS {
        if def.giver == npc_id && !def.offer.is_empty() && save.quest(def.id).is_none() {
            let ok = save.level >= def.min_level
                && def
                    .requires
                    .map(|r| save.quest(r).map(|q| q.state == QuestState::Done).unwrap_or(false))
                    .unwrap_or(true);
            if ok {
                return Some('!');
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::appearance::Appearance;

    #[test]
    fn chain_integrity() {
        for q in QUESTS {
            if let Some(r) = q.requires {
                assert!(quest_def(r).is_some(), "{} requires missing {}", q.id, r);
            }
            if let Some(c) = q.chain_to {
                assert!(quest_def(c).is_some());
            }
            let chained = QUESTS.iter().any(|o| o.chain_to == Some(q.id));
            assert!(chained || !q.offer.is_empty(), "{}: quest can never be obtained", q.id);
        }
    }

    #[test]
    fn full_dialog_flow() {
        let mut s = CharacterSave::new("T", Appearance::default());
        assert_eq!(npc_marker("elder_wynn", &s), Some('!'));
        let d = talk("elder_wynn", "Elder Wynn", &mut s);
        assert_eq!(d.action, DialogAction::OfferQuest("first_breeze"));
        accept(&mut s, "first_breeze");
        assert_eq!(on_event(&mut s, QuestEvent::Reached("lookout")), vec!["First Breeze"]);
        let d = talk("scout_tamsin", "Tamsin", &mut s);
        assert_eq!(d.action, DialogAction::CompleteQuest("first_breeze"));
        let r = complete(&mut s, "first_breeze").unwrap();
        assert!(r.overflow.is_empty());
        assert_eq!(s.inventory.count("novice_charm"), 1, "item reward handed over");
        let d = talk("scout_tamsin", "Tamsin", &mut s);
        assert_eq!(d.action, DialogAction::OfferQuest("gust_dodger"));
    }

    #[test]
    fn tutorial_npcs_each_hand_out_novice_gear() {
        let mut givers = vec![];
        for q in QUESTS {
            for (id, _) in q.items {
                let d = crate::items::item(id).unwrap_or_else(|| panic!("{} rewards unknown item {}", q.id, id));
                if d.slot().is_some() {
                    assert_eq!(d.level, 1, "{}: novice gear is level 1", id);
                    givers.push(q.turn_in);
                }
            }
        }
        for npc in ["scout_tamsin", "sergeant_brask", "ranger_pell"] {
            assert!(givers.contains(&npc), "{} gives no novice gear", npc);
        }
        // Rewards are listed before accepting.
        let r = reward_list(quest_def("sparring_basics").unwrap());
        assert!(r.iter().any(|(id, _)| *id == "novice_sword"));
        assert!(r.iter().any(|(_, t)| t.contains("EXP")));
    }
}
