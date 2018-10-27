use specs::{Component, DenseVecStorage};

#[derive(Debug, Clone, Serialize, Deserialize, Copy)]
pub enum ShipComponentType {
    AX2900Drive,
    HG900Drive,
    HG43WarpDrive,
    Boltor89Cannons,
    AX17KXDrive,
    FoodRecycler,
    MiningDrill
}

impl ShipComponentType {
    pub fn thrust(self) -> f32 {
        match self {
            ShipComponentType::AX2900Drive => 1.0,
            ShipComponentType::HG900Drive => 5.0,
            ShipComponentType::AX17KXDrive => 100.0,
            _ => 0.0
        }
    }

    pub fn can_warp(self) -> bool {
        match self {
            ShipComponentType::HG43WarpDrive => true,
            ShipComponentType::AX17KXDrive => true,
            _ => false
        }
    }

    pub fn drill_speed(self) -> f32 {
        match self {
            ShipComponentType::MiningDrill => 0.01,
            _ => 0.0
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShipComponent {
    age: u8,
    tag: ShipComponentType
}

impl ShipComponent {
    pub fn new(tag: ShipComponentType, age: u8) -> Self {
        Self {
            tag, age
        }
    }

    pub fn tag(&self) -> ShipComponentType {
        self.tag
    }
}

#[derive(Debug, Serialize, Deserialize, Component)]
pub struct Components {
    inner: Vec<ShipComponent>
}

impl Components {
    pub fn new(inner: Vec<ShipComponent>) -> Self {
        Self {
            inner
        }
    }

    fn component_types(&self) -> impl Iterator<Item=ShipComponentType> + '_ {
        self.inner.iter().map(ShipComponent::tag)
    }

    pub fn thrust(&self) -> f32 {
        self.component_types().map(ShipComponentType::thrust).sum() 
    }

    pub fn drill_speed(&self) -> Option<f32> {
        let speed = self.component_types().map(ShipComponentType::drill_speed).sum();
        Some(speed).filter(|speed| *speed > 0.0)
    }
}