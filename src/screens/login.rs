//! Login / registration screen with "remember me" (and its shared-PC warning).

use crate::gfx::{self, rgb, rgba};
use crate::render::{self, Camera};
use crate::screens::select::SelectScreen;
use crate::widgets::{self, ButtonStyle, TextField, Ui};
use crate::{App, Go, Screen};
use galebound_core::auth::{self, Remembered};
use galebound_core::world::Theme;
use macroquad::prelude::{is_key_down, is_key_pressed, KeyCode};

#[derive(PartialEq, Clone, Copy)]
enum Mode {
    Login,
    Register,
}

#[derive(PartialEq, Clone, Copy)]
enum Focus {
    Email,
    Password,
    Confirm,
}

/// Work that runs one frame after the button press, so the "Signing in..."
/// text is visible while the password is being hashed.
#[derive(PartialEq, Clone, Copy)]
enum Pending {
    None,
    Login,
    Register,
    Remembered,
}

pub struct LoginScreen {
    mode: Mode,
    email: TextField,
    password: TextField,
    confirm: TextField,
    focus: Focus,
    remember: bool,
    show_warning: bool,
    error: Option<String>,
    info: Option<String>,
    pending: Pending,
    pending_frames: u8,
    remembered: Option<Remembered>,
}

impl LoginScreen {
    pub fn new(app: &App) -> Self {
        let remembered = app.store.remembered();
        let mut email = TextField::new("you@example.com", false, 254);
        if let Some(r) = &remembered {
            email.text = r.email.clone();
        }
        LoginScreen {
            mode: Mode::Login,
            email,
            password: TextField::new("Password", true, 128),
            confirm: TextField::new("Repeat password", true, 128),
            focus: Focus::Email,
            remember: false,
            show_warning: false,
            error: None,
            info: None,
            pending: Pending::None,
            pending_frames: 0,
            remembered,
        }
    }

    fn focused_field(&mut self) -> &mut TextField {
        match self.focus {
            Focus::Email => &mut self.email,
            Focus::Password => &mut self.password,
            Focus::Confirm => &mut self.confirm,
        }
    }

    fn start(&mut self, p: Pending) {
        self.pending = p;
        self.pending_frames = 0;
        self.error = None;
    }

    fn run_pending(&mut self, app: &mut App) -> Go {
        let p = self.pending;
        self.pending = Pending::None;
        let result = match p {
            Pending::Login => app.store.login(&self.email.text, &self.password.text),
            Pending::Register => app.store.register(&self.email.text, &self.password.text, &self.confirm.text),
            Pending::Remembered => match &self.remembered {
                Some(r) => app.store.login_remembered(r),
                None => return Go::Stay,
            },
            Pending::None => return Go::Stay,
        };
        match result {
            Ok(mut acc) => {
                if p != Pending::Remembered {
                    if self.remember {
                        if let Err(e) = app.store.remember(&mut acc) {
                            self.error = Some(e.to_string());
                        }
                    } else if self.remembered.as_ref().map(|r| r.email == acc.email).unwrap_or(false) {
                        // Logged in normally with "remember" off: clear the old token.
                        app.store.forget(Some(&mut acc));
                    }
                }
                self.password.text.clear();
                self.confirm.text.clear();
                app.account = Some(acc);
                Go::To(Screen::Select(SelectScreen::new(app)))
            }
            Err(e) => {
                if p == Pending::Remembered {
                    self.remembered = None;
                    self.error = Some("Your saved login has expired. Please sign in again.".into());
                } else {
                    self.error = Some(e.to_string());
                }
                Go::Stay
            }
        }
    }

    pub fn frame(&mut self, app: &mut App) -> Go {
        gfx::begin_frame(rgb(120, 180, 240));
        let t = app.time as f32;
        // Animated world backdrop
        let cam = Camera { x: t * 40.0, y: 0.0 };
        render::background(Theme::Glade, &cam, t);
        gfx::clear_cam();
        gfx::rect(0.0, 0.0, gfx::VW, gfx::VH, rgba(20, 30, 60, 60));

        // Title
        let bob = (t * 1.5).sin() * 4.0;
        gfx::text_centered_outlined(crate::GAME_TITLE, gfx::VW * 0.5, 50.0 + bob, 64.0, rgb(255, 244, 200));
        gfx::text_centered_outlined("Ride the wind. Master the blade.", gfx::VW * 0.5, 122.0, 20.0, rgb(230, 240, 255));

        if self.pending != Pending::None {
            self.pending_frames += 1;
            let (px, py, pw, ph) = (440.0, 300.0, 400.0, 120.0);
            widgets::panel(px, py, pw, ph);
            let msg = if self.pending == Pending::Register { "Creating your account..." } else { "Signing in..." };
            widgets::title(msg, px + pw * 0.5, py + 46.0, 24.0);
            if self.pending_frames >= 2 {
                return self.run_pending(app);
            }
            return Go::Stay;
        }

        let mut ui = Ui::begin();
        if self.show_warning {
            ui.enabled = false;
        }

        // Remembered account shortcut
        if let Some(r) = self.remembered.clone() {
            if self.mode == Mode::Login {
                let (px, py, pw, ph) = (440.0, 180.0, 400.0, 230.0);
                widgets::panel(px, py, pw, ph);
                widgets::title("Welcome back!", px + pw * 0.5, py + 28.0, 28.0);
                gfx::text_centered(&r.email, px + pw * 0.5, py + 72.0, 20.0, widgets::ink_soft());
                if widgets::button(&ui, px + 40.0, py + 108.0, pw - 80.0, 48.0, "Continue", ButtonStyle::Primary, true)
                    || (ui.enabled && is_key_pressed(KeyCode::Enter))
                {
                    self.start(Pending::Remembered);
                }
                if widgets::button(&ui, px + 40.0, py + 166.0, pw - 80.0, 38.0, "Use a different account", ButtonStyle::Secondary, true) {
                    let mut acc = app.store.load(&r.email).ok();
                    app.store.forget(acc.as_mut());
                    self.remembered = None;
                    self.email.text.clear();
                }
                if let Some(e) = &self.error {
                    widgets::message(e, gfx::VW * 0.5, py + ph + 16.0, true);
                }
                footer(&app.save_error);
                return Go::Stay;
            }
        }

        let (px, py, pw) = (420.0, 160.0, 440.0);
        let ph = if self.mode == Mode::Register { 470.0 } else { 420.0 };
        widgets::panel(px, py, pw, ph);
        let heading = if self.mode == Mode::Login { "Sign In" } else { "Create Account" };
        widgets::title(heading, px + pw * 0.5, py + 24.0, 30.0);

        // Keyboard: Tab cycles fields, Enter submits.
        if ui.enabled && is_key_pressed(KeyCode::Tab) {
            let shift = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
            self.focus = match (self.focus, self.mode, shift) {
                (Focus::Email, _, false) => Focus::Password,
                (Focus::Password, Mode::Register, false) => Focus::Confirm,
                (Focus::Password, Mode::Login, false) => Focus::Email,
                (Focus::Confirm, _, false) => Focus::Email,
                (Focus::Email, Mode::Register, true) => Focus::Confirm,
                (Focus::Email, Mode::Login, true) => Focus::Password,
                (Focus::Password, _, true) => Focus::Email,
                (Focus::Confirm, _, true) => Focus::Password,
            };
        }
        if ui.enabled {
            self.focused_field().type_into(&ui);
        }

        let fx = px + 40.0;
        let fw = pw - 80.0;
        let mut y = py + 100.0;
        if self.email.draw(&ui, fx, y, fw, 42.0, "E-mail", self.focus == Focus::Email, app.time) {
            self.focus = Focus::Email;
        }
        y += 76.0;
        if self.password.draw(&ui, fx, y, fw, 42.0, "Password", self.focus == Focus::Password, app.time) {
            self.focus = Focus::Password;
        }
        y += 50.0;
        if self.mode == Mode::Register {
            y += 26.0;
            if self.confirm.draw(&ui, fx, y, fw, 42.0, "Confirm password", self.focus == Focus::Confirm, app.time) {
                self.focus = Focus::Confirm;
            }
            y += 50.0;
            gfx::text(&format!("At least {} characters.", auth::MIN_PASSWORD_LEN), fx, y - 4.0, 15.0, widgets::ink_soft());
            y += 14.0;
        }

        // Show password + remember me
        if widgets::checkbox(&ui, fx, y, "Show password", self.password.reveal) {
            self.password.reveal = !self.password.reveal;
            self.confirm.reveal = self.password.reveal;
        }
        if widgets::checkbox(&ui, fx + 190.0, y, "Remember me", self.remember) {
            if self.remember {
                self.remember = false;
            } else {
                self.show_warning = true;
            }
        }
        y += 40.0;

        let label = if self.mode == Mode::Login { "Sign In" } else { "Create Account" };
        let submit = widgets::button(&ui, fx, y, fw, 48.0, label, ButtonStyle::Primary, true)
            || (ui.enabled && (is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter)));
        if submit {
            self.start(if self.mode == Mode::Login { Pending::Login } else { Pending::Register });
        }
        y += 62.0;
        let switch = if self.mode == Mode::Login { "New here? Create an account" } else { "Have an account? Sign in" };
        if widgets::button(&ui, fx, y, fw, 36.0, switch, ButtonStyle::Secondary, true) {
            self.mode = if self.mode == Mode::Login { Mode::Register } else { Mode::Login };
            self.error = None;
            self.info = None;
            self.focus = Focus::Email;
        }

        if let Some(e) = &self.error {
            widgets::message(e, gfx::VW * 0.5, py + ph + 14.0, true);
        } else if let Some(i) = &self.info {
            widgets::message(i, gfx::VW * 0.5, py + ph + 14.0, false);
        }

        if self.show_warning {
            self.warning_modal();
        }
        footer(&app.save_error);
        Go::Stay
    }

    fn warning_modal(&mut self) {
        let ui = Ui::begin();
        gfx::rect(0.0, 0.0, gfx::VW, gfx::VH, rgba(10, 14, 30, 170));
        let (px, py, pw, ph) = (340.0, 190.0, 600.0, 320.0);
        widgets::panel(px, py, pw, ph);
        gfx::circle(px + pw * 0.5, py + 44.0, 22.0, widgets::gold());
        gfx::text_centered("!", px + pw * 0.5, py + 30.0, 30.0, widgets::ink());
        widgets::title("Shared or public computer?", px + pw * 0.5, py + 76.0, 26.0);
        let body = "\"Remember me\" keeps you signed in on this computer. Anyone who uses it after you \
                    could open your account and play your characters. Only turn this on if this is \
                    your own personal device - never on a school, library, internet cafe or shared PC.";
        gfx::text_wrapped(body, px + 40.0, py + 116.0, pw - 80.0, 18.0, widgets::ink());
        if widgets::button(&ui, px + 40.0, py + ph - 76.0, 250.0, 46.0, "Cancel", ButtonStyle::Secondary, true)
            || is_key_pressed(KeyCode::Escape)
        {
            self.show_warning = false;
        }
        if widgets::button(&ui, px + pw - 290.0, py + ph - 76.0, 250.0, 46.0, "It's my PC - remember me", ButtonStyle::Primary, true) {
            self.remember = true;
            self.show_warning = false;
        }
    }
}

fn footer(save_error: &Option<String>) {
    gfx::text_centered_outlined(
        "Accounts are stored on this computer for now. Passwords are salted and hashed.",
        gfx::VW * 0.5,
        gfx::VH - 34.0,
        15.0,
        rgb(240, 244, 255),
    );
    gfx::text_outlined(crate::VERSION, gfx::VW - 16.0 - gfx::text_w(crate::VERSION, 13.0), 12.0, 13.0, rgb(220, 230, 255));
    if let Some(e) = save_error {
        gfx::text_centered_outlined(e, gfx::VW * 0.5, gfx::VH - 60.0, 15.0, widgets::danger());
    }
}
