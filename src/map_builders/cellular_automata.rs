use crate::{Map, Position, SHOW_MAPGEN_VISUALIZER, TileType, spawner};
use crate::map_builders::MapBuilder;
use specs::World;
use rltk::{RandomNumberGenerator, FastNoise};
use std::collections::HashMap;

const MIN_ROOM_SIZE: i32 = 8;

pub struct CellularAutomataBuilder {
    map: Map,
    starting_position: Position,
    depth: i32,
    history: Vec<Map>,
    noise_areas: HashMap<i32, Vec<usize>>,
}

impl MapBuilder for CellularAutomataBuilder {
    fn build_map(&mut self) {
        self.build();
    }

    fn spawn_entities(&mut self, ecs: &mut World) {
        for (_i, area) in self.noise_areas.iter() {
            spawner::spawn_region(ecs, area, self.depth);
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

impl CellularAutomataBuilder {
    pub fn new(new_depth: i32) -> CellularAutomataBuilder {
        CellularAutomataBuilder {
            map: Map::new(new_depth),
            starting_position: Position { x: 0, y: 0 },
            depth: new_depth,
            history: Vec::new(),
            noise_areas: HashMap::new(),
        }
    }

    #[allow(clippy::map_entry)]
    pub fn build(&mut self) {
        let mut rng = RandomNumberGenerator::new();

        for y in 1..self.map.height - 1 {
            for x in 1..self.map.width - 1 {
                let roll = rng.roll_dice(1, 100);
                let i = self.map.xy_idx(x, y);
                if roll > 55 {
                    self.map.tiles[i] = TileType::Floor;
                } else {
                    self.map.tiles[i] = TileType::Wall;
                }
            }
        }

        self.take_snapshot();

        self.build_automata();

        let mut noise = rltk::FastNoise::seeded(rng.roll_dice(1, 65536) as u64);
        noise.set_noise_type(rltk::NoiseType::Cellular);
        noise.set_frequency(0.08);
        noise.set_cellular_distance_function(rltk::CellularDistanceFunction::Manhattan);

        self.build_noise_map(&mut noise);

        self.take_snapshot();
    }

    fn build_noise_map(&mut self, noise: &mut FastNoise) {
        for y in 1..self.map.height - 1 {
            for x in 1..self.map.width - 1 {
                let i = self.map.xy_idx(x, y);
                if self.map.tiles[i] == TileType::Floor {
                    let cell_value_float = noise.get_noise(x as f32, y as f32) * 10240.0;
                    let cell_value = cell_value_float as i32;

                    if self.noise_areas.contains_key(&cell_value) {
                        self.noise_areas.get_mut(&cell_value).unwrap().push(i);
                    } else {
                        self.noise_areas.insert(cell_value, vec![i]);
                    }
                }
            }
        }
    }

    fn build_automata(&mut self) {
        for _i in 0..15 {
            let mut new_tiles = self.map.tiles.clone();

            for y in 1..self.map.height - 1 {
                for x in 1..self.map.width - 1 {
                    let i = self.map.xy_idx(x, y);

                    let mut neighbors = 0;
                    if self.map.tiles[i - 1] == TileType::Wall { neighbors += 1; }
                    if self.map.tiles[i + 1] == TileType::Wall { neighbors += 1; }
                    if self.map.tiles[i - self.map.width as usize] == TileType::Wall { neighbors += 1; }
                    if self.map.tiles[i + self.map.width as usize] == TileType::Wall { neighbors += 1; }
                    if self.map.tiles[i - (self.map.width as usize - 1)] == TileType::Wall { neighbors += 1; }
                    if self.map.tiles[i - (self.map.width as usize + 1)] == TileType::Wall { neighbors += 1; }
                    if self.map.tiles[i + (self.map.width as usize - 1)] == TileType::Wall { neighbors += 1; }
                    if self.map.tiles[i + (self.map.width as usize + 1)] == TileType::Wall { neighbors += 1; }

                    if neighbors > 4 || neighbors == 0 {
                        new_tiles[i] = TileType::Wall;
                    } else {
                        new_tiles[i] = TileType::Floor;
                    }
                }
            }

            self.map.tiles = new_tiles.clone();
            self.take_snapshot();
        }

        self.starting_position = Position { x: self.map.width / 2, y: self.map.height / 2 };
        let mut start_index = self.map.xy_idx(self.starting_position.x, self.starting_position.y);
        while self.map.tiles[start_index] != TileType::Floor {
            self.starting_position.x -= 1;
            start_index = self.map.xy_idx(self.starting_position.x, self.starting_position.y);
        }

        self.take_snapshot();

        let map_starts: Vec<usize> = vec![start_index];

        rltk::console::log(format!("Length of map_starts is {}", map_starts.len()));
        rltk::console::log(format!("Length of the map tiles is {}", self.map.tiles.len()));

        let dijkstra_map = rltk::DijkstraMap::new(self.map.width as usize, self.map.height as usize, &map_starts, &self.map, 200.0);
        let mut exit_tile = (0, 0.0f32);
        for (i, tile) in self.map.tiles.iter_mut().enumerate() {
            if *tile == TileType::Floor {
                let distance = dijkstra_map.map[i];
                if distance == f32::MAX {
                    *tile = TileType::Wall;
                } else {
                    if distance > exit_tile.1 {
                        exit_tile.0 = i;
                        exit_tile.1 = distance;
                    }
                }
            }
        }

        self.take_snapshot();
        self.map.tiles[exit_tile.0] = TileType::StairsDown;
        self.take_snapshot();
    }
}