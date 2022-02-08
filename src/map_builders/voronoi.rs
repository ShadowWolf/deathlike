use crate::{Position, Map, SHOW_MAPGEN_VISUALIZER, spawner, TileType, impl_map_builder_with_noise_areas};
use std::collections::HashMap;
use crate::map_builders::{build_snapshot, MapBuilder};
use specs::World;
use rltk::RandomNumberGenerator;
use crate::map_builders::map_processing::{remove_unreachable_areas, generate_voronoi_spawn_regions};

#[derive(PartialEq, Copy, Clone)]
pub enum DistanceAlgorithm { Pythagoras, Manhattan, Chebyshev }

pub struct VoronoiBuilder {
    map: Map,
    starting_position: Position,
    depth: i32,
    history: Vec<Map>,
    noise_areas: HashMap<i32, Vec<usize>>,
    number_of_seeds: usize,
    distance_algorithm: DistanceAlgorithm
}

impl_map_builder_with_noise_areas!(VoronoiBuilder);

impl VoronoiBuilder {
    fn new(new_depth: i32, number_of_seeds: usize, distance_algorithm: DistanceAlgorithm) -> VoronoiBuilder {
        VoronoiBuilder {
            map: Map::new(new_depth),
            starting_position: Position::origin(),
            history: Vec::new(),
            noise_areas: HashMap::new(),
            number_of_seeds,
            depth: new_depth,
            distance_algorithm,
        }
    }

    pub fn pythagoras(new_depth: i32) -> VoronoiBuilder {
        VoronoiBuilder::new(new_depth, 64, DistanceAlgorithm::Pythagoras)
    }

    pub fn manhattan(new_depth: i32) -> VoronoiBuilder {
        VoronoiBuilder::new(new_depth, 64, DistanceAlgorithm::Manhattan)
    }

    #[allow(dead_code)]
    pub fn chebyshev(new_depth: i32) -> VoronoiBuilder {
        VoronoiBuilder::new(new_depth, 64, DistanceAlgorithm::Chebyshev)
    }

    fn build(&mut self) {
        let mut rng = RandomNumberGenerator::new();

        let mut voronoi_seeds: Vec<(usize, rltk::Point)> = Vec::new();

        while voronoi_seeds.len() < self.number_of_seeds {
            let v_x = rng.roll_dice(1, self.map.width - 1);
            let v_y = rng.roll_dice(1, self.map.height - 1);

            let candidate = (self.map.xy_idx(v_x, v_y), rltk::Point::new(v_x, v_y));
            if !voronoi_seeds.contains(&candidate) {
                voronoi_seeds.push(candidate);
            }
        }

        let mut voronoi_distance = vec![(0, 0.0f32); self.number_of_seeds];
        let mut voronoi_membership: Vec<i32> = vec![0; self.map.width as usize * self.map.height as usize];
        for (i, vid) in voronoi_membership.iter_mut().enumerate() {
            let x = i as i32 % self.map.width;
            let y = i as i32 / self.map.width;

            for (seed, (_, end)) in voronoi_seeds.iter().enumerate() {
                let distance;
                match self.distance_algorithm {
                    DistanceAlgorithm::Pythagoras => {
                        distance = rltk::DistanceAlg::PythagorasSquared.distance2d(rltk::Point::new(x, y), *end);
                    }
                    DistanceAlgorithm::Manhattan => {
                        distance = rltk::DistanceAlg::Manhattan.distance2d(rltk::Point::new(x, y), *end);
                    }
                    DistanceAlgorithm::Chebyshev => {
                        distance = rltk::DistanceAlg::Chebyshev.distance2d(rltk::Point::new(x, y), *end);
                    }
                }

                voronoi_distance[seed] = (seed, distance);
            }

            voronoi_distance.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            *vid = voronoi_distance[0].0 as i32;
        }

        for y in 1..self.map.height - 1 {
            for x in 1..self.map.width - 1 {
                let mut neighbors = 0;
                let current_index = self.map.xy_idx(x, y);
                let current_seed = voronoi_membership[current_index];

                if voronoi_membership[self.map.xy_idx(x - 1, y)] != current_seed { neighbors += 1; }
                if voronoi_membership[self.map.xy_idx(x + 1, y)] != current_seed { neighbors += 1; }
                if voronoi_membership[self.map.xy_idx(x, y - 1)] != current_seed { neighbors += 1; }
                if voronoi_membership[self.map.xy_idx(x, y + 1)] != current_seed { neighbors += 1; }

                if neighbors < 2 {
                    self.map.tiles[current_index] = TileType::Floor;
                }
            }

            self.take_snapshot();
        }

        self.starting_position = Position { x: self.map.width / 2, y: self.map.height / 2 };
        while self.map.get_tile(self.starting_position.x, self.starting_position.y) != TileType::Floor {
            self.starting_position.x -= 1;
        }

        let start_index = self.map.xy_idx(self.starting_position.x, self.starting_position.y);

        let exit_tile = remove_unreachable_areas(&mut self.map, start_index);
        self.take_snapshot();

        self.map.tiles[exit_tile] = TileType::StairsDown;
        self.take_snapshot();

        self.noise_areas = generate_voronoi_spawn_regions(&self.map, &mut rng);
        self.take_snapshot();
    }
}