use super::*;
use cgmath::Zero;
use crate::{Marker, MarkerAllocator};

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
        Write<'a, MarkerAllocator>,
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
        WriteStorage<'a, NoCollide>,
        WriteStorage<'a, ExplosionSize>,

        WriteStorage<'a, Marker>
    );

    fn run(&mut self, (
        entities, mut allocator,
        mut attack,
        mut pos, mut rot, mut vel, mut size, mut model, mut time, mut selectable, mut side, mut smoke, mut target, mut speed, mut health, mut nocollide, mut explosion_size,
        mut markers
    ): Self::SystemData) {

        for (entity, attack) in (&entities, &mut attack).join() {
            let entity_pos = pos.get(entity).unwrap().clone();
            let entity_rot = rot.get(entity).unwrap().clone();

            if let Some(target_entity) = target.get(entity).map(|target| target.entity) {
                let target_pos = pos.get(target_entity).unwrap().0;

                if attack.time != 0.0 || entity_pos.distance(target_pos) > attack.range + CLOSE_ENOUGH_DISTANCE * 2.0 {
                    continue;
                }

                attack.time = attack.delay;

                entities.build_entity()
                    .with(entity_pos, &mut pos)
                    .with(entity_rot, &mut rot)
                    .with(Velocity(Vector3::zero()), &mut vel)
                    .with(Size(0.1), &mut size)
                    .with(Model::Missile, &mut model)
                    .with(TimeLeft(20.0), &mut time)
                    .with(Selectable::new(false), &mut selectable)
                    .with(Side::Friendly, &mut side)
                    .with(SpawnSmoke(0), &mut smoke)
                    .with(AttackTarget {entity: target_entity, kamikaze: true}, &mut target)
                    .with(MaxSpeed(5.0), &mut speed)
                    .with(Health(1.0), &mut health)
                    .with(NoCollide, &mut nocollide)
                    .with(ExplosionSize(10.0), &mut explosion_size)
                    .marked(&mut markers, &mut allocator)
                    .build();
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
        Write<'a, MarkerAllocator>,

        WriteStorage<'a, SpawnSmoke>,
        ReadStorage<'a, Velocity>,
        
        WriteStorage<'a, Position>,
        WriteStorage<'a, Size>,
        WriteStorage<'a, TimeLeft>,
        WriteStorage<'a, Image>,
        WriteStorage<'a, NoCollide>,

        WriteStorage<'a, Marker>
    );

    fn run(&mut self, (entities, paused, mut allocator, mut smoke, vel, mut pos, mut size, mut time, mut image, mut nocollide, mut markers): Self::SystemData) {
        if paused.0 {
            return;
        }

        for (entity, vel, mut smoke) in (&entities, &vel, &mut smoke).join() {
            if let Some(p) = pos.get(entity).map(|p| p.0) {
                if smoke.0 % 2 == 0 {
                    entities.build_entity()
                        .with(Position(p - vel.0), &mut pos)
                        .with(Size(2.0), &mut size)
                        .with(TimeLeft(2.0), &mut time)
                        .with(Image::Smoke, &mut image)
                        .with(NoCollide, &mut nocollide)
                        .marked(&mut markers, &mut allocator)
                        .build();
                }

                smoke.0 = (smoke.0 + 1) % 2;
            }
        }
    }
}

pub struct KamikazeSystem;

impl<'a> System<'a> for KamikazeSystem {
    type SystemData = (
        Entities<'a>,
        Read<'a, Meshes>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, Size>,
        ReadStorage<'a, components::Rotation>,
        ReadStorage<'a, AttackTarget>,
        ReadStorage<'a, Model>,
        WriteStorage<'a, SeekPosition>,
        WriteStorage<'a, Health>
    );

    fn run(&mut self, (entities, meshes, position, size, rotation, target, model, mut seek, mut health): Self::SystemData) {
        for (entity, target) in (&entities, &target).join() {
            if target.kamikaze {
                let entity_pos = position.get(entity).unwrap().0;
                let entity_rot = rotation.get(entity).unwrap().0;
                let entity_size = size.get(entity).unwrap().0;
                let entity_model = model.get(entity).unwrap();

                if entities.is_alive(target.entity) {
                    let target_pos = position.get(target.entity).unwrap().0;
                    let target_rot = rotation.get(target.entity).unwrap().0;
                    let target_size = size.get(target.entity).unwrap().0;
                    let target_model = model.get(target.entity).unwrap();

                    if meshes.intersects(*entity_model, entity_pos, entity_rot, entity_size, *target_model, target_pos, target_rot, target_size) {
                        health.get_mut(target.entity).unwrap().0 -= 25.0;
                        health.get_mut(entity).unwrap().0 = 0.0;
                    } else {
                        seek.insert(entity, SeekPosition::to_point(target_pos, false)).unwrap();
                    }
                } else {
                    seek.remove(entity);
                }
            }
        }
    }
}

pub struct DestroyShips;

impl<'a> System<'a> for DestroyShips {
    type SystemData = (
        Entities<'a>,
        Write<'a, MarkerAllocator>,

        ReadStorage<'a, Health>,
        ReadStorage<'a, ExplosionSize>,
        
        WriteStorage<'a, Position>,
        WriteStorage<'a, Size>,
        WriteStorage<'a, TimeLeft>,
        WriteStorage<'a, NoCollide>,
        WriteStorage<'a, Explosion>,

        WriteStorage<'a, Marker>,
        
        ReadStorage<'a, Parent>
    );

    fn run(&mut self, (entities, mut allocator, health, explosion_size, mut position, mut size, mut time, mut nocollide, mut explosion, mut markers, parents): Self::SystemData) {
        for (entity, health) in (&entities, &health).join() {
            if health.0 <= 0.0 {
                delete_entity(entity, &entities, &parents);
                if let Some(pos) = position.get(entity).cloned() {
                    let explosion_size = explosion_size.get(entity).map(|size| size.0)
                        .or(size.get(entity).map(|size| size.0 * 2.0))
                        .unwrap_or(10.0);
                    
                    create_explosion(pos.0, explosion_size, &entities, &mut position, &mut size, &mut time, &mut nocollide, &mut explosion, &mut markers, &mut allocator);
                }
            }
        }
    }
}

fn create_explosion(
    position: Vector3<f32>, explosion_size: f32,
    entities: &Entities, pos: &mut WriteStorage<Position>, size: &mut WriteStorage<Size>, time: &mut WriteStorage<TimeLeft>, nocollide: &mut WriteStorage<NoCollide>, explosion: &mut WriteStorage<Explosion>,
    markers: &mut WriteStorage<Marker>, allocator: &mut Write<MarkerAllocator>
) -> Entity {
    entities.build_entity()
        .with(Position(position), pos)
        .with(Size(explosion_size), size)
        .with(TimeLeft(1.0), time)
        .with(NoCollide, nocollide)
        .with(Explosion, explosion)
        .marked(markers, allocator)
        .build()
}

pub struct StepExplosion;

impl<'a> System<'a> for StepExplosion {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, TimeLeft>,
        ReadStorage<'a, Explosion>,
        WriteStorage<'a, Image>,
    );

    fn run(&mut self, (entities, time, explosion, mut image): Self::SystemData) {
        for (entity, _) in (&entities, &explosion).join() {
            let time = time.get(entity).unwrap().0;

            let percentage = time / 1.0;

            let im = if percentage > 5.0 / 6.0 {
                Image::Explosion1
            } else if percentage > 4.0 / 6.0 {
                Image::Explosion2
            } else if percentage > 3.0 / 6.0 {
                Image::Explosion3
            } else if percentage > 2.0 / 6.0 {
                Image::Explosion4
            } else if percentage > 1.0 / 6.0 {
                Image::Explosion5
            } else {
                Image::Explosion6
            };

            image.insert(entity, im).unwrap();
        }
    }
}
