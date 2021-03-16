mod components;
mod damage_system;
mod gamelog;
mod gui;
mod inventory_system;
mod item_collection_system;
mod item_drop_system;
mod map;
mod map_indexing_system;
mod melee_combat_system;
mod monster_ai_system;
mod player;
mod random_table;
mod rect;
mod rollable;
mod save_load_system;
mod spawner;
mod visibility_system;

pub use components::*;
pub use gamelog::*;
pub use gui::*;
pub use map::*;
pub use player::*;
pub use random_table::*;
pub use rect::*;
pub use rollable::*;
pub use save_load_system::*;
pub use spawner::*;

use crate::TileType::Floor;
use rltk::{GameState, Point, Rltk};
use specs::prelude::*;
use specs::saveload::{SimpleMarker, SimpleMarkerAllocator};

#[derive(PartialEq, Copy, Clone)]
pub enum RunState {
    AwaitingInput,
    PreRun,
    PlayerTurn,
    MonsterTurn,
    ShowInventory,
    ShowDropItem,
    ShowTargeting {
        range: i32,
        item: Entity,
    },
    MainMenu {
        menu_selection: gui::MainMenuSelection,
    },
    SaveGame,
    NextLevel,
    ShowRemoveItem,
    GameOver,
}

enum FloorChangeType {
    Desecend,
    Ascend,
    BackToStart,
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

        let mut potion_system = inventory_system::UseItemSystem {};
        potion_system.run_now(&self.ecs);

        let mut item_drop_system = item_drop_system::ItemDropSystem {};
        item_drop_system.run_now(&self.ecs);

        let mut item_remove_system = inventory_system::ItemRemoveSystem {};
        item_remove_system.run_now(&self.ecs);

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

    fn draw_interface(&mut self, ctx: &mut Rltk) {
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

        gui::draw_ui(&self.ecs, ctx);
    }

    fn fetch_entities_to_remove_on_level_change(&mut self) -> Vec<Entity> {
        let entities = self.ecs.entities();
        let player = self.ecs.read_storage::<Player>();
        let backpack = self.ecs.read_storage::<InBackpack>();
        let player_entity = self.ecs.fetch::<Entity>();
        let equipped = self.ecs.read_storage::<Equipped>();

        let mut to_delete: Vec<Entity> = Vec::new();
        for entity in entities.join() {
            let p = player.get(entity);
            if p.is_some() {
                continue;
            }

            let bp = backpack.get(entity);
            if let Some(bp) = bp {
                if bp.owner == *player_entity {
                    continue;
                }
            }

            let eq = equipped.get(entity);
            if let Some(eq) = eq {
                if eq.owner == *player_entity {
                    continue;
                }
            }

            to_delete.push(entity);
        }

        to_delete
    }

    fn create_new_world(&mut self, floor_action: FloorChangeType) -> Map {
        let mut worldmap_resource = self.ecs.write_resource::<Map>();
        let current_depth = worldmap_resource.depth;

        let desired_depth;
        match floor_action {
            FloorChangeType::Desecend => {
                desired_depth = current_depth + 1;
            }
            FloorChangeType::Ascend => {
                desired_depth = i32::min(current_depth - 1, 1);
            }
            FloorChangeType::BackToStart => {
                desired_depth = 1;
            }
        }

        *worldmap_resource = Map::new_map_rooms_and_corridors(desired_depth);
        worldmap_resource.clone()
    }

    fn go_to_next_level(&mut self) {
        let to_delete = self.fetch_entities_to_remove_on_level_change();
        for target in to_delete {
            self.ecs
                .delete_entity(target)
                .expect("unable to delete entity during floor change");
        }

        let map = self.create_new_world(FloorChangeType::Desecend);

        self.populate_rooms(&map);

        let (player_x, player_y) = self.setup_player_point(map);

        let player_entity = self.setup_player_position(player_x, player_y);

        self.setup_player_viewshed(&player_entity);

        let mut log = self.ecs.fetch_mut::<GameLog>();
        log.entries
            .push("You descend to the next level - suddenly your strength returns".to_string());

        let mut combat_stats = self.ecs.write_storage::<CombatStats>();
        let player_stats = combat_stats.get_mut(player_entity);
        if let Some(player_stats) = player_stats {
            player_stats.hp = i32::max(player_stats.hp, player_stats.max_hp / 2);
        }
    }

    fn setup_player_viewshed(&mut self, player_entity: &Entity) {
        let mut viewshed_components = self.ecs.write_storage::<Viewshed>();
        let vs = viewshed_components.get_mut(*player_entity);
        if let Some(vs) = vs {
            vs.dirty = true;
        }
    }

    fn setup_player_position(&mut self, player_x: i32, player_y: i32) -> Entity {
        let mut position_components = self.ecs.write_storage::<Position>();
        let player_entity = self.ecs.fetch::<Entity>();
        let player_position_component = position_components.get_mut(*player_entity);
        if let Some(player_position_component) = player_position_component {
            player_position_component.x = player_x;
            player_position_component.y = player_y;
        }

        *player_entity
    }

    fn setup_player_point(&mut self, map: Map) -> (i32, i32) {
        let (player_x, player_y) = map.rooms[0].center();
        let mut player_position = self.ecs.write_resource::<Point>();
        *player_position = Point::new(player_x, player_y);
        (player_x, player_y)
    }

    fn populate_rooms(&mut self, map: &Map) {
        for room in map.rooms.iter().skip(1) {
            spawner::spawn_room(&mut self.ecs, room, map.depth);
        }
    }

    pub fn game_over_cleanup(&mut self) {
        self.ecs.delete_all();

        let map = self.create_new_world(FloorChangeType::BackToStart);
        self.populate_rooms(&map);

        let (player_x, player_y) = self.setup_player_point(map);

        let player_entity = self.setup_player_position(player_x, player_y);

        self.setup_player_viewshed(&player_entity);
    }
}

impl GameState for State {
    fn tick(&mut self, ctx: &mut Rltk) {
        let mut new_run_state = self.determine_run_state();

        ctx.cls();

        match new_run_state {
            RunState::MainMenu { .. } => {}
            _ => {
                map::draw_map(&self.ecs, ctx);

                self.draw_interface(ctx);
            }
        }

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
                    ItemMenuResult::NoResponse => {}
                    ItemMenuResult::Selected => {
                        let item_entity = selection.unwrap();
                        let mut intent = self.ecs.write_storage::<WantsToUseItem>();
                        let ranged_items = self.ecs.read_storage::<Ranged>();
                        let ranged_item = ranged_items.get(item_entity);
                        match ranged_item {
                            None => {
                                intent
                                    .insert(
                                        *self.ecs.fetch::<Entity>(),
                                        WantsToUseItem {
                                            item: item_entity,
                                            target: None,
                                        },
                                    )
                                    .expect("Unable to insert use item intent");

                                new_run_state = RunState::PlayerTurn;
                            }
                            Some(ranged) => {
                                new_run_state = RunState::ShowTargeting {
                                    item: item_entity,
                                    range: ranged.range,
                                }
                            }
                        }
                    }
                }
            }
            RunState::ShowDropItem => {
                let (response, selection) = show_drop_item(self, ctx);
                match response {
                    ItemMenuResult::Cancel => new_run_state = RunState::AwaitingInput,
                    ItemMenuResult::NoResponse => {}
                    ItemMenuResult::Selected => {
                        let item_entity = selection.unwrap();
                        let mut intent = self.ecs.write_storage::<WantsToDropItem>();
                        intent
                            .insert(
                                *self.ecs.fetch::<Entity>(),
                                WantsToDropItem { item: item_entity },
                            )
                            .expect("Unable to insert drop item intent");

                        new_run_state = RunState::PlayerTurn;
                    }
                }
            }
            RunState::ShowTargeting { range, item } => {
                let (result, target_point) = gui::ranged_target(self, ctx, range);
                match result {
                    ItemMenuResult::Cancel => new_run_state = RunState::AwaitingInput,
                    ItemMenuResult::NoResponse => {}
                    ItemMenuResult::Selected => {
                        let mut intent = self.ecs.write_storage::<WantsToUseItem>();
                        intent
                            .insert(
                                *self.ecs.fetch::<Entity>(),
                                WantsToUseItem {
                                    item,
                                    target: target_point,
                                },
                            )
                            .expect("Unable to insert targeting intent");
                        new_run_state = RunState::PlayerTurn;
                    }
                }
            }
            RunState::MainMenu { .. } => {
                let result = gui::show_main_menu(self, ctx);
                match result {
                    MainMenuResult::NoSelection { selected } => {
                        new_run_state = RunState::MainMenu {
                            menu_selection: selected,
                        }
                    }
                    MainMenuResult::Selected { selected } => match selected {
                        MainMenuSelection::NewGame => new_run_state = RunState::PreRun,
                        MainMenuSelection::LoadGame => {
                            save_load_system::load_game(&mut self.ecs);
                            new_run_state = RunState::AwaitingInput;
                            delete_saved_game();
                        }
                        MainMenuSelection::Quit => {
                            ::std::process::exit(0);
                        }
                    },
                }
            }
            RunState::SaveGame => {
                save_load_system::save_game(&mut self.ecs);
                new_run_state = RunState::MainMenu {
                    menu_selection: MainMenuSelection::LoadGame,
                };
            }
            RunState::NextLevel => {
                self.go_to_next_level();
                new_run_state = RunState::PreRun;
            }
            RunState::ShowRemoveItem => {
                let (response, selection) = show_remove_item(self, ctx);
                match response {
                    ItemMenuResult::Cancel => new_run_state = RunState::AwaitingInput,
                    ItemMenuResult::NoResponse => {}
                    ItemMenuResult::Selected => {
                        let item_entity = selection.unwrap();
                        let mut intent = self.ecs.write_storage::<WantsToRemoveItem>();
                        intent
                            .insert(
                                *self.ecs.fetch::<Entity>(),
                                WantsToRemoveItem { item: item_entity },
                            )
                            .expect("Unable to insert remove item intent");

                        new_run_state = RunState::PlayerTurn;
                    }
                }
            }
            RunState::GameOver => {
                let result = gui::game_over(ctx);
                match result {
                    GameOverResult::NoSelection => {}
                    GameOverResult::QuitToMenu => {
                        self.game_over_cleanup();
                        new_run_state = RunState::MainMenu {
                            menu_selection: MainMenuSelection::NewGame,
                        };
                    }
                }
            }
        }

        self.store_run_state(&new_run_state);

        damage_system::delete_dead(&mut self.ecs);
    }
}

fn main() -> rltk::BError {
    use rltk::RltkBuilder;
    let context = RltkBuilder::simple80x50().with_title("Deathlike").build()?;

    let mut gs = State { ecs: World::new() };

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
    gs.ecs.register::<InBackpack>();
    gs.ecs.register::<WantsToPickupItem>();
    gs.ecs.register::<WantsToUseItem>();
    gs.ecs.register::<WantsToDropItem>();
    gs.ecs.register::<Consumable>();
    gs.ecs.register::<ProvidesHealing>();
    gs.ecs.register::<Ranged>();
    gs.ecs.register::<InflictsDamage>();
    gs.ecs.register::<AreaOfEffect>();
    gs.ecs.register::<Confusion>();
    gs.ecs.register::<SimpleMarker<Savable>>();
    gs.ecs.register::<SerializationHelper>();
    gs.ecs.register::<Equippable>();
    gs.ecs.register::<Equipped>();
    gs.ecs.register::<MeleePowerBonus>();
    gs.ecs.register::<DefenseBonus>();
    gs.ecs.register::<WantsToRemoveItem>();

    gs.ecs.insert(SimpleMarkerAllocator::<Savable>::new());

    let map = Map::new_map_rooms_and_corridors(1);
    let (player_x, player_y) = map.rooms[0].center();

    gs.ecs.insert(RunState::MainMenu {
        menu_selection: gui::MainMenuSelection::NewGame,
    });

    gs.ecs.insert(Point::new(player_x, player_y));
    gs.ecs.insert(GameLog {
        entries: vec!["Welcome to DeathLike".to_string()],
    });

    let player_entity = spawner::player(&mut gs.ecs, player_x, player_y);
    gs.ecs.insert(rltk::RandomNumberGenerator::new());

    gs.ecs.insert(player_entity);

    for room in map.rooms.iter().skip(1) {
        spawner::spawn_room(&mut gs.ecs, room, map.depth);
    }

    gs.ecs.insert(map);

    rltk::main_loop(context, gs)
}
