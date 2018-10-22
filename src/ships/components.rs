use specs::{Component, DenseVecStorage};

#[derive(Copy, Clone)]
pub enum Resource {
    Fuel,
    Waste,
    Food,
    Materials
}

pub struct Converter {
    pub from: Resource,
    pub to: Resource,
    pub speed: f32
}

impl Converter {
    pub fn new(from: Resource, to: Resource, speed: f32) -> Self {
        Self {
            from, to, speed
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Copy)]
pub enum ShipComponentType {
    AX2900Drive,
    HG900Drive,
    HG43WarpDrive,
    FuelDrum,
    Boltor89Cannons,
    AX17KXDrive,
    FoodRecycler
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

    pub fn fuel_capacity(self) -> f32 {
        match self {
            ShipComponentType::AX2900Drive => 20.0,
            ShipComponentType::HG900Drive => 100.0,
            ShipComponentType::FuelDrum => 2000.0,
            ShipComponentType::AX17KXDrive => 200.0,
            _ => 0.0
        }
    }

    pub fn converter(self) -> Option<Converter> {
        match self {
            ShipComponentType::FoodRecycler => Some(Converter::new(Resource::Waste, Resource::Food, 1.0)),
            _ => None
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

    pub fn fuel_capacity(&self) -> f32 {
        self.component_types().map(ShipComponentType::fuel_capacity).sum()
    }

    pub fn thrust(&self) -> f32 {
        self.component_types().map(ShipComponentType::thrust).sum() 
    }

    pub fn converters(&self) -> impl Iterator<Item=Converter> + '_ {
        self.component_types().filter_map(ShipComponentType::converter)
    }
}

// todo: move ui stuff to systems