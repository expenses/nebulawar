use cgmath::*;
use camera::*;
use context;
use ships::*;
use people::*;
use maps::*;
use rand::{Rng, ThreadRng};
use rand::distributions::*;
use {average_position, circle_size};
use util::*;
use bincode;
use failure::{self, ResultExt};
use specs::World;

use std::collections::*;

mod system;

pub use self::system::*;

pub type Ships = AutoIDMap<ShipID, Ship>;
pub type People = AutoIDMap<PersonID, Person>;

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

#[derive(Serialize, Deserialize)]
pub struct State {
    pub ships: AutoIDMap<ShipID, Ship>,
    pub people: AutoIDMap<PersonID, Person>,
    pub system: System,
    pub camera: Camera,
    pub selected: HashSet<ShipID>,
    pub formation: Formation,
    time: f32,
    pub paused: bool
}

impl State {
    pub fn new(world: &mut World, rng: &mut ThreadRng) -> Self {
        let mut state = Self {
            ships: AutoIDMap::new(),
            people: AutoIDMap::new(),
            system: System::new(Vector2::new(rng.gen_range(-1.0, 1.0), rng.gen_range(-1.0, 1.0)), rng, world),
            camera: Camera::default(),
            selected: HashSet::new(),
            formation: Formation::DeltaWing,
            time: 0.0,
            paused: false
        };

        let carrier = state.ships.push(Ship::new(ShipType::Carrier, Vector3::new(0.0, 0.0, 10.0), (0.0, 0.0, 0.0)));

        for _ in 0 .. 45 {
            state.people.push(Person::new(Occupation::Worker, carrier));
        }

        for _ in 0 .. 20 {
            state.people.push(Person::new(Occupation::Marine, carrier));
        }

        for _ in 0 .. 25 {
            state.people.push(Person::new(Occupation::Pilot, carrier));
        }

        for _ in 0 .. 10 {
            state.people.push(Person::new(Occupation::Government, carrier));
        }

        let tanker = state.ships.push(Ship::new(ShipType::Tanker, Vector3::new(0.0, 0.0, 0.0), (0.0, 0.0, 0.0)));

        for _ in 0 .. 10 {
            state.people.push(Person::new(Occupation::Worker, tanker));
        }
        
        for i in 0 .. 20 {
            let x = (50.0 - i as f32) * 3.0;
            let fighter = state.ships.push(Ship::new(ShipType::Fighter, Vector3::new(x, 5.0, 0.0), (0.0, 0.0, 0.0)));
            state.people.push(Person::new(Occupation::Pilot, fighter));
        }

        state
    }

    pub fn time(&self) -> f32 {
        self.time
    }

    pub fn load(&mut self, filename: &str) -> Result<(), failure::Context<String>> {
        let file = ::std::fs::File::open(filename).context(format!("Failed to load '{}'.", filename))?;
        *self = bincode::deserialize_from(file).context(format!("Failed to load '{}'.", filename))?;

        info!("Loaded game from '{}'.", filename);

        Ok(())
    }

    pub fn save(&self, filename: &str) -> Result<(), failure::Context<String>> {
        let file = ::std::fs::File::create(filename).context(format!("Failed to save to '{}'.", filename))?;
        bincode::serialize_into(file, self).context(format!("Failed to save to '{}'.", filename))?;

        info!("Saved game to '{}'", filename);

        Ok(())
    }

    pub fn selected(&self) -> impl Iterator<Item=&Ship> {
        self.selected.iter().filter_map(move |id| self.ships.get(*id))
    }

    pub fn people_on_ship(&self, ship: ShipID) -> impl Iterator<Item=&Person> {
        self.people.iter().filter(move |person| person.ship() == ship)
    }

    pub fn step(&mut self, secs: f32) {
        if !self.paused {
            self.time += secs;

            let ids: Vec<_> = self.ships.ids().cloned().collect();

            for id in ids {
                let (ship, mut ships) = self.ships.split_one_off(id).unwrap();
                ship.step(secs, &mut ships, &self.people);
            }
        }

        self.camera.step(&self.ships);
    }

    pub fn render(&self, context: &mut context::Context) {
        for ship in self.selected() {
            if let Some((x, y, z)) = context.screen_position(ship.position(), &self.camera) {
                let fuel = ship.fuel_perc();
                context.render_circle(x, y, circle_size(z), [1.0, fuel, fuel, 1.0]);
            }

            if !ship.commands.is_empty() {
                context.render_3d_lines(ship.command_path(&self.ships));
            }
        }

        for ship in self.ships.iter() {
            ship.render(context, &self.camera, &self.system);
        }

        context.render_system(&self.system, &self.camera);
    }

    pub fn selection_info(&self) -> BTreeMap<&ShipType, u64> {
        summarize(self.selected().map(Ship::tag)).0
    }

    pub fn average_position(&self) -> Option<Vector3<f32>> {
        average_position(&self.selected, &self.ships)
    }
}