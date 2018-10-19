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
extern crate pedot;
extern crate failure;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate odds;

extern crate specs;
#[macro_use]
extern crate specs_derive;

use rand::*;
use glium::*;
use glutin::*;
use glutin::dpi::*;
use cgmath::*;
use collision::*;
use std::collections::*;
use std::time::*;
use specs::{Join, World, RunNow, DenseVecStorage};

mod camera;
mod util;
mod context;
mod ships;
mod controls;
mod people;
mod maps;
mod ui;
mod state;

use state::*;
use ui::*;
use controls::*;

use util::*;
use ships::*;
use maps::*;

fn average_position(selection: &HashSet<ShipID>, ships: &AutoIDMap<ShipID, Ship>) -> Option<Vector3<f32>> {
    let (sum_position, num) = selection.iter()
            .filter_map(|id| ships.get(*id))
            .fold((Vector3::zero(), 0), |(vector, total), ship| {
                (vector + ship.position(), total + 1)
            });

    if num > 0 {
        Some(sum_position / num as f32)
    } else {
        None
    }
}

struct Game {
    context: context::Context,
    state: State,
    controls: Controls,
    rng: ThreadRng,
    ui: UI,
    world: specs::World
}

impl Game {
    fn new(mut world: World, events_loop: &EventsLoop) -> Self {
        let mut rng = rand::thread_rng();

        Self {
            context: context::Context::new(events_loop),
            state: State::new(&mut world, &mut rng),
            controls: Controls::default(),
            rng,
            ui: UI::new(),
            world
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
            VirtualKeyCode::Z if pressed => {
                let result = self.state.save("game.sav");
                self.print_potential_error(result);
            },
            VirtualKeyCode::L if pressed => {
                let result = self.state.load("game.sav");
                self.print_potential_error(result);
            },
            VirtualKeyCode::LShift => self.controls.shift = pressed,
            VirtualKeyCode::P if pressed => self.state.paused = !self.state.paused,
            VirtualKeyCode::Slash if pressed => self.context.toggle_debug(),
            VirtualKeyCode::Comma if pressed => self.state.formation.rotate_left(),
            VirtualKeyCode::Period if pressed => self.state.formation.rotate_right(),
            VirtualKeyCode::Back if pressed => for ship in &self.state.selected {
                self.state.ships.remove(*ship);
            },
            _ => {}
        }
    }

    fn ship_under_mouse(&self) -> Option<ShipID> {
        let (mouse_x, mouse_y) = self.controls.mouse();

        self.state.ships.iter()
            .filter_map(|ship| {
                self.context.screen_position(ship.position(), &self.state.camera)
                    .filter(|(x, y, z)| {
                        (mouse_x - x).hypot(mouse_y - y) < circle_size(*z)
                    })
                    .map(|(_, _, z)| (ship, z))
            })
            .min_by(|(_, z_a), (_, z_b)| z_a.partial_cmp(z_b).unwrap_or(::std::cmp::Ordering::Less))
            .map(|(ship, _)| ship.id())
    }

    fn right_click_interaction(&self) -> Option<(ShipID, Interaction)> {
        self.ship_under_mouse()
            .map(|ship| {
                let interaction = if self.state.ships[ship].out_of_fuel() {
                    Interaction::Refuel
                } else {
                    Interaction::RefuelFrom
                };

                (ship, interaction)
            })
    }

    fn order_ships_to(&mut self, target: Vector3<f32>) {
        if let Some(avg) = self.state.average_position() {
            let positions = self.state.formation.arrange(self.state.selected.len(), avg, target, 2.5);
            
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

    fn update(&mut self, secs: f32) {
        {
            let mut world_secs = self.world.write_resource::<Secs>();
            *world_secs = Secs(secs);
        }

        SpinSystem.run_now(&self.world.res);

        if self.controls.middle_clicked() {
            self.state.camera.set_focus(&self.state.selected);
        }

        if self.controls.left_clicked() {
            if !self.controls.shift {
                self.state.selected.clear();
            }

            if let Some(ship) = self.ship_under_mouse() {
                self.state.selected.insert(ship);
            }
        }

        if let Some((left, top, right, bottom)) = self.controls.left_drag_rect() {
            if !self.controls.shift {
                self.state.selected.clear();
            }

            for ship in self.state.ships.iter() {
                if let Some((x, y, _)) = self.context.screen_position(ship.position(), &self.state.camera) {
                    if left <= x && x <= right && top <= y && y <= bottom {
                        if !self.state.selected.remove(&ship.id()) {
                            self.state.selected.insert(ship.id());
                        }
                    }
                }
            }
        }

        if self.controls.right_clicked() {
            if let Some((target, interaction)) = self.right_click_interaction() {
                for ship in &self.state.selected {
                    if *ship != target {
                        if let Some(ship) = self.state.ships.get_mut(*ship) {
                            if !self.controls.shift {
                                ship.commands.clear();
                            }

                            ship.commands.push(Command::GoToAnd(target, interaction));
                        }
                    }
                }
            } else if let Some(target) = self.point_under_mouse() {
                self.order_ships_to(target);
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
        
        self.state.step(secs);
        self.ui.step(secs);
    }

    fn render(&mut self) {
        self.context.clear(&self.state.system);
        self.state.render(&mut self.context);

        self.render_2d_components();

        if self.context.is_debugging() {
            self.render_debug();
        }

        AsteroidRenderer {
            context: &mut self.context,
            camera: &self.state.camera,
            system: &self.state.system
        }.run_now(&self.world.res);

        self.context.finish();
    }

    fn render_2d_components(&mut self) {
        self.ui.render(&self.state, &mut self.context);
        
        if let Some(top_left) = self.controls.left_dragging() {
            self.context.render_rect(top_left, self.controls.mouse());
        }

        // actually draw ui components onto the screen
        self.context.flush_ui(&self.state.camera, &self.state.system);

        if let Some((_, interaction)) = self.right_click_interaction() {
            let (x, y) = self.controls.mouse();
            self.context.render_image(interaction.image(), x + 32.0, y + 32.0, 64.0, 64.0, [0.0; 4]);
        }
    }

    fn render_debug(&mut self) {
        if let Some(point) = self.point_under_mouse() {
            self.context.render_model(context::Model::Asteroid, point, Quaternion::zero(), 0.1, &self.state.camera, &self.state.system);
        }
    }

    fn change_distance(&mut self, delta: f32) {
        self.state.camera.change_distance(delta)
    }

    fn print_error<E: failure::Fail>(&mut self, error: &E) {
        error!("{}", error);
        if let Some(cause) = error.cause() {
            error!("Cause: {}", cause);
        }

        self.ui.append_to_log(error.to_string());
    }

    fn print_potential_error<E: failure::Fail>(&mut self, result: Result<(), E>) {
        if let Err(error) = result {
            self.print_error(&error);
        }
    }
}

struct AsteroidRenderer<'a> {
    context: &'a mut context::Context,
    camera: &'a camera::Camera,
    system: &'a System
}

impl<'a> specs::System<'a> for AsteroidRenderer<'a> {
    type SystemData = (
        specs::ReadStorage<'a, Position>,
        specs::ReadStorage<'a, ObjectSpin>,
        specs::ReadStorage<'a, Size>,
        specs::ReadStorage<'a, context::Model>
    );

    fn run(&mut self, (pos, spin, size, model): Self::SystemData) {
        for (pos, spin, size, model) in (&pos, &spin, &size, &model).join() {
            self.context.render_model(*model, pos.0, spin.to_quat(), size.0, &self.camera, &self.system);
        }
    }
}

struct SpinSystem;

impl<'a> specs::System<'a> for SpinSystem {
    type SystemData = (
        specs::Read<'a, Secs>,
        specs::WriteStorage<'a, ObjectSpin>
    );

    fn run(&mut self, (secs, mut spins): Self::SystemData) {
        for spin in (&mut spins).join() {
            spin.turn(secs.0);
        }
    }
}

#[derive(Component, Default)]
struct Secs(f32);

fn main() {
    env_logger::init();

    let mut world = World::new();
    world.add_resource(Secs(0.0));
    world.register::<context::Model>();
    world.register::<Position>();
    world.register::<ObjectSpin>();
    world.register::<MineableMaterials>();
    world.register::<Size>();

    let mut events_loop = EventsLoop::new();
    
    let mut game = Game::new(world, &events_loop);

    let mut time = Instant::now();

    let mut closed = false;
    while !closed {
        events_loop.poll_events(|event| if let glutin::Event::WindowEvent {event, ..} = event {
            game.context.copy_event(&event);

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

        let now = Instant::now();
        
        let secs = now.duration_since(time).subsec_nanos() as f32 / 10.0_f32.powi(9);
        time = now;

        game.update(secs);
        game.render();
    }
}

fn circle_size(z: f32) -> f32 {
    5000.0 * (1.0 - z)
}