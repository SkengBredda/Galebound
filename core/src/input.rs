//! Engine-agnostic input: actions, control layouts and per-tick input state.
//! The game crate translates key tokens (e.g. "A", "Left", "LeftAlt") to real keys.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    Left,
    Right,
    Up,
    Down,
    Jump,
    Light,
    Heavy,
    Dodge,
    Interact,
    Potion,
    QuestLog,
    Menu,
    Inventory,
    Character,
    Abilities,
    WorldMap,
    /// Pick up loot and gold from the ground.
    Pickup,
    /// Cycle the minimap size (small / medium / large / hidden).
    Minimap,
    /// Open the Pets window.
    Pets,
    /// Type a chat message (online).
    Chat,
    /// Friends, party and block list.
    Social,
    Guild,
    /// Achievements, cosmetics and daily attendance.
    Achievements,
    Hotbar1,
    Hotbar2,
    Hotbar3,
    Hotbar4,
    Hotbar5,
    Hotbar6,
    Hotbar7,
    Hotbar8,
}

pub const HOTBAR_ACTIONS: [Action; 8] = [
    Action::Hotbar1,
    Action::Hotbar2,
    Action::Hotbar3,
    Action::Hotbar4,
    Action::Hotbar5,
    Action::Hotbar6,
    Action::Hotbar7,
    Action::Hotbar8,
];

pub const ALL_ACTIONS: &[Action] = &[
    Action::Left,
    Action::Right,
    Action::Up,
    Action::Down,
    Action::Jump,
    Action::Light,
    Action::Heavy,
    Action::Dodge,
    Action::Interact,
    Action::Potion,
    Action::QuestLog,
    Action::Menu,
    Action::Inventory,
    Action::Character,
    Action::Abilities,
    Action::WorldMap,
    Action::Pickup,
    Action::Minimap,
    Action::Pets,
    Action::Chat,
    Action::Social,
    Action::Guild,
    Action::Achievements,
    Action::Hotbar1,
    Action::Hotbar2,
    Action::Hotbar3,
    Action::Hotbar4,
    Action::Hotbar5,
    Action::Hotbar6,
    Action::Hotbar7,
    Action::Hotbar8,
];

impl Action {
    pub fn label(self) -> &'static str {
        match self {
            Action::Left => "Move Left",
            Action::Right => "Move Right",
            Action::Up => "Up / Portal / Climb",
            Action::Down => "Down / Drop / Climb",
            Action::Jump => "Jump",
            Action::Light => "Light Attack",
            Action::Heavy => "Heavy Attack",
            Action::Dodge => "Dodge",
            Action::Interact => "Talk / Interact",
            Action::Potion => "Drink Potion",
            Action::QuestLog => "Quest Log",
            Action::Menu => "Menu",
            Action::Inventory => "Inventory",
            Action::Character => "Character",
            Action::Abilities => "Abilities",
            Action::WorldMap => "World Map",
            Action::Pickup => "Pick Up Loot",
            Action::Minimap => "Minimap Size",
            Action::Pets => "Pets",
            Action::Chat => "Chat",
            Action::Social => "Friends & Party",
            Action::Guild => "Guild",
            Action::Achievements => "Achievements",
            Action::Hotbar1 => "Hotbar 1",
            Action::Hotbar2 => "Hotbar 2",
            Action::Hotbar3 => "Hotbar 3",
            Action::Hotbar4 => "Hotbar 4",
            Action::Hotbar5 => "Hotbar 5",
            Action::Hotbar6 => "Hotbar 6",
            Action::Hotbar7 => "Hotbar 7",
            Action::Hotbar8 => "Hotbar 8",
        }
    }
    fn bit(self) -> u32 {
        1 << (self as u32)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    /// WASD to move, Space jump, J/K/L attack-heavy-dodge.
    Wasd,
    /// Arrow keys to move, Space jump, X/C/Z light-heavy-dodge.
    Arrows,
    /// Old-school side-scroller MMO: arrows, Alt jump, Ctrl attack, Shift skill.
    ClassicMmo,
}

pub const ALL_LAYOUTS: &[Layout] = &[Layout::Wasd, Layout::Arrows, Layout::ClassicMmo];

impl Layout {
    pub fn key(self) -> &'static str {
        match self {
            Layout::Wasd => "wasd",
            Layout::Arrows => "arrows",
            Layout::ClassicMmo => "classic",
        }
    }
    pub fn from_key(s: &str) -> Option<Layout> {
        ALL_LAYOUTS.iter().copied().find(|l| l.key() == s)
    }
    pub fn name(self) -> &'static str {
        match self {
            Layout::Wasd => "WASD",
            Layout::Arrows => "Arrow Keys",
            Layout::ClassicMmo => "Classic MMO",
        }
    }
    /// Key tokens bound to each action (first one is shown in the UI).
    pub fn keys(self, a: Action) -> &'static [&'static str] {
        use Action::*;
        // Shared by every layout.
        match a {
            Inventory => return &["I"],
            Character => return &["P"],
            Abilities => return &["B"],
            WorldMap => return &["M"],
            Pickup => return &["F"],
            Minimap => return &["N"],
            Pets => return &["O"],
            Chat => return &["Enter"],
            Social => return &["U"],
            Guild => return &["G"],
            Achievements => return &["H"],
            Hotbar1 => return &["Key1"],
            Hotbar2 => return &["Key2"],
            Hotbar3 => return &["Key3"],
            Hotbar4 => return &["Key4"],
            Hotbar5 => return &["Key5"],
            Hotbar6 => return &["Key6"],
            Hotbar7 => return &["Key7"],
            Hotbar8 => return &["Key8"],
            _ => {}
        }
        match self {
            Layout::Wasd => match a {
                Left => &["A"],
                Right => &["D"],
                Up => &["W"],
                Down => &["S"],
                Jump => &["Space"],
                Light => &["J"],
                Heavy => &["K"],
                Dodge => &["L", "LeftShift"],
                Interact => &["E"],
                Potion => &["Q"],
                QuestLog => &["Tab"],
                Menu => &["Escape"],
                _ => &[],
            },
            Layout::Arrows => match a {
                Left => &["Left"],
                Right => &["Right"],
                Up => &["Up"],
                Down => &["Down"],
                Jump => &["Space"],
                Light => &["X"],
                Heavy => &["C"],
                Dodge => &["Z"],
                Interact => &["V"],
                Potion => &["A"],
                QuestLog => &["Tab"],
                Menu => &["Escape"],
                _ => &[],
            },
            Layout::ClassicMmo => match a {
                Left => &["Left"],
                Right => &["Right"],
                Up => &["Up"],
                Down => &["Down"],
                Jump => &["LeftAlt", "RightAlt"],
                Light => &["LeftControl", "RightControl"],
                Heavy => &["LeftShift", "RightShift"],
                Dodge => &["Z"],
                Interact => &["Space"],
                Potion => &["Delete", "End"],
                QuestLog => &["Tab"],
                Menu => &["Escape"],
                _ => &[],
            },
        }
    }
    pub fn key_label(self, a: Action) -> String {
        let k = self.keys(a)[0];
        match k {
            "LeftShift" | "RightShift" => "Shift".into(),
            "LeftAlt" | "RightAlt" => "Alt".into(),
            "LeftControl" | "RightControl" => "Ctrl".into(),
            "Escape" => "Esc".into(),
            "Left" => "Left".into(),
            "Right" => "Right".into(),
            "Up" => "Up".into(),
            "Down" => "Down".into(),
            other => other.strip_prefix("Key").unwrap_or(other).into(),
        }
    }
}

/// Snapshot of actions for one simulation tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InputState {
    held: u32,
    pressed: u32,
}

impl InputState {
    pub fn set(&mut self, a: Action, held: bool, pressed: bool) {
        if held {
            self.held |= a.bit();
        } else {
            self.held &= !a.bit();
        }
        if pressed {
            self.pressed |= a.bit();
        }
    }
    pub fn held(&self, a: Action) -> bool {
        self.held & a.bit() != 0
    }
    pub fn pressed(&self, a: Action) -> bool {
        self.pressed & a.bit() != 0
    }
    /// Call after the first fixed tick of a frame so presses aren't repeated.
    pub fn clear_pressed(&mut self) {
        self.pressed = 0;
    }
    pub fn axis_x(&self) -> f32 {
        (self.held(Action::Right) as i32 - self.held(Action::Left) as i32) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn layouts_bind_everything() {
        for &l in ALL_LAYOUTS {
            for &a in ALL_ACTIONS {
                assert!(!l.keys(a).is_empty());
            }
            // No key bound to two different actions within a layout.
            let mut seen = std::collections::HashMap::new();
            for &a in ALL_ACTIONS {
                for k in l.keys(a) {
                    if let Some(prev) = seen.insert(*k, a) {
                        panic!("{:?}: key {} bound to {:?} and {:?}", l, k, prev, a);
                    }
                }
            }
        }
    }
}
