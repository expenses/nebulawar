use cgmath::*;
use camera::*;
use *;
use context::*;

pub enum Formation {
    Screen,
    DeltaWing
}

impl Formation {
    pub fn arrange(&self, ships: usize, position: Vector3<f32>, target: Vector3<f32>, distance: f32) -> Vec<Vector3<f32>> {
        let mut step = target - position;
        step.y = 0.0;
        let step = step.normalize_to(distance);

        let step_sideways = Quaternion::from_angle_y(Rad(FRAC_PI_2)).rotate_vector(step);

        match *self {
            Formation::Screen => {                
                let step_up = UP * distance;

                let width = (ships as f32).sqrt().ceil() as usize;

                let middle_x = (width - 1) as f32 / 2.0;

                let middle_y = (ships as f32 / width as f32).floor() / 2.0;

                (0 .. ships)
                    .map(|i| {
                        let x = (i % width) as f32 - middle_x;
                        let y = (i / width) as f32 - middle_y;

                        target + step_sideways * x + step_up * y
                    })
                    .collect()
            },
            Formation::DeltaWing => {
                let middle_x = (ships - 1) as f32 / 2.0;

                (0 .. ships)
                    .map(|i| {
                        let x = i as f32 - middle_x;

                        let y = -(i as f32 - middle_x).abs();

                        target + step * y + step_sideways * x
                    })
                    .collect()
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComponentType {
    AX2900Drive,
    HG900Drive,
    HG43WarpDrive,
    FuelDrum,
    Boltor89Cannons
}

impl ComponentType {
    fn thrust(&self) -> f32 {
        match *self {
            ComponentType::AX2900Drive => 1.0,
            ComponentType::HG900Drive => 5.0,
            _ => 0.0
        }
    }

    fn can_warp(&self) -> bool {
        match *self {
            ComponentType::HG43WarpDrive => true,
            _ => false
        }
    }

    fn fuel_storage(&self) -> f32 {
        match *self {
            ComponentType::AX2900Drive => 20.0,
            ComponentType::HG900Drive => 100.0,
            ComponentType::FuelDrum => 2000.0,
            _ => 0.0
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Component {
    age: u8,
    tag: ComponentType
}

impl Component {
    fn new(tag: ComponentType, age: u8) -> Self {
        Self {
            tag, age
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Copy)]
pub enum Interaction {
    Follow,
    Refuel,
    RefuelFrom
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
    Tanker
}

impl ShipType {
    fn model(&self) -> Model {
        match *self {
            ShipType::Fighter => Model::Fighter,
            ShipType::Tanker => Model::Tanker
        }
    }

    fn crew_capacity(&self) -> usize {
        match *self {
            ShipType::Fighter => 1,
            ShipType::Tanker => 10
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
            ]
        }
    } 

    fn mass(&self) -> f32 {
        match *self {
            ShipType::Fighter => 2.0,
            ShipType::Tanker => 100.0
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

    fn component_types(&self) -> impl Iterator<Item=&ComponentType> {
        self.components.iter().map(|component| &component.tag)
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

    fn move_towards(&mut self, point: Vector3<f32>) -> bool {
        if self.fuel.is_empty() {
            return self.position == point;
        }
        
        self.fuel.reduce(0.01);

        self.position = move_towards(self.position, point, self.speed());

        if self.position == point {
            true
        } else {
            self.angle = look_at(point - self.position);
            false
        }
    }

    pub fn step<'a>(&'a mut self, ships: &'a mut LimitedAccessMapView<'a, ShipID, Ship>) {
        let mut next = false;

        if let Some(command) = self.commands.first().cloned() {
            match command {
                Command::MoveTo(position) => {
                    if self.move_towards(position) {
                        next = true;
                    }
                },
                Command::GoToAnd(ship, interaction) => {
                    let target = ships.get_mut(ship).unwrap();

                    if self.position.distance(target.position) < 5.0 || self.move_towards(target.position) {
                        next = true;
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

pub type Ships = AutoIDMap<ShipID, Ship>;


#[derive(Serialize, Deserialize)]
struct Storage {
    amount: f32
}

impl Storage {
    fn empty() -> Self {
        Self {
            amount: 0.0
        }
    }

    fn new(amount: f32) -> Self {
        Self {
            amount
        }
    }

    fn reduce(&mut self, amount: f32) -> f32 {
        let reduced_by = self.amount.min(amount);
        self.amount -= reduced_by;
        reduced_by
    } 

    fn increase(&mut self, amount: f32, limit: f32) -> f32 {
        let increased_by = (limit - self.amount).min(amount);
        self.amount += increased_by;
        increased_by
    }

    fn is_empty(&self) -> bool {
        self.amount == 0.0
    }

    fn amount(&self) -> f32 {
        self.amount
    }
}
