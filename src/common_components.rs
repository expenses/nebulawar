use specs::*;
use cgmath::*;
use util::*;
use rand::*;
use ships::*;

#[derive(Component, Default)]
pub struct Drag(pub Option<(f32, f32, f32, f32)>);

#[derive(Component, Default, NewtypeProxy)]
pub struct Secs(pub f32);

#[derive(Component, Default)]
pub struct Time(pub f32);

#[derive(Component, Default)]
pub struct ShiftPressed(pub bool);

impl ShiftPressed {
    pub fn pressed(&self) -> bool {
        self.0
    }
}

#[derive(Component, Default)]
pub struct LeftClick(pub Option<(f32, f32)>);

#[derive(Component, Default)]
pub struct RightClick(pub Option<(f32, f32)>);

#[derive(Component, Default)]
pub struct Mouse(pub (f32, f32));

#[derive(Component, Default)]
pub struct Paused(pub bool);

impl Paused {
    pub fn switch(&mut self) {
        self.0 = !self.0;
    }
}

#[derive(Component, Default)]
pub struct EntityUnderMouse(pub Option<(Entity, Vector3<f32>)>);

#[derive(Component, Default)]
pub struct RightClickInteraction(pub Option<(Entity, Interaction)>);

#[derive(Deserialize, Serialize, Component, NewtypeProxy)]
pub struct Position(pub Vector3<f32>);

#[derive(Deserialize, Serialize, Component)]
pub struct MineableMaterials(pub u32);

#[derive(Deserialize, Serialize, Component, NewtypeProxy)]
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

#[derive(Component, NewtypeProxy)]
pub struct Fuel(pub StoredResource);