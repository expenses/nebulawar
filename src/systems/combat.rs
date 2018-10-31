use super::*;

pub struct TestDeleteSystem;

impl<'a> System<'a> for TestDeleteSystem {
    type SystemData = (
        Read<'a, Controls>,
        WriteStorage<'a, Selectable>,
        WriteStorage<'a, Health>
    );

    fn run(&mut self, (controls, selectable, mut health): Self::SystemData) {
        if controls.delete {
            (&mut health, &selectable).join()
                .filter(|(_, selectable)| selectable.selected)
                .for_each(|(health, _)| {
                    health.0 = 0.0;
                });
        }
    }
}

pub struct ShootStuffSystem;

impl<'a> System<'a> for ShootStuffSystem {
    type SystemData = (
        Entities<'a>,
        Write<'a, U64MarkerAllocator>,
        WriteStorage<'a, CanAttack>,
        
        WriteStorage<'a, Position>,
        WriteStorage<'a, components::Rotation>,
        WriteStorage<'a, Velocity>,
        WriteStorage<'a, Size>,
        WriteStorage<'a, Model>,
        WriteStorage<'a, TimeLeft>,
        WriteStorage<'a, Selectable>,
        WriteStorage<'a, Side>,
        WriteStorage<'a, SpawnSmoke>,
        WriteStorage<'a, AttackTarget>,
        WriteStorage<'a, MaxSpeed>,
        WriteStorage<'a, Health>,

        WriteStorage<'a, U64Marker>
    );

    fn run(&mut self, (
        entities, mut allocator,
        mut attack,
        mut pos, mut rot, mut vel, mut size, mut model, mut time, mut selectable, mut side, mut smoke, mut target, mut speed, mut health,
        mut markers
    ): Self::SystemData) {

        for (entity, attack) in (&entities, &mut attack).join() {
            if let Some(p) = pos.get(entity).cloned() {
                if let Some(target_entity) = target.get(entity).map(|target| target.entity) {
                    let target_pos = pos.get(target_entity).unwrap().0;

                    if attack.time != 0.0 || p.distance(target_pos) > attack.range + CLOSE_ENOUGH_DISTANCE * 2.0 {
                        continue;
                    }

                    attack.time = attack.delay;

                    entities.build_entity()
                        .with(p, &mut pos)
                        .with(Rotation(Quaternion::zero()), &mut rot)
                        .with(Velocity(Vector3::zero()), &mut vel)
                        .with(Size(0.1), &mut size)
                        .with(Model::Missile, &mut model)
                        .with(TimeLeft(20.0), &mut time)
                        .with(Selectable::new(false), &mut selectable)
                        .with(Side::Friendly, &mut side)
                        .with(SpawnSmoke, &mut smoke)
                        .with(AttackTarget {entity: target_entity, kamikaze: true}, &mut target)
                        .with(MaxSpeed(5.0), &mut speed)
                        .with(Health(1.0), &mut health)
                        .marked(&mut markers, &mut allocator)
                        .build();
                }
            }
        }
    }
}

pub struct ReduceAttackTime;

impl<'a> System<'a> for ReduceAttackTime {
    type SystemData = (
        Read<'a, Secs>,
        Read<'a, Paused>,
        WriteStorage<'a, CanAttack>,
    );

    fn run(&mut self, (secs, paused, mut attack): Self::SystemData) {
        if paused.0 {
            return;
        }

        for attack in (&mut attack).join() {
            attack.time = move_towards(attack.time, 0.0, secs.0);
        }
    }
}

pub struct SpawnSmokeSystem;

impl<'a> System<'a> for SpawnSmokeSystem {
    type SystemData = (
        Entities<'a>,
        Read<'a, Paused>,
        Write<'a, U64MarkerAllocator>,

        ReadStorage<'a, SpawnSmoke>,
        ReadStorage<'a, Velocity>,
        
        WriteStorage<'a, Position>,
        WriteStorage<'a, Size>,
        WriteStorage<'a, TimeLeft>,
        WriteStorage<'a, Image>,
        WriteStorage<'a, U64Marker>
    );

    fn run(&mut self, (entities, paused, mut allocator, smoke, vel, mut pos, mut size, mut time, mut image, mut markers): Self::SystemData) {
        if paused.0 {
            return;
        }

        for (entity, vel, _) in (&entities, &vel, &smoke).join() {
            if let Some(p) = pos.get(entity).map(|p| p.0) {
                entities.build_entity()
                    .with(Position(p - vel.0), &mut pos)
                    .with(Size(2.0), &mut size)
                    .with(TimeLeft(2.0), &mut time)
                    .with(Image::Smoke, &mut image)
                    .marked(&mut markers, &mut allocator)
                    .build();
            }
        }
    }
}

pub struct KamikazeSystem;

impl<'a> System<'a> for KamikazeSystem {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, Size>,
        ReadStorage<'a, AttackTarget>,
        WriteStorage<'a, SeekPosition>,
        WriteStorage<'a, Health>
    );

    fn run(&mut self, (entities, position, size, target, mut seek, mut health): Self::SystemData) {
        for (entity, target) in (&entities, &target).join() {
            if target.kamikaze {
                if let Some(pos) = position.get(entity).cloned() {
                    if let Some(target_pos) = position.get(target.entity).cloned() {
                        if pos.distance(target_pos.0) < size.get(target.entity).unwrap().0 * 2.0 {
                            health.get_mut(target.entity).unwrap().0 -= 25.0;
                            health.get_mut(entity).unwrap().0 = 0.0;
                        } else {
                            seek.insert(entity, SeekPosition::to_point(target_pos.0, false)).unwrap();
                        }
                    }
                }
            }
        }
    }
}

pub struct DestroyShips;

impl<'a> System<'a> for DestroyShips {
    type SystemData = (
        Entities<'a>,
        Write<'a, U64MarkerAllocator>,

        ReadStorage<'a, Health>,
        
        WriteStorage<'a, Position>,
        WriteStorage<'a, Size>,
        WriteStorage<'a, TimeLeft>,
        WriteStorage<'a, Image>,
        WriteStorage<'a, U64Marker>,
        
        ReadStorage<'a, Parent>
    );

    fn run(&mut self, (entities, mut allocator, health, mut position, mut size, mut time, mut image, mut markers, parents): Self::SystemData) {
        for (entity, health) in (&entities, &health).join() {
            if health.0 <= 0.0 {
                delete_entity(entity, &entities, &parents);
                if let Some(pos) = position.get(entity).cloned() {
                    let explosion_size = size.get(entity).map(|size| size.0).unwrap_or(5.0) * 2.0;
                    create_explosion(pos.0, explosion_size, &entities, &mut position, &mut size, &mut time, &mut image, &mut markers, &mut allocator);
                }
            }
        }
    }
}

fn create_explosion(
    position: Vector3<f32>, explosion_size: f32,
    entities: &Entities, pos: &mut WriteStorage<Position>, size: &mut WriteStorage<Size>, time: &mut WriteStorage<TimeLeft>, image: &mut WriteStorage<Image>,
    markers: &mut WriteStorage<U64Marker>, allocator: &mut Write<U64MarkerAllocator>
) -> Entity {
    entities.build_entity()
        .with(Position(position), pos)
        .with(Size(explosion_size), size)
        .with(TimeLeft(2.0), time)
        .with(Image::Star, image)
        .marked(markers, allocator)
        .build()
}