use crate::{Map};

#[allow(dead_code)]
#[derive(PartialEq, Copy, Clone)]
pub enum Symmetry { None, Horizontal, Vertical, Both }

pub fn paint(map: &mut Map, mode: Symmetry, brush_size: i32, x: i32, y: i32) {
    match mode {
        Symmetry::None => {
            apply_paint(map, brush_size, x, y);
        }
        Symmetry::Horizontal => {
            let center_x = map.width / 2;
            if x == center_x {
                apply_paint(map, brush_size, x, y);
            } else {
                apply_horizontal_symmetry(map, brush_size, center_x, x, y);
            }
        }
        Symmetry::Vertical => {
            let center_y = map.height / 2;
            if y == center_y {
                apply_paint(map, brush_size, x, y);
            } else {
                apply_vertical_symmetry(map, brush_size, center_y, x, y);
            }
        }
        Symmetry::Both => {
            let center_x = map.width / 2;
            let center_y = map.height / 2;
            if y == center_y && x == center_x {
                apply_paint(map, brush_size, x, y);
            } else {
                let dist_x = i32::abs(center_x - x);
                let dist_y = i32::abs(center_y - y);

                apply_paint(map, brush_size, center_x + dist_x, center_y + dist_y);
                apply_paint(map, brush_size, center_x - dist_x, center_y - dist_y);
                apply_paint(map, brush_size, center_x - dist_x, center_y + dist_y);
                apply_paint(map, brush_size, center_x + dist_x, center_y - dist_y);
            }
        }
    }
}

fn apply_horizontal_symmetry(map: &mut Map, brush_size: i32, center_x: i32, x: i32, y: i32) {
    let d = i32::abs(center_x - x);
    apply_paint(map, brush_size, center_x + d, y);
    apply_paint(map, brush_size, center_x - d, y);
}

fn apply_vertical_symmetry(map: &mut Map, brush_size: i32, center_y: i32, x: i32, y: i32) {
    let d = i32::abs(center_y - y);
    apply_paint(map, brush_size, x, center_y + d);
    apply_paint(map, brush_size, x, center_y - d);
}

fn apply_paint(map: &mut Map, brush_size: i32, x: i32, y: i32) {
    match brush_size {
        1 => {
            map.set_floor(x, y);
        }
        _ => {
            let half_brush_size = brush_size / 2;
            for brush_y in y - half_brush_size .. y + half_brush_size {
                for brush_x in x - half_brush_size .. x + half_brush_size {
                    if brush_x > 1 && brush_x < map.width - 1 && brush_y > 1 && brush_y < map.height - 1 {
                        map.set_floor(brush_x, brush_y);
                    }
                }
            }
        }
    }
}