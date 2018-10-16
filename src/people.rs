use ships::*;
use maps::*;

#[derive(Deserialize, Serialize)]
pub enum Occupation {
    Worker,
    Pilot,
    Engineer,
    Marine,
    Government
}

#[derive(Deserialize, Serialize)]
pub struct Person {
    id: PersonID,
    occupation: Occupation,
    age: u16,
    ship: ShipID
}

impl Person {
    pub fn new(occupation: Occupation, ship: ShipID) -> Self {
        Self {
            occupation, ship,
            age: 30,
            id: PersonID::default()
        }
    }
}

impl IDed<PersonID> for Person {
    fn set_id(&mut self, id: PersonID) {
        self.id = id;
    }

    fn get_id(&self) -> PersonID {
        self.id
    }
}

#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug, Default, Deserialize, Serialize)]
pub struct PersonID(u32);

impl ID for PersonID {
    fn increment(&mut self) {
        *self = PersonID(self.0 + 1)
    }
}