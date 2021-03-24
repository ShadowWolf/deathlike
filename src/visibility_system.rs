use super::{Map, Player, Position, Viewshed};
use crate::{GameLog, Hidden, Name};
use rltk::{field_of_view, Point, RandomNumberGenerator};
use specs::prelude::*;

pub struct VisibilitySystem {}

impl<'a> System<'a> for VisibilitySystem {
    type SystemData = (
        WriteExpect<'a, Map>,
        Entities<'a>,
        WriteStorage<'a, Viewshed>,
        WriteStorage<'a, Position>,
        ReadStorage<'a, Player>,
        WriteStorage<'a, Hidden>,
        WriteExpect<'a, RandomNumberGenerator>,
        ReadStorage<'a, Name>,
        WriteExpect<'a, GameLog>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (mut map, entities, mut viewshed, pos, player, mut hidden, mut rng, names, mut log) =
            data;

        for (ent, viewshed, pos) in (&entities, &mut viewshed, &pos).join() {
            if viewshed.dirty {
                viewshed.dirty = false;

                viewshed.visible_tiles.clear();
                viewshed.visible_tiles =
                    field_of_view(Point::new(pos.x, pos.y), viewshed.range, &*map);
                viewshed
                    .visible_tiles
                    .retain(|p| p.x >= 0 && p.x < map.width && p.y >= 0 && p.y < map.height);

                let p: Option<&Player> = player.get(ent);
                if p.is_some() {
                    for t in map.visible_tiles.iter_mut() {
                        *t = false
                    }
                    for vis in viewshed.visible_tiles.iter() {
                        let idx = map.xy_idx(vis.x, vis.y);
                        map.revealed_tiles[idx] = true;
                        map.visible_tiles[idx] = true;

                        for t in map.tile_content[idx].iter() {
                            let hidden_item = hidden.get(*t);
                            if hidden_item.is_some() {
                                if rng.roll_dice(1, 24) == 1 {
                                    let name = names.get(*t);
                                    if let Some(name) = name {
                                        log.entries.push(format!("You spotted a {}", name.name))
                                    }
                                    hidden.remove(*t);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
