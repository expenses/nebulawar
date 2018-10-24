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
extern crate serde;
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
extern crate spade;
extern crate noise;
#[macro_use]
extern crate newtype_proxy;
extern crate tint;

use rand::*;
use glium::*;
use glutin::*;
use glutin::dpi::*;
use std::time::*;
use specs::{World, RunNow, Entities, ReadStorage, Join};
use specs::shred::FetchMut;

mod camera;
mod util;
mod context;
mod ships;
mod controls;
mod ui;
mod state;
mod common_components;
mod systems;
mod entities;

use state::*;
use ui::*;
use controls::*;
use common_components::*;
use systems::*;
use util::*;
use ships::*;
use systems::focus_on_selected;
use entities::*;
use camera::*;

struct Game {
    context: context::Context,
    rng: ThreadRng,
    ui: UI,
    world: specs::World
}

impl Game {
    fn new(mut world: World, events_loop: &EventsLoop) -> Self {
        let mut rng = rand::thread_rng();

        use cgmath::Vector2;
        let system = System::new(Vector2::new(rng.gen_range(-1.0, 1.0), rng.gen_range(-1.0, 1.0)), &mut rng, &mut world);
        world.add_resource(system);

        add_starting_entities(&mut world);

        Self {
            context: context::Context::new(events_loop),
            rng,
            ui: UI::new(),
            world
        }
    }

    fn handle_mouse_button(&mut self, button: MouseButton, pressed: bool) {
        let mut controls: FetchMut<Controls> = self.world.write_resource();

        match button {
            MouseButton::Left => controls.handle_left(pressed),
            MouseButton::Right => controls.handle_right(pressed),
            MouseButton::Middle => controls.handle_middle(pressed),
            _ => {}
        }
    }

    fn handle_kp2(&mut self, key: VirtualKeyCode, pressed: bool) {
        let mut controls: FetchMut<Controls> = self.world.write_resource();

        match key {
            VirtualKeyCode::Left   | VirtualKeyCode::A => controls.left     = pressed,
            VirtualKeyCode::Right  | VirtualKeyCode::D => controls.right    = pressed,
            VirtualKeyCode::Up     | VirtualKeyCode::W => controls.forwards = pressed,
            VirtualKeyCode::Down   | VirtualKeyCode::S => controls.back     = pressed,
            VirtualKeyCode::LShift | VirtualKeyCode::T => controls.shift = pressed,
            _ => {}
        }
    }

    fn handle_keypress(&mut self, key: VirtualKeyCode, pressed: bool) {

        match key {
            VirtualKeyCode::C => focus_on_selected(&mut self.world),
            VirtualKeyCode::Z if pressed => {
                // todo: saving etc
                //let result = self.state.save("game.sav");
                //self.print_potential_error(result);
            },
            VirtualKeyCode::L if pressed => {
                //let result = self.state.load("game.sav");
                //self.print_potential_error(result);
            },
            VirtualKeyCode::P if pressed => self.world.write_resource::<Paused>().switch(),
            VirtualKeyCode::Slash if pressed => self.context.toggle_debug(),
            VirtualKeyCode::Comma if pressed => self.world.write_resource::<Formation>().rotate_left(),
            VirtualKeyCode::Period if pressed => self.world.write_resource::<Formation>().rotate_right(),
            VirtualKeyCode::Back if pressed => {
                let to_delete: Vec<_> = {
                    let (entities, selectable): (Entities, ReadStorage<Selectable>) = self.world.system_data();

                    (&entities, &selectable).join()
                        .filter(|(_, selectable)| selectable.selected)
                        .map(|(entity, _)| entity)
                        .collect()
                };

                self.world.delete_entities(&to_delete).unwrap();
            }
            _ => self.handle_kp2(key, pressed)
        }
    }

    fn update(&mut self, secs: f32) {
        *self.world.write_resource() = Secs(secs);

        EventHandlerSystem.run_now(&self.world.res);

        SpinSystem.run_now(&self.world.res);
        ShipMovementSystem.run_now(&self.world.res);

        EntityUnderMouseSystem {
            context: &self.context
        }.run_now(&self.world.res);

        AveragePositionSystem.run_now(&self.world.res);

        RightClickInteractionSystem {
            context: &self.context
        }.run_now(&self.world.res);

        MoveOrderSystem {
            context: &self.context
        }.run_now(&self.world.res);

        MiddleClickSystem.run_now(&self.world.res);

        LeftClickSystem {
            context: &self.context
        }.run_now(&self.world.res);

        DragSelectSystem {
            context: &self.context
        }.run_now(&self.world.res);

        RightClickSystem {
            context: &self.context
        }.run_now(&self.world.res);

        UpdateControlsSystem.run_now(&self.world.res);
        
        TimeStepSystem.run_now(&self.world.res);
        StepCameraSystem.run_now(&self.world.res);
        self.ui.step(secs);
    }

    fn render(&mut self) {
        self.context.clear();

        RenderSystem {
            context: &mut self.context
        }.run_now(&self.world.res);

        RenderCommandPaths {
            context: &mut self.context
        }.run_now(&self.world.res);

        ObjectRenderer {
            context: &mut self.context
        }.run_now(&self.world.res);

        RenderDebug {
            context: &mut self.context
        }.run_now(&self.world.res);

        RenderSelected {
            context: &mut self.context
        }.run_now(&self.world.res);

        RenderMovementOrder {
            context: &mut self.context
        }.run_now(&self.world.res);

        self.render_2d_components();

        self.context.finish();
    }

    fn render_2d_components(&mut self) {
        RenderUI {
            context: &mut self.context
        }.run_now(&self.world.res);
        self.ui.render(&mut self.context);
        
        RenderDragSelection {
            context: &mut self.context
        }.run_now(&self.world.res);

        // actually draw ui components onto the screen
        let camera = self.world.read_resource();
        self.context.flush_ui(&camera, &self.world.read_resource());

        RenderMouse {
            context: &mut self.context
        }.run_now(&self.world.res);
    }

    fn change_distance(&mut self, delta: f32) {
        self.camera_mut().change_distance(delta)
    }

    fn camera_mut(&mut self) -> FetchMut<camera::Camera> {
        self.world.write_resource()
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

fn main() {
    env_logger::init();

    let mut world = World::new();
    
    world.add_resource(Time(0.0));
    world.add_resource(Secs(0.0));
    world.add_resource(Formation::default());
    world.add_resource(camera::Camera::default());
    world.add_resource(Paused(false));
    world.add_resource(RightClickInteraction(None));
    world.add_resource(EntityUnderMouse(None));
    world.add_resource(Controls::default());
    world.add_resource(MoveOrder(None));
    world.add_resource(AveragePosition(None));
    world.add_resource(Events(Vec::new()));

    world.register::<context::Model>();
    world.register::<Position>();
    world.register::<ObjectSpin>();
    world.register::<MineableMaterials>();
    world.register::<Size>();
    world.register::<Fuel>();
    world.register::<ships::ShipStorage>();
    world.register::<Commands>();
    world.register::<common_components::Rotation>();
    world.register::<ships::components::Components>();
    world.register::<ships::ShipType>();
    world.register::<Selectable>();
    world.register::<CreationTime>();
    world.register::<Parent>();
    world.register::<Occupation>();
    world.register::<Side>();

    let mut events_loop = EventsLoop::new();
    
    let mut game = Game::new(world, &events_loop);

    let mut time = Instant::now();

    let mut closed = false;
    while !closed {
        events_loop.poll_events(|event| if let glutin::Event::WindowEvent {event, ..} = event {
            game.context.copy_event(&event);

            match event {
                glutin::WindowEvent::CloseRequested => closed = true,
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
                event @ _ => game.world.write_resource::<Events>().push(event)
            }
        });

        let now = Instant::now();
        
        let secs = now.duration_since(time).subsec_nanos() as f32 / 10.0_f32.powi(9);
        time = now;

        game.update(secs);
        game.render();
    }
}

// todo: redo circle drawing
fn circle_size(z: f32) -> f32 {
    FAR / 2.0 * (1.0 - z)
}