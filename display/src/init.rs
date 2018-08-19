//! Set up resource content for game.

use calx::color::*;
use calx::Rgba;
use crate::brush::{Brush, Builder, Geom};
use std::rc::Rc;
use std::str::FromStr;
use vec_map::VecMap;

#[cfg_attr(rustfmt, rustfmt_skip)]
pub fn terrain_brushes() -> VecMap<Rc<Brush>> {
    use world::Terrain::*;
    let mut ret = VecMap::new();

    ret.insert(Empty as usize, Builder::new("assets/floors.png").tile(0, 0).finish());
    ret.insert(Entrance as usize, Builder::new("assets/portals.png")
        .color(LIGHTCYAN)
        .tile(0, 0).merge()
        .tile(32, 0).merge()
        .tile(64, 0).merge()
        .tile(96, 0).merge()
        .tile(128, 0).merge()
        .tile(160, 0).merge()
        .tile(192, 0).merge()
        .tile(224, 0).merge()
        .tile(256, 0).merge()
        .tile(288, 0).merge()
        .tile(320, 0).merge()
        .tile(352, 0).merge()
        .tile(384, 0).finish());
    ret.insert(Exit as usize, Builder::new("assets/portals.png")
        .color(LIGHTCYAN)
        .tile(0, 0).merge()
        .tile(32, 0).merge()
        .tile(64, 0).merge()
        .tile(96, 0).merge()
        .tile(128, 0).merge()
        .tile(160, 0).merge()
        .tile(192, 0).merge()
        .tile(224, 0).merge()
        .tile(256, 0).merge()
        .tile(288, 0).merge()
        .tile(320, 0).merge()
        .tile(352, 0).merge()
        .tile(384, 0).finish());
    ret.insert(Ground as usize, Builder::new("assets/floors.png").color(SLATEGRAY).tile(32, 0).finish());
    ret.insert(Grass as usize, Builder::new("assets/floors.png").color(DARKGREEN).tile(32, 0).finish());
    ret.insert(Snow as usize, Builder::new("assets/floors.png").color(WHITE).tile(32, 0).finish());
    ret.insert(Sand as usize, Builder::new("assets/floors.png").color(YELLOW).tile(32, 0).finish());
    ret.insert(Water as usize, Builder::new("assets/floors.png").colors(MIDNIGHTBLUE, ROYALBLUE).tile(96, 0).finish());
    ret.insert(Shallows as usize, Builder::new("assets/floors.png").colors(STEELBLUE, ROYALBLUE).tile(96, 0).finish());
    ret.insert(Magma as usize, Builder::new("assets/floors.png").colors(YELLOW, DARKRED).tile(96, 0).finish());
    ret.insert(Tree as usize, Builder::new("assets/props.png")
        .color(SADDLEBROWN).tile(160, 64)
        .color(GREEN).tile(192, 64).finish());
    ret.insert(Wall as usize, Builder::new("assets/walls.png").color(LIGHTSLATEGRAY).wall(0, 0, 32, 0).finish());
    ret.insert(Rock as usize, Builder::new("assets/blobs.png").color(DARKGOLDENROD).blob(0, 0, 0, 32, 0, 160).finish());
    ret.insert(Door as usize, Builder::new("assets/walls.png")
        .color(SADDLEBROWN).wall(128, 0, 160, 0)
        .color(LIGHTSLATEGRAY).wall(0, 0, 96, 0).finish());
    ret.insert(OpenDoor as usize, Builder::new("assets/walls.png").color(LIGHTSLATEGRAY).wall(0, 0, 96, 0).finish());
    ret.insert(Window as usize, Builder::new("assets/walls.png").color(LIGHTSLATEGRAY).wall(0, 0, 64, 0).finish());
    ret.insert(Pillar as usize, Builder::new("assets/props.png").color(GAINSBORO).tile(0, 32).finish());
    ret.insert(Grass2 as usize, Builder::new("assets/floors.png").color(DARKGREEN).tile(64, 0).finish());

    ret
}

#[cfg_attr(rustfmt, rustfmt_skip)]
pub fn entity_brushes() -> VecMap<Rc<Brush>> {
    use world::Icon::*;
    let mut ret = VecMap::new();

    ret.insert(Player as usize, Builder::new("assets/mobs.png").color(AZURE).mob(0, 0).finish());
    ret.insert(Snake as usize, Builder::new("assets/mobs.png").color(GREEN).mob(1*32, 0).finish());
    ret.insert(Dreg as usize, Builder::new("assets/mobs.png").color(OLIVE).mob(2*32, 0).finish());
    ret.insert(Ogre as usize, Builder::new("assets/mobs.png").color(DARKCYAN).mob(3*32, 0).finish());
    ret.insert(Wraith as usize, Builder::new("assets/mobs.png").color(GRAY).mob(4*32, 0).finish());
    ret.insert(Octopus as usize, Builder::new("assets/mobs.png").color(WHEAT).mob(5*32, 0).finish());
    ret.insert(Bug as usize, Builder::new("assets/mobs.png").color(0xFF00FFFF).mob(6*32, 0).finish());
    ret.insert(Ooze as usize, Builder::new("assets/mobs.png").color(LIGHTSKYBLUE).mob(7*32, 0).finish());
    ret.insert(Efreet as usize, Builder::new("assets/mobs.png").color(ORANGE).mob(0, 1*32).finish());

    // The serpent has a special sprite structure where it's split to the head and mound parts,
    // and the mound part doesn't move during the idle animation.
    ret.insert(Serpent as usize, Builder::new("assets/mobs.png").color(CORAL)
        .splat(Some(Geom::new(16, 14, 1*32, 32, 32, 32))).tile(2*32, 32).merge()
        .tile(1*32, 32).tile(2*32, 32).merge()
        .finish());

    ret.insert(Sword as usize, Builder::new("assets/props.png").color(LIGHTGRAY).tile(4*32, 1*32).finish());
    ret.insert(Helmet as usize, Builder::new("assets/props.png").color(LIGHTGRAY).tile(5*32, 1*32).finish());
    ret.insert(Armor as usize, Builder::new("assets/props.png").color(LIGHTGRAY).tile(2*32, 2*32).finish());

    ret.insert(Scroll1 as usize, Builder::new("assets/props.png").color(LIGHTYELLOW).tile(7*32, 2*32).finish());
    ret.insert(Wand1 as usize, Builder::new("assets/props.png").color(RED).tile(7*32, 1*32).finish());
    ret.insert(Wand2 as usize, Builder::new("assets/props.png").color(CYAN).tile(7*32, 1*32).finish());
    ret
}

#[cfg_attr(rustfmt, rustfmt_skip)]
pub fn misc_brushes() -> VecMap<Rc<Brush>> {
    use crate::Icon::*;
    let mut ret = VecMap::new();

    ret.insert(SolidBlob as usize, Builder::new("assets/blobs.png").colors(BLACK, BLACK).blob(0, 64, 0, 96, 0, 128).finish());
    ret.insert(CursorTop as usize, Builder::new("assets/props.png").color(RED).tile(32, 0).finish());
    ret.insert(CursorBottom as usize, Builder::new("assets/props.png").color(RED).tile(0, 0).finish());
    ret.insert(Portal as usize, Builder::new("assets/props.png").color(Rgba::from_str("#fa08").unwrap()).tile(0, 0).finish());
    ret.insert(HealthPip as usize, Builder::new("assets/gui.png").color(LIMEGREEN).rect(0, 8, 4, 4).finish());
    ret.insert(DarkHealthPip as usize, Builder::new("assets/gui.png").color(DARKSLATEGRAY).rect(0, 8, 4, 4).finish());
    ret.insert(BlockedOffSectorCell as usize, Builder::new("assets/floors.png").color(LIGHTGRAY).tile(0, 32).finish());

    ret
}
