use specs::{*, saveload::*, error::*};
use cgmath::*;
use util::*;
use rand::ThreadRng;
use ships::*;
use serde::*;

#[derive(Component, NewtypeProxy, Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
#[storage(VecStorage)]
pub struct Position(pub Vector3<f32>);

#[derive(ConvertSaveload, Component, NewtypeProxy)]
pub struct Materials(pub StoredResource);

#[derive(ConvertSaveload, Component, NewtypeProxy)]
pub struct MineableMaterials(pub StoredResource);

#[derive(ConvertSaveload, Component, NewtypeProxy, Clone, Copy)]
#[storage(VecStorage)]
pub struct Size(pub f32);

#[derive(Component, NewtypeProxy, ConvertSaveload, Clone)]
pub struct Rotation(pub Quaternion<f32>);

#[derive(Component, ConvertSaveload)]
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

#[derive(Component, ConvertSaveload)]
pub struct CreationTime(pub f32);

impl CreationTime {
    pub fn from_age(age: u16) -> Self {
        let days = age as f32 * 360.0;
        let seconds = days * 24.0 * 60.0 * 60.0;
        CreationTime(-seconds)
    }
}

#[derive(Component, NewtypeProxy, ConvertSaveload)]
pub struct Parent(pub Entity);

#[derive(Deserialize, Serialize, Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Component)]
pub enum Occupation {
    Worker,
    Pilot,
    Engineer,
    Marine,
    Government
}


#[derive(ConvertSaveload, Component)]
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

#[derive(Component, PartialEq, Serialize, Deserialize, Clone)]
pub enum Side {
    Friendly,
    Neutral,
    Enemy
}

impl Side {
    pub fn colour(&self) -> [f32; 4] {
        match *self {
            Side::Friendly => [0.0, 0.8, 0.0, 1.0],
            Side::Neutral => [1.0, 0.8, 0.2, 1.0],
            Side::Enemy => [0.9, 0.0, 0.0, 1.0]
        }
    }
}

#[derive(Component, ConvertSaveload)]
pub struct DrillSpeed(pub f32);

#[derive(Component, Debug, Clone, PartialEq, ConvertSaveload)]
pub struct TimeLeft(pub f32);

#[derive(ConvertSaveload, Component, NewtypeProxy, Debug, Clone, PartialEq)]
pub struct Velocity(pub Vector3<f32>);

#[derive(Component, ConvertSaveload)]
pub struct MaxSpeed(pub f32);

#[derive(Component)]
pub struct SeekPosition {
    point: Vector3<f32>,
    within_distance: Option<f32>,
    last_point: bool
}

impl SeekPosition {
    pub fn to_point(point: Vector3<f32>, last_point: bool) -> Self {
        Self {
            point, last_point,
            within_distance: None
        }
    }

    pub fn within_distance(point: Vector3<f32>, within_distance: f32, last_point: bool) -> Self {
        Self {
            point, last_point,
            within_distance: Some(within_distance)
        }
    }

    pub fn target_point(&self, from: Vector3<f32>) -> Vector3<f32> {
        if let Some(distance) = self.within_distance {
            self.point + (from - self.point).normalize_to(distance)
        } else {
            self.point
        }
    }

    pub fn delta(&self, point: Vector3<f32>) -> Vector3<f32> {
        self.target_point(point) - point
    }

    pub fn close_enough(&self, point: Vector3<f32>) -> bool {
        close_enough(self.target_point(point), point)
    }

    pub fn last_point(&self) -> bool {
        self.last_point
    }
}

#[derive(Component)]
pub struct SeekForce(pub Vector3<f32>);

#[derive(Component)]
pub struct AvoidanceForce(pub Vector3<f32>);

#[derive(Component)]
pub struct FrictionForce(pub Vector3<f32>);


#[derive(Component, NewtypeProxy)]
pub struct Commands(pub Vec<Command>);

impl Commands {
    pub fn order(&mut self, shift: bool, command: Command) {
        if !shift {
            self.clear();
        }

        self.push(command);
    }
}

impl<M: Serialize + Marker> ConvertSaveload<M> for Commands {
    type Data = Vec<<Command as ConvertSaveload<M>>::Data>;
    type Error = Error;

    fn convert_into<F: FnMut(Entity) -> Option<M>>(&self, mut ids: F) -> Result<Self::Data, Self::Error> {
        self.0.iter().map(|command| command.convert_into(&mut ids)).collect()
    }

    fn convert_from<F: FnMut(M) -> Option<Entity>>(data: Self::Data, mut ids: F) -> Result<Self, Self::Error> {
        let commands: Result<Vec<Command>, Self::Error> =  data.into_iter()
            .map(|data| Command::convert_from(data, &mut ids))
            .collect();

        commands.map(Commands)
    }
}

#[derive(Component, ConvertSaveload)]
pub struct CanAttack {
    pub time: f32,
    pub delay: f32
}


#[derive(Component, Serialize, Deserialize, Default)]
#[storage(NullStorage)]
pub struct SpawnSmoke;