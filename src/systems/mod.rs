use specs::*;
use common_components::{self, *};
use context::*;
use camera::*;
use ships::*;
use cgmath::{Vector3, Quaternion, Zero, Matrix4, MetricSpace};
use util::*;
use collision::*;
use controls::Controls;
use glium::glutin::{WindowEvent, MouseScrollDelta, dpi::LogicalPosition};

mod rendering;
mod storage;

pub use self::rendering::*;
use self::storage::*;

pub struct SpinSystem;

impl<'a> System<'a> for SpinSystem {
    type SystemData = (
        Entities<'a>,
        Read<'a, Secs>,
        Read<'a, Paused>,
        WriteStorage<'a, ObjectSpin>,
        WriteStorage<'a, common_components::Rotation>
    );

    fn run(&mut self, (entities, secs, paused, mut spins, mut rotations): Self::SystemData) {
        if paused.0 {
            return;
        }

        for (entity, spin) in (&entities, &mut spins).join() {
            spin.turn(secs.0);

            rotations.insert(
                entity,
                common_components::Rotation(spin.to_quat())
            ).unwrap();
        }
    }
}

pub struct DragSelectSystem<'a> {
    pub context: &'a Context,
}

impl<'a> System<'a> for DragSelectSystem<'a> {
    type SystemData = (
        Read<'a, Controls>,
        Read<'a, Camera>,
        ReadStorage<'a, Position>,
        WriteStorage<'a, Selectable>
    );

    fn run(&mut self, (controls, camera, pos, mut selectable): Self::SystemData) {
        if let Some((left, top, right, bottom)) = controls.left_drag_rect() {
            for (pos, selectable) in (&pos, &mut selectable).join() {
                if let Some((x, y, _)) = self.context.screen_position(pos.0, &camera) {
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
        WriteStorage<'a, Position>,
        WriteStorage<'a, common_components::Rotation>,
        WriteStorage<'a, Commands>,
        WriteStorage<'a, Fuel>,
        WriteStorage<'a, Materials>,
        WriteStorage<'a, MineableMaterials>,
        ReadStorage<'a, ShipType>,
        ReadStorage<'a, Components>,
        ReadStorage<'a, Size>,
        ReadStorage<'a, DrillSpeed>
    );

    fn run(&mut self, (entities, paused, mut pos, mut rot, mut commands, mut fuel, mut materials, mut mineable, tag, components, size, drill_speed): Self::SystemData) {
        if paused.0 {
            return;
        }

        for (entity, rot, commands, tag, components) in (&entities, &mut rot, &mut commands, &tag, &components).join() {
            let finished = commands.first()
                .map(|command| handle_command(command, entity, rot, &mut pos, components, tag, &mut fuel, &mut materials, &mut mineable, &size, &drill_speed).unwrap_or(true))
                .unwrap_or(false);
            
            if finished {
                commands.remove(0);
            }
        }
    }
}

#[derive(PartialEq)]
enum MovementStatus {
    Moving, 
    Reached,
    OutOfFuel
}

fn move_ship(entity: Entity, pos: &mut WriteStorage<Position>, fuel: &mut WriteStorage<Fuel>, rotation: &mut Quaternion<f32>, tag: &ShipType, components: &Components, point: Vector3<f32>) -> Option<MovementStatus> {
    let pos = &mut pos.get_mut(entity)?.0;
    let fuel = fuel.get_mut(entity)?;

    if fuel.is_empty() {
        return Some(MovementStatus::OutOfFuel);
    }
    
    fuel.reduce(0.01);

    let speed = components.thrust() / tag.mass();
    *pos = move_towards(*pos, point, speed);

    if *pos == point {
        Some(MovementStatus::Reached)
    } else {
        *rotation = look_at(point - *pos);
        Some(MovementStatus::Moving)
    }
}

fn handle_command(
    command: &Command,
    entity: Entity, rot: &mut Quaternion<f32>, pos: &mut WriteStorage<Position>,
    components: &Components, tag: &ShipType,
    fuel: &mut WriteStorage<Fuel>, materials: &mut WriteStorage<Materials>, mineable_materials: &mut WriteStorage<MineableMaterials>,
    size: &ReadStorage<Size>, drill_speed: &ReadStorage<DrillSpeed>
) -> Option<bool> {
    
    match command {
        Command::MoveTo(position) => {
            Some(move_ship(entity, pos, fuel, rot, tag, components, *position)? == MovementStatus::Reached)
        },
        Command::GoToAnd(target, interaction) => {
            let position = pos.get(entity)?.0;
            let target_position = pos.get(*target)?.0;

            let distance = size.get(*target)?.0 + size.get(entity)?.0;

            if position.distance(target_position) < distance {
                match interaction {
                    Interaction::Follow => Some(false),
                    Interaction::Refuel => transfer_from_storages(fuel, entity, *target, 0.1),
                    Interaction::RefuelFrom => transfer_from_storages(fuel, *target, entity, 0.1),
                    Interaction::Mine => {
                        transfer_between_different(mineable_materials, materials, *target, entity, drill_speed.get(entity).unwrap().0)
                    },
                    Interaction::Attack => Some(true),
                }
            } else {
                Some(move_ship(entity, pos, fuel, rot, tag, components, target_position)? == MovementStatus::OutOfFuel)
            }
        }
    }
}

pub struct RightClickSystem<'a> {
    pub context: &'a Context,
}

impl<'a> System<'a> for RightClickSystem<'a> {
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
                    &Command::GoToAnd(entity, interaction) => {
                        order.to_move.iter()
                            .for_each(|e| commands.get_mut(*e).unwrap().order(controls.shift, Command::GoToAnd(entity, interaction)));
                    },
                    &Command::MoveTo(target) => {
                        if let Some(avg) = avg_pos.0 {
                            let positions = formation.arrange(order.to_move.len(), avg, target, 2.5);

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

pub struct LeftClickSystem<'a> {
    pub context: &'a Context
}

impl<'a> System<'a> for LeftClickSystem<'a> {
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

pub struct RightClickInteractionSystem<'a> {
    pub context: &'a Context
}

impl<'a> System<'a> for RightClickInteractionSystem<'a> {
    type SystemData = (
        Entities<'a>,
        Write<'a, RightClickOrder>,
        Read<'a, EntityUnderMouse>,
        Read<'a, Camera>,
        Read<'a, Controls>,
        Read<'a, MovementPlane>,
        ReadStorage<'a, Fuel>,
        ReadStorage<'a, MineableMaterials>,
        ReadStorage<'a, Side>,
        ReadStorage<'a, Selectable>,
        ReadStorage<'a, DrillSpeed>
    );

    fn run(&mut self, (entities, mut order, entity, camera, controls, plane, fuel, mineable, side, selectable, drill): Self::SystemData) {
        let ordering = (&entities, &selectable, &side).join()
            .filter(|(_, selectable, side)| selectable.selected && **side == Side::Friendly)
            .map(|(entity, _, _)| entity);

        if let Some((entity, _)) = entity.0 {
            let interaction = if side.get(entity) == Some(&Side::Enemy) {
                order.to_move = ordering.collect();

                Interaction::Attack
            } else if fuel.get(entity).filter(|fuel| fuel.is_empty()).is_some() {
                order.to_move = ordering.collect();

                Interaction::Refuel
            } else if mineable.get(entity).filter(|mineable| !mineable.is_empty()).is_some() {
                order.to_move = ordering.filter(|entity| drill.get(*entity).is_some()).collect();

                Interaction::Mine
            } else {
                Interaction::Follow
            };

            order.command = Some(Command::GoToAnd(entity, interaction));
        } else {
            let ray = self.context.ray(&camera, controls.mouse());

            order.command = Plane::new(UP, -plane.0).intersection(&ray)
                .map(|point| Command::MoveTo(point_to_vector(point)));

            order.to_move = ordering.collect();
        }
    }
}

pub struct EntityUnderMouseSystem<'a> {
    pub context: &'a Context
}

impl<'a> System<'a> for EntityUnderMouseSystem<'a> {
    type SystemData = (
        Entities<'a>,
        Read<'a, Camera>,
        Read<'a, Controls>,
        Write<'a, EntityUnderMouse>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, common_components::Rotation>,
        ReadStorage<'a, Size>,
        ReadStorage<'a, Model>
    );

    fn run(&mut self, (entities, camera, controls, mut entity, pos, rot, size, model): Self::SystemData) {
        let ray = self.context.ray(&camera, controls.mouse());

        entity.0 = (&entities, &pos, &rot, &size, &model).join()
            .filter_map(|(entity, pos, rot, size, model)| {
                let rotation: Matrix4<f32> = rot.0.into();

                let transform = Matrix4::from_translation(pos.0) * rotation * Matrix4::from_scale(size.0);

                let mesh = self.context.collision_mesh(*model);
                
                let bound: Aabb3<f32> = mesh.compute_bound();

                if !bound.intersects_transformed(&ray, &transform) {
                    return None;
                }

                mesh.intersection_transformed(&ray, &transform)
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
        Write<'a, Log>
    );

    fn run(&mut self, (secs, mut log): Self::SystemData) {
        log.step(secs.0);
    }
}