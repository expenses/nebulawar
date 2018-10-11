use cgmath::*;
use std::f32::consts::*;
use util::*;
use collision::*;

pub struct Camera {
    center: Vector3<f32>,
    longitude: f32,
    latitude: f32,
    distance: f32
}

impl Camera {
    pub fn set_position(&mut self, position: Vector3<f32>) {
        self.center = position;
    }

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
    }

    pub fn move_forwards(&mut self, amount: f32) {
        self.center.x -= amount * self.longitude.sin();
        self.center.z -= amount * self.longitude.cos();
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

    /*pub fn fire_ray(&self, x: f32, y: f32) -> Ray<f32, Point3<f32>, Vector3<f32>> {
        let dx = (FOV * 0.5).tan() * 

        //Ray::new(vector_to_point(self.position()), -self.direction())
    }*/
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            center: Vector3::new(0.0, 0.0, 0.0),
            longitude: 0.0,
            latitude: FRAC_PI_4,
            distance: 30.0
        }
    }
}