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

use rand::*;
use glium::*;
use glutin::*;
use glutin::dpi::*;
use cgmath::*;
use collision::*;
use std::collections::*;
use std::f32::consts::*;
use std::time::*;

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
    if !selection.is_empty() {
        let position = selection.iter().fold(Vector3::zero(), |vector, index| {
            vector + ships[*index].position()
        }) / selection.len() as f32;

        Some(position)
    } else {
        None
    }
}

struct Game {
    context: context::Context,
    state: State,
    controls: Controls,
    rng: ThreadRng,
    ui: UI
}

impl Game {
    fn new(events_loop: &EventsLoop) -> Self {
        let mut rng = rand::thread_rng();

        Self {
            context: context::Context::new(events_loop),
            state: State::new(&mut rng),
            controls: Controls::default(),
            rng,
            ui: UI::new()
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
            VirtualKeyCode::P if pressed => self.state.paused = !self.state.paused,
            VirtualKeyCode::Slash if pressed => self.context.toggle_debug(),
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

    fn update(&mut self, secs: f32) {
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
                if let Some((x, y, _)) = self.context.screen_position(ship.position(), &self.state.camera) {
                    if left <= x && x <= right && top <= y && y <= bottom {
                        self.state.selected.insert(ship.id());
                    }
                }
            }
        }

        if self.controls.right_clicked() {
            if let Some(target) = self.point_under_mouse() {
                if let Some(avg) = self.state.average_position() {
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
        
        self.state.step(secs);
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

        self.ui.render(&mut self.context);

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