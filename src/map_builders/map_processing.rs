use crate::{Map, TileType};
use std::collections::HashMap;

pub fn  remove_unreachable_areas(map: &mut Map, start_index: usize) -> usize {
    map.populate_blocked();
    let map_starts: Vec<usize> = vec![start_index];
    let dijkstra_map = rltk::DijkstraMap::new(map.width as usize, map.height as usize, &map_starts, map, 200.0);
    let mut exit_tile = (0, 0.0f32);
    for (i, tile) in map.tiles.iter_mut().enumerate() {
        if *tile == TileType::Floor {
            let distance_to_start = dijkstra_map.map[i];
            if distance_to_start == f32::MAX {
                *tile = TileType::Wall;
            } else if distance_to_start > exit_tile.1 {
                exit_tile.0 = i;
                exit_tile.1 = distance_to_start;
            }
        }
    }

    exit_tile.0
}

#[allow(clippy::map_entry)]
pub fn generate_voronoi_spawn_regions(map: &Map, rng: &mut rltk::RandomNumberGenerator) -> HashMap<i32, Vec<usize>> {
    let mut noise_areas: HashMap<i32, Vec<usize>> = HashMap::new();
    let mut noise = rltk::FastNoise::seeded(rng.roll_dice(1, 65536) as u64);
    noise.set_noise_type(rltk::NoiseType::Cellular);
    noise.set_frequency(0.08);
    noise.set_cellular_distance_function(rltk::CellularDistanceFunction::Manhattan);

    for y in 1..map.height - 1 {
        for x in 1..map.width - 1 {
            let i = map.xy_idx(x, y);
            if map.tiles[i] == TileType::Floor {
                let cv = noise.get_noise(x as f32, y as f32) * 10240.0;
                let cell_value = cv as i32;

                if noise_areas.contains_key(&cell_value) {
                    noise_areas.get_mut(&cell_value).unwrap().push(i);
                } else {
                    noise_areas.insert(cell_value, vec![i]);
                }
            }
        }
    }
    noise_areas
}