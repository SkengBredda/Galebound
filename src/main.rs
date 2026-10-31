//! Galebound Online - a side-scrolling action MMO prototype.
//!
//! Screens: Login -> Character Select -> (Character Creation) -> Game.
//! All gameplay lives in the dependency-free `galebound-core` crate; this
//! crate only draws, reads input and moves between screens.

mod art;
mod charsprite;
mod display;
mod font;
mod gfx;
mod hud;
mod keys;
mod lootart;
mod minimap;
mod petart;
mod pixel;
mod render;
mod screens;
mod settings_panel;
mod sprites;
mod townart;
mod icons;
mod widgets;
mod windows;

use galebound_core::auth::AccountStore;
use galebound_core::save::Account;
use macroquad::prelude::{get_time, is_quit_requested, next_frame, prevent_quit, Conf};
use screens::{create::CreateScreen, login::LoginScreen, play::PlayScreen, select::SelectScreen};

pub const GAME_TITLE: &str = "Galebound Online";
pub const VERSION: &str = "0.7.0";

/// Shared state that outlives individual screens.
pub struct App {
    pub store: AccountStore,
    pub account: Option<Account>,
    pub time: f64,
    pub quit: bool,
    /// Last save error, shown on screen so problems are never silent.
    pub save_error: Option<String>,
}

impl App {
    pub fn save_account(&mut self) {
        if let Some(acc) = &self.account {
            match self.store.save(acc) {
                Ok(()) => self.save_error = None,
                Err(e) => self.save_error = Some(e.to_string()),
            }
        }
    }
}

pub enum Screen {
    Login(LoginScreen),
    Select(SelectScreen),
    Create(CreateScreen),
    Play(Box<PlayScreen>),
}

pub enum Go {
    Stay,
    To(Screen),
}

fn window_conf() -> Conf {
    let mode = display::saved_mode();
    display::started_in(mode);
    Conf {
        window_title: GAME_TITLE.to_owned(),
        window_width: display::WINDOW_W as i32,
        window_height: display::WINDOW_H as i32,
        fullscreen: mode != galebound_core::save::DisplayMode::Windowed,
        window_resizable: true,
        high_dpi: true,
        sample_count: 4,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    prevent_quit();
    gfx::init();
    let mut app = App { store: AccountStore::default_location(), account: None, time: 0.0, quit: false, save_error: None };
    let mut screen = Screen::Login(LoginScreen::new(&app));

    loop {
        app.time = get_time();
        if is_quit_requested() {
            app.quit = true;
        }
        let go = match &mut screen {
            Screen::Login(s) => s.frame(&mut app),
            Screen::Select(s) => s.frame(&mut app),
            Screen::Create(s) => s.frame(&mut app),
            Screen::Play(s) => s.frame(&mut app),
        };
        if app.quit {
            // Save progress before closing the window.
            if let Screen::Play(p) = &mut screen {
                p.save_now(&mut app);
            }
            app.save_account();
            break;
        }
        if let Go::To(next) = go {
            screen = next;
        }
        gfx::end_frame();
        next_frame().await;
    }
}
