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
use collision::*;

mod camera;
mod lines;
mod resources;
mod util;
mod context;

use resources::*;
use camera::*;
use util::*;
use std::f32::consts::*;

#[derive(Default)]
pub struct Controls {
    pub mouse: (f64, f64),
    pub left_pressed: bool,
    pub left_drag: Option<(f64, f64)>,
    pub right_pressed: bool,
    pub left: bool,
    pub right: bool,
    pub forwards: bool,
    pub back: bool
}

impl Controls {
    pub fn right_dragging(&self) -> bool {
        self.mouse != (0.0, 0.0) && self.right_pressed
    }
}

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
    camera: Camera,
    controls: Controls
}

impl Game {
    fn new(events_loop: &EventsLoop) -> Self {
        Self {
            context: context::Context::new(events_loop),
            ships: Vec::new(),
            world: World::new(),
            camera: Camera::default(),
            controls: Controls::default()
        }
    }

    fn handle_mouse_movement(&mut self, x: f64, y: f64) {
        let (mouse_x, mouse_y) = self.controls.mouse;
        let (delta_x, delta_y) = (x - mouse_x, y - mouse_y);

        if self.controls.right_dragging() {
            self.camera.rotate_longitude(delta_x as f32 / 200.0);
            self.camera.rotate_latitude(delta_y as f32 / 200.0);
        } else if self.controls.left_pressed && self.controls.left_drag.is_none() {
            self.controls.left_drag = Some((x, y));
        }

        self.controls.mouse = (x, y);
    }

    fn point_under_mouse(&self) -> Option<Vector3<f32>> {
        let (x, y) = self.controls.mouse;
        let ray = self.context.ray(&self.camera, (x as f32, y as f32));

        Plane::new(Vector3::new(0.0, 1.0, 0.0), 0.0).intersection(&ray).map(point_to_vector)
    }

    fn handle_mouse_button(&mut self, button: MouseButton, pressed: bool) {
        match button {
            MouseButton::Left => {
                self.controls.left_pressed = pressed;
                if let None = self.controls.left_drag.take() {
                    if let Some(point) = self.point_under_mouse() {
                        self.ships[0].position = point;
                    }
                }
            }
            MouseButton::Right => self.controls.right_pressed = pressed,
            MouseButton::Middle if pressed => self.camera.set_position(self.ships[0].position),
            _ => {}
        }
    }

    fn handle_keypress(&mut self, key: VirtualKeyCode, pressed: bool) {
        match key {
            VirtualKeyCode::Left => self.controls.left = pressed,
            VirtualKeyCode::Right => self.controls.right = pressed,
            VirtualKeyCode::Up => self.controls.forwards = pressed,
            VirtualKeyCode::Down => self.controls.back = pressed,
            VirtualKeyCode::T if pressed => if let Some(point) = self.point_under_mouse() {
                self.ships.push(Ship::new(ShipType::Fighter, point, (0.0, 0.0, 0.0)));

            },
            _ => {}
        }
    }

    fn update(&mut self) {
        if self.controls.left {
            self.camera.move_sideways(-0.2);
        }

        if self.controls.right {
            self.camera.move_sideways(0.2);
        }

        if self.controls.forwards {
            self.camera.move_forwards(0.2);
        }

        if self.controls.back {
            self.camera.move_forwards(-0.2);
        }
    }

    fn render(&mut self) {
        self.context.clear();

        for ship in &self.ships {
            self.context.render(ship.tag.model(), ship.position_matrix(), &self.camera, self.world.light);
        }

        self.context.render_skybox(&self.camera, self.world.skybox);

        if let Some((x, y)) = self.context.screen_position(self.ships[0].position_matrix(), &self.camera) {
            self.context.render_circle(x, y, 50.0);
        }

        if let Some((start_x, start_y)) = self.controls.left_drag {
            let (end_x, end_y) = self.controls.mouse;
            let (start_x, start_y) = (start_x as f32, start_y as f32);
            let (end_x, end_y) = (end_x as f32, end_y as f32);

            self.context.render_line((start_x, start_y), (end_x, start_y));
            self.context.render_line((start_x, end_y), (end_x, end_y));

            self.context.render_line((start_x, start_y), (start_x, end_y));
            self.context.render_line((end_x, start_y), (end_x, end_y));
        }

        self.context.finish();
    }
}

fn main() {
    let mut events_loop = EventsLoop::new();
    
    let mut game = Game::new(&events_loop);

    game.ships.push(Ship::new(ShipType::Fighter, Vector3::new(5.0, 0.0, 0.0), (0.0, 0.0, 0.0)));
    game.ships.push(Ship::new(ShipType::Tanker, Vector3::new(0.0, 0.0, 0.0), (0.0, 0.0, 0.0)));

    let mut closed = false;
    while !closed {
        events_loop.poll_events(|event| if let glutin::Event::WindowEvent {event, ..} = event {
            match event {
                glutin::WindowEvent::CloseRequested => closed = true,
                glutin::WindowEvent::CursorMoved {position: LogicalPosition {x, y}, ..} => game.handle_mouse_movement(x, y),
                glutin::WindowEvent::MouseInput {state, button, ..} => {
                    game.handle_mouse_button(button, state == ElementState::Pressed);
                },
                glutin::WindowEvent::KeyboardInput {input: KeyboardInput {state, virtual_keycode: Some(key), ..}, ..} => {
                    game.handle_keypress(key, state == ElementState::Pressed);
                },
                glutin::WindowEvent::MouseWheel {delta, ..} => match delta {
                    MouseScrollDelta::PixelDelta(LogicalPosition {y, ..}) => game.camera.change_distance(y as f32 / 20.0),
                    MouseScrollDelta::LineDelta(_, y) => game.camera.change_distance(-y * 2.0)
                },
                _ => ()
            }
        });

        game.update();
        game.render();
    }
}