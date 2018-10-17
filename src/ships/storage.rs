#[derive(Serialize, Deserialize)]
pub struct Storage {
    amount: f32
}

impl Storage {
    pub fn empty() -> Self {
        Self {
            amount: 0.0
        }
    }

    pub fn new(amount: f32) -> Self {
        Self {
            amount
        }
    }

    pub fn reduce(&mut self, amount: f32) -> f32 {
        let reduced_by = self.amount.min(amount);
        self.amount -= reduced_by;
        reduced_by
    } 

    pub fn increase(&mut self, amount: f32, limit: f32) -> f32 {
        let increased_by = (limit - self.amount).min(amount);
        self.amount += increased_by;
        increased_by
    }

    pub fn is_empty(&self) -> bool {
        self.amount == 0.0
    }

    pub fn amount(&self) -> f32 {
        self.amount
    }

    pub fn transfer_to(&mut self, other: &mut Self, amount: f32, other_max: f32) -> f32 {
        let amount = self.transfer_amount(other, amount, other_max);
        self.reduce(amount);
        other.increase(amount, other_max);
        amount
    }

    fn transfer_amount(&self, other: &Self, amount: f32, other_max: f32) -> f32 {
        self.amount_can_transfer(other, other_max).min(amount)
    }

    fn amount_can_transfer(&self, other: &Self, other_max: f32) -> f32 {
        self.amount.min(other.amount_to_max(other_max))
    }

    fn amount_to_max(&self, max: f32) -> f32 {
        max - self.amount
    }
}
