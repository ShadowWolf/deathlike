use crate::{
    AreaOfEffect, CombatStats, Confusion, Consumable, Entity, EquipmentSlot, Equippable, Equipped,
    GameLog, InBackpack, InflictsDamage, MagicMapper, Map, Name, ParticleBuilder, Position,
    ProvidesHealing, RunState, SufferDamage, WantsToRemoveItem, WantsToUseItem,
};
use rltk::RGB;
use specs::prelude::*;
use specs::world::EntitiesRes;

pub struct UseItemSystem {}

impl UseItemSystem {
    fn process_magic_map_actions(
        &self,
        log: &mut WriteExpect<GameLog>,
        item_to_use: &WantsToUseItem,
        magic_mappers: &ReadStorage<MagicMapper>,
        run_state: &mut WriteExpect<RunState>,
    ) -> bool {
        let is_mapper = magic_mappers.get(item_to_use.item);
        if is_mapper.is_some() {
            log.entries.push("All is revealed to you!".to_string());
            **run_state = RunState::MagicMapReveal { row: 0 };

            return true;
        }

        false
    }
}

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
        ReadStorage<'a, Equippable>,
        WriteStorage<'a, Equipped>,
        WriteStorage<'a, InBackpack>,
        WriteExpect<'a, ParticleBuilder>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, MagicMapper>,
        WriteExpect<'a, RunState>,
    );

    #[allow(clippy::cognitive_complexity)]
    fn run(&mut self, data: Self::SystemData) {
        let (
            player_entity,
            mut log,
            entities,
            mut wants_to_use_item,
            names,
            mut combat_stats,
            consumables,
            healing_items,
            damaging_items,
            map,
            mut suffer_damage,
            aoe_items,
            mut confusion,
            equippable,
            mut equipped,
            mut backpack,
            mut particle_builder,
            positions,
            magic_mappers,
            mut run_state,
        ) = data;

        for (entity, item_to_use) in (&entities, &wants_to_use_item).join() {
            let mut used_item = false;

            let targets = self.determine_targets(
                &player_entity,
                &map,
                &aoe_items,
                item_to_use,
                &positions,
                &mut particle_builder,
            );

            self.process_equip_actions(
                &player_entity,
                &mut log,
                &entities,
                &names,
                &equippable,
                &mut equipped,
                &mut backpack,
                item_to_use,
                &targets,
            );

            used_item |= self.process_magic_map_actions(
                &mut log,
                item_to_use,
                &magic_mappers,
                &mut run_state,
            );

            used_item |= self.process_healing_actions(
                &player_entity,
                &mut log,
                &names,
                &mut combat_stats,
                &healing_items,
                &entity,
                item_to_use,
                &targets,
                &positions,
                &mut particle_builder,
            );

            used_item |= self.process_damage_actions(
                &player_entity,
                &mut log,
                &names,
                &damaging_items,
                &mut suffer_damage,
                &entity,
                item_to_use,
                &targets,
                &positions,
                &mut particle_builder,
            );

            used_item |= self.process_confusion_actions(
                &player_entity,
                &mut log,
                &names,
                &mut confusion,
                &entity,
                item_to_use,
                &targets,
                &positions,
                &mut particle_builder,
            );

            if used_item {
                self.process_consumables(&entities, &consumables, item_to_use)
            }
        }

        wants_to_use_item.clear();
    }
}

impl UseItemSystem {
    fn determine_targets(
        &self,
        player_entity: &Entity,
        map: &Map,
        aoe_items: &ReadStorage<AreaOfEffect>,
        item_to_use: &WantsToUseItem,
        positions: &ReadStorage<Position>,
        particle_builder: &mut ParticleBuilder,
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
                                let pos = positions.get(*mob);
                                if let Some(pos) = pos {
                                    particle_builder.request(
                                        pos.x,
                                        pos.y,
                                        RGB::named(rltk::ORANGE),
                                        RGB::named(rltk::BLACK),
                                        rltk::to_cp437('░'),
                                        200.0,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        targets
    }

    #[allow(clippy::too_many_arguments)]
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
        positions: &ReadStorage<Position>,
        particle_builder: &mut ParticleBuilder,
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

                    let pos = positions.get(*mob);
                    if let Some(pos) = pos {
                        particle_builder.request(
                            pos.x,
                            pos.y,
                            RGB::named(rltk::RED),
                            RGB::named(rltk::BLACK),
                            rltk::to_cp437('‼'),
                            200.0,
                        );
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

    #[allow(clippy::too_many_arguments)]
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
        positions: &ReadStorage<Position>,
        particle_builder: &mut ParticleBuilder,
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

                            let pos = positions.get(*target);
                            if let Some(pos) = pos {
                                particle_builder.request(
                                    pos.x,
                                    pos.y,
                                    RGB::named(rltk::GREEN),
                                    RGB::named(rltk::BLACK),
                                    rltk::to_cp437('♥'),
                                    200.0,
                                );
                            }
                        }
                    }
                }
            }
        }
        used_item
    }
}

impl UseItemSystem {
    #[allow(clippy::too_many_arguments)]
    fn process_confusion_actions(
        &self,
        player_entity: &Entity,
        log: &mut GameLog,
        names: &ReadStorage<Name>,
        confusion: &mut WriteStorage<Confusion>,
        entity: &Entity,
        item_to_use: &WantsToUseItem,
        targets: &[Entity],
        positions: &ReadStorage<Position>,
        particle_builder: &mut ParticleBuilder,
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

                    let pos = positions.get(*mob);
                    if let Some(pos) = pos {
                        particle_builder.request(
                            pos.x,
                            pos.y,
                            RGB::named(rltk::MAGENTA),
                            RGB::named(rltk::BLACK),
                            rltk::to_cp437('?'),
                            200.0,
                        );
                    }
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

pub struct ItemRemoveSystem {}

impl<'a> System<'a> for ItemRemoveSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, WantsToRemoveItem>,
        WriteStorage<'a, Equipped>,
        WriteStorage<'a, InBackpack>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (entities, mut remove_item, mut equipped, mut backpack) = data;

        for (entity, to_remove) in (&entities, &remove_item).join() {
            equipped.remove(to_remove.item);
            backpack
                .insert(to_remove.item, InBackpack { owner: entity })
                .expect("unable to place item in backpack");
        }

        remove_item.clear();
    }
}

impl UseItemSystem {
    fn find_items_to_unequip(
        &self,
        player_entity: &Entity,
        log: &mut WriteExpect<GameLog>,
        entities: &Entities,
        names: &ReadStorage<Name>,
        equipped: &mut WriteStorage<Equipped>,
        target_slot: EquipmentSlot,
        target: Entity,
    ) -> Vec<Entity> {
        let mut unequip: Vec<Entity> = Vec::new();
        for (item_entity, already_equipped, name) in (entities, equipped, names).join() {
            if already_equipped.owner == target && already_equipped.slot == target_slot {
                unequip.push(item_entity);
                if target == *player_entity {
                    log.entries.push(format!("You unequip {}", name.name));
                }
            }
        }
        unequip
    }
}

impl UseItemSystem {
    fn process_equip_actions(
        &mut self,
        player_entity: &Entity,
        mut log: &mut WriteExpect<GameLog>,
        entities: &Entities,
        names: &ReadStorage<Name>,
        equippable: &ReadStorage<Equippable>,
        mut equipped: &mut WriteStorage<Equipped>,
        backpack: &mut WriteStorage<InBackpack>,
        item_to_use: &WantsToUseItem,
        targets: &[Entity],
    ) {
        let item_equippable = equippable.get(item_to_use.item);
        match item_equippable {
            None => {}
            Some(equip) => {
                let target_slot = equip.slot;
                let target = targets[0];

                let unequip = self.find_items_to_unequip(
                    &player_entity,
                    &mut log,
                    &entities,
                    &names,
                    &mut equipped,
                    target_slot,
                    target,
                );

                for item in unequip.iter() {
                    equipped.remove(*item);
                    backpack
                        .insert(*item, InBackpack { owner: target })
                        .expect("unable to unequip item for equip");
                }

                equipped
                    .insert(
                        item_to_use.item,
                        Equipped {
                            owner: target,
                            slot: target_slot,
                        },
                    )
                    .expect("unable to equip item");
                backpack.remove(item_to_use.item);

                if target == *player_entity {
                    log.entries.push(format!(
                        "You equip {}",
                        names.get(item_to_use.item).unwrap().name
                    ))
                }
            }
        }
    }
}
