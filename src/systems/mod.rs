use specs::{*, saveload::*};
use components::{self, *};
use context::*;
use camera::*;
use ships::*;
use cgmath::{Vector3, Quaternion, Zero, Matrix4, MetricSpace};
use util::*;
use collision::*;
use controls::Controls;
use glium::glutin::{WindowEvent, MouseScrollDelta, dpi::LogicalPosition};
use resources::*;

mod rendering;
mod storage;
mod steering;
mod saving;

pub use self::rendering::*;
pub use self::steering::*;
pub use self::saving::*;
use self::storage::*;

pub struct SpinSystem;

impl<'a> System<'a> for SpinSystem {
    type SystemData = (
        Entities<'a>,
        Read<'a, Secs>,
        Read<'a, Paused>,
        WriteStorage<'a, ObjectSpin>,
        WriteStorage<'a, components::Rotation>
    );

    fn run(&mut self, (entities, secs, paused, mut spins, mut rotations): Self::SystemData) {
        if paused.0 {
            return;
        }

        for (entity, spin) in (&entities, &mut spins).join() {
            spin.turn(secs.0);

            rotations.insert(
                entity,
                components::Rotation(spin.to_quat())
            ).unwrap();
        }
    }
}

pub struct DragSelectSystem;

impl<'a> System<'a> for DragSelectSystem {
    type SystemData = (
        Read<'a, Controls>,
        Read<'a, Camera>,
        Read<'a, ScreenDimensions>,
        ReadStorage<'a, Position>,
        WriteStorage<'a, Selectable>
    );

    fn run(&mut self, (controls, camera, screen_dims, pos, mut selectable): Self::SystemData) {
        if let Some((left, top, right, bottom)) = controls.left_drag_rect() {
            for (pos, selectable) in (&pos, &mut selectable).join() {
                if let Some((x, y, _)) = camera.screen_position(pos.0, screen_dims.0) {
                    let selected = x >= left && x <= right && y >= top && y <= bottom;
                    
                    if !controls.shift {
                        selectable.selected = selected;
                    } else if selected {
                        selectable.selected = !selectable.selected;
                    }
                } else if !controls.shift {
                    selectable.selected = false;
                }
            }
        }
    }
}

pub struct ShipMovementSystem;

impl<'a> System<'a> for ShipMovementSystem {
    type SystemData = (
        Entities<'a>,
        Read<'a, Paused>,
        WriteStorage<'a, Commands>,
        WriteStorage<'a, Materials>,
        WriteStorage<'a, MineableMaterials>,
        WriteStorage<'a, SeekPosition>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, Size>,
        ReadStorage<'a, DrillSpeed>
    );

    fn run(&mut self, (entities, paused, mut commands, mut materials, mut mineable, mut seek, pos, size, drill_speed): Self::SystemData) {
        if paused.0 {
            return;
        }

        for (entity, commands) in (&entities, &mut commands).join() {
            let last = commands.len() == 1;

            let finished = commands.first()
                .map(|command| handle_command(command, entity, &mut materials, &mut mineable, &size, &drill_speed, &pos, &mut seek, last).unwrap_or(true))
                .unwrap_or(false);
            
            if finished {
                commands.remove(0);
            }
        }
    }
}

fn handle_command(
    command: &Command,
    entity: Entity,
    materials: &mut WriteStorage<Materials>, mineable_materials: &mut WriteStorage<MineableMaterials>,
    size: &ReadStorage<Size>, drill_speed: &ReadStorage<DrillSpeed>, pos: &ReadStorage<Position>,
    seek: &mut WriteStorage<SeekPosition>, last: bool
) -> Option<bool> {
    
    let entity_position = pos.get(entity)?.0;

    match command {
        Command::MoveTo(position) => {
            let reached = close_enough(entity_position, *position);

            if !reached {
                seek.insert(entity, SeekPosition::to_point(*position, last)).unwrap();
            }

            Some(reached)
        },
        Command::GoToAnd(target, interaction) => {
            let target_position = pos.get(*target)?.0;

            let distance = size.get(*target)?.0 + size.get(entity)?.0;

            if entity_position.distance(target_position) - CLOSE_ENOUGH_DISTANCE < distance {
                match interaction {
                    Interaction::Follow => Some(false),
                    Interaction::Mine => {
                        transfer_between_different(mineable_materials, materials, *target, entity, drill_speed.get(entity).unwrap().0)
                    },
                    Interaction::Attack => Some(true),
                }
            } else {
                seek.insert(entity, SeekPosition::within_distance(target_position, distance, last)).unwrap();
                Some(false)
            }
        }
    }
}

pub struct RightClickSystem;

impl<'a> System<'a> for RightClickSystem {
    type SystemData = (
        Read<'a, RightClickOrder>,
        Read<'a, Controls>,
        Read<'a, Formation>,
        Read<'a, AveragePosition>,
        WriteStorage<'a, Commands>
    );

    fn run(&mut self, (order, controls, formation, avg_pos, mut commands): Self::SystemData) {
        if controls.right_clicked() {
            if let Some(ref command) = order.command {
                match command {
                    Command::GoToAnd(entity, interaction) => {
                        order.to_move.iter()
                            .for_each(|e| commands.get_mut(*e).unwrap().order(controls.shift, Command::GoToAnd(*entity, *interaction)));
                    },
                    Command::MoveTo(target) => {
                        if let Some(avg) = avg_pos.0 {
                            let positions = formation.arrange(order.to_move.len(), avg, *target, 4.0);

                            order.to_move.iter()
                                .zip(positions)
                                .for_each(|(entity, position)| commands.get_mut(*entity).unwrap().order(controls.shift, Command::MoveTo(position)));
                        }
                    }
                }
            }
        }
    }
}

pub fn focus_on_selected(world: &mut World) {
    world.exec(|mut selectable: WriteStorage<Selectable>| {
        (&mut selectable).join().for_each(|selectable| selectable.camera_following = selectable.selected)
    });
}

pub fn avg_position<I: Iterator<Item = Vector3<f32>>>(iterator: I) -> Option<Vector3<f32>> {
    let (len, sum) = iterator.fold((0, Vector3::zero()), |(len, sum), position| {
        (len + 1, sum + position)
    });

    if len > 0 {
        Some(sum / len as f32)
    } else {
        None
    }
}

pub struct StepCameraSystem;
    
impl<'a> System<'a> for StepCameraSystem {
    type SystemData = (
        Write<'a, Camera>,
        Read<'a, Controls>,
        ReadStorage<'a, Position>,
        WriteStorage<'a, Selectable>,
    );

    fn run(&mut self, (mut camera, controls, pos, mut selectable): Self::SystemData) {
        let mut clear = false;
        
        if controls.left {
            camera.move_sideways(-0.5);
            clear = true;
        }

        if controls.right {
            camera.move_sideways(0.5);
            clear = true;
        }

        if controls.forwards {
            camera.move_forwards(0.5);
            clear = true;
        }

        if controls.back {
            camera.move_forwards(-0.5);
            clear = true;
        }

        if clear {
            (&mut selectable).join()
                .for_each(|selectable| selectable.camera_following = false);
        }

        camera.step();

        let iterator = (&pos, &selectable).join()
            .filter(|(_, selectable)| selectable.camera_following)
            .map(|(pos, _)| pos.0);

        if let Some(position) = avg_position(iterator) {
            camera.move_towards(position);
        }
    }
}

pub struct TimeStepSystem;

impl<'a> System<'a> for TimeStepSystem {
    type SystemData = (
        Read<'a, Paused>,
        Write<'a, Time>,
        Read<'a, Secs>
    );

    fn run(&mut self, (paused, mut time, secs): Self::SystemData) {
        if paused.0 {
            return;
        }

        *time = Time(time.0 + secs.0);
    }
}

pub struct LeftClickSystem;

impl<'a> System<'a> for LeftClickSystem {
    type SystemData = (
        Read<'a, Controls>,
        Read<'a, EntityUnderMouse>,
        WriteStorage<'a, Selectable>
    );

    fn run(&mut self, (controls, entity, mut selectable): Self::SystemData) {
        if controls.left_clicked() {
            if !controls.shift {
                (&mut selectable).join().for_each(|selectable| selectable.selected = false);
            }

            if let Some((entity, _)) = entity.0 {
                if let Some(selectable) = selectable.get_mut(entity) {
                    selectable.selected = !selectable.selected;
                }
            }
        }
    }
}

pub struct RightClickInteractionSystem;

impl<'a> System<'a> for RightClickInteractionSystem {
    type SystemData = (
        Entities<'a>,
        Write<'a, RightClickOrder>,
        Read<'a, EntityUnderMouse>,
        Read<'a, MovementPlane>,
        Read<'a, MouseRay>,
        ReadStorage<'a, MineableMaterials>,
        ReadStorage<'a, Side>,
        ReadStorage<'a, Selectable>,
        ReadStorage<'a, DrillSpeed>,
        ReadStorage<'a, Commands>
    );

    fn run(&mut self, (entities, mut order, entity, plane, ray, mineable, side, selectable, drill, commands): Self::SystemData) {
        let ordering = (&entities, &selectable, &side, &commands).join()
            .filter(|(_, selectable, side, _)| selectable.selected && **side == Side::Friendly)
            .map(|(entity, _, _, _)| entity);

        if let Some((entity, _)) = entity.0 {
            let interaction = if side.get(entity) == Some(&Side::Enemy) {
                order.to_move = ordering.collect();

                Interaction::Attack
            } else if mineable.get(entity).filter(|mineable| !mineable.is_empty()).is_some() {
                order.to_move = ordering.filter(|entity| drill.get(*entity).is_some()).collect();

                Interaction::Mine
            } else {
                Interaction::Follow
            };

            order.command = Some(Command::GoToAnd(entity, interaction));
        } else {
            order.command = Plane::new(UP, -plane.0).intersection(&ray.0)
                .map(|point| Command::MoveTo(point_to_vector(point)));

            order.to_move = ordering.collect();
        }
    }
}

pub struct EntityUnderMouseSystem<'a>(pub &'a Context);

impl<'a> System<'a> for EntityUnderMouseSystem<'a> {
    type SystemData = (
        Entities<'a>,
        Read<'a, Camera>,
        Read<'a, MouseRay>,
        Write<'a, EntityUnderMouse>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, components::Rotation>,
        ReadStorage<'a, Size>,
        ReadStorage<'a, Model>
    );

    fn run(&mut self, (entities, camera, ray, mut entity, pos, rot, size, model): Self::SystemData) {
        entity.0 = (&entities, &pos, &rot, &size, &model).join()
            .filter_map(|(entity, pos, rot, size, model)| {
                let rotation: Matrix4<f32> = rot.0.into();

                let transform = Matrix4::from_translation(pos.0) * rotation * Matrix4::from_scale(size.0);

                let mesh = self.0.collision_mesh(*model);
                
                let bound: Aabb3<f32> = mesh.compute_bound();

                if !bound.intersects_transformed(&ray.0, &transform) {
                    return None;
                }

                mesh.intersection_transformed(&ray.0, &transform)
                    .map(point_to_vector)
                    .map(|intersection| (entity, intersection, camera.position().distance2(intersection)))
            })
            .min_by(|(_, _, distance_a), (_, _, distance_b)| cmp_floats(*distance_a, *distance_b))
            .map(|(entity, intersection, _)| (entity, intersection));
    }
}

// todo: combat
// todo:recyc

pub struct UpdateControlsSystem;

impl<'a> System<'a> for UpdateControlsSystem {
    type SystemData = Write<'a, Controls>;

    fn run(&mut self, mut controls: Self::SystemData) {
        controls.update();
    }
}

pub struct MiddleClickSystem;

impl<'a> System<'a> for MiddleClickSystem {
    type SystemData = (
        Read<'a, Controls>,
        WriteStorage<'a, Selectable>
    );

    fn run(&mut self, (controls, mut selectable): Self::SystemData) {
        if controls.middle_clicked() {
            (&mut selectable).join()
                .for_each(|selectable| selectable.camera_following = selectable.selected);
        }
    }
}

pub struct AveragePositionSystem;

impl<'a> System<'a> for AveragePositionSystem {
    type SystemData = (
        Write<'a, AveragePosition>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, Selectable>,
        ReadStorage<'a, Side>
    );

    fn run(&mut self, (mut avg_pos, pos, selectable, side): Self::SystemData) {
        let iterator = (&pos, &selectable, &side).join()
            .filter(|(_, selectable, side)| selectable.selected && **side == Side::Friendly)
            .map(|(pos, _, _)| pos.0);

        avg_pos.0 = avg_position(iterator);
    }
}

pub struct EventHandlerSystem;

impl<'a> System<'a> for EventHandlerSystem {
    type SystemData = (
        Write<'a, Events>,
        Write<'a, Camera>,
        Write<'a, MovementPlane>,
        Write<'a, Controls>
    );

    fn run(&mut self, (mut events, mut camera, mut plane, mut controls): Self::SystemData) {
        events.drain(..).for_each(|event| match event {
            WindowEvent::CursorMoved {position: LogicalPosition {x, y}, ..} => {
                let (x, y) = (x as f32, y as f32);
                let (mouse_x, mouse_y) = controls.mouse();
                let (delta_x, delta_y) = (x - mouse_x, y - mouse_y);
                
                controls.set_mouse(x, y);

                if controls.right_dragging() {
                    camera.rotate_longitude(delta_x / 200.0);
                    camera.rotate_latitude(delta_y / 200.0);
                } else if controls.shift {
                    plane.0 -= delta_y / 10.0;
                }
            },
            WindowEvent::MouseWheel {delta, ..} => match delta {
                MouseScrollDelta::PixelDelta(LogicalPosition {y, ..}) => camera.change_distance(y as f32 / 20.0),
                MouseScrollDelta::LineDelta(_, y) => camera.change_distance(-y * 2.0)
            },
            _ => {}
        })
    }
}

pub struct StepLogSystem;

impl<'a> System<'a> for StepLogSystem {
    type SystemData = (
        Read<'a, Secs>,
        Read<'a, Paused>,
        Write<'a, Log>
    );

    fn run(&mut self, (secs, paused, mut log): Self::SystemData) {
        if paused.0 {
            return;
        }

        log.step(secs.0);
    }
}

pub struct TestDeleteSystem;

impl<'a> System<'a> for TestDeleteSystem {
    type SystemData = (
        Entities<'a>,
        Read<'a, Controls>,
        Write<'a, U64MarkerAllocator>,
        WriteStorage<'a, Selectable>,
        WriteStorage<'a, Position>,
        WriteStorage<'a, Size>,
        WriteStorage<'a, TimeLeft>,
        WriteStorage<'a, Image>,
        WriteStorage<'a, U64Marker>,
        ReadStorage<'a, Parent>
    );

    fn run(&mut self, (entities, controls, mut allocator, selectable, mut position, mut size, mut time, mut image, mut markers, parent): Self::SystemData) {
        if controls.delete {
            (&entities, &selectable).join()
                .filter(|(_, selectable)| selectable.selected)
                .for_each(|(entity, _)| {
                    let p = *position.get(entity).unwrap();
                    let s = *size.get(entity).unwrap();

                    delete_entity(entity, &entities, &parent);

                    entities.build_entity()
                        .with(p, &mut position)
                        .with(s, &mut size)
                        .with(TimeLeft(2.0), &mut time)
                        .with(Image::Star, &mut image)
                        .marked(&mut markers, &mut allocator)
                        .build();
                });
        }
    }
}

pub struct TickTimedEntities;

impl<'a> System<'a> for TickTimedEntities {
    type SystemData = (
        Entities<'a>,
        Read<'a, Secs>,
        Read<'a, Paused>,
        WriteStorage<'a, TimeLeft>,
        ReadStorage<'a, Parent>
    );

    fn run(&mut self, (entities, secs, paused, mut time, parents): Self::SystemData) {
        if paused.0 {
            return;
        }

        for (entity, time) in (&entities, &mut time).join() {
            time.0 -= secs.0;
            if time.0 < 0.0 {
                delete_entity(entity, &entities, &parents);
            }
        }
    }
}

pub struct SetMouseRay;

impl<'a> System<'a> for SetMouseRay {
    type SystemData = (
        Write<'a, MouseRay>,
        Read<'a, Controls>,
        Read<'a, Camera>,
        Read<'a, ScreenDimensions>
    );

    fn run(&mut self, (mut ray, controls, camera, screen_dims): Self::SystemData) {
        ray.0 = camera.ray(controls.mouse(), screen_dims.0);
    }
}

pub struct ShootStuffSystem;

impl<'a> System<'a> for ShootStuffSystem {
    type SystemData = (
        Entities<'a>,
        Write<'a, U64MarkerAllocator>,
        WriteStorage<'a, AttackTime>,
        ReadStorage<'a, AttackDelay>,
        
        WriteStorage<'a, Position>,
        WriteStorage<'a, components::Rotation>,
        WriteStorage<'a, Velocity>,
        WriteStorage<'a, Size>,
        WriteStorage<'a, Model>,
        WriteStorage<'a, TimeLeft>,
        WriteStorage<'a, Selectable>,
        WriteStorage<'a, Side>,

        WriteStorage<'a, U64Marker>
    );

    fn run(&mut self, (
        entities, mut allocator,
        mut attack_time, delay,
        mut pos, mut rot, mut vel, mut size, mut model, mut time, mut selectable, mut side,
        mut markers
    ): Self::SystemData) {

        for (entity, attack_time, delay) in (&entities, &mut attack_time, &delay).join() {
            if attack_time.0 != 0.0 {
                continue;
            }

            attack_time.0 = delay.0;

            if let Some(p) = pos.get(entity).map(|p| p.0) {
                entities.build_entity()
                    .with(Position(p), &mut pos)
                    .with(Rotation(Quaternion::zero()), &mut rot)
                    .with(Velocity(Vector3::new(0.0, 0.0, 1.0)), &mut vel)
                    .with(Size(0.1), &mut size)
                    .with(Model::Missile, &mut model)
                    .with(TimeLeft(2.0), &mut time)
                    .with(Selectable::new(false), &mut selectable)
                    .with(Side::Friendly, &mut side)
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
        WriteStorage<'a, AttackTime>,
    );

    fn run(&mut self, (secs, paused, mut time): Self::SystemData) {
        if paused.0 {
            return;
        }

        for time in (&mut time).join() {
            time.0 = move_towards(time.0, 0.0, secs.0);
        }
    }
}

fn delete_entity(entity: Entity, entities: &Entities, parents: &ReadStorage<Parent>) {
    entities.delete(entity).unwrap();

    (entities, parents).join()
        .filter(|(_, parent)| parent.0 == entity)
        .for_each(|(entity, _)| delete_entity(entity, entities, parents));

}