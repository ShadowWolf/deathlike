use super::{CombatStats, GameLog, Map, Name, Player, Position};
use crate::{save_exists, Equipped, Hidden, InBackpack, ItemHasOwner, RunState, State, Viewshed};
use rltk::{console, Point, Rltk, VirtualKeyCode, RGB};
use specs::prelude::*;
use specs::world::EntitiesRes;

#[derive(PartialEq, Copy, Clone)]
pub enum ItemMenuResult {
    Cancel,
    NoResponse,
    Selected,
}

#[derive(PartialEq, Copy, Clone)]
pub enum MainMenuSelection {
    NewGame,
    LoadGame,
    Quit,
}

#[derive(PartialEq, Copy, Clone)]
pub enum MainMenuResult {
    NoSelection { selected: MainMenuSelection },
    Selected { selected: MainMenuSelection },
}

#[derive(PartialEq, Copy, Clone)]
pub enum GameOverResult {
    NoSelection,
    QuitToMenu,
}

pub fn draw_ui(ecs: &World, ctx: &mut Rltk) {
    ctx.draw_box(
        0,
        43,
        79,
        6,
        RGB::named(rltk::WHITE),
        RGB::named(rltk::BLACK),
    );

    let combat_stats = ecs.read_storage::<CombatStats>();
    let players = ecs.read_storage::<Player>();
    for (_player, stats) in (&players, &combat_stats).join() {
        let health = format!(" HP: {} / {} ", stats.hp, stats.max_hp);
        ctx.print_color(
            12,
            43,
            RGB::named(rltk::YELLOW),
            RGB::named(rltk::BLACK),
            &health,
        );

        let map = ecs.fetch::<Map>();
        let depth = format!(" Depth: {}", map.depth);
        ctx.print_color(
            2,
            43,
            RGB::named(rltk::YELLOW),
            RGB::named(rltk::BLACK),
            &depth,
        );

        ctx.draw_bar_horizontal(
            28,
            43,
            51,
            stats.hp,
            stats.max_hp,
            RGB::named(rltk::RED),
            RGB::named(rltk::BLACK),
        );
    }

    let log = ecs.fetch::<GameLog>();
    let mut y = 44;
    for s in log.entries.iter().rev() {
        if y < 49 {
            ctx.print(2, y, s);
        }
        y += 1;
    }

    let mouse_pos = ctx.mouse_pos();
    ctx.set_bg(mouse_pos.0, mouse_pos.1, RGB::named(rltk::MAGENTA));
    draw_tooltips(ecs, ctx);
}

pub fn show_inventory(gs: &mut State, ctx: &mut Rltk) -> (ItemMenuResult, Option<Entity>) {
    let player_entity = gs.ecs.fetch::<Entity>();
    let names = gs.ecs.read_storage::<Name>();
    let backpack = gs.ecs.read_storage::<InBackpack>();
    let entities = gs.ecs.entities();

    let inventory = (&backpack, &names)
        .join()
        .filter(|item| item.0.owner == *player_entity);
    let count = inventory.count();

    let y = (25 - (count / 2)) as i32;
    draw_title_box("Inventory".to_string(), ctx, count, y);

    let (_, items) = print_container_items(ctx, &*player_entity, &names, &backpack, &entities, y);

    process_item_selection(ctx, count, items)
}

pub fn show_drop_item(gs: &mut State, ctx: &mut Rltk) -> (ItemMenuResult, Option<Entity>) {
    let player_entity = gs.ecs.fetch::<Entity>();
    let names = gs.ecs.read_storage::<Name>();
    let backpack = gs.ecs.read_storage::<InBackpack>();
    let entities = gs.ecs.entities();

    let inventory = (&backpack, &names)
        .join()
        .filter(|item| item.0.owner == *player_entity);
    let count = inventory.count();

    let y = (25 - (count / 2)) as i32;
    draw_title_box("Drop which item?".to_string(), ctx, count, y);
    let (_, items) = print_container_items(ctx, &*player_entity, &names, &backpack, &entities, y);

    process_item_selection(ctx, count, items)
}

pub fn show_remove_item(gs: &mut State, ctx: &mut Rltk) -> (ItemMenuResult, Option<Entity>) {
    let player_entity = gs.ecs.fetch::<Entity>();
    let names = gs.ecs.read_storage::<Name>();
    let equippable = gs.ecs.read_storage::<Equipped>();
    let entities = gs.ecs.entities();

    let inventory = (&equippable, &names)
        .join()
        .filter(|item| item.0.owner == *player_entity);
    let count = inventory.count();

    let y = (25 - (count / 2)) as i32;
    draw_title_box("Un-Equip which item?".to_string(), ctx, count, y);

    let (_, items) = print_container_items(ctx, &*player_entity, &names, &equippable, &entities, y);

    process_item_selection(ctx, count, items)
}

pub fn show_main_menu(gs: &mut State, ctx: &mut Rltk) -> MainMenuResult {
    let run_state = gs.ecs.fetch::<RunState>();
    let show_load_game = save_exists();

    ctx.print_color_centered(
        15,
        RGB::named(rltk::YELLOW),
        RGB::named(rltk::BLACK),
        "Deathlike",
    );

    if let RunState::MainMenu {
        menu_selection: selection,
    } = *run_state
    {
        let selected_color = RGB::named(rltk::MAGENTA);
        let idle_color = RGB::named(rltk::WHITE);
        let background = RGB::named(rltk::BLACK);

        ctx.print_color_centered(
            24,
            if selection == MainMenuSelection::NewGame {
                selected_color
            } else {
                idle_color
            },
            background,
            "Begin New Game",
        );

        if show_load_game {
            ctx.print_color_centered(
                25,
                if selection == MainMenuSelection::LoadGame {
                    selected_color
                } else {
                    idle_color
                },
                background,
                "Load Game",
            );
        }

        ctx.print_color_centered(
            26,
            if selection == MainMenuSelection::Quit {
                selected_color
            } else {
                idle_color
            },
            background,
            "Quit Game",
        );

        return match ctx.key {
            None => MainMenuResult::NoSelection {
                selected: selection,
            },
            Some(key) => match key {
                VirtualKeyCode::Escape => MainMenuResult::NoSelection {
                    selected: selection,
                },
                VirtualKeyCode::Up => {
                    let mut new_selection;
                    match selection {
                        MainMenuSelection::NewGame => new_selection = MainMenuSelection::Quit,
                        MainMenuSelection::LoadGame => new_selection = MainMenuSelection::NewGame,
                        MainMenuSelection::Quit => new_selection = MainMenuSelection::LoadGame,
                    }

                    if new_selection == MainMenuSelection::LoadGame && !show_load_game {
                        new_selection = MainMenuSelection::NewGame;
                    }

                    MainMenuResult::NoSelection {
                        selected: new_selection,
                    }
                }
                VirtualKeyCode::Down => {
                    let mut new_selection;
                    match selection {
                        MainMenuSelection::NewGame => new_selection = MainMenuSelection::LoadGame,
                        MainMenuSelection::LoadGame => new_selection = MainMenuSelection::Quit,
                        MainMenuSelection::Quit => new_selection = MainMenuSelection::NewGame,
                    }

                    if new_selection == MainMenuSelection::LoadGame && !show_load_game {
                        new_selection = MainMenuSelection::Quit;
                    }

                    MainMenuResult::NoSelection {
                        selected: new_selection,
                    }
                }
                VirtualKeyCode::Return => MainMenuResult::Selected {
                    selected: selection,
                },
                _ => MainMenuResult::NoSelection {
                    selected: selection,
                },
            },
        };
    }

    MainMenuResult::NoSelection {
        selected: MainMenuSelection::NewGame,
    }
}

pub fn game_over(ctx: &mut Rltk) -> GameOverResult {
    let bg = RGB::named(rltk::BLACK);
    ctx.print_color_centered(
        15,
        RGB::named(rltk::YELLOW),
        bg,
        "Your journey has ended".to_string(),
    );
    ctx.print_color_centered(
        17,
        RGB::named(rltk::WHITE),
        bg,
        "One day, we might tell you how that happened".to_string(),
    );
    ctx.print_color_centered(
        18,
        RGB::named(rltk::WHITE),
        bg,
        "Sadly I am teh lazy".to_string(),
    );
    ctx.print_color_centered(
        20,
        RGB::named(rltk::MAGENTA),
        bg,
        "Press any key to return to the main menu",
    );

    match ctx.key {
        None => GameOverResult::NoSelection,
        Some(_) => GameOverResult::QuitToMenu,
    }
}

fn process_item_selection(
    ctx: &mut Rltk,
    count: usize,
    equippable: Vec<Entity>,
) -> (ItemMenuResult, Option<Entity>) {
    match ctx.key {
        None => (ItemMenuResult::NoResponse, None),
        Some(key) => match key {
            VirtualKeyCode::Escape => (ItemMenuResult::Cancel, None),
            _ => {
                let selection = rltk::letter_to_option(key);
                if selection > -1 && selection < count as i32 {
                    return (
                        ItemMenuResult::Selected,
                        Some(equippable[selection as usize]),
                    );
                }
                (ItemMenuResult::NoResponse, None)
            }
        },
    }
}

fn print_container_items(
    ctx: &mut Rltk,
    player_entity: &Entity,
    names: &ReadStorage<Name>,
    backpack: &ReadStorage<impl ItemHasOwner + specs::Component>,
    entities: &Read<EntitiesRes>,
    mut y: i32,
) -> (i32, Vec<Entity>) {
    let mut equippable: Vec<Entity> = Vec::new();
    for (j, (entity, _pack, name)) in (entities, backpack, names)
        .join()
        .filter(|item| item.1.owner() == *player_entity)
        .enumerate()
    {
        ctx.set(
            17,
            y,
            RGB::named(rltk::WHITE),
            RGB::named(rltk::BLACK),
            rltk::to_cp437('('),
        );
        ctx.set(
            18,
            y,
            RGB::named(rltk::YELLOW),
            RGB::named(rltk::BLACK),
            97 + j as rltk::FontCharType,
        );
        ctx.set(
            19,
            y,
            RGB::named(rltk::WHITE),
            RGB::named(rltk::BLACK),
            rltk::to_cp437(')'),
        );

        ctx.print(21, y, &name.name.to_string());
        equippable.push(entity);
        y += 1;
    }
    (y, equippable)
}

fn draw_title_box(title_text: String, ctx: &mut Rltk, count: usize, y: i32) {
    ctx.draw_box(
        15,
        y - 2,
        31,
        (count + 3) as i32,
        RGB::named(rltk::WHITE),
        RGB::named(rltk::BLACK),
    );
    ctx.print_color(
        18,
        y - 2,
        RGB::named(rltk::YELLOW),
        RGB::named(rltk::BLACK),
        title_text,
    );
    ctx.print_color(
        18,
        y + count as i32 + 1,
        RGB::named(rltk::YELLOW),
        RGB::named(rltk::BLACK),
        "Escape to exit/cancel",
    );
}

fn draw_tooltips(ecs: &World, ctx: &mut Rltk) {
    let map = ecs.fetch::<Map>();
    let names = ecs.read_storage::<Name>();
    let positions = ecs.read_storage::<Position>();
    let hidden = ecs.read_storage::<Hidden>();

    let mouse_pos = ctx.mouse_pos();
    if mouse_pos.0 >= map.width || mouse_pos.1 >= map.height {
        return;
    }

    let mut tooltip: Vec<String> = Vec::new();
    for (name, position, _h) in (&names, &positions, &hidden).join() {
        let idx = map.xy_idx(position.x, position.y);
        if position.x == mouse_pos.0 && position.y == mouse_pos.1 && map.visible_tiles[idx] {
            tooltip.push(name.name.to_string());
        }
    }

    if !tooltip.is_empty() {
        let mut width: i32 = 0;
        for s in tooltip.iter() {
            if width < s.len() as i32 {
                width = s.len() as i32;
            }
        }

        width += 3;

        if mouse_pos.0 > 40 {
            let arrow_pos = Point::new(mouse_pos.0 - 2, mouse_pos.1);
            let left_x = mouse_pos.0 - width;
            let mut y = mouse_pos.1;
            for s in tooltip.iter() {
                ctx.print_color(
                    left_x,
                    y,
                    RGB::named(rltk::WHITE),
                    RGB::named(rltk::GRAY),
                    s,
                );
                let padding = (width - s.len() as i32) - 1;
                for i in 0..padding {
                    ctx.print_color(
                        arrow_pos.x - i,
                        y,
                        RGB::named(rltk::WHITE),
                        RGB::named(rltk::GRAY),
                        &" ".to_string(),
                    );
                }

                y += 1;
            }

            ctx.print_color(
                arrow_pos.x,
                arrow_pos.y,
                RGB::named(rltk::WHITE),
                RGB::named(rltk::GRAY),
                &"->".to_string(),
            )
        } else {
            let arrow_pos = Point::new(mouse_pos.0 + 1, mouse_pos.1);
            let left_x = mouse_pos.0 + 3;
            let mut y = mouse_pos.1;
            for s in tooltip.iter() {
                ctx.print_color(
                    left_x + 1,
                    y,
                    RGB::named(rltk::WHITE),
                    RGB::named(rltk::GRAY),
                    s,
                );
                let padding = (width - s.len() as i32) - 1;
                for i in 0..padding {
                    ctx.print_color(
                        arrow_pos.x + 1 + i,
                        y,
                        RGB::named(rltk::WHITE),
                        RGB::named(rltk::GRAY),
                        &" ".to_string(),
                    );
                }

                y += 1;
            }
            ctx.print_color(
                arrow_pos.x,
                arrow_pos.y,
                RGB::named(rltk::WHITE),
                RGB::named(rltk::GRAY),
                &"<-".to_string(),
            );
        }
    }
}

pub fn ranged_target(
    gs: &mut State,
    ctx: &mut Rltk,
    range: i32,
) -> (ItemMenuResult, Option<Point>) {
    let player_entity = gs.ecs.fetch::<Entity>();
    let player_pos = gs.ecs.fetch::<Point>();
    let viewsheds = gs.ecs.read_storage::<Viewshed>();

    ctx.print_color(
        5,
        0,
        RGB::named(rltk::YELLOW),
        RGB::named(rltk::BLACK),
        "Select Target",
    );

    let mut available_cells = Vec::new();
    let visible = viewsheds.get(*player_entity);
    if let Some(visible) = visible {
        for i in visible.visible_tiles.iter() {
            let distance = rltk::DistanceAlg::Pythagoras.distance2d(*player_pos, *i);
            if distance <= range as f32 {
                ctx.set_bg(i.x, i.y, RGB::named(rltk::BLUE));
                available_cells.push(i);
            }
        }
    } else {
        return (ItemMenuResult::Cancel, None);
    }

    let (mouse_x, mouse_y) = ctx.mouse_pos();
    let valid_target = available_cells
        .iter()
        .any(|idx| idx.x == mouse_x && idx.y == mouse_y);

    if valid_target {
        ctx.set_bg(mouse_x, mouse_y, RGB::named(rltk::CYAN));
        if ctx.left_click {
            console::log("Target acquired!".to_string());
            return (ItemMenuResult::Selected, Some(Point::new(mouse_x, mouse_y)));
        }
    } else {
        ctx.set_bg(mouse_x, mouse_y, RGB::named(rltk::RED));
        if ctx.left_click {
            console::log("No valid target".to_string());
            return (ItemMenuResult::Cancel, None);
        }
    }

    (ItemMenuResult::NoResponse, None)
}
