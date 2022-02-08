use std::collections::HashSet;
use rltk::RandomNumberGenerator;
use crate::Map;
use crate::map_builders::waveform_collapse::map_chunk::MapChunk;

pub struct Solver {
    constraints: Vec<MapChunk>,
    chunk_size: i32,
    chunks: Vec<Option<usize>>,
    chunks_x: usize,
    chunks_y: usize,
    remaining: Vec<(usize, i32)>,
    pub possible: bool
}

impl Solver {
    pub fn new(constraints: Vec<MapChunk>, chunk_size: i32, map: &Map) -> Solver {
        let chunks_x = (map.width / chunk_size) as usize;
        let chunks_y = (map.height / chunk_size) as usize;
        let mut remaining: Vec<(usize, i32)> = Vec::new();

        for i in 0..(chunks_x * chunks_y) {
            remaining.push((i, 0));
        }

        Solver {
            constraints,
            chunk_size,
            chunks: vec![None; chunks_x * chunks_y],
            chunks_x,
            chunks_y,
            remaining,
            possible: true
        }
    }

    fn chunk_index(&self, x: usize, y: usize) -> usize {
        ((y * self.chunks_x) + x) as usize
    }

    fn identify_chunk(&self, index: usize) -> i32 {
        match self.chunks[index] {
            None => 0,
            Some(_) => 1
        }
    }

    fn count_neighbors(&self, chunk_x: usize, chunk_y: usize) -> i32 {
        let mut neighbors = 0;

        if chunk_x > 0 {
            let left = self.chunk_index(chunk_x - 1, chunk_y);
            neighbors += self.identify_chunk(left);
        }

        if chunk_x < self.chunks_x - 1 {
            let right = self.chunk_index(chunk_x + 1, chunk_y);
            neighbors += self.identify_chunk(right);
        }

        if chunk_y > 0 {
            let up = self.chunk_index(chunk_x, chunk_y - 1);
            neighbors += self.identify_chunk(up);
        }

        if chunk_y < self.chunks_y - 1 {
            let down = self.chunk_index(chunk_x, chunk_y + 1);
            neighbors += self.identify_chunk(down);
        }

        neighbors
    }

    pub fn iteration(&mut self, map: &mut Map, rng: &mut RandomNumberGenerator) -> bool {
        if self.remaining.is_empty() { return true; }

        let mut remain_copy = self.remaining.clone();
        let has_neighbors = self.populate_neighbors_for_remaining_items(&mut remain_copy);

        self.remaining = remain_copy;

        let remaining_index = if !has_neighbors {
            (rng.roll_dice(1, self.remaining.len() as i32) - 1) as usize
        } else {
            0usize
        };

        let chunk_index = self.remaining[remaining_index].0;
        self.remaining.remove(remaining_index);

        let chunk_x = chunk_index % self.chunks_x;
        let chunk_y = chunk_index / self.chunks_x;

        let mut options: Vec<Vec<usize>> = Vec::new();

        let total_neighbors = self.populate_constraint_options_for_chunk(chunk_x, chunk_y, &mut options);

        if total_neighbors == 0 {
            let new_chunk_index = (rng.roll_dice(1, self.constraints.len() as i32) - 1) as usize;
            self.chunks[chunk_index] = Some(new_chunk_index);
            self.populate_chunk_constraints(map, chunk_x, chunk_y, new_chunk_index)
        } else {
            let mut options_to_check = HashSet::new();

            for o in options.iter() {
                for i in o.iter() {
                    options_to_check.insert(*i);
                }
            }

            let mut possible_options = Vec::new();
            for new_chunk_index in options_to_check.iter() {
                let possible = options.iter().any(|o| o.contains(new_chunk_index));
                if possible {
                    possible_options.push(*new_chunk_index);
                }
            }

            if possible_options.is_empty() {
                rltk::console::log("This chunk is not possible!");
                self.possible = false;
                return true;
            } else {
                let new_chunk_index = if possible_options.len() == 1 { 0 } else {
                    rng.roll_dice(1, possible_options.len() as i32) - 1
                } as usize;

                self.chunks[chunk_index] = Some(new_chunk_index);
                self.populate_chunk_constraints(map, chunk_x, chunk_y, new_chunk_index);
            }
        }

        false
    }

    fn populate_constraint_options_for_chunk(&mut self, chunk_x: usize, chunk_y: usize, options: &mut Vec<Vec<usize>>) -> i32 {
        let mut total_neighbors = 0;
        if chunk_x > 0 {
            let left = self.chunk_index(chunk_x - 1, chunk_y);
            match self.chunks[left] {
                None => {}
                Some(t) => {
                    total_neighbors += 1;
                    options.push(self.constraints[t].compatible_with[3].clone());
                }
            }
        }

        if chunk_x < self.chunks_x - 1 {
            let right = self.chunk_index(chunk_x + 1, chunk_y);
            match self.chunks[right] {
                None => {}
                Some(t) => {
                    total_neighbors += 1;
                    options.push(self.constraints[t].compatible_with[2].clone());
                }
            }
        }

        if chunk_y > 0 {
            let up = self.chunk_index(chunk_x, chunk_y - 1);
            match self.chunks[up] {
                None => {}
                Some(t) => {
                    total_neighbors += 1;
                    options.push(self.constraints[t].compatible_with[1].clone());
                }
            }
        }

        if chunk_y < self.chunks_y - 1 {
            let down = self.chunk_index(chunk_x, chunk_y + 1);
            match self.chunks[down] {
                None => {}
                Some(t) => {
                    total_neighbors += 1;
                    options.push(self.constraints[t].compatible_with[0].clone());
                }
            }
        }

        total_neighbors
    }

    fn populate_neighbors_for_remaining_items(&mut self, remain_copy: &mut Vec<(usize, i32)>) -> bool {
        let mut has_neighbors = false;

        for r in remain_copy.iter_mut() {
            let i = r.0;
            let chunk_x = i % self.chunks_x;
            let chunk_y = i / self.chunks_x;
            let neighbors = self.count_neighbors(chunk_x, chunk_y);
            if neighbors > 0 {
                has_neighbors = true;
            }

            *r = (i, neighbors);
        }

        remain_copy.sort_by(|a, b| b.1.cmp(&a.1));
        has_neighbors
    }

    fn populate_chunk_constraints(&mut self, map: &mut Map, chunk_x: usize, chunk_y: usize, new_chunk_index: usize) {
        let cs = self.chunk_size as i32;
        let cx = chunk_x as i32;
        let cy = chunk_y as i32;

        let left = cx * cs;
        let right = (cx + 1) * cs;
        let top = cy * cs;
        let bottom = (cy + 1) * cs;

        let mut i = 0usize;
        for y in top..bottom {
            for x in left..right {
                let map_index = map.xy_idx(x, y);
                let tile = self.constraints[new_chunk_index].pattern[i];
                map.tiles[map_index] = tile;
                i += 1;
            }
        }
    }
}

