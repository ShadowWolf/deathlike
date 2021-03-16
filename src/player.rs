use super::{
    CombatStats, GameLog, Item, Map, Player, Position, RunState, State, Viewshed, WantsToMelee,
    MAP_HEIGHT, MAP_WIDTH,
};
use crate::{EntityMoved, Monster, TileType, WantsToPickupItem};
use rltk::{Point, Rltk, VirtualKeyCode};
use specs::prelude::*;
use std::cmp::{max, min};

pub fn try_move_player(delta_x: i32, delta_y: i32, ecs: &mut World) {
    let mut positions = ecs.write_storage::<Position>();
    let players = ecs.read_storage::<Player>();
    let mut viewsheds = ecs.write_storage::<Viewshed>();
    let combat_stats = ecs.read_storage::<CombatStats>();
    let entities = ecs.entities();
    let mut wants_to_melee = ecs.write_storage::<WantsToMelee>();
    let map = ecs.fetch::<Map>();
    let mut entity_moved = ecs.write_storage::<EntityMoved>();

    for (entity, _player, pos, viewshed) in
        (&entities, &players, &mut positions, &mut viewsheds).join()
    {
        let destination_idx = map.xy_idx(pos.x + delta_x, pos.y + delta_y);

        for potential_target in map.tile_content[destination_idx].iter() {
            let target = combat_stats.get(*potential_target);

            if let Some(_target) = target {
                wants_to_melee
                    .insert(
                        entity,
                        WantsToMelee {
                            target: *potential_target,
                        },
                    )
                    .expect("Add target failed");
            }
        }

        if !map.blocked[destination_idx] {
            pos.x = min(MAP_WIDTH as i32 - 1, max(0, pos.x + delta_x));
            pos.y = min(MAP_HEIGHT as i32 - 1, max(0, pos.y + delta_y));

            let mut player_position = ecs.write_resource::<Point>();
            player_position.x = pos.x;
            player_position.y = pos.y;

            viewshed.dirty = true;

            entity_moved
                .insert(entity, EntityMoved {})
                .expect("unable to insert move record");
        }
    }
}

pub fn player_input(gs: &mut State, ctx: &mut Rltk) -> RunState {
    match ctx.key {
        None => return RunState::AwaitingInput,
        Some(key) => match key {
            VirtualKeyCode::Left | VirtualKeyCode::Numpad4 | VirtualKeyCode::H => {
                try_move_player(-1, 0, &mut gs.ecs)
            }
            VirtualKeyCode::Right | VirtualKeyCode::Numpad6 | VirtualKeyCode::L => {
                try_move_player(1, 0, &mut gs.ecs)
            }
            VirtualKeyCode::Up | VirtualKeyCode::Numpad8 | VirtualKeyCode::K => {
                try_move_player(0, -1, &mut gs.ecs)
            }
            VirtualKeyCode::Down | VirtualKeyCode::Numpad2 | VirtualKeyCode::J => {
                try_move_player(0, 1, &mut gs.ecs)
            }

            VirtualKeyCode::Numpad9 => try_move_player(1, -1, &mut gs.ecs),
            VirtualKeyCode::Numpad7 => try_move_player(-1, -1, &mut gs.ecs),
            VirtualKeyCode::Numpad1 => try_move_player(-1, 1, &mut gs.ecs),
            VirtualKeyCode::Numpad3 => try_move_player(1, 1, &mut gs.ecs),
            VirtualKeyCode::G => get_item(&mut gs.ecs),
            VirtualKeyCode::I => return RunState::ShowInventory,
            VirtualKeyCode::D => return RunState::ShowDropItem,
            VirtualKeyCode::Escape => return RunState::SaveGame,
            VirtualKeyCode::Numpad5 | VirtualKeyCode::Space => return skip_turn(&mut gs.ecs),
            VirtualKeyCode::R => return RunState::ShowRemoveItem,
            VirtualKeyCode::Period => {
                if try_next_level(&mut gs.ecs) {
                    return RunState::NextLevel;
                }
            }
            _ => return RunState::AwaitingInput,
        },
    }

    RunState::PlayerTurn
}

pub fn skip_turn(ecs: &mut World) -> RunState {
    let player_entity = ecs.fetch::<Entity>();
    let viewshed_components = ecs.read_storage::<Viewshed>();
    let monsters = ecs.read_storage::<Monster>();

    let map = ecs.fetch::<Map>();

    let mut can_heal = true;
    let viewshed = viewshed_components.get(*player_entity).unwrap();
    for tile in viewshed.visible_tiles.iter() {
        let i = map.index_of(tile);
        for entity_id in map.tile_content[i].iter() {
            let mob = monsters.get(*entity_id);
            if mob.is_some() {
                can_heal = false;
            }
        }

        if !can_heal {
            break;
        }
    }

    if can_heal {
        let mut combat_stats = ecs.write_storage::<CombatStats>();
        let player_stats = combat_stats.get_mut(*player_entity).unwrap();
        player_stats.hp = i32::min(player_stats.hp + 1, player_stats.max_hp);
    }

    RunState::PlayerTurn
}

pub fn try_next_level(ecs: &mut World) -> bool {
    let player_position = ecs.fetch::<Point>();
    let map = ecs.fetch::<Map>();
    let player_index = map.xy_idx(player_position.x, player_position.y);
    if map.tiles[player_index] == TileType::StairsDown {
        true
    } else {
        let mut log = ecs.write_resource::<GameLog>();
        log.entries
            .push("There is no way to go down from here.".to_string());
        false
    }
}

fn get_item(ecs: &mut World) {
    let player_pos = ecs.fetch::<Point>();
    let player_entity = ecs.fetch::<Entity>();
    let entities = ecs.entities();
    let items = ecs.read_storage::<Item>();
    let positions = ecs.read_storage::<Position>();
    let mut log = ecs.fetch_mut::<GameLog>();

    let mut target_item: Option<Entity> = None;
    for (item_entity, _item, position) in (&entities, &items, &positions).join() {
        if position.x == player_pos.x && position.y == player_pos.y {
            target_item = Some(item_entity);
        }
    }

    match target_item {
        None => log
            .entries
            .push("There is nothing here to pick up".to_string()),
        Some(item) => {
            let mut pickup = ecs.write_storage::<WantsToPickupItem>();
            pickup
                .insert(
                    *player_entity,
                    WantsToPickupItem {
                        collected_by: *player_entity,
                        item,
                    },
                )
                .expect("Unable to insert pickup action");
        }
    }
}
