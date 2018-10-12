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
extern crate rand;

use rand::rngs::*;
use rand::Rng;
use rand::distributions::*;

use glium::*;
use glutin::*;
use glutin::dpi::*;
use cgmath::*;
use collision::*;
use std::collections::*;
use std::f32::consts::*;

mod camera;
mod util;
mod context;
mod ships;
mod controls;
mod people;

use people::*;
use controls::*;

use camera::*;
use util::*;
use ships::*;

fn average_position(selection: &HashSet<ShipID>, ships: &AutoIDMap<ShipID, Ship>) -> Option<Vector3<f32>> {
    if !selection.is_empty() {
        let position = selection.iter().fold(Vector3::zero(), |vector, index| {
            vector + ships[*index].position
        }) / selection.len() as f32;

        Some(position)
    } else {
        None
    }
}

// http://corysimon.github.io/articles/uniformdistn-on-sphere/
fn uniform_sphere_distribution(rng: &mut ThreadRng) -> Vector3<f32> {
    use std::f64::consts::PI;

    let uniform = Normal::new(0.0, 1.0);

    let x = uniform.ind_sample(rng);
    let y = uniform.ind_sample(rng);

    let theta = 2.0 * PI * x;

    // Ensure that the phi value is between -1 and 1 but is still random

    let mut value = (1.0 - 2.0 * y);

    while value > 1.0 {
        value -= 2.0;
    }

    while value < -1.0 {
        value += 2.0;
    }

    let phi = value.acos();

    Vector3::new(
        (phi.sin() * theta.cos()) as f32,
        (phi.sin() * theta.sin()) as f32,
        phi.cos() as f32
    )
}

pub struct System {
    pub location: (f32, f32),
    pub stars: Vec<(Vector3<f32>, f32)>,
    pub light: Vector3<f32>,
    pub background_color: (f32, f32, f32)
}

impl System {
    fn new(location: (f32, f32), rng: &mut ThreadRng) -> Self {
        let distance_from_center = location.0.hypot(location.1);

        //let stars = rng.gen_range(100, 1000);
        let stars = 10000;

        let stars = (0 .. stars)
            .map(|_| (uniform_sphere_distribution(rng), rng.gen()))
            .collect();

        Self {
            light: uniform_sphere_distribution(rng),
            background_color: (0.0, 0.0, rng.gen_range(0.0, 0.25)),
            stars, location
        }
    }
}

struct Game {
    context: context::Context,
    
    ships: AutoIDMap<ShipID, Ship>,
    people: AutoIDMap<PersonID, Person>,

    system: System,
    camera: Camera,
    controls: Controls,
    plane_y: f32,
    selected: HashSet<ShipID>,
    rng: ThreadRng
}

impl Game {
    fn new(events_loop: &EventsLoop) -> Self {
        let mut rng = rand::thread_rng();

        Self {
            context: context::Context::new(events_loop),
            
            ships: AutoIDMap::new(),
            people: AutoIDMap::new(),

            system: System::new((-1.0, -1.0), &mut rng),
            camera: Camera::default(),
            controls: Controls::new(),
            plane_y: 0.0,
            selected: HashSet::new(),
            rng
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
            VirtualKeyCode::Left  | VirtualKeyCode::A => self.controls.left     = pressed,
            VirtualKeyCode::Right | VirtualKeyCode::D => self.controls.right    = pressed,
            VirtualKeyCode::Up    | VirtualKeyCode::W => self.controls.forwards = pressed,
            VirtualKeyCode::Down  | VirtualKeyCode::S => self.controls.back     = pressed,
            VirtualKeyCode::T if pressed => if let Some(point) = self.point_under_mouse() {
                self.ships.push(Ship::new(ShipType::Fighter, point, (0.0, 0.0, 0.0)));
                self.plane_y += 0.5;
            },
            VirtualKeyCode::C => self.camera.set_focus(&self.selected),
            _ => {}
        }
    }

    fn average_position(&self) -> Option<Vector3<f32>> {
        average_position(&self.selected, &self.ships)
    }

    fn update(&mut self) {
        if self.controls.middle_clicked() {
            self.camera.set_focus(&self.selected);
        }

        if self.controls.left_clicked() {
            self.selected.clear();
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

        if self.controls.right_clicked() {
            if let Some(target) = self.point_under_mouse() {
                if let Some(avg) = self.average_position() {
                    let positions = Formation::DeltaWing.arrange(self.selected.len(), avg, target, 2.5);
                    
                    let ships = &mut self.ships;

                    self.selected.iter()
                        .zip(positions.iter())
                        .for_each(|(id, position)| ships.get_mut(*id).unwrap().commands.push(Command::MoveTo(*position)));
                }
            }
        }

        self.controls.update();

        if self.controls.left {
            self.camera.move_sideways(-0.5);
        }

        if self.controls.right {
            self.camera.move_sideways(0.5);
        }

        if self.controls.forwards {
            self.camera.move_forwards(0.5);
        }

        if self.controls.back {
            self.camera.move_forwards(-0.5);
        }

        for ship in self.ships.iter_mut() {
            ship.step();
        }

        self.camera.step(&self.ships);
    }

    fn render(&mut self) {
        self.context.clear(&self.system);

        for ship in self.ships.iter() {
            ship.render(&mut self.context, &self.camera, &self.system);
        }

        self.context.render_system(&self.system, &self.camera);

        for ship in self.ships.iter() {
            if self.selected.contains(&ship.id()) {
                if let Some((x, y)) = self.context.screen_position(ship.position_matrix(), &self.camera) {
                    self.context.render_circle(x, y, 25.0);
                }

                if let Some(Command::MoveTo(e)) = ship.commands.first() {
                    if let Some(e) = self.context.screen_position(Matrix4::from_translation(*e), &self.camera) {
                        if let Some(s) = self.context.screen_position(ship.position_matrix(), &self.camera) {
                            self.context.render_line(e, s);
                        }
                    } 
                }
            }
        }

        if let Some(top_left) = self.controls.left_drag() {
            self.context.render_rect(top_left, self.controls.mouse());
        }

        self.context.render_text(&format!("Ship count: {}", self.ships.len()), 10.0, 10.0);
        self.context.render_text(&format!("Population: {}", self.people.len()), 10.0, 40.0);

        self.context.flush_ui();
        self.context.finish();
    }
}

fn main() {
    let mut events_loop = EventsLoop::new();
    
    let mut game = Game::new(&events_loop);

    let tanker = game.ships.push(Ship::new(ShipType::Tanker, Vector3::new(0.0, 0.0, 0.0), (0.0, 0.0, 0.0)));

    for _ in 0 .. 10 {
        game.people.push(Person::new(Occupation::Worker, tanker));
    }
    
    for i in 0 .. 100 {
        let x = (50.0 - i as f32) * 3.0;
        let fighter = game.ships.push(Ship::new(ShipType::Fighter, Vector3::new(x, 5.0, 0.0), (0.0, 0.0, 0.0)));
        game.people.push(Person::new(Occupation::Pilot, fighter));
    }


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