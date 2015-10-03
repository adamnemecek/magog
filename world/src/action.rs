/*! Functions for changing world and entity state. */

use std::io::prelude::*;
use std::path::{Path};
use std::fs::{self, File};
use rand::StdRng;
use rand::SeedableRng;
use std::iter::Filter;
use calx::{Dijkstra, Dir6, HexFov};
use calx_ecs::{Entity};
use world::{World};
use flags;
use area::{self, Area};
use location::{Location};
use content::{Biome, AreaSpec};
use item::{Slot};
use components::{CompositeStats};
use ::{Msg};
use msg;
use query::{self, ControlState};

/// Player input action.
#[derive(Copy, Eq, PartialEq, Clone, Debug, RustcEncodable, RustcDecodable)]
pub enum Input {
    /// Take a step in the given direction.
    Step(Dir6),
    /// Melee attack in the given direction.
    Melee(Dir6),
    /// Shoot in the given direction.
    Shoot(Dir6),
    /// Do nothing for a turn.
    Pass,
}


/// Top-level game state update function. Only valid to call if
/// control_state() returned ReadyToUpdate.
pub fn update(w: &mut World) {
    assert!(query::control_state(w) == ControlState::ReadyToUpdate);

    ai_main(w);

    w.flags.tick += 1;
    w.flags.player_acted = false;
}

/// Run AI for all autonomous mobs.
fn ai_main(w: &mut World) {
    let actives: Vec<Entity> = w.ecs.brain.iter().map(|(&e, _)| e).collect();
    for e in actives.into_iter() { update_entity(w, e); }
}

pub fn update_entity(w: &mut World, e: Entity) {
    if query::is_mob(w, e) && !query::is_player(w, e) && query::ticks_this_frame(w, e) {
        mob_ai(w, e);
    }
}

pub fn mob_ai(w: &mut World, e: Entity) {
    unimplemented!();
    /*
        assert!(self.is_mob());
        assert!(!self.is_player());
        assert!(self.ticks_this_frame());

        if self.brain_state() == Some(BrainState::Asleep) {
            if let Some(p) = action::player() {
                // TODO: Line-of-sight, stealth concerns, other enemies than
                // player etc.
                if let Some(d) = p.distance_from(self) {
                    if d < 6 {
                        self.wake_up();
                    }
                }
            }

            return;
        }

        if let Some(p) = action::player() {
            let loc = self.location().expect("no location");

            let vec_to_enemy = loc.v2_at(p.location().expect("no location"));
            if let Some(v) = vec_to_enemy {
                if v.hex_dist() == 1 {
                    // Melee range, hit.
                    self.melee(Dir6::from_v2(v));
                } else {
                    // Walk towards.
                    let pathing_depth = 16;
                    let pathing = Dijkstra::new(
                        vec![p.location().expect("no location")], |&loc| !loc.blocks_walk(),
                        pathing_depth);

                    let steps = pathing.sorted_neighbors(&loc);
                    if steps.len() > 0 {
                        self.step(loc.dir6_towards(steps[0]).expect("No loc pair orientation"));
                    } else {
                        self.step(rng().gen());
                        // TODO: Fall asleep if things get boring.
                    }
                }
            }
        }
        */
}

/// Give player input. Only valid to call if control_state() returned
/// AwaitingInput.
pub fn input(w: &mut World, input: Input) {
    assert!(query::control_state(w) == ControlState::AwaitingInput);
    let p = query::player(w).expect("No player to receive input");
    match input {
        Input::Step(d) => {
            step(w, p, d);
        }
        Input::Melee(d) => {
            melee(w, p, d);
        }
        Input::Shoot(d) => {
            shoot(w, p, d);
        }
        Input::Pass => {
        }
    }
    w.flags.player_acted = true;

    // Run one world update cycle right away, so that we don't get awkward
    // single frames rendered where the player has acted and the rest of the
    // world hasn't.
    if query::control_state(w) == ControlState::ReadyToUpdate {
        update(w);
    }
}

/// Try to move the entity in direction.
pub fn step(w: &mut World, e: Entity, dir: Dir6) {
    let target_loc = query::location(w, e).unwrap() + dir.to_v2();
    if query::can_enter(w, e, target_loc) {
        place_entity(w, e, target_loc);
    }
}

/// Fight in a direction.
pub fn melee(w: &mut World, e: Entity, dir: Dir6) {
    unimplemented!();
    /*
    let loc = self.location().expect("no location") + dir.to_v2();
    if let Some(e) = loc.mob_at() {
        let us = self.stats();
        e.damage(us.power + us.attack);
    }
    */
}

pub fn shoot(w: &mut World, e: Entity, dir: Dir6) {
    unimplemented!();
    /*
    let stats = self.stats();

    if stats.ranged_range > 0 {
        action::shoot(self.location().unwrap(), dir, stats.ranged_range, stats.ranged_power);
    }
    */
}

pub fn pick_up(w: &mut World, picker: Entity, item: Entity) -> bool {
    if !query::can_be_picked_up(w, item) {
        return false;
    }

    match query::free_bag_slot(w, picker) {
        Some(slot) => {
            equip(w, item, picker, slot);
            return true;
        }
        // Inventory full.
        None => { return false; }
    }
}

/// Equip an item to a slot. Slot must be empty.
pub fn equip(w: &mut World, item: Entity, e: Entity, slot: Slot) {
    w.spatial.equip(item, e, slot);
    recompose_stats(w, e)
}

/// Generate composed stats from base stats and the stats of equipped items.
/// This function must be called after any operation that changes the composed
/// stats affecting state of an entity.
pub fn recompose_stats(w: &mut World, e: Entity) {
    let mut stats = query::base_stats(w, e);
    for &slot in [
        Slot::Body,
        Slot::Feet,
        Slot::Head,
        Slot::Melee,
        Slot::Ranged,
        Slot::TrinketF,
        Slot::TrinketG,
        Slot::TrinketH,
        Slot::TrinketI].iter() {
        if let Some(item) = w.spatial.entity_equipped(e, slot) {
            stats = stats + query::stats(w, item);
        }
    }

    w.ecs.composite_stats.insert(e, CompositeStats(stats));
}

pub fn place_entity(w: &mut World, e: Entity, loc: Location) {
    w.spatial.insert_at(e, loc);
    after_entity_move(w, e);
}

/// Clear map memory of an entity.
pub fn forget_map(w: &mut World, e: Entity) {
    w.ecs.map_memory.get_mut(e).map(
        |mm| {
            mm.seen.clear();
            mm.remembered.clear();
        });
}

/// Callback when entity moves to a new location.
fn after_entity_move(w: &mut World, e: Entity) {
    let loc = query::location(w, e).expect("Entity must have location for callback");

    do_fov(w, e);

    for item in w.spatial.entities_at(loc).into_iter() {
        if item != e {
            on_step_on(w, e, item);
        }
    }

    if query::is_player(w, e) {
        w.flags.camera = loc;

        if query::terrain(w, loc).is_exit() {
            area::next_level(w);
        }
    }
}

pub fn do_fov(w: &mut World, e: Entity) {
    if !w.ecs.map_memory.contains(e) { return; }

    if let Some(loc) = query::location(w, e) {
        let sight_range = 12;
        let seen_locs: Vec<Location> = HexFov::new(
            |pt| query::blocks_sight(w, (loc + pt)), sight_range)
            .fake_isometric()
            .map(|pt| loc + pt)
            .collect();
        let mut mm = &mut w.ecs.map_memory[e];
        mm.seen.clear();
        mm.seen.extend(seen_locs.clone().into_iter());
        mm.remembered.extend(seen_locs.into_iter());
    }
}

pub fn on_step_on(w: &mut World, stepper: Entity, item: Entity) {
    if query::is_instant_item(w, item) && query::is_player(w, stepper) {
        unimplemented!();
    }
}


////////////////////////////////////////////////////////////////////////

static SAVE_FILENAME: &'static str = "magog_save.json";

pub fn save_game(w: &World) {
    // Only save if there's still a living player around.
    if query::game_over(w) {
        return;
    }

    let save_data = w.save();
    File::create(SAVE_FILENAME).unwrap()
        .write_all(&save_data.into_bytes()).unwrap();
}

pub fn load_game() -> Result<World, ()> {
    if !save_exists() { return Err(()); }
    let path = Path::new(SAVE_FILENAME);
    let mut save_data = String::new();
    File::open(&path).unwrap().read_to_string(&mut save_data).unwrap();
    // TODO: Informative error message if load fails.
    match World::load(&save_data[..]) {
        Ok(w) => Ok(w),
        _ => Err(())
    }
}

pub fn _delete_save() {
    let _ = fs::remove_file(SAVE_FILENAME);
}

pub fn save_exists() -> bool { fs::metadata(SAVE_FILENAME).is_ok() }

