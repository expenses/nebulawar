use components::{self, *};
use specs::{*, saveload::*, error::*};
use camera::*;
use state::*;
use resources::*;
use ships::*;
use context::{Model, Image};
use std::fs::*;

type ComponentsA<'a> = (
    WriteStorage<'a, Position>,
    WriteStorage<'a, Velocity>,
    WriteStorage<'a, components::Rotation>,
    WriteStorage<'a, Size>,
    WriteStorage<'a, Selectable>,
    WriteStorage<'a, Model>,
    WriteStorage<'a, ObjectSpin>,
    WriteStorage<'a, Side>,
    WriteStorage<'a, Commands>,
    WriteStorage<'a, ShipType>,
    WriteStorage<'a, MaxSpeed>,
    WriteStorage<'a, Occupation>,
    WriteStorage<'a, Parent>,
    WriteStorage<'a, CreationTime>,
    WriteStorage<'a, DrillSpeed>,
    WriteStorage<'a, MineableMaterials>
);

type ComponentsB<'a> = (
    WriteStorage<'a, Materials>,
    WriteStorage<'a, TimeLeft>,
    WriteStorage<'a, Image>,
    WriteStorage<'a, CanAttack>
);

type ComponentsASerialized = <ComponentsA<'static> as SerializeComponents<Error, U64Marker>>::Data;
type ComponentsBSerialized = <ComponentsB<'static> as SerializeComponents<Error, U64Marker>>::Data;

pub struct SaveSystem;

impl<'a> System<'a> for SaveSystem {
    type SystemData = (
        Entities<'a>,

        Read<'a, Camera>,
        Read<'a, StarSystem>,
        Read<'a, Time>,
        Read<'a, Paused>,
        Read<'a, Formation>,
        Read<'a, Log>,
        Read<'a, MovementPlane>,

        ComponentsA<'a>,
        ComponentsB<'a>,
        
        ReadStorage<'a, U64Marker>
    );

    fn run(&mut self, (
        entities,
        cam, sys, time, paused, formation, log, plane,
        comp_a, comp_b,
        markers
    ): Self::SystemData) {
        let ids = |entity| markers.get(entity).cloned();

        let comp_a = (&entities, &markers).join()
            .map(|(entity, marker)| (marker, comp_a.serialize_entity(entity, ids)))
            .map(|(marker, result): (&U64Marker, Result<ComponentsASerialized, Error>)| {
                EntityData {
                    marker: *marker,
                    components: result.unwrap()
                }
            })
            .collect();

        let comp_b = (&entities, &markers).join()
            .map(|(entity, marker)| (marker, comp_b.serialize_entity(entity, ids)))
            .map(|(marker, result): (&U64Marker, Result<ComponentsBSerialized, Error>)| {
                EntityData {
                    marker: *marker,
                    components: result.unwrap()
                }
            })
            .collect();

        let data = GameData {
            camera: cam.clone(),
            system: sys.clone(),
            time: time.clone(),
            paused: paused.clone(),
            formation: formation.clone(),
            log: log.clone(),
            plane: plane.clone(),

            comp_a, comp_b
        };

        let game = File::create("save.sav").unwrap();

        bincode::serialize_into(game, &data).unwrap();
    }
}

pub struct LoadSystem;

impl<'a> System<'a> for LoadSystem {
    type SystemData = (
        Entities<'a>,
        Write<'a, U64MarkerAllocator>,

        Write<'a, Camera>,
        Write<'a, StarSystem>,
        Write<'a, Time>,
        Write<'a, Paused>,
        Write<'a, Formation>,
        Write<'a, Log>,
        Write<'a, MovementPlane>,

        ComponentsA<'a>,
        ComponentsB<'a>,

        WriteStorage<'a, U64Marker>
    );

    fn run(&mut self, (
        entities, mut allocator,
        mut camera, mut system, mut time, mut paused, mut formation, mut log, mut plane,
        mut comp_a, mut comp_b,
        mut markers
    ): Self::SystemData) {
        let file = File::open("save.sav").unwrap();

        let data: GameData = bincode::deserialize_from(file).unwrap();

        let mut func = |marker| allocator.retrieve_entity(marker, &mut markers, &entities);

        *time = data.time;
        *camera = data.camera;
        *system = data.system;
        *paused = data.paused;
        *formation = data.formation;
        *log = data.log;
        *plane = data.plane;

        data.comp_a.into_iter().for_each(|entity_data| {
            let result: Result<(), Error> = comp_a.deserialize_entity(func(entity_data.marker), entity_data.components, |e| Some(func(e)));
            result.unwrap();
        });

        data.comp_b.into_iter().for_each(|entity_data| {
            let result: Result<(), Error> = comp_b.deserialize_entity(func(entity_data.marker), entity_data.components, |e| Some(func(e)));
            result.unwrap();
        });
    }
}

#[derive(Serialize, Deserialize)]
struct GameData {
    camera: Camera,
    system: StarSystem,
    time: Time,
    paused: Paused,
    formation: Formation,
    log: Log,
    plane: MovementPlane,

    comp_a: Vec<EntityData<U64Marker, ComponentsASerialized>>,
    comp_b: Vec<EntityData<U64Marker, ComponentsBSerialized>>
}