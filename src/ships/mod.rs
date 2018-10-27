use cgmath::Vector3;
use context::*;
use specs::{DenseVecStorage, Component, Entity, ReadStorage};
use components::*;

pub mod components;
mod storage;
mod formations;

pub use self::components::*;
pub use self::storage::*;
pub use self::formations::*;

#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq)]
pub enum Interaction {
    Follow,
    Mine,
    Attack
}

impl Interaction {
    pub fn image(&self) -> Image {
        match *self {
            Interaction::Follow => Image::Move,
            Interaction::Mine => Image::Mine,
            Interaction::Attack => Image::Mine
        }
    }
}

#[derive(Debug)]
pub enum Command {
    MoveTo(Vector3<f32>),
    GoToAnd(Entity, Interaction)
}

impl Command {
    pub fn point(&self, positions: &ReadStorage<Position>) -> Vector3<f32> {
        match *self {
            Command::MoveTo(point) => point,
            Command::GoToAnd(entity, _) => positions.get(entity).unwrap().0,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, PartialOrd, Ord, Component)]
pub enum ShipType {
    Fighter,
    Tanker,
    Carrier,
    Miner
}

impl ShipType {
    pub fn model(&self) -> Model {
        match *self {
            ShipType::Fighter => Model::Fighter,
            ShipType::Tanker => Model::Tanker,
            ShipType::Carrier => Model::Carrier,
            ShipType::Miner => Model::Miner
        }
    }

    fn crew_capacity(&self) -> usize {
        match *self {
            ShipType::Fighter => 1,
            ShipType::Tanker => 10,
            ShipType::Carrier => 100,
            ShipType::Miner => 5
        }
    }

    pub fn default_components(&self, age: u8) -> Components {
        Components::new(
            match *self {
                ShipType::Fighter => vec![
                    ShipComponent::new(ShipComponentType::AX2900Drive, age),
                    ShipComponent::new(ShipComponentType::Boltor89Cannons, age)
                ],
                ShipType::Tanker => vec![
                    ShipComponent::new(ShipComponentType::HG900Drive, age),
                    ShipComponent::new(ShipComponentType::HG43WarpDrive, age)
                ],
                ShipType::Carrier => vec![
                    ShipComponent::new(ShipComponentType::AX17KXDrive, age),
                    ShipComponent::new(ShipComponentType::AX17KXDrive, age),
                    ShipComponent::new(ShipComponentType::FoodRecycler, age)
                ],
                ShipType::Miner => vec![
                    ShipComponent::new(ShipComponentType::HG900Drive, age),
                    ShipComponent::new(ShipComponentType::HG43WarpDrive, age),
                    ShipComponent::new(ShipComponentType::MiningDrill, age),
                ]
            }
        )
    } 

    pub fn mass(&self) -> f32 {
        match *self {
            ShipType::Fighter => 2.0,
            ShipType::Tanker => 100.0,
            ShipType::Carrier => 2000.0,
            ShipType::Miner => 20.0
        }
    }

    pub fn size(&self) -> f32 {
        match *self {
            ShipType::Fighter => 1.0,
            ShipType::Tanker => 2.0,
            ShipType::Carrier => 4.0,
            ShipType::Miner => 2.0
        }
    }
}

#[derive(Component, NewtypeProxy)]
pub struct Commands(pub Vec<Command>);

impl Commands {
    pub fn order(&mut self, shift: bool, command: Command) {
        if !shift {
            self.0.clear();
        }

        self.0.push(command);
    }
}