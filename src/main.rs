extern crate glium;

extern crate obj;
extern crate genmesh;
extern crate image;
extern crate arrayvec;
extern crate cgmath;
extern crate lyon;
extern crate collision;

use glium::*;
use glutin::*;
use glutin::dpi::*;
use cgmath::*;

mod camera;
mod lines;
mod resources;
mod util;
mod context;

use resources::*;
use camera::*;
use util::*;
use std::f32::consts::*;


struct World {
    light: [f32; 3],
    skybox: usize
}

impl World {
    fn new() -> Self {
        Self {
            light: [50.0, 50.0, 50.0],
            skybox: 0
        }
    }
}

enum ShipType {
    Fighter,
    Tanker
}

impl ShipType {
    fn model(&self) -> usize {
        match *self {
            ShipType::Fighter => 0,
            ShipType::Tanker => 1
        }
    }
}

struct Ship {
    tag: ShipType,
    position: Vector3<f32>,
    angle: Euler<Rad<f32>>
}

impl Ship {
    fn new(tag: ShipType, position: Vector3<f32>, angle: (f32, f32, f32)) -> Self {
        let (roll, yaw, pitch) = angle;
        Self {
            tag, position,
            angle: Euler::new(Rad(roll), Rad(yaw), Rad(pitch))
        }
    }

    fn position_matrix(&self) -> Matrix4<f32> {
        let angle: Matrix4<f32> = self.angle.into();
        Matrix4::from_translation(self.position) * angle
    }

    fn yaw(&self) -> f32 {
        self.angle.y.0
    }

    fn move_forwards(&mut self, amount: f32) {
        self.position.x -= self.yaw().sin() * amount;
        self.position.z -= self.yaw().cos() * amount;
    }

    fn change_yaw(&mut self, amount: f32) {
        self.angle.y = Rad(self.yaw() + amount)
    }
}


struct Game {
    context: context::Context,
    ships: Vec<Ship>,
    world: World,
    camera: Camera
}

fn main() {
    let mut events_loop = glutin::EventsLoop::new();
    
    let mut renderer = context::Context::new(&events_loop);

    let mut camera = Camera::default();

    let world = World::new();

    let steps_needed = PI / 0.01;
    let distance_traveled = 0.1 * steps_needed;

    let radius = distance_traveled / PI;

    let mut ship = Ship::new(ShipType::Fighter, Vector3::new(radius, 0.0, 0.0), (0.0, 0.0, 0.0));

    let s2 = Ship::new(ShipType::Tanker, Vector3::new(0.0, 0.0, 0.0), (0.0, 0.0, 0.0));

    let mut time: f32 = 0.0;

    let world_plane = collision::Plane::new(Vector3::new(0.0, 1.0, 0.0), 0.0);


    let mut closed = false;
    while !closed {
        events_loop.poll_events(|event| if let glutin::Event::WindowEvent {event, ..} = event {
            match event {
                glutin::WindowEvent::CloseRequested => closed = true,
                glutin::WindowEvent::CursorMoved {position: LogicalPosition {x, y}, ..} => {
                    let (mouse_x, mouse_y) = renderer.controls.mouse;
                    let (delta_x, delta_y) = (x - mouse_x, y - mouse_y);

                    if renderer.controls.right_dragging() {
                        camera.rotate_longitude(delta_x as f32 / 200.0);
                        camera.rotate_latitude(delta_y as f32 / 200.0);
                    } else if renderer.controls.left_pressed && renderer.controls.left_drag.is_none() {
                        renderer.controls.left_drag = Some((x, y));
                    }

                    renderer.controls.mouse = (x, y);
                },
                glutin::WindowEvent::MouseInput {state, button, ..} => {
                    let pressed = state == ElementState::Pressed;

                    match button {
                        MouseButton::Left => {
                            renderer.controls.left_pressed = pressed;
                            if let None = renderer.controls.left_drag.take() {
                                let ray = renderer.ray(&camera);

                                use collision::Continuous;
                                if let Some(point) = world_plane.intersection(&ray) {
                                    ship.position = point_to_vector(point);
                                }
                            }
                        }
                        MouseButton::Right => renderer.controls.right_pressed = pressed,
                        _ => {}
                    }
                },
                glutin::WindowEvent::KeyboardInput {input: KeyboardInput {state, virtual_keycode: Some(key), ..}, ..} => {
                    let pressed = state == ElementState::Pressed;
                    
                    match key {
                        VirtualKeyCode::Left => renderer.controls.left = pressed,
                        VirtualKeyCode::Right => renderer.controls.right = pressed,
                        VirtualKeyCode::Up => renderer.controls.forwards = pressed,
                        VirtualKeyCode::Down => renderer.controls.back = pressed,
                        _ => {}
                    }
                },
                glutin::WindowEvent::MouseWheel {delta, ..} => match delta {
                    MouseScrollDelta::PixelDelta(LogicalPosition {y, ..}) => camera.change_distance(y as f32 / 20.0),
                    MouseScrollDelta::LineDelta(_, y) => camera.change_distance(-y * 2.0)
                },
                _ => ()
            }
        });

        

        //ship.change_yaw(0.01);
        //ship.move_forwards(0.1);
        //ship.position.x = time.sin() * 10.0;
        //ship.position.z = time.cos() * 10.0;

        if renderer.controls.left {
            camera.move_sideways(-0.2);
        }

        if renderer.controls.right {
            camera.move_sideways(0.2);
        }

        if renderer.controls.forwards {
            camera.move_forwards(0.2);
        }

        if renderer.controls.back {
            camera.move_forwards(-0.2);
        }

        renderer.clear();

        renderer.render(ship.tag.model(), ship.position_matrix(), &camera, world.light);

        renderer.render(s2.tag.model(), s2.position_matrix(), &camera, world.light);

        renderer.render_skybox(&camera, world.skybox);

        if let Some((x, y)) = renderer.screen_position(ship.position_matrix(), &camera) {
            renderer.render_circle(x, y, 50.0);
        }

        if let Some((start_x, start_y)) = renderer.controls.left_drag {
            let (end_x, end_y) = renderer.controls.mouse;
            let (start_x, start_y) = (start_x as f32, start_y as f32);
            let (end_x, end_y) = (end_x as f32, end_y as f32);

            renderer.render_line((start_x, start_y), (end_x, start_y));
            renderer.render_line((start_x, end_y), (end_x, end_y));

            renderer.render_line((start_x, start_y), (start_x, end_y));
            renderer.render_line((end_x, start_y), (end_x, end_y));
        }
        renderer.finish();

        time += 0.02;
    }
}