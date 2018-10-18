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
pub enum ComponentType {
    AX2900Drive,
    HG900Drive,
    HG43WarpDrive,
    FuelDrum,
    Boltor89Cannons,
    AX17KXDrive,
    FoodRecycler
}

impl ComponentType {
    pub fn thrust(self) -> f32 {
        match self {
            ComponentType::AX2900Drive => 1.0,
            ComponentType::HG900Drive => 5.0,
            ComponentType::AX17KXDrive => 100.0,
            _ => 0.0
        }
    }

    pub fn can_warp(self) -> bool {
        match self {
            ComponentType::HG43WarpDrive => true,
            ComponentType::AX17KXDrive => true,
            _ => false
        }
    }

    pub fn fuel_capacity(self) -> f32 {
        match self {
            ComponentType::AX2900Drive => 20.0,
            ComponentType::HG900Drive => 100.0,
            ComponentType::FuelDrum => 2000.0,
            ComponentType::AX17KXDrive => 200.0,
            _ => 0.0
        }
    }

    pub fn converter(self) -> Option<Converter> {
        match self {
            ComponentType::FoodRecycler => Some(Converter::new(Resource::Waste, Resource::Food, 1.0)),
            _ => None
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Components {
    inner: Vec<Component>
}

impl Components {
    pub fn new(inner: Vec<Component>) -> Self {
        Self {
            inner
        }
    }

    fn component_types(&self) -> impl Iterator<Item=ComponentType> + '_ {
        self.inner.iter().map(Component::tag)
    }

    pub fn fuel_capacity(&self) -> f32 {
        self.component_types().map(ComponentType::fuel_capacity).sum()
    }

    pub fn thrust(&self) -> f32 {
        self.component_types().map(ComponentType::thrust).sum() 
    }

    pub fn converters(&self) -> impl Iterator<Item=Converter> + '_ {
        self.component_types().filter_map(ComponentType::converter)
    }
}