mod components;
mod damage_system;
mod gamelog;
mod gui;
mod inventory_system;
mod item_collection_system;
mod item_drop_system;
mod map;
mod map_builders;
mod map_indexing_system;
mod melee_combat_system;
mod monster_ai_system;
mod particle_system;
mod player;
mod random_table;
mod rect;
mod rollable;
mod save_load_system;
mod spawner;
mod trigger_system;
mod visibility_system;
mod rex_assets;

pub use components::*;
pub use gamelog::*;
pub use gui::*;
pub use map::*;
pub use particle_system::*;
pub use player::*;
pub use random_table::*;
pub use rect::*;
pub use rollable::*;
pub use save_load_system::*;
pub use spawner::*;
pub use trigger_system::*;

use crate::map_builders::MapBuilder;
use rltk::{GameState, Point, RandomNumberGenerator, Rltk};
use specs::prelude::*;
use specs::saveload::{SimpleMarker, SimpleMarkerAllocator};

#[derive(PartialEq, Copy, Clone, Debug)]
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
    MagicMapReveal {
        row: i32,
    },
    MapGeneration,
}

pub struct MapGenState {
    next_state: Option<RunState>,
    history: Vec<Map>,
    index: usize,
    timer: f32,
    total_time: f32,
}

pub struct State {
    pub ecs: World,
    pub mapgen: MapGenState,
    last_get_state: RunState,
    last_set_state: RunState,
}

const SHOW_MAPGEN_VISUALIZER: bool = true;
#[allow(dead_code)]
const SHOW_RUNSTATE_DEBUG: bool = true;

#[allow(dead_code)]
const GENERATE_RANDOM_MAPS: bool = true;

impl State {
    fn run_systems(&mut self) {
        let mut vis = visibility_system::VisibilitySystem {};
        vis.run_now(&self.ecs);

        let mut mob = monster_ai_system::MonsterAI {};
        mob.run_now(&self.ecs);

        let mut triggers = trigger_system::TriggerSystem {};
        triggers.run_now(&self.ecs);

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

        let mut particle_system = particle_system::ParticleSpawnSystem {};
        particle_system.run_now(&self.ecs);

        self.ecs.maintain();
    }

    fn determine_run_state(&mut self) -> RunState {
        let run_state = self.ecs.fetch::<RunState>();
        if (*run_state) == self.last_get_state {
            self.last_get_state = *run_state.clone();
            rltk::console::log(format!("Starting with state {:?}", *run_state));
        }
        *run_state
    }

    fn store_run_state(&mut self, state: &RunState) {
        if *state != self.last_set_state {
            self.last_set_state = *state;
            rltk::console::log(format!("Storing run state {:?}", state));
        }
        let mut run_writer = self.ecs.write_resource::<RunState>();
        *run_writer = *state;
    }

    fn draw_interface(&mut self, ctx: &mut Rltk) {
        let positions = self.ecs.read_storage::<Position>();
        let renderables = self.ecs.read_storage::<Renderable>();
        let hidden = self.ecs.read_storage::<Hidden>();
        let map = self.ecs.fetch::<Map>();

        let mut renderable_objects = (&positions, &renderables, !&hidden)
            .join()
            .collect::<Vec<_>>();
        renderable_objects.sort_by(|&a, &b| b.1.render_order.cmp(&a.1.render_order));
        for (pos, render, _hidden) in renderable_objects.iter() {
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

    fn determine_start_position(&mut self, builder: &mut Box<dyn MapBuilder>) -> Position {
        let mut map_resource = self.ecs.write_resource::<Map>();
        *map_resource = builder.get_map();
        builder.get_starting_position()
    }

    fn set_player_position(&mut self, player_position: &Position) {
        let player_entity = self.ecs.fetch::<Entity>();

        let mut position_resource = self.ecs.write_resource::<Point>();
        *position_resource = Point::new(player_position.x, player_position.y);

        let mut position_compnents = self.ecs.write_storage::<Position>();
        let player_position_component = position_compnents.get_mut(*player_entity);
        if let Some(pos) = player_position_component {
            pos.x = player_position.x;
            pos.y = player_position.y;
        }
    }

    fn generate_world_map(&mut self, new_depth: i32) {
        self.mapgen.index = 0;
        self.mapgen.timer = 0.;
        self.mapgen.history.clear();

        let mut builder = if GENERATE_RANDOM_MAPS { map_builders::random_builder(new_depth) } else { map_builders::static_builder(new_depth) };
        builder.build_map();

        self.mapgen.history = builder.get_snapshot_history();

        let start_position = self.determine_start_position(&mut builder);

        builder.spawn_entities(&mut self.ecs);

        self.set_player_position(&start_position);

        self.reset_player_viewshed();
    }

    fn go_to_next_level(&mut self) {
        let to_delete = self.fetch_entities_to_remove_on_level_change();
        for target in to_delete {
            self.ecs
                .delete_entity(target)
                .expect("unable to delete entity during floor change");
        }

        let current_depth;
        {
            let map_resource = self.ecs.fetch::<Map>();
            current_depth = map_resource.depth;
        }
        self.generate_world_map(current_depth + 1);

        let mut log = self.ecs.fetch_mut::<GameLog>();
        log.entries
            .push("You descend to the next level - suddenly your strength returns".to_string());

        let player_entity = self.ecs.fetch::<Entity>();
        let mut combat_stats = self.ecs.write_storage::<CombatStats>();
        let player_stats = combat_stats.get_mut(*player_entity);
        if let Some(player_stats) = player_stats {
            player_stats.hp = i32::max(player_stats.hp, player_stats.max_hp / 2);
        }
    }

    fn reset_player_viewshed(&mut self) {
        let player_entity = self.ecs.fetch::<Entity>();
        let mut viewshed_components = self.ecs.write_storage::<Viewshed>();
        let vs = viewshed_components.get_mut(*player_entity);
        if let Some(vs) = vs {
            vs.dirty = true;
        }
    }

    pub fn game_over_cleanup(&mut self) {
        let mut to_delete = Vec::new();
        for e in self.ecs.entities().join() {
            to_delete.push(e);
        }
        for del in to_delete.iter() {
            self.ecs
                .delete_entity(*del)
                .expect("Could not delete entity");
        }

        {
            let player_entity = spawner::player(&mut self.ecs, 0, 0);
            let mut writer = self.ecs.write_resource::<Entity>();
            *writer = player_entity;
        }

        self.generate_world_map(1);
    }
}

impl GameState for State {
    fn tick(&mut self, ctx: &mut Rltk) {
        let mut new_run_state = self.determine_run_state();

        ctx.cls();
        remove_dead_particles(&mut self.ecs, ctx);

        match new_run_state {
            RunState::MainMenu { .. } => {}
            _ => {
                map::draw_map(&self.ecs.fetch::<Map>(), ctx);

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

                match *self.ecs.fetch::<RunState>() {
                    RunState::MagicMapReveal { .. } => {
                        new_run_state = RunState::MagicMapReveal { row: 0 }
                    }
                    _ => new_run_state = RunState::MonsterTurn,
                }
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
            RunState::MagicMapReveal { row } => {
                let mut map = self.ecs.fetch_mut::<Map>();
                for x in 0..MAP_WIDTH {
                    let i = map.xy_idx(x as i32, row);
                    map.revealed_tiles[i] = true;
                }

                if row as usize == MAP_HEIGHT - 1 {
                    new_run_state = RunState::MonsterTurn;
                } else {
                    new_run_state = RunState::MagicMapReveal { row: row + 1 };
                }
            }
            RunState::MapGeneration => {
                if !SHOW_MAPGEN_VISUALIZER {
                    new_run_state = self.mapgen.next_state.unwrap();
                }

                ctx.cls();
                draw_map(&self.mapgen.history[self.mapgen.index], ctx);

                self.mapgen.timer += ctx.frame_time_ms;
                if self.mapgen.timer > self.mapgen.total_time {
                    self.mapgen.timer = 0.;
                    self.mapgen.index += 1;
                    if self.mapgen.index >= self.mapgen.history.len() {
                        new_run_state = self.mapgen.next_state.unwrap();
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

    let mut gs = State {
        ecs: World::new(),
        mapgen: MapGenState {
            next_state: Some(RunState::MainMenu {
                menu_selection: MainMenuSelection::NewGame,
            }),
            history: Vec::new(),
            timer: 0.,
            index: 0,
            total_time: 300.0,
        },
        last_get_state: RunState::GameOver,
        last_set_state: RunState::GameOver,
    };

    register_components(&mut gs);

    gs.ecs.insert(SimpleMarkerAllocator::<Savable>::new());
    gs.ecs.insert(RunState::MapGeneration {});
    gs.ecs.insert(Map::new(1));
    gs.ecs.insert(Point::new(0, 0));
    gs.ecs.insert(RandomNumberGenerator::new());

    let player_entity = spawner::player(&mut gs.ecs, 0, 0);
    gs.ecs.insert(player_entity);
    gs.ecs.insert(GameLog {
        entries: vec!["Welcome to deathlike!".to_string()],
    });
    gs.ecs.insert(ParticleBuilder::new());
    gs.ecs.insert(rex_assets::RexAssets::new());
    gs.generate_world_map(1);

    rltk::main_loop(context, gs)
}

fn register_components(gs: &mut State) {
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
    gs.ecs.register::<ParticleLifetime>();
    gs.ecs.register::<MagicMapper>();
    gs.ecs.register::<Hidden>();
    gs.ecs.register::<EntryTrigger>();
    gs.ecs.register::<EntityMoved>();
    gs.ecs.register::<SingleActivation>();
}
