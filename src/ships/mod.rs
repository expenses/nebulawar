use cgmath::*;
use camera::*;
use *;
use context::*;

mod components;
mod storage;
mod formations;

use self::components::*;
use self::storage::*;
pub use self::formations::*;

pub type Ships = AutoIDMap<ShipID, Ship>;

#[derive(Deserialize, Serialize, Clone, Copy)]
pub enum Interaction {
    Follow,
    Refuel,
    RefuelFrom
}

impl Interaction {
    pub fn image(&self) -> Image {
        match *self {
            Interaction::Follow => Image::Move,
            Interaction::RefuelFrom => Image::RefuelFrom,
            Interaction::Refuel => Image::Refuel
        }
    }
}

#[derive(PartialEq)]
enum MovementStatus {
    Moving, 
    Reached,
    OutOfFuel
}

#[derive(Deserialize, Serialize, Clone)]
pub enum Command {
    MoveTo(Vector3<f32>),
    GoToAnd(ShipID, Interaction)
}

impl Command {
    fn point(&self, ships: &Ships) -> Vector3<f32> {
        match *self {
            Command::MoveTo(point) => point,
            Command::GoToAnd(id, _) => ships[id].position()
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub enum ShipType {
    Fighter,
    Tanker,
    Carrier
}

impl ShipType {
    fn model(&self) -> Model {
        match *self {
            ShipType::Fighter => Model::Fighter,
            ShipType::Tanker => Model::Tanker,
            ShipType::Carrier => Model::Carrier
        }
    }

    fn crew_capacity(&self) -> usize {
        match *self {
            ShipType::Fighter => 1,
            ShipType::Tanker => 10,
            ShipType::Carrier => 100
        }
    }

    fn default_components(&self, age: u8) -> Vec<Component> {
        match *self {
            ShipType::Fighter => vec![
                Component::new(ComponentType::AX2900Drive, age),
                Component::new(ComponentType::Boltor89Cannons, age)
            ],
            ShipType::Tanker => vec![
                Component::new(ComponentType::HG900Drive, age),
                Component::new(ComponentType::HG43WarpDrive, age),
                Component::new(ComponentType::FuelDrum, age)
            ],
            ShipType::Carrier => vec![
                Component::new(ComponentType::AX17KXDrive, age),
                Component::new(ComponentType::AX17KXDrive, age)
            ]
        }
    } 

    fn mass(&self) -> f32 {
        match *self {
            ShipType::Fighter => 2.0,
            ShipType::Tanker => 100.0,
            ShipType::Carrier => 2000.0
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct Ship {
    id: ShipID,
    tag: ShipType,
    position: Vector3<f32>,
    angle: Quaternion<f32>,
    pub commands: Vec<Command>,
    components: Vec<Component>,
    fuel: Storage,
    materials: Storage,
    food: Storage
}

impl Ship {
    pub fn new(tag: ShipType, position: Vector3<f32>, angle: (f32, f32, f32)) -> Self {
        let (pitch, yaw, roll) = angle;

        let mut ship = Self {
            id: ShipID::default(),
            position,
            angle: Euler::new(Rad(pitch), Rad(yaw), Rad(roll)).into(),
            commands: Vec::new(),
            components: tag.default_components(0),
            tag,
            fuel: Storage::empty(),
            food: Storage::empty(),
            materials: Storage::empty()
        };

        ship.fuel = Storage::new(ship.max_fuel());

        ship
    }

    pub fn position(&self) -> Vector3<f32> {
        self.position
    }

    pub fn tag(&self) -> &ShipType {
        &self.tag
    }

    pub fn id(&self) -> ShipID {
        self.id
    }

    pub fn out_of_fuel(&self) -> bool {
        self.fuel.is_empty()
    }

    pub fn fuel_perc(&self) -> f32 {
        self.fuel.amount() / self.max_fuel()
    }

    fn component_types(&self) -> impl Iterator<Item=ComponentType> + '_ {
        self.components.iter().map(Component::tag)
    }

    fn max_fuel(&self) -> f32 {
        self.component_types().map(ComponentType::fuel_storage).sum()
    }

    fn thrust(&self) -> f32 {
        self.component_types().map(ComponentType::thrust).sum() 
    }

    fn speed(&self) -> f32 {
        self.thrust() / self.tag.mass()
    }

    fn move_towards(&mut self, point: Vector3<f32>) -> MovementStatus {
        if self.fuel.is_empty() {
            return MovementStatus::OutOfFuel;
        }
        
        self.fuel.reduce(0.01);

        self.position = move_towards(self.position, point, self.speed());

        if self.position == point {
            MovementStatus::Reached
        } else {
            self.angle = look_at(point - self.position);
            MovementStatus::Moving
        }
    }

    pub fn step<'a>(&'a mut self, ships: &'a mut LimitedAccessMapView<'a, ShipID, Ship>) {
        let mut next = false;

        if let Some(command) = self.commands.first().cloned() {
            match command {
                Command::MoveTo(position) => {
                    if self.move_towards(position) == MovementStatus::Reached {
                        next = true;
                    }
                },
                Command::GoToAnd(ship, interaction) => {
                    let target = ships.get_mut(ship).unwrap();

                    if self.position.distance(target.position) < 5.0 {
                        match interaction {
                            Interaction::Follow => {},
                            Interaction::Refuel => {
                                let target_max = target.max_fuel();

                                if self.fuel.transfer_to(&mut target.fuel, 0.1, target_max) == 0.0 {
                                    next = true;
                                }
                            },
                            Interaction::RefuelFrom => {
                                let max = self.max_fuel();

                                if target.fuel.transfer_to(&mut self.fuel, 0.1, max) == 0.0 {
                                    next = true;
                                }
                            }
                        }
                    } else {
                        if self.move_towards(target.position) == MovementStatus::OutOfFuel {
                            next = true;
                        }
                    }
                }
            }
        }

        if next {
            self.commands.remove(0);
        }
    }

    pub fn render(&self, context: &mut context::Context, camera: &Camera, system: &System) {
        context.render_model(self.tag.model(), self.position, self.angle, 1.0, camera, system);
    }

    pub fn command_path<'a>(&'a self, ships: &'a Ships) -> impl Iterator<Item=Vector3<f32>> + 'a {
        iter_owned([self.position()])
            .chain(self.commands.iter().map(move |command| command.point(ships)))
    }
}

impl IDed<ShipID> for Ship {
    fn set_id(&mut self, id: ShipID) {
        self.id = id;
    }

    fn get_id(&self) -> ShipID {
        self.id
    }
}

#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug, Default, Deserialize, Serialize)]
pub struct ShipID(u32);

impl ID for ShipID {
    fn increment(&mut self) {
        *self = ShipID(self.0 + 1)
    }
}