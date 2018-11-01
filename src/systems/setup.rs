use super::*;
use glium::glutin::{WindowEvent, MouseScrollDelta, dpi::LogicalPosition, ElementState, MouseButton, VirtualKeyCode, KeyboardInput};

pub struct EventHandlerSystem;

impl<'a> System<'a> for EventHandlerSystem {
    type SystemData = (
        Write<'a, Events>,
        Write<'a, Camera>,
        Write<'a, MovementPlane>,
        Write<'a, Controls>,
        Write<'a, Paused>,
        Write<'a, Formation>,
        Write<'a, Debug>,
        WriteStorage<'a, Selectable>
    );

    fn run(&mut self, (mut events, mut camera, mut plane, mut controls, mut paused, mut formation, mut debug, mut selectable): Self::SystemData) {
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
            WindowEvent::MouseInput {state, button, ..} => {
                let pressed = state == ElementState::Pressed;

                match button {
                    MouseButton::Left => controls.handle_left(pressed),
                    MouseButton::Right => controls.handle_right(pressed),
                    MouseButton::Middle => controls.handle_middle(pressed),
                    _ => {}
                }
            },
            WindowEvent::KeyboardInput {input: KeyboardInput {state, virtual_keycode: Some(key), ..}, ..} => {
                let pressed = state == ElementState::Pressed;

                match key {
                    VirtualKeyCode::C => {
                        (&mut selectable).join()
                            .for_each(|selectable| selectable.camera_following = selectable.selected);
                    },
                    VirtualKeyCode::P if pressed => paused.switch(),
                    VirtualKeyCode::Slash if pressed => debug.0 = !debug.0,
                    VirtualKeyCode::Comma if pressed => formation.rotate_left(),
                    VirtualKeyCode::Period if pressed => formation.rotate_right(),
                    
                    VirtualKeyCode::Left   | VirtualKeyCode::A      => controls.left     = pressed,
                    VirtualKeyCode::Right  | VirtualKeyCode::D      => controls.right    = pressed,
                    VirtualKeyCode::Up     | VirtualKeyCode::W      => controls.forwards = pressed,
                    VirtualKeyCode::Down   | VirtualKeyCode::S      => controls.back     = pressed,
                    VirtualKeyCode::LShift | VirtualKeyCode::T      => controls.shift    = pressed,
                    VirtualKeyCode::Back   | VirtualKeyCode::Delete => controls.delete   = pressed,
                    VirtualKeyCode::Z => controls.save = pressed,
                    VirtualKeyCode::L => controls.load = pressed,
                    _ => {}
                }
            }
            _ => {}
        })
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

pub struct UpdateControlsSystem;

impl<'a> System<'a> for UpdateControlsSystem {
    type SystemData = Write<'a, Controls>;

    fn run(&mut self, mut controls: Self::SystemData) {
        controls.update();
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