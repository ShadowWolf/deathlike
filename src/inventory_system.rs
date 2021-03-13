use crate::{
    AreaOfEffect, CombatStats, Confusion, Consumable, Entity, GameLog, InflictsDamage, Map, Name,
    ProvidesHealing, SufferDamage, WantsToUseItem,
};
use specs::prelude::*;
use specs::world::EntitiesRes;

pub struct UseItemSystem {}

impl<'a> System<'a> for UseItemSystem {
    #[allow(clippy::type_complexity)]
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
        ReadStorage<'a, AreaOfEffect>,
        WriteStorage<'a, Confusion>,
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
            aoe_items,
            mut confusion,
        ) = data;

        for (entity, item_to_use) in (&entities, &wants_to_use_item).join() {
            let mut used_item = false;

            let targets = self.determine_targets(&player_entity, &map, &aoe_items, item_to_use);

            used_item |= self.process_healing_actions(
                &*player_entity,
                &mut log,
                &names,
                &mut combat_stats,
                &healing_items,
                &entity,
                item_to_use,
                &targets,
            );

            used_item |= self.process_damage_actions(
                &*player_entity,
                &mut log,
                &names,
                &damaging_items,
                &mut suffer_damage,
                &entity,
                item_to_use,
                &targets,
            );

            used_item |= self.process_confusion_actions(
                &*player_entity,
                &mut log,
                &names,
                &mut confusion,
                &entity,
                item_to_use,
                &targets,
            );

            if used_item {
                self.process_consumables(&entities, &consumables, item_to_use)
            }
        }
    }
}

impl UseItemSystem {
    fn determine_targets(
        &self,
        player_entity: &Entity,
        map: &Map,
        aoe_items: &ReadStorage<AreaOfEffect>,
        item_to_use: &WantsToUseItem,
    ) -> Vec<Entity> {
        let mut targets: Vec<Entity> = Vec::new();

        match item_to_use.target {
            None => {
                targets.push(*player_entity);
            }
            Some(target) => {
                let aoe_item = aoe_items.get(item_to_use.item);
                match aoe_item {
                    None => {
                        let index = map.xy_idx(target.x, target.y);
                        for mob in map.tile_content[index].iter() {
                            targets.push(*mob);
                        }
                    }
                    Some(area_effect) => {
                        let mut blast_tiles =
                            rltk::field_of_view(target, area_effect.radius, &*map);

                        blast_tiles.retain(|p| {
                            p.x > 0 && p.x < map.width - 1 && p.y > 0 && p.y < map.height - 1
                        });

                        for tile_index in blast_tiles.iter() {
                            let index = map.xy_idx(tile_index.x, tile_index.y);
                            for mob in map.tile_content[index].iter() {
                                targets.push(*mob);
                            }
                        }
                    }
                }
            }
        }
        targets
    }

    fn process_damage_actions(
        &self,
        player_entity: &Entity,
        log: &mut GameLog,
        names: &ReadStorage<Name>,
        damaging_items: &ReadStorage<InflictsDamage>,
        mut suffer_damage: &mut WriteStorage<SufferDamage>,
        entity: &Entity,
        item_to_use: &WantsToUseItem,
        targets: &[Entity],
    ) -> bool {
        let damaging_item = damaging_items.get(item_to_use.item);
        let mut used_item = false;
        match damaging_item {
            None => {}
            Some(damage) => {
                for mob in targets.iter() {
                    SufferDamage::new_damage(&mut suffer_damage, *mob, damage.damage);
                    if entity == player_entity {
                        let player_name = names.get(*mob).unwrap();
                        let item_name = names.get(item_to_use.item).unwrap();
                        log.entries.push(format!(
                            "You use {} on {} and inflict {} damage",
                            player_name.name, item_name.name, damage.damage
                        ));
                    }
                    used_item = true;
                }
            }
        }
        used_item
    }

    fn process_consumables(
        &self,
        entities: &EntitiesRes,
        consumables: &ReadStorage<Consumable>,
        item_to_use: &WantsToUseItem,
    ) {
        let consumable = consumables.get(item_to_use.item);
        if consumable.is_some() {
            entities
                .delete(item_to_use.item)
                .expect("Unable to delete used item");
        }
    }

    fn process_healing_actions(
        &self,
        player_entity: &Entity,
        log: &mut GameLog,
        names: &ReadStorage<Name>,
        combat_stats: &mut WriteStorage<CombatStats>,
        healing_items: &ReadStorage<ProvidesHealing>,
        entity: &Entity,
        item_to_use: &WantsToUseItem,
        targets: &[Entity],
    ) -> bool {
        let mut used_item = false;
        let healing_item = healing_items.get(item_to_use.item);
        match healing_item {
            None => {}
            Some(heal) => {
                for target in targets.iter() {
                    let stats = combat_stats.get_mut(*target);
                    match stats {
                        None => {}
                        Some(s) => {
                            s.hp = i32::min(s.max_hp, s.hp + heal.heal_amount);
                            if entity == player_entity {
                                log.entries.push(format!(
                                    "You drink the {}, healing {} HP",
                                    names.get(item_to_use.item).unwrap().name,
                                    heal.heal_amount
                                ));
                            }
                            used_item = true;
                        }
                    }
                }
            }
        }
        used_item
    }
}

impl UseItemSystem {
    fn process_confusion_actions(
        &self,
        player_entity: &Entity,
        log: &mut GameLog,
        names: &ReadStorage<Name>,
        confusion: &mut WriteStorage<Confusion>,
        entity: &Entity,
        item_to_use: &WantsToUseItem,
        targets: &[Entity],
    ) -> bool {
        let mut confused_mobs = Vec::new();
        let causes_confusion = confusion.get(item_to_use.item);
        let mut used_item = false;
        match causes_confusion {
            None => {}
            Some(conf) => {
                for mob in targets.iter() {
                    confused_mobs.push((*mob, conf.turns));
                    if entity == player_entity {
                        let mob_name = names.get(*mob).unwrap();
                        let item_name = names.get(item_to_use.item).unwrap();
                        log.entries.push(format!(
                            "You confused {} by using {} on them",
                            mob_name.name, item_name.name
                        ));
                    }
                    used_item = true;
                }
            }
        }

        for (mob, turns) in confused_mobs.iter() {
            confusion
                .insert(*mob, Confusion { turns: *turns })
                .expect("Unable to insert confusion status");
        }

        used_item
    }
}
