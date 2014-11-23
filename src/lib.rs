#![crate_name="calx"]
#![feature(phase)]
#![feature(tuple_indexing)]
#![feature(if_let)]

extern crate time;
extern crate collections;
extern crate serialize;

extern crate glutin;
extern crate gfx;
#[phase(plugin)]
extern crate gfx_macros;
extern crate device;
extern crate image;

pub use canvas::{Canvas, Context};
pub use canvas::{Image};
pub use canvas_util::{CanvasUtil};
pub use rgb::{Rgb, Rgba};
pub use geom::{Rect, V2, RectIter};
pub use util::{color_key};
pub use util::{Primitive};
pub use fonter::{Fonter, CanvasWriter};
pub use event::{Event};

pub use glutin::VirtualKeyCode as Key;

mod atlas;
mod canvas;
mod canvas_util;
mod event;
mod fonter;
mod geom;
mod renderer;
mod rgb;
mod util;
pub mod color;
pub mod dijkstra;
pub mod text;
pub mod timing;
pub mod vorud;

pub trait Color {
    fn to_rgba(&self) -> [f32, ..4];
}
