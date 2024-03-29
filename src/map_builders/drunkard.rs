use std::collections::HashMap;
use crate::{Map, Position, spawner, SHOW_MAPGEN_VISUALIZER, TileType, impl_map_builder_with_noise_areas};
use crate::map_builders::{build_snapshot, MapBuilder};
use specs::World;
use rltk::RandomNumberGenerator;
use crate::map_builders::map_processing::{remove_unreachable_areas, generate_voronoi_spawn_regions};
use crate::map_builders::drawing::{Symmetry, paint};


#[derive(PartialEq, Copy, Clone)]
pub enum DrunkSpawnMode { StartingPoint, Random }

pub struct DrunkardSettings {
    pub spawn_mode: DrunkSpawnMode,
    pub drunken_lifetime: i32,
    pub floor_percent: f32,
    pub brush_size: i32,
    pub symmetry: Symmetry,
}

pub struct DrunkardsWalkBuilder {
    map: Map,
    starting_position: Position,
    depth: i32,
    history: Vec<Map>,
    noise_areas: HashMap<i32, Vec<usize>>,
    settings: DrunkardSettings
}

impl_map_builder_with_noise_areas!(DrunkardsWalkBuilder);

impl DrunkardsWalkBuilder {
    pub fn new(new_depth: i32, settings: DrunkardSettings) -> DrunkardsWalkBuilder {
        DrunkardsWalkBuilder {
            map: Map::new(new_depth),
            starting_position: Position {x: 0, y: 0 },
            depth: new_depth,
            history: Vec::new(),
            noise_areas: HashMap::new(),
            settings,
        }
    }

    pub fn open_area(new_depth: i32) -> DrunkardsWalkBuilder {
        DrunkardsWalkBuilder::new(new_depth, DrunkardSettings {
            floor_percent: 0.5,
            drunken_lifetime: 400,
            spawn_mode: DrunkSpawnMode::StartingPoint,
            brush_size: 1,
            symmetry: Symmetry::None
        })
    }

    pub fn open_halls(new_depth: i32) -> DrunkardsWalkBuilder {
        DrunkardsWalkBuilder::new(new_depth, DrunkardSettings {
            spawn_mode: DrunkSpawnMode::Random,
            drunken_lifetime: 400,
            floor_percent: 0.5,
            brush_size: 1,
            symmetry: Symmetry::None
        })
    }

    pub fn winding_passages(new_depth: i32) -> DrunkardsWalkBuilder {
        DrunkardsWalkBuilder::new(new_depth, DrunkardSettings {
            spawn_mode: DrunkSpawnMode::Random,
            floor_percent: 0.4,
            drunken_lifetime: 100,
            brush_size: 1,
            symmetry: Symmetry::None
        })
    }

    pub fn big_passages(new_depth: i32) -> DrunkardsWalkBuilder {
        DrunkardsWalkBuilder::new(new_depth, DrunkardSettings {
            spawn_mode: DrunkSpawnMode::Random,
            drunken_lifetime: 100,
            floor_percent: 0.4,
            brush_size: 2,
            symmetry: Symmetry::None
        })
    }

    pub fn fearful_symmetry(new_depth: i32) -> DrunkardsWalkBuilder {
        DrunkardsWalkBuilder::new(new_depth, DrunkardSettings {
            spawn_mode: DrunkSpawnMode::Random,
            drunken_lifetime: 100,
            floor_percent: 0.4,
            brush_size: 2,
            symmetry: Symmetry::Both,
        })
    }

    #[allow(clippy::map_entry)]
    fn build(&mut self) {
        let mut rng = RandomNumberGenerator::new();

        self.starting_position = Position { x: self.map.width / 2, y: self.map.height / 2 };
        let start_index = self.map.xy_idx(self.starting_position.x, self.starting_position.y);
        self.map.tiles[start_index] = TileType::Floor;

        let total_tiles = self.map.width * self.map.height;
        let desired_floor_tiles = (self.settings.floor_percent * total_tiles as f32) as usize;
        let mut floor_tile_count = self.map.tiles.iter().filter(|a| **a == TileType::Floor).count();
        let mut digger_count = 0;
        let mut active_digger_count = 0;

        while floor_tile_count < desired_floor_tiles {
            let mut mutated_tiles = false;
            let mut drunk_x;
            let mut drunk_y;

            match self.settings.spawn_mode {
                DrunkSpawnMode::StartingPoint => {
                    drunk_x = self.starting_position.x;
                    drunk_y = self.starting_position.y;
                },
                DrunkSpawnMode::Random => {
                    if digger_count == 0 {
                        drunk_x = self.starting_position.x;
                        drunk_y = self.starting_position.y;
                    } else {
                        drunk_x = rng.roll_dice(1, self.map.width - 3) + 1;
                        drunk_y = rng.roll_dice(1, self.map.height - 3) + 1;
                    }
                }
            }

            let mut drunk_health = self.settings.drunken_lifetime;

            while drunk_health > 0 {
                if self.map.get_tile(drunk_x, drunk_y) == TileType::Wall {
                    mutated_tiles = true;
                }

                paint(&mut self.map, self.settings.symmetry, self.settings.brush_size, drunk_x, drunk_y);
                self.map.set_tile(drunk_x, drunk_y, TileType::StairsDown);

                let stagger_direction = rng.roll_dice(1, 4);
                match stagger_direction {
                    1 => { if drunk_x > 2 { drunk_x -= 1; } }
                    2 => { if drunk_x < self.map.width - 2 { drunk_x += 1; } }
                    3 => { if drunk_y > 2 { drunk_y -= 1; } }
                    _ => { if drunk_y < self.map.height - 2 { drunk_y += 1; } }
                }

                drunk_health -= 1;
            }

            if mutated_tiles {
                self.take_snapshot();
                active_digger_count += 1;
            }

            digger_count += 1;
            for t in self.map.tiles.iter_mut() {
                if *t == TileType::StairsDown {
                    *t = TileType::Floor;
                }
            }

            floor_tile_count = self.map.tiles.iter().filter(|a| **a == TileType::Floor).count();
        }

        rltk::console::log(format!("{} dwarves gave up their sobriety, of whom {} actually found a wall.", digger_count, active_digger_count));

        let exit_tile = remove_unreachable_areas(&mut self.map, start_index);
        self.take_snapshot();

        self.map.tiles[exit_tile] = TileType::StairsDown;
        self.take_snapshot();

        self.noise_areas = generate_voronoi_spawn_regions(&self.map, &mut rng);
    }

}