use super::{Map, Monster, Position, Viewshed, RunState};
use rltk::{Point};
use specs::prelude::*;
use crate::WantsToMelee;

pub struct MonsterAI {}

impl<'a> System<'a> for MonsterAI {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        WriteExpect<'a, Map>,
        ReadExpect<'a, Point>,
        ReadExpect<'a, Entity>,
        ReadExpect<'a, RunState>,
        Entities<'a>,
        WriteStorage<'a, Viewshed>,
        ReadStorage<'a, Monster>,
        WriteStorage<'a, Position>,
        WriteStorage<'a, WantsToMelee>
    );

    fn run(&mut self, data: Self::SystemData) {
        let (mut map, player_pos, player_entity, run_state, entities, mut viewshed, monster, mut position, mut wants_to_melee) = data;

        if *run_state != RunState::MonsterTurn {
            return;
        }

        for (entity, mut viewshed, _monster, mut pos) in (&entities, &mut viewshed, &monster, &mut position).join() {
            let distance = rltk::DistanceAlg::Pythagoras.distance2d(Point::new(pos.x, pos.y), *player_pos);
            if distance < 1.5 {
                wants_to_melee.insert(entity, WantsToMelee { target: *player_entity }).expect("Unable to attack");
            }
            else if viewshed.visible_tiles.contains(&*player_pos) {
                let path = rltk::a_star_search(map.xy_idx(pos.x, pos.y), map.xy_idx(player_pos.x, player_pos.y), &*map);
                if path.success && path.steps.len() > 1 {
                    let idx = map.xy_idx(pos.x, pos.y);
                    map.blocked[idx] = false;
                    pos.x = path.steps[1] as i32 % map.width;
                    pos.y = path.steps[1] as i32 / map.width;

                    let new_idx = map.xy_idx(pos.x, pos.y);
                    map.blocked[new_idx] = true;
                    viewshed.dirty = true;
                }
            }
        }
    }
}
