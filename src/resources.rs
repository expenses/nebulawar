use specs::*;
use cgmath::*;
use glium::glutin::*;
use ships::*;
use odds::vec::*;
use context;

#[derive(Component, Default, NewtypeProxy)]
pub struct Secs(pub f32);

#[derive(Component, Default, Serialize, Deserialize, Clone)]
pub struct Time(pub f32);

#[derive(Component, Default, Serialize, Deserialize, Clone)]
pub struct Paused(pub bool);

impl Paused {
    pub fn switch(&mut self) {
        self.0 = !self.0;
    }
}

#[derive(Component, Default)]
pub struct EntityUnderMouse(pub Option<(Entity, Vector3<f32>)>);

// todo: have this on a per-entity basis

#[derive(Component, Default)]
pub struct RightClickOrder {
    pub to_move: Vec<Entity>,
    pub command: Option<Command>
}

#[derive(Component, Default)]
pub struct AveragePosition(pub Option<Vector3<f32>>);

#[derive(Component, Default, NewtypeProxy)]
pub struct Events(pub Vec<WindowEvent>);

#[derive(Component, Default, Clone, Serialize, Deserialize)]
pub struct MovementPlane(pub f32);

#[derive(Component, NewtypeProxy, Default, Clone, Serialize, Deserialize)]
pub struct Log(pub Vec<LogItem>);

impl Log {
    pub fn append(&mut self, text: String) {
        self.push(LogItem {
            age: 0.0,
            content: text
        })
    }

    pub fn step(&mut self, secs: f32) {
        self.retain_mut(|item| {
            item.age += secs;
            item.age < 5.0
        });
    }

    pub fn render(&self, context: &mut context::Context) {
        let (_, height) = context.screen_dimensions();

        for (i, item) in self.iter().enumerate() {
            context.render_text(&item.content, 10.0, height - 30.0 - i as f32 * 20.0);
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LogItem {
    age: f32,
    content: String
}

#[derive(Component, NewtypeProxy)]
pub struct MouseRay(pub collision::Ray<f32, Point3<f32>, Vector3<f32>>);

impl Default for MouseRay {
    fn default() -> Self {
        MouseRay(collision::Ray::new(
            Point3::new(0.0, 0.0, 0.0),
            Vector3::zero()
        ))
    }
}

#[derive(Component, Default)]
pub struct ScreenDimensions(pub (f32, f32));