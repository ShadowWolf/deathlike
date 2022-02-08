mod constraints;
mod map_chunk;
mod solver;

use std::collections::HashMap;
use specs::World;
use rltk::{RandomNumberGenerator};

use crate::{Map, Position, SHOW_MAPGEN_VISUALIZER, TileType, spawner, impl_map_builder_with_noise_areas};
use crate::map_builders::{build_snapshot, MapBuilder};
use crate::map_builders::map_processing::{remove_unreachable_areas, generate_voronoi_spawn_regions};
use crate::map_builders::waveform_collapse::constraints::{build_patterns, patterns_to_constraints, render_pattern_to_map};
use crate::map_builders::waveform_collapse::map_chunk::MapChunk;
use crate::map_builders::waveform_collapse::solver::Solver;

pub struct WaveformCollapseBuilder {
    map: Map,
    starting_position: Position,
    depth: i32,
    history: Vec<Map>,
    noise_areas: HashMap<i32, Vec<usize>>,
    derive_from: Option<Box<dyn MapBuilder>>,
}

impl_map_builder_with_noise_areas!(WaveformCollapseBuilder);

impl WaveformCollapseBuilder {
    pub fn derived_map(new_depth: i32, builder: Box<dyn MapBuilder>) -> WaveformCollapseBuilder {
        WaveformCollapseBuilder::new(new_depth, Some(builder))
    }

    pub fn new(new_depth: i32, derive_from: Option<Box<dyn MapBuilder>>) -> WaveformCollapseBuilder {
        WaveformCollapseBuilder {
            map: Map::new(new_depth),
            starting_position: Position::origin(),
            depth: new_depth,
            history: Vec::new(),
            noise_areas: HashMap::new(),
            derive_from
        }
    }

    fn build(&mut self) {
        let mut rng = RandomNumberGenerator::new();

        const CHUNK_SIZE: i32 = 8;

        let derived_map = &mut self.derive_from.as_mut().unwrap();
        derived_map.build_map();
        self.map = derived_map.get_map();
        for t in self.map.tiles.iter_mut() {
            if *t == TileType::StairsDown { *t = TileType::Floor }
        }
        self.take_snapshot();

        let patterns = build_patterns(&self.map, CHUNK_SIZE, true, true);
        let constraints = patterns_to_constraints(patterns, CHUNK_SIZE);
        self.render_tile_gallery(&constraints, CHUNK_SIZE);

        self.map = Map::new(self.depth);
        loop {
            let mut solver = Solver::new(constraints.clone(), CHUNK_SIZE, &self.map);
            while !solver.iteration(&mut self.map, &mut rng) {
                self.take_snapshot();
            }

            self.take_snapshot();
            if solver.possible {
                break;
            }
        }

        self.starting_position = Position { x: self.map.width / 2, y: self.map.height / 2 };
        self.take_snapshot();

        let start_index = self.map.xy_idx(self.starting_position.x, self.starting_position.y);
        let exit_tile = remove_unreachable_areas(&mut self.map, start_index);
        self.take_snapshot();

        self.map.tiles[exit_tile] = TileType::StairsDown;
        self.take_snapshot();

        self.noise_areas = generate_voronoi_spawn_regions(&self.map, &mut rng);
    }

    fn render_tile_gallery(&mut self, constraints: &Vec<MapChunk>, chunk_size: i32) {
        self.map = Map::new(0);
        let mut counter = 0;
        let mut x = 1;
        let mut y = 1;

        while counter < constraints.len() {
            render_pattern_to_map(&mut self.map, &constraints[counter], chunk_size, x, y);

            x += chunk_size + 1;
            if x + chunk_size > self.map.width {
                x = 1;
                y += chunk_size + 1;

                if y + chunk_size > self.map.height {
                    self.take_snapshot();
                    self.map = Map::new(0);

                    x = 1;
                    y = 1;
                }
            }

            counter += 1;
        }

        self.take_snapshot();
    }
}
