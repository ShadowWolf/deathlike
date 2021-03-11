mod components;
mod map;
mod map_indexing_system;
mod melee_combat_system;
mod monster_ai_system;
mod player;
mod rect;
mod visibility_system;
mod damage_system;
mod gui;
mod gamelog;
mod spawner;
mod item_collection_system;
mod inventory_system;
mod item_drop_system;

pub use components::*;
pub use map::*;
pub use player::*;
pub use rect::*;
pub use gui::*;
pub use gamelog::*;
pub use spawner::*;


use rltk::{GameState, Point, Rltk};
use specs::prelude::*;

#[derive(PartialEq, Copy, Clone)]
pub enum RunState {
    AwaitingInput,
    PreRun,
    PlayerTurn,
    MonsterTurn,
    ShowInventory,
    ShowDropItem,
}

pub struct State {
    pub ecs: World,
}

impl State {
    fn run_systems(&mut self) {

        let mut vis = visibility_system::VisibilitySystem {};
        vis.run_now(&self.ecs);

        let mut mob = monster_ai_system::MonsterAI {};
        mob.run_now(&self.ecs);

        let mut map_index = map_indexing_system::MapIndexingSystem {};
        map_index.run_now(&self.ecs);

        let mut melee_combat_system = melee_combat_system::MeleeCombatSystem {};
        melee_combat_system.run_now(&self.ecs);

        let mut damage_system = damage_system::DamageSystem {};
        damage_system.run_now(&self.ecs);

        let mut pickup_system = item_collection_system::ItemCollectionSystem {};
        pickup_system.run_now(&self.ecs);

        let mut potion_system = inventory_system::PotionUseSystem {};
        potion_system.run_now(&self.ecs);

        let mut item_drop_system = item_drop_system::ItemDropSystem {};
        item_drop_system.run_now(&self.ecs);

        self.ecs.maintain();
    }

    fn determine_run_state(&mut self) -> RunState {
        let run_state = self.ecs.fetch::<RunState>();
        *run_state
    }

    fn store_run_state(&mut self, state: &RunState) {
        let mut run_writer = self.ecs.write_resource::<RunState>();
        *run_writer = *state;
    }
}

impl GameState for State {
    fn tick(&mut self, ctx: &mut Rltk) {
        ctx.cls();

        draw_map(&self.ecs, ctx);

        let mut new_run_state= self.determine_run_state();

        match new_run_state {
            RunState::PreRun => {
                self.run_systems();
                self.ecs.maintain();
                new_run_state = RunState::AwaitingInput;
            }
            RunState::AwaitingInput => {
                new_run_state = player_input(self, ctx);
            }
            RunState::PlayerTurn => {
                self.run_systems();
                self.ecs.maintain();
                new_run_state = RunState::MonsterTurn;
            }
            RunState::MonsterTurn => {
                self.run_systems();
                self.ecs.maintain();
                new_run_state = RunState::AwaitingInput;
            }
            RunState::ShowInventory => {
                let (response, selection) = show_inventory(self, ctx);
                match response {
                    ItemMenuResult::Cancel => new_run_state = RunState::AwaitingInput,
                    ItemMenuResult::NoResponse => {},
                    ItemMenuResult::Selected => {
                        let item_entity = selection.unwrap();
                        let mut intent = self.ecs.write_storage::<WantsToDrinkPotion>();
                        intent.insert(*self.ecs.fetch::<Entity>(), WantsToDrinkPotion { potion: item_entity }).expect("Unable to insert potion drink intent");

                        new_run_state = RunState::PlayerTurn;
                    }
                }
            }
            RunState::ShowDropItem => {
                let (response, selection) = show_drop_item(self, ctx);
                match response {
                    ItemMenuResult::Cancel => new_run_state = RunState::AwaitingInput,
                    ItemMenuResult::NoResponse => {},
                    ItemMenuResult::Selected => {
                        let item_entity = selection.unwrap();
                        let mut intent = self.ecs.write_storage::<WantsToDropItem>();
                        intent.insert(*self.ecs.fetch::<Entity>(), WantsToDropItem { item: item_entity }).expect("Unable to insert drop item intent");

                        new_run_state = RunState::PlayerTurn;
                    }
                }
            }
        }

        self.store_run_state(&new_run_state);

        damage_system::delete_dead(&mut self.ecs);

        let positions = self.ecs.read_storage::<Position>();
        let renderables = self.ecs.read_storage::<Renderable>();
        let map = self.ecs.fetch::<Map>();

        let mut renderable_objects = (&positions, &renderables).join().collect::<Vec<_>>();
        renderable_objects.sort_by(|&a, &b| b.1.render_order.cmp(&a.1.render_order));
        for (pos, render) in renderable_objects.iter() {
            let idx = map.xy_idx(pos.x, pos.y);
            if map.visible_tiles[idx] {
                ctx.set(pos.x, pos.y, render.fg, render.bg, render.glyph);
            }
        }

        draw_ui(&self.ecs, ctx);
    }
}


fn main() -> rltk::BError {
    use rltk::RltkBuilder;
    let context = RltkBuilder::simple80x50().with_title("Deathlike").build()?;

    let mut gs = State {
        ecs: World::new(),
    };

    gs.ecs.register::<Position>();
    gs.ecs.register::<Renderable>();
    gs.ecs.register::<Player>();
    gs.ecs.register::<Viewshed>();
    gs.ecs.register::<Monster>();
    gs.ecs.register::<Name>();
    gs.ecs.register::<BlocksTile>();
    gs.ecs.register::<CombatStats>();
    gs.ecs.register::<WantsToMelee>();
    gs.ecs.register::<SufferDamage>();
    gs.ecs.register::<Item>();
    gs.ecs.register::<Potion>();
    gs.ecs.register::<InBackpack>();
    gs.ecs.register::<WantsToPickupItem>();
    gs.ecs.register::<WantsToDrinkPotion>();
    gs.ecs.register::<WantsToDropItem>();

    let map = Map::new_map_rooms_and_corridors();
    let (player_x, player_y) = map.rooms[0].center();

    gs.ecs.insert(RunState::PreRun);

    gs.ecs.insert(Point::new(player_x, player_y));
    gs.ecs.insert(GameLog {
        entries: vec!["Welcome to DeathLike".to_string()]
    });

    let player_entity = spawner::player(&mut gs.ecs, player_x, player_y);
    gs.ecs.insert(rltk::RandomNumberGenerator::new());

    gs.ecs.insert(player_entity);

    for room in map.rooms.iter().skip(1) {
        spawner::spawn_room(&mut gs.ecs, room);
    }

    gs.ecs.insert(map);

    rltk::main_loop(context, gs)
}