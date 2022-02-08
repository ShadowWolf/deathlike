use std::collections::HashSet;
use crate::{TileType, Map};
use crate::map_builders::waveform_collapse::map_chunk::{MapChunk, tile_index_in_chunk};

pub fn build_patterns(map: &Map, chunk_size: i32, include_flipping: bool, dedupe: bool) -> Vec<Vec<TileType>> {
    let chunks_x = map.width / chunk_size;
    let chunks_y = map.height / chunk_size;
    let mut patterns = Vec::new();

    for cy in 0..chunks_y {
        for cx in 0..chunks_x {
            let mut pattern = Vec::new();
            let start_x = cx * chunk_size;
            let end_x = (cx + 1) * chunk_size;
            let start_y = cy * chunk_size;
            let end_y = (cy + 1) * chunk_size;

            for y in start_y..end_y {
                for x in start_x..end_x {
                    let i = map.xy_idx(x, y);
                    pattern.push(map.tiles[i]);
                }
            }

            patterns.push(pattern);

            if include_flipping {
                // Horizontal flipping
                {
                    let hp = flip_horizontally(map, start_x, end_x, start_y, end_y);
                    patterns.push(hp);
                }

                // Vertical Flipping
                {
                    let vp = flip_vertically(map, start_x, end_x, start_y, end_y);
                    patterns.push(vp);
                }

                // Flip Horizontally and Vertically
                {
                    let bp = flip_horizontally_and_vertically(map, start_x, end_x, start_y, end_y);
                    patterns.push(bp);
                }

            }
        }
    }

    if dedupe {
        rltk::console::log(format!("There are {} patterns before dedupe", patterns.len()));
        let unique_patterns: HashSet<Vec<TileType>> = patterns.drain(..).collect();
        patterns.extend(unique_patterns.into_iter());
        rltk::console::log(format!("There are {} patterns after dedupe", patterns.len()));
    }

    patterns
}

fn flip_horizontally_and_vertically(map: &Map, start_x: i32, end_x: i32, start_y: i32, end_y: i32) -> Vec<TileType> {
    let mut bp = Vec::new();
    for y in start_y..end_y {
        for x in start_x..end_x {
            let i = map.xy_idx(end_x - (x + 1), end_y - (y + 1));
            bp.push(map.tiles[i]);
        }
    }
    bp
}

fn flip_horizontally(map: &Map, start_x: i32, end_x: i32, start_y: i32, end_y: i32) -> Vec<TileType> {
    let mut hp = Vec::new();
    for y in start_y..end_y {
        for x in start_x..end_x {
            let i = map.xy_idx(end_x - (x + 1), y);
            hp.push(map.tiles[i]);
        }
    }
    hp
}

fn flip_vertically(map: &Map, start_x: i32, end_x: i32, start_y: i32, end_y: i32) -> Vec<TileType> {
    let mut vp = Vec::new();
    for y in start_y..end_y {
        for x in start_x..end_x {
            let i = map.xy_idx(x, end_y - (y + 1));
            vp.push(map.tiles[i]);
        }
    }
    vp
}

pub fn render_pattern_to_map(map: &mut Map, chunk: &MapChunk, chunk_size: i32, start_x: i32, start_y: i32) {
    let mut i = 0usize;
    for tile_y in 0..chunk_size {
        for tile_x in 0..chunk_size {
            let map_index = map.xy_idx(start_x + tile_x, start_y + tile_y);
            map.tiles[map_index] = chunk.pattern[i];
            map.visible_tiles[map_index] = true;
            i += 1;
        }
    }

    for exit_count in 0..3 {
        highlight_exits(map, chunk, chunk_size, start_x, start_y, exit_count);
    }
}

fn highlight_exits(map: &mut Map, chunk: &MapChunk, chunk_size: i32, start_x: i32, start_y: i32, exit_count: usize) {
    for (x, dir) in chunk.exits[exit_count].iter().enumerate() {
        if *dir {
            let x_pos = match exit_count {
                0 | 1 => start_x + x as i32,
                2 => start_x,
                _ => start_x + chunk_size - 1
            };
            let y_pos = match exit_count {
                0 => start_y,
                1 => start_y + chunk_size - 1,
                _ => start_y + x as i32,
            };

            let index = map.xy_idx(x_pos, y_pos);
            map.tiles[index] = TileType::StairsDown;
        }
    }
}

pub fn patterns_to_constraints(patterns: Vec<Vec<TileType>>, chunk_size: i32) -> Vec<MapChunk> {
    let mut constraints: Vec<MapChunk> = Vec::new();

    for p in patterns {
        let mut new_chunk = MapChunk {
            pattern: p,
            exits: [ Vec::new(), Vec::new(), Vec::new(), Vec::new()],
            has_exits: true,
            compatible_with: [Vec::new(), Vec::new(), Vec::new(), Vec::new()]
        };
        for exit in new_chunk.exits.iter_mut() {
            for _i in 0..chunk_size {
                exit.push(false);
            }
        }

        for x in 0..chunk_size {
            populate_chunk(&mut new_chunk, chunk_size, x);
        }

        constraints.push(new_chunk);
    }

    let ch = constraints.clone();
    for c in constraints.iter_mut() {
        for (j, potential) in ch.iter().enumerate() {
            if !c.has_exits || !potential.has_exits {
                for compat in c.compatible_with.iter_mut() {
                    compat.push(j);
                }
            } else {
                for (direction, exits) in c.exits.iter_mut().enumerate() {
                    let opposite = match direction {
                        0 => 1,
                        1 => 0,
                        2 => 3,
                        _ => 2
                    };

                    let mut is_fit = false;
                    let mut has_any_exit = false;
                    for (slot, can_enter) in exits.iter().enumerate() {
                        if *can_enter {
                            has_any_exit = true;
                            if potential.exits[opposite][slot] {
                                is_fit = true;
                            }
                        }
                    }

                    if is_fit {
                        c.compatible_with[direction].push(j);
                    }

                    if !has_any_exit {
                        let matching_exit_count = potential.exits[opposite].iter().filter(|a| !**a).count();
                        if matching_exit_count == 0 {
                            c.compatible_with[direction].push(j);
                        }
                    }
                }
            }
        }
    }

    constraints
}

fn populate_chunk(chunk: &mut MapChunk, chunk_size: i32, x: i32) {
    let mut exit_count = 0;
    if identify_exit(chunk, chunk_size, x, 0, 0) {
        exit_count += 1;
    }

    if identify_exit(chunk, chunk_size, x, chunk_size - 1, 1) {
        exit_count += 1;
    }

    if identify_exit(chunk, chunk_size, 0, x, 2) {
        exit_count += 1;
    }

    if identify_exit(chunk, chunk_size, chunk_size - 1, x, 3) {
        exit_count += 1;
    }

    chunk.has_exits = exit_count > 0;
}

fn identify_exit(chunk: &mut MapChunk, chunk_size: i32, x: i32, y: i32, exit_number: usize) -> bool {
    let index = tile_index_in_chunk(chunk_size, x, y);
    if chunk.pattern[index] == TileType::Floor {
        chunk.exits[exit_number][x as usize] = true;
        true
    } else {
        false
    }
}