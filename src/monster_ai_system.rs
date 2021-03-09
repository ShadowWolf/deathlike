use super::{Map, Monster, Name, Position, Viewshed};
use rltk::{console, field_of_view, Point};
use specs::prelude::*;

pub struct MonsterAI {}

impl<'a> System<'a> for MonsterAI {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        WriteExpect<'a, Map>,
        WriteStorage<'a, Viewshed>,
        ReadExpect<'a, Point>,
        ReadStorage<'a, Monster>,
        ReadStorage<'a, Name>,
        WriteStorage<'a, Position>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (mut map, mut viewshed, player_position, monster, name, mut position) = data;

        for (mut viewshed, _monster, name, mut position) in
            (&mut viewshed, &monster, &name, &mut position).join()
        {
            let distance = rltk::DistanceAlg::Pythagoras
                .distance2d(Point::new(position.x, position.y), *player_position);
            if distance < 1.5 {
                console::log(&format!("{} shouts insults", name.name));
                return;
            }

            if viewshed.visible_tiles.contains(&*player_position) {
                let path = rltk::a_star_search(
                    map.xy_idx(position.x, position.y) as i32,
                    map.xy_idx(player_position.x, player_position.y) as i32,
                    &mut *map,
                );
                if path.success && path.steps.len() > 1 {
                    position.x = path.steps[1] as i32 % map.width;
                    position.y = path.steps[1] as i32 / map.width;
                    viewshed.dirty = true;
                }
            }
        }
    }
}
