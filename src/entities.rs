use crate::components::*;
use specs::{World, Builder, Entity};
use crate::ships::*;
use cgmath::{Vector3, Quaternion, Zero, Rotation};
use super::*;
use crate::context::*;
use specs::saveload::*;
use crate::util::*;

pub fn create_ship(world: &mut World, tag: ShipType, position: Vector3<f32>, rotation: Quaternion<f32>, side: Side) -> Entity {
    let components = tag.default_components(0);

    let mut entity = world.create_entity()
        .with(Position(position))
        .with(Size(tag.size()))
        .with(tag.model())
        .with(MaxSpeed(components.thrust() / tag.mass()))
        .with(Health(tag.mass()))
        .with(tag)
        .with(Rotation(rotation))
        .with(Commands(Vec::new()))
        // todo: make materials optional
        .with(Materials(StoredResource::empty(500.0)))
        .with(Selectable::new(false))
        .with(Velocity(Vector3::zero()))
        .with(side)
        .with(CanAttack {
            delay: 1.0,
            time: 1.0,
            range: 50.0
        })
        .marked::<U64Marker>();

    if let Some(speed) = components.drill_speed() {
        entity = entity.with(DrillSpeed(speed));
    }

    entity.build()

}


pub fn create_person(parent: Entity, world: &mut World, occupation: Occupation) {
    world.create_entity()
        .with(CreationTime::from_age(30))
        .with(occupation)
        .with(Parent(parent))
        .marked::<U64Marker>()
        .build();
}

pub fn add_asteroid(rng: &mut ThreadRng, world: &mut World) {
    let size: f32 = rng.gen_range(5.0, 50.0);

    let x = rng.gen_range(500.0, 1000.0) * rng.gen_range(-1.0, 1.0);
    let y = rng.gen_range(-100.0, 100.0);
    let z = rng.gen_range(500.0, 1000.0) * rng.gen_range(-1.0, 1.0);

    let resources = size.powi(3) * rng.gen_range(0.1, 1.0);

    world.create_entity()
        .with(Model::Asteroid)
        .with(ObjectSpin::random(rng))
        .with(Position(Vector3::new(x, y, z)))
        .with(MineableMaterials(StoredResource::full(resources)))
        .with(Size(size))
        .with(Selectable::new(false))
        .with(Side::Neutral)
        .marked::<U64Marker>()
        .build();
}

pub fn add_starting_entities(world: &mut World, rng: &mut ThreadRng) {
    let carrier = create_ship(world, ShipType::Carrier, Vector3::new(0.0, 0.0, 1.0), Quaternion::zero(), Side::Friendly);

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

    let tanker = create_ship(world, ShipType::Tanker, Vector3::new(0.0, 0.0, -20.0), Quaternion::zero(), Side::Friendly);

    for _ in 0 .. 10 {
        create_person(tanker, world, Occupation::Worker);
    }
    
    for i in 0 .. 50 {
        let x = (50.0 - i as f32) * 3.0;
        let fighter = create_ship(world, ShipType::Fighter, Vector3::new(x, 5.0, 0.0), Quaternion::zero(), Side::Friendly);
        create_person(fighter, world, Occupation::Pilot);
    }

    for i in 0 .. 2 {
        create_ship(world, ShipType::Miner, Vector3::new(0.0, 2.5 - i as f32 * 15.0, 30.0), Quaternion::zero(), Side::Friendly);
    }

    create_ship(world, ShipType::Carrier, Vector3::new(100.0, 0.0, 100.0), Quaternion::between_vectors(UP, uniform_sphere_distribution(rng)), Side::Enemy);
}
