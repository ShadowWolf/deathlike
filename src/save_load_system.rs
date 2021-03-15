use crate::*;
use serde_json::de::StrRead;
use serde_json::{Deserializer, Serializer};
use specs::error::NoError;
use specs::prelude::*;
use specs::saveload::{
    DeserializeComponents, MarkedBuilder, SerializeComponents, SimpleMarker, SimpleMarkerAllocator,
};
use std::fs;
use std::fs::File;
use std::path::Path;

macro_rules! serialize_individual_items {
    ($ecs: expr, $ser: expr, $data: expr, $( $type: ty ), *) => {
        $(
            SerializeComponents::<NoError, SimpleMarker<Savable>>::serialize(
                &( $ecs.read_storage::<$type>(), ),
                &$data.0,
                &$data.1,
                &mut $ser,
            )
            .unwrap();
        )
        *
    };
}

macro_rules! deserialize_individual_items {
    ($ecs: expr, $de: expr, $data: expr, $( $type: ty ), *) => {
        $(
            DeserializeComponents::<NoError, _>::deserialize(
                &mut ( &mut $ecs.write_storage::<$type>(), ),
                &$data.0,
                &mut $data.1,
                &mut $data.2,
                &mut $de,
            )
            .unwrap();
        )
        *
    };
}

pub fn save_game(ecs: &mut World) {
    let map_copy = ecs.get_mut::<Map>().unwrap().clone();
    let save_helper = ecs
        .create_entity()
        .with(SerializationHelper { map: map_copy })
        .marked::<SimpleMarker<Savable>>()
        .build();

    {
        let data = (ecs.entities(), ecs.read_storage::<SimpleMarker<Savable>>());

        let writer = File::create("./savegame.json").unwrap();
        let mut serializer = Serializer::new(writer);
        serialize_individual_items!(
            ecs,
            serializer,
            data,
            Position,
            Renderable,
            Player,
            Viewshed,
            Monster,
            Name,
            BlocksTile,
            CombatStats,
            SufferDamage,
            WantsToMelee,
            Item,
            Consumable,
            Ranged,
            InflictsDamage,
            AreaOfEffect,
            Confusion,
            ProvidesHealing,
            InBackpack,
            WantsToPickupItem,
            WantsToUseItem,
            WantsToDropItem,
            SerializationHelper
        );
    }

    ecs.delete_entity(save_helper)
        .expect("failed to cleanup saver");
}

pub fn save_exists() -> bool {
    Path::new("./savegame.json").exists()
}

pub fn load_game(ecs: &mut World) {
    clear_game_world(ecs);

    let data = fs::read_to_string("./savegame.json").unwrap();
    let mut deserializer = serde_json::Deserializer::from_str(&data);

    load_game_resources(ecs, &mut deserializer);

    let delete_me = populate_world_from_save_file(ecs);

    ecs.delete_entity(delete_me.unwrap())
        .expect("could not delete helper");
}

fn populate_world_from_save_file(ecs: &mut World) -> Option<Entity> {
    let entities = ecs.entities();
    let helper = ecs.read_storage::<SerializationHelper>();
    let player = ecs.read_storage::<Player>();
    let position = ecs.read_storage::<Position>();

    let mut delete_me: Option<Entity> = None;

    for (e, h) in (&entities, &helper).join() {
        let mut world_map = ecs.write_resource::<Map>();
        *world_map = h.map.clone();
        world_map.tile_content = vec![Vec::new(); MAP_COUNT];
        delete_me = Some(e);
    }

    let mut player_assigned = 0;

    for (e, _p, pos) in (&entities, &player, &position).join() {
        let mut player_position = ecs.write_resource::<Point>();
        *player_position = Point::new(pos.x, pos.y);
        let mut player_entity = ecs.write_resource::<Entity>();
        *player_entity = e;
        player_assigned += 1;
    }

    if player_assigned == 0 {
        rltk::console::log("Did not assign any player resources");
    }

    delete_me
}

fn load_game_resources(ecs: &mut World, deserializer: &mut Deserializer<StrRead>) {
    let mut d = (
        &mut ecs.entities(),
        &mut ecs.write_storage::<SimpleMarker<Savable>>(),
        &mut ecs.write_resource::<SimpleMarkerAllocator<Savable>>(),
    );

    deserialize_individual_items!(
        ecs,
        *deserializer,
        d,
        Position,
        Renderable,
        Player,
        Viewshed,
        Monster,
        Name,
        BlocksTile,
        CombatStats,
        SufferDamage,
        WantsToMelee,
        Item,
        Consumable,
        Ranged,
        InflictsDamage,
        AreaOfEffect,
        Confusion,
        ProvidesHealing,
        InBackpack,
        WantsToPickupItem,
        WantsToUseItem,
        WantsToDropItem,
        SerializationHelper
    );
}

fn clear_game_world(ecs: &mut World) {
    let mut to_delete = Vec::new();
    for e in ecs.entities().join() {
        to_delete.push(e);
    }
    for del in to_delete.iter() {
        ecs.delete_entity(*del).expect("Could not delete item");
    }
}
