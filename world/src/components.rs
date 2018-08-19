use calx_ecs::Entity;
use crate::item::ItemType;
use crate::location::Location;
use crate::location_set::LocationSet;
use crate::stats::Stats;
use crate::FovStatus;
use std::collections::BTreeMap;

/// The visual representation for an entity
///
/// How this is interpreted depends on the frontend module.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Serialize, Deserialize)]
pub enum Icon {
    Player,
    Snake,
    Dreg,
    Ogre,
    Wraith,
    Octopus,
    Bug,
    Ooze,
    Efreet,
    Serpent,

    Sword,
    Helmet,
    Armor,
    Wand1,
    Wand2,
    Scroll1,
}

/// Entity name and appearance.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Desc {
    pub name: String,
    pub icon: Icon,
}

/// Entity animation state.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Anim {
    pub tween_from: Location,
    pub tween_current: u32,
    pub tween_max: u32,
}

impl Anim {
    pub fn tick(&mut self) {
        if self.tween_current > 0 {
            self.tween_current -= 1;
        }
    }
}

impl Desc {
    pub fn new(name: &str, icon: Icon) -> Desc {
        // XXX: Not idiomatic to set this to be called with a non-owned
        // &str instead of a String, I just want to get away from typing
        // .to_string() everywhere with the calls that mostly use string
        // literals.
        Desc {
            name: name.to_string(),
            icon,
        }
    }
}

/// Map field-of-view and remembered terrain.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MapMemory {
    pub seen: LocationSet,
    pub remembered: LocationSet,
}

impl MapMemory {
    pub fn status(&self, loc: Location) -> Option<FovStatus> {
        if self.seen.contains(&loc) {
            Some(FovStatus::Seen)
        } else if self.remembered.contains(&loc) {
            Some(FovStatus::Remembered)
        } else {
            None
        }
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Brain {
    pub state: BrainState,
    pub alignment: Alignment,
    pub shout: ShoutType,
}

impl Brain {
    /// Create default enemy brain.
    pub fn enemy() -> Brain {
        Brain {
            ..Default::default()
        }
    }

    /// Create default player brain.
    pub fn player() -> Brain {
        Brain {
            state: BrainState::PlayerControl,
            alignment: Alignment::Good,
            ..Default::default()
        }
    }
}

impl Default for Brain {
    fn default() -> Brain {
        Brain {
            state: BrainState::Asleep,
            alignment: Alignment::Evil,
            shout: ShoutType::Silent,
        }
    }
}

/// Mob behavior state.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum BrainState {
    /// AI mob is inactive, but can be startled into action by noise or
    /// motion.
    Asleep,
    /// AI mob is looking for a fight.
    Hunting(Entity),
    /// Mob is under player control.
    PlayerControl,
}

/// How does a mob vocalize when alerted?
#[derive(Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum ShoutType {
    /// Humanoids
    Shout,
    /// Reptiles
    Hiss,
    /// Insects
    Buzz,
    /// Large monsters
    Roar,
    /// Slimes
    Gurgle,
    Silent,
}

/// Used to determine who tries to fight whom.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum Alignment {
    /// Attack anything and everything.
    Chaotic,
    /// Player alignment. The noble path of slaughtering everything that moves
    /// and gets in the way of taking their shiny stuff.
    Good,
    /// Enemy alignment. The foul cause of working together to defend your
    /// home and belongings against a powerful and devious intruder.
    Evil,
}

/// Damage state component. The default state is undamaged and unarmored.
#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub struct Health {
    /// The more wounds you have, the more hurt you are. How much damage you
    /// can take before dying depends on entity power level, not described by
    /// Wounds component. Probably in MobStat or something.
    pub wounds: i32,
    /// Armor points get eaten away before you start getting wounds.
    pub armor: i32,
}

impl Health {
    pub fn new() -> Health { Default::default() }
}

/// Items can be picked up and carried and they do stuff.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Item {
    pub item_type: ItemType,
    /// How many uses a wand or similar has left.
    pub charges: u32,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Serialize, Deserialize)]
/// Temporary creature properties
pub enum Status {
    /// Creature is acting erratically
    Confused,
    /// Is dead (not undead-dead, no-longer-subject-to-animate-things-logic-dead)
    Dead,
    /// Moves 1/3 slower than usual, stacks with Slow intrinsic.
    Slowed,
    /// Moves 1/3 faster than usual, stacks with Quick intrinsic.
    Hasted,
    /// Creature is delayed.
    ///
    /// This gets jumped up every time after the creature acted.
    Delayed,
}

pub type Statuses = BTreeMap<Status, u32>;

/// Stats component in the ECS that supports caching applied modifiers for efficiency.
#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub struct StatsComponent {
    /// Base stats that are intrinsic to this entity
    pub base: Stats,
    /// Modified stats derived from base and various effects that apply.
    ///
    /// Must be explicitly regenerated whenever an attached stats-affecting entity changes.
    pub actual: Stats,
}

impl StatsComponent {
    pub fn new(base: Stats) -> StatsComponent { StatsComponent { base, actual: base } }
}
