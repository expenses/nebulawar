use specs::*;
use cgmath::*;
use util::*;
use rand::*;
use ships::*;
use glium::glutin::*;
use odds::vec::*;
use context;

#[derive(Component, Default, NewtypeProxy)]
pub struct Secs(pub f32);

#[derive(Component, Default)]
pub struct Time(pub f32);

#[derive(Component, Default)]
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

#[derive(Deserialize, Serialize, Component, NewtypeProxy, Clone, Copy, PartialEq, Debug)]
#[storage(VecStorage)]
pub struct Position(pub Vector3<f32>);

#[derive(Deserialize, Serialize, Component, NewtypeProxy)]
pub struct Materials(pub StoredResource);

#[derive(Deserialize, Serialize, Component, NewtypeProxy)]
pub struct MineableMaterials(pub StoredResource);

#[derive(Deserialize, Serialize, Component, NewtypeProxy, Clone, Copy)]
#[storage(VecStorage)]
pub struct Size(pub f32);

#[derive(Component, NewtypeProxy)]
pub struct Rotation(pub Quaternion<f32>);

#[derive(Component)]
pub struct Selectable {
    pub selected: bool,
    pub camera_following: bool
}

impl Selectable {
    pub fn new(selected: bool) -> Self {
        Self {
            selected,
            camera_following: false
        }
    }
}

#[derive(Component)]
pub struct CreationTime(pub f32);

impl CreationTime {
    pub fn from_age(age: u16) -> Self {
        let days = age as f32 * 360.0;
        let seconds = days * 24.0 * 60.0 * 60.0;
        CreationTime(-seconds)
    }
}

#[derive(Component, NewtypeProxy)]
pub struct Parent(pub Entity);

#[derive(Deserialize, Serialize, Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Component)]
pub enum Occupation {
    Worker,
    Pilot,
    Engineer,
    Marine,
    Government
}


#[derive(Deserialize, Serialize, Component)]
pub struct ObjectSpin {
    initial_rotation: Quaternion<f32>,
    rotation_axis: Vector3<f32>,
    rotation: f32,
    rotation_speed: f32
}

impl ObjectSpin {
    pub fn random(rng: &mut ThreadRng) -> Self {
        let initial = uniform_sphere_distribution(rng);

        use cgmath::Rotation;

        Self {
            initial_rotation: Quaternion::between_vectors(UP, initial),
            rotation_axis: uniform_sphere_distribution(rng),
            rotation: 0.0,
            rotation_speed: 0.1
        }
    }

    pub fn turn(&mut self, secs: f32) {
        self.rotation += self.rotation_speed * secs;
    }

    pub fn to_quat(&self) -> Quaternion<f32> {
        self.initial_rotation * Quaternion::from_axis_angle(self.rotation_axis, Rad(self.rotation))
    }
}

#[derive(Component, PartialEq)]
pub enum Side {
    Friendly,
    Neutral,
    Enemy
}

impl Side {
    pub fn color(&self) -> [f32; 4] {
        match *self {
            Side::Friendly => [0.0, 0.8, 0.0, 1.0],
            Side::Neutral => [1.0, 0.8, 0.2, 1.0],
            Side::Enemy => [0.9, 0.0, 0.0, 1.0]
        }
    }
}

#[derive(Component, NewtypeProxy, Default)]
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

pub struct LogItem {
    age: f32,
    content: String
}

#[derive(Component)]
pub struct DrillSpeed(pub f32);

#[derive(Component, Default)]
pub struct MovementPlane(pub f32);

#[derive(Component)]
pub struct TimeLeft(pub f32);

#[derive(Component)]
pub struct Velocity(pub Vector3<f32>);

#[derive(Component)]
pub struct MaxSpeed(pub f32);

#[derive(Component)]
pub enum SeekPosition {
    Point(Vector3<f32>),
    WithinDistance(Vector3<f32>, f32)
}

impl SeekPosition {
    pub fn target_point(&self, from: Vector3<f32>) -> Vector3<f32> {
        match *self {
            SeekPosition::Point(point) => point,
            SeekPosition::WithinDistance(point, distance) => point + (from - point).normalize_to(distance)
        }
    }

    pub fn delta(&self, point: Vector3<f32>) -> Vector3<f32> {
        self.target_point(point) - point
    }

    pub fn close_enough(&self, point: Vector3<f32>) -> bool {
        close_enough(self.target_point(point), point)
    }
}

#[derive(Component)]
pub struct SeekForce(pub Vector3<f32>);

#[derive(Component)]
pub struct AvoidanceForce(pub Vector3<f32>);

#[derive(Component)]
pub struct FrictionForce(pub Vector3<f32>);
