use specs::prelude::*;
use super::{GameLog, Entity, WantsToDrinkPotion, Name, Potion, CombatStats};

pub struct PotionUseSystem {}

impl<'a> System<'a> for PotionUseSystem {
    type SystemData = ( ReadExpect<'a, Entity>,
    WriteExpect<'a, GameLog>,
    Entities<'a>,
    WriteStorage<'a, WantsToDrinkPotion>,
    ReadStorage<'a, Name>,
        ReadStorage<'a, Potion>,
        WriteStorage<'a, CombatStats>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (player_entity, mut log, entities, wants_to_drink, names, potions, mut combat_stats) = data;

        for (entity, drink, stats) in (&entities, &wants_to_drink, &mut combat_stats).join() {
            let potion = potions.get(drink.potion);

            match potion {
                None => {},
                Some(potion) => {
                    stats.hp = i32::min(stats.max_hp, stats.hp + potion.heal_amount);
                    if entity == *player_entity {
                        log.entries.push(format!("You drink the {}, healing {} HP", names.get(drink.potion).unwrap().name, potion.heal_amount));
                    }
                    entities.delete(drink.potion).expect("Delete of potion that was drank failed");
                },
            }
        }
    }
}