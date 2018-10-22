use glium::*;
use genmesh;
use obj::*;
use context::Vertex;
use image;
use std::fs;
use runic;
use util::*;
use glium::texture::*;
use std::io;
use failure;
use specs::*;

#[cfg(feature = "embed_resources")]
macro_rules! load_resource {
    ($filename:expr) => (
        include_bytes!(concat!("../../", $filename))
    )
}

#[cfg(not(feature = "embed_resources"))]
macro_rules! load_resource {
    ($filename:expr) => (
        &fs::read($filename)?
    )
}

pub fn billboard(display: &Display) -> VertexBuffer<Vertex> {
    let normal = [0.0, 0.0, 1.0];
        
    let top_left = Vertex {
        position: [-0.5, 0.5, 0.0],
        texture: [0.0; 2],
        normal
    };

    let top_right = Vertex {
        position: [0.5, 0.5, 0.0],
        texture: [1.0, 0.0],
        normal
    };

    let bottom_left = Vertex {
        position: [-0.5, -0.5, 0.0],
        texture: [0.0, 1.0],
        normal
    };

    let bottom_right = Vertex {
        position: [0.5, -0.5, 0.0],
        texture: [1.0; 2],
        normal
    };

    VertexBuffer::new(display, &[top_left, top_right, bottom_left, top_right, bottom_right, bottom_left]).unwrap()
}

pub fn load_image(display: &Display, data: &[u8]) -> SrgbTexture2d {
    let image = image::load_from_memory_with_format(data, image::ImageFormat::PNG).unwrap().to_rgba();
    let image_dimensions = image.dimensions();
    let image = RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);
    SrgbTexture2d::new(display, image).unwrap()
}

// Returns a vertex buffer that should be rendered as `TrianglesList`.
pub fn load_wavefront(display: &Display, data: &[u8]) -> VertexBuffer<Vertex> {
    let mut data = io::BufReader::new(data);
    let data = Obj::load_buf(&mut data).unwrap();

    let Obj {texture, normal, position, objects, ..} = data;

    let vertices: Vec<Vertex> = objects.into_iter()
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
        .collect();

    VertexBuffer::new(display, &vertices).unwrap()
}

#[derive(Copy, Clone)]
pub enum Image {
    Star = 0,
    Button = 1,
    Move = 2,
    Refuel = 3,
    RefuelFrom = 4,
    Mine = 5
}

#[derive(Deserialize, Serialize, Component, Copy, Clone)]
pub enum Model {
    Fighter = 0,
    Tanker = 1,
    Carrier = 2,
    Asteroid = 3
}

pub struct ObjModel {
    pub vertices: VertexBuffer<Vertex>,
    pub texture: SrgbTexture2d
}

impl ObjModel {
    fn new(display: &Display, model: &[u8], image: &[u8]) -> io::Result<Self> {
        Ok(Self {
            vertices: load_wavefront(display, model),
            texture: load_image(display, image)
        })
    }
}

pub struct Resources {
    pub models: [ObjModel; 4],
    pub images: [SrgbTexture2d; 6],
    pub font: runic::CachedFont<'static>
}

impl Resources {
    pub fn new(display: &Display) -> Result<Self, failure::Error> {
        Ok(Self {
            models: [
                ObjModel::new(display, load_resource!("resources/models/fighter.obj"),   load_resource!("resources/models/fighter.png"))?,
                ObjModel::new(display, load_resource!("resources/models/tanker.obj"),    load_resource!("resources/models/tanker.png"))?,
                ObjModel::new(display, load_resource!("resources/models/carrier.obj"),   load_resource!("resources/models/carrier.png"))?,
                ObjModel::new(display, load_resource!("resources/models/asteroid.obj"),  load_resource!("resources/models/asteroid.png"))?
            ],
            images: [
                load_image(display, load_resource!("resources/star.png")),
                load_image(display, load_resource!("resources/ui/button.png")),
                load_image(display, load_resource!("resources/ui/move.png")),
                load_image(display, load_resource!("resources/ui/refuel.png")),
                load_image(display, load_resource!("resources/ui/refuel_from.png")),
                load_image(display, load_resource!("resources/ui/mine.png"))
            ],
            font: runic::CachedFont::from_bytes(include_bytes!("pixel_operator/PixelOperator.ttf"), display)?
        })
    }

    pub fn image_dimensions(&self, image: Image) -> (u32, u32) {
        self.images[image as usize].dimensions()
    }
}