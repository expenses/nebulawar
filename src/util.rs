use cgmath::*;
use std::f32::consts::*;
use arrayvec;
use std::ops::*;

pub const FOV: f32 = FRAC_PI_3;
pub const FAR: f32 = 10240.0;
pub const NEAR: f32 = 0.1;
pub const UP: Vector3<f32> = Vector3 {
    x: 0.0,
    y: 1.0,
    z: 0.0
};

pub fn perspective_matrix(aspect_ratio: f32) -> Matrix4<f32> {
    let f = 1.0 / (FOV / 2.0).tan();

    Matrix4::new(
        f *   aspect_ratio, 0.0,    0.0,                            0.0,
        0.0,                f,      0.0,                            0.0,
        0.0,                0.0,    (FAR+NEAR)/(FAR-NEAR),      1.0,
        0.0,                0.0,    -(2.0*FAR*NEAR)/(FAR-NEAR), 0.0
    )
}

pub fn opengl_pos_to_screen_pos(x: f32, y: f32, width: f32, height: f32) -> (f32, f32) {
    (
        (x + 1.0) / 2.0 * width / 2.0,
        (1.0 - y) / 2.0 * height / 2.0
    )
}

pub fn screen_pos_to_opengl_pos(x: f32, y: f32, width: f32, height: f32) -> (f32, f32) {
    (
        2.0 * x / width - 1.0,
        - 2.0 * y / height + 1.0
    )
}

pub fn matrix_to_array(matrix: Matrix4<f32>) -> [[f32; 4]; 4] {
    matrix.into()
}

pub fn vector_to_point(vector: Vector3<f32>) -> Point3<f32> {
    Point3::new(vector.x, vector.y, vector.z)
}

pub fn point_to_vector(point: Point3<f32>) -> Vector3<f32> {
    Vector3::new(point.x, point.y, point.z)
}

pub fn vector_to_array(vector: Vector3<f32>) -> [f32; 3] {
    vector.into()
}

pub fn iter_owned<T, A: arrayvec::Array<Item=T>>(array: A) -> arrayvec::IntoIter<A> {
    arrayvec::ArrayVec::from(array).into_iter()
}

pub fn look_at(point: Vector3<f32>) -> Quaternion<f32> {
    Quaternion::look_at(point, UP).invert()
}

pub trait Positioned {
    fn distance(&self, other: &Self) -> f32;
    fn normalize_to(&self, value: f32) -> Self;
}

impl Positioned for Vector3<f32> {
    fn distance(&self, other: &Self) -> f32 {
        (*self).distance(*other)
    }

    fn normalize_to(&self, value: f32) -> Self {
        (*self).normalize_to(value)
    }
}

impl Positioned for f32 {
    fn distance(&self, other: &Self) -> f32 {
        (*self - *other).abs()
    }

    fn normalize_to(&self, value: f32) -> Self {
        if self.is_sign_positive() {
            value
        } else {
            -value
        }
    }
}

pub fn move_towards<T: Sub<Output=T> + Add<Output=T> + Positioned + Clone>(position: T, target: T, step: f32) -> T {
    let delta = target.clone() - position.clone();

    if step < position.distance(&target) {
        position + delta.normalize_to(step)
    } else {
        target
    }
}