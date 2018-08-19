use calx::{color, lerp, Rgba};
use crate::backend::Core;
use crate::brush::Brush;
use crate::draw_util::DrawUtil;
use crate::render::Layer;
use crate::view::ScreenVector;
use euclid::point2;
use std::cmp::Ordering;
use std::rc::Rc;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Coloring {
    /// Use map memory coloring for this sprite.
    MapMemory,
    /// Use the darkness level in [0.0, 1.0] for this sprite.
    Shaded { ambient: f32, diffuse: f32 },
}

impl Default for Coloring {
    fn default() -> Self {
        Coloring::Shaded {
            ambient: 1.0,
            diffuse: 1.0,
        }
    }
}

impl Eq for Coloring {}

impl Coloring {
    pub fn apply(self, fore: Rgba, back: Rgba) -> (Rgba, Rgba) {
        fn darken(c: f32, col: Rgba) -> Rgba {
            Rgba::new(
                col.r * c,
                col.g * lerp(0.2f32, 1.0f32, c),
                col.b * lerp(0.4f32, 1.0f32, c),
                col.a,
            )
        }

        match self {
            Coloring::MapMemory => (Rgba::from(0x2222_22ffu32), Rgba::from(0x0408_08ff)),
            Coloring::Shaded { ambient, diffuse } => {
                let (fore, back) = (
                    lerp(color::BLACK, fore, diffuse),
                    lerp(color::BLACK, back, diffuse),
                );
                let (fore, back) = (darken(ambient, fore), darken(ambient, back));
                (fore, back)
            }
        }
    }
}

/// Drawable display element.
///
/// Sprites are basically a way to defer somewhat complex draw instructions. The reason they exist
/// is that scene draw order is not necessarily trivially reflectable in scene data traversal, so
/// emitting sprites and then sorting them is the simplest way to go ahead.
#[derive(Clone, PartialEq)]
pub struct Sprite {
    pub layer: Layer,
    // XXX: Not using Point2D<f32> because floats don't have Ord.
    pub offset: [i32; 2],

    // TODO: Replace this with a generic "Drawable" trait object once we start having other things
    // than frames as sprites.
    pub brush: Rc<Brush>,
    pub frame_idx: usize,
    pub color: Coloring,
}

impl Sprite {
    pub fn new(layer: Layer, offset: ScreenVector, brush: Rc<Brush>) -> Sprite {
        let offset = [offset.x as i32, offset.y as i32];
        Sprite {
            layer,
            offset,
            brush,
            frame_idx: 0,
            color: Default::default(),
        }
    }

    pub fn idx(mut self, frame_idx: usize) -> Sprite {
        self.frame_idx = frame_idx;
        self
    }

    pub fn color(mut self, color: Coloring) -> Sprite {
        self.color = color;
        self
    }
}

impl Eq for Sprite {}

impl Ord for Sprite {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.layer, self.offset[1]).cmp(&(other.layer, other.offset[1]))
    }
}

impl PartialOrd for Sprite {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}

impl Sprite {
    pub fn draw(&self, core: &mut Core) {
        let pos = point2(self.offset[0] as f32, self.offset[1] as f32);
        for splat in &self.brush[self.frame_idx] {
            let (fore, back) = self.color.apply(splat.color, splat.back_color);
            core.draw_image_2color(&splat.image, pos - splat.offset, fore.into(), back.into());
        }
    }
}
