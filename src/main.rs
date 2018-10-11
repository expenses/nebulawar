extern crate glium;

extern crate obj;
extern crate genmesh;
extern crate image;
extern crate arrayvec;
extern crate cgmath;
extern crate lyon;
extern crate collision;
extern crate runic;
#[macro_use]
extern crate derive_is_enum_variant;

use glium::*;
use glutin::*;
use glutin::dpi::*;
use cgmath::*;
use collision::*;
use std::collections::*;

mod camera;
mod lines;
mod resources;
mod util;
mod context;
mod ships;
mod controls;

use controls::*;

use resources::*;
use camera::*;
use util::*;
use ships::*;

pub struct World {
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

struct Game {
    context: context::Context,
    ships: Ships,
    world: World,
    camera: Camera,
    controls: Controls,
    plane_y: f32,
    selected: HashSet<usize>
}

impl Game {
    fn new(events_loop: &EventsLoop) -> Self {
        Self {
            context: context::Context::new(events_loop),
            ships: Ships::default(),
            world: World::new(),
            camera: Camera::default(),
            controls: Controls::new(),
            plane_y: 0.0,
            selected: HashSet::new()
        }
    }

    fn handle_mouse_movement(&mut self, x: f32, y: f32) {
        let (mouse_x, mouse_y) = self.controls.mouse();
        let (delta_x, delta_y) = (x - mouse_x, y - mouse_y);
        self.controls.set_mouse(x, y);

        if self.controls.right_dragging() {
            self.camera.rotate_longitude(delta_x / 200.0);
            self.camera.rotate_latitude(delta_y / 200.0);
        }
    }

    fn point_under_mouse(&self) -> Option<Vector3<f32>> {
        let ray = self.context.ray(&self.camera, self.controls.mouse());

        Plane::new(Vector3::new(0.0, 1.0, 0.0), self.plane_y).intersection(&ray).map(point_to_vector)
    }

    fn handle_mouse_button(&mut self, button: MouseButton, pressed: bool) {
        match button {
            MouseButton::Left => self.controls.handle_left(pressed),
            MouseButton::Right => self.controls.handle_right(pressed),
            MouseButton::Middle => self.controls.handle_middle(pressed),
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
                self.ships.push(ShipType::Fighter, point, (0.0, 0.0, 0.0));
                self.plane_y += 0.5;
            },
            _ => {}
        }
    }

    fn update(&mut self) {
        if self.controls.middle_clicked() {
            self.camera.set_focus(&self.selected);
        }

        if let Some((mut left, mut top)) = self.controls.left_drag() {
            let (mut right, mut bottom) = self.controls.mouse();
            
            if right < left {
                std::mem::swap(&mut right, &mut left);
            }

            if bottom < top {
                std::mem::swap(&mut top, &mut bottom);
            }

            self.selected.clear();

            for ship in self.ships.iter() {
                if let Some((x, y)) = self.context.screen_position(ship.position_matrix(), &self.camera) {
                    if left <= x && x <= right && top <= y && y <= bottom {
                        self.selected.insert(ship.id());
                    }
                }
            }
        }

        if self.controls.left_clicked() {
            if let Some(point) = self.point_under_mouse() {
                for id in &self.selected {
                    self.ships.get_mut(*id).unwrap().commands = vec![Command::MoveTo(point)];
                }
            }
        }

        self.controls.update();

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

        for ship in self.ships.iter_mut() {
            ship.step();
        }

        self.camera.step(&self.ships);
    }

    fn render(&mut self) {
        self.context.clear();

        for ship in self.ships.iter() {
            ship.render(&mut self.context, &self.camera, &self.world);
        }

        self.context.render_skybox(&self.camera, self.world.skybox);

        for ship in self.ships.iter() {
            if self.selected.contains(&ship.id()) {
                 if let Some((x, y)) = self.context.screen_position(ship.position_matrix(), &self.camera) {
                    self.context.render_circle(x, y, 50.0);
                }
            }
        }

        if let Some((start_x, start_y)) = self.controls.left_drag() {
            let (end_x, end_y) = self.controls.mouse();
            let (start_x, start_y) = (start_x as f32, start_y as f32);
            let (end_x, end_y) = (end_x as f32, end_y as f32);

            self.context.render_line((start_x, start_y), (end_x, start_y));
            self.context.render_line((start_x, end_y), (end_x, end_y));

            self.context.render_line((start_x, start_y), (start_x, end_y));
            self.context.render_line((end_x, start_y), (end_x, end_y));
        }

        self.context.render_text(&format!("Ship count: {}", self.ships.len()), 10.0, 10.0);

        self.context.finish();
    }
}

fn main() {
    let mut events_loop = EventsLoop::new();
    
    let mut game = Game::new(&events_loop);

    game.ships.push(ShipType::Fighter, Vector3::new(5.0, 0.0, 0.0), (0.0, 0.0, 0.0));
    game.ships.push(ShipType::Tanker, Vector3::new(0.0, 0.0, 0.0), (0.0, 0.0, 0.0));

    game.ships[0].commands.push(Command::MoveTo(Vector3::new(-100.0, -50.0, 10.0)));
    game.selected.insert(0);
    game.selected.insert(1);

    let mut closed = false;
    while !closed {
        events_loop.poll_events(|event| if let glutin::Event::WindowEvent {event, ..} = event {
            match event {
                glutin::WindowEvent::CloseRequested => closed = true,
                glutin::WindowEvent::CursorMoved {position: LogicalPosition {x, y}, ..} => game.handle_mouse_movement(x as f32, y as f32),
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