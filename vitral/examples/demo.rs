extern crate euclid;
extern crate vitral;

use euclid::{point2, rect, Rect};
use vitral::{Align, AppConfig, ButtonAction, Color, Core, PngBytes, RectUtil, Scene, SceneSwitch};

struct World {
    font: vitral::FontData,
    image: vitral::ImageData,
    fore_color: Color,
    back_color: Color,
}

impl World {
    pub fn new() -> World {
        let font = vitral::add_tilesheet_font(
            "font",
            PngBytes(include_bytes!("../tilesheet-font.png")),
            (32u8..128).map(|c| c as char),
        );
        let image = vitral::add_sheet("julia", PngBytes(include_bytes!("../julia.png")));
        let image = vitral::get_image(&image).unwrap();

        World {
            font,
            image,
            fore_color: [1.0, 0.5, 0.1, 1.0],
            back_color: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

struct DemoScene;

impl Scene<World> for DemoScene {
    fn update(&mut self, ctx: &mut World) -> Option<SceneSwitch<World>> { None }

    fn render(&mut self, ctx: &mut World, core: &mut Core) -> Option<SceneSwitch<World>> {
        core.draw_image(&ctx.image, point2(20, 20), [1.0, 1.0, 1.0, 1.0]);
        let bounds = core.bounds();

        let (_, title_area) = bounds.horizontal_split(12);
        self.title_bar(ctx, core, &title_area, "Vitral Demo");

        let (_, widget_area) = title_area.vertical_split(-12);
        if self.quit_button(ctx, core, &widget_area) {
            return Some(SceneSwitch::Pop);
        }

        None
    }
}

impl DemoScene {
    fn bright_color(&self) -> Color { [1.0, 0.7, 0.2, 1.0] }

    fn title_bar(&self, ctx: &World, core: &mut Core, bounds: &Rect<i32>, text: &str) {
        core.fill_rect(bounds, ctx.back_color);
        {
            let bounds = bounds.inclusivize();
            core.draw_line(
                1.0,
                ctx.fore_color,
                bounds.bottom_left(),
                bounds.bottom_right(),
            );
        }

        // Margin
        let bounds = bounds.inflate(-2, -2);

        core.draw_text(
            &ctx.font,
            bounds.anchor(&point2(0, -1)),
            Align::Center,
            ctx.fore_color,
            text,
        );
    }

    fn quit_button(&self, ctx: &World, core: &mut Core, bounds: &Rect<i32>) -> bool {
        let click_state = core.click_state(bounds);

        let color = if click_state != ButtonAction::Inert {
            self.bright_color()
        } else {
            ctx.fore_color
        };

        core.fill_rect(bounds, color);
        core.fill_rect(&bounds.inflate(-1, -1), ctx.back_color);

        let inner = bounds.inflate(-3, -3).inclusivize();

        core.draw_line(1.0, color, inner.bottom_right(), inner.origin);

        core.draw_line(1.0, color, inner.top_right(), inner.bottom_left());

        core.click_state(bounds) == ButtonAction::LeftClicked
    }
}

fn main() {
    vitral::run_app(
        AppConfig::new("Vitral Demo"),
        World::new(),
        vec![Box::new(DemoScene)],
    );
}
