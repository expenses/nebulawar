#![cfg_attr(feature = "cargo-clippy", allow(clippy::type_complexity))]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::too_many_arguments))]

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
use specs::{World, RunNow};
use specs::shred::{Fetch, FetchMut, Dispatcher};

mod camera;
mod util;
mod context;
mod ships;
mod controls;
mod state;
mod components;
mod systems;
mod entities;
mod tests;
mod resources;

use state::*;
use controls::*;
use components::*;
use systems::*;
use util::*;
use ships::*;
use systems::focus_on_selected;
use entities::*;
use resources::*;

struct Game {
    context: context::Context,
    world: specs::World,
    dispatcher: Dispatcher<'static, 'static>
}

impl Game {
    fn new(mut world: World, events_loop: &EventsLoop) -> Self {
        

        add_starting_entities(&mut world);

        let dispatcher = specs::DispatcherBuilder::new()
            .with(EventHandlerSystem, "events", &[])
            .with(SeekSystem, "seek", &[])
            .with(AvoidanceSystem, "avoidance", &[])
            .with(FrictionSystem, "friction", &[])

            // these have to wait for events because of stuff like paused being pressed
            .with(SetMouseRay, "mouse_ray", &["events"])
            .with(TimeStepSystem, "time step", &["events"])
            .with(StepLogSystem, "step log", &["events"])
            .with(ReduceAttackTime, "reduce_attack", &["events"])
            .with(TickTimedEntities, "tick_timed", &["events"])
            .with(TestDeleteSystem, "test_delete", &["events"])
            .with(SpinSystem, "spin", &["events"])
    
            .with(MergeForceSystem, "merge", &["events", "seek", "avoidance", "friction"])

            .with(ApplyVelocitySystem, "apply", &["merge"])
            .with(SetRotationSystem, "set_rotation", &["merge"])

            .with(ShipMovementSystem, "ship_movement", &["apply"])
            .with(SpawnSmokeSystem, "smoke", &["apply"])
            .with(FinishSeekSystem, "finish_seek", &["apply", "set_rotation"])
            
            .build();

        Self {
            context: context::Context::new(events_loop),
            world, dispatcher
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

    fn handle_keypress(&mut self, key: VirtualKeyCode, pressed: bool) {
        // todo: move this stuff into ecs


        match key {
            VirtualKeyCode::C => focus_on_selected(&mut self.world),
            VirtualKeyCode::P if pressed => self.world.write_resource::<Paused>().switch(),
            VirtualKeyCode::Slash if pressed => self.context.toggle_debug(),
            VirtualKeyCode::Comma if pressed => self.world.write_resource::<Formation>().rotate_left(),
            VirtualKeyCode::Period if pressed => self.world.write_resource::<Formation>().rotate_right(),
            _ => self.handle_kp2(key, pressed)
        }
    }

    fn update(&mut self, secs: f32) {
        *self.world.write_resource() = Secs(secs);
        *self.world.write_resource() = ScreenDimensions(self.context.screen_dimensions());

        self.dispatcher.dispatch(&self.world.res);

        EntityUnderMouseSystem      (&self.context).run_now(&self.world.res);
        AveragePositionSystem                      .run_now(&self.world.res);
        RightClickInteractionSystem                .run_now(&self.world.res);
        MiddleClickSystem                          .run_now(&self.world.res);
        LeftClickSystem                            .run_now(&self.world.res);
        DragSelectSystem                           .run_now(&self.world.res);
        RightClickSystem                           .run_now(&self.world.res);
        ShootStuffSystem                           .run_now(&self.world.res);
        StepCameraSystem                           .run_now(&self.world.res);

        let controls: Controls = {
            let controls: Fetch<Controls> = self.world.read_resource();
            controls.clone()
        };

        if controls.save {
            SaveSystem.run_now(&self.world.res);
        } else if controls.load {
            self.world = create_world();
            LoadSystem.run_now(&self.world.res);
        }
        
        UpdateControlsSystem                       .run_now(&self.world.res);

        self.world.maintain();
    }

    fn render(&mut self) {
        self.context.clear();

        
        RenderCommandPaths  (&mut self.context).run_now(&self.world.res);
        ObjectRenderer      (&mut self.context).run_now(&self.world.res);
        RenderDebug         (&mut self.context).run_now(&self.world.res);
        RenderSelected      (&mut self.context).run_now(&self.world.res);
        RenderMovementPlane (&mut self.context).run_now(&self.world.res);
        RenderSystem        (&mut self.context).run_now(&self.world.res);
        RenderUI            (&mut self.context).run_now(&self.world.res);
        RenderLogSystem     (&mut self.context).run_now(&self.world.res);
        RenderDragSelection (&mut self.context).run_now(&self.world.res);
        FlushUI             (&mut self.context).run_now(&self.world.res);
        RenderBillboards    (&mut self.context).run_now(&self.world.res);
        RenderMouse         (&mut self.context).run_now(&self.world.res);

        self.context.finish();
    }

    fn print_error<E: failure::Fail>(&mut self, error: &E) {
        error!("{}", error);
        if let Some(cause) = error.cause() {
            error!("Cause: {}", cause);
        }

        self.world.write_resource::<Log>().append(error.to_string());
    }

    fn print_potential_error<E: failure::Fail>(&mut self, result: Result<(), E>) {
        if let Err(error) = result {
            self.print_error(&error);
        }
    }
}

fn main() {
    env_logger::init();

    let mut events_loop = EventsLoop::new();
    
    let mut game = Game::new(create_world(), &events_loop);

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
                event => game.world.write_resource::<Events>().push(event)
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

fn create_world() -> World {
    let mut world = World::new();
    
    // Stuff to save
    
    world.add_resource(Time(0.0));
    world.add_resource(Formation::default());
    world.add_resource(camera::Camera::default());
    world.add_resource(Paused(false));
    world.add_resource(Log(Vec::new()));
    world.add_resource(MovementPlane(0.0));

    world.register::<Position>();
    world.register::<Velocity>();
    world.register::<components::Rotation>();
    world.register::<Size>();
    world.register::<Selectable>();
    world.register::<context::Model>();
    world.register::<ObjectSpin>();
    world.register::<Side>();
    world.register::<Commands>();
    world.register::<ships::ShipType>();
    world.register::<MaxSpeed>();
    world.register::<Occupation>();
    world.register::<Parent>();
    world.register::<CreationTime>();
    world.register::<DrillSpeed>();
    world.register::<MineableMaterials>();
    
    world.register::<Materials>();
    world.register::<TimeLeft>();
    world.register::<context::Image>();
    world.register::<CanAttack>();
    world.register::<SpawnSmoke>();

    // Temp generated stuff
    
    world.add_resource(Secs(0.0));
    world.add_resource(RightClickOrder::default());
    world.add_resource(EntityUnderMouse(None));
    world.add_resource(Controls::default());
    world.add_resource(AveragePosition(None));
    world.add_resource(Events(Vec::new()));
    world.add_resource(MouseRay::default());
    world.add_resource(specs::saveload::U64MarkerAllocator::new());
    world.add_resource(ScreenDimensions((0.0, 0.0)));
    
    world.register::<SeekPosition>();
    world.register::<SeekForce>();
    world.register::<AvoidanceForce>();
    world.register::<FrictionForce>();

    world.register::<specs::saveload::U64Marker>();

    let mut rng = rand::thread_rng();

    use cgmath::Vector2;
    let system = StarSystem::new(Vector2::new(rng.gen_range(-1.0, 1.0), rng.gen_range(-1.0, 1.0)), &mut rng, &mut world);
    world.add_resource(system);

    world
}