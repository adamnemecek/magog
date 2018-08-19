use calx::Dir6;
use calx::{color, command_parser, Rgba};
use display::{self, Backend, Core};
use euclid::{Point2D, Rect};
use glium::glutin::ElementState;
use scancode::Scancode;
use std::fs::File;
use std::io::prelude::*;
use std::rc::Rc;
use vitral::{Align, FontData, RectUtil};
use world::{Command, CommandResult, Event, ItemType, Location, Mutate, Query, Slot, World};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum State {
    Main,
    Inventory(InventoryAction),
    Console,
    Aim(AimAction),
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum InventoryAction {
    Drop,
    Equip,
    Use,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum AimAction {
    Zap(Slot),
    // Maybe add intrinsic abilities not tied to a specific entity later
}

pub struct GameLoop {
    core: Core,
    font: Rc<FontData>,
    pub world: World,
    pub console: display::Console,
    camera_loc: Location,
    state: State,
}

enum Side {
    West,
    East,
}

impl GameLoop {
    pub fn new(backend: &mut Backend, world: World) -> GameLoop {
        let font = display::font();
        GameLoop {
            core: backend.new_core(),
            font: font.clone(),
            world,
            console: display::Console::new(font),
            camera_loc: Location::new(0, 0, 0),
            state: State::Main,
        }
    }

    /// Step command that turns into melee attack if an enemy is in the way.
    fn smart_step(&mut self, dir: Dir6) -> CommandResult {
        let player = self.world.player().ok_or(())?;
        let loc = self.world.location(player).ok_or(())?;
        let destination = loc.jump(&self.world, dir);

        if let Some(mob) = self.world.mob_at(destination) {
            if self.world.is_hostile_to(player, mob) {
                // Fight on!
                self.world.melee(dir)
            } else {
                // Do we want to do something smarter than walk into friendlies?
                // The world might treat this as a displace action so keep it like this for now.
                self.world.step(dir)
            }
        } else {
            self.world.step(dir)
        }
    }

    fn side_step(&mut self, side: Side) -> CommandResult {
        let player = self.world.player().ok_or(())?;
        let loc = self.world.location(player).ok_or(())?;
        let flip = (loc.x + loc.y) % 2 == 0;

        let actual_dir = match side {
            Side::West => if flip {
                Dir6::Southwest
            } else {
                Dir6::Northwest
            },
            Side::East => if flip {
                Dir6::Southeast
            } else {
                Dir6::Northeast
            },
        };

        self.smart_step(actual_dir)
    }

    fn game_input(&mut self, backend: &mut Backend, scancode: Scancode) -> CommandResult {
        use scancode::Scancode::*;
        match scancode {
            Tab => {
                self.enter_state(State::Console);
                Ok(Vec::new())
            }
            Q | Pad7 | Home => self.smart_step(Dir6::Northwest),
            W | Up | Pad8 => self.smart_step(Dir6::North),
            E | Pad9 | PageUp => self.smart_step(Dir6::Northeast),
            A | Pad1 | End => self.smart_step(Dir6::Southwest),
            S | Down | Pad2 => self.smart_step(Dir6::South),
            D | Pad3 | PageDown => self.smart_step(Dir6::Southeast),
            Left | Pad4 => self.side_step(Side::West),
            Right | Pad6 => self.side_step(Side::East),
            I => {
                self.enter_state(State::Inventory(InventoryAction::Equip));
                Ok(Vec::new())
            }
            // XXX: Key mnemonics, bit awkward when D is taken by movement.
            B => {
                self.enter_state(State::Inventory(InventoryAction::Drop));
                Ok(Vec::new())
            }
            U => {
                self.enter_state(State::Inventory(InventoryAction::Use));
                Ok(Vec::new())
            }
            G => self.world.take(),
            Space | Pad5 => self.world.pass(),
            F5 => {
                self.world
                    .save(&mut File::create("save.gam").unwrap())
                    .unwrap();
                Ok(Vec::new())
            }
            F9 => {
                let mut savefile = File::open("save.gam").unwrap();
                self.world = World::load(&mut savefile).unwrap();
                Ok(Vec::new())
            }
            F12 => {
                backend.save_screenshot("magog");
                Ok(Vec::new())
            }
            _ => Ok(Vec::new()),
        }
    }

    fn zap(&mut self, slot: Slot, dir: Dir6) -> CommandResult {
        let ret = self.world.zap_item(slot, dir)?;
        self.enter_state(State::Main);
        Ok(ret)
    }

    fn aim_input(&mut self, slot: Slot, scancode: Scancode) -> CommandResult {
        use scancode::Scancode::*;
        match scancode {
            Q => self.zap(slot, Dir6::Northwest),
            W => self.zap(slot, Dir6::North),
            E => self.zap(slot, Dir6::Northeast),
            A => self.zap(slot, Dir6::Southwest),
            S => self.zap(slot, Dir6::South),
            D => self.zap(slot, Dir6::Southeast),
            Escape => {
                self.enter_state(State::Main);
                Ok(Vec::new())
            }
            _ => Ok(Vec::new()),
        }
    }

    fn inventory_input(&mut self, scancode: Scancode) -> CommandResult {
        use scancode::Scancode::*;
        for slot in SLOT_DATA.iter() {
            if scancode == slot.code {
                if let State::Inventory(action) = self.state {
                    let ret = self.inventory_action(slot.slot, action);
                    if ret.is_ok() {
                        return ret;
                    }
                }
            }
        }

        match scancode {
            Escape => {
                self.enter_state(State::Main);
                Ok(Vec::new())
            }
            _ => Ok(Vec::new()),
        }
    }

    fn enter_state(&mut self, new_state: State) {
        if self.state == new_state {
            return;
        }

        if let State::Aim(_) = new_state {
            let _ = writeln!(&mut self.console, "Press direction to aim, Esc to cancel");
        }

        self.state = new_state;
    }

    fn inventory_action(&mut self, slot: Slot, action: InventoryAction) -> CommandResult {
        match action {
            InventoryAction::Drop => {
                let ret = self.world.drop(slot);
                // After succesful drop, go back to main state.
                if ret.is_ok() {
                    self.enter_state(State::Main);
                }
                ret
            }
            // Can equip multiple items in one go, wait for ESC to return to main state.
            InventoryAction::Equip => self.world.equip(slot),
            InventoryAction::Use => {
                let player = self.world.player().ok_or(())?;

                if let Some(item) = self.world.entity_equipped(player, slot) {
                    match self.world.item_type(item) {
                        Some(ItemType::UntargetedUsable(_)) => {
                            let ret = self.world.use_item(slot)?;
                            self.enter_state(State::Main);
                            return Ok(ret);
                        }
                        Some(ItemType::TargetedUsable(_)) => {
                            // If we need to aim, switch to aim state before calling world.
                            self.enter_state(State::Aim(AimAction::Zap(slot)));
                            return Ok(Vec::new());
                        }
                        _ => {}
                    }
                }
                Err(())
            }
        }
    }

    fn console_input(&mut self, scancode: Scancode) -> CommandResult {
        use scancode::Scancode::*;
        match scancode {
            Tab => {
                self.enter_state(State::Main);
                Ok(Vec::new())
            }
            Enter | PadEnter => {
                let input = self.console.get_input();
                let _ = writeln!(&mut self.console, "{}", input);
                if let Err(e) = self.parse(&input) {
                    let _ = writeln!(&mut self.console, "{}", e);
                }
                Ok(Vec::new())
            }
            _ => Ok(Vec::new()),
        }
    }

    fn todo(&mut self) {
        // TODO: Bring back some debug commands
    }

    command_parser!{
        fn todo(&mut self);
    }

    fn draw_inventory(&mut self) -> Result<(), ()> {
        let player = self.world.player().ok_or(())?;

        // Start with hardcoded invetory data to test the UI logic.
        let bounds = self.core.bounds();
        self.core.fill_rect(&bounds, [0.0, 0.0, 0.0, 0.99]);

        let mut letter_pos = Point2D::new(0.0, 0.0);
        let mut slot_name_pos = Point2D::new(20.0, 0.0);
        let mut item_name_pos = Point2D::new(80.0, 0.0);
        let text_color = [1.0, 1.0, 1.0, 1.0];

        for slot in SLOT_DATA.iter() {
            // TODO: Bounding box for these is a button...
            letter_pos = self.core.draw_text(
                &*self.font,
                letter_pos,
                Align::Left,
                text_color,
                &format!("{})", slot.key),
            );
            slot_name_pos = self.core.draw_text(
                &*self.font,
                slot_name_pos,
                Align::Left,
                text_color,
                slot.name,
            );
            let item_name = if let Some(item) = self.world.entity_equipped(player, slot.slot) {
                self.world.entity_name(item)
            } else {
                "".to_string()
            };

            item_name_pos = self.core.draw_text(
                &*self.font,
                item_name_pos,
                Align::Left,
                text_color,
                &item_name,
            );
        }

        Ok(())
    }

    pub fn status_draw(&mut self, area: &Rect<f32>) {
        self.core.fill_rect(area, Rgba::from(0x33_11_11_ff).into());
        self.core.draw_text(
            &*self.font,
            area.origin,
            Align::Left,
            color::RED.into(),
            "Welcome to status bar",
        );
    }

    /// Entry point for game view.
    pub fn draw(&mut self, backend: &mut Backend) -> bool {
        self.core.begin_frame();
        let screen_area = self.core.screen_bounds();

        let (view_area, status_area) = screen_area.horizontal_split(-32.0);

        // Ugh
        self.world
            .player()
            .map(|x| self.world.location(x).map(|l| self.camera_loc = l));

        let mut view = display::WorldView::new(self.camera_loc, view_area);
        view.show_cursor = true;

        self.core.set_clip(view_area);
        view.draw(&self.world, &mut self.core);
        self.core.clear_clip();

        self.core.set_clip(status_area);
        self.status_draw(&status_area);
        self.core.clear_clip();

        match self.state {
            State::Inventory(_) => {
                let _ = self.draw_inventory();
            }
            State::Console => {
                let mut console_area = screen_area;
                console_area.size.height = 184.0;
                self.console.draw_large(&mut self.core, &console_area);
            }
            _ => {
                let mut console_area = screen_area;
                console_area.size.height = 32.0;
                self.console.draw_small(&mut self.core, &console_area);
            }
        }

        // TODO FIXME: Needs to be written better, need kb interrupts outside player input phase...
        if let Some(event) = backend.poll_key() {
            if event.state == ElementState::Pressed {
                let scancode_adjust = if cfg!(target_os = "linux") { 8 } else { 0 };
                if let Some(scancode) =
                    Scancode::new((event.scancode as i32 + scancode_adjust) as u8)
                {
                    let ret = match self.state {
                        State::Inventory(_) => self.inventory_input(scancode),
                        State::Console => self.console_input(scancode),
                        State::Aim(AimAction::Zap(slot)) => self.aim_input(slot, scancode),
                        _ => self.game_input(backend, scancode),
                    };

                    if let Ok(events) = ret {
                        // Input event caused a successful world step and we got an event sequence out.
                        // Convert events into UI display effects.
                        for e in events {
                            match e {
                                Event::Msg(text) => {
                                    let _ = writeln!(&mut self.console, "{}", text);
                                }
                                Event::Damage { entity, amount } => {
                                    let name = self.world.entity_name(entity);
                                    // TODO: Use graphical effect
                                    let _ = writeln!(&mut self.console, "{} dmg {}", name, amount);
                                }
                            }
                        }
                    }
                }
            }
        }

        if self.world.player_can_act() {
            self.world.tick_anims();
        } else {
            // When playing turn-based and running the animations between player's inputs, speed
            // things up so that the pace feels snappy.
            const FAST_FORWARD: usize = 3;

            for _ in 0..FAST_FORWARD {
                if self.world.player_can_act() {
                    break;
                }
                // TODO FIXME process events in return value.
                let _ = self.world.next_tick();
            }
        }

        backend.update(&mut self.core)
    }
}

struct SlotData {
    key: char,
    code: Scancode,
    slot: Slot,
    name: &'static str,
}

#[cfg_attr(rustfmt, rustfmt_skip)]
static SLOT_DATA: [SlotData; 34] = [
    SlotData { key: '1', code: Scancode::Num1, slot: Slot::Spell1,     name: "Ability" },
    SlotData { key: '2', code: Scancode::Num2, slot: Slot::Spell2,     name: "Ability" },
    SlotData { key: '3', code: Scancode::Num3, slot: Slot::Spell3,     name: "Ability" },
    SlotData { key: '4', code: Scancode::Num4, slot: Slot::Spell4,     name: "Ability" },
    SlotData { key: '5', code: Scancode::Num5, slot: Slot::Spell5,     name: "Ability" },
    SlotData { key: '6', code: Scancode::Num6, slot: Slot::Spell6,     name: "Ability" },
    SlotData { key: '7', code: Scancode::Num7, slot: Slot::Spell7,     name: "Ability" },
    SlotData { key: '8', code: Scancode::Num8, slot: Slot::Spell8,     name: "Ability" },
    SlotData { key: 'a', code: Scancode::A,    slot: Slot::Melee,      name: "Weapon" },
    SlotData { key: 'b', code: Scancode::B,    slot: Slot::Ranged,     name: "Ranged" },
    SlotData { key: 'c', code: Scancode::C,    slot: Slot::Head,       name: "Head" },
    SlotData { key: 'd', code: Scancode::D,    slot: Slot::Body,       name: "Body" },
    SlotData { key: 'e', code: Scancode::E,    slot: Slot::Feet,       name: "Feet" },
    SlotData { key: 'f', code: Scancode::F,    slot: Slot::TrinketF,   name: "Trinket" },
    SlotData { key: 'g', code: Scancode::G,    slot: Slot::TrinketG,   name: "Trinket" },
    SlotData { key: 'h', code: Scancode::H,    slot: Slot::TrinketH,   name: "Trinket" },
    SlotData { key: 'i', code: Scancode::I,    slot: Slot::TrinketI,   name: "Trinket" },
    SlotData { key: 'j', code: Scancode::J,    slot: Slot::InventoryJ, name: "" },
    SlotData { key: 'k', code: Scancode::K,    slot: Slot::InventoryK, name: "" },
    SlotData { key: 'l', code: Scancode::L,    slot: Slot::InventoryL, name: "" },
    SlotData { key: 'm', code: Scancode::M,    slot: Slot::InventoryM, name: "" },
    SlotData { key: 'n', code: Scancode::N,    slot: Slot::InventoryN, name: "" },
    SlotData { key: 'o', code: Scancode::O,    slot: Slot::InventoryO, name: "" },
    SlotData { key: 'p', code: Scancode::P,    slot: Slot::InventoryP, name: "" },
    SlotData { key: 'q', code: Scancode::Q,    slot: Slot::InventoryQ, name: "" },
    SlotData { key: 'r', code: Scancode::R,    slot: Slot::InventoryR, name: "" },
    SlotData { key: 's', code: Scancode::S,    slot: Slot::InventoryS, name: "" },
    SlotData { key: 't', code: Scancode::T,    slot: Slot::InventoryT, name: "" },
    SlotData { key: 'u', code: Scancode::U,    slot: Slot::InventoryU, name: "" },
    SlotData { key: 'v', code: Scancode::V,    slot: Slot::InventoryV, name: "" },
    SlotData { key: 'w', code: Scancode::W,    slot: Slot::InventoryW, name: "" },
    SlotData { key: 'x', code: Scancode::X,    slot: Slot::InventoryX, name: "" },
    SlotData { key: 'y', code: Scancode::Y,    slot: Slot::InventoryY, name: "" },
    SlotData { key: 'z', code: Scancode::Z,    slot: Slot::InventoryZ, name: "" },
];
