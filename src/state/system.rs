use cgmath::*;
use super::*;

enum SystemType {
    Asteroids,
    Planetoid,
    Nebula,
    BlackHole
}

#[derive(Deserialize, Serialize)]
pub struct SystemObject {
    id: SystemObjectID,
    inner: SystemObjectInner
}

impl SystemObject {
    fn new_asteroid(asteroid: Asteroid) -> Self {
        Self {
            id: SystemObjectID::default(),
            inner: SystemObjectInner::Asteroid(asteroid)
        }
    }

    fn step(&mut self) {
        match self.inner {
            SystemObjectInner::Asteroid(ref mut a) => a.step(),
            _ => unimplemented!()
        }
    }

    pub fn render(&self, context: &mut Context, system: &System, camera: &Camera) {
        match self.inner {
            SystemObjectInner::Asteroid(ref a) => a.render(context, system, camera),
            _ => unimplemented!()
        }
    }
}

impl IDed<SystemObjectID> for SystemObject {
    fn set_id(&mut self, id: SystemObjectID) {
        self.id = id;
    }
}

#[derive(Deserialize, Serialize)]
enum SystemObjectInner {
    Asteroid(Asteroid),
    Station(Station)
}

#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug, Default, Deserialize, Serialize)]
pub struct SystemObjectID(u32);

impl ID for SystemObjectID {
    fn increment(&mut self) {
        *self = SystemObjectID(self.0 + 1)
    }
}

#[derive(Deserialize, Serialize)]
struct Station {
    location: Vector3<f32>,
    spin: ObjectSpin
}


#[derive(Deserialize, Serialize)]
struct ObjectSpin {
    initial_rotation: Quaternion<f32>,
    rotation_axis: Vector3<f32>,
    rotation: f32
}

impl ObjectSpin {
    fn random(rng: &mut ThreadRng) -> Self {
        let initial = uniform_sphere_distribution(rng);

        Self {
            initial_rotation: Quaternion::between_vectors(UP, initial),
            rotation_axis: uniform_sphere_distribution(rng),
            rotation: 0.0
        }
    }

    fn turn(&mut self, amount: f32) {
        self.rotation += amount;
    }

    fn to_quat(&self) -> Quaternion<f32> {
        self.initial_rotation * Quaternion::from_axis_angle(self.rotation_axis, Rad(self.rotation))
    }
}

#[derive(Deserialize, Serialize)]
pub struct Asteroid {
    location: Vector3<f32>,
    resources: u32,
    size: f32,
    spin: ObjectSpin
}

impl Asteroid {
    fn new(rng: &mut ThreadRng) -> Self {
        let size: f32 = rng.gen_range(0.5, 5.0);

        let x = rng.gen_range(500.0, 1000.0) * rng.gen_range(-1.0, 1.0);
        let y = rng.gen_range(-100.0, 100.0);
        let z = rng.gen_range(500.0, 1000.0) * rng.gen_range(-1.0, 1.0);
       
        Self {
            size,
            resources: (size.powi(3) * rng.gen_range(0.1, 1.0)) as u32,
            location: Vector3::new(x, y, z),
            spin: ObjectSpin::random(rng)
        }
    }

    fn step(&mut self) {
        self.spin.turn(0.002);
    }

    fn render(&self, context: &mut Context, system: &System, camera: &Camera) {
        context.render_model(Model::Asteroid, self.location, self.spin.to_quat(), self.size, camera, system);
    }
}

#[derive(Deserialize, Serialize)]
pub struct System {
    pub location: Vector2<f32>,
    pub stars: Vec<(Vector3<f32>, f32)>,
    pub light: Vector3<f32>,
    pub background_color: (f32, f32, f32),
    pub system_objects: AutoIDMap<SystemObjectID, SystemObject>
}

impl System {
    pub fn new(location: Vector2<f32>, rng: &mut ThreadRng) -> Self {
        // todo: more random generation
        let _distance_from_center = location.magnitude();

        let stars = 10000;

        let stars = (0 .. stars)
            .map(|_| (uniform_sphere_distribution(rng), rng.gen()))
            .collect();

        let mut light = uniform_sphere_distribution(rng);
        light.y = light.y.abs();

        Self {
            light,
            background_color: (0.0, 0.0, rng.gen_range(0.0, 0.25)),
            stars, location,
            system_objects: (0 .. rng.gen_range(5, 10)).map(|_| SystemObject::new_asteroid(Asteroid::new(rng))).collect()
        }
    }

    pub fn step(&mut self) {
        for object in self.system_objects.iter_mut() {
            object.step();
        }
    }
}
