#![cfg(test)]

use cgmath::*;
use components::*;
use specs::saveload::*;
use specs::*;
use systems::*;
use *;

fn get_data<C: Component + Clone>(world: &World, entity: Entity) -> Option<C> {
    let data: ReadStorage<C> = world.system_data();
    data.get(entity).cloned()
}

#[test]
fn test_velocity() {
    let mut world = create_world();

    let entity = world
        .create_entity()
        .with(Position(Vector3::new(1.0, 2.0, 3.0)))
        .with(Velocity(Vector3::new(-0.1, -0.1, -0.1)))
        .build();

    ApplyVelocitySystem.run_now(&world);

    world.maintain();

    assert_eq!(
        get_data(&world, entity),
        Some(Position(Vector3::new(0.9, 1.9, 2.9)))
    );
}

#[test]
fn test_saveload() {
    let mut world_a = create_world();
    let world_b = create_world();

    let entity = world_a
        .create_entity()
        .with(Velocity(Vector3::new(1.0, 2.0, 3.0)))
        //.with(TimeLeft(2.0))
        //.with(DrillSpeed(9999999.233))
        .marked::<Marker>()
        .build();

    let mut controls = Controls::default();
    controls.save = true;
    controls.load = true;

    *world_a.write_resource() = controls.clone();
    *world_b.write_resource() = controls;

    SaveSystem.run_now(&world_a.res);
    LoadSystem.run_now(&world_b.res);

    assert_eq!(
        get_data(&world_b, entity),
        Some(Velocity(Vector3::new(1.0, 2.0, 3.0)))
    );
}
