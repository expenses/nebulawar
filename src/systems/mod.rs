use specs::*;
use common_components::{self, *};
use context::*;
use camera::*;
use ships::*;
use cgmath::{Vector3, Quaternion, Zero};
use util::*;
use collision::*;

mod rendering;

pub use self::rendering::*;

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
        Read<'a, Drag>,
        Read<'a, ShiftPressed>,
        Read<'a, Camera>,
        ReadStorage<'a, Position>,
        WriteStorage<'a, Selectable>
    );

    fn run(&mut self, (drag, shift, camera, pos, mut selectable): Self::SystemData) {
        if let Some((left, top, right, bottom)) = drag.0 {
            for (pos, selectable) in (&pos, &mut selectable).join() {
                if let Some((x, y, _)) = self.context.screen_position(pos.0, &camera) {
                    let selected = x >= left && x <= right && y >= top && y <= bottom;
                    
                    if !shift.0 {
                        selectable.selected = selected;
                    } else if selected {
                        selectable.selected = !selectable.selected;
                    }
                } else if !shift.0 {
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
        ReadStorage<'a, ShipType>,
        ReadStorage<'a, Components>,
        ReadStorage<'a, Size>
    );

    fn run(&mut self, (entities, paused, mut pos, mut rot, mut commands, mut fuel, tag, components, size): Self::SystemData) {
        if paused.0 {
            return;
        }

        for (entity, rot, commands, tag, components) in (&entities, &mut rot, &mut commands, &tag, &components).join() {
            let finished = commands.first()
                .map(|command| handle_command(command, entity, rot, &mut pos, components, tag, &mut fuel, &size).unwrap_or(true))
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

fn transfer_fuel(fuel: &mut WriteStorage<Fuel>, ship_a: Entity, ship_b: Entity, amount: f32) -> Option<bool> {
    let can_transfer = {
        let fuel_a = fuel.get(ship_a)?;
        let fuel_b = fuel.get(ship_b)?;

        fuel_a.transfer_amount(&fuel_b, amount)
    };

    if can_transfer == 0.0 {
        Some(true)
    } else {
        fuel.get_mut(ship_a)?.reduce(can_transfer);
        fuel.get_mut(ship_b)?.increase(can_transfer);

        Some(false)
    }
} 

fn handle_command(
    command: &Command,
    entity: Entity, rot: &mut Quaternion<f32>, pos: &mut WriteStorage<Position>,
    components: &Components, tag: &ShipType,
    fuel: &mut WriteStorage<Fuel>, size: &ReadStorage<Size>
) -> Option<bool> {
    
    match command {
        Command::MoveTo(position) => {
            Some(move_ship(entity, pos, fuel, rot, tag, components, *position)? == MovementStatus::Reached)
        },
        Command::GoToAnd(target, interaction) => {
            let position = pos.get(entity)?.0;
            let target_position = pos.get(*target)?.0;

            let distance = (size.get(*target)?.0) * 2.0;

            if position.distance(&target_position) < distance {
                match interaction {
                    Interaction::Follow => Some(false),
                    Interaction::Refuel => transfer_fuel(fuel, entity, *target, 0.1),
                    Interaction::RefuelFrom => transfer_fuel(fuel, *target, entity, 0.1),
                    Interaction::Mine => Some(false)
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
        Read<'a, RightClickInteraction>,
        Read<'a, RightClick>,
        Read<'a, ShiftPressed>,
        Read<'a, Formation>,
        Read<'a, Camera>,
        WriteStorage<'a, Commands>,
        ReadStorage<'a, Selectable>,
        ReadStorage<'a, Position>
    );

    fn run(&mut self, (interaction, click, shift, formation, camera, mut commands, selectable, pos): Self::SystemData) {
        if let Some((x, y)) = click.0 {
            if let Some((entity, interaction)) = interaction.0 {
                for (selectable, commands) in (&selectable, &mut commands).join() {
                    if selectable.selected {
                        if !shift.pressed() {
                            commands.clear();
                        }

                        commands.push(Command::GoToAnd(entity, interaction));
                    }
                }
            } else {
                let ray = self.context.ray(&camera, (x, y));
                if let Some(target) = Plane::new(UP, 0.0).intersection(&ray).map(point_to_vector) {
                    if let Some(avg) = avg_position(&pos, &selectable, |s| s.selected) {
                        let len = (&selectable, &commands).join().filter(|(selectable, _)| selectable.selected).count();

                        let positions = formation.arrange(len, avg, target, 2.5);

                        (&selectable, &mut commands).join()
                            .filter(|(selectable, _)| selectable.selected)
                            .map(|(_, commands)| commands)
                            .zip(positions)
                            .for_each(|(commands, position)| {
                                if !shift.pressed() {
                                    commands.clear();
                                }

                                commands.push(Command::MoveTo(position));
                            });
                    }
                }
            }
        }
    }
}

pub fn clear_focus(world: &mut World) {
    world.exec(|mut selectable: WriteStorage<Selectable>| {
        (&mut selectable).join().for_each(|selectable| selectable.camera_following = false)
    });
}

pub fn focus_on_selected(world: &mut World) {
    world.exec(|mut selectable: WriteStorage<Selectable>| {
        (&mut selectable).join().for_each(|selectable| selectable.camera_following = selectable.selected)
    });
}

pub fn avg_position<P: Fn(&Selectable) -> bool>(pos: &ReadStorage<Position>, selectable: &ReadStorage<Selectable>, predicate: P) -> Option<Vector3<f32>> {
    let (len, sum) = (pos, selectable).join()
        .filter(|(_, selectable)| predicate(selectable))
        .map(|(pos, _)| pos.0)
        .fold((0, Vector3::zero()), |(len, sum), position| {
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
        ReadStorage<'a, Position>,
        ReadStorage<'a, Selectable>
    );

    fn run(&mut self, (mut camera, pos, selectable): Self::SystemData) {
        camera.step();

        if let Some(position) = avg_position(&pos, &selectable, |s| s.camera_following) {
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
        Read<'a, LeftClick>,
        Read<'a, ShiftPressed>,
        Read<'a, EntityUnderMouse>,
        WriteStorage<'a, Selectable>
    );

    fn run(&mut self, (click, shift, entity, mut selectable): Self::SystemData) {
        if let Some(_) = click.0 {
            if !shift.0 {
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
        Write<'a, RightClickInteraction>,
        Read<'a, EntityUnderMouse>,
        ReadStorage<'a, Fuel>,
        ReadStorage<'a, MineableMaterials>
    );

    fn run(&mut self, (mut interaction, entity, fuel, mineable): Self::SystemData) {
        if let Some((entity, _)) = entity.0 {

            let possible_interaction = match (fuel.get(entity), mineable.get(entity)) {
                (Some(fuel), _) if fuel.is_empty() => Some(Interaction::Refuel),
                (Some(_), _) => Some(Interaction::Follow),
                (_, Some(mineable)) if mineable.0 > 0 => Some(Interaction::Mine),
                _ => None
            };

            interaction.0 = possible_interaction.map(|interaction| (entity, interaction));
        } else {
            interaction.0 = None;
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
        Read<'a, Mouse>,
        Write<'a, EntityUnderMouse>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, common_components::Rotation>,
        ReadStorage<'a, Size>,
        ReadStorage<'a, Model>
    );

    fn run(&mut self, (entities, camera, mouse, mut entity, pos, rot, size, model): Self::SystemData) {
        use collision::Continuous;

        use cgmath::MetricSpace;
        let ray = self.context.ray(&camera, mouse.0);

        entity.0 = (&entities, &pos, &rot, &size, &model).join()
            .filter_map(|(entity, pos, rot, size, model)| {
                use cgmath::{self, Rotation};

                // we need to transform the ray around the mesh so we can keep the mesh constant

                let r: cgmath::Matrix4<f32> = if rot.0 == Quaternion::zero() {
                    Quaternion::zero().into()
                } else {
                    rot.0.invert().into()
                };

                let ray = ray.transform(r * cgmath::Matrix4::from_scale(1.0 / size.0) * cgmath::Matrix4::from_translation(-pos.0));

                let mesh = self.context.collision_mesh(*model);
                
                let bound: Aabb3<f32> = mesh.compute_bound();

                if !bound.intersects(&ray) {
                    return None;
                }

                self.context.collision_mesh(*model)
                    .intersection(&ray)
                    .map(point_to_vector)
                    // transform the point back
                    .map(|intersection| (rot.0 * intersection) * size.0 + pos.0)
                    .map(|intersection| (entity, intersection, camera.position().distance2(intersection)))
            })
            .min_by(|(_, _, a), (_, _, b)| cmp_floats(*a, *b))
            .map(|(entity, intersection, _)| (entity, intersection));
    }
}

// todo: mining
// todo: combat
// todo:recyc
// todo: move eveyrthing into ecs