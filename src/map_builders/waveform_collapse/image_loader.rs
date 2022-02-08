use rltk::rex::XpFile;
use crate::{Map};

pub fn load_rex_map(new_depth: i32, xp_file: &XpFile) -> Map {
    let mut map: Map = Map::new(new_depth);

    for layer in &xp_file.layers {
        for y in 0..layer.height {
            for x in 0..layer.width {
                let cell = layer.get(x, y).unwrap();
                if x < map.width as usize && y < map.height as usize {
                    match cell.ch {
                        32 => map.set_floor(x as i32, y as i32), // space character
                        35 => map.set_wall(x as i32, y as i32), // # character
                        _ => {}
                    }
                }
            }
        }
    }

    map
}