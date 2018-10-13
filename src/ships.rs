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

        match *self {
            Formation::Screen => {                
                let step_sideways = Quaternion::from_angle_y(Rad(FRAC_PI_2)).rotate_vector(step);
                let step_up = Vector3::new(0.0, distance, 0.0);

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
                let step_sideways = Quaternion::from_angle_y(Rad(FRAC_PI_2)).rotate_vector(step);

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

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum ComponentType {
    AX2900Drive,
    HG900Drive,
    HG43WarpDrive,
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

#[derive(Deserialize, Serialize)]
pub enum Command {
    MoveTo(Vector3<f32>)
}

#[derive(Debug, Serialize, Deserialize)]
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
                Component::new(ComponentType::HG43WarpDrive, age)
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
    pub position: Vector3<f32>,
    angle: Quaternion<f32>,
    pub commands: Vec<Command>,
    components: Vec<Component>
}

impl Ship {
    pub fn new(tag: ShipType, position: Vector3<f32>, angle: (f32, f32, f32)) -> Self {
        let (pitch, yaw, roll) = angle;

        Self {
            id: ShipID::default(),
            position,
            angle: Euler::new(Rad(pitch), Rad(yaw), Rad(roll)).into(),
            commands: Vec::new(),
            components: tag.default_components(0),
            tag,
        }
    }

    pub fn id(&self) -> ShipID {
        self.id
    }

    fn speed(&self) -> f32 {
        self.components.iter().map(|component| component.tag.thrust()).sum::<f32>() / self.tag.mass()
    }

    pub fn position_matrix(&self) -> Matrix4<f32> {
        let angle: Matrix4<f32> = self.angle.into();
        Matrix4::from_translation(self.position) * angle
    }

    pub fn step(&mut self) {
        let mut clear = false;
        if let Some(Command::MoveTo(position)) = self.commands.first() {
            let delta = position - self.position;

            if self.speed() < self.position.distance(*position) {
                let step = delta.normalize_to(self.speed());

                self.position += step;
            } else {
                self.position = *position;
                clear = true;
            }

            self.angle = Quaternion::look_at(delta, Vector3::new(0.0, 1.0, 0.0)).invert();;
        }

        if clear {
            self.commands.remove(0);
        }
    }

    pub fn render(&self, context: &mut context::Context, camera: &Camera, system: &System) {
        context.render(self.tag.model(), self.position_matrix(), camera, system, Mode::Normal);
    }

    pub fn info(&self) -> String {
        format!("{:?} {:?} - Components: {:?}", self.tag, self.id, self.components.iter().map(|c| c.tag).collect::<Vec<_>>())
    }
}

impl IDed<ShipID> for Ship {
    fn set_id(&mut self, id: ShipID) {
        self.id = id;
    }
}

#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug, Default, Deserialize, Serialize)]
pub struct ShipID(u32);

impl ID for ShipID {
    fn increment(&mut self) {
        *self = ShipID(self.0 + 1)
    }
}