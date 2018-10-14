use glium::*;
use genmesh;
use obj::*;
use std::io::*;
use context::Vertex;
use image;
use std::fs;
use runic;
use util::*;
use glium::texture::*;
use std::path;

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

pub enum Image {
    Star = 0
}

pub enum Model {
    Fighter = 0,
    Tanker = 1
}

pub struct ObjModel {
    pub vertices: VertexBuffer<Vertex>,
    pub texture: SrgbTexture2d
}

impl ObjModel {
    fn new<P: AsRef<path::Path>>(display: &Display, model: P, image: P) -> Self {
        Self {
            vertices: load_wavefront(display, &fs::read(model).unwrap()),
            texture: load_image(display, &fs::read(image).unwrap())
        }
    }
}

pub struct Resources {
    pub models: [ObjModel; 2],
    pub images: [SrgbTexture2d; 1],
    pub font: runic::CachedFont<'static>
}

impl Resources {
    pub fn new(display: &Display) -> Self {
        let root = path::PathBuf::from("resources");
        let models = root.join("models");

        Self {
            models: [
                ObjModel::new(display, models.join("fighter.obj"), models.join("fighter.png")),
                ObjModel::new(display, models.join("tanker.obj"), models.join("tanker.png"))
            ],
            images: [
                load_image(display, &fs::read(root.join("star.png")).unwrap())
            ],
            font: runic::CachedFont::from_bytes(include_bytes!("pixel_operator/PixelOperator.ttf"), display).unwrap()
        }
    }
}