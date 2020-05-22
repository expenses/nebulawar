#![cfg_attr(feature = "cargo-clippy", allow(clippy::type_complexity))]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::too_many_arguments))]

extern crate obj;
extern crate genmesh;
extern crate image;
extern crate arrayvec;
extern crate cgmath;
extern crate lyon;
extern crate collision;
#[macro_use]
extern crate derive_is_enum_variant;
extern crate rand;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate bincode;
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
extern crate nalgebra;
extern crate ncollide3d;
extern crate wgpu;
extern crate winit;
extern crate futures;
extern crate zerocopy;

use rand::*;
use rand::rngs::*;
use winit::*;
use specs::{World, RunNow};
use specs::shred::{Dispatcher, DispatcherBuilder};

mod camera;
mod util;
mod context;
mod ships;
mod controls;
mod star_system;
mod components;
mod systems;
mod entities;
mod tests;
mod resources;

use crate::star_system::*;
use controls::*;
use crate::components::*;
use systems::*;
use crate::util::*;
use crate::ships::*;
use entities::*;
use crate::resources::*;

struct Game {
    context: context::Context,
    world: specs::World,
    dispatcher: Dispatcher<'static, 'static>
}

impl Game {
    async fn new(mut world: World, events_loop: &event_loop::EventLoop<()>) -> Self {
        let builder = DispatcherBuilder::new()
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
            .with(MiddleClickSystem, "middle_click", &["events"])
            .with(SaveSystem, "save", &["events"])
            .with(LoadSystem, "load", &["events"])
    
            .with(MergeForceSystem, "merge", &["events", "seek", "avoidance", "friction"])

            .with(ApplyVelocitySystem, "apply", &["merge"])
            .with(SetRotationSystem, "set_rotation", &["merge"])

            .with(ShipMovementSystem, "ship_movement", &["apply"])
            .with(SpawnSmokeSystem, "spawn_smoke", &["apply"])
            .with(AveragePositionSystem, "avg_pos", &["apply"])
            .with(DragSelectSystem, "drag", &["apply"])
            .with(ShootStuffSystem, "shooting", &["apply"])
            .with(KamikazeSystem, "kamikaze", &["apply"])
            .with(StepCameraSystem, "camera", &[])

            .with(FinishSeekSystem, "finish_seek", &["apply", "set_rotation"])

            .with(EntityUnderMouseSystem, "mouse_entity", &["mouse_ray", "apply", "set_rotation", "spin"])

            .with(RightClickInteractionSystem, "right_click_interaction", &["mouse_entity"])
            .with(LeftClickSystem, "left_click", &["mouse_entity"])

            .with(RightClickSystem, "right_click", &["right_click_interaction"])
            
            .with(DestroyShips, "destroy_ships", &["kamikaze"])

            .with(StepExplosion, "step_explosion", &["destroy_ships"])

            .with(UpdateControlsSystem, "update_controls", &["left_click", "middle_click", "right_click"]);

        info!("Dispatcher graph:\n{:?}", builder);

        let (context, meshes) = context::Context::new(events_loop).await;

        world.add_resource(Meshes::new(meshes));

        Self {
            context, world,
            dispatcher: builder.build()
        }
    }

    fn update(&mut self, secs: f32) {
        *self.world.write_resource() = Secs(secs);
        *self.world.write_resource() = ScreenDimensions(self.context.screen_dimensions());

        self.dispatcher.dispatch(&self.world.res);

        self.world.maintain();
    }

    fn render(&mut self) {
        RenderCommandPaths.run_now(&self.world.res);
        RenderSystem.run_now(&self.world.res);
        ObjectRenderer.run_now(&self.world.res); 
        RenderBillboards.run_now(&self.world.res);
        RenderDebug.run_now(&self.world.res);
        RenderSelected.run_now(&self.world.res);
        RenderMovementPlane.run_now(&self.world.res);
        RenderUI            .run_now(&self.world.res);
        RenderLogSystem     .run_now(&self.world.res);
        RenderDragSelection .run_now(&self.world.res);
        RenderMouse         .run_now(&self.world.res);


        let (camera, system, mut buffers, mut line_buffers): (
            specs::Read<camera::Camera>, specs::Read<star_system::StarSystem>,
            specs::Write<context::Buffers>, specs::Write<context::LineBuffers>
        ) = self.world.system_data();
        self.context.render(&mut buffers, &mut line_buffers, wgpu::Color {r: 0.0, g: 0.0, b: 0.0, a: 1.0}, &camera, &system);
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

    fn request_redraw(&self) {
        self.context.request_redraw();
    }
}

use std::alloc::System;

#[global_allocator]
static GLOBAL: System = System;


fn main() {
    #[cfg(feature = "native")]
    futures::executor::block_on(run());
    #[cfg(feature = "wasm")]
    wasm_bindgen_futures::spawn_local(run());
}

async fn run() {
    #[cfg(feature = "native")]
    env_logger::init();
    #[cfg(feature = "wasm")]
    {
        console_error_panic_hook::set_once();
        console_log::init_with_level(log::Level::Trace).unwrap();
    }

    let events_loop = event_loop::EventLoop::new();
    
    let mut game = Game::new(create_world(), &events_loop).await;

    let mut time = wasm_timer::Instant::now();

    events_loop.run(move |event, _, control_flow| match event {
        winit::event::Event::WindowEvent {event, ..} => {
            game.context.copy_event(&event);

            match event {
                winit::event::WindowEvent::CloseRequested => *control_flow = winit::event_loop::ControlFlow::Exit,
                winit::event::WindowEvent::Resized(size) => {
                    game.context.resize(size.width, size.height);
                }
                event => game.world.write_resource::<Events>().push(event.to_static().unwrap())
            }
        },
        winit::event::Event::MainEventsCleared => {
            let now = wasm_timer::Instant::now();
        
            let secs = now.duration_since(time).subsec_nanos() as f32 / 10.0_f32.powi(9);
            time = now;

            game.update(secs);
            game.request_redraw();
        },
        winit::event::Event::RedrawRequested(_) => game.render(),
        _ => {}
    });
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
    world.add_resource(Debug(false));
    world.add_resource(Help(true));
    world.add_resource(context::Buffers::default());
    world.add_resource(context::LineBuffers::default());

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
    world.register::<AttackTarget>();
    world.register::<Health>();
    world.register::<NoCollide>();
    world.register::<ExplosionSize>();
    world.register::<Explosion>();

    // Temp generated stuff
    
    world.add_resource(Secs(0.0));
    world.add_resource(RightClickOrder::default());
    world.add_resource(EntityUnderMouse(None));
    world.add_resource(Controls::default());
    world.add_resource(AveragePosition(None));
    world.add_resource(Events(Vec::new()));
    world.add_resource(MouseRay::default());
    world.add_resource(specs::saveload::U64MarkerAllocator::new());
    world.add_resource(ScreenDimensions::default());
    
    world.register::<SeekPosition>();
    world.register::<SeekForce>();
    world.register::<AvoidanceForce>();
    world.register::<FrictionForce>();

    world.register::<specs::saveload::U64Marker>();

    let mut rng = rand::thread_rng();

    use cgmath::Vector2;
    let system = StarSystem::new(Vector2::new(rng.gen_range(-1.0, 1.0), rng.gen_range(-1.0, 1.0)), &mut rng, &mut world);
    world.add_resource(system);

    add_starting_entities(&mut world, &mut rng);

    world
}
