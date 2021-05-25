use crate::{Position, Map, TileType, spawner, SHOW_MAPGEN_VISUALIZER};
use std::collections::HashMap;
use rltk::{RandomNumberGenerator, Point};
use specs::World;
use crate::map_builders::MapBuilder;
use crate::map_builders::map_processing::{remove_unreachable_areas, generate_voronoi_spawn_regions};

#[allow(dead_code)]
#[derive(PartialEq, Copy, Clone)]
pub enum DLAAlgorithm { WalkInwards, WalkOutwards, CentralAttractor }

#[allow(dead_code)]
#[derive(PartialEq, Copy, Clone)]
pub enum DLASymmetry { None, Horizontal, Vertical, Both }

pub struct DLABuilder {
    map: Map,
    starting_position: Position,
    depth: i32,
    history: Vec<Map>,
    noise_areas: HashMap<i32, Vec<usize>>,
    algorithm: DLAAlgorithm,
    brush_size: i32,
    symmetry: DLASymmetry,
    floor_percent: f32,
}

impl MapBuilder for DLABuilder {
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

impl DLABuilder {
    fn create(new_depth: i32, algorithm: DLAAlgorithm, symmetry: DLASymmetry, brush_size: i32, floor_percent: f32) -> DLABuilder {
        DLABuilder {
            map: Map::new(new_depth),
            starting_position: Position { x: 0, y: 0 },
            depth: new_depth,
            history: Vec::new(),
            noise_areas: HashMap::new(),
            algorithm,
            brush_size,
            symmetry,
            floor_percent,
        }
    }

    pub fn walk_inwards(new_depth: i32) -> DLABuilder {
        DLABuilder::create(new_depth, DLAAlgorithm::WalkInwards, DLASymmetry::None, 1, 0.25)
    }

    pub fn walk_outwards(new_depth: i32) -> DLABuilder {
        DLABuilder::create(new_depth, DLAAlgorithm::WalkOutwards, DLASymmetry::None, 2, 0.25)
    }

    pub fn central_attractor(new_depth: i32) -> DLABuilder {
        DLABuilder::create(new_depth, DLAAlgorithm::CentralAttractor, DLASymmetry::None, 2, 0.25)
    }

    pub fn insectoid(new_depth: i32) -> DLABuilder {
        DLABuilder::create(new_depth, DLAAlgorithm::CentralAttractor, DLASymmetry::Horizontal, 2, 0.25)
    }

    fn process_walk_inwards(&self, rng: &mut RandomNumberGenerator) -> (i32, i32) {
        let mut digger_x = rng.roll_dice(1, self.map.width - 3) + 1;
        let mut digger_y = rng.roll_dice(1, self.map.height - 3) + 1;
        let mut prev_x = digger_x;
        let mut prev_y = digger_y;
        let mut digger_index = self.map.xy_idx(digger_x, digger_y);

        while self.map.tiles[digger_index] == TileType::Wall {
            prev_x = digger_x;
            prev_y = digger_y;

            let direction = rng.roll_dice(1, 4);
            let stagger = self.stagger(direction, digger_x, digger_y);
            digger_x = stagger.0;
            digger_y = stagger.1;
            digger_index = self.map.xy_idx(digger_x, digger_y);
        }

        (prev_x, prev_y)
    }

    fn process_walk_outwards(&self, rng: &mut RandomNumberGenerator) -> (i32, i32) {
        let mut digger_x = self.starting_position.x;
        let mut digger_y = self.starting_position.y;
        let mut digger_index = self.map.xy_idx(digger_x, digger_y);

        while self.map.tiles[digger_index] == TileType::Floor {
            let direction = rng.roll_dice(1, 4);
            let stagger = self.stagger(direction, digger_x, digger_y);
            digger_x = stagger.0;
            digger_y = stagger.1;
            digger_index = self.map.xy_idx(digger_x, digger_y);
        }

        (digger_x, digger_y)
    }

    fn process_central_attractor(&self, rng: &mut RandomNumberGenerator) -> (i32, i32) {
        let mut digger_x = rng.roll_dice(1, self.map.width - 3) + 1;
        let mut digger_y = rng.roll_dice(1, self.map.height - 3) + 1;
        let mut prev_x = digger_x;
        let mut prev_y = digger_y;
        let mut digger_index = self.map.xy_idx(digger_x, digger_y);

        let mut path = rltk::line2d(rltk::LineAlg::Bresenham, Point::new(digger_x, digger_y), Point::new(self.starting_position.x, self.starting_position.y));

        while self.map.tiles[digger_index] == TileType::Wall && !path.is_empty() {
            prev_x = digger_x;
            prev_y = digger_y;

            let p = path[0];
            digger_x = p.x;
            digger_y = p.y;
            path.remove(0);
            digger_index = self.map.xy_idx(digger_x, digger_y);
        }

        (prev_x, prev_y)
    }

    fn stagger(&self, direction: i32, mut digger_x: i32, mut digger_y: i32) -> (i32, i32) {
        match direction {
            1 => { if digger_x > 2 {digger_x -= 1; } }
            2 => { if digger_x < self.map.width - 2 {digger_x += 1; } }
            3 => { if digger_y > 2 { digger_y += 1; } }
            _ => { if digger_y < self.map.height - 2 { digger_y += 1; }}
        }

        (digger_x, digger_y)
    }

    fn apply_paint(&mut self, x: i32, y: i32) {
        match self.brush_size {
            1 => {
                let i = self.map.xy_idx(x, y);
                self.map.tiles[i] = TileType::Floor;
            }
            _ => {
                let half_brush_size = self.brush_size / 2;
                for brush_y in y - half_brush_size .. y + half_brush_size {
                    for brush_x in x - half_brush_size .. x + half_brush_size {
                        if brush_x > 1 && brush_x < self.map.width - 1 && brush_y > 1 && brush_y < self.map.height - 1 {
                            let i = self.map.xy_idx(x, y);
                            self.map.tiles[i] = TileType::Floor;
                        }
                    }
                }
            }
        }
    }

    fn paint(&mut self, x: i32, y: i32) {
        match self.symmetry {
            DLASymmetry::None => {
                self.apply_paint(x, y);
            }
            DLASymmetry::Horizontal => {
                let center_x = self.map.width / 2;
                if x == center_x {
                    self.apply_paint(x, y);
                } else {
                    let dist = i32::abs(center_x - x);
                    self.apply_paint(center_x + dist, y);
                    self.apply_paint(center_x - dist, y);
                }
            }
            DLASymmetry::Vertical => {
                let center_y = self.map.height / 2;
                if y == center_y {
                    self.apply_paint(x, y);
                } else {
                    let dist = i32::abs(center_y - y);
                    self.apply_paint(x, center_y + dist);
                    self.apply_paint(x, center_y - dist);
                }
            }
            DLASymmetry::Both => {
                let center_y = self.map.height / 2;
                let center_x = self.map.width / 2;
                if y == center_y && x == center_x {
                    self.apply_paint(x, y);
                } else {
                    let dist_x = i32::abs(center_x - x);
                    self.apply_paint(center_x + dist_x, y);
                    self.apply_paint(center_x - dist_x, y);
                    let dist_y = i32::abs(center_y - y);
                    self.apply_paint(x, center_y + dist_y);
                    self.apply_paint(x, center_y - dist_y);
                }
            }
        }
    }

    #[allow(clippy::map_entry)]
    pub fn build(&mut self) {
        let mut rng = RandomNumberGenerator::new();

        self.starting_position = Position { x: self.map.width / 2, y: self.map.height / 2 };
        let start_index = self.map.xy_idx(self.starting_position.x, self.starting_position.y);
        self.take_snapshot();

        self.map.tiles[start_index] = TileType::Floor;
        self.map.tiles[start_index - 1] = TileType::Floor;
        self.map.tiles[start_index + 1] = TileType::Floor;
        self.map.tiles[start_index - self.map.width as usize] = TileType::Floor;
        self.map.tiles[start_index + self.map.width as usize] = TileType::Floor;

        let total_tiles = self.map.width * self.map.height;
        let desired_floor_tiles = (self.floor_percent * total_tiles as f32) as usize;
        let mut floor_tile_count = self.map.tiles.iter().filter(|a| **a == TileType::Floor).count();

        let iterations = 0;
        while floor_tile_count < desired_floor_tiles {
            match self.algorithm {
                DLAAlgorithm::WalkInwards => {
                    let (x, y) = self.process_walk_inwards(&mut rng);
                    self.paint(x, y);
                },
                DLAAlgorithm::WalkOutwards => {
                    let (x, y) = self.process_walk_outwards(&mut rng);
                    self.paint(x, y);
                }
                DLAAlgorithm::CentralAttractor => {
                    let (x, y) = self.process_central_attractor(&mut rng);
                    self.paint(x, y);
                }
            }

            if iterations % 10 == 0 {
                self.take_snapshot();
            }

            floor_tile_count = self.map.tiles.iter().filter(|a| **a == TileType::Floor).count();
        }

        let exit_tile = remove_unreachable_areas(&mut self.map, start_index);
        self.take_snapshot();

        self.map.tiles[exit_tile] = TileType::StairsDown;
        self.take_snapshot();

        self.noise_areas = generate_voronoi_spawn_regions(&self.map, &mut rng);

        self.take_snapshot();
    }
}