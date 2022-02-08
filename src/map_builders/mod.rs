mod bsp_dungeon;
mod bsp_interior;
mod cellular_automata;
mod room_and_corridor_creation;
mod simple_map;
mod drunkard;
mod map_processing;
mod maze;
mod dla;
mod drawing;
mod voronoi;
mod waveform_collapse;
mod prefab_builder;
mod prefab_levels;

use crate::{Map, Position, SHOW_MAPGEN_VISUALIZER};

use specs::World;

use crate::map_builders::dla::DLABuilder;
use crate::map_builders::bsp_dungeon::BspDungeonBuilder;
use crate::map_builders::bsp_interior::BspInteriorBuilder;
use crate::map_builders::cellular_automata::CellularAutomataBuilder;
use crate::map_builders::drunkard::DrunkardsWalkBuilder;
use crate::map_builders::maze::MazeBuilder;
use crate::map_builders::simple_map::SimpleMapBuilder;
use crate::map_builders::voronoi::VoronoiBuilder;
use crate::map_builders::waveform_collapse::WaveformCollapseBuilder;

pub trait MapBuilder {
    fn build_map(&mut self);
    fn spawn_entities(&mut self, ecs: &mut World);
    fn get_map(&self) -> Map;
    fn get_starting_position(&self) -> Position;
    fn get_snapshot_history(&self) -> Vec<Map>;
    fn take_snapshot(&mut self);
}

#[macro_export]
macro_rules! impl_map_builder_with_noise_areas {
    ($($t:ty),+ $(,)?) => ($(
        impl MapBuilder for $t {
            fn build_map(&mut self) {
                self.build();
            }

            fn spawn_entities(&mut self, ecs: &mut World) {
                for (_d, area) in self.noise_areas.iter() {
                    spawner::spawn_region(ecs, area, self.depth);
                }
            }

            fn get_map(&self) -> Map {
                self.map.clone()
            }

            fn get_starting_position(&self) -> Position {
                self.starting_position.clone()
            }

            fn get_snapshot_history(&self) -> Vec<Map> {
                self.history.clone()
            }

            fn take_snapshot(&mut self) {
                match build_snapshot(&self.map) {
                    None => {}
                    Some(t) => self.history.push(t),
                };
            }
        }
    )+)
}

#[macro_export]
macro_rules! impl_map_builder_with_rooms {
    ($($t:ty),+ $(,)?) => ($(
        impl MapBuilder for $t {
                fn build_map(&mut self) {
                    self.build();
                }

                fn spawn_entities(&mut self, ecs: &mut World) {
                    for room in self.rooms.iter().skip(1) {
                        spawner::spawn_room(ecs, room, self.depth);
                    }
                }

                fn get_map(&self) -> Map {
                    self.map.clone()
                }

                fn get_starting_position(&self) -> Position {
                    self.starting_position.clone()
                }

                fn get_snapshot_history(&self) -> Vec<Map> {
                    self.history.clone()
                }

                fn take_snapshot(&mut self) {
                    match build_snapshot(&self.map) {
                        None => {}
                        Some(t) => self.history.push(t),
                    };
                }
        }
    )+)
}

pub fn build_snapshot(map: &Map) -> Option<Map> {
    if SHOW_MAPGEN_VISUALIZER {
        let mut snapshot = map.clone();
        for v in snapshot.revealed_tiles.iter_mut() {
            *v = true;
        }

        return Some(snapshot);
    }

    None
}

pub fn static_builder(new_depth: i32) -> Box<dyn MapBuilder> {
    Box::new(DLABuilder::walk_inwards(new_depth))
}

pub fn random_builder(new_depth: i32) -> Box<dyn MapBuilder> {
    let mut rng = rltk::RandomNumberGenerator::new();
    let builder = rng.roll_dice(1, 16);
    rltk::log(format!("Using builder # {}", builder));
    let result: Box<dyn MapBuilder> = match builder {
        1 => Box::new(BspDungeonBuilder::new(new_depth)),
        2 => Box::new(BspInteriorBuilder::new(new_depth)),
        3 => Box::new(CellularAutomataBuilder::new(new_depth)),
        4 => Box::new(DrunkardsWalkBuilder::open_area(new_depth)),
        5 => Box::new(DrunkardsWalkBuilder::open_halls(new_depth)),
        6 => Box::new(DrunkardsWalkBuilder::winding_passages(new_depth)),
        7 => Box::new(DrunkardsWalkBuilder::big_passages(new_depth)),
        8 => Box::new(DrunkardsWalkBuilder::fearful_symmetry(new_depth)),
        9 => Box::new(MazeBuilder::new(new_depth)),
        10 => Box::new(DLABuilder::walk_inwards(new_depth)),
        11 => Box::new(DLABuilder::walk_outwards(new_depth)),
        12 => Box::new(DLABuilder::central_attractor(new_depth)),
        13 => Box::new(DLABuilder::insectoid(new_depth)),
        14 => Box::new(VoronoiBuilder::pythagoras(new_depth)),
        15 => Box::new(VoronoiBuilder::manhattan(new_depth)),
        _ => Box::new(SimpleMapBuilder::new(new_depth))
    };

    if rng.roll_dice(1, 3) == 1 {
        rltk::log("Layering the waveform collapse builder on top");
        Box::new(WaveformCollapseBuilder::derived_map(new_depth, result))
    } else {
        result
    }
}
