#[derive(Debug, Clone, Serialize, Deserialize, Copy)]
pub enum ComponentType {
    AX2900Drive,
    HG900Drive,
    HG43WarpDrive,
    FuelDrum,
    Boltor89Cannons
}

impl ComponentType {
    pub fn thrust(self) -> f32 {
        match self {
            ComponentType::AX2900Drive => 1.0,
            ComponentType::HG900Drive => 5.0,
            _ => 0.0
        }
    }

    pub fn can_warp(self) -> bool {
        match self {
            ComponentType::HG43WarpDrive => true,
            _ => false
        }
    }

    pub fn fuel_storage(self) -> f32 {
        match self {
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
    pub fn new(tag: ComponentType, age: u8) -> Self {
        Self {
            tag, age
        }
    }

    pub fn tag(&self) -> ComponentType {
        self.tag
    }
}
