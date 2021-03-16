use super::{gamelog::GameLog, CombatStats, Name, SufferDamage, WantsToMelee};
use crate::{DefenseBonus, Equipped, MeleePowerBonus};
use specs::prelude::*;

pub struct MeleeCombatSystem {}

impl MeleeCombatSystem {
    fn determine_melee_power_bonus(
        &self,
        owner_entity: &Entity,
        entities: &Entities,
        melee_items: &ReadStorage<MeleePowerBonus>,
        equipped: &ReadStorage<Equipped>,
    ) -> i32 {
        let mut offense_bonus = 0;
        for (_item_entity, melee_power, equipped_by) in (entities, melee_items, equipped).join() {
            if equipped_by.owner == *owner_entity {
                offense_bonus += melee_power.power;
            }
        }

        offense_bonus
    }

    fn determine_defense_bonus(
        &self,
        target_entity: &Entity,
        entities: &Entities,
        defense_items: &ReadStorage<DefenseBonus>,
        equipped: &ReadStorage<Equipped>,
    ) -> i32 {
        let mut defensive_bonus = 0;

        for (_item, defense_power, equipped_by) in (entities, defense_items, equipped).join() {
            if equipped_by.owner == *target_entity {
                defensive_bonus += defense_power.defense
            }
        }

        defensive_bonus
    }
}

impl<'a> System<'a> for MeleeCombatSystem {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, WantsToMelee>,
        ReadStorage<'a, Name>,
        ReadStorage<'a, CombatStats>,
        WriteStorage<'a, SufferDamage>,
        WriteExpect<'a, GameLog>,
        ReadStorage<'a, MeleePowerBonus>,
        ReadStorage<'a, DefenseBonus>,
        ReadStorage<'a, Equipped>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            mut wants_melee,
            names,
            combat_stats,
            mut inflict_data,
            mut log,
            melee_bonus,
            defense_bonus,
            equipped,
        ) = data;

        for (entity, wants_melee, name, stats) in
            (&entities, &wants_melee, &names, &combat_stats).join()
        {
            if stats.hp > 0 {
                let attack_power =
                    self.determine_melee_power_bonus(&entity, &entities, &melee_bonus, &equipped);

                let target_stats = combat_stats.get(wants_melee.target).unwrap();
                if target_stats.hp > 0 {
                    let defense = self.determine_defense_bonus(
                        &wants_melee.target,
                        &entities,
                        &defense_bonus,
                        &equipped,
                    );

                    let target_name = names.get(wants_melee.target).unwrap();
                    let damage = i32::max(
                        0,
                        (stats.power + attack_power) - (target_stats.defense + defense),
                    );

                    if damage == 0 {
                        log.entries.push(format!(
                            "{} is unable to hurt {}",
                            &name.name, &target_name.name
                        ));
                    } else {
                        log.entries.push(format!(
                            "{} hits {} for {} damage",
                            &name.name, target_name.name, damage
                        ));
                        SufferDamage::new_damage(&mut inflict_data, wants_melee.target, damage);
                    }
                }
            }
        }

        wants_melee.clear();
    }
}
