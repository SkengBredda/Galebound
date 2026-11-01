// Release builds on Windows open just the game window, without a console.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
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
mod online;
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

use galebound_core::net::{ClientMsg, NetClient, ServerMsg};
use galebound_core::save::{self, Account};
use std::time::Duration;
use macroquad::prelude::{get_time, is_quit_requested, next_frame, prevent_quit, Conf};
use screens::{create::CreateScreen, login::LoginScreen, play::PlayScreen, select::SelectScreen};

pub const GAME_TITLE: &str = "Galebound Online";
pub const VERSION: &str = "0.8.1";

/// How long to wait for the server to answer a menu action.
pub const REPLY_WAIT: Duration = Duration::from_secs(8);

/// Shared state that outlives individual screens.
pub struct App {
    /// Your account as the server sent it (no password data).
    pub account: Option<Account>,
    /// The connection to the server, once signed in.
    pub net: Option<NetClient>,
    pub time: f64,
    pub quit: bool,
    /// Last save / connection problem, shown on screen so it's never silent.
    pub save_error: Option<String>,
}

impl App {
    /// Send the account's settings to the server.
    pub fn save_settings(&mut self) {
        if let (Some(acc), Some(net)) = (&self.account, &self.net) {
            net.send(&ClientMsg::Settings { text: save::encode_settings(&acc.settings) });
        }
    }

    /// Ask the server for something that answers with the account (create /
    /// delete a character) and keep the new copy.
    pub fn account_request(&mut self, m: ClientMsg) -> Result<(), String> {
        let net = self.net.as_mut().ok_or("Not connected")?;
        match net.request(&m, REPLY_WAIT, |r| matches!(r, ServerMsg::Account { .. }))? {
            ServerMsg::Account { account, .. } => {
                self.account = Some(save::decode_account(&account)?);
                Ok(())
            }
            _ => Err("Unexpected reply".into()),
        }
    }

    /// Lost the server? (checked by the menus)
    pub fn connection_lost(&self) -> bool {
        self.net.as_ref().map(|n| n.is_closed()).unwrap_or(true)
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
    let mut app = App { account: None, net: None, time: 0.0, quit: false, save_error: None };
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
                if let Some(n) = &app.net {
                    n.send(&ClientMsg::Leave);
                }
            }
            // give the writer thread a moment to send the last save
            std::thread::sleep(Duration::from_millis(150));
            break;
        }
        if let Go::To(next) = go {
            screen = next;
        }
        gfx::end_frame();
        next_frame().await;
    }
}
