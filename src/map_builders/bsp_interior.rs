use crate::map_builders::{build_snapshot, MapBuilder};
use crate::{spawner, Map, Position, Rect, TileType, SHOW_MAPGEN_VISUALIZER, impl_map_builder_with_rooms};
use rltk::RandomNumberGenerator;
use specs::World;

const MIN_ROOM_SIZE: i32 = 8;

pub struct BspInteriorBuilder {
    map: Map,
    starting_position: Position,
    depth: i32,
    rooms: Vec<Rect>,
    history: Vec<Map>,
    rects: Vec<Rect>,
}

impl_map_builder_with_rooms!(BspInteriorBuilder);

impl BspInteriorBuilder {
    pub fn new(new_depth: i32) -> BspInteriorBuilder {
        BspInteriorBuilder {
            map: Map::new(new_depth),
            starting_position: Position::origin(),
            depth: new_depth,
            rooms: Vec::new(),
            history: Vec::new(),
            rects: Vec::new(),
        }
    }

    pub fn build(&mut self) {
        let mut rng = RandomNumberGenerator::new();
        self.rects.clear();
        self.rects
            .push(Rect::new(1, 1, self.map.width - 2, self.map.height - 2));
        let first_room = self.rects[0];
        self.add_subrects(first_room, &mut rng);

        let rooms = self.rects.clone();
        for r in rooms.iter() {
            let room = *r;
            self.rooms.push(room);
            for y in room.y1..room.y2 {
                for x in room.x1..room.x2 {
                    let i = self.map.xy_idx(x, y);
                    if i > 0 && i < ((self.map.width * self.map.height) - 1) as usize {
                        self.map.tiles[i] = TileType::Floor;
                    }
                }
            }

            self.take_snapshot();
        }

        self.add_corridors(&mut rng);

        self.add_stairs();

        let (start_x, start_y) = self.rooms[0].center();
        self.starting_position = Position {
            x: start_x,
            y: start_y,
        };
    }

    fn add_stairs(&mut self) {
        let (x, y) = self.rooms[self.rooms.len() - 1].center();
        let stairs_position = self.map.xy_idx(x, y);
        self.map.tiles[stairs_position] = TileType::StairsDown;
    }

    fn add_corridors(&mut self, rng: &mut RandomNumberGenerator) {
        for i in 0..self.rooms.len() - 1 {
            let room = self.rooms[i];
            let next_room = self.rooms[i + 1];
            let start_x = room.x1 + (rng.roll_dice(1, i32::abs(room.x1 - room.x2)) - 1);
            let start_y = room.y1 + (rng.roll_dice(1, i32::abs(room.y1 - room.y2)) - 1);
            let end_x =
                next_room.x1 + (rng.roll_dice(1, i32::abs(next_room.x1 - next_room.x2)) - 1);
            let end_y =
                next_room.y1 + (rng.roll_dice(1, i32::abs(next_room.y1 - next_room.y2)) - 1);

            self.draw_corridor(start_x, start_y, end_x, end_y);
            self.take_snapshot();
        }
    }

    fn draw_corridor(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
        let mut x = x1;
        let mut y = y1;

        while x != x2 || y != y2 {
            if x < x2 {
                x += 1;
            } else if x > x2 {
                x -= 1;
            } else if y < y2 {
                y += 1;
            } else if y > y2 {
                y -= 1;
            }

            let idx = self.map.xy_idx(x, y);
            self.map.tiles[idx] = TileType::Floor;
        }
    }

    fn add_subrects(&mut self, rect: Rect, rng: &mut RandomNumberGenerator) {
        if !self.rects.is_empty() {
            self.rects.remove(self.rects.len() - 1);
        }

        let width = rect.x2 - rect.x1;
        let height = rect.y2 - rect.y1;
        let half_width = width / 2;
        let half_height = height / 2;

        let split = rng.roll_dice(1, 4);

        if split <= 2 {
            let h1 = Rect::new(rect.x1, rect.y1, half_width - 1, height);
            self.rects.push(h1);
            if half_width > MIN_ROOM_SIZE {
                self.add_subrects(h1, rng);
            }

            let h2 = Rect::new(rect.x1 + half_width, rect.y1, half_width, height);
            self.rects.push(h2);

            if half_width > MIN_ROOM_SIZE {
                self.add_subrects(h2, rng);
            }
        } else {
            let v1 = Rect::new(rect.x1, rect.y1, width, half_height - 1);
            self.rects.push(v1);
            if half_height > MIN_ROOM_SIZE {
                self.add_subrects(v1, rng);
            }

            let v2 = Rect::new(rect.x1, rect.y1 + half_height, width, half_height);
            self.rects.push(v2);
            if half_height > MIN_ROOM_SIZE {
                self.add_subrects(v2, rng);
            }
        }
    }
}
