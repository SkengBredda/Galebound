//! Character appearance: indices into option lists. The renderer turns these
//! into shapes; this module only holds the data and the option names.

pub const HAIR_STYLES: &[&str] = &[
    "Windswept Spikes",
    "Short Bob",
    "High Ponytail",
    "Long & Flowing",
    "Side Swoop",
    "Twin Buns",
    "Shaggy Layers",
    "Braided Crown",
    "Long Locs",
    "Short Curls",
];

/// Body types (affect shoulders, hips and the underwear base layer).
pub const BODIES: &[&str] = &["Masculine", "Feminine"];

pub const HAIR_COLORS: &[(&str, [u8; 3])] = &[
    ("Coal", [38, 34, 40]),
    ("Chestnut", [110, 62, 34]),
    ("Honey", [214, 160, 72]),
    ("Ember", [196, 70, 44]),
    ("Frost", [222, 230, 240]),
    ("Sky", [92, 160, 230]),
    ("Moss", [86, 150, 84]),
    ("Plum", [140, 78, 168]),
    ("Ash Pink", [222, 178, 190]),
    ("Espresso", [64, 42, 34]),
];

pub const FACES: &[&str] = &["Calm", "Determined", "Cheerful", "Sleepy", "Fierce", "Sly Wink"];

/// Index 0 = none (the character's underwear shows).
pub const TOPS: &[&str] = &[
    "None",
    "Traveller's Tunic",
    "Leather Vest",
    "Cosy Hoodie",
    "Apprentice Robe",
    "Iron Breastplate",
    "Scarf Jacket",
    "Sport Crop Top",
    "Adventurer's Garb",
];

/// Index 0 = none (underwear).
pub const BOTTOMS: &[&str] = &[
    "None",
    "Trail Trousers",
    "Ranger Shorts",
    "Pleated Skirt",
    "Cargo Pants",
    "Wide Hakama",
    "Fitted Leggings",
    "Baggy Shorts",
];

pub const SHOES: &[&str] = &[
    "Leather Boots",
    "Sneakers",
    "Sandals",
    "Steel Greaves",
    "Soft Slippers",
    "Tall Riding Boots",
    "High-top Sneakers",
];

pub const SKIN_TONES: &[(&str, [u8; 3])] = &[
    ("Porcelain", [255, 228, 212]),
    ("Peach", [250, 210, 180]),
    ("Sand", [234, 192, 150]),
    ("Honey", [214, 162, 116]),
    ("Caramel", [180, 124, 84]),
    ("Umber", [140, 92, 62]),
    ("Cocoa", [106, 68, 46]),
    ("Ebony", [72, 46, 34]),
];

/// Dye colours available for the top and bottom.
pub const DYES: &[(&str, [u8; 3])] = &[
    ("Forest", [70, 128, 86]),
    ("Navy", [56, 78, 140]),
    ("Crimson", [168, 56, 64]),
    ("Saffron", [222, 168, 64]),
    ("Slate", [96, 104, 118]),
    ("Ivory", [232, 226, 210]),
    ("Violet", [118, 82, 160]),
    ("Teal", [52, 146, 150]),
    ("Charcoal", [40, 38, 44]),
    ("Ash Grey", [140, 140, 146]),
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Appearance {
    /// 0 = masculine (boxers), 1 = feminine (underwear + bra).
    pub body: u8,
    pub hair: u8,
    pub hair_color: u8,
    pub face: u8,
    pub top: u8,
    pub top_dye: u8,
    pub bottom: u8,
    pub bottom_dye: u8,
    pub shoes: u8,
    pub skin: u8,
}

/// One editable slot in the character creator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    Body,
    Hair,
    HairColor,
    Face,
    Top,
    TopDye,
    Bottom,
    BottomDye,
    Shoes,
    Skin,
}

pub const SLOTS: &[Slot] = &[
    Slot::Body,
    Slot::Hair,
    Slot::HairColor,
    Slot::Face,
    Slot::Top,
    Slot::TopDye,
    Slot::Bottom,
    Slot::BottomDye,
    Slot::Shoes,
    Slot::Skin,
];

impl Slot {
    pub fn label(self) -> &'static str {
        match self {
            Slot::Body => "Body Type",
            Slot::Hair => "Hairstyle",
            Slot::HairColor => "Hair Colour",
            Slot::Face => "Face",
            Slot::Top => "Top",
            Slot::TopDye => "Top Dye",
            Slot::Bottom => "Bottom",
            Slot::BottomDye => "Bottom Dye",
            Slot::Shoes => "Shoes",
            Slot::Skin => "Skin Tone",
        }
    }
    pub fn count(self) -> usize {
        match self {
            Slot::Body => BODIES.len(),
            Slot::Hair => HAIR_STYLES.len(),
            Slot::HairColor => HAIR_COLORS.len(),
            Slot::Face => FACES.len(),
            Slot::Top => TOPS.len(),
            Slot::TopDye | Slot::BottomDye => DYES.len(),
            Slot::Bottom => BOTTOMS.len(),
            Slot::Shoes => SHOES.len(),
            Slot::Skin => SKIN_TONES.len(),
        }
    }
    pub fn option_name(self, i: usize) -> &'static str {
        match self {
            Slot::Body => BODIES[i],
            Slot::Hair => HAIR_STYLES[i],
            Slot::HairColor => HAIR_COLORS[i].0,
            Slot::Face => FACES[i],
            Slot::Top => TOPS[i],
            Slot::TopDye | Slot::BottomDye => DYES[i].0,
            Slot::Bottom => BOTTOMS[i],
            Slot::Shoes => SHOES[i],
            Slot::Skin => SKIN_TONES[i].0,
        }
    }
}

impl Appearance {
    pub fn get(&self, s: Slot) -> u8 {
        match s {
            Slot::Body => self.body,
            Slot::Hair => self.hair,
            Slot::HairColor => self.hair_color,
            Slot::Face => self.face,
            Slot::Top => self.top,
            Slot::TopDye => self.top_dye,
            Slot::Bottom => self.bottom,
            Slot::BottomDye => self.bottom_dye,
            Slot::Shoes => self.shoes,
            Slot::Skin => self.skin,
        }
    }
    pub fn set(&mut self, s: Slot, v: u8) {
        let v = (v as usize % s.count()) as u8;
        match s {
            Slot::Body => self.body = v,
            Slot::Hair => self.hair = v,
            Slot::HairColor => self.hair_color = v,
            Slot::Face => self.face = v,
            Slot::Top => self.top = v,
            Slot::TopDye => self.top_dye = v,
            Slot::Bottom => self.bottom = v,
            Slot::BottomDye => self.bottom_dye = v,
            Slot::Shoes => self.shoes = v,
            Slot::Skin => self.skin = v,
        }
    }
    pub fn cycle(&mut self, s: Slot, delta: i32) {
        let n = s.count() as i32;
        let v = (self.get(s) as i32 + delta).rem_euclid(n);
        self.set(s, v as u8);
    }
    /// Clamp every index into range (protects against hand-edited save files).
    pub fn sanitized(mut self) -> Self {
        for &s in SLOTS {
            let v = self.get(s);
            self.set(s, v);
        }
        self
    }
    /// Random look. Body type is kept; clothes are always worn (never "None").
    pub fn randomize(&mut self, rng: &mut crate::math::Rng) {
        for &s in SLOTS {
            if s == Slot::Body {
                continue;
            }
            let v = (rng.next_u32() as usize % s.count()) as u8;
            self.set(s, v);
        }
        if self.top == 0 {
            self.top = 1 + (rng.next_u32() % (TOPS.len() as u32 - 1)) as u8;
        }
        if self.bottom == 0 {
            self.bottom = 1 + (rng.next_u32() % (BOTTOMS.len() as u32 - 1)) as u8;
        }
    }

    pub fn feminine(&self) -> bool {
        self.body == 1
    }

    /// Encode as comma-separated indices (save format v2).
    pub fn encode(&self) -> String {
        format!(
            "{},{},{},{},{},{},{},{},{},{}",
            self.body, self.hair, self.hair_color, self.face, self.top, self.top_dye, self.bottom, self.bottom_dye, self.shoes, self.skin
        )
    }

    /// Decode either format: 10 numbers (v2) or the original 9 numbers (v1,
    /// before body types and the "None" clothing option were added).
    pub fn decode(s: &str) -> Option<Appearance> {
        let n: Vec<u8> = s.split(',').map(|x| x.trim().parse().unwrap_or(0)).collect();
        let a = match n.len() {
            10 => Appearance { body: n[0], hair: n[1], hair_color: n[2], face: n[3], top: n[4], top_dye: n[5], bottom: n[6], bottom_dye: n[7], shoes: n[8], skin: n[9] },
            9 => Appearance {
                body: 0,
                hair: n[0],
                hair_color: n[1],
                face: n[2],
                top: n[3].saturating_add(1), // "None" was inserted at index 0
                top_dye: n[4],
                bottom: n[5].saturating_add(1),
                bottom_dye: n[6],
                shoes: n[7],
                skin: n[8],
            },
            _ => return None,
        };
        Some(a.sanitized())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn old_saves_keep_their_outfit() {
        // v1 "Leather Vest" (old index 1) + "Ranger Shorts" (old index 1)
        let a = Appearance::decode("3,4,0,1,2,1,4,4,2").unwrap();
        assert_eq!(TOPS[a.top as usize], "Leather Vest");
        assert_eq!(BOTTOMS[a.bottom as usize], "Ranger Shorts");
        assert_eq!(a.body, 0);
        let b = Appearance::decode(&a.encode()).unwrap();
        assert_eq!(a, b);
    }
    #[test]
    fn randomize_always_dresses() {
        let mut r = crate::math::Rng::new(5);
        for _ in 0..200 {
            let mut a = Appearance { body: 1, ..Default::default() };
            a.randomize(&mut r);
            assert!(a.top != 0 && a.bottom != 0 && a.body == 1);
        }
    }
}
