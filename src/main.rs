#![crate_name="magog"]
#![feature(old_io, old_path, unicode)]

extern crate image;
extern crate "calx_util" as util;

#[macro_use]
extern crate "calx_backend" as backend;

extern crate world;
extern crate time;

use gamestate::GameState;
use titlestate::TitleState;

pub static SCREEN_W: u32 = 640;
pub static SCREEN_H: u32 = 360;

pub mod drawable;
pub mod tilecache;
pub mod viewutil;
pub mod worldview;
mod gamestate;
mod titlestate;
mod sprite;
mod msg_queue;

pub trait State {
    fn process(&mut self, event: backend::Event) -> Option<Transition>;
}

pub enum Transition {
    Game(Option<u32>),
    Title,
    Exit,
}

pub fn version() -> String {
    let next_release = "0.1.0";
    let git_hash = include_str!("git_hash.inc");
    // Set is_release to true for one commit to make a release.
    let is_release = false;

    if is_release {
        format!("{}", next_release)
    } else {
        format!("{}-alpha+g{}", next_release, git_hash)
    }
}

pub fn compiler_version() -> String {
    include_str!("rustc_version.inc").to_string()
}

pub fn main() {
    println!("Magog v{}", version());
    println!("{}", compiler_version());
    let mut canvas = backend::CanvasBuilder::new()
        .set_size(SCREEN_W, SCREEN_H)
        .set_frame_interval(0.030f64);
    tilecache::init(&mut canvas);
    let mut state: Box<State> = Box::new(TitleState::new());

    for evt in canvas.run() {
        match state.process(evt) {
            Some(Transition::Title) => { state = Box::new(TitleState::new()); }
            Some(Transition::Game(seed)) => {
                state = Box::new(GameState::new(seed)); }
            Some(Transition::Exit) => { break; }
            _ => ()
        }
    }
}
