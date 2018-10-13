use cgmath::*;
use std::f32::consts::*;
use std::ops::*;
use std::collections::*;
use std::collections::hash_map::*;
use std::hash::*;

pub const FOV: f32 = FRAC_PI_3;
pub const FAR: f32 = 10240.0;
pub const NEAR: f32 = 0.1;

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

pub trait ID: Hash + Copy + Eq + Default {
    fn increment(&mut self);
}

pub trait IDed<I> {
    fn set_id(&mut self, id: I);
}

#[derive(Deserialize, Serialize)]
pub struct AutoIDMap<I: ID, T> {
    next_id: I,
    inner: HashMap<I, T>
}

impl<I: ID, T: IDed<I>> AutoIDMap<I, T> {
    pub fn new() -> Self {
        Self {
            next_id: I::default(),
            inner: HashMap::new()
        }
    }

    pub fn push(&mut self, mut item: T) -> I {
        let id = self.next_id;

        item.set_id(id);
        self.inner.insert(id, item);
        self.next_id.increment();

        id
    }

    pub fn get(&self, id: I) -> Option<&T> {
        self.inner.get(&id)
    }

    pub fn get_mut(&mut self, id: I) -> Option<&mut T> {
        self.inner.get_mut(&id)
    }

    pub fn iter(&self) -> Values<I, T> {
        self.inner.values()
    }

    pub fn iter_mut(&mut self) -> ValuesMut<I, T> {
        self.inner.values_mut()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<I: ID, T> Index<I> for AutoIDMap<I, T> {
    type Output = T;

    fn index(&self, index: I) -> &T {
        &self.inner[&index]
    }
}

impl<I: ID, T: IDed<I>> IndexMut<I> for AutoIDMap<I, T> {
    fn index_mut(&mut self, index: I) -> &mut T {
        self.get_mut(index).unwrap()
    }
}