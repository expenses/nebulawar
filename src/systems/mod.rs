use specs::*;
use crate::components::{self, *};
use crate::context::*;
use crate::camera::*;
use crate::ships::*;
use cgmath::{Vector3, MetricSpace, Zero, Quaternion};
use crate::util::*;
use crate::controls::Controls;
use crate::resources::*;
use crate::star_system::*;
use ncollide3d::query::RayCast;
use ncollide3d::shape::Plane;
use nalgebra::Unit;

mod rendering;
mod storage;
mod steering;
mod saving;
mod combat;
mod setup;

pub use self::rendering::*;
pub use self::steering::*;
pub use self::saving::*;
pub use self::combat::*;
pub use self::setup::*;
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
                if let Some(pos) = camera.screen_position(pos.0, screen_dims.0.into(), false) {
                    let selected = pos.x >= left && pos.x <= right && pos.y >= top && pos.y <= bottom;
                    
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
        WriteStorage<'a, AttackTarget>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, Size>,
        ReadStorage<'a, DrillSpeed>,
        ReadStorage<'a, CanAttack>
    );

    fn run(&mut self, (entities, paused, mut commands, mut materials, mut mineable, mut seek, mut attack_target, pos, size, drill_speed, attack): Self::SystemData) {
        if paused.0 {
            return;
        }

        for (entity, commands) in (&entities, &mut commands).join() {
            let last = commands.len() == 1;

            let finished = commands.first()
                .map(|command| handle_command(command, entity, &mut materials, &mut mineable, &size, &drill_speed, &pos, &mut seek, &mut attack_target, last, &attack).unwrap_or(true))
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
    seek: &mut WriteStorage<SeekPosition>, attack_target: &mut WriteStorage<AttackTarget>, last: bool, attack: &ReadStorage<CanAttack>
) -> Option<bool> {
    
    let entity_position = pos.get(entity)?.0;

    attack_target.remove(entity);

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

            let distance = if *interaction == Interaction::Attack {
                attack_target.insert(entity, AttackTarget {entity: *target, kamikaze: false}).unwrap();
                
                attack.get(entity)?.range - CLOSE_ENOUGH_DISTANCE * 2.0
            } else {
                size.get(*target)?.0 + size.get(entity)?.0
            };

            if entity_position.distance(target_position) - CLOSE_ENOUGH_DISTANCE < distance {
                match interaction {
                    Interaction::Follow => Some(false),
                    Interaction::Mine => {
                        transfer_between_different(mineable_materials, materials, *target, entity, drill_speed.get(entity).unwrap().0)
                    },
                    Interaction::Attack => Some(false),
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

        if let Some(position) = avg(iterator) {
            camera.move_towards(position);
        }
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
            order.to_move = ordering.collect();

            order.command = if !order.to_move.is_empty() {
                let iso = make_iso(Vector3::new(0.0, plane.0, 0.0), Quaternion::zero());

                Plane::new(Unit::new_normalize(vector_to_na_vector(UP)))
                    .toi_with_ray(&iso, &ray.0, 1_000_000.0, true)
                    .map(|toi| ray.0.origin + ray.0.dir * toi)
                    .map(|point| Command::MoveTo(Vector3::new(point.x, point.y, point.z)))
            } else {
                None
            };
        }
    }
}

// todo:recyc

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

fn delete_entity(entity: Entity, entities: &Entities, parents: &ReadStorage<Parent>) {
    entities.delete(entity).unwrap();

    (entities, parents).join()
        .filter(|(_, parent)| parent.0 == entity)
        .for_each(|(entity, _)| delete_entity(entity, entities, parents));

}
