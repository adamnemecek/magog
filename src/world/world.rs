use std::collections::hashmap::{HashMap};
use cgmath::vector::{Vector2};
use cgmath::point::{Point2};
use world::terrain::TerrainType;
use world::mobs::{Mob, MobId};

pub struct World {
    seed: u32,
    next_id: u64,
    tick: u64,
    pub depth: int,
    pub area: HashMap<Location, TerrainType>,
    pub mobs: HashMap<MobId, Mob>,
}

impl World {
    pub fn new(seed: u32) -> World {
        World {
            seed: seed,
            next_id: 1,
            tick: 0,
            depth: 0,
            area: HashMap::new(),
            mobs: HashMap::new(),
        }
    }

    pub fn terrain_get(&self, loc: Location) -> Option<TerrainType> {
        self.area.find(&loc).map(|x| *x)
    }

    pub fn terrain_set(&mut self, loc: Location, t: TerrainType) {
        self.area.insert(loc, t);
    }

    pub fn terrain_clear(&mut self, loc: Location) {
        self.area.remove(&loc);
    }

    fn make_id(&mut self) -> u64 {
        let ret = self.next_id;
        self.next_id += 1;
        ret
    }

    pub fn insert_mob(&mut self, mut mob: Mob) -> MobId {
        mob.id = self.make_id();
        self.mobs.insert(mob.id, mob);
        mob.id
    }

    pub fn remove_mob(&mut self, id: MobId) { self.mobs.remove(&id); }

    pub fn rng_seed(&self) -> u32 { self.seed }

    pub fn get_tick(&self) -> u64 { self.tick }

    pub fn advance_frame(&mut self) {
        self.tick += 1;
    }
}


// TODO: Add third dimension for multiple persistent levels.
#[deriving(Eq, PartialEq, Clone, Hash, Show)]
pub struct Location {
    pub x: i8,
    pub y: i8,
}

impl Location {
    pub fn new(x: i8, y: i8) -> Location { Location { x: x, y: y } }
}

impl Add<Vector2<int>, Location> for Location {
    fn add(&self, other: &Vector2<int>) -> Location {
        Location::new(
            (self.x as int + other.x) as i8,
            (self.y as int + other.y) as i8)
    }
}

impl Sub<Location, Vector2<int>> for Location {
    fn sub(&self, other: &Location) -> Vector2<int> {
        Vector2::new((self.x - other.x) as int, (self.y - other.y) as int)
    }
}


pub static DIRECTIONS6: [Vector2<int>, ..6] = [
    Vector2 { x: -1, y: -1 },
    Vector2 { x:  0, y: -1 },
    Vector2 { x:  1, y:  0 },
    Vector2 { x:  1, y:  1 },
    Vector2 { x:  0, y:  1 },
    Vector2 { x: -1, y:  0 },
];

pub static DIRECTIONS8: [Vector2<int>, ..8] = [
    Vector2 { x: -1, y: -1 },
    Vector2 { x:  0, y: -1 },
    Vector2 { x:  1, y: -1 },
    Vector2 { x:  1, y:  0 },
    Vector2 { x:  1, y:  1 },
    Vector2 { x:  0, y:  1 },
    Vector2 { x: -1, y:  1 },
    Vector2 { x: -1, y:  0 },
];


// Positions on a virtual infinite 2D chart, which may map to different actual
// Locations.
#[deriving(Eq, PartialEq, Clone, Hash, Show)]
pub struct ChartPos {
    pub x: int,
    pub y: int,
}

impl<'a> ChartPos {
    pub fn new(x: int, y: int) -> ChartPos { ChartPos { x: x, y: y } }

    pub fn from_location(loc: Location) -> ChartPos {
        ChartPos::new(loc.x as int, loc.y as int)
    }

    pub fn to_location(self) -> Location {
        Location::new(self.x as i8, self.y as i8)
    }

    pub fn to_point(self) -> Point2<int> {
        Point2::new(self.x, self.y)
    }
}

impl Add<Vector2<int>, ChartPos> for ChartPos {
    fn add(&self, other: &Vector2<int>) -> ChartPos {
        ChartPos::new(
            (self.x + other.x),
            (self.y + other.y))
    }
}

//pub struct Chart(HashMap<ChartPos, Location>);
pub type Chart = HashMap<ChartPos, Location>;
