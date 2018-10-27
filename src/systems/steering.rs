use components::{self, *};
use specs::*;
use util::*;
use cgmath::{InnerSpace, Vector3, Zero};

pub struct ApplyVelocitySystem;

impl<'a> System<'a> for ApplyVelocitySystem {
    type SystemData = (
        WriteStorage<'a, Position>,
        ReadStorage<'a, Velocity>
    );

    fn run(&mut self, (mut position, velocity): Self::SystemData) {
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
            if velocity.0.magnitude() > 0.0 {
                rotation.0 = look_at(velocity.0);
            }
        }
    }
}

// technically steering _and_ arrival
pub struct SeekSystem;

impl<'a> System<'a> for SeekSystem {
    type SystemData = (
        WriteStorage<'a, Velocity>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, SeekPosition>,
        ReadStorage<'a, MaxSpeed>
    );

    fn run(&mut self, (mut vel, pos, seek, speed): Self::SystemData) {
        for (vel, pos, seek, speed) in (&mut vel, &pos, &seek, &speed).join() {
            let max_speed = speed.0;
            let max_acceleration = 0.01;

            let braking_distance = vel.0.magnitude2() / (2.0 * max_acceleration);

            let delta = seek.delta(pos.0);

            let desired = if delta.magnitude() < braking_distance {
                Vector3::zero()
            } else {
                delta * max_speed
            };

            let steering = desired - vel.0;
            let steering = truncate_vector(steering, max_acceleration);

            vel.0 = truncate_vector(vel.0 + steering, max_speed);
        }
    }
}

pub struct FinishSeekSystem;

impl<'a> System<'a> for FinishSeekSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, SeekPosition>,
        WriteStorage<'a, Velocity>,
        ReadStorage<'a, Position>
    );

    fn run(&mut self, (entities, mut seek, mut vel, pos): Self::SystemData) {
        for (entity, vel, pos) in (&entities, &mut vel, &pos).join() {
            if seek.get(entity).map(|seek| seek.close_enough(pos.0)).unwrap_or(false) {
                seek.remove(entity);
                vel.0 = Vector3::zero();
            }
        }
    }
}