use super::{BlocksTile, Map, Position};
use specs::prelude::*;

pub struct MapIndexingSystem {}

impl<'a> System<'a> for MapIndexingSystem {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        WriteExpect<'a, Map>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, BlocksTile>,
        Entities<'a>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (mut map, position, blockers, entities) = data;

        map.populate_blocked();
        map.clear_content_index();

        for (entity, position) in (&entities, &position).join() {
            let idx = map.xy_idx(position.x, position.y);

            let p: Option<&BlocksTile> = blockers.get(entity);
            if p.is_some() {
                map.blocked[idx] = true;
            }

            map.tile_content[idx].push(entity);
        }
    }
}
