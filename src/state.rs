use cgmath::*;
use camera::*;
use context::{self, *};
use ships::*;
use people::*;
use maps::*;
use rand::*;
use rand::distributions::*;
use {average_position, circle_size};
use bincode;

use std::collections::*;

// http://corysimon.github.io/articles/uniformdistn-on-sphere/
fn uniform_sphere_distribution(rng: &mut ThreadRng) -> Vector3<f32> {
    use std::f64::consts::PI;

    let uniform = Uniform::new(0.0, 1.0);

    let x = uniform.sample(rng);
    let y = uniform.sample(rng);

    let theta = 2.0 * PI * x;
    let phi = (1.0 - 2.0 * y).acos();

    Vector3::new(
        (phi.sin() * theta.cos()) as f32,
        (phi.sin() * theta.sin()) as f32,
        phi.cos() as f32
    )
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
            initial_rotation: Euler::new(Rad(initial.x), Rad(initial.y), Rad(initial.z)).into(),
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

    pub fn step(&mut self) {
        self.spin.turn(0.002);
    }

    pub fn render(&self, context: &mut Context, system: &System, camera: &Camera) {
        context.render_model(Model::Asteroid, self.location, self.spin.to_quat(), self.size, camera, system);
    }
}

#[derive(Deserialize, Serialize)]
pub struct System {
    pub location: Vector2<f32>,
    pub stars: Vec<(Vector3<f32>, f32)>,
    pub light: Vector3<f32>,
    pub background_color: (f32, f32, f32),
    pub asteroids: Vec<Asteroid>
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
            asteroids: (0 .. rng.gen_range(5, 10)).map(|_| Asteroid::new(rng)).collect()
        }
    }

    pub fn step(&mut self) {
        for asteroid in &mut self.asteroids {
            asteroid.step();
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct State {
    pub ships: AutoIDMap<ShipID, Ship>,
    pub people: AutoIDMap<PersonID, Person>,
    pub system: System,
    pub camera: Camera,
    pub selected: HashSet<ShipID>,
    pub paused: bool
}

impl State {
    pub fn new(rng: &mut ThreadRng) -> Self {
        let mut state = Self {
            ships: AutoIDMap::new(),
            people: AutoIDMap::new(),
            system: System::new(Vector2::new(rng.gen_range(-1.0, 1.0), rng.gen_range(-1.0, 1.0)), rng),
            camera: Camera::default(),
            selected: HashSet::new(),
            paused: false
        };

        let tanker = state.ships.push(Ship::new(ShipType::Tanker, Vector3::new(0.0, 0.0, 0.0), (0.0, 0.0, 0.0)));

        for _ in 0 .. 10 {
            state.people.push(Person::new(Occupation::Worker, tanker));
        }
        
        for i in 0 .. 100 {
            let x = (50.0 - i as f32) * 3.0;
            let fighter = state.ships.push(Ship::new(ShipType::Fighter, Vector3::new(x, 5.0, 0.0), (0.0, 0.0, 0.0)));
            state.people.push(Person::new(Occupation::Pilot, fighter));
        }

        state
    }

    pub fn load(filename: &str) -> Self {
        let file = ::std::fs::File::open(filename).unwrap();
        bincode::deserialize_from(file).unwrap()
    }

    pub fn save(&self, filename: &str) {
        let file = ::std::fs::File::create(filename).unwrap();
        bincode::serialize_into(file, self).unwrap();
    }

    pub fn selected(&self) -> impl Iterator<Item=&Ship> {
        self.selected.iter().map(move |id| &self.ships[*id])
    }

    pub fn step(&mut self, secs: f32) {
        if !self.paused {
            self.system.step();

            for ship in self.ships.iter_mut() {
                ship.step();
            }
        }

        self.camera.step(&self.ships);
    }

    pub fn render(&self, context: &mut context::Context) {
        for ship in self.ships.iter() {
            ship.render(context, &self.camera, &self.system);
        }

        for ship in self.selected() {
            if let Some((x, y, z)) = context.screen_position(ship.position(), &self.camera) {
                let fuel = ship.fuel_perc();
                context.render_circle(x, y, circle_size(z), [1.0, fuel, fuel, 1.0]);
            }

            if !ship.commands.is_empty() {
                context.render_3d_lines(ship.command_path(&self.ships));
            }
        }

        context.render_system(&self.system, &self.camera);
    }

    pub fn selection_info(&self) -> BTreeMap<&ShipType, usize> {
        self.selected().fold(BTreeMap::new(), |mut map, ship| {
            *map.entry(ship.tag()).or_insert(0) += 1;
            map
        })
    }

    pub fn average_position(&self) -> Option<Vector3<f32>> {
        average_position(&self.selected, &self.ships)
    }
}