#![cfg(tests)]

use specs::*;
use components::*;
use systems::*;
use *;
use cgmath::*;

#[test]
fn test_velocity() {
    let mut world = World::new();

    setup_world(&mut world);

    let entity = world.create_entity()
        .with(Position(Vector3::new(1.0, 2.0, 3.0)))
        .with(Velocity(Vector3::new(-0.1, -0.1, -0.1)))
        .build();

    ApplyVelocitySystem.run_now(&world.res);

    let data: ReadStorage<Position> = world.system_data();

    assert_eq!(data.get(entity), Some(&Position(Vector3::new(0.9, 1.9, 2.9))));
}