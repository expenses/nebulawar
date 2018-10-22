use specs::*;
use common_components::{self, *};
use context::*;
use camera::*;
use circle_size;
use ships::*;
use cgmath::{Vector3, Quaternion, Zero};
use util::*;
use collision::*;

mod rendering;

pub use self::rendering::*;

pub struct SpinSystem;

impl<'a> System<'a> for SpinSystem {
    type SystemData = (
        Read<'a, Secs>,
        Read<'a, Paused>,
        WriteStorage<'a, ObjectSpin>
    );

    fn run(&mut self, (secs, paused, mut spins): Self::SystemData) {
        if paused.0 {
            return;
        }

        for spin in (&mut spins).join() {
            spin.turn(secs.0);
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
        WriteStorage<'a, ShipStorage>,
        ReadStorage<'a, ShipType>,
        ReadStorage<'a, Components>,
        ReadStorage<'a, Size>
    );

    fn run(&mut self, (entities, paused, mut pos, mut rot, mut commands, mut storages, tag, components, size): Self::SystemData) {
        if paused.0 {
            return;
        }

        for (entity, rot, commands, tag, components) in (&entities, &mut rot, &mut commands, &tag, &components).join() {
            let mut next = false;

            if let Some(command) = commands.0.first() {
                handle_command(command, entity, &mut rot.0, &mut pos, components, tag, &mut storages, &size);
            }

            if next {
                println!("{:?}", commands.0);
                commands.0.remove(0);
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

fn move_ship(position: &mut Vector3<f32>, rotation: &mut Quaternion<f32>, storage: &mut ShipStorage, tag: &ShipType, components: &Components, point: Vector3<f32>) -> MovementStatus {
    if storage.fuel.is_empty() {
        return MovementStatus::OutOfFuel;
    }
    
    storage.fuel.reduce(0.01);

    let speed = components.thrust() / tag.mass();
    *position = move_towards(*position, point, speed);

    if *position == point {
        MovementStatus::Reached
    } else {
        *rotation = look_at(point - *position);
        MovementStatus::Moving
    }
}

fn transfer_fuel(storage: &mut WriteStorage<ShipStorage>, ship_a: Entity, ship_b: Entity, amount: f32) -> Option<bool> {
    let can_transfer = {
        let storage_a = storage.get(ship_a)?;
        let storage_b = storage.get(ship_b)?;

        storage_a.fuel.transfer_amount(&storage_b.fuel, amount)
    };

    if can_transfer == 0.0 {
        Some(true)
    } else {
        storage.get_mut(ship_a)?.fuel.reduce(can_transfer);
        storage.get_mut(ship_b)?.fuel.increase(can_transfer);

        Some(false)
    }
} 

fn handle_command(command: &Command, entity: Entity, rot: &mut Quaternion<f32>, pos: &mut WriteStorage<Position>, components: &Components, tag: &ShipType, storages: &mut WriteStorage<ShipStorage>, size: &ReadStorage<Size>) -> bool {
    match command {
        Command::MoveTo(position) => {
            let pos = pos.get_mut(entity).unwrap();
            let storage = storages.get_mut(entity).unwrap();

            move_ship(&mut pos.0, rot, storage, tag, components, *position) == MovementStatus::Reached
        },
        Command::GoToAnd(ship, interaction) => {
            let position = pos.get(entity).unwrap().0;
            let target_position = pos.get(*ship).unwrap().0;

            let distance = (size.get(*ship).unwrap().0) * 2.0;

            if position.distance(&target_position) < distance {
                match interaction {
                    Interaction::Follow => false,
                    Interaction::Refuel => transfer_fuel(storages, entity, *ship, 0.1).unwrap_or(true),
                    Interaction::RefuelFrom => transfer_fuel(storages, *ship, entity, 0.1).unwrap_or(true),
                    Interaction::Mine => false
                }
            } else {
                let pos = pos.get_mut(entity).unwrap();
                let storage = storages.get_mut(entity).unwrap();

                move_ship(&mut pos.0, rot, storage, tag, components, target_position) == MovementStatus::OutOfFuel
            }
        }
    }
}

pub struct RightClickSystem<'a> {
    pub context: &'a Context,
}

impl<'a> System<'a> for RightClickSystem<'a> {
    type SystemData = (
        Entities<'a>,
        Read<'a, RightClick>,
        Read<'a, ShiftPressed>,
        Read<'a, Formation>,
        Read<'a, Camera>,
        WriteStorage<'a, Commands>,
        ReadStorage<'a, Selectable>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, ShipStorage>,
        ReadStorage<'a, MineableMaterials>,
        ReadStorage<'a, Size>
    );

    fn run(&mut self, (entities, click, shift, formation, camera, mut commands, selectable, pos, storage, mineable, sizes): Self::SystemData) {
        if let Some((x, y)) = click.0 {
            if let Some(entity) = entity_at(x, y, &entities, &pos, &sizes, &self.context, &camera) {
                if let Some(interaction) = right_click_interaction(entity, &storage, &mineable) {
                    for (selectable, commands) in (&selectable, &mut commands).join() {
                        if selectable.selected {
                            if !shift.0 {
                                commands.0.clear();
                            }

                            commands.0.push(Command::GoToAnd(entity, interaction));
                        }
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
                                if !shift.0 {
                                    commands.0.clear();
                                }

                                commands.0.push(Command::MoveTo(position));
                            });
                    }
                }
            }
        }
    }
}

fn entity_at(mouse_x: f32, mouse_y: f32, entities: &Entities, positions: &ReadStorage<Position>, sizes: &ReadStorage<Size>, context: &Context, camera: &Camera) -> Option<Entity> {
    entities.join()
        .filter_map(|entity| positions.get(entity).map(|position| (entity, position)))
        .filter_map(|(entity, position)| {
            context.screen_position(position.0, camera)
                .filter(|(x, y, z)| {
                    (mouse_x - x).hypot(mouse_y - y) < circle_size(*z) * sizes.get(entity).map(|size| size.0).unwrap_or(1.0)
                })
                .map(|(_, _, z)| (entity, z))
        })
        .min_by(|(_, z_a), (_, z_b)| z_a.partial_cmp(z_b).unwrap_or(::std::cmp::Ordering::Less))
        .map(|(entity, _)| entity)
}

fn right_click_interaction(entity: Entity, storage: &ReadStorage<ShipStorage>, mineable: &ReadStorage<MineableMaterials>) -> Option<Interaction> {
    match (storage.get(entity), mineable.get(entity)) {
        (Some(storage), _) if storage.fuel.is_empty() => Some(Interaction::Refuel),
        (Some(_), _) => Some(Interaction::Follow),
        (_, Some(mineable)) if mineable.0 > 0 => Some(Interaction::Mine),
        _ => None
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
        Entities<'a>,
        Read<'a, LeftClick>,
        Read<'a, ShiftPressed>,
        Read<'a, Camera>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, Size>,
        WriteStorage<'a, Selectable>
    );

    fn run(&mut self, (entities, click, shift, camera, position, size, mut selectable): Self::SystemData) {
        if let Some((x, y)) = click.0 {
            if !shift.0 {
                (&mut selectable).join().for_each(|selectable| selectable.selected = false);
            }

            if let Some(entity) = entity_at(x, y, &entities, &position, &size, &self.context, &camera) {
                if let Some(selectable) = selectable.get_mut(entity) {
                    selectable.selected = !selectable.selected;
                }
            }
        }
    }
}