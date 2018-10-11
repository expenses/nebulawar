use cgmath::*;
use std::f32::consts::*;
use util::*;
use *;

pub struct Camera {
    center: Vector3<f32>,
    longitude: f32,
    latitude: f32,
    distance: f32,
    focus: HashSet<usize>
}

impl Camera {
    pub fn rotate_longitude(&mut self, amount: f32) {
        self.longitude += amount;
    }

    pub fn rotate_latitude(&mut self, amount: f32) {
        self.latitude = (self.latitude + amount).max(-FRAC_PI_2).min(FRAC_PI_2);
    }

    pub fn change_distance(&mut self, amount: f32) {
        self.distance += amount;
        self.distance = self.distance.max(2.0).min(100.0);
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
            self.longitude.sin(),
            self.latitude.sin(),
            self.longitude.cos()
        )
    }

    pub fn position(&self) -> Vector3<f32> {   
        self.center + self.direction() * self.distance
    }

    pub fn view_matrix(&self) -> Matrix4<f32> {
        Matrix4::look_at_dir(
            vector_to_point(self.position()), self.direction(), Vector3::new(0.0, 1.0, 0.0)
        )
    }

    pub fn step(&mut self, ships: &Ships) {
        if !self.focus.is_empty() {
            self.center = self.focus.iter().fold(Vector3::zero(), |vector, index| {
                vector + ships[*index].position
            }) / self.focus.len() as f32;
        }
    }

    pub fn set_focus(&mut self, ships: &HashSet<usize>) {
        self.focus.clone_from(ships)
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            center: Vector3::new(0.0, 0.0, 0.0),
            longitude: 0.0,
            latitude: FRAC_PI_4,
            distance: 30.0,
            focus: HashSet::new()
        }
    }
}