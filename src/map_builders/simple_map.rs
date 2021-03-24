use super::{Map, MapBuilder};
use crate::map_builders::room_and_corridor_creation::{
    apply_horizontal_tunnel, apply_room_to_map, apply_vertical_tunnel,
};
use crate::{spawn_room, Position, Rect, TileType};
use rltk::RandomNumberGenerator;
use specs::prelude::*;

pub struct SimpleMapBuilder {}

impl SimpleMapBuilder {
    fn rooms_and_cooridors(map: &mut Map) -> Position {
        const MAX_ROOMS: i32 = 30;
        const MIN_SIZE: i32 = 6;
        const MAX_SIZE: i32 = 10;

        let mut rng = RandomNumberGenerator::new();

        for i in 0..MAX_ROOMS {
            let w = rng.range(MIN_SIZE, MAX_SIZE);
            let h = rng.range(MIN_SIZE, MAX_SIZE);
            let x = rng.roll_dice(1, map.width - w - 1) - 1;
            let y = rng.roll_dice(1, map.height - h - 1) - 1;

            let new_room = Rect::new(x, y, w, h);

            if !map.rooms.iter().any(|r| new_room.intersect(r)) {
                apply_room_to_map(map, &new_room);

                if !map.rooms.is_empty() {
                    let (nx, ny) = new_room.center();
                    let (px, py) = map.rooms[map.rooms.len() - 1].center();

                    if rng.range(0, 2) == 1 {
                        apply_horizontal_tunnel(map, px, nx, py);
                        apply_vertical_tunnel(map, py, ny, px);
                    } else {
                        apply_vertical_tunnel(map, py, ny, px);
                        apply_horizontal_tunnel(map, px, nx, ny);
                    }
                }

                map.rooms.push(new_room);
            }
        }

        let (stairs_x, stairs_y) = map.rooms[map.rooms.len() - 1].center();
        let stairs_index = map.xy_idx(stairs_x, stairs_y);
        map.tiles[stairs_index] = TileType::StairsDown;
        let (start_x, start_y) = map.rooms[0].center();
        Position {
            x: start_x,
            y: start_y,
        }
    }
}

impl MapBuilder for SimpleMapBuilder {
    fn build(new_depth: i32) -> (Map, Position) {
        let mut map = Map::new(new_depth);
        let player_pos = SimpleMapBuilder::rooms_and_cooridors(&mut map);
        (map, player_pos)
    }

    fn spawn(map: &Map, ecs: &mut World, new_depth: i32) {
        for room in map.rooms.iter().skip(1) {
            spawn_room(ecs, room, new_depth);
        }
    }
}
