mod room_and_corridor_creation;
mod simple_map;

use crate::map_builders::simple_map::SimpleMapBuilder;
use crate::{Map, Position};
use specs::World;

trait MapBuilder {
    fn build(new_depth: i32) -> (Map, Position);
    fn spawn(map: &Map, ecs: &mut World, new_depth: i32);
}

pub fn build_random_map(new_depth: i32) -> (Map, Position) {
    SimpleMapBuilder::build(new_depth)
}

pub fn spawn(map: &mut Map, ecs: &mut World, new_depth: i32) {
    SimpleMapBuilder::spawn(map, ecs, new_depth);
}
