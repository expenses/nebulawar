use cgmath::*;
use super::*;
use specs::{DenseVecStorage, World, Component};
use crate::context;
use crate::util::*;
use entities::*;
use spade::delaunay::FloatDelaunayTriangulation;
use tint::Colour;
use zerocopy::AsBytes;

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
            0 ..= 29 => SystemType::Asteroids,
            30 ..= 59 => SystemType::Planetoid,
            60 ..= 89 => SystemType::Nebula,
            90 ..= 100 => SystemType::BlackHole,
            _ => unreachable!()
        }
    }
}

#[derive(Serialize, Deserialize, Component)]
pub struct StarSystem {
    pub location: Vector2<f32>,
    pub stars: Vec<(f64, f64, f32)>,
    pub light: Vector3<f32>,
    pub background: Vec<context::Vertex>,
    pub ambient_colour: [f32; 3],
    #[serde(skip)]
    pub star_buffer: Option<wgpu::Buffer>
}

impl Clone for StarSystem {
    fn clone(&self) -> Self {
        Self {
            location: self.location,
            stars: self.stars.clone(),
            light: self.light,
            background: self.background.clone(),
            ambient_colour: self.ambient_colour,
            star_buffer: None
        }
    }
}

impl StarSystem {
    pub fn new(location: Vector2<f32>, rng: &mut ThreadRng, world: &mut World) -> Self {
        let _distance_from_center = location.magnitude();

        let stars = 10000;

        let stars = (0 .. stars)
            .map(|_| (rng.gen_range(0.0, 1.0), rng.gen_range(0.0, 1.0), rng.gen_range(0.0, 1.0)))
            .collect();

        let mut light = uniform_sphere_distribution(rng);
        light.y = light.y.abs();

        let system_type = SystemType::random(rng);

        info!("Generated a {:?} system at {:?}.", system_type, location);

        for _ in 0 .. rng.gen_range(5, 10) {
            add_asteroid(rng, world);
        }

        let (background, ambient_colour) = make_background(rng);

        Self {
            light, background, stars, location, ambient_colour,
            star_buffer: None,
        }
    }

    pub fn star_buffer(&mut self, device: &wgpu::Device) -> &wgpu::Buffer {
        let Self { star_buffer, stars, .. } = self;

        star_buffer.get_or_insert_with(|| {
            let vec = stars.iter()
                .map(|&(x, y, brightness)| (uniform_sphere_distribution_from_coords(x, y), brightness))
                .map(|(position, brightness)| {
                    let position = position * (BACKGROUND_DISTANCE + 100.0);
                    let rotation: Matrix4<f32> = look_at(position).into();
                    let matrix = Matrix4::from_translation(position) * rotation * Matrix4::from_scale(BACKGROUND_DISTANCE / 400.0);

                    context::InstanceVertex {
                        instance_pos: matrix.into(),
                        uv_dimensions: [0.0; 2],
                        diff_offset: [brightness; 2],
                        spec_offset: [0.0; 2]
                    }
                })
                .collect::<Vec<_>>();

            device.create_buffer_with_data(vec.as_bytes(), wgpu::BufferUsage::VERTEX)
        })
    }
}

impl Default for StarSystem {
    fn default() -> Self {
        Self {
            location: Vector2::zero(),
            stars: Vec::new(),
            light: Vector3::zero(),
            background: Vec::new(),
            ambient_colour: [0.0; 3],
            star_buffer: None,
        }
    }
}

// https://www.redblobgames.com/x/1842-delaunay-voronoi-sphere/#delaunay
fn make_background(rng: &mut ThreadRng) -> (Vec<context::Vertex>, [f32; 3]) {
    let nebula_colour = Colour::new(rng.gen_range(0.0, 360.0), 1.0, rng.gen_range(0.5, 1.0), 1.0).from_hsv();
    let nebula_colour = Vector3::new(nebula_colour.red as f32, nebula_colour.green as f32, nebula_colour.blue as f32);
    let colour_mod = rng.gen_range(-0.5, 1.0);

    let mut dlt = FloatDelaunayTriangulation::with_walk_locate();

    // Get the point to rotate the sphere around
    let target_point = ColouredVertex::rand(rng, Quaternion::zero(), nebula_colour, colour_mod);

    // Get the rotation to that point
    let rotation_quat = Quaternion::look_at(target_point.vector, UP);

    for _ in 0 .. 100 {
        dlt.insert(ColouredVertex::rand(rng, rotation_quat, nebula_colour, colour_mod));
    }

    let triangles_to_fill_gap = dlt.edges()
        // get all edges that touch the 'infinite face'
        .filter(|edge| edge.sym().face() == dlt.infinite_face())
        // make a triangle to the target point
        .flat_map(|edge| iter_owned([target_point, *edge.to(), *edge.from()]));

    let vertices: Vec<_> = dlt.triangles()
        // flat map to vertices
        .flat_map(|face| iter_owned(face.as_triangle()))
        .map(|vertex| *vertex)
        // chain with gap triangles
        .chain(triangles_to_fill_gap)
        // map to game vertices
        .map(|vertex| {
            context::Vertex::with_colour(
                vertex.vector * (BACKGROUND_DISTANCE + 2500.0),
                vertex.colour
            )
        })
        // collect into vec
        .collect();

    let ambient = avg(vertices.iter().map(|vertex| vertex.normal.into()));

    (vertices, ambient.unwrap().into())
}

#[derive(PartialEq, Debug, Clone, Copy)]
struct ColouredVertex {
    vector: Vector3<f32>,
    projected_x: f32,
    projected_y: f32,
    colour: [f32; 3]
}

impl ColouredVertex {
    fn rand(rng: &mut ThreadRng, rotation_quat: Quaternion<f32>, colour: Vector3<f32>, colour_mod: f64) -> Self {
        use noise::{NoiseFn, Seedable};

        let vector = uniform_sphere_distribution(rng);
        let rotated_vector = rotation_quat * vector;

        let value = noise::Perlin::new()
            .set_seed(rng.gen())
            .get([f64::from(vector.x), f64::from(vector.y), f64::from(vector.z)]) + colour_mod;

        let value = value.max(0.0);

        Self {
            vector,
            colour: (colour * value as f32).into(),
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
