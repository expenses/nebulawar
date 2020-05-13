#![cfg_attr(feature = "cargo-clippy", allow(clippy::unreadable_literal))]

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
use arrayvec::*;
use ncollide3d::shape::TriMesh;
use nalgebra::Point3;

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
    let image = image::load_from_memory(data).unwrap().to_rgba();
    let image_dimensions = image.dimensions();
    let image = RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);
    SrgbTexture2d::new(display, image).unwrap()
}

// Returns a vertex buffer that should be rendered as `TrianglesList`.
pub fn load_wavefront(data: &[u8]) ->  Vec<Vertex> {
    let mut data = io::BufReader::new(data);
    let data = ObjData::load_buf(&mut data).unwrap();

    let ObjData {texture, normal, position, objects, ..} = data;

    objects.into_iter()
        .flat_map(|object| object.groups)
        .flat_map(|group| group.polys)
        .flat_map(|polygon| {
            match polygon.into_genmesh() {
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
    pub texture: SrgbTexture2d
}

impl ObjModel {
    fn new(display: &Display, model: &[u8], image: &[u8]) -> io::Result<(Self, TriMesh<f32>)> {
        let vertices = load_wavefront(model);

        let points: Vec<_> = vertices.iter()
            .map(|vertex| Point3::new(vertex.position[0], vertex.position[1], vertex.position[2]))
            .collect();

        let faces = (0 .. vertices.len() / 3)
            .map(|i| Point3::new(i * 3, i * 3 + 1, i * 3 + 2))
            .collect();

        Ok((
            Self {
                vertices: VertexBuffer::new(display, &vertices).unwrap(),
                texture: load_image(display, image)
            },
            TriMesh::new(points, faces, None)
        ))
    }
}

pub type MeshArray = ArrayVec<[TriMesh<f32>; 6]>;
type Models = ArrayVec<[ObjModel; 6]>;

pub struct Resources {
    pub models: Models,
    pub image: SrgbTexture2d,
    pub billboard: VertexBuffer<Vertex>,
    pub font: runic::CachedFont<'static>
}

impl Resources {
    pub fn new(display: &Display) -> Result<(Self, MeshArray), failure::Error> {
        let mut meshes = MeshArray::new();
        let mut models = Models::new();

        add_model(&mut meshes, &mut models, display, load_resource!("models/fighter.obj"),  load_resource!("models/fighter.png"))?;
        add_model(&mut meshes, &mut models, display, load_resource!("models/tanker.obj"),   load_resource!("models/tanker.png"))?;
        add_model(&mut meshes, &mut models, display, load_resource!("models/carrier.obj"),  load_resource!("models/carrier.png"))?;
        add_model(&mut meshes, &mut models, display, load_resource!("models/asteroid.obj"), load_resource!("models/asteroid.png"))?;
        add_model(&mut meshes, &mut models, display, load_resource!("models/miner.obj"),    load_resource!("models/miner.png"))?;
        add_model(&mut meshes, &mut models, display, load_resource!("models/missile.obj"),  load_resource!("models/missile.png"))?;

        Ok((
            Self {
                models,
                image: load_image(display, load_resource!("output/packed.png")),
                font: runic::CachedFont::from_bytes(include_bytes!("pixel_operator/PixelOperator.ttf"), display)?,
                billboard: VertexBuffer::new(display, &BILLBOARD_VERTICES).unwrap(),
            },
            meshes
        ))
    }
}

pub fn add_model(meshes: &mut MeshArray, models: &mut Models, display: &Display, obj: &[u8], png: &[u8]) -> Result<(), failure::Error> {
    let (object, mesh) = ObjModel::new(display, obj, png)?;
    models.push(object);
    meshes.push(mesh);

    Ok(())
}
