use glium::*;
use genmesh;
use obj::*;
use std::io::*;
use context::Vertex;
use arrayvec::*;
use image;
use std::fs;

use glium::texture::*;

pub fn load_image(display: &Display, data: &[u8]) -> SrgbTexture2d {
    let image = image::load_from_memory(data).unwrap().to_rgba();
    let image_dimensions = image.dimensions();
    let image = RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);
    SrgbTexture2d::new(display, image).unwrap()
}

// Returns a vertex buffer that should be rendered as `TrianglesList`.
pub fn load_wavefront(display: &Display, data: &[u8]) -> VertexBuffer<Vertex> {
    let mut data = BufReader::new(data);
    let data = Obj::load_buf(&mut data).unwrap();

    let Obj {texture, normal, position, objects, ..} = data;

    let vertices: Vec<Vertex> = objects.into_iter()
        .flat_map(|object| object.groups)
        .flat_map(|group| group.polys)
        .flat_map(|polygon| {
            match polygon {
                genmesh::Polygon::PolyTri(genmesh::Triangle { x: v1, y: v2, z: v3 }) => ArrayVec::from([v1, v2, v3]),
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


pub struct Model {
    pub vertices: VertexBuffer<Vertex>,
    pub texture: SrgbTexture2d
}

impl Model {
    pub fn new(display: &Display, model: &str, image_filename: &str) -> Self {
        Self {
            vertices: load_wavefront(display, &fs::read(model).unwrap()),
            texture: load_image(&display, &fs::read(image_filename).unwrap())
        }
    }
}

pub struct Resources {
    pub models: [Model; 2],
    pub skybox: VertexBuffer<Vertex>,
    pub skybox_images: [SrgbTexture2d; 1],
    pub font: runic::CachedFont<'static>
}

impl Resources {
    pub fn new(display: &Display) -> Self {
        Self {
            models: [
                Model::new(display, "models/fighter.obj", "models/fighter.png"),
                Model::new(display, "models/tanker.obj", "models/tanker.png")
            ],
            skybox: load_wavefront(display, &fs::read("models/skybox.obj").unwrap()),
            skybox_images: [
                load_image(display, &fs::read("models/skybox.png").unwrap())
            ],
            font: runic::CachedFont::from_bytes(include_bytes!("DS-DIGIB.TTF"), display).unwrap()
        }
    }
}
use runic;