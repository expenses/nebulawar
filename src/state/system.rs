use cgmath::*;
use super::*;
use specs::{DenseVecStorage, World, Builder};
use context::*;

#[derive(Debug)]
enum SystemType {
    Asteroids,
    Planetoid,
    Nebula,
    BlackHole
}

impl SystemType {
    fn random(rng: &mut ThreadRng) -> Self {
        let num = rng.gen_range(0, 100);

        match num {
            0 ... 29 => SystemType::Asteroids,
            30 ... 59 => SystemType::Planetoid,
            60 ... 89 => SystemType::Nebula,
            90 ... 100 => SystemType::BlackHole,
            _ => unreachable!()
        }
    }
}

#[derive(Deserialize, Serialize, Component)]
pub struct Position(pub Vector3<f32>);

#[derive(Deserialize, Serialize, Component)]
pub struct MineableMaterials(u32);

#[derive(Deserialize, Serialize, Component)]
pub struct Size(pub f32);

#[derive(Deserialize, Serialize, Component)]
pub struct ObjectSpin {
    initial_rotation: Quaternion<f32>,
    rotation_axis: Vector3<f32>,
    rotation: f32,
    rotation_speed: f32
}

impl ObjectSpin {
    fn random(rng: &mut ThreadRng) -> Self {
        let initial = uniform_sphere_distribution(rng);

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

pub fn add_asteroid(rng: &mut ThreadRng, world: &mut World) {
    let size: f32 = rng.gen_range(0.5, 5.0);

    let x = rng.gen_range(500.0, 1000.0) * rng.gen_range(-1.0, 1.0);
    let y = rng.gen_range(-100.0, 100.0);
    let z = rng.gen_range(500.0, 1000.0) * rng.gen_range(-1.0, 1.0);

    let location = Vector3::new(x, y, z);

    let resources = (size.powi(3) * rng.gen_range(0.1, 1.0)) as u32;

    let spin = ObjectSpin::random(rng);

    world.create_entity()
        .with(Model::Asteroid)
        .with(spin)
        .with(Position(location))
        .with(MineableMaterials(resources))
        .with(Size(size))
        .build();
}

#[derive(Deserialize, Serialize)]
pub struct System {
    pub location: Vector2<f32>,
    pub stars: Vec<(Vector3<f32>, f32)>,
    pub light: Vector3<f32>,
    pub background_color: (f32, f32, f32)
}

impl System {
    pub fn new(location: Vector2<f32>, rng: &mut ThreadRng, world: &mut World) -> Self {
        // todo: more random generation
        let _distance_from_center = location.magnitude();

        let stars = 10000;

        let stars = (0 .. stars)
            .map(|_| (uniform_sphere_distribution(rng), rng.gen()))
            .collect();

        let mut light = uniform_sphere_distribution(rng);
        light.y = light.y.abs();

        let system_type = SystemType::random(rng);

        info!("Generated a {:?} system at {:?}.", system_type, location);

        for _ in 0 .. rng.gen_range(5, 10) {
            add_asteroid(rng, world);
        }

        Self {
            light,
            background_color: (0.0, 0.0, rng.gen_range(0.0, 0.25)),
            stars, location
        }
    }
}
