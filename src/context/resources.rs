use glium::*;
use genmesh;
use obj::*;
use context::Vertex;
use image;
use runic;
use util::*;
use glium::texture::*;
use std::io;
use failure;
use specs::*;
use collision::primitive::ConvexPolyhedron;
use cgmath::Vector2;

const NORMAL: [f32; 3] = [0.0, 0.0, 1.0];

const TOP_LEFT: Vertex = Vertex {
    position: [-0.5, 0.5, 0.0],
    texture: [0.0; 2],
    normal: NORMAL
};

const TOP_RIGHT: Vertex = Vertex {
    position: [0.5, 0.5, 0.0],
    texture: [1.0, 0.0],
    normal: NORMAL
};

const BOTTOM_LEFT: Vertex = Vertex {
    position: [-0.5, -0.5, 0.0],
    texture: [0.0, 1.0],
    normal: NORMAL
};

const BOTTOM_RIGHT: Vertex = Vertex {
    position: [0.5, -0.5, 0.0],
    texture: [1.0; 2],
    normal: NORMAL
};

pub const BILLBOARD_VERTICES: [Vertex; 6] = [TOP_LEFT, TOP_RIGHT, BOTTOM_LEFT, TOP_RIGHT, BOTTOM_RIGHT, BOTTOM_LEFT];

#[cfg(feature = "embed_resources")]
macro_rules! load_resource {
    ($filename:expr) => (
        include_bytes!(concat!("../../resources/", $filename))
    )
}

#[cfg(not(feature = "embed_resources"))]
macro_rules! load_resource {
    ($filename:expr) => ({
        use std::fs;
        use std::path::*;
        &fs::read(PathBuf::from("resources").join($filename))?
    })
}

pub fn load_image(display: &Display, data: &[u8]) -> SrgbTexture2d {
    let image = image::load_from_memory_with_format(data, image::ImageFormat::PNG).unwrap().to_rgba();
    let image_dimensions = image.dimensions();
    let image = RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);
    SrgbTexture2d::new(display, image).unwrap()
}

// Returns a vertex buffer that should be rendered as `TrianglesList`.
pub fn load_wavefront(data: &[u8]) ->  Vec<Vertex> {
    let mut data = io::BufReader::new(data);
    let data = Obj::load_buf(&mut data).unwrap();

    let Obj {texture, normal, position, objects, ..} = data;

    objects.into_iter()
        .flat_map(|object| object.groups)
        .flat_map(|group| group.polys)
        .flat_map(|polygon| {
            match polygon {
                genmesh::Polygon::PolyTri(genmesh::Triangle { x: v1, y: v2, z: v3 }) => iter_owned([v1, v2, v3]),
                genmesh::Polygon::PolyQuad(_) => unimplemented!("Quad polygons not supported, use triangles instead.")
            }
        })
        .map(|v| {
            Vertex {
                position: position[v.0],
                normal: v.2.map(|index| normal[index]).unwrap_or([0.0, 0.0, 0.0]),
                texture: v.1.map(|index| texture[index]).unwrap_or([0.0, 0.0]),
            }
        })
        .collect()
}

#[derive(Copy, Clone, Component, Serialize, Deserialize, PartialEq)]
pub enum Image {
    Star,
    Smoke,
    Move,
    Mine,
    Attack
}

impl Image {
    pub fn translate(&self, uv: [f32; 2]) -> [f32; 2] {
        let mut uv: Vector2<f32> = uv.into();
        uv = Vector2::new(uv.x * self.dimensions().x, uv.y * self.dimensions().y);
        (self.offset() + uv).into()
    }
}

include!(concat!(env!("OUT_DIR"), "/packed_textures.rs"));

#[derive(Serialize, Deserialize, Component, Copy, Clone)]
pub enum Model {
    Fighter = 0,
    Tanker = 1,
    Carrier = 2,
    Asteroid = 3,
    Miner = 4,
    Missile = 5
}

pub struct ObjModel {
    pub vertices: VertexBuffer<Vertex>,
    pub collision_mesh: ConvexPolyhedron<f32>,
    pub texture: SrgbTexture2d
}

impl ObjModel {
    fn new(display: &Display, model: &[u8], image: &[u8]) -> io::Result<Self> {
        let vertices = load_wavefront(model);

        let points: Vec<_> = vertices.iter()
            .map(|vertex| vector_to_point(vertex.position.into()))
            .collect();

        let faces = (0 .. vertices.len() / 3)
            .map(|i| (i * 3, i * 3 + 1, i * 3 + 2))
            .collect();

        Ok(Self {
            vertices: VertexBuffer::new(display, &vertices).unwrap(),
            collision_mesh: ConvexPolyhedron::new_with_faces(points, faces),
            texture: load_image(display, image)
        })
    }
}

pub struct Resources {
    pub models: [ObjModel; 6],
    pub image: SrgbTexture2d,
    pub billboard: VertexBuffer<Vertex>,
    pub font: runic::CachedFont<'static>
}

impl Resources {
    pub fn new(display: &Display) -> Result<Self, failure::Error> {
        Ok(Self {
            models: [
                ObjModel::new(display, load_resource!("models/fighter.obj"),  load_resource!("models/fighter.png"))?,
                ObjModel::new(display, load_resource!("models/tanker.obj"),   load_resource!("models/tanker.png"))?,
                ObjModel::new(display, load_resource!("models/carrier.obj"),  load_resource!("models/carrier.png"))?,
                ObjModel::new(display, load_resource!("models/asteroid.obj"), load_resource!("models/asteroid.png"))?,
                ObjModel::new(display, load_resource!("models/miner.obj"),    load_resource!("models/miner.png"))?,
                ObjModel::new(display, load_resource!("models/missile.obj"),  load_resource!("models/missile.png"))?
            ],
            image: load_image(display, load_resource!("output/packed.png")),
            font: runic::CachedFont::from_bytes(include_bytes!("pixel_operator/PixelOperator.ttf"), display)?,
            billboard: VertexBuffer::new(display, &BILLBOARD_VERTICES).unwrap()
        })
    }
}