//! Character creation: customise body, hair, face, clothes, shoes and skin
//! tone with a live animated preview. Everyone starts as a Novice; the
//! left panel previews the five classes you can choose at level 10.

use crate::gfx::{self, rgb, rgba};
use crate::charsprite::{self, Anim};
use crate::render::{self, Camera};
use crate::screens::select::SelectScreen;
use crate::widgets::{self, ButtonStyle, TextField, Ui};
use crate::{App, Go, Screen};
use galebound_core::appearance::{Appearance, SLOTS};
use galebound_core::classes::{AttackKind, ClassId, CLASS_CHANGE_LEVEL, EVERY_CLASS};
use galebound_core::math::Rng;
use galebound_core::save::{CharacterSave, MAX_CHARACTERS};
use galebound_core::world::Theme;
use macroquad::prelude::{is_key_pressed, KeyCode};

#[derive(PartialEq, Clone, Copy)]
enum Preview {
    Idle,
    Walk,
    Attack,
}

pub struct CreateScreen {
    /// Class shown in the preview (the character itself is always a Novice).
    class: ClassId,
    look: Appearance,
    name: TextField,
    name_focused: bool,
    preview: Preview,
    facing: f32,
    error: Option<String>,
    rng: Rng,
    anim_start: f32,
    show_garb: bool,
}

pub fn validate_name(name: &str, existing: &[CharacterSave]) -> Result<String, String> {
    let n = galebound_core::save::validate_name(name)?;
    let n = n.as_str();
    if existing.iter().any(|c| c.name.eq_ignore_ascii_case(n)) {
        return Err("You already have a character with that name.".into());
    }
    Ok(n.to_string())
}

impl CreateScreen {
    pub fn new(app: &App) -> Self {
        let seed = (app.time * 1000.0) as u64 ^ 0x5eed;
        let mut rng = Rng::new(seed);
        let mut look = Appearance::default();
        look.randomize(&mut rng);
        CreateScreen {
            class: ClassId::Novice,
            look,
            name: TextField::new("Hero name", false, 14),
            name_focused: true,
            preview: Preview::Idle,
            facing: 1.0,
            error: None,
            rng,
            anim_start: 0.0,
            show_garb: true,
        }
    }

    pub fn frame(&mut self, app: &mut App) -> Go {
        gfx::begin_frame(rgb(120, 180, 240));
        let t = app.time as f32;
        render::background(Theme::Glade, &Camera { x: 1600.0 + t * 10.0, y: 0.0 }, t);
        gfx::clear_cam();
        gfx::rect(0.0, 0.0, gfx::VW, gfx::VH, rgba(20, 30, 60, 50));
        let ui = Ui::begin();
        if self.name_focused {
            self.name.type_into(&ui);
        }
        gfx::text_centered_outlined("Create Your Character", gfx::VW * 0.5, 16.0, 36.0, rgb(255, 244, 200));

        // ------------------------------------------------ class paths (left, preview only)
        let (lx, ly, lw, lh) = (24.0, 70.0, 360.0, 630.0);
        widgets::panel(lx, ly, lw, lh);
        widgets::title("Your Path", lx + lw * 0.5, ly + 16.0, 24.0);
        gfx::text_wrapped(
            &format!("Everyone starts as a Novice. At level {} you choose a class in its town. Preview them here:", CLASS_CHANGE_LEVEL),
            lx + 26.0,
            ly + 48.0,
            lw - 52.0,
            14.0,
            widgets::ink_soft(),
        );
        for (i, &c) in EVERY_CLASS.iter().enumerate() {
            let info = c.info();
            let by = ly + 112.0 + i as f32 * 40.0;
            let style = if c == self.class { ButtonStyle::Primary } else { ButtonStyle::Secondary };
            let label = if c == ClassId::Novice { "Novice (you start here)".to_string() } else { info.name.to_string() };
            if widgets::button(&ui, lx + 24.0, by, lw - 48.0, 34.0, &label, style, true) {
                self.class = c;
                self.anim_start = t;
            }
        }
        let info = self.class.info();
        let mut y = ly + 362.0;
        y += gfx::text_wrapped(&format!("{} - {}", info.role, info.weapon_name), lx + 26.0, y, lw - 52.0, 16.0, widgets::accent());
        y += 4.0;
        y += gfx::text_wrapped(info.blurb, lx + 26.0, y, lw - 52.0, 14.0, widgets::ink());
        y += 8.0;
        gfx::text(&format!("Main stats: {} / {}", info.primary.name(), info.secondary.name()), lx + 26.0, y, 14.0, widgets::ink_soft());
        y += 22.0;
        let labels = ["Power", "Defence", "Speed", "Range", "Difficulty"];
        for (i, l) in labels.iter().enumerate() {
            gfx::text(l, lx + 26.0, y, 14.0, widgets::ink_soft());
            for k in 0..5 {
                let filled = (k as u8) < info.ratings[i];
                let c = if filled { if i == 4 { widgets::danger() } else { widgets::gold() } } else { rgb(220, 206, 180) };
                gfx::rrect(lx + 130.0 + k as f32 * 36.0, y + 2.0, 30.0, 12.0, 4.0, c);
            }
            y += 21.0;
        }
        if self.class != ClassId::Novice {
            let town = galebound_core::world::load_map(info.town).map(|m| m.name).unwrap_or("its town");
            gfx::text_wrapped(&format!("Taught in {} from level {}.", town, CLASS_CHANGE_LEVEL), lx + 26.0, y + 4.0, lw - 52.0, 14.0, widgets::accent());
        }

        // ------------------------------------------------ preview (centre)
        let cx = 640.0;
        let base_y = 420.0;
        gfx::ellipse(cx, base_y + 8.0, 150.0, 34.0, 0.0, rgba(255, 255, 255, 70));
        gfx::ellipse(cx, base_y + 4.0, 120.0, 24.0, 0.0, rgba(80, 140, 90, 120));
        let at = t - self.anim_start;
        let attack = if self.preview == Preview::Attack {
            // Cycle through the class's signature moves in the preview.
            let kinds = [AttackKind::LightNeutral, AttackKind::LightSide, AttackKind::LightDown, AttackKind::HeavyNeutral, AttackKind::HeavySide, AttackKind::HeavyDown];
            let idx = ((at / 1.0) as usize) % kinds.len();
            Some((kinds[idx], at % 1.0))
        } else {
            None
        };
        let (anim, frame) = match (self.preview, attack) {
            (Preview::Attack, Some((k, p))) => {
                let f = if p < 0.15 { 0 } else if p < 0.35 { 1 } else if p < 0.55 { 2 } else { 3 };
                (Anim::Attack(charsprite::motion_for(self.class, k)), f)
            }
            (Preview::Walk, _) => (Anim::Run, charsprite::frame_for_loop(Anim::Run, at * 0.7)),
            _ => (Anim::Idle, charsprite::frame_for_loop(Anim::Idle, at)),
        };
        // Novices wear the Adventurer's Garb over the chosen outfit (take it off in the inventory to show it).
        let mut shown = self.look;
        if self.class == ClassId::Novice && self.show_garb {
            shown.top = 8;
            shown.top_dye = 5;
        }
        render::char_ui(&shown, Some(info.weapon), anim, frame, cx, base_y, 2.4, self.facing < 0.0);
        if let Some((k, _)) = attack {
            let name = galebound_core::classes::attack_def(self.class, k).name;
            gfx::text_centered_outlined(name, cx, base_y + 28.0, 20.0, rgb(255, 236, 160));
        }
        let bw = 96.0;
        let bx = cx - (bw * 4.0 + 24.0) * 0.5;
        for (i, (label, p)) in [("Idle", Some(Preview::Idle)), ("Walk", Some(Preview::Walk)), ("Attack", Some(Preview::Attack)), ("Turn", None)].iter().enumerate() {
            let style = if p.map(|p| p == self.preview).unwrap_or(false) { ButtonStyle::Primary } else { ButtonStyle::Secondary };
            if widgets::button(&ui, bx + i as f32 * (bw + 8.0), base_y + 64.0, bw, 36.0, label, style, true) {
                match p {
                    Some(p) => {
                        self.preview = *p;
                        self.anim_start = t;
                    }
                    None => self.facing = -self.facing,
                }
            }
        }

        if self.class == ClassId::Novice {
            let label = if self.show_garb { "Showing: Novice garb" } else { "Showing: your outfit" };
            if widgets::button(&ui, cx - 110.0, 64.0, 220.0, 30.0, label, ButtonStyle::Secondary, true) {
                self.show_garb = !self.show_garb;
            }
        }
        // Name field + create under the preview
        let nx = cx - 200.0;
        let ny = base_y + 146.0;
        if self.name.draw(&ui, nx, ny, 400.0, 44.0, "Character name", self.name_focused, app.time) {
            self.name_focused = true;
        }
        if let Some(e) = &self.error {
            widgets::message(e, cx, ny + 54.0, true);
        }

        // ------------------------------------------------ appearance (right)
        let (rx, ry, rw, rh) = (gfx::VW - 384.0, 70.0, 360.0, 630.0);
        widgets::panel(rx, ry, rw, rh);
        widgets::title("Appearance", rx + rw * 0.5, ry + 16.0, 24.0);
        let mut sy = ry + 50.0;
        for &slot in SLOTS {
            gfx::text(slot.label(), rx + 26.0, sy, 14.0, widgets::ink_soft());
            let ay = sy + 18.0;
            if widgets::arrow_button(&ui, rx + 26.0, ay, 26.0, true) {
                self.look.cycle(slot, -1);
            }
            let v = self.look.get(slot) as usize;
            let label = format!("{}  ({}/{})", slot.option_name(v), v + 1, slot.count());
            gfx::text_centered(&label, rx + rw * 0.5, ay + 6.0, 16.0, widgets::ink());
            if widgets::arrow_button(&ui, rx + rw - 52.0, ay, 26.0, false) {
                self.look.cycle(slot, 1);
            }
            sy += 50.0;
        }
        sy += 6.0;
        if widgets::button(&ui, rx + 26.0, sy, rw - 52.0, 40.0, "Randomise look", ButtonStyle::Secondary, true) {
            self.look.randomize(&mut self.rng);
        }

        // ------------------------------------------------ bottom buttons
        let existing = app.account.as_ref().map(|a| a.characters.clone()).unwrap_or_default();
        let full = existing.len() >= MAX_CHARACTERS;
        if widgets::button(&ui, cx - 200.0, gfx::VH - 70.0, 190.0, 50.0, "Back", ButtonStyle::Secondary, true)
            || is_key_pressed(KeyCode::Escape)
        {
            return Go::To(Screen::Select(SelectScreen::new(app)));
        }
        let create = widgets::button(&ui, cx + 10.0, gfx::VH - 70.0, 190.0, 50.0, "Create", ButtonStyle::Primary, !full)
            || (is_key_pressed(KeyCode::Enter) && !full);
        if create {
            match validate_name(&self.name.text, &existing) {
                Ok(name) => {
                    let c = CharacterSave::new(&name, self.look);
                    let msg = galebound_core::net::ClientMsg::CreateChar { save: galebound_core::save::encode_character(&c) };
                    match app.account_request(msg) {
                        Ok(()) => {
                            let mut sel = SelectScreen::new(app);
                            sel.select_last(app);
                            return Go::To(Screen::Select(sel));
                        }
                        Err(e) => self.error = Some(e),
                    }
                }
                Err(e) => self.error = Some(e),
            }
        }
        if full {
            widgets::message("All character slots are full - delete one first.", cx, ny + 54.0, true);
        }
        Go::Stay
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names() {
        assert!(validate_name("Skeng", &[]).is_ok());
        assert!(validate_name("ab", &[]).is_err());
        assert!(validate_name("bad!name", &[]).is_err());
    }
}
