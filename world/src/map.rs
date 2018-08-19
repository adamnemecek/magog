use calx::{self, CellVector, DenseTextMap, Dir6, HexGeom, IntoPrefab, RngExt};
use crate::mapsave::{self, build_textmap, MapSave};
use crate::spec::EntitySpawn;
use crate::terrain::Terrain;
use euclid::vec2;
use log::Level::Trace;
use rand::{seq, Rng};
use std::collections::{hash_map, HashMap, HashSet};
use std::error::Error;
use std::fmt;
use std::ops::Index;
use std::str::FromStr;

// NOTE ON STABLE ORDER
//
// HashSet has a nondeterministic iteration order, but it's very important that `Map` methods
// introduce no randomness other than what comes in explicitly via the `Rng` parameter. All public
// methods that operate on a HashSet and return some list or selection of values from it must
// perform an extra step to sort the data in some deterministic order.
//
// The other approach would be to use a deterministic container like `BTreeMap`, but CellVectors do
// not implement `Ord` so they can't be used as keys for it.

/// Representation of a game level during procedural map generation.
#[derive(Clone, Debug)]
pub struct Map {
    contents: HashMap<CellVector, MapCell>,
}

impl<'a> From<&'a Map> for mapsave::Prefab {
    fn from(map: &'a Map) -> mapsave::Prefab {
        map.contents
            .iter()
            .filter_map(|(pos, c)| {
                if c.terrain != Terrain::Empty {
                    Some((*pos, (c.terrain, c.spawns.clone())))
                } else {
                    None
                }
            }).collect()
    }
}

impl Map {
    pub fn iter(&self) -> impl Iterator<Item = (&CellVector, &MapCell)> { self.contents.iter() }

    pub fn insert(&mut self, pos: CellVector, value: MapCell) { self.contents.insert(pos, value); }

    pub fn push_spawn(&mut self, pos: CellVector, spawn: EntitySpawn) {
        debug_assert!(self.contains(pos));
        self.contents.get_mut(&pos).map(|c| c.spawns.push(spawn));
    }

    pub fn get(&self, pos: CellVector) -> Option<&MapCell> { self.contents.get(&pos) }

    /// Build an empty map.
    pub fn new() -> Map {
        Map {
            contents: HashMap::new(),
        }
    }

    /// Build a map with a shaped base of default tiles.
    pub fn new_base(points: impl IntoIterator<Item = CellVector>) -> Map {
        let mut ret = Map::new();

        for p in points.into_iter() {
            ret.insert(p, MapCell::default());
        }

        ret
    }

    /// Build a prefab vault map from ASCII map.
    pub fn new_vault(textmap: &str) -> Result<Self, Box<Error>> {
        let prefab: HashMap<CellVector, char> = DenseTextMap(textmap).into_prefab()?;
        let mut ret = Map::new();

        for (&pos, c) in &prefab {
            use crate::Terrain::*;
            let is_border_pos = !calx::hex_neighbors(pos).all(|p| prefab.contains_key(&p));
            let mut cell = MapCell::default();
            // The only cells that get marked as Border are wall tiles or similar shaped blocks.
            // Regular ground style terrain at the edge of the prefab is still counts as Interior.
            cell.vault_kind = Some(VaultKind::Interior);

            match c {
                ' ' => {
                    continue;
                }
                '_' => {
                    // This is a dummy tile that expands the bounds of the vault.
                    // They're put in vault maps to prevent a narrow vault with a single exit from
                    // being spawned in a place where the exit is blocked.
                    cell = MapCell::new_bumper();
                }
                '%' => {
                    cell.can_dig = false;
                    // Designate undiggable edge of the default blocking terrain
                }
                '#' => {
                    cell.terrain = Wall;
                    if is_border_pos {
                        cell.can_dig = false;
                        cell.vault_kind = Some(VaultKind::Border);
                    }
                }
                '.' => {
                    cell.terrain = Ground;
                }
                '>' => {
                    cell.terrain = Exit;
                }
                '<' => {
                    cell.terrain = Entrance;
                }
                '~' => {
                    cell.terrain = Water;
                }
                'I' => {
                    cell.terrain = Pillar;
                }
                '+' => {
                    // Door.
                    if is_border_pos {
                        // If the door is on the edge of the map, it is only a potential entryway.
                        // The default terrain is wall, but the position can be dug.
                        cell.terrain = Wall;
                        cell.vault_kind = Some(VaultKind::Border);
                    } else {
                        // On the other hand, if the door cell is completely surrounded by defined
                        // terrain, it's a vault interior tile,  so it gets an actual door terrain
                        // right away.
                        cell.terrain = Door;
                    }
                }

                'a' => {
                    cell.terrain = Ground;
                    // TODO: Put generic "monster" spawns in factory, don't want all vaults to have
                    // fixed types.
                    cell.spawns.push(EntitySpawn::from_str("dreg").unwrap());
                }

                c => {
                    die!("Unknown map glyph '{}'", c);
                }
            }

            ret.insert(pos, cell);
        }

        Ok(ret)
    }

    /// Build a random rectangular room.
    pub fn new_plain_room(rng: &mut (impl Rng + ?Sized)) -> Map {
        let (w, h) = (rng.gen_range(2, 8), rng.gen_range(2, 8));

        let mut ret = Map::new();

        for y in -1..h + 1 {
            for x in -1..w + 1 {
                let x_wall = x == -1 || x == w;
                let y_wall = y == -1 || y == h;
                let p = vec2(x, y);

                if x_wall || y_wall {
                    if !(x_wall && y_wall) {
                        ret.insert(p, MapCell::new_terrain(Terrain::Wall).border());
                    } else {
                        ret.insert(p, MapCell::new_terrain(Terrain::Wall).undiggable().border());
                    }
                } else {
                    ret.insert(p, MapCell::new_terrain(Terrain::Ground).interior());
                }
            }
        }

        ret
    }

    /// Return whether position is in defined area of this map.
    pub fn contains(&self, pos: CellVector) -> bool { self.contents.contains_key(&pos) }

    /// Return positions from the map that satisfy the given predicate.
    ///
    /// The result is guaranteed to be in stable order.
    pub fn find_positions(&self, p: impl Fn(CellVector, &MapCell) -> bool) -> Vec<CellVector> {
        let mut ret: Vec<CellVector> = self
            .iter()
            .filter_map(|(&pos, c)| if p(pos, c) { Some(pos) } else { None })
            .collect();
        ret.sort_by_key(|v| (v.x, v.y));
        ret
    }

    /// Return possible positions for placing a room on this map.
    ///
    /// The result is guaranteed to be in stable order.
    pub fn room_positions(&self, room: &Map) -> Vec<CellVector> {
        self.find_positions(|p, _| self.is_valid_placement(p, room))
    }

    pub fn entrances(&self) -> Vec<CellVector> {
        self.find_positions(|_, c| c.terrain == Terrain::Entrance)
    }

    pub fn exits(&self) -> Vec<CellVector> {
        self.find_positions(|_, c| c.terrain == Terrain::Exit)
    }

    pub fn open_ground(&self) -> Vec<CellVector> {
        self.find_positions(|_, c| c.is_walkable() && !c.is_border())
    }

    /// Return whether a room can be placed on this Map in the given position.
    pub fn is_valid_placement(&self, offset: CellVector, room: &Map) -> bool {
        for (&p, c) in room {
            let pos = offset + p;

            if !self.contains(pos) {
                if c.is_walkable() || c.is_bumper() {
                    // Trying to place walkable terrain or a Terrain::Empty boundary shim outside
                    // domain, no-go.
                    return false;
                } else {
                    // Otherwise it's a wall or something, go nuts and skip the remaining section
                    // that compares it against the existing terrain.
                    continue;
                }
            }

            let existing = &self[pos];

            if existing.is_interior() {
                // Never clobber existing interior.
                return false;
            }

            if c.is_interior() && !existing.can_dig {
                // Interior can't go on no-dig area.
                return false;
            }

            if c.is_border() {
                // Border cell, check that it doesn't land adjacent to existing border.
                if existing.is_border() {
                    // It can be fused with an existing border, in which case we can skip the
                    // adjacency check.
                    if existing.terrain != c.terrain {
                        // Can't fuse borders if they have different terrain.
                        return false;
                    }
                } else {
                    // Not landing on border, check time.
                    // Assume that these are fake-isometric walls and only check adjacency along
                    // the fake-isometric axes.
                    for p in calx::taxicab_neighbors(p) {
                        let pos = offset + p;
                        if !room.contains(p) && self.get(pos).map_or(false, |c| c.is_border()) {
                            // Bad touch
                            return false;
                        }
                    }
                }

                if existing.is_walkable() && c.can_dig {
                    // The border is going to get a door here. Check that there won't be any
                    // adjacent doors.
                    for p in calx::hex_neighbors(p) {
                        if room.get(p).map_or(false, |c| c.is_border() && c.can_dig)
                            && self.get(offset + p).map_or(false, |c| c.is_walkable())
                        {
                            // Two doors in a row, no-no.
                            return false;
                        }
                    }
                }
            }
        }

        true
    }

    /// Place a room on the map in the given position
    pub fn place_room_at(&mut self, offset: CellVector, room: &Map) {
        debug_assert!(self.is_valid_placement(offset, room));

        for (&p, c) in room {
            let pos = p + offset;
            let mut c = c.clone();

            if c.is_bumper() {
                continue;
            }

            if self.contains(pos) {
                let existing = &self[pos];

                // Putting the border cell on top of a dug tunnel, create a door.
                if existing.is_walkable() && c.is_border() && c.can_dig {
                    c = c.to_door();
                }

                // Undiggability propagates to new cells, otherwise clobber the old cell.
                if !existing.can_dig {
                    c.can_dig = false;
                }
            }

            self.insert(pos, c);
        }
    }

    /// Helper function to randomly place a room
    pub fn place_room(
        &mut self,
        rng: &mut (impl Rng + ?Sized),
        room: &Map,
    ) -> Result<(), Box<Error>> {
        let sites = self.room_positions(room);
        if sites.is_empty() {
            die!("No room left");
        }
        self.place_room_at(rng.pick_slice(&sites).unwrap(), room);
        Ok(())
    }

    /// Return whether a tunnel can be dug in `pos + dir` from `pos`.
    ///
    /// Will return true if the cell is traversable but should not be dug, ie. if it's a vault
    /// interior cell or an already dug tunnel.
    pub fn can_tunnel(&self, pos: CellVector, dir: Dir6) -> bool {
        let target_pos = pos + dir.into();
        if self.get(target_pos).map_or(false, |c| c.is_walkable()) {
            // Moving through open space, all is good.
            return true;
        }

        if self.get(target_pos).map_or(true, |c| {
            !c.can_dig || c.vault_kind == Some(VaultKind::Interior)
        }) {
            trace!("Cannot tunnel into {:?}", target_pos);
            // Target cell is undiggable wall, untraversable vault interior or outside the map.
            return false;
        }

        // Use fake-isometric logic for side-walls when digging along the fake isometric axes, and
        // a more strict hex-based logic when digging along the third axis.
        for &p in &match dir {
            Dir6::North => vec![vec2(-1, 0), vec2(0, -1), vec2(-2, -1), vec2(-1, -2)],
            Dir6::Northeast => vec![vec2(-1, -1), vec2(1, -1)],
            Dir6::Southeast => vec![vec2(1, -1), vec2(1, 1)],
            Dir6::South => vec![vec2(1, 0), vec2(0, 1), vec2(2, 1), vec2(1, 2)],
            Dir6::Southwest => vec![vec2(-1, 1), vec2(1, 1)],
            Dir6::Northwest => vec![vec2(-1, -1), vec2(-1, 1)],
        } {
            let p = pos + p;
            if self.get(p).map_or(false, |c| c.is_walkable()) {
                // Breaching a wall on the side. No go.
                return false;
            }
        }

        if self.get(target_pos).map_or(false, |c| c.is_border()) {
            // Trying to breach a border moving along a non-isometric axis. This causes bad visuals, so
            // it's forbidden.
            if !dir.is_fake_isometric() {
                return false;
            }

            // Can't make a new door if there's already an existing door next to it.
            for p in calx::hex_neighbors(target_pos) {
                if self
                    .get(p)
                    .map_or(false, |c| c.is_border() && c.is_walkable())
                {
                    return false;
                }
            }
        }

        if self.get(pos).map_or(false, |c| c.is_border()) && !dir.is_fake_isometric() {
            // Also you can't move out of a border except in a fake-isometric direction, so if
            // you're moving out from a vault and starting a tunnel, you need to make a clean space
            // in front of the door.
            return false;
        }

        true
    }

    /// Dig a cell of tunnel in a given position.
    ///
    /// Do nothing when going through a vault interior, the premade vault map should take care of
    /// connectivity there, but the tunnel can still path through it.
    fn dig(&mut self, pos: CellVector) {
        if !self.contains(pos) {
            return;
        }

        let existing = self[pos].clone();

        debug_assert!(existing.can_dig);

        if existing.is_walkable() || existing.is_interior() {
            return;
        } else if existing.is_border() {
            self.insert(pos, existing.to_door());
        } else {
            self.insert(pos, MapCell::new_terrain(Terrain::Ground));
        }
    }

    /// Join disconnected regions on map with tunnels.
    pub fn join_disjoint_regions(&mut self, rng: &mut (impl Rng + ?Sized)) -> Option<Map> {
        let mut ret = self.clone();
        // Keep looping until all disjoint regions are joined.
        loop {
            let floors: HashSet<CellVector> = ret
                .contents
                .iter()
                .filter_map(|(&p, c)| if c.is_walkable() { Some(p) } else { None })
                .collect();

            // Remove vault interior bubbles from consideration, they can't be connected.
            let regions: Vec<Vec<CellVector>> = separate_regions(floors)
                .into_iter()
                .filter(|p| !ret.is_interior_bubble(p))
                .collect();

            if regions.len() < 2 {
                // All in order.
                break;
            }

            // Connect first into second.
            let p1 = *seq::sample_iter(rng, &regions[0], 1).unwrap()[0];
            let p2 = *seq::sample_iter(rng, &regions[1], 1).unwrap()[0];

            if log_enabled!(Trace) {
                trace!("Merging disjoint map:\n{}", ret);
            }

            if let Some(connected) = ret.find_tunnel(p1, p2) {
                ret = connected;
            } else {
                return None;
            }
        }
        Some(ret)
    }

    /// Return if the set of points forms a "vault interior bubble".
    ///
    /// The set is assumed to be connected. An interior bubble consists entirely of cells inside a
    /// vault and is also entirely surrounded by cells inside a vault (interior or border). This
    /// method is used to recognize and discard sealed decorative chambers that may be present in
    /// prefab vaults but should not factor into map connectivity analysis.
    fn is_interior_bubble<'a>(&self, points: impl IntoIterator<Item = &'a CellVector>) -> bool {
        points.into_iter().all(|&p| {
            !self.is_vault_exterior(p) && calx::hex_neighbors(p).all(|p_edge| {
                !self.is_vault_exterior(p_edge) && self.get(p_edge).map_or(true, |c| !c.can_dig)
            })
        })
    }

    /// Return if the cell is an on-map vault-exterior cell.
    fn is_vault_exterior(&self, pos: CellVector) -> bool {
        self.get(pos).map_or(false, |c| c.vault_kind.is_none())
    }

    /// Try to find a tunnel from `p1` to `p2`.
    ///
    /// Return a clone of the map with the tunnel drawn if a tunnel can be
    /// found.
    fn find_tunnel(&self, p1: CellVector, p2: CellVector) -> Option<Map> {
        // Do A* search to find the path. But since digging part of a tunnel changes
        // whether the rest can be dug, we'll be using entire map snapshots as
        // the search states.

        // NB: Be careful with maintaining stability here when choosing from equally
        // good nodes in the open set.
        let mut seed = self.clone();
        seed.dig(p1);
        let mut open = HashMap::new();
        let mut closed = HashSet::new();
        open.insert(p1, seed);

        while !open.is_empty() {
            let mut points: Vec<CellVector> = open.keys().cloned().collect();
            // Sort them in regular order to remove instability from iterating HashMap.
            points.sort_by_key(|v| (v.x, v.y));
            // Now do the A* sort, pick the one that's closest to the target.
            points.sort_by_key(|&v| (v - p2).hex_dist());
            let p = points[0];

            let map = open[&p].clone();
            if log_enabled!(Trace) {
                if let Ok((mut prefab, legend)) = build_textmap(&mapsave::Prefab::from(&map)) {
                    for (&p, c) in &map.contents {
                        if !c.can_dig {
                            prefab.insert(p, 'x');
                        }
                    }
                    for &o in open.keys() {
                        prefab.insert(o, '?');
                    }
                    prefab.insert(p, '!');
                    let mapsave = MapSave::new(prefab, legend);
                    trace!("Searching tunnel:\n{}", mapsave);
                } else {
                    trace!("*** COULDN'T CONSTRUCT TEXTMAP ***");
                }
            }
            for &d in Dir6::iter() {
                let q = p + d.into();
                if closed.contains(&q) {
                    continue;
                }

                if map.can_tunnel(p, d) {
                    let mut new_map = map.clone();
                    new_map.dig(q);
                    if q == p2 {
                        return Some(new_map);
                    }
                    open.insert(q, new_map);
                }
            }

            closed.insert(p);
            open.remove(&p);
        }

        None
    }
}

impl fmt::Display for Map {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let save = MapSave::from_prefab(&mapsave::Prefab::from(self))
            .expect("Couldn't convert Map to displayable");
        writeln!(f, "{}", save)
    }
}

impl<'a> IntoIterator for &'a Map {
    type Item = (&'a CellVector, &'a MapCell);
    type IntoIter = hash_map::Iter<'a, CellVector, MapCell>;

    fn into_iter(self) -> Self::IntoIter { self.contents.iter() }
}

impl Index<CellVector> for Map {
    type Output = MapCell;

    fn index(&self, key: CellVector) -> &MapCell { self.get(key).expect("no entry found for key") }
}

/// Convert a point cloud into subsets of connected points.
///
/// Result is in stable order.
fn separate_regions(mut points: HashSet<CellVector>) -> Vec<Vec<CellVector>> {
    // Have to be extra nasty to get this whole thing kept in stable order, convert vecs to
    // sortable tuples, sort, convert back.
    let mut sets: Vec<Vec<(i32, i32)>> = Vec::new();

    while !points.is_empty() {
        let seed = *points.iter().next().unwrap();
        let subset = flood_fill(points.clone(), seed);
        debug_assert!(!subset.is_empty());
        points = points.difference(&subset).cloned().collect();
        let mut subset: Vec<(i32, i32)> = subset.into_iter().map(|p| (p.x, p.y)).collect();
        subset.sort();
        sets.push(subset);
    }

    sets.sort();

    sets.into_iter()
        .map(|a| a.into_iter().map(|(x, y)| vec2(x, y)).collect())
        .collect()
}

fn flood_fill(mut points: HashSet<CellVector>, seed: CellVector) -> HashSet<CellVector> {
    let mut ret = HashSet::new();
    let mut edge = HashSet::new();

    if !points.remove(&seed) {
        return ret;
    }
    edge.insert(seed);

    while !edge.is_empty() {
        // XXX: Is there a set data type that supports 'pop'?
        let pos = *edge.iter().next().unwrap();
        edge.remove(&pos);
        ret.insert(pos);

        for p in calx::hex_neighbors(pos) {
            let p = p.into();
            if points.contains(&p) {
                debug_assert!(!ret.contains(&p) && !edge.contains(&p));
                points.remove(&p);
                edge.insert(p);
            }
        }
    }

    ret
}

/// Cell type for static map data.
///
/// Map cells represent terrain at map generation stage, they're not connected to live game state
/// and are data-driven.
#[derive(Clone, Debug)]
pub struct MapCell {
    pub terrain: Terrain,
    pub spawns: Vec<EntitySpawn>,
    pub can_dig: bool,
    pub vault_kind: Option<VaultKind>,
}

impl Default for MapCell {
    fn default() -> Self {
        MapCell {
            terrain: Terrain::Empty,
            spawns: Vec::new(),
            can_dig: true,
            vault_kind: None,
        }
    }
}

impl MapCell {
    pub fn new_terrain(t: Terrain) -> MapCell {
        let mut ret = Self::default();
        ret.terrain = t;
        ret
    }

    pub fn new_bumper() -> MapCell { MapCell::default() }

    pub fn is_walkable(&self) -> bool { !self.terrain.blocks_walk() }

    pub fn is_border(&self) -> bool { self.vault_kind == Some(VaultKind::Border) }

    pub fn is_interior(&self) -> bool { self.vault_kind == Some(VaultKind::Interior) }

    /// This is a fake cell that doesn't describe actual terrain but limits the positioning of a
    /// vault to ensure that you can connect to its entrance.
    pub fn is_bumper(&self) -> bool { return self.terrain == Terrain::Empty && self.can_dig; }

    pub fn to_door(mut self) -> MapCell {
        self.terrain = Terrain::Door;
        self
    }

    pub fn undiggable(mut self) -> MapCell {
        self.can_dig = false;
        self
    }

    pub fn border(mut self) -> MapCell {
        self.vault_kind = Some(VaultKind::Border);
        self
    }

    pub fn interior(mut self) -> MapCell {
        self.vault_kind = Some(VaultKind::Interior);
        self
    }
}

/// Classify map cells based on where they are in a vault.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum VaultKind {
    /// Cell is the interior of a vault.
    ///
    /// You can traverse these from vault edge to vault edge but should never place new cells on
    /// top of them.
    Interior,

    /// Cell is a wall tile on the border of a vault.
    ///
    /// If it's diggable, you can put a door on it. When placing new vaults, you should not have
    /// the border tiles from two vaults adjacent but not overlapping.
    Border,
}
