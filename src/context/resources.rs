#![cfg_attr(feature = "cargo-clippy", allow(clippy::unreadable_literal))]

use crate::context::Vertex;
use crate::util::*;
use arrayvec::*;
use nalgebra::Point3;
use ncollide3d::shape::TriMesh;
use obj::*;
use specs::*;
use std::io;
use zerocopy::*;

const NORMAL: [f32; 3] = [0.0, 0.0, 1.0];

const TOP_LEFT: Vertex = Vertex {
    position: [-0.5, 0.5, 0.0],
    diff_texture: [0.0; 2],
    spec_texture: [0.0; 2],
    normal: NORMAL,
};

const TOP_RIGHT: Vertex = Vertex {
    position: [0.5, 0.5, 0.0],
    diff_texture: [1.0, 0.0],
    spec_texture: [0.0; 2],
    normal: NORMAL,
};

const BOTTOM_LEFT: Vertex = Vertex {
    position: [-0.5, -0.5, 0.0],
    diff_texture: [0.0, 1.0],
    spec_texture: [0.0; 2],
    normal: NORMAL,
};

const BOTTOM_RIGHT: Vertex = Vertex {
    position: [0.5, -0.5, 0.0],
    diff_texture: [1.0; 2],
    spec_texture: [0.0; 2],
    normal: NORMAL,
};

pub const BILLBOARD_VERTICES: [Vertex; 6] = [
    TOP_LEFT,
    TOP_RIGHT,
    BOTTOM_LEFT,
    TOP_RIGHT,
    BOTTOM_RIGHT,
    BOTTOM_LEFT,
];

macro_rules! load_resource {
    ($filename:expr) => {
        include_bytes!(concat!("../../resources/", $filename))
    };
}

pub fn load_image(
    encoder: &mut wgpu::CommandEncoder,
    device: &wgpu::Device,
    bytes: &[u8],
) -> wgpu::TextureView {
    let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .unwrap()
        .into_rgba();

    let temp_buf = device.create_buffer_with_data(&*image, wgpu::BufferUsage::COPY_SRC);

    let texture_extent = wgpu::Extent3d {
        width: image.width(),
        height: image.height(),
        depth: 1,
    };

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        size: texture_extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
        label: Some("Hectic Texture"),
    });

    encoder.copy_buffer_to_texture(
        wgpu::BufferCopyView {
            buffer: &temp_buf,
            offset: 0,
            bytes_per_row: 4 * image.width(),
            rows_per_image: 0,
        },
        wgpu::TextureCopyView {
            texture: &texture,
            mip_level: 0,
            array_layer: 0,
            origin: wgpu::Origin3d::ZERO,
        },
        texture_extent,
    );

    texture.create_default_view()
}

// Returns a vertex buffer that should be rendered as `TrianglesList`.
pub fn load_wavefront(data: &[u8], model: Model) -> Vec<Vertex> {
    let mut data = io::BufReader::new(data);
    let data = ObjData::load_buf(&mut data).unwrap();

    let ObjData {
        texture,
        normal,
        position,
        objects,
        ..
    } = data;

    objects
        .into_iter()
        .flat_map(|object| object.groups)
        .flat_map(|group| group.polys)
        .flat_map(|polygon| match polygon.into_genmesh() {
            genmesh::Polygon::PolyTri(genmesh::Triangle {
                x: v1,
                y: v2,
                z: v3,
            }) => iter_owned([v1, v2, v3]),
            genmesh::Polygon::PolyQuad(_) => {
                unimplemented!("Quad polygons not supported, use triangles instead.")
            }
        })
        .map(|v| {
            let texture = v.1.map(|index| texture[index]).unwrap_or([0.0, 0.0]);
            let texture = [texture[0], 1.0 - texture[1]];
            Vertex {
                position: position[v.0],
                normal: v.2.map(|index| normal[index]).unwrap_or([0.0, 0.0, 0.0]),
                diff_texture: model.diffuse().translate(texture),
                spec_texture: model
                    .specular()
                    .map(|image| image.translate(texture))
                    .unwrap_or([-1.0; 2]),
            }
        })
        .collect()
}

include!(concat!(env!("OUT_DIR"), "/packed_textures.rs"));

#[derive(Serialize, Deserialize, Component, Copy, Clone, Hash, PartialEq, Eq)]
pub enum Model {
    Fighter = 0,
    Tanker = 1,
    Carrier = 2,
    Asteroid = 3,
    Miner = 4,
    Missile = 5,
}

impl Model {
    pub fn diffuse(self) -> Image {
        match self {
            Model::Fighter => Image::Fighter,
            Model::Tanker => Image::Tanker,
            Model::Carrier => Image::Carrier,
            Model::Asteroid => Image::Asteroid,
            Model::Miner => Image::Miner,
            Model::Missile => Image::Missile,
        }
    }

    pub fn specular(self) -> Option<Image> {
        Some(match self {
            Model::Fighter => Image::FighterSpecular,
            Model::Tanker => Image::TankerSpecular,
            Model::Carrier => Image::CarrierSpecular,
            Model::Miner => Image::MinerSpecular,
            _ => return None,
        })
    }
}

pub struct ObjModel {
    pub vertices: wgpu::Buffer,
    pub vertices_len: usize,
}

impl ObjModel {
    fn new(device: &wgpu::Device, bytes: &[u8], model: Model) -> io::Result<(Self, TriMesh<f32>)> {
        let vertices = load_wavefront(bytes, model);

        let points: Vec<_> = vertices
            .iter()
            .map(|vertex| Point3::new(vertex.position[0], vertex.position[1], vertex.position[2]))
            .collect();

        let faces = (0..vertices.len() / 3)
            .map(|i| Point3::new(i * 3, i * 3 + 1, i * 3 + 2))
            .collect();

        Ok((
            Self {
                vertices_len: vertices.len(),
                vertices: device
                    .create_buffer_with_data(vertices.as_bytes(), wgpu::BufferUsage::VERTEX),
            },
            TriMesh::new(points, faces, None),
        ))
    }
}

pub type MeshArray = ArrayVec<[TriMesh<f32>; 6]>;
type Models = ArrayVec<[ObjModel; 6]>;

pub struct Resources {
    pub models: Models,
    pub image: wgpu::TextureView,
    //pub font: runic::CachedFont<'static>
}

impl Resources {
    pub fn new(
        encoder: &mut wgpu::CommandEncoder,
        device: &wgpu::Device,
    ) -> Result<(Self, MeshArray), failure::Error> {
        let mut meshes = MeshArray::new();
        let mut models = Models::new();

        add_model(
            &mut meshes,
            &mut models,
            device,
            load_resource!("models/fighter.obj"),
            Model::Fighter,
        )?;
        add_model(
            &mut meshes,
            &mut models,
            device,
            load_resource!("models/tanker.obj"),
            Model::Tanker,
        )?;
        add_model(
            &mut meshes,
            &mut models,
            device,
            load_resource!("models/carrier.obj"),
            Model::Carrier,
        )?;
        add_model(
            &mut meshes,
            &mut models,
            device,
            load_resource!("models/asteroid.obj"),
            Model::Asteroid,
        )?;
        add_model(
            &mut meshes,
            &mut models,
            device,
            load_resource!("models/miner.obj"),
            Model::Miner,
        )?;
        add_model(
            &mut meshes,
            &mut models,
            device,
            load_resource!("models/missile.obj"),
            Model::Missile,
        )?;

        Ok((
            Self {
                models,
                image: load_image(encoder, device, load_resource!("output/packed.png")),
            },
            meshes,
        ))
    }
}

pub fn add_model(
    meshes: &mut MeshArray,
    models: &mut Models,
    device: &wgpu::Device,
    obj: &[u8],
    model: Model,
) -> Result<(), failure::Error> {
    let (object, mesh) = ObjModel::new(device, obj, model)?;
    models.push(object);
    meshes.push(mesh);

    Ok(())
}
