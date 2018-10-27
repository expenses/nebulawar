use cgmath::*;
use super::*;
use specs::{DenseVecStorage, World, Component};
use context;
use util::*;
use entities::*;
use spade;
use spade::delaunay::FloatDelaunayTriangulation;
use tint::Color;

#[derive(Debug)]
enum SystemType {
    Asteroids,
    Planetoid,
    Nebula,
    BlackHole
}

impl SystemType {
    fn random(rng: &mut ThreadRng) -> Self {
        let num = rng.gen_range(0, 100);

        match num {
            0 ... 29 => SystemType::Asteroids,
            30 ... 59 => SystemType::Planetoid,
            60 ... 89 => SystemType::Nebula,
            90 ... 100 => SystemType::BlackHole,
            _ => unreachable!()
        }
    }
}

#[derive(Deserialize, Serialize, Component)]
pub struct System {
    pub location: Vector2<f32>,
    pub stars: Vec<context::Vertex>,
    pub light: Vector3<f32>,
    pub background: Vec<context::Vertex>
}

impl System {
    pub fn new(location: Vector2<f32>, rng: &mut ThreadRng, world: &mut World) -> Self {
        let _distance_from_center = location.magnitude();

        let stars = 10000;

        let stars = (0 .. stars)
            .map(|_| {
                context::Vertex::with_brightness(
                    uniform_sphere_distribution(rng) * (BACKGROUND_DISTANCE + 100.0),
                    rng.gen_range(0.0_f32, 1.0)
                )
            })
            .collect();

        let mut light = uniform_sphere_distribution(rng);
        light.y = light.y.abs();

        let system_type = SystemType::random(rng);

        info!("Generated a {:?} system at {:?}.", system_type, location);

        for _ in 0 .. rng.gen_range(5, 10) {
            add_asteroid(rng, world);
        }

        Self {
            light,
            background: make_background(rng),
            stars, location
        }
    }
}

impl Default for System {
    fn default() -> Self {
        Self {
            location: Vector2::zero(),
            stars: Vec::new(),
            light: Vector3::zero(),
            background: Vec::new()
        }
    }
}

// https://www.redblobgames.com/x/1842-delaunay-voronoi-sphere/#delaunay
fn make_background(rng: &mut ThreadRng) -> Vec<context::Vertex> {
    let nebula_color = Color::new(rng.gen_range(0.0, 360.0), 1.0, rng.gen_range(0.5, 1.0), 1.0).from_hsv();
    let nebula_color = Vector3::new(nebula_color.red as f32, nebula_color.green as f32, nebula_color.blue as f32);

    let mut dlt = FloatDelaunayTriangulation::with_walk_locate();

    // Get the point to rotate the sphere around
    let target_point = ColouredVertex::rand(rng, Quaternion::zero(), nebula_color);

    // Get the rotation to that point
    let rotation_quat = Quaternion::look_at(target_point.vector, UP);

    for _ in 0 .. 100 {
        dlt.insert(ColouredVertex::rand(rng, rotation_quat, nebula_color));
    }

    let triangles_to_fill_gap = dlt.edges()
        // get all edges that touch the 'infinite face'
        .filter(|edge| edge.sym().face() == dlt.infinite_face())
        // make a triangle to the target point
        .flat_map(|edge| iter_owned([target_point, *edge.to(), *edge.from()]));

    let vertices = dlt.triangles()
        // flat map to vertices
        .flat_map(|face| iter_owned(face.as_triangle()))
        .map(|vertex| *vertex)
        // chain with gap triangles
        .chain(triangles_to_fill_gap)
        // map to game vertices
        .map(|vertex| {
            context::Vertex::with_color(
                vertex.vector * (BACKGROUND_DISTANCE + 2500.0),
                vertex.color
            )
        })
        // collect into vec
        .collect();

    vertices
}

#[derive(PartialEq, Debug, Clone, Copy)]
struct ColouredVertex {
    vector: Vector3<f32>,
    projected_x: f32,
    projected_y: f32,
    color: [f32; 3]
}

impl ColouredVertex {
    fn rand(rng: &mut ThreadRng, rotation_quat: Quaternion<f32>, color: Vector3<f32>) -> Self {
        use noise::{self, NoiseFn, Seedable};

        let vector = uniform_sphere_distribution(rng);
        let rotated_vector = rotation_quat * vector;

        let value = noise::Perlin::new()
            .set_seed(rng.gen())
            .get([vector.x as f64, vector.y as f64, vector.z as f64]);

        Self {
            vector,
            color: (color * value as f32).into(),
            // calculate points stereographically projected
            projected_x: rotated_vector.x / (1.0 - rotated_vector.z),
            projected_y: rotated_vector.y / (1.0 - rotated_vector.z)
        }
    }
}

impl spade::PointN for ColouredVertex {
    type Scalar = f32;

    fn dimensions() -> usize {
        2
    }

    fn from_value(_: Self::Scalar) -> Self {
        unimplemented!()
    }

    fn nth(&self, index: usize) -> &Self::Scalar {
        match index {
            0 => &self.projected_x,
            1 => &self.projected_y,
            _ => unreachable!()
        }
    }

    fn nth_mut(&mut self, index: usize) -> &mut Self::Scalar {
        match index {
            0 => &mut self.projected_x,
            1 => &mut self.projected_y,
            _ => unreachable!()
        }
    }
}

impl spade::TwoDimensional for ColouredVertex {}