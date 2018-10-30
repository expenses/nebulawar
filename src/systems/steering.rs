use components::{self, *};
use specs::*;
use util::*;
use cgmath::{InnerSpace, Vector3, Zero};
use resources::*;
use context::*;

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
            let max_speed = speed.0;
            let max_acceleration = 0.01;

            let braking_distance = vel.magnitude2() / (2.0 * max_acceleration);

            let delta = seek_pos.delta(pos.0);

            let desired = if delta.magnitude() < braking_distance && seek_pos.last_point() {
                Vector3::zero()
            } else {
                delta.normalize_to(max_speed)
            };

            let force = calc_force(vel.0, desired, max_acceleration);
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
        ReadStorage<'a, Image>
    );

    fn run(&mut self, (entities, mut avoidance, vel, positions, speed, sizes, image): Self::SystemData) {
        // collect the entity positions into a vec to avoid having to deref the ecs storage (which can be slow)
        // Also dont collect entities with an image because having images push entities about seems kinda wierd
        let entity_positions: Vec<_> = (&positions, &sizes, !&image).join().collect();

        for (entity, vel, pos, speed, size) in (&entities, &vel, &positions, &speed, &sizes).join() {
            let max_speed = speed.0;
            let max_acceleration = 0.01;

            let mut sum = Vector3::zero();
            let mut count = 0;

            for (p, s, _) in &entity_positions {
                let distance = pos.distance(&p.0);

                if distance > 0.0 && distance < (size.0 + s.0) {
                    let diff = (pos.0 - p.0).normalize_to(1.0 / distance);
                    sum += diff;
                    count += 1;
                }
            }

            let force = if count > 0 {
                let desired = sum.normalize_to(max_speed);
                calc_force(vel.0, desired, max_acceleration)
            } else {
                Vector3::zero()
            };

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