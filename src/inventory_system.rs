use super::{CombatStats, Entity, GameLog, Name, WantsToUseItem};
use crate::{Consumable, InflictsDamage, Map, ProvidesHealing, SufferDamage};
use specs::prelude::*;

pub struct ItemUseSystem {}

impl<'a> System<'a> for ItemUseSystem {
    type SystemData = (
        ReadExpect<'a, Entity>,
        WriteExpect<'a, GameLog>,
        Entities<'a>,
        WriteStorage<'a, WantsToUseItem>,
        ReadStorage<'a, Name>,
        WriteStorage<'a, CombatStats>,
        ReadStorage<'a, Consumable>,
        ReadStorage<'a, ProvidesHealing>,
        ReadStorage<'a, InflictsDamage>,
        ReadExpect<'a, Map>,
        WriteStorage<'a, SufferDamage>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            player_entity,
            mut log,
            entities,
            wants_to_use_item,
            names,
            mut combat_stats,
            consumables,
            healing_items,
            damaging_items,
            map,
            mut suffer_damage,
        ) = data;

        for (entity, item_to_use, stats) in
            (&entities, &wants_to_use_item, &mut combat_stats).join()
        {
            let healing_item = healing_items.get(item_to_use.item);
            match healing_item {
                None => {}
                Some(heal) => {
                    stats.hp = i32::min(stats.max_hp, stats.hp + heal.heal_amount);
                    if entity == *player_entity {
                        log.entries.push(format!(
                            "You drink the {}, healing {} HP",
                            names.get(item_to_use.item).unwrap().name,
                            heal.heal_amount
                        ));
                    }
                }
            }

            let damaging_item = damaging_items.get(item_to_use.item);
            match damaging_item {
                None => {}
                Some(damage) => {
                    let target_point = item_to_use.target.unwrap();
                    let index = map.xy_idx(target_point.x, target_point.y);

                    let mobs: Vec<_> = map.tile_content[index].iter().collect();
                    if mobs.is_empty() {
                        let item_name = names.get(item_to_use.item).unwrap();
                        log.entries
                            .push(format!("You cast {} at the darkness!", item_name.name));
                    } else {
                        for mob in mobs {
                            SufferDamage::new_damage(&mut suffer_damage, *mob, damage.damage);
                            if entity == *player_entity {
                                let mob_name = names.get(*mob).unwrap();
                                let item_name = names.get(item_to_use.item).unwrap();
                                log.entries.push(format!(
                                    "You use {} on {}, inflicting {} damage",
                                    item_name.name, mob_name.name, damage.damage
                                ));
                            }
                        }
                    }
                }
            }

            let consumable = consumables.get(item_to_use.item);
            if consumable.is_some() {
                entities
                    .delete(item_to_use.item)
                    .expect("Unable to delete used item");
            }
        }
    }
}
