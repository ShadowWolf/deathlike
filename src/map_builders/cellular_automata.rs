use crate::{Map, Position, SHOW_MAPGEN_VISUALIZER, TileType, spawner};
use crate::map_builders::MapBuilder;
use specs::World;
use rltk::{RandomNumberGenerator};
use std::collections::HashMap;
use crate::map_builders::map_processing::{generate_voronoi_spawn_regions, remove_unreachable_areas};

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

        let exit_tile = remove_unreachable_areas(&mut self.map, start_index);
        self.take_snapshot();

        self.map.tiles[exit_tile] = TileType::StairsDown;
        self.take_snapshot();

        self.noise_areas = generate_voronoi_spawn_regions(&self.map, &mut rng);
        self.take_snapshot();
    }
}