use common_components::*;
use specs::{World, Builder, Entity};
use ships::*;
use cgmath::{Vector3, Quaternion, Zero};
use rand::*;
use context::*;

pub fn create_ship(world: &mut World, tag: ShipType, position: Vector3<f32>, side: Side) -> Entity {
    let components = tag.default_components(0);

    world.create_entity()
        .with(Position(position))
        .with(Size(tag.size()))
        .with(tag.model())
        .with(tag)
        .with(Rotation(Quaternion::zero()))
        .with(Commands(Vec::new()))
        .with(ShipStorage {
            food: StoredResource::empty(500.0),
            materials: StoredResource::empty(500.0),
            waste: StoredResource::full(1000.0)
        })
        .with(Fuel(StoredResource::full(components.fuel_capacity())))
        .with(components)
        .with(Selectable::new(false))
        .with(side)
        .build()

}


pub fn create_person(parent: Entity, world: &mut World, occupation: Occupation) {
    world.create_entity()
        .with(CreationTime::from_age(30))
        .with(occupation)
        .with(Parent(parent))
        .build();
}

pub fn add_asteroid(rng: &mut ThreadRng, world: &mut World) {
    let size: f32 = rng.gen_range(5.0, 50.0);

    let x = rng.gen_range(500.0, 1000.0) * rng.gen_range(-1.0, 1.0);
    let y = rng.gen_range(-100.0, 100.0);
    let z = rng.gen_range(500.0, 1000.0) * rng.gen_range(-1.0, 1.0);

    let location = Vector3::new(x, y, z);

    let resources = (size.powi(3) * rng.gen_range(0.1, 1.0)) as u32;

    let spin = ObjectSpin::random(rng);

    world.create_entity()
        .with(Model::Asteroid)
        .with(spin)
        .with(Position(location))
        .with(MineableMaterials(resources))
        .with(Size(size))
        .with(Selectable::new(false))
        .with(Side::Neutral)
        .build();
}

pub fn add_starting_entities(world: &mut World) {
    let carrier = create_ship(world, ShipType::Carrier, Vector3::new(0.0, 0.0, 1.0), Side::Friendly);

    for _ in 0 .. 45 {
        create_person(carrier, world, Occupation::Worker);
    }

    for _ in 0 .. 20 {
        create_person(carrier, world, Occupation::Marine);
    }

    for _ in 0 .. 25 {
        create_person(carrier, world, Occupation::Pilot);
    }

    for _ in 0 .. 10 {
        create_person(carrier, world, Occupation::Government);
    }

    let tanker = create_ship(world, ShipType::Tanker, Vector3::new(0.0, 0.0, -20.0), Side::Friendly);

    for _ in 0 .. 10 {
        create_person(tanker, world, Occupation::Worker);
    }
    
    for i in 0 .. 20 {
        let x = (50.0 - i as f32) * 3.0;
        let fighter = create_ship(world, ShipType::Fighter, Vector3::new(x, 5.0, 0.0), Side::Friendly);
        create_person(fighter, world, Occupation::Pilot);
    }

    for i in 0 .. 2 {
        create_ship(world, ShipType::Miner, Vector3::new(0.0, 2.5 - i as f32 * 15.0, 30.0), Side::Friendly);
    }

    create_ship(world, ShipType::Fighter, Vector3::new(100.0, 0.0, 100.0), Side::Enemy);
}