use cgmath::*;
use std::f32::consts::*;
use util::*;
use *;

#[derive(Serialize, Deserialize)]
pub struct Camera {
    center: Vector3<f32>,
    longitude: f32,
    latitude: f32,
    distance: f32,
    target_distance: f32,
    focus: HashSet<ShipID>
}

impl Camera {
    // the camera cannot go up to a latitude of a full half pi because they it starts to flip over, so go to 99%.
    const MAX_VERTICALITY: f32 = FRAC_PI_2 * 0.99;

    const MIN_ZOOM: f32 = 2.0;
    const MAX_ZOOM: f32 = 100.0;
    const MAX_ZOOM_SPEED: f32 = 2.0;
    const DEFAULT_ZOOM: f32 = 30.0;

    pub fn rotate_longitude(&mut self, amount: f32) {
        self.longitude += amount;
    }

    pub fn rotate_latitude(&mut self, amount: f32) {
        self.latitude = (self.latitude + amount).max(-Self::MAX_VERTICALITY).min(Self::MAX_VERTICALITY);
    }

    pub fn change_distance(&mut self, amount: f32) {
        self.target_distance += amount;
        self.target_distance = self.target_distance.max(Self::MIN_ZOOM).min(Self::MAX_ZOOM);
    }

    pub fn move_sideways(&mut self, amount: f32) {
        self.center.x -= amount * (-self.longitude).cos();
        self.center.z -= amount * (-self.longitude).sin();
        self.focus.clear();
    }

    pub fn move_forwards(&mut self, amount: f32) {
        self.center.x -= amount * self.longitude.sin();
        self.center.z -= amount * self.longitude.cos();
        self.focus.clear();
    }

    fn direction(&self) -> Vector3<f32> {
        Vector3::new(
            self.longitude.sin() * self.latitude.cos(),
            self.latitude.sin(),
            self.longitude.cos() * self.latitude.cos()
        )
    }

    pub fn position(&self) -> Vector3<f32> {   
        self.center + self.direction() * self.distance
    }

    pub fn view_matrix(&self) -> Matrix4<f32> {
        Matrix4::look_at_dir(
            vector_to_point(self.position()), self.direction(), UP
        )
    }

    pub fn step(&mut self, ships: &AutoIDMap<ShipID, Ship>) {
        self.distance = move_towards(self.distance, self.target_distance, Self::MAX_ZOOM_SPEED);

        if let Some(position) = average_position(&self.focus, ships) {
            self.center = position;
        }
    }

    pub fn set_focus(&mut self, ships: &HashSet<ShipID>) {
        self.focus.clone_from(ships)
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            center: Vector3::new(0.0, 0.0, 0.0),
            longitude: 0.0,
            latitude: FRAC_PI_4,
            distance: Self::DEFAULT_ZOOM,
            target_distance: Self::DEFAULT_ZOOM,
            focus: HashSet::new()
        }
    }
}