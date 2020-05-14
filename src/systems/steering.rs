use crate::components;
use specs::*;
use crate::util::*;
use cgmath::{InnerSpace, Vector3, Zero};
use super::*;

pub struct ApplyVelocitySystem;

impl<'a> System<'a> for ApplyVelocitySystem {
    type SystemData = (
        Read<'a, Paused>,
        WriteStorage<'a, Position>,
        ReadStorage<'a, Velocity>
    );

    fn run(&mut self, (paused, mut position, velocity): Self::SystemData) {
        if paused.0 {
            return;
        }

        for (position, velocity) in (&mut position, &velocity).join() {
            position.0 += velocity.0;
        }
    }
}

pub struct SetRotationSystem;

impl<'a> System<'a> for SetRotationSystem {
    type SystemData = (
        WriteStorage<'a, components::Rotation>,
        ReadStorage<'a, Velocity>
    );

    fn run(&mut self, (mut rotation, velocity): Self::SystemData) {
        for (rotation, velocity) in (&mut rotation, &velocity).join() {
            if velocity.magnitude() > 0.0 {
                rotation.0 = look_at(velocity.0);
            }
        }
    }
}

// technically steering _and_ arrival
pub struct SeekSystem;

impl<'a> System<'a> for SeekSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, SeekForce>,
        ReadStorage<'a, Velocity>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, SeekPosition>,
        ReadStorage<'a, MaxSpeed>
    );

    fn run(&mut self, (entities, mut seek, vel, pos, seek_pos, speed): Self::SystemData) {
        for (entity, vel, pos, seek_pos, speed) in (&entities, &vel, &pos, &seek_pos, &speed).join() {
            let force = seek_and_arrival_force(pos.0, vel.0, seek_pos, speed.0, 0.01);
            seek.insert(entity, SeekForce(force)).unwrap();
        }
    }
}

pub struct FinishSeekSystem;

impl<'a> System<'a> for FinishSeekSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, SeekForce>,
        WriteStorage<'a, SeekPosition>,
        WriteStorage<'a, Velocity>,
        ReadStorage<'a, Position>
    );

    fn run(&mut self, (entities, mut seek, mut seek_pos, mut vel, pos): Self::SystemData) {
        for (entity, vel, pos) in (&entities, &mut vel, &pos).join() {
            if seek_pos.get(entity).map(|seek| seek.close_enough(pos.0) && seek.last_point()).unwrap_or(false) {
                seek.remove(entity);
                seek_pos.remove(entity);
                vel.0 = Vector3::zero();
            }
        }
    }
}

pub struct AvoidanceSystem;

impl<'a> System<'a> for AvoidanceSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, AvoidanceForce>,
        ReadStorage<'a, Velocity>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, MaxSpeed>,
        ReadStorage<'a, Size>,
        ReadStorage<'a, NoCollide>
    );

    fn run(&mut self, (entities, mut avoidance, vel, positions, speed, sizes, nocollide): Self::SystemData) {
        // collect the entity positions into a vec to avoid having to deref the ecs storage (which can be slow)
        // Also dont collect entities with an image because having images push entities about seems kinda wierd
        let entity_positions: Vec<_> = (&positions, &sizes, !&nocollide).join().collect();

        let iterator = entity_positions.iter().map(|(pos, size, _)| (pos.0, size.0));

        for (entity, vel, pos, speed, size) in (&entities, &vel, &positions, &speed, &sizes).join() {
            let force = avoidance_force(pos.0, vel.0, size.0, iterator.clone(), speed.0, 0.01);

            avoidance.insert(entity, AvoidanceForce(force)).unwrap();
        }
    }
}

pub struct FrictionSystem;

impl<'a> System<'a> for FrictionSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, FrictionForce>,
        ReadStorage<'a, Velocity>,
        ReadStorage<'a, SeekPosition>
    );

    fn run(&mut self, (entities, mut friction, vel, seek): Self::SystemData) {
        for (entity, vel) in (&entities, &vel).join() {
            let force = if seek.get(entity).is_none() {
                calc_force(vel.0, Vector3::zero(), 0.01)                
            } else {
                Vector3::zero()
            };            

            friction.insert(entity, FrictionForce(force)).unwrap();
        }
    }
}

pub struct MergeForceSystem;

impl<'a> System<'a> for MergeForceSystem {
    type SystemData = (
        Entities<'a>,
        Read<'a, Paused>,
        WriteStorage<'a, Velocity>,
        ReadStorage<'a, SeekForce>,
        ReadStorage<'a, AvoidanceForce>,
        ReadStorage<'a, FrictionForce>,
        ReadStorage<'a, MaxSpeed>
    );

    fn run(&mut self, (entities, paused, mut vel, seek, avoid, friction, speed): Self::SystemData) {
        if paused.0 {
            return;
        }

        for (entity, vel, avoid, friction, speed) in (&entities, &mut vel, &avoid, &friction, &speed).join() {
            let seek = seek.get(entity).map(|seek| seek.0).unwrap_or_else(Vector3::zero);

            let combined = seek + avoid.0 * 10.0 + friction.0;
            let combined = limit_vector(combined, 0.01);
            vel.0 = limit_vector(vel.0 + combined, speed.0);
        }
    }
}

fn calc_force(vel: Vector3<f32>, desired: Vector3<f32>, max_force: f32) -> Vector3<f32> {
    let steering = desired - vel;
    limit_vector(steering, max_force)
}

fn seek_and_arrival_force(pos: Vector3<f32>, vel: Vector3<f32>, seek_pos: &SeekPosition, max_speed: f32, max_force: f32) -> Vector3<f32> {
    let braking_distance = vel.magnitude2() / (2.0 * max_force);

    let delta = seek_pos.delta(pos);

    let desired = if delta.magnitude() < braking_distance && seek_pos.last_point() {
        Vector3::zero()
    } else {
        delta.normalize_to(max_speed)
    };

    calc_force(vel, desired, max_force)
}

fn avoidance_force<I: Iterator<Item=(Vector3<f32>, f32)>>(pos: Vector3<f32>, vel: Vector3<f32>, size: f32, iterator: I, max_speed: f32, max_force: f32) -> Vector3<f32> {
    let iterator = iterator
        .map(|(p, s)| (p, s, pos.distance(p)))
        .filter(|(_, s, distance)| *distance > 0.0 && *distance < (size + s))
        .map(|(p, _, distance)| (pos - p).normalize_to(1.0 / distance));

    if let Some(avg) = avg(iterator) {
        let desired = avg.normalize_to(max_speed);
        calc_force(vel, desired, max_force)
    } else {
        Vector3::zero()
    }
}
