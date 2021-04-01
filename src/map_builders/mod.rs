mod bsp_dungeon;
mod room_and_corridor_creation;
mod simple_map;

use crate::map_builders::bsp_dungeon::BspDungeonBuilder;
use crate::map_builders::simple_map::SimpleMapBuilder;
use crate::{Map, Position};
use rltk::RandomNumberGenerator;
use specs::World;

pub trait MapBuilder {
    fn build_map(&mut self);
    fn spawn_entities(&mut self, ecs: &mut World);
    fn get_map(&mut self) -> Map;
    fn get_starting_position(&mut self) -> Position;
    fn get_snapshot_history(&self) -> Vec<Map>;
    fn take_snapshot(&mut self);
}

pub fn random_builder(new_depth: i32) -> Box<dyn MapBuilder> {
    let mut rng = RandomNumberGenerator::new();
    let builder = rng.roll_dice(1, 2);

    match builder {
        1 => Box::new(BspDungeonBuilder::new(new_depth)),
        _ => Box::new(SimpleMapBuilder::new(new_depth)),
    }
}
