use cgmath::*;
use std::f32::consts::*;
use util::*;
use specs::*;

#[derive(Serialize, Deserialize, Clone, Component)]
pub struct Camera {
    center: Vector3<f32>,
    longitude: f32,
    latitude: f32,
    distance: f32,
    target_distance: f32
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
    }

    pub fn move_forwards(&mut self, amount: f32) {
        self.center.x -= amount * self.longitude.sin();
        self.center.z -= amount * self.longitude.cos();
    }

    pub fn direction(&self) -> Vector3<f32> {
        Vector3::new(
            self.longitude.sin() * self.latitude.cos(),
            self.latitude.sin(),
            self.longitude.cos() * self.latitude.cos()
        )
    }

    pub fn position(&self) -> Vector3<f32> {   
        self.center + self.direction() * self.distance
    }

    pub fn view_matrix_only_direction(&self) -> Matrix4<f32> {
        Matrix4::look_at_dir(
            vector_to_point(Vector3::zero()), self.direction(), UP
        )
    }

    pub fn view_matrix(&self) -> Matrix4<f32> {
        Matrix4::look_at_dir(
            vector_to_point(self.position()), self.direction(), UP
        )
    }

    pub fn step(&mut self) {
        self.distance = move_towards(self.distance, self.target_distance, Self::MAX_ZOOM_SPEED);
    }

    pub fn move_towards(&mut self, target: Vector3<f32>) {
        self.center = move_towards(self.center, target, 50.0);
    }

    pub fn screen_position(&self, point: Vector3<f32>, (screen_width, screen_height): (f32, f32)) -> Option<(f32, f32, f32)> {
        let modelview = self.view_matrix() * Matrix4::from_translation(point);

        let gl_position = perspective_matrix(screen_height / screen_width) * modelview * Vector4::new(0.0, 0.0, 0.0, 1.0);

        let x = gl_position[0] / gl_position[3];
        let y = gl_position[1] / gl_position[3];
        let z = gl_position[2] / gl_position[3];

        let (x, y) = opengl_pos_to_screen_pos(x, y, screen_width, screen_height);
        // this may be dpi dependent, not sure
        let (x, y) = (x * 2.0, y * 2.0);

        if z < 1.0 {
            Some((x, y, z))
        } else {
            None
        }
    }

    // http://webglfactory.blogspot.com/2011/05/how-to-convert-world-to-screen.html
    // http://antongerdelan.net/opengl/raycasting.html
    pub fn ray(&self, (x, y): (f32, f32), (screen_width, screen_height): (f32, f32)) -> collision::Ray<f32, Point3<f32>, Vector3<f32>> {
        let (x, y) = screen_pos_to_opengl_pos(x, y, screen_width, screen_height);

        let clip = Vector4::new(-x, -y, -1.0, 1.0);

        let eye = perspective_matrix(screen_height / screen_width).invert().unwrap() * clip;
        let eye = Vector4::new(eye.x, eye.y, -1.0, 0.0);

        let direction = (self.view_matrix().invert().unwrap() * eye).truncate().normalize_to(-1.0);

        collision::Ray::new(vector_to_point(self.position()), direction)
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            center: Vector3::new(0.0, 0.0, 0.0),
            longitude: 0.0,
            latitude: FRAC_PI_4,
            distance: Self::DEFAULT_ZOOM,
            target_distance: Self::DEFAULT_ZOOM
        }
    }
}