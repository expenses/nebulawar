pub struct Camera {
    center: [f32; 3],
    longitude: f32,
    latitude: f32,
    distance: f32
}

impl Camera {
    pub fn rotate_longitude(&mut self, amount: f32) {
        self.longitude += amount;
    }

    pub fn rotate_latitude(&mut self, amount: f32) {
        self.latitude += amount;
    }

    pub fn change_distance(&mut self, amount: f32) {
        self.distance += amount;
        self.distance = self.distance.max(0.1).min(100.0);
    }

    pub fn move_sideways(&mut self, amount: f32) {
        self.center[0] -= amount * (-self.longitude).cos();
        self.center[2] -= amount * (-self.longitude).sin();
    }

    pub fn move_forwards(&mut self, amount: f32) {
        self.center[0] -= amount * self.longitude.sin();
        self.center[2] -= amount * self.longitude.cos();
    }

    pub fn position(&self) -> [f32; 3] {
        let x_off = self.longitude.sin() * self.distance;
        let y_off = self.longitude.cos() * self.distance;
        let z_off = self.latitude.sin() * self.distance;

        [
            self.center[0] + x_off,
            self.center[1] + z_off,
            self.center[2] + y_off
        ]
    }

    pub fn view_matrix(&self) -> [[f32; 4]; 4] {
        let up = [0.0, 1.0, 0.0];
        let position = self.position();
        let direction = [
            -self.longitude.sin(),
            -self.latitude.sin(),
            -self.longitude.cos()
        ];
        
        view_matrix(&position, &direction, &up)   
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            center: [0.0; 3],
            longitude: 0.0,
            latitude: 0.0,
            distance: 5.0
        }
    }
}


fn view_matrix(position: &[f32; 3], direction: &[f32; 3], up: &[f32; 3]) -> [[f32; 4]; 4] {
    let f = {
        let f = direction;
        let len = f[0] * f[0] + f[1] * f[1] + f[2] * f[2];
        let len = len.sqrt();
        [f[0] / len, f[1] / len, f[2] / len]
    };

    let s = [up[1] * f[2] - up[2] * f[1],
             up[2] * f[0] - up[0] * f[2],
             up[0] * f[1] - up[1] * f[0]];

    let s_norm = {
        let len = s[0] * s[0] + s[1] * s[1] + s[2] * s[2];
        let len = len.sqrt();
        [s[0] / len, s[1] / len, s[2] / len]
    };

    let u = [f[1] * s_norm[2] - f[2] * s_norm[1],
             f[2] * s_norm[0] - f[0] * s_norm[2],
             f[0] * s_norm[1] - f[1] * s_norm[0]];

    let p = [-position[0] * s_norm[0] - position[1] * s_norm[1] - position[2] * s_norm[2],
             -position[0] * u[0] - position[1] * u[1] - position[2] * u[2],
             -position[0] * f[0] - position[1] * f[1] - position[2] * f[2]];

    [
        [s_norm[0], u[0], f[0], 0.0],
        [s_norm[1], u[1], f[1], 0.0],
        [s_norm[2], u[2], f[2], 0.0],
        [p[0], p[1], p[2], 1.0],
    ]
}