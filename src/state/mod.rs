use cgmath::*;
use super::*;
use specs::{DenseVecStorage, World, Component};
use context;
use util::*;
use entities::*;

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
pub struct System {
    pub location: Vector2<f32>,
    pub stars: Vec<context::Vertex>,
    pub light: Vector3<f32>,
    pub background_color: (f32, f32, f32),
}

impl System {
    pub fn new(location: Vector2<f32>, rng: &mut ThreadRng, world: &mut World) -> Self {
        // todo: more random generation
        let _distance_from_center = location.magnitude();

        let stars = 10000;

        let stars = (0 .. stars)
            .map(|_| {
                context::Vertex {
                    position: (uniform_sphere_distribution(rng) * (BACKGROUND_DISTANCE + 100.0)).into(),
                    normal: [0.0; 3],
                    texture: [rng.gen_range(0.0_f32, 1.0); 2]
                }
            })
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

impl Default for System {
    fn default() -> Self {
        Self {
            location: Vector2::zero(),
            stars: Vec::new(),
            light: Vector3::zero(),
            background_color: (0.0, 0.0, 0.0)
        }
    }
}