use std::rand::Rng;
use collections::hashmap::HashSet;

use calx::text::Map2DUtil;
use cgmath::aabb::{Aabb, Aabb2};
use cgmath::point::{Point2};
use area::{Area, DIRECTIONS6, Location};
use area;

pub trait MapGen {
    fn gen_cave<R: Rng>(&mut self, rng: &mut R);
    fn gen_prefab(&mut self, prefab: &str);
}

impl MapGen for Area {
    fn gen_cave<R: Rng>(&mut self, rng: &mut R) {
        let center = Location(Point2::new(0i8, 0i8));
        let mut edge = HashSet::new();
        let bounds = Aabb2::new(Point2::new(-16i8, -16i8), Point2::new(16i8, 16i8));
        let mut dug = 1;
        self.dig(center);
        for &v in DIRECTIONS6.iter() {
            edge.insert(center + v);
        }

        for _itercount in range(0, 10000) {
            let pick = *rng.sample(edge.iter(), 1)[0];
            let nfloor = DIRECTIONS6.iter().count(|&v| self.is_open(pick + v));
            assert!(nfloor > 0);

            // Weight digging towards narrow corners.
            if rng.gen_range(0, nfloor) != 0 {
                continue;
            }

            self.dig(pick);
            if rng.gen::<uint>() % 10 == 0 {
                self.set(pick, area::Magma);
            } else if rng.gen::<uint>() % 10 == 0 {
                self.set(pick, area::Water);
            } else if rng.gen::<uint>() % 10 == 0 {
                self.set(pick, area::Grass);
            }
            dug += 1;

            for &v in DIRECTIONS6.iter() {
                let p = pick + v;
                if !self.defined(p) && bounds.contains(p.p()) {
                    edge.insert(p);
                }
            }

            if dug > 384 { break; }
        }
        self.set(*rng.sample(edge.iter(), 1)[0], area::Downstairs);
    }

    fn gen_prefab(&mut self, prefab: &str) {
        for (c, x, y) in prefab.chars().map2d() {
            if c == '.' {
                self.set.insert(Location(Point2::new(x as i8, y as i8)), area::Floor);
            }
            if c == '~' {
                self.set.insert(Location(Point2::new(x as i8, y as i8)), area::Water);
            }
        }

    }
}