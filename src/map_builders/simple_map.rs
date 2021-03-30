use super::{Map, MapBuilder};
use crate::map_builders::room_and_corridor_creation::{
    apply_horizontal_tunnel, apply_room_to_map, apply_vertical_tunnel,
};
use crate::{spawn_room, Position, Rect, TileType, SHOW_MAPGEN_VISUALIZER};
use rltk::RandomNumberGenerator;
use specs::prelude::*;

pub struct SimpleMapBuilder {
    map: Map,
    starting_position: Position,
    depth: i32,
    pub rooms: Vec<Rect>,
    history: Vec<Map>,
}

impl SimpleMapBuilder {
    pub fn new(new_depth: i32) -> SimpleMapBuilder {
        SimpleMapBuilder {
            depth: new_depth,
            map: Map::new(new_depth),
            starting_position: Position { x: 0, y: 0 },
            rooms: Vec::new(),
            history: Vec::new(),
        }
    }

    fn rooms_and_cooridors(&mut self) {
        const MAX_ROOMS: i32 = 30;
        const MIN_SIZE: i32 = 6;
        const MAX_SIZE: i32 = 10;

        let mut rng = RandomNumberGenerator::new();

        for i in 0..MAX_ROOMS {
            let w = rng.range(MIN_SIZE, MAX_SIZE);
            let h = rng.range(MIN_SIZE, MAX_SIZE);
            let x = rng.roll_dice(1, self.map.width - w - 1) - 1;
            let y = rng.roll_dice(1, self.map.height - h - 1) - 1;

            let new_room = Rect::new(x, y, w, h);

            if !self.rooms.iter().any(|r| new_room.intersect(r)) {
                apply_room_to_map(&mut self.map, &new_room);
                self.take_snapshot();

                if !self.rooms.is_empty() {
                    let (nx, ny) = new_room.center();
                    let (px, py) = self.rooms[self.rooms.len() - 1].center();

                    if rng.range(0, 2) == 1 {
                        apply_horizontal_tunnel(&mut self.map, px, nx, py);
                        apply_vertical_tunnel(&mut self.map, py, ny, nx);
                    } else {
                        apply_vertical_tunnel(&mut self.map, py, ny, px);
                        apply_horizontal_tunnel(&mut self.map, px, nx, ny);
                    }
                }

                self.rooms.push(new_room);
                self.take_snapshot();
            }
        }

        let (stairs_x, stairs_y) = self.rooms[self.rooms.len() - 1].center();
        let stairs_index = self.map.xy_idx(stairs_x, stairs_y);
        self.map.tiles[stairs_index] = TileType::StairsDown;

        let (start_x, start_y) = self.rooms[0].center();

        self.starting_position = Position {
            x: start_x,
            y: start_y,
        };
    }
}

impl MapBuilder for SimpleMapBuilder {
    fn build_map(&mut self) {
        self.rooms_and_cooridors();
    }

    fn spawn_entities(&mut self, ecs: &mut World) {
        for room in self.rooms.iter().skip(1) {
            spawn_room(ecs, room, self.depth);
        }
    }

    fn get_map(&mut self) -> Map {
        self.map.clone()
    }

    fn get_starting_position(&mut self) -> Position {
        self.starting_position.clone()
    }

    fn get_snapshot_history(&self) -> Vec<Map> {
        self.history.clone()
    }

    fn take_snapshot(&mut self) {
        if SHOW_MAPGEN_VISUALIZER {
            let mut snapshot = self.map.clone();
            for v in snapshot.revealed_tiles.iter_mut() {
                *v = true;
            }
            self.history.push(snapshot);
        }
    }
}
