use super::{Entity, GameLog, InBackpack, Name, Position, WantsToDropItem};
use specs::prelude::*;

pub struct ItemDropSystem {}

impl ItemDropSystem {
    fn get_drop_position(&self, dropped_pos: &Position) -> Position {
        Position {
            x: dropped_pos.x,
            y: dropped_pos.y,
        }
    }
}

impl<'a> System<'a> for ItemDropSystem {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        ReadExpect<'a, Entity>,
        WriteExpect<'a, GameLog>,
        Entities<'a>,
        WriteStorage<'a, WantsToDropItem>,
        ReadStorage<'a, Name>,
        WriteStorage<'a, Position>,
        WriteStorage<'a, InBackpack>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            player_entity,
            mut log,
            entities,
            mut wants_to_drop_item,
            names,
            mut positions,
            mut backpack,
        ) = data;

        for (entity, to_drop) in (&entities, &wants_to_drop_item).join() {
            let entity_position = positions.get(entity).unwrap();
            let drop_position = self.get_drop_position(entity_position);
            positions
                .insert(to_drop.item, drop_position)
                .expect("Unable to insert drop action");
            backpack
                .remove(to_drop.item)
                .expect("Unable to remove item from backpack");

            if entity == *player_entity {
                log.entries.push(format!(
                    "You drop the {}.",
                    names.get(to_drop.item).unwrap().name
                ));
            }
        }

        wants_to_drop_item.clear();
    }
}
