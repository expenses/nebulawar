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
#[macro_use]
extern crate serde_derive;
extern crate bincode;

use rand::*;
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
mod maps;

use people::*;
use controls::*;

use camera::*;
use util::*;
use ships::*;
use maps::*;

fn average_position(selection: &HashSet<ShipID>, ships: &AutoIDMap<ShipID, Ship>) -> Option<Vector3<f32>> {
    if !selection.is_empty() {
        let position = selection.iter().fold(Vector3::zero(), |vector, index| {
            vector + ships[*index].position()
        }) / selection.len() as f32;

        Some(position)
    } else {
        None
    }
}

// http://corysimon.github.io/articles/uniformdistn-on-sphere/
fn uniform_sphere_distribution(rng: &mut ThreadRng) -> Vector3<f32> {
    use std::f64::consts::PI;

    let uniform = Uniform::new(0.0, 1.0);

    let x = uniform.sample(rng);
    let y = uniform.sample(rng);

    let theta = 2.0 * PI * x;
    let phi = (1.0 - 2.0 * y).acos();

    Vector3::new(
        (phi.sin() * theta.cos()) as f32,
        (phi.sin() * theta.sin()) as f32,
        phi.cos() as f32
    )
}

#[derive(Deserialize, Serialize)]
pub struct System {
    pub location: Vector2<f32>,
    pub stars: Vec<(Vector3<f32>, f32)>,
    pub light: Vector3<f32>,
    pub background_color: (f32, f32, f32)
}

impl System {
    fn new(location: Vector2<f32>, rng: &mut ThreadRng) -> Self {
        // todo: more random generation
        let _distance_from_center = location.magnitude();

        let stars = 10000;

        let stars = (0 .. stars)
            .map(|_| (uniform_sphere_distribution(rng), rng.gen()))
            .collect();

        let mut light = uniform_sphere_distribution(rng);
        light.y = light.y.abs();

        Self {
            light,
            background_color: (0.0, 0.0, rng.gen_range(0.0, 0.25)),
            stars, location
        }
    }
}

#[derive(Serialize, Deserialize)]
struct State {
    ships: AutoIDMap<ShipID, Ship>,
    people: AutoIDMap<PersonID, Person>,
    system: System,
    camera: Camera,
    selected: HashSet<ShipID>,
}

impl State {
    fn new(rng: &mut ThreadRng) -> Self {
        let mut state = Self {
            ships: AutoIDMap::new(),
            people: AutoIDMap::new(),
            system: System::new(Vector2::new(rng.gen_range(-1.0, 1.0), rng.gen_range(-1.0, 1.0)), rng),
            camera: Camera::default(),
            selected: HashSet::new()
        };

        let tanker = state.ships.push(Ship::new(ShipType::Tanker, Vector3::new(0.0, 0.0, 0.0), (0.0, 0.0, 0.0)));

        for _ in 0 .. 10 {
            state.people.push(Person::new(Occupation::Worker, tanker));
        }
        
        for i in 0 .. 100 {
            let x = (50.0 - i as f32) * 3.0;
            let fighter = state.ships.push(Ship::new(ShipType::Fighter, Vector3::new(x, 5.0, 0.0), (0.0, 0.0, 0.0)));
            state.people.push(Person::new(Occupation::Pilot, fighter));
        }

        state
    }

    fn load(filename: &str) -> Self {
        let file = ::std::fs::File::open(filename).unwrap();
        bincode::deserialize_from(file).unwrap()
    }

    fn save(&self, filename: &str) {
        let file = ::std::fs::File::create(filename).unwrap();
        bincode::serialize_into(file, self).unwrap();
    }

    fn selected(&self) -> impl Iterator<Item=&Ship> {
        self.selected.iter().map(move |id| &self.ships[*id])
    }

    fn render(&self, context: &mut context::Context) {
        for ship in self.ships.iter() {
            ship.render(context, &self.camera, &self.system);
        }

        context.render_system(&self.system, &self.camera);

        for ship in self.selected() {
            if let Some((x, y)) = context.screen_position(ship.position(), &self.camera) {
                let fuel = ship.fuel_perc();
                context.render_circle(x, y, 25.0, [1.0, fuel, fuel]);
            }

            if !ship.commands.is_empty() {
                context.render_3d_lines(ship.command_path());
            }
        }
    }

    fn selection_info(&self) -> BTreeMap<&ShipType, usize> {
        self.selected().fold(BTreeMap::new(), |mut map, ship| {
            *map.entry(ship.tag()).or_insert(0) += 1;
            map
        })
    }
}

struct Game {
    context: context::Context,
    
    state: State,
    controls: Controls,
    rng: ThreadRng,
    paused: bool
}

impl Game {
    fn new(events_loop: &EventsLoop) -> Self {
        let mut rng = rand::thread_rng();

        Self {
            context: context::Context::new(events_loop),
            state: State::new(&mut rng),
            controls: Controls::default(),
            paused: false,
            rng
        }
    }

    fn handle_mouse_movement(&mut self, x: f32, y: f32) {
        let (mouse_x, mouse_y) = self.controls.mouse();
        let (delta_x, delta_y) = (x - mouse_x, y - mouse_y);
        self.controls.set_mouse(x, y);

        if self.controls.right_dragging() {
            self.state.camera.rotate_longitude(delta_x / 200.0);
            self.state.camera.rotate_latitude(delta_y / 200.0);
        }
    }

    fn point_under_mouse(&mut self) -> Option<Vector3<f32>> {
        let ray = self.context.ray(&self.state.camera, self.controls.mouse());

        Plane::new(UP, 0.0).intersection(&ray).map(point_to_vector)
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
            VirtualKeyCode::T => self.controls.shift = pressed,
            VirtualKeyCode::C => self.state.camera.set_focus(&self.state.selected),
            VirtualKeyCode::Z if pressed => self.state.save("game.sav"),
            VirtualKeyCode::L if pressed => self.state = State::load("game.sav"),
            VirtualKeyCode::LShift => self.controls.shift = pressed,
            VirtualKeyCode::P if pressed => self.paused = !self.paused,
            VirtualKeyCode::Slash if pressed => self.context.toggle_debug(),
            _ => {}
        }
    }

    fn average_position(&self) -> Option<Vector3<f32>> {
        average_position(&self.state.selected, &self.state.ships)
    }

    fn update(&mut self) {
        if self.controls.middle_clicked() {
            self.state.camera.set_focus(&self.state.selected);
        }

        if self.controls.left_clicked() {
            self.state.selected.clear();
        }

        if let Some((mut left, mut top)) = self.controls.left_dragged() {
            let (mut right, mut bottom) = self.controls.mouse();
            
            if right < left {
                std::mem::swap(&mut right, &mut left);
            }

            if bottom < top {
                std::mem::swap(&mut top, &mut bottom);
            }

            if !self.controls.shift {
                self.state.selected.clear();
            }

            for ship in self.state.ships.iter() {
                if let Some((x, y)) = self.context.screen_position(ship.position(), &self.state.camera) {
                    if left <= x && x <= right && top <= y && y <= bottom {
                        self.state.selected.insert(ship.id());
                    }
                }
            }
        }

        if self.controls.right_clicked() {
            if let Some(target) = self.point_under_mouse() {
                if let Some(avg) = self.average_position() {
                    let positions = Formation::DeltaWing.arrange(self.state.selected.len(), avg, target, 2.5);
                    
                    let ships = &mut self.state.ships;
                    let queue = self.controls.shift;

                    self.state.selected.iter()
                        .zip(positions.iter())
                        .for_each(|(id, position)| {
                            let ship = ships.get_mut(*id).unwrap();

                            if !queue {
                                ship.commands.clear();
                            }

                            ship.commands.push(Command::MoveTo(*position))
                        });
                }
            }
        }

        self.controls.update();

        if self.controls.left {
            self.state.camera.move_sideways(-0.5);
        }

        if self.controls.right {
            self.state.camera.move_sideways(0.5);
        }

        if self.controls.forwards {
            self.state.camera.move_forwards(0.5);
        }

        if self.controls.back {
            self.state.camera.move_forwards(-0.5);
        }

        if !self.paused {
            for ship in self.state.ships.iter_mut() {
                ship.step();
            }
        }

        self.state.camera.step(&self.state.ships);
    }

    fn render(&mut self) {
        self.context.clear(&self.state.system);

        self.state.render(&mut self.context);

        if let Some(top_left) = self.controls.left_dragging() {
            self.context.render_rect(top_left, self.controls.mouse());
        }

        self.context.render_text(&format!("Ship count: {}", self.state.ships.len()), 10.0, 10.0);
        self.context.render_text(&format!("Population: {}", self.state.people.len()), 10.0, 40.0);

        for (i, (tag, num)) in self.state.selection_info().iter().enumerate() {
            self.context.render_text(&format!("{:?}: {}", tag, num), 10.0, 70.0 + i as f32 * 30.0);
        }

        self.context.flush_ui(&self.state.camera, &self.state.system);
        self.context.finish();
    }

    fn change_distance(&mut self, delta: f32) {
        self.state.camera.change_distance(delta)
    }
}

fn main() {
    let mut events_loop = EventsLoop::new();
    
    let mut game = Game::new(&events_loop);

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
                    MouseScrollDelta::PixelDelta(LogicalPosition {y, ..}) => game.change_distance(y as f32 / 20.0),
                    MouseScrollDelta::LineDelta(_, y) => game.change_distance(-y * 2.0)
                },
                _ => ()
            }
        });

        game.update();
        game.render();
    }
}