use specs::*;
use cgmath::*;
use glium::glutin::*;
use ships::*;
use odds::vec::*;
use context;
use ncollide3d::shape::*;
use ncollide3d::query::Ray;
use ncollide3d::bounding_volume::*;
use util::*;

#[derive(Component, Default, NewtypeProxy)]
pub struct Secs(pub f32);

#[derive(Component, Default, Serialize, Deserialize, Clone)]
pub struct Time(pub f32);

#[derive(Component, Default, Serialize, Deserialize, Clone)]
pub struct Paused(pub bool);

impl Paused {
    pub fn switch(&mut self) {
        self.0 = !self.0;
    }
}

#[derive(Component, Default)]
pub struct EntityUnderMouse(pub Option<(Entity, Vector3<f32>)>);

// todo: have this on a per-entity basis

#[derive(Component, Default)]
pub struct RightClickOrder {
    pub to_move: Vec<Entity>,
    pub command: Option<Command>
}

#[derive(Component, Default)]
pub struct AveragePosition(pub Option<Vector3<f32>>);

#[derive(Component, Default, NewtypeProxy)]
pub struct Events(pub Vec<WindowEvent>);

#[derive(Component, Default, Clone, Serialize, Deserialize)]
pub struct MovementPlane(pub f32);

#[derive(Component, NewtypeProxy, Default, Clone, Serialize, Deserialize)]
pub struct Log(pub Vec<LogItem>);

impl Log {
    pub fn append(&mut self, text: String) {
        self.push(LogItem {
            age: 0.0,
            content: text
        })
    }

    pub fn step(&mut self, secs: f32) {
        self.retain_mut(|item| {
            item.age += secs;
            item.age < 5.0
        });
    }

    pub fn render(&self, context: &mut context::Context) {
        let (_, height) = context.screen_dimensions();

        for (i, item) in self.iter().enumerate() {
            context.render_text(&item.content, 10.0, height - 30.0 - i as f32 * 20.0);
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LogItem {
    age: f32,
    content: String
}

#[derive(Component, NewtypeProxy)]
pub struct MouseRay(pub Ray<f32>);

impl Default for MouseRay {
    fn default() -> Self {
        MouseRay(Ray::new(
            nalgebra::Point3::new(0.0, 0.0, 0.0),
            nalgebra::Vector3::zero()
        ))
    }
}

#[derive(Component, Default)]
pub struct ScreenDimensions(pub (f32, f32));

#[derive(Component, Default, Clone, Serialize, Deserialize)]
pub struct Debug(pub bool);

#[derive(Component)]
pub struct Meshes {
    meshes: context::MeshArray
}

impl Meshes {
    pub fn new(meshes: context::MeshArray) -> Self {
        Self {
            meshes
        }
    }

    pub fn get_mesh(&self, model: context::Model) -> &TriMesh<f32> {
        &self.meshes[model as usize]
    }

    pub fn get_bbox(&self, model: context::Model, pos: Vector3<f32>, rot: Quaternion<f32>, size: f32) -> AABB<f32> {
        let bbox: AABB<f32> = self.get_mesh(model).bounding_volume(&make_iso(Vector3::zero(), rot));

        let pos = vector_to_na_vector(pos);

        AABB::new(*bbox.mins() * size + pos, *bbox.maxs() * size + pos)
    }

    pub fn intersects(
        &self,
        model_a: context::Model, pos_a: Vector3<f32>, rot_a: Quaternion<f32>, size_a: f32,
        model_b: context::Model, pos_b: Vector3<f32>, rot_b: Quaternion<f32>, size_b: f32
    ) -> bool {
        
        let bb_a = self.get_bbox(model_a, pos_a, rot_a, size_a);
        let bb_b = self.get_bbox(model_b, pos_b, rot_b, size_b);

        bb_a.intersects(&bb_b)
    }
}

impl Default for Meshes {
    fn default() -> Self {
        Self {
            meshes: context::MeshArray::default()
        }
    }
}