#![cfg_attr(feature = "cargo-clippy", allow(clippy::type_complexity))]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::too_many_arguments))]

extern crate arrayvec;
extern crate cgmath;
extern crate collision;
extern crate genmesh;
extern crate image;
extern crate lyon;
extern crate obj;
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
extern crate noise;
extern crate spade;
#[macro_use]
extern crate newtype_proxy;
extern crate futures;
extern crate nalgebra;
extern crate ncollide3d;
extern crate tint;
extern crate wgpu;
extern crate winit;
extern crate zerocopy;

use rand::rngs::*;
use rand::*;
use specs::prelude::*;
use specs::shred::{Dispatcher, DispatcherBuilder};
use winit::*;

mod camera;
mod components;
mod context;
mod controls;
mod entities;
mod resources;
mod ships;
mod star_system;
mod systems;
mod tests;
mod util;

use crate::components::*;
use crate::resources::*;
use crate::ships::*;
use crate::star_system::*;
use crate::util::*;
use controls::*;
use entities::*;
use systems::*;

pub struct SaveLoad;

pub type Marker = specs::saveload::SimpleMarker<SaveLoad>;
pub type MarkerAllocator = specs::saveload::SimpleMarkerAllocator<SaveLoad>;

struct Game {
    context: context::Context,
    world: specs::World,
    update_dispatcher: Dispatcher<'static, 'static>,
    render_dispatcher: Dispatcher<'static, 'static>,
}

impl Game {
    async fn new(mut world: World, events_loop: &event_loop::EventLoop<()>) -> Self {
        let update_builder = DispatcherBuilder::new()
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
            .with(
                MergeForceSystem,
                "merge",
                &["events", "seek", "avoidance", "friction"],
            )
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
            .with(
                EntityUnderMouseSystem,
                "mouse_entity",
                &["mouse_ray", "apply", "set_rotation", "spin"],
            )
            .with(
                RightClickInteractionSystem,
                "right_click_interaction",
                &["mouse_entity"],
            )
            .with(LeftClickSystem, "left_click", &["mouse_entity"])
            .with(
                RightClickSystem,
                "right_click",
                &["right_click_interaction"],
            )
            .with(DestroyShips, "destroy_ships", &["kamikaze"])
            .with(StepExplosion, "step_explosion", &["destroy_ships"])
            .with(
                UpdateControlsSystem,
                "update_controls",
                &["left_click", "middle_click", "right_click"],
            );

        let render_builder = DispatcherBuilder::new()
            .with(RenderCommandPaths, "RenderCommandPaths", &[])
            .with(RenderSystem, "RenderSystem", &[])
            .with(ObjectRenderer, "ObjectRenderer", &[])
            .with(RenderBillboards, "RenderBillboards", &[])
            .with(RenderDebug, "RenderDebug", &[])
            .with(RenderSelected, "RenderSelected", &[])
            .with(RenderMovementPlane, "RenderMovementPlane", &[])
            .with(RenderUI, "RenderUI", &[])
            .with(RenderLogSystem, "RenderLogSystem", &[])
            .with(RenderDragSelection, "RenderDragSelection", &[])
            .with(RenderMouse, "RenderMouse", &[]);

        info!("Update dispatcher graph:\n{:?}", update_builder);
        info!("Render dispatcher graph:\n{:?}", render_builder);

        let (context, meshes) = context::Context::new(events_loop).await;

        world.insert(Meshes::new(meshes));
        world.insert(Dpi(context.dpi()));

        Self {
            context,
            world,
            update_dispatcher: update_builder.build(),
            render_dispatcher: render_builder.build(),
        }
    }

    fn update(&mut self, secs: f32) {
        *self.world.write_resource() = Secs(secs);
        *self.world.write_resource() = ScreenDimensions(self.context.screen_dimensions().into());

        self.update_dispatcher.dispatch(&self.world);

        self.world.maintain();
    }

    fn render(&mut self) {
        self.render_dispatcher.dispatch(&self.world);

        let (camera, mut system, mut buffers, mut line_buffers, mut billboards, mut text): (
            specs::Read<camera::Camera>,
            specs::Write<star_system::StarSystem>,
            specs::Write<context::ModelBuffers>,
            specs::Write<context::LineBuffers>,
            specs::Write<context::BillboardBuffer>,
            specs::Write<context::TextBuffer>,
        ) = self.world.system_data();
        self.context.render(
            &mut buffers,
            &mut line_buffers,
            &mut billboards,
            &mut text,
            wgpu::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
            &camera,
            &mut system,
        );
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
        winit::event::Event::WindowEvent { event, .. } => {
            game.context.copy_event(&event);

            match event {
                winit::event::WindowEvent::CloseRequested => {
                    *control_flow = winit::event_loop::ControlFlow::Exit
                }
                winit::event::WindowEvent::Resized(size) => {
                    game.context.resize(size.width, size.height);
                }
                event => game
                    .world
                    .write_resource::<Events>()
                    .push(event.to_static().unwrap()),
            }
        }
        winit::event::Event::MainEventsCleared => {
            let now = wasm_timer::Instant::now();

            let secs = now.duration_since(time).subsec_nanos() as f32 / 10.0_f32.powi(9);
            time = now;

            game.update(secs);
            game.request_redraw();
        }
        winit::event::Event::RedrawRequested(_) => game.render(),
        _ => {}
    });
}

fn create_world() -> World {
    let mut world = World::new();

    // Stuff to save

    world.insert(Time(0.0));
    world.insert(Formation::default());
    world.insert(camera::Camera::default());
    world.insert(Paused(false));
    world.insert(Log(Vec::new()));
    world.insert(MovementPlane(0.0));
    world.insert(Debug(false));
    world.insert(Help(true));

    world.insert(context::ModelBuffers::default());
    world.insert(context::LineBuffers::default());
    world.insert(context::BillboardBuffer::default());
    world.insert(context::TextBuffer::default());

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

    world.insert(Secs(0.0));
    world.insert(RightClickOrder::default());
    world.insert(EntityUnderMouse(None));
    world.insert(Controls::default());
    world.insert(AveragePosition(None));
    world.insert(Events(Vec::new()));
    world.insert(MouseRay::default());
    world.insert(MarkerAllocator::new());
    world.insert(ScreenDimensions::default());

    world.register::<SeekPosition>();
    world.register::<SeekForce>();
    world.register::<AvoidanceForce>();
    world.register::<FrictionForce>();

    world.register::<Marker>();

    let mut rng = rand::thread_rng();

    use cgmath::Vector2;
    let system = StarSystem::new(
        Vector2::new(rng.gen_range(-1.0, 1.0), rng.gen_range(-1.0, 1.0)),
        &mut rng,
        &mut world,
    );
    world.insert(system);

    add_starting_entities(&mut world, &mut rng);

    world
}
