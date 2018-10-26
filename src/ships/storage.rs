use std::fmt;

#[derive(Serialize, Deserialize, Debug)]
pub struct StoredResource {
    amount: f32,
    capacity: f32
}

impl StoredResource {
    pub fn empty(capacity: f32) -> Self {
        Self::new(0.0, capacity)
    }

    pub fn full(capacity: f32) -> Self {
        Self::new(capacity, capacity)
    }

    pub fn new(amount: f32, capacity: f32) -> Self {
        debug_assert!(amount <= capacity);
        debug_assert!(capacity >= 0.0);
        debug_assert!(amount >= 0.0);

        Self {
            amount, capacity
        }
    }

    pub fn percentage(&self) -> f32 {
        self.amount / self.capacity
    }

    pub fn reduce(&mut self, amount: f32) -> f32 {
        let reduced_by = self.amount.min(amount);
        self.amount -= reduced_by;
        reduced_by
    } 

    pub fn increase(&mut self, amount: f32) -> f32 {
        let increased_by = (self.capacity - self.amount).min(amount);
        self.amount += increased_by;
        increased_by
    }

    pub fn is_empty(&self) -> bool {
        self.amount == 0.0
    }

    pub fn amount(&self) -> f32 {
        self.amount
    }

    pub fn capacity(&self) -> f32 {
        self.capacity
    }

    pub fn transfer_to(&mut self, other: &mut Self, amount: f32) -> f32 {
        let amount = self.transfer_amount(other, amount);
        self.reduce(amount);
        other.increase(amount);
        amount
    }

    pub fn transfer_amount(&self, other: &Self, amount: f32) -> f32 {
        self.amount_can_transfer(other).min(amount)
    }

    fn amount_can_transfer(&self, other: &Self) -> f32 {
        self.amount.min(other.amount_to_capacity())
    }

    fn amount_to_capacity(&self) -> f32 {
        self.capacity - self.amount
    }
}

impl fmt::Display for StoredResource {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "{:.2}/{:.2}", self.amount, self.capacity)
    }
}