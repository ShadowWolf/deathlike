use rltk::XpCell;
use specs::World;
use crate::{Map, Position, SHOW_MAPGEN_VISUALIZER, spawner, TileType};
use crate::map_builders::{build_snapshot, MapBuilder, prefab_levels};
use crate::map_builders::map_processing::remove_unreachable_areas;

#[derive(PartialEq, Clone)]
#[allow(dead_code)]
pub enum PrefabMode {
    RexLevel{ template: &'static str },
    Constant { level: prefab_levels::PrefablLevel },
}

pub struct PrefabBuilder {
    map: Map,
    starting_position: Position,
    depth: i32,
    history: Vec<Map>,
    mode: PrefabMode,
    spawns: Vec<(usize, String)>,
}

impl MapBuilder for PrefabBuilder {
    fn build_map(&mut self) {
        self.build();
    }

    fn spawn_entities(&mut self, ecs: &mut World) {
        for (location, entity_name) in self.spawns.iter() {
            assert!(self.map.tiles[location] == TileType::Floor);
            spawner::spawn_entity(ecs, &(location, entity_name));
        }
    }

    fn get_map(&self) -> Map {
        self.map.clone()
    }

    fn get_starting_position(&self) -> Position {
        self.starting_position.clone()
    }

    fn get_snapshot_history(&self) -> Vec<Map> {
        self.history.clone()
    }

    fn take_snapshot(&mut self) {
        match build_snapshot(&self.map) {
            None => {}
            Some(t) => self.history.push(t),
        };
    }
}

impl PrefabBuilder {
    pub fn new(new_depth: i32) -> PrefabBuilder {
        PrefabBuilder {
            map: Map::new(new_depth),
            starting_position: Position {x: 0, y: 0},
            depth: new_depth,
            history: Vec::new(),
            mode: PrefabMode::RexLevel { template: "../../resources/wfc-demo1.xp" },
            spawns: Vec::new(),
        }
    }

    #[allow(dead_code)]
    fn load_rex_map(&mut self, path: &str) {
        let xp_file = rltk::rex::XpFile::from_resource(path).unwrap();

        for layer in &xp_file.layers {
            for y in 0..layer.height {
                for x in 0..layer.width {
                    let cell = layer.get(x, y).unwrap();
                    if x < self.map.width as usize && y < self.map.height as usize {
                        let i = self.map.xy_idx(x as i32, y as i32);
                        self.parse_map_character(std::char::from_u32(cell.ch).unwrap(), i)
                    }
                }
            }
        }
    }

    #[allow(dead_code)]
    fn load_ascii_map(&mut self, level: &prefab_levels::PrefabLevel) {
        let mut string_vec: Vec<char> = level.template.chars().filter(|a| *a != '\r' && *a != '\n').collect();
        for c in string_vec.iter_mut() {
            if *c as u8 == 160u8 {
                *c = ' ';
            }
        }

        let mut i = 0;
        for ty in 0..level.height {
            for tx in 0..level.width {
                if tx < self.map.width as usize && ty < self.map.height as usize {
                    let i = self.map.xy_idx(tx as i32, ty as i32);
                    self.parse_map_character(string_vec[i], i);
                }
                i += 1;
            }
        }
    }

    fn parse_map_character(&mut self, cell: char, i: usize) {
        match cell {
            ' ' => self.map.tiles[i] = TileType::Floor,
            '#' => self.map.tiles[i] = TileType::Wall,
            '@' => {
                self.map.tiles[i] = TileType::Floor;
                self.starting_position = Position {
                    x: i as i32 % self.map.width,
                    y: y as i32 / self.map.width,
                }
            },
            '>' => self.map.tiles[i] = TileType::StairsDown,
            'g' => {
                self.map.tiles[i] = TileType::Floor;
                self.spawns.push((i, "Goblin".to_string()));
            },
            'o' => {
                self.map.tiles[i] = TileType::Floor;
                self.spawns.push((i, "Orc".to_string()));
            },
            '^' => {
                self.map.tiles[i] = TileType::Floor;
                self.spawns.push((i, "Bear Trap".to_string()));
            },
            '!' => {
                self.map.tiles[i] = TileType::Floor;
                self.spawns.push((i, "Health Potion".to_string()));
            }
            _ => {
                rltk::console::log(format!("Unknown glyph found while loading map: {}", (cell.ch as u8) as char))
            }
        }
    }

    fn build(&mut self) {
        match self.mode {
            PrefabMode::RexLevel {template} => self.load_rex_map(&template),
            PrefabMode::Constant {level} => self.load_ascii_map(&level),
        }

        if self.starting_position.x == 0 {
            self.starting_position = Position { x: self.map.width / 2, y: self.map.height / 2 };
            let mut start_index = self.map.xy_idx(self.starting_position.x, self.starting_position.y);
            while self.map.tiles[start_index] != TileType::Floor {
                self.starting_position.x -= 1;
                start_index = self.map.xy_idx(self.starting_position.x, self.starting_position.y);
            }

            self.take_snapshot();

            let exit_tile = remove_unreachable_areas(&mut self.map, start_index);
            self.take_snapshot();

            self.map.tiles[exit_tile] = TileType::StairsDown;
            self.take_snapshot();
        }

    }
}