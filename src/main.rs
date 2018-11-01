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
extern crate nalgebra;
extern crate ncollide3d;

use rand::*;
use glium::*;
use glutin::*;
use glutin::dpi::*;
use std::time::*;
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

use star_system::*;
use controls::*;
use components::*;
use systems::*;
use util::*;
use ships::*;
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
            .with(StepCameraSystem, "camera", &["apply"])

            .with(FinishSeekSystem, "finish_seek", &["apply", "set_rotation"])

            .with(EntityUnderMouseSystem, "mouse_entity", &["mouse_ray", "apply", "set_rotation", "spin"])

            .with(RightClickInteractionSystem, "right_click_interaction", &["mouse_entity"])
            .with(LeftClickSystem, "left_click", &["mouse_entity"])

            .with(RightClickSystem, "right_click", &["right_click_interaction"])
            
            .with(DestroyShips, "destroy_ships", &["kamikaze"])

            .with(UpdateControlsSystem, "update_controls", &["left_click", "right_click", "middle_click", "drag"]);

        info!("Dispatcher graph:\n{:?}", builder);

        let (context, meshes) = context::Context::new(events_loop);

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
        self.context.clear();

        RenderCommandPaths  (&mut self.context).run_now(&self.world.res);
        RenderSystem        (&mut self.context).run_now(&self.world.res);
        ObjectRenderer      (&mut self.context).run_now(&self.world.res); 
        RenderBillboards    (&mut self.context).run_now(&self.world.res);
        FlushSmoke          (&mut self.context).run_now(&self.world.res); 
        RenderDebug         (&mut self.context).run_now(&self.world.res);
        RenderSelected      (&mut self.context).run_now(&self.world.res);
        RenderMovementPlane (&mut self.context).run_now(&self.world.res);
        RenderUI            (&mut self.context).run_now(&self.world.res);
        RenderLogSystem     (&mut self.context).run_now(&self.world.res);
        RenderDragSelection (&mut self.context).run_now(&self.world.res);
        FlushUI             (&mut self.context).run_now(&self.world.res);
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
    world.add_resource(Debug(false));

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