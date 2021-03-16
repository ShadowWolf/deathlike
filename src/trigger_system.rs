use crate::{
    EntityMoved, EntryTrigger, GameLog, Hidden, InflictsDamage, Map, Name, ParticleBuilder,
    Position, SingleActivation, SufferDamage,
};
use rltk::RGB;
use specs::prelude::*;

pub struct TriggerSystem {}

impl<'a> System<'a> for TriggerSystem {
    type SystemData = (
        ReadExpect<'a, Map>,
        WriteStorage<'a, EntityMoved>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, EntryTrigger>,
        WriteStorage<'a, Hidden>,
        ReadStorage<'a, Name>,
        Entities<'a>,
        WriteExpect<'a, GameLog>,
        ReadStorage<'a, InflictsDamage>,
        WriteExpect<'a, ParticleBuilder>,
        WriteStorage<'a, SufferDamage>,
        ReadStorage<'a, SingleActivation>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            map,
            mut entity_moved,
            position,
            entry_triggers,
            mut hidden_items,
            names,
            entities,
            mut log,
            inflicts_damage,
            mut particle_builder,
            mut suffer_damage,
            single_activations,
        ) = data;

        let mut remove_activated_items: Vec<Entity> = Vec::new();
        for (entity, mut _entity_moved, pos) in (&entities, &entity_moved, &position).join() {
            let i = map.xy_idx(pos.x, pos.y);
            for entity_id in map.tile_content[i].iter() {
                if entity != *entity_id {
                    let trigger_option = entry_triggers.get(*entity_id);
                    if trigger_option.is_some() {
                        let name = names.get(*entity_id);
                        if let Some(name) = name {
                            log.entries.push(format!("{} triggers!", name.name));
                        }

                        let damage = inflicts_damage.get(*entity_id);
                        if let Some(damage) = damage {
                            particle_builder.request(
                                pos.x,
                                pos.y,
                                RGB::named(rltk::ORANGE),
                                RGB::named(rltk::BLACK),
                                rltk::to_cp437('‼'),
                                200.0,
                            );

                            SufferDamage::new_damage(&mut suffer_damage, entity, damage.damage);
                        }

                        let single_act = single_activations.get(*entity_id);
                        if single_act.is_some() {
                            remove_activated_items.push(*entity_id);
                        }

                        hidden_items.remove(*entity_id);
                    }
                }
            }
        }

        for sa in remove_activated_items.iter() {
            entities
                .delete(*sa)
                .expect("unable to delete single trigger action");
        }

        entity_moved.clear();
    }
}
